#!/bin/sh
# The machine answers a ping.  Row `C-011`, doc 06 §8 — ONE slice of it, named
# honestly below.
#
# ## What is being proved, and what is NOT
#
# `C-011` reads "Network stack: Ethernet to IPv6/IPv4 to TCP to TLS 1.3 to
# HTTP/1.1 — RFC 1122 MUSTs".  This check does not prove that row.  There is no
# TCP here: no three-way handshake, no sequence numbers, no retransmission, no
# window.  There is no TLS 1.3 and no HTTP/1.1.  There is no IPv6, no IP
# fragment reassembly, no IP options, no UDP, no routing table, no DHCP.
#
# What is proved is the slice underneath all of that, which nothing above it can
# be built without and which cannot be established by inspection:
#
#     Ethernet frames move both ways over virtio-net; ARP is answered for our
#     address and only ours; an IPv4 header is parsed and its checksum verified;
#     an ICMP echo request is answered with a well-formed echo reply.
#
# TCP's first SYN never leaves the host until ARP has produced the next hop's
# hardware address, and it never arrives until an IPv4 header has been parsed
# and checksummed.  So this is the piece worth settling first.
#
# ## Why the checksum is the centre of it
#
# RFC 1122 §3.2.1.2 makes it a MUST that a datagram with a bad header checksum
# is silently discarded.  That is the requirement a driver which "looks like it
# works" fails most quietly: a program that never computes the checksum at all
# behaves identically on every well-formed frame.  So one frame in this check
# carries a deliberately corrupted IPv4 header checksum, and the *absence* of a
# reply is the evidence that the arithmetic in `योगफलम्` actually ran.  The probe
# verifies its own corruption is genuinely invalid before sending it, because a
# control that is accidentally well-formed proves the opposite of what it claims.
#
# ## Why the frames are real frames
#
# The guest is not asked what it thinks happened.  `-netdev socket` hands QEMU a
# TCP connection carrying raw Ethernet frames with a four-byte big-endian length
# in front of each, and the probe on the other end of it writes the request
# bytes and reads the reply bytes.  Every field of the reply is then checked by
# the probe's own arithmetic — the IPv4 header checksum and the ICMP checksum
# are recomputed host-side and must verify — so "answered a ping" means a frame
# that a real peer would accept, not a serial line that says so.
#
# ## Why the payload is random
#
# The ICMP identifier, sequence number and 56 payload bytes are drawn fresh on
# every run and must come back byte for byte.  A program that printed a
# plausible tag, or replayed a canned frame, cannot produce them.  The guest
# also prints the identifier and sequence it saw, in hex, on its serial line —
# so the same number is witnessed twice, once off the wire and once out of the
# machine, and the two have to agree.
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "$here/.." && pwd)
prog=$root/spec/network.sas
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# 77 is SKIPPED.  A check that could not run has not passed, and reporting it as
# 0 is how a missing tool becomes a green row.
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }
command -v python3 >/dev/null 2>&1 || {
  echo "SKIPPED: python3 is not installed; it is the host end of the frame socket"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

# The assembler is `cargo run -p sadhana`, like every other check in this tree.
#
# This was drafted to find the assembler via $SADHANA or a `sadhana` on PATH,
# and to report SKIPPED when it found neither. In THIS tree it would find
# neither: nothing installs `sadhana` on PATH, so the check would have exited
# 77 on every run, for ever, and 77 is a step the gate accepts. A check that
# quietly never runs is the exact shape of the failure that reopened this row —
# evidence that was never evidence — so the skip path is removed rather than
# left as the default. The assembler is in the tree; there is nothing to skip.

[ -f "$prog" ] || { echo "RED, as written: $prog does not exist."; exit 1; }

fail=0
note() { echo "  FAIL  $*"; fail=1; }
same() {  # $1 = what to call it, $2 = got, $3 = wanted
    [ "$2" = "$3" ] || note "$1
        got    $2
        wanted $3"
}

# Assembly failure is reported as itself, with the assembler's own words, rather
# than being let fall into `set -e` where the wrong line is never named.
assemble() {  # $1 = link address, $2 = output elf
    if ! cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
            --स्थान "$1" "$prog" "$2" > "$tmp/asm.log" 2>&1; then
        echo "RED: network.sas did not assemble at $1 —"
        sed 's/^/    /' "$tmp/asm.log"
        exit 1
    fi
}

# ---------------------------------------------------------------------------
# The host end of the wire.  Speaks QEMU's socket-netdev framing: each frame is
# preceded by its length as a four-byte big-endian integer.
# ---------------------------------------------------------------------------
cat > "$tmp/probe.py" <<'PROBE_PY'
import os, random, socket, struct, sys, time

GUEST_IP="10.0.2.15"; PROBE_IP="10.0.2.2"; OTHER_IP="10.0.2.99"
PROBE_MAC=bytes.fromhex("525400aabbcc"); BCAST=b"\xff"*6
portfile=sys.argv[1]

def ip2b(s): return bytes(int(x) for x in s.split("."))

def csum(b):
    if len(b)%2: b+=b"\x00"
    s=sum(struct.unpack("!%dH"%(len(b)//2),b))
    while s>>16: s=(s&0xffff)+(s>>16)
    return (~s)&0xffff

def eth(d,s,t,p):
    f=d+s+struct.pack("!H",t)+p
    return f+b"\x00"*(60-len(f)) if len(f)<60 else f

class Link:
    def __init__(s,sk): s.s=sk; s.buf=b""
    def send(s,f): s.s.sendall(struct.pack("!I",len(f))+f)
    def recv(s,timeout):
        end=time.monotonic()+timeout
        while True:
            if len(s.buf)>=4:
                n=struct.unpack("!I",s.buf[:4])[0]
                if n>65536: raise RuntimeError("bad framing on the socket netdev")
                if len(s.buf)>=4+n:
                    f=s.buf[4:4+n]; s.buf=s.buf[4+n:]; return f
            left=end-time.monotonic()
            if left<=0: return None
            s.s.settimeout(left)
            try: d=s.s.recv(65536)
            except socket.timeout: return None
            if not d: return None
            s.buf+=d

def arp_req(t):
    b=struct.pack("!HHBBH",1,0x0800,6,4,1)+PROBE_MAC+ip2b(PROBE_IP)+b"\x00"*6+ip2b(t)
    return eth(BCAST,PROBE_MAC,0x0806,b)

def echo(dm,dip,i,q,pay,bad=False):
    ic=struct.pack("!BBHHH",8,0,0,i,q)+pay
    ic=ic[:2]+struct.pack("!H",csum(ic))+ic[4:]
    ip=struct.pack("!BBHHHBBH",0x45,0,20+len(ic),0x1234,0,64,1,0)+ip2b(PROBE_IP)+ip2b(dip)
    c=csum(ip)
    # One bit in the high byte.  Not `^0xffff`: in ones-complement arithmetic 0
    # and 0xffff are the same value, so that flip would be a no-op on a header
    # whose checksum happened to be 0 and the control would silently pass.
    if bad: c^=0x0100
    ip=ip[:10]+struct.pack("!H",c)+ip[12:]
    return eth(dm,PROBE_MAC,0x0800,ip+ic)

out=[]
def say(k,v): out.append("PROBE %s %s"%(k,v))
def done():
    print("\n".join(out)); sys.exit(0)

srv=socket.socket(); srv.setsockopt(socket.SOL_SOCKET,socket.SO_REUSEADDR,1)
srv.bind(("127.0.0.1",0)); srv.listen(1)
# Written under a temporary name and renamed, so the shell never reads a
# half-written port number and hands QEMU a truncated one.
open(portfile+".tmp","w").write(str(srv.getsockname()[1]))
os.rename(portfile+".tmp",portfile)

srv.settimeout(40)
try: c,_=srv.accept()
except socket.timeout:
    say("FAIL","qemu never connected to the frame socket"); done()
lk=Link(c)

def drain():
    while lk.recv(0.30) is not None: pass

# ---- 1. ARP, retried until the guest driver is up --------------------------
# The retry is not politeness: QEMU connects this socket before the guest has
# booted, and a single request sent into that window is simply gone.  Retrying
# is also what a real host does.
gmac=None; deadline=time.monotonic()+30
while time.monotonic()<deadline and gmac is None:
    lk.send(arp_req(GUEST_IP))
    t=time.monotonic()+0.6
    while time.monotonic()<t:
        f=lk.recv(max(0.05,t-time.monotonic()))
        if f is None: break
        if len(f)>=42 and f[12:14]==b"\x08\x06" and f[20:22]==b"\x00\x02" \
           and f[28:32]==ip2b(GUEST_IP) and f[38:42]==ip2b(PROBE_IP):
            gmac=f[22:28]
            # The Ethernet source and the ARP sender-hardware-address must be
            # the same six bytes.  A reply that disagrees with itself would
            # still "work" against a lenient peer and is wrong.
            say("ARP-ETH-SRC-EQUALS-SHA","1" if f[6:12]==gmac else "0")
            say("ARP-TARGET-IS-PROBE","1" if f[32:38]==PROBE_MAC else "0")
            break
if gmac is None:
    say("FAIL","no ARP reply from %s within 30s"%GUEST_IP); done()
say("ARP-REPLY-MAC",":".join("%02x"%b for b in gmac))

# ---- 2. the ping ----------------------------------------------------------
def ping(tag,ident,seq,pay,dst_ip=GUEST_IP,bad=False,expect=True,wait=8.0):
    drain()
    lk.send(echo(gmac,dst_ip,ident,seq,pay,bad))
    end=time.monotonic()+wait
    while time.monotonic()<end:
        f=lk.recv(max(0.05,end-time.monotonic()))
        if f is None: continue
        if len(f)<34 or f[12:14]!=b"\x08\x00": continue
        ip=f[14:34]
        if ip[9]!=1: continue
        if not expect:
            say(tag+"-SILENT","0")
            say("FAIL",tag+": the guest answered a frame it was required to drop")
            return None
        tl=struct.unpack("!H",ip[2:4])[0]
        ic=f[34:14+tl]
        ok=1
        def bad_(why):
            say("FAIL",tag+": "+why)
        if f[0:6]!=PROBE_MAC: ok=0; bad_("reply destination MAC is not the probe")
        if f[6:12]!=gmac:     ok=0; bad_("reply source MAC is not the guest MAC")
        if ip[0]!=0x45:       ok=0; bad_("reply version/IHL is not 0x45")
        if csum(ip)!=0:       ok=0; bad_("reply IPv4 header checksum does not verify")
        if ip[12:16]!=ip2b(GUEST_IP): ok=0; bad_("reply source IP is not the guest")
        if ip[16:20]!=ip2b(PROBE_IP): ok=0; bad_("reply destination IP is not the probe")
        if ip[8]==0:          ok=0; bad_("reply TTL is zero")
        if len(ic)<8 or ic[0]!=0 or ic[1]!=0: ok=0; bad_("reply is not ICMP type 0 code 0")
        if csum(ic)!=0:       ok=0; bad_("reply ICMP checksum does not verify")
        if len(ic)>=8 and struct.unpack("!HH",ic[4:8])!=(ident,seq):
            ok=0; bad_("identifier and sequence were not echoed")
        if ic[8:]!=pay:       ok=0; bad_("payload was not echoed byte for byte")
        say(tag+"-OK",str(ok))
        say(tag+"-TTL",str(ip[8]))
        return f
    if expect:
        say(tag+"-OK","0"); say("FAIL",tag+": no echo reply within %gs"%wait)
    else:
        say(tag+"-SILENT","1")
    return None

i1=random.randrange(1,0xffff); q1=random.randrange(1,0xffff)
pay1=bytes(random.randrange(256) for _ in range(56))
say("ECHO-IDSEQ-SENT","%04x%04x"%(i1,q1))
ping("ECHO",i1,q1,pay1)

# ---- 3. three frames that MUST NOT be answered ----------------------------
b=echo(gmac,GUEST_IP,i1,q1,pay1,bad=True)
say("CTL-BADCSUM-REALLY-INVALID","1" if csum(b[14:34])!=0 else "0")
ping("CTL-BADCSUM",i1,q1,pay1,bad=True,expect=False,wait=3.0)
ping("CTL-WRONGIP",i1,q1,pay1,dst_ip=OTHER_IP,expect=False,wait=3.0)

drain(); lk.send(arp_req(OTHER_IP))
seen=0; end=time.monotonic()+3.0
while time.monotonic()<end:
    f=lk.recv(max(0.05,end-time.monotonic()))
    if f is None: continue
    if len(f)>=42 and f[12:14]==b"\x08\x06" and f[20:22]==b"\x00\x02": seen=1
say("CTL-ARP-OTHER-SILENT","0" if seen else "1")
if seen: say("FAIL","the guest answered an ARP for an address that is not its own")

# ---- 4. still alive: the silence above was selective, not death ------------
i2=random.randrange(1,0xffff); q2=random.randrange(1,0xffff)
pay2=bytes(random.randrange(256) for _ in range(56))
say("ECHO2-IDSEQ-SENT","%04x%04x"%(i2,q2))
ping("ECHO2",i2,q2,pay2)
say("DISTINCT-IDSEQ","1" if (i1,q1)!=(i2,q2) else "0")
done()
PROBE_PY

# ---------------------------------------------------------------------------
# One run: a probe and a whole QEMU process, both gone by the time it returns.
# ---------------------------------------------------------------------------
run() {  # $1 = elf, $2 = mac to give the device, $3 = tag for the output files
    elf=$1; mac=$2; where=$3
    rm -f "$tmp/port"

    python3 "$tmp/probe.py" "$tmp/port" > "$tmp/probe-$where.out" 2>"$tmp/probe-$where.err" &
    ppid=$!

    # The probe must be listening before QEMU is told to connect: `connect=` does
    # not retry, and a QEMU that cannot reach the port exits immediately.
    n=0
    while [ ! -s "$tmp/port" ] && [ "$n" -lt 20 ]; do n=$((n + 1)); sleep 1; done
    port=$(cat "$tmp/port" 2>/dev/null || true)
    case "$port" in
        ''|*[!0-9]*) note "$where: the probe never published a port number"; return ;;
    esac

    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$elf" \
        -netdev socket,id=n0,connect=127.0.0.1:"$port" \
        -device virtio-net-device,netdev=n0,mac="$mac" > "$tmp/raw-$where" 2>&1 &
    qpid=$!

    # Bounded by an explicit SIGKILL from a background `sleep`, not by an alarm:
    # QEMU runs its own timer subsystem and survives SIGALRM.  A driver that
    # mis-programs a queue spins forever on the used ring, and an unbounded wait
    # here would hang the check rather than fail it.  The watcher's own stderr is
    # closed off — the SUBSHELL is what announces "Terminated: 15  sleep 120",
    # and that word landing mid-report reads as a failure to the next reader.
    ( sleep 120; kill -9 "$qpid" 2>/dev/null ) 2>/dev/null & watcher=$!

    wait "$ppid" 2>/dev/null || true
    kill -9 "$qpid" 2>/dev/null || true
    wait "$qpid" 2>/dev/null || true
    # `pkill -P` FIRST: killing the subshell alone orphans its `sleep` to init.
    # By PARENT pid, never by pattern — another agent's QEMU has been seen live
    # on this host and a pattern kill would take it down.
    pkill -P "$watcher" 2>/dev/null || true
    kill "$watcher" 2>/dev/null || true
    wait "$watcher" 2>/dev/null || true

    # QEMU's serial line ends every line with CR.  Without stripping it nothing
    # matches, which looks exactly like a program that printed nothing at all.
    # LC_ALL=C because macOS `tr` aborts on invalid UTF-8 and TRUNCATES the
    # stream, and OpenSBI's banner is not guaranteed clean.
    LC_ALL=C tr -d '\r' < "$tmp/raw-$where" > "$tmp/out-$where"
}

probe() {  # $1 = where, $2 = key -> the value the probe reported, or empty
    sed -n "s/^PROBE $2 //p" "$tmp/probe-$1.out" 2>/dev/null | tail -1
}
serial() {  # $1 = where, $2 = extended regexp -> matching serial lines
    grep -aE "$2" "$tmp/out-$1" 2>/dev/null || true
}
count() {  # $1 = where, $2 = exact tag -> how many times the guest said it
    grep -ac "^$2\$" "$tmp/out-$1" 2>/dev/null || true
}

scenario() {  # $1 = elf, $2 = mac, $3 = label
    elf=$1; mac=$2; where=$3
    echo "  [$where] booting, with mac=$mac on the wire"
    run "$elf" "$mac" "$where"

    # The probe's own failures first: they name the wire fact that went wrong.
    if [ -s "$tmp/probe-$where.err" ]; then
        note "$where: the probe died —
$(sed 's/^/        /' "$tmp/probe-$where.err")"
        return
    fi
    # Guarded with `if` rather than `&&`: under `set -e` a bare `grep` that finds
    # nothing is the last command of the function and would end the script with
    # a clean exit code, turning "the probe reported nothing wrong" into a pass
    # that skipped every assertion below.
    if sed -n 's/^PROBE FAIL //p' "$tmp/probe-$where.out" | grep -aq .; then
        sed -n 's/^PROBE FAIL /  FAIL  '"$where"': /p' "$tmp/probe-$where.out"
        fail=1
    fi

    # A named failure mode is reported as itself rather than as a missing reply.
    for tag in NET-NO-DEVICE NET-NO-MAC NET-NO-RXQUEUE NET-NO-TXQUEUE; do
        if [ "$(count "$where" "$tag")" != "0" ]; then
            note "$where: the driver reported $tag and never came up"
        fi
    done

    # ---- the property ----------------------------------------------------
    same "$where the driver came up"        "$(count "$where" NET-UP)" "1"
    # The hardware address is read out of the device's config space, not built
    # into the program.  If it does not match what -device was told, the driver
    # is not reading config and every reply it sends carries someone else's
    # address.
    flat=$(printf '%s' "$mac" | tr -d ':')
    same "$where read its MAC from config space" \
         "$(serial "$where" '^[0-9a-f]{16}$' | head -1)" "0000$flat"
    same "$where ARP reply carried that MAC"    "$(probe "$where" ARP-REPLY-MAC)" "$mac"
    same "$where ARP reply agrees with itself"  "$(probe "$where" ARP-ETH-SRC-EQUALS-SHA)" "1"
    same "$where ARP reply is addressed to the probe" \
         "$(probe "$where" ARP-TARGET-IS-PROBE)" "1"
    same "$where said NET-ARP-REPLY"            "$(count "$where" NET-ARP-REPLY)" "1"

    # The echo reply, every field of it recomputed host-side.
    same "$where answered the ping"             "$(probe "$where" ECHO-OK)" "1"
    same "$where answered the second ping"      "$(probe "$where" ECHO2-OK)" "1"
    same "$where reset the TTL to 64"           "$(probe "$where" ECHO-TTL)" "64"
    same "$where said NET-ICMP-ECHO-REPLY twice" \
         "$(count "$where" NET-ICMP-ECHO-REPLY)" "2"

    # The same random number witnessed twice: once off the wire by the probe,
    # once out of the machine on its serial line.  A canned reply cannot make
    # these agree, and neither can a serial line that is merely optimistic.
    s1=$(probe "$where" ECHO-IDSEQ-SENT); s2=$(probe "$where" ECHO2-IDSEQ-SENT)
    same "$where the two runs used different identifiers" \
         "$(probe "$where" DISTINCT-IDSEQ)" "1"
    same "$where printed the identifier and sequence it received" \
         "$(serial "$where" '^[0-9a-f]{16}$' | sed -n 2p)" "00000000$s1"
    same "$where printed the second identifier and sequence" \
         "$(serial "$where" '^[0-9a-f]{16}$' | sed -n 3p)" "00000000$s2"

    # ---- the controls ----------------------------------------------------
    # Each of these would pass vacuously if the guest answered everything, and
    # each is paired with the guest naming the reason on its serial line — so a
    # silence caused by a crash is told apart from a silence that is a decision.
    same "$where the corrupted checksum really was invalid" \
         "$(probe "$where" CTL-BADCSUM-REALLY-INVALID)" "1"
    same "$where dropped the frame with a bad IPv4 checksum (RFC 1122 3.2.1.2)" \
         "$(probe "$where" CTL-BADCSUM-SILENT)" "1"
    same "$where NAMED the bad checksum"        "$(count "$where" NET-IPV4-BAD-CHECKSUM)" "1"
    same "$where dropped the frame for another address" \
         "$(probe "$where" CTL-WRONGIP-SILENT)" "1"
    same "$where NAMED the foreign address"     "$(count "$where" NET-IPV4-NOT-FOR-US)" "1"
    same "$where ignored an ARP for another address" \
         "$(probe "$where" CTL-ARP-OTHER-SILENT)" "1"
    same "$where NAMED the foreign ARP"         "$(count "$where" NET-ARP-NOT-FOR-US)" "1"
}

echo "Assembling at two link addresses..."
assemble ०षोड्८०२००००० "$tmp/n-a.elf"
assemble ०षोड्८०४००००० "$tmp/n-b.elf"

echo
echo "Two boots, each a separate QEMU process with a raw Ethernet socket on it."
echo
# Two link addresses, as check-journal.sh does and for the same reason: the
# queue area, the message strings and every branch target are built with auipc,
# and an address that was a constant which happened to be right at 0x80200000
# stops being right at 0x80400000.  The two MACs differ so that a program which
# had the address baked in rather than read from config space fails here.
scenario "$tmp/n-a.elf" 52:54:00:ab:cd:ef 0x80200000
scenario "$tmp/n-b.elf" 52:54:00:12:34:56 0x80400000
echo

[ "$fail" -eq 0 ] || {
    echo "the machine does not answer a ping."
    exit 1; }

echo "ok  the machine answers a ping."
echo
echo "    At each of two link addresses, with a different hardware address on"
echo "    the wire, over a raw Ethernet frame socket into QEMU's virtio-net:"
echo
echo "      ARP     a who-has for 10.0.2.15 is answered, with the MAC the"
echo "              driver read out of the device's config space — not one"
echo "              compiled into the program."
echo "      IPv4    a header is parsed, its checksum verified, and its"
echo "              destination matched against our own address."
echo "      ICMP    an echo request is answered with an echo reply whose IPv4"
echo "              and ICMP checksums the probe recomputes and verifies, whose"
echo "              TTL is reset to 64, and whose identifier, sequence and 56"
echo "              random payload bytes come back byte for byte.  The same"
echo "              identifier is printed on the serial line, so the number is"
echo "              witnessed on the wire and inside the machine both."
echo
echo "    And three frames that must NOT be answered, each unanswered AND named"
echo "    by the guest as the reason:"
echo
echo "      a corrupted IPv4 header checksum   -> NET-IPV4-BAD-CHECKSUM"
echo "      a datagram for 10.0.2.99           -> NET-IPV4-NOT-FOR-US"
echo "      an ARP who-has for 10.0.2.99       -> NET-ARP-NOT-FOR-US"
echo
echo "    A second ping after those three is answered, so the silence was a"
echo "    decision and not a dead machine."
echo
echo "    This is ONE slice of row C-011, not the row. There is no TCP here: no"
echo "    handshake, no sequence numbers, no retransmission, no window. No TLS"
echo "    1.3, no HTTP/1.1, no IPv6, no fragment reassembly, no IP options, no"
echo "    UDP. What is settled is that frames move both ways over virtio-net,"
echo "    that ARP and ICMP echo are answered for our address and only ours,"
echo "    and that a bad IPv4 header checksum is discarded rather than trusted."
