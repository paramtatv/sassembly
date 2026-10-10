# Networking: host setup, and what v1.1.0-alpha does not cover

The network stack is `.t1` (`spec/net/`), running above yantra's frames-only virtio-net device
(ADR-0047) or above QEMU's virtio-net. The device only moves frames. Whatever reaches the
outside world is the host's choice, made with one flag after `--record-events <log>`.

## The three ways out

| Path | Flag | Privilege | What it reaches |
|---|---|---|---|
| Program to program | `--net-peer=PATH` | none | another `yantra-run` started with the same PATH |
| TAP | `--net-tap=IFNAME` | an interface made beforehand (root once), or `CAP_NET_ADMIN` | the host's network, through bridging or routing the operator sets up |
| QEMU user-mode | (QEMU's `-netdev user`) | none | the demo gateway 10.0.2.2 and DNS 10.0.2.3; `tools/check-net-t1.sh` |

Every live run writes a frame log, and `yantra-run --events <log> prog.elf` replays it with no backend
and no flag. The replay matches the live run in output, instruction count and the sha256 of frames sent.

## Platform status

| Platform | `--net-peer` | `--net-tap` | Tested |
|---|---|---|---|
| Linux x86_64, aarch64, riscv64 | yes | yes (`/dev/net/tun`) | a Linux x86_64 host, 2026-10-07: full suite, TAP probe once |
| macOS aarch64 | yes | refused by name: vmnet needs the `com.apple.vm.networking` entitlement or root | an Apple M1 host, 2026-10-09: `net_frames` 21/21 (all but the Linux TAP probe), `net_names` 4/4, `net_t1` 16/16 with `--include-ignored`, UDP and DNS pairs included |
| macOS x86_64 | yes (same code as aarch64) | refused, as above | not run |
| Windows | refused by name: std has no server side for a named pipe | refused by name: needs the Wintun driver and administrator rights | refusal paths compile by `cfg` only |
| Browser (`yantra-wasm`) | absent (no backends) | absent | see "Not in the alpha" |

A refusal says `NET_BACKEND_UNAVAILABLE` (ADR-0048's name), the flag, and what the platform would need.
Nothing runs.

## Linux TAP, once per machine

```sh
sudo ip tuntap add dev sastap0 mode tap user "$(id -u)"
sudo sysctl -w net.ipv6.conf.sastap0.disable_ipv6=1   # else the kernel's router solicitation is the first frame
sudo ip link set sastap0 up
yantra-run --record-events run.log --net-tap=sastap0 prog.elf
```

After this, attaching needs no privilege. To reach the internet, route or bridge `sastap0` yourself
(NAT through the host, or a bridge with the uplink). yantra does no NAT, and there is no slirp in Rust
(owner ruling).

## macOS and Windows today

- **macOS**: use `--net-peer` between programs, QEMU user-mode for the demos, or a Linux VM with TAP
  for the internet. A vmnet backend is future work. It needs either a signed, entitled binary or root.
- **Windows**: no backend in this build. Run the Linux build under WSL2 (Linux backends apply), or use
  QEMU. A named-pipe peer and a Wintun TAP are future work.

## Not in the alpha

1. **Frame-log replay in the browser.** `yantra::netdev::replay_net` builds for every target, but
   `yantra-wasm` exports no way to hand it a log. A net image run in the page halts at the device
   (code 14) or at its first wait (code 15). Ruled out of v1.1.0-alpha on 2026-10-09, because Sassembly-web
   was stopped on 2026-10-07.
2. **A fence before Notify (V-011).** The driver orders its ring writes before the Notify store by
   program order alone. `.t1` has no fence primitive, and the compiler is frozen. That is safe on yantra
   (sequential), on QEMU TCG and on x86 hosts. It must close before the driver runs on real RVWMO
   hardware.
3. **DNS on untrusted networks.** With no randomness, the source port and query ID carry at most
   16 bits, so an off-path spoofer can guess them, and eight junk frames can starve a lookup (ADR-0048).
   Use it on trusted links only.
4. **TCP, HTTP/1.1, TLS**: not in this alpha.
