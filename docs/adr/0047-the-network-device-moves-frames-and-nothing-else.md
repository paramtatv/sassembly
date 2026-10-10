# ADR-0047 — The network device moves frames and nothing else

**Status:** ACCEPTED, 2026-10-08 — the owner ruled "names: primaries; ADR-0047 accepted". The naming ruling is ADR-0048; the placeholder names of D9 are replaced there (nine ruled, five still placeholders). Owner rulings of 2026-10-07 are recorded below.
**Writes code:** yes, `yantra` only (`crates/yantra/src/netdev.rs`, `threads.rs`, `bin/yantra-run.rs`). **Changes a frozen production:** no. No `.t1` in this ADR; the names are ADR-0048's.
**Builds on:** W-351 (virtio-mmio), W-371/W-373 (events, WAIT), W-375 (clock record), W-376 (threads), W-377 (sockets). **Row:** to be filed against SAS-020's networking plan; none existed.

## Decision

**D1. Frames only ("a-fully pure").** yantra gains a virtio-net device that moves raw Ethernet frames.
No socket protocol logic, no TLS, no NAT/slirp and no TCP/IP in Rust. Everything above the frame is
`.t1`, written later, after the naming ruling. TLS is not part of this device.

**D2. It looks like QEMU's virtio-net**, so one `.t1` driver runs on both. Legacy virtio-mmio
(version 1) in slot 2 (`0x1000_3000`; slot 0 is the GPU, slot 1 overlaps the file window). Device ID 1.
Features: `VIRTIO_NET_F_MTU` (3), `_MAC` (5), `_STATUS` (16); nothing else. Config space at `+0x100`:
`mac[6]`, `status` (LINK_UP), `max_virtqueue_pairs` (0), `mtu` (`max_frame - 14`), all read-only. Queue 0
receiveq, queue 1 transmitq, 256 entries at most. Each buffer starts with the 10-octet `virtio_net_hdr`
(ignored on TX, zero on RX), then one frame. A bit the device did not offer is refused when acknowledged.

**D3. No device without a flag.** With no `--net-*` flag, any access to slot 2 halts `Device` with a
placeholder-named refusal (`NET_WINDOW_REFUSED`, English, `TODO(owner)`).

**D4. Waits move frames.** TX is queued at QueueNotify and handed to the backend at the next WAIT or at
exit. RX happens only at a WAIT: one frame per wait, written into the next posted buffer, and the
`SASEVENT` word receives the octets placed (0 = dropped: no buffer, one too small, or the MAC filter).
A live wait blocks until a frame arrives; `--net-timeout ms` turns expiry into a refusal. Between waits
nothing changes, so the instruction count is a function of `(program, log)`.

**D5. Backends, host side only** (the device knows no transport; `std::net` stays in `yantra-run.rs`):
- `--net-peer=PATH`: ONE flag. The first instance binds a unix-domain stream socket at PATH, the second
  connects; frames cross as a 2-octet big-endian length and the frame. Linux and macOS. Windows would be a
  named pipe; std has no server side for one, so this build refuses by name there.
- `--net-tap=IFNAME`: opt-in, platform-native. Linux: `/dev/net/tun` (`IFF_TAP|IFF_NO_PI`), the crate's
  one `unsafe` (a single `ioctl`). Creating an interface needs `CAP_NET_ADMIN`; attaching to one made
  beforehand for the user does not. macOS (vmnet: entitlement or root) and Windows (Wintun driver and
  administrator rights) are refusal paths only.

**D6. Caps, each refusing by name:** `--net-max-frame` (default 1514, 14 to 65,535), `--net-max-tx-frames`
and `--net-max-rx-frames` (each default 65,536, counted separately).

**D7. `--net-mac` is also the optional MAC allow-list.** Given, it sets the device MAC and turns the
filter on: a transmitted frame whose source is another MAC is refused; a received frame addressed to
neither the MAC nor a group is dropped and counted. Not given, there is no filter. Filtering by IP or
port is the `.t1` stack's, not the device's, which sees no addresses.

**D8. The frame log** (`--record-events` writes it, `--events` replays it with no backend and no flag):

```
# yantra-net v1 mac=.. max-frame=1514 max-tx=65536 max-rx=65536 filter=0 ...
@N                       (threaded runs only) run thread N
n=<wait>:<handle>:<hex>  the frame delivered at wait <wait>, to handle <handle> (0)
n=<wait>:<handle>:none   nothing arrived
```

`<wait>` is the ordinal over the whole run and `<handle>` the receiving handle (0 for the device; later
rows add more); both are checked on reading, so a spliced log is refused. The header carries the device
configuration. A threaded image runs by readiness: with another thread runnable a frame already here wakes
the first waiting thread, otherwise the runnable thread runs; with none runnable the host blocks. The
chosen thread's `@N` precedes its `n=` record, as in W-376. The other log readers refuse an `n=` record by
name; the net reader refuses plain, `t=`, `s=` lines. The closing `net:` line gives the counts and the
sha256 of the frames sent.

**D9. Refusal names are placeholders** (English constants with `TODO(owner)` in `netdev.rs`):
`NET_WINDOW_REFUSED`, `NET_FRAME_TOO_LARGE`, `NET_FRAME_RUNT`, `NET_TX_NO_HEADER`, `NET_TX_COUNT_EXCEEDED`,
`NET_RX_COUNT_EXCEEDED`, `NET_SRC_MAC_REFUSED`, `NET_TIMEOUT`, `NET_PEER_CLOSED`, `NET_BACKEND_UNAVAILABLE`,
`NET_LOG_TAG_MISMATCH`.

## Consequences

- yantra changes, so a Stage 2 re-execution is owed: same image, same instruction count (no compiler or
  `.t1` change).
- `yantra-wasm` keeps the device and replay (`netdev::replay_net`, `NetThreads`); only the backends, which
  live in the host binary, are absent there.
- The pure `.t1` stack (ARP, IP, UDP, TCP, DNS, HTTP) is separate work after the naming ruling.

## What would show this wrong

- A QEMU virtio-net `.t1` driver that needs a feature or a register this device lacks (for example
  `MRG_RXBUF`): then D2's set grows, and that is an ADR amendment.
- A guest that must wait for a frame while other work proceeds in the same thread: D4's block-until-frame
  wait would starve it; the answer is threads (D8), not a timer.

## What was tested

Linux only (a Linux x86_64 host): negotiation as a guest does it; refusal without a flag; every cap; the allow-list;
live then replay identical in output, instruction count, memory and frames out; a threaded run in both wake
orders; two `yantra-run` processes over `--net-peer`; a silent peer and `--net-timeout`; TAP against a
pre-made interface (an ignored test, run once). Not tested: macOS vmnet, Windows named pipe and Wintun
(refusal paths compile only by `cfg`; the macOS and Windows builds were not run).

Addendum 2026-10-09: macOS aarch64 (an Apple M1 host, with the `net_t1.rs` fix below) runs
`net_frames` 21/21 (all but the Linux TAP probe), `net_names` 4/4 and `net_t1` 16/16 with
`--include-ignored`, including two processes over `--net-peer`, each log replaying alone. The fix: the
peer socket path moved out of the test's scratch directory, because macOS caps `sun_path` at 104
octets and the deferred UDP test made 106. `yantra-run` refused that path by name, and the test now
fails on that refusal instead of waiting 60 s. macOS x86_64 and Windows are still not run. The host
setup and the alpha's exclusions are in `docs/networking-host-setup.md`.
