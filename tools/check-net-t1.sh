#!/bin/sh
# ADR-0048 — the .t1 network library under QEMU's virtio-net, the second device that shares
# no code with yantra's frame device.
#
# ## What is being proved
#
# spec/net/*.t1 (the virtio-net driver, Ethernet, ARP, IPv4, ICMP echo, UDP, DNS) is built with
# two demo programs, each handed the role `q`, and run on QEMU `virt` with a user-mode netdev
# (slirp) and a virtio-net-device. The guest is 10.0.2.15.
#
#  1. spec/net/net-demo.t1: it ARPs for 10.0.2.2, the gateway slirp answers for, and pings it.
#     The program prints "Q <status>", status 0 being a checked echo reply. QEMU's own
#     filter-dump shows an ARP request and reply, an ICMP echo request and reply.
#  2. spec/net/net-udp-demo.t1 (milestone 3): it ARPs for 10.0.2.3, slirp's DNS server, and
#     asks it for "localhost" with an A-record query over UDP port 53. The program prints
#     "Q <status>" and "A <address>". The capture shows the ARP pair, a UDP datagram to port
#     53 and one from port 53, and the A record in that response is the address the guest
#     printed. The name is answered by the HOST's resolver through slirp: a host whose
#     resolver cannot answer "localhost" fails this half honestly (FAIL, not a skip).
#
# The driver needs no change between the two devices: on QEMU there is no WAIT (the store
# would fault), so the role sets `सञ्चारपङ्क्तिःॱप्रतीक्षाचालः` to 0 and the driver only polls
# the used ring, bounded. The device is not pinned: QEMU puts it in the last virtio-mmio slot
# and the driver's scan (slots 0, 2, 3, ... 7, never 1) finds it.
#
# The rest of the library's tests are crates/yantra/tests/net_t1.rs (--include-ignored).
#
# Exits 0 on PASS, 1 on FAIL, 77 when QEMU or python3 is absent.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

for t in qemu-system-riscv64 python3; do
  command -v "$t" >/dev/null 2>&1 || {
    echo "SKIPPED: $t is not installed"
    echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
    exit 77; }
done

RAM=738197504                     # the driver's region is at 0xA800_0000
: "${CARGO_TARGET_DIR:=$root/target}"
echo "Building t1_image and yantra-run (release)..."
cargo build --quiet --release --manifest-path "$root/Cargo.toml" --bin t1_image --bin yantra-run
T1="$CARGO_TARGET_DIR/release/t1_image"
qemu-system-riscv64 --version | head -1

net="$root/spec/net"
lib="$net/sanchara-pankti.t1 $net/sambandha.t1 $net/marga.t1 $net/marga-sandesha.t1 $net/sthana-nirdesha.t1 $net/nihsandhi-pinda.t1 $net/nama-nirdesha.t1 $net/sanchara.t1"

# build NAME DEMO MODULE: the library and a demo program into $tmp/NAME.elf
build() {
  files="$lib $net/$2"
  load=""; for f in $files; do load="$load --load $f"; done
  # shellcheck disable=SC2086
  "$T1" --spec-root "$root/spec" --compiler "$root/crates/sadhana-t1/src" $load \
      --entry "$3" मुख्यम् -o "$tmp/$1.elf" $files > "$tmp/$1.build.log" 2>&1 || true
  [ -s "$tmp/$1.elf" ] || { echo "  FAIL  $1: the program did not build:"; tail -20 "$tmp/$1.build.log"; exit 1; }
  grep -q '^gate:     AGREED' "$tmp/$1.build.log" && echo "  PASS  build $1: the differential gate says AGREED" \
    || { echo "  FAIL  build $1: no AGREED"; exit 1; }
}

# qemu NAME: run the role q of $tmp/NAME.elf; serial in $tmp/NAME.serial, capture $tmp/NAME.pcap
qemu() {
  addr=$(python3 "$root/tools/t1-qemu-args.py" --ram $RAM "$tmp/$1.elf" "$tmp/$1.q" "$tmp/$1.args" "$tmp/$1.elf" q \
      | sed -n 's/^addr //p')
  timeout 120 qemu-system-riscv64 -machine virt -bios none -m 768M -display none -kernel "$tmp/$1.q" \
      -device "loader,file=$tmp/$1.args,addr=$addr" \
      -netdev user,id=n0 -device virtio-net-device,netdev=n0 \
      -object filter-dump,id=f1,netdev=n0,file="$tmp/$1.pcap" \
      -serial "file:$tmp/$1.serial" -monitor none -no-reboot > "$tmp/$1.out" 2>&1 || true
}

# kinds PCAP: arp1/arp2, icmp8/icmp0, udp>53 (to port 53) and udp<53 (from port 53), in order
kinds() {
python3 - "$1" <<'PY'
import struct, sys
d = open(sys.argv[1], "rb").read()
o, out = 24, []
while o < len(d):
    _, _, cl, _ = struct.unpack("<IIII", d[o:o + 16]); fr = d[o + 16:o + 16 + cl]; o += 16 + cl
    et = fr[12:14].hex()
    if et == "0806": out.append("arp%d" % fr[21])
    elif et == "0800" and fr[23] == 1: out.append("icmp%d" % fr[34])
    elif et == "0800" and fr[23] == 17:
        sp, dp = struct.unpack(">HH", fr[34:38])
        if dp == 53: out.append("udp>53")
        elif sp == 53: out.append("udp<53")
print(" ".join(out))
PY
}

# answer PCAP: the first A record's address of the DNS response in the capture, as a number
answer() {
python3 - "$1" <<'PY'
import struct, sys
d = open(sys.argv[1], "rb").read()
o = 24
while o < len(d):
    _, _, cl, _ = struct.unpack("<IIII", d[o:o + 16]); fr = d[o + 16:o + 16 + cl]; o += 16 + cl
    if fr[12:14].hex() == "0800" and fr[23] == 17 and struct.unpack(">H", fr[34:36])[0] == 53:
        m = fr[42:]
        qd, an = struct.unpack(">HH", m[4:8]); p = 12
        for _ in range(qd):
            while m[p]: p += 1 + m[p]
            p += 5
        for _ in range(an):
            while m[p] and m[p] < 192: p += 1 + m[p]
            p += 2 if m[p] >= 192 else 1
            t, c, _ttl, rl = struct.unpack(">HHIH", m[p:p + 10]); p += 10
            if t == 1 and rl == 4:
                print(struct.unpack(">I", m[p:p + 4])[0]); sys.exit(0)
            p += rl
print("none")
PY
}

fail=0

build net net-demo.t1 सञ्चारपरीक्षा
qemu net
q=$(LC_ALL=C tr -d '\r' < "$tmp/net.serial" | sed -n 's/^Q //p')
if [ "$q" = 0 ]; then echo "  PASS  QEMU ping: the guest says Q 0 (a checked echo reply from 10.0.2.2)"
else echo "  FAIL  QEMU ping: the guest says Q ${q:-<none>}"; fail=1; fi
k=$(kinds "$tmp/net.pcap")
if [ "$k" = "arp1 arp2 icmp8 icmp0" ]; then echo "  PASS  QEMU's own capture: $k"
else echo "  FAIL  QEMU's own capture: ${k:-<none>} (want: arp1 arp2 icmp8 icmp0)"; fail=1; fi

build dns net-udp-demo.t1 सन्देशपरीक्षा
qemu dns
q=$(LC_ALL=C tr -d '\r' < "$tmp/dns.serial" | sed -n 's/^Q //p')
a=$(LC_ALL=C tr -d '\r' < "$tmp/dns.serial" | sed -n 's/^A //p')
if [ "$q" = 0 ]; then echo "  PASS  QEMU DNS: the guest says Q 0 and A ${a:-<none>} for localhost, asked of 10.0.2.3"
else echo "  FAIL  QEMU DNS: the guest says Q ${q:-<none>}"; fail=1; fi
k=$(kinds "$tmp/dns.pcap")
if [ "$k" = "arp1 arp2 udp>53 udp<53" ]; then echo "  PASS  QEMU's own capture: $k"
else echo "  FAIL  QEMU's own capture: ${k:-<none>} (want: arp1 arp2 udp>53 udp<53)"; fail=1; fi
c=$(answer "$tmp/dns.pcap")
if [ "$c" != none ] && [ "$c" = "${a:-x}" ]; then echo "  PASS  the A record in slirp's response is the address the guest printed ($c)"
else echo "  FAIL  the response's A record is ${c:-<none>}, the guest printed ${a:-<none>}"; fail=1; fi

[ "$fail" = 0 ] && echo "PASS" || echo "FAIL"
exit "$fail"
