# The frames device, natively (L8 / NATIVE-GAPS C4)

Phase 1: yantra's virtio-net (slot 2, `0x1000_3000..0x1000_3FFF`) with yantra-run's `--net-peer`
backend, in the x86-64 Linux runtime of `anuvada.py` (`runtime_net`). It is compiled in only when the
translation asks (`ANUVADA_NET=1`, set by `t1-native-lib.sh translate … net`, oracle only, own cache
entry `-net`) and is live only when `YANTRA_NET_PEER` is set. Without both, the translation and every
answer are what they were: a slot-2 access is `BadAccess`, t1-run reruns under yantra-run.

## Route (tools/t1-run.sh)

`t1-run.sh --record-events LOG --net-peer PATH [--net-mac M] [--net-max-frame N] [--net-max-tx-frames N]
[--net-max-rx-frames N] [--net-timeout MS] IMAGE role` (FLAG VALUE or FLAG=VALUE) runs natively on
x86_64 Linux. Everything else is yantra-run's: `--net-tap`, an unknown flag or a value yantra-run would
judge, `--files` with net, macOS / aarch64 / Windows, `--events` replay, a SASTHRDS image. `YANTRA_VIRTIO_DEFER=N` runs natively (L33, `ANUVADA_VD=1`: pending TX and staged RX
completions are served N guest words later, as in yantra). stderr is held until `net: peer … listening|connected`, then streamed. A
native-only halt before that reruns whole; after it, it is a terminal refusal (exit 1), never a rerun.

## What matches yantra-run, octet for octet

stdout; the halt line (Device halts with yantra's `why` in Rust Debug form; `Wait { pc }` at a refused
wait); exit status; the `net: … sent sha256 …` line (the runtime prints a marker, t1-run hashes the side
file `YANTRA_NET_SENT`); the event log (header + `n=<wait>:0:<hex|none>`), which replays under
`yantra-run --events LOG IMAGE role`; the rendezvous lines; the timeout and peer-closed refusals; every
register-file and virtqueue refusal (`check`, the read/write tables, `virtqueue::process` order).

## Known differences (phase 1)

- yantra-run prints `events: N net records logged …`, `events: LIVE …`, `args: …` and step lines;
  native omits them (stderr diagnostics only; stdout, halt, net:, rc and the log match).
- Slot 0 (yantra's GPU): only the identity words a driver's scan reads (MagicValue, Version, DeviceID,
  VendorID, Status = 0), net runs only; any other slot-0 access is the load/store refusal as before.
- A rendezvous refusal (`… (nothing ran)`) leaves no LOG, as yantra-run.
- No background RX drain: the peer stream is read only at a WAIT. Two programs that both transmit a large
  burst without waiting can fill both socket buffers and deadlock (yantra-run's reader thread drains).
- The TX outbox is also sent when it fills (256 KiB) and at every stdout flush, so frames can leave
  earlier than under yantra (never later; order is kept). A send failure's line can appear earlier.
- The send-failure line names errno 32 and 104 as Rust does; any other is `error (os error N)`.
- The clock log header is written at load and replaced by the net header once the peer is up; a
  backend failure can leave that file (t1-run uses a private work file, so LOG is untouched).

## Phase 2

- TAP backend (`--net-tap`): `/dev/net/tun` + TUNSETIFF in the runtime; same outbox / wait paths.
- aarch64 Linux (`runtime_a64`) and Darwin (AF_UNIX works; syscall numbers / no accept4, no MSG_NOSIGNAL:
  SO_NOSIGPIPE).
- The `.t1` translator port (today only the oracle carries it, so a net run always translates by
  anuvada.py).
- An RX drain (a reader thread or a non-blocking pre-read into a ring at every flush) to remove the
  deadlock caveat; `--files` together with net once the side file is outside the window by construction.
