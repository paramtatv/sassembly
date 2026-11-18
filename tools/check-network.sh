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

# Host-end deps of the stream netdev wire: nc with unix-socket (-U) support, and
# a QEMU that has the `stream` netdev.  Missing or unsuitable = 77, not FAIL.
command -v nc >/dev/null 2>&1 || {
  echo "SKIP nc is not installed (needed for the unix-socket host end)" >&2
  exit 77; }
# Capability test, not a help grep (BSD prints -U inside a flag bundle): -U on an
# absent socket fails to connect; an nc without -U rejects the option itself.
if nc -U "$tmp/no-such.sock" </dev/null 2>&1 | grep -aEqi 'illegal option|invalid option|unknown option|usage:'; then
  echo "SKIP nc lacks -U (unix-socket) support" >&2
  exit 77
fi
qemu-system-riscv64 -netdev help 2>&1 | grep -qw stream || {
  echo "SKIP qemu-system-riscv64 lacks the stream netdev (needs QEMU >= 7.2)" >&2
  exit 77; }

# The assembler is tools/sadhana-native.sh (no Rust); its exit 77 (no Stage 1) is this check's 77.
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
    asm_rc=0  # set -e: capture the rc instead of dying on it
    "$root/tools/sadhana-native.sh" \
            --स्थान "$1" "$prog" "$2" > "$tmp/asm.log" 2>&1 || asm_rc=$?
    if [ "$asm_rc" -eq 77 ]; then
        echo "SKIPPED: sadhana-native.sh exited 77 (no Stage 1 / wrapper image)"
        exit 77
    fi
    if [ "$asm_rc" -ne 0 ]; then
        echo "RED: network.sas did not assemble at $1 —"
        sed 's/^/    /' "$tmp/asm.log"
        exit 1
    fi
}

# ---------------------------------------------------------------------------
# The host end of the wire.  Speaks QEMU's stream-netdev framing: each frame is
# preceded by its length as a four-byte big-endian integer.  QEMU listens on a
# unix socket; `nc -U` is the host end, fed and read through two fifos.  Every
# frame is built and checked in awk on hex strings; bytes go out through octal
# `printf` escapes and come back through `od`.
#
# No timers.  Each wait is an event: the guest's NET-UP line opens the exchange,
# a reply closes each step, and "must not be answered" is settled by ORDER — the
# guest handles frames one at a time, so when the reply to the frame sent LAST
# arrives, anything it was going to say about the earlier ones has come first.
# A guest that never answers ends the wait when the SIGKILL watcher below kills
# QEMU and the socket closes.
# ---------------------------------------------------------------------------
cat > "$tmp/probe.awk" <<'PROBE_AWK'
function h2n(s,  i,n){n=0;for(i=1;i<=length(s);i++)n=n*16+index("0123456789abcdef",substr(s,i,1))-1;return n}
function n2h(n,w){return sprintf("%0" w "x",n)}
function ip2h(s,  a,r,i){split(s,a,".");r="";for(i=1;i<=4;i++)r=r sprintf("%02x",a[i]);return r}
function csum(h,  i,s){
    if(length(h)%4)h=h "00"
    s=0
    for(i=1;i<=length(h);i+=4)s+=h2n(substr(h,i,4))
    while(s>=65536)s=(s%65536)+int(s/65536)
    return 65535-s
}
function eth(d,s,t,p,  f){f=d s t p;while(length(f)<120)f=f "00";return f}
function echo(dm,dip,i,q,pay,bad,  ic,ip,c){
    ic="0800" "0000" i q pay
    c=csum(ic)
    ic="0800" n2h(c,4) i q pay
    ip="4500" n2h(20+length(ic)/2,4) "1234" "0000" "4001" "0000" ip2h(PROBE_IP) ip2h(dip)
    c=csum(ip)
    # One bit in the high byte.  Not `^0xffff`: in ones-complement arithmetic 0
    # and 0xffff are the same value, so that flip would be a no-op on a header
    # whose checksum happened to be 0 and the control would silently pass.
    if(bad){if(int(c/256)%2)c-=256;else c+=256}
    ip=substr(ip,1,20) n2h(c,4) substr(ip,25)
    return eth(dm,PROBE_MAC,"0800",ip ic)
}
function arpreq(t){
    return eth("ffffffffffff",PROBE_MAC,"0806","0001" "0800" "06" "04" "0001" PROBE_MAC ip2h(PROBE_IP) "000000000000" ip2h(t))
}
function chk(why){if(tag!="")print "PROBE FAIL " tag ": " why; ok=0}
BEGIN{
    GUEST_IP="10.0.2.15";PROBE_IP="10.0.2.2";OTHER_IP="10.0.2.99";PROBE_MAC="525400aabbcc"
    f=a1
    if(m=="arpreq")print arpreq(a1)
    else if(m=="arpother")print arpreq(OTHER_IP)
    else if(m=="echo")print echo(a1,a2,a3,a4,a5,a6+0)
    else if(m=="wire"){
        h=n2h(length(a1)/2,8) a1;r=""
        for(i=1;i<=length(h);i+=2)r=r sprintf("\\%03o",h2n(substr(h,i,2)))
        printf "%s",r
    }
    else if(m=="badvalid")print (csum(substr(a1,29,40))!=0)?1:0
    else if(m=="succ")print n2h((h2n(a1)+1)%65536,4)
    else if(m=="chkarp"){
        f=a1
        if(length(f)>=84&&substr(f,25,4)=="0806"&&substr(f,41,4)=="0002"&&substr(f,57,8)==ip2h(GUEST_IP)&&substr(f,77,8)==ip2h(PROBE_IP)){
            g=substr(f,45,12)
            print 1,g,(substr(f,13,12)==g)?1:0,(substr(f,65,12)==PROBE_MAC)?1:0
        } else print 0
    }
    else if(m=="classify"){
        f=a1
        if(length(f)>=84&&substr(f,25,4)=="0806"&&substr(f,41,4)=="0002")print "arp"
        else if(length(f)>=84&&substr(f,25,4)=="0800"&&substr(f,47,2)=="01")print "icmp",substr(f,77,8)
        else print "skip"
    }
    else if(m=="chkecho"){
        # a1 = frame, a2 = ident, a3 = seq, a4 = payload, a5 = guest MAC; tag = the probe's name for it
        f=a1;gm=a5;ok=1
        tl=h2n(substr(f,33,4))
        ip=substr(f,29,40)
        ic=substr(f,69,2*(tl-20))
        if(substr(f,1,12)!=PROBE_MAC)chk("reply destination MAC is not the probe")
        if(substr(f,13,12)!=gm)chk("reply source MAC is not the guest MAC")
        if(substr(ip,1,2)!="45")chk("reply version/IHL is not 0x45")
        if(csum(ip)!=0)chk("reply IPv4 header checksum does not verify")
        if(substr(ip,25,8)!=ip2h(GUEST_IP))chk("reply source IP is not the guest")
        if(substr(ip,33,8)!=ip2h(PROBE_IP))chk("reply destination IP is not the probe")
        if(substr(ip,17,2)=="00")chk("reply TTL is zero")
        if(length(ic)<16||substr(ic,1,2)!="00"||substr(ic,3,2)!="00")chk("reply is not ICMP type 0 code 0")
        if(csum(ic)!=0)chk("reply ICMP checksum does not verify")
        if(length(ic)>=16&&substr(ic,9,8)!=a2 a3)chk("identifier and sequence were not echoed")
        if(substr(ic,17)!=a4)chk("payload was not echoed byte for byte")
        print "PROBE " tag "-OK " ok
        print "PROBE " tag "-TTL " h2n(substr(ip,17,2))
    }
    exit 0
}
PROBE_AWK

# ---------------------------------------------------------------------------
# One run: a probe and a whole QEMU process, both gone by the time it returns.
# ---------------------------------------------------------------------------
trap '' PIPE   # a write to a dead socket is a failed step, not a dead script
A() {  # $1 = mode, then up to six arguments -> awk's answer
    m=$1; shift
    awk -v m="$m" -v tag="${tag:-}" -v a1="${1:-}" -v a2="${2:-}" -v a3="${3:-}" \
        -v a4="${4:-}" -v a5="${5:-}" -v a6="${6:-}" -f "$tmp/probe.awk"
}
say() { printf 'PROBE %s %s\n' "$1" "$2" >> "$pout"; }
tx() {  # $1 = frame as hex -> the frame on the wire, length first
    fmt=$(A wire "$1")
    # the format IS the bytes, every one of them an octal escape
    # shellcheck disable=SC2059
    printf "$fmt" >&5 || true
}
rx() {  # one frame off the wire -> $fr as hex; false when the socket has closed
    fr=
    h=$(dd bs=1 count=4 <&6 2>/dev/null | od -An -v -tx1 | tr -d ' \n')
    [ "${#h}" -eq 8 ] || return 1
    n=$((0x$h))
    [ "$n" -le 65536 ] || { echo "bad framing on the socket netdev" >> "$perr"; return 1; }
    fr=$(dd bs=1 count="$n" <&6 2>/dev/null | od -An -v -tx1 | tr -d ' \n')
    [ "${#fr}" -eq $((2 * n)) ]
}
rnd() {  # -> $r1 = identifier, $r2 = sequence (never 0), $r3 = 56 payload bytes
    x=$(od -An -v -N60 -tx1 /dev/urandom | tr -d ' \n')
    [ "${#x}" -eq 120 ] || { echo "no random bytes from /dev/urandom" >> "$perr"; return 1; }
    r1=$(printf '%s' "$x" | cut -c1-4); r2=$(printf '%s' "$x" | cut -c5-8)
    r3=$(printf '%s' "$x" | cut -c9-120)
    [ "$r1" != 0000 ] || r1=0001
    [ "$r2" != 0000 ] || r2=0001
}

drive() {  # $1 = where; the whole exchange, ending with every PROBE line written
    where=$1; gip=10.0.2.15
    # ---- 0. the guest says it is up -----------------------------------------
    up=0
    while IFS= read -r l <&4; do
        printf '%s\n' "$l" >> "$tmp/raw-$where"
        case $l in *NET-UP*) up=1; break ;; esac
    done
    cat <&4 >> "$tmp/raw-$where" & catpid=$!
    [ "$up" -eq 1 ] || { say FAIL "no ARP reply from $gip within 30s"; return; }
    [ -S "$sock" ] || { say FAIL "qemu never connected to the frame socket"; return; }
    nc -U "$sock" < "$tmp/tx-$where" > "$tmp/rx-$where" 2>/dev/null & ncpid=$!
    exec 5> "$tmp/tx-$where"
    exec 6< "$tmp/rx-$where"

    # ---- 1. ARP -------------------------------------------------------------
    tag=
    tx "$(A arpreq "$gip")"
    got=
    while rx; do
        c=$(A chkarp "$fr")
        case $c in 1\ *) got=$c; break ;; esac
    done
    [ -n "$got" ] || { say FAIL "no ARP reply from $gip within 30s"; return; }
    set -- $got
    gmac=$2
    say ARP-ETH-SRC-EQUALS-SHA "$3"
    say ARP-TARGET-IS-PROBE "$4"
    say ARP-REPLY-MAC "$(printf '%s' "$gmac" | sed 's/../&:/g; s/:$//')"

    # ---- 2. the ping --------------------------------------------------------
    rnd || return; i1=$r1; q1=$r2; pay1=$r3
    say ECHO-IDSEQ-SENT "$i1$q1"
    tx "$(A echo "$gmac" "$gip" "$i1" "$q1" "$pay1" 0)"
    found=
    while rx; do
        c=$(A classify "$fr")
        case $c in icmp*) found=$fr; break ;; esac
    done
    if [ -n "$found" ]; then
        tag=ECHO A chkecho "$found" "$i1" "$q1" "$pay1" "$gmac" >> "$pout"
    else
        say ECHO-OK 0; say FAIL "ECHO: no echo reply within 8s"
    fi

    # ---- 3. three frames that MUST NOT be answered, then one that must --------
    rnd || return; i2=$r1; q2=$r2; pay2=$r3
    qw=$(A succ "$q1")
    b=$(A echo "$gmac" "$gip" "$i1" "$q1" "$pay1" 1)
    say CTL-BADCSUM-REALLY-INVALID "$(A badvalid "$b")"
    tx "$b"
    tx "$(A echo "$gmac" 10.0.2.99 "$i1" "$qw" "$pay1" 0)"
    tx "$(A arpother)"
    # ---- 4. still alive: the silence above was selective, not death ----------
    say ECHO2-IDSEQ-SENT "$i2$q2"
    tx "$(A echo "$gmac" "$gip" "$i2" "$q2" "$pay2" 0)"
    bs=1; ws=1; as=1; found=
    while rx; do
        c=$(A classify "$fr")
        case $c in
            arp) as=0 ;;
            "icmp $i2$q2") found=$fr; break ;;
            "icmp $i1$q1") bs=0 ;;
            "icmp $i1$qw") ws=0 ;;
        esac
    done
    say CTL-BADCSUM-SILENT "$bs"
    [ "$bs" -eq 1 ] || say FAIL "CTL-BADCSUM: the guest answered a frame it was required to drop"
    say CTL-WRONGIP-SILENT "$ws"
    [ "$ws" -eq 1 ] || say FAIL "CTL-WRONGIP: the guest answered a frame it was required to drop"
    say CTL-ARP-OTHER-SILENT "$as"
    [ "$as" -eq 1 ] || say FAIL "the guest answered an ARP for an address that is not its own"
    if [ -n "$found" ]; then
        tag=ECHO2 A chkecho "$found" "$i2" "$q2" "$pay2" "$gmac" >> "$pout"
    else
        say ECHO2-OK 0; say FAIL "ECHO2: no echo reply within 8s"
    fi
    if [ "$i1$q1" != "$i2$q2" ]; then say DISTINCT-IDSEQ 1; else say DISTINCT-IDSEQ 0; fi
}

run() {  # $1 = elf, $2 = mac to give the device, $3 = tag for the output files
    elf=$1; mac=$2; where=$3
    pout=$tmp/probe-$where.out; perr=$tmp/probe-$where.err
    : > "$pout"; : > "$perr"; : > "$tmp/raw-$where"
    sock=$tmp/s-$where
    # sun_path is 104 bytes on macOS; a longer name is a bind that cannot happen
    [ "${#sock}" -lt 100 ] || {
        echo "SKIPPED: \$TMPDIR is too long for a unix socket name: $sock"
        echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
        exit 77; }
    mkfifo "$tmp/ser-$where" "$tmp/tx-$where" "$tmp/rx-$where" || {
        echo "SKIPPED: mkfifo failed in $tmp"
        echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
        exit 77; }
    qpid=; ncpid=; catpid=

    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$elf" \
        -netdev stream,id=n0,addr.type=unix,addr.path="$sock",server=on \
        -device virtio-net-device,netdev=n0,mac="$mac" > "$tmp/ser-$where" 2>&1 &
    qpid=$!
    exec 4< "$tmp/ser-$where"

    # Bounded by an explicit SIGKILL from a background `sleep`, not by an alarm:
    # QEMU runs its own timer subsystem and survives SIGALRM.  A driver that
    # mis-programs a queue spins forever on the used ring, and an unbounded wait
    # here would hang the check rather than fail it.  The watcher's own stderr is
    # closed off — the SUBSHELL is what announces "Terminated: 15  sleep 120",
    # and that word landing mid-report reads as a failure to the next reader.
    ( sleep 120; kill -9 "$qpid" 2>/dev/null ) 2>/dev/null & watcher=$!

    LC_ALL=C drive "$where"
    exec 5>&- 6<&- 4<&-
    kill -9 "$qpid" 2>/dev/null || true
    wait "$qpid" 2>/dev/null || true
    [ -z "$ncpid" ] || kill "$ncpid" 2>/dev/null || true
    [ -z "$ncpid" ] || wait "$ncpid" 2>/dev/null || true
    [ -z "$catpid" ] || wait "$catpid" 2>/dev/null || true
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
