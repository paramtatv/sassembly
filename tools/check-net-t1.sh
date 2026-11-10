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
# The rest of the library's tests are net_t1 (--include-ignored).
#
# Exits 0 on PASS, 1 on FAIL, 77 when QEMU is absent. Native only: no cargo, no yantra-run.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d "${TMPDIR:-/tmp}/cknett1.XXXXXX") || { echo "SKIPPED: no temporary directory"; exit 77; }
trap 'rm -rf "$tmp"' EXIT

for t in qemu-system-riscv64; do
  command -v "$t" >/dev/null 2>&1 || {
    echo "SKIPPED: $t is not installed"
    echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
    exit 77; }
done

RAM=738197504                     # the driver's region is at 0xA800_0000
T1="$root/tools/t1-build.sh"   # R3-SAD retired t1_image (Rust): same arguments, native route
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
  grep -a -q " octets) by " "$tmp/$1.build.log" && echo "  PASS  build $1: t1-build.sh wrote the image (R3-SAD: the native route replaces the t1_image AGREED gate)" \
    || { echo "  FAIL  build $1: t1-build.sh did not report the image written"; exit 1; }
}

# qemu NAME: run the role q of $tmp/NAME.elf; serial in $tmp/NAME.serial, capture $tmp/NAME.pcap
qemu() {
  addr=$(sh "$root/tools/t1-qemu-args.sh" --ram $RAM "$tmp/$1.elf" "$tmp/$1.q" "$tmp/$1.args" "$tmp/$1.elf" q \
      | sed -n 's/^addr //p')
  qemu-system-riscv64 -machine virt -bios none -m 768M -display none -kernel "$tmp/$1.q" \
      -device "loader,file=$tmp/$1.args,addr=$addr" \
      -netdev user,id=n0 -device virtio-net-device,netdev=n0 \
      -object filter-dump,id=f1,netdev=n0,file="$tmp/$1.pcap" \
      -serial "file:$tmp/$1.serial" -monitor none -no-reboot > "$tmp/$1.out" 2>&1 || true
}

# pcap_bytes FILE: the capture as decimal bytes, one stream (portable od).
# kinds PCAP: arp1/arp2, icmp8/icmp0, udp>53 (to port 53) and udp<53 (from port 53), in order
kinds() {
od -An -v -tu1 "$1" | LC_ALL=C awk '
{ for (i = 1; i <= NF; i++) d[n++] = $i + 0 }
function le32(i) { return d[i] + d[i+1] * 256 + d[i+2] * 65536 + d[i+3] * 16777216 }
function be16(i) { return d[i] * 256 + d[i+1] }
END {
    o = 24; out = ""
    while (o < n) {
        cl = le32(o + 8); f = o + 16; o += 16 + cl
        et = (d[f+12] == 8 && d[f+13] == 6) ? "arp" : ((d[f+12] == 8 && d[f+13] == 0) ? "ip" : "")
        if (et == "arp") out = out (out == "" ? "" : " ") "arp" d[f+21]
        else if (et == "ip" && d[f+23] == 1) out = out (out == "" ? "" : " ") "icmp" d[f+34]
        else if (et == "ip" && d[f+23] == 17) {
            sp = be16(f + 34); dp = be16(f + 36)
            if (dp == 53) out = out (out == "" ? "" : " ") "udp>53"
            else if (sp == 53) out = out (out == "" ? "" : " ") "udp<53"
        }
    }
    print out
}'
}

# answer PCAP: the first A record's address of the DNS response in the capture, as a number
answer() {
od -An -v -tu1 "$1" | LC_ALL=C awk '
{ for (i = 1; i <= NF; i++) d[n++] = $i + 0 }
function le32(i) { return d[i] + d[i+1] * 256 + d[i+2] * 65536 + d[i+3] * 16777216 }
function be16(i) { return d[i] * 256 + d[i+1] }
function be32(i) { return d[i] * 16777216 + d[i+1] * 65536 + d[i+2] * 256 + d[i+3] }
END {
    o = 24
    while (o < n) {
        cl = le32(o + 8); f = o + 16; o += 16 + cl
        if (d[f+12] == 8 && d[f+13] == 0 && d[f+23] == 17 && be16(f + 34) == 53) {
            m = f + 42                      # m[i] is d[m + i]
            qd = be16(m + 4); an = be16(m + 6); p = 12
            for (k = 0; k < qd; k++) {
                while (d[m+p] != 0) p += 1 + d[m+p]
                p += 5
            }
            for (k = 0; k < an; k++) {
                while (d[m+p] != 0 && d[m+p] < 192) p += 1 + d[m+p]
                p += (d[m+p] >= 192) ? 2 : 1
                t = be16(m + p); rl = be16(m + p + 8); p += 10
                if (t == 1 && rl == 4) { printf "%d\n", be32(m + p); exit }
                p += rl
            }
        }
    }
    print "none"
}'
}

# rcode PCAP: the RCODE of the DNS response in the capture (slirp relays to the HOST resolver, so the capture, not a constant, is the oracle)
rcode() {
od -An -v -tu1 "$1" | LC_ALL=C awk '
{ for (i = 1; i <= NF; i++) d[n++] = $i + 0 }
function le32(i) { return d[i] + d[i+1] * 256 + d[i+2] * 65536 + d[i+3] * 16777216 }
function be16(i) { return d[i] * 256 + d[i+1] }
END {
    o = 24
    while (o < n) {
        cl = le32(o + 8); f = o + 16; o += 16 + cl
        if (d[f+12] == 8 && d[f+13] == 0 && d[f+23] == 17 && be16(f + 34) == 53) { printf "%d\n", d[f + 42 + 3] % 16; exit }
    }
    print "none"
}'
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
c=$(answer "$tmp/dns.pcap"); rc=$(rcode "$tmp/dns.pcap")
# rcode 0: guest must print Q 0 and the pcap's A; rcode 3 (host resolver has no localhost): guest must print the NXDOMAIN status and no A
if [ "$rc" = 0 ]; then
    if [ "$q" = 0 ] && [ "$c" != none ] && [ "$c" = "${a:-x}" ]; then echo "  PASS  QEMU DNS: rcode 0, the guest says Q 0 and A $a, equal to the A record in slirp's response"
    else echo "  FAIL  QEMU DNS: rcode 0 but the guest says Q ${q:-<none>} A ${a:-<none>}, the response's A is $c"; fail=1; fi
elif [ "$rc" = 3 ]; then
    if [ "$q" = 56963891 ] && [ -z "$a" ]; then echo "  PASS  QEMU DNS: rcode 3 (host resolver: no such name), the guest says Q 56963891 and no A"
    else echo "  FAIL  QEMU DNS: rcode 3 but the guest says Q ${q:-<none>} A ${a:-<none>}"; fail=1; fi
else echo "  FAIL  QEMU DNS: the response's rcode is ${rc:-<none>} (want 0 or 3)"; fail=1; fi
k=$(kinds "$tmp/dns.pcap")
if [ "$k" = "arp1 arp2 udp>53 udp<53" ]; then echo "  PASS  QEMU's own capture: $k"
else echo "  FAIL  QEMU's own capture: ${k:-<none>} (want: arp1 arp2 udp>53 udp<53)"; fail=1; fi


[ "$fail" = 0 ] && echo "PASS" || echo "FAIL"
exit "$fail"
