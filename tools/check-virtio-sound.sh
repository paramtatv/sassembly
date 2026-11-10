#!/bin/sh
# Sound the guest wrote comes out of the HOST — row `F-009`, doc 10 §7.
#
# ## What is being proved
#
# `virtio-sound.sas` finds a virtio-sound device (DeviceID 25) by scanning the
# virtio-mmio window, brings it up through the §3.1.1 handshake over the LEGACY
# register set (this transport offers no VIRTIO_F_VERSION_1 — measured:
# DeviceFeatures[1] is 0), sets up the control queue (0) and the tx queue (2)
# with QueuePFN/GuestPageSize/QueueAlign, and then drives the whole PCM path:
# PCM_INFO, PCM_SET_PARAMS, PCM_PREPARE, PCM_START, four periods of real frames
# down the tx queue, PCM_STOP, PCM_RELEASE.
#
# Two different things are asserted, and they fail independently:
#
#   1. THE DEVICE'S OWN ANSWER. Every command's status word is read back out of
#      the buffer the DEVICE wrote and must be 0x8000 — VIRTIO_SND_S_OK. The
#      driver fills that buffer with 0xff before every submission, so "the
#      device never touched it" reads back as ffffffff and cannot be mistaken
#      for success. Three further words come from the device's own PCM_INFO
#      description — the format bitmap, the rate bitmap and the direction —
#      and the check requires that the format and rate the driver then ASKED
#      for in SET_PARAMS (S16 = bit 5, 48000 = bit 7) are bits the device said
#      it had. "What we asked for" and "what the device offers" stay tied to
#      one measurement instead of being two independent guesses.
#
#   2. THE AUDIO. QEMU is run with `-audiodev wav`, so the host writes every
#      frame the device consumed into a WAV file, and that file is read back
#      sample by sample. The guest plays a square wave: 4096 samples of
#      alternating -2000 / +2000, flipping every 64 samples. The check requires
#      exactly that — the sample count, the two values and nothing else, 64
#      runs of exactly 64, alternating.
#
# (2) is the row. (1) alone would prove the device accepted eight commands,
# which a device will happily do for a stream nobody ever listens to. The WAV
# is written by the HOST's audio backend out of frames the device pulled from
# guest memory, and it is the only assertion here that the guest cannot fake by
# printing.
#
# ## Why the run length is the real test
#
# The 64-sample run is where the frame arithmetic becomes falsifiable. QEMU's
# mixer resamples from the stream rate the guest set to the backend rate, so a
# driver that asked for 44100 instead of 48000 still produces a two-valued
# square wave — but its runs come out 69.7 samples long, not 64, and they do
# not divide evenly. The same goes for the channel count: mono frames would
# halve every run. A histogram alone would pass all of those. This is the
# same shape of assertion as the odd pixel at (1,1) in `check-virtio-gpu.sh`.
#
# ## Controls
#
# TWO LINK ADDRESSES. The queue region, the command page, the status buffer and
# the PCM frames are all computed with `auipc` and handed to the DEVICE as guest
# physical addresses. Running at 0x80200000 and 0x80400000 is what shows those
# addresses are computed rather than constants that happened to be right once.
#
# A NEGATIVE RUN. The same ELF is booted with no virtio-sound device attached at
# all, and is required to print VIRTIO-SOUND-NO-DEVICE and to print no PERIOD
# and no DONE. Without this, a program that printed its whole transcript
# unconditionally would pass everything above. It also proves the failure tags
# are reachable, so a real failure reports as itself rather than as silence.
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# NOT in the repo: the coordinating session gates on a fingerprint of the
# working tree, so a scratch file dropped there is a file that exists while the
# gate is looking, even if it is deleted a line later.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

# A QEMU without the device cannot measure this row. Found out here, up front,
# rather than by reading NO-DEVICE out of the guest and calling it a failure of
# the driver.
qemu-system-riscv64 -device help 2>&1 | grep -aq 'virtio-sound-device' || {
  echo "SKIPPED: this qemu-system-riscv64 has no virtio-sound-device."
  echo "         $(qemu-system-riscv64 --version 2>&1 | head -1)"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

prog=$(CDPATH= cd -- "$here/.." && pwd)/spec/virtio-sound.sas
[ -f "$prog" ] || { echo "RED, as written: virtio-sound.sas does not exist."; exit 1; }

# ---- the assembler ----------------------------------------------------------
# `cargo run`, and ONLY `cargo run` — the same way every other check in this
# tree assembles.
#
# This was written with a `$SADHANA` / `$here/../sadhana` fallback chain, so it
# could run while a gate held the cargo lock. That is a real convenience and it
# is removed on purpose: the fallback resolves to a path at the repository
# ROOT, and a stale binary left sitting there would make this check assemble
# with an assembler that is not the one in the tree — and still print `ok`. A
# check that can silently measure the wrong build is worse than one that has to
# wait for a lock.
#
# The assembler announces success on stderr, so its output is captured rather
# than discarded and reprinted only when it fails. `>/dev/null` alone would let
# the banner land in the middle of this report; `2>/dev/null` would throw away
# the Sanskrit diagnostic, which is the only thing that says WHY.
assemble() {  # $1 = link address, $2 = output elf
    root=$(CDPATH= cd -- "$here/.." && pwd)
    asm_rc=0
    out=$("$root/tools/sadhana-native.sh" --स्थान "$1" "$prog" "$2" 2>&1) || asm_rc=$?
    [ "$asm_rc" -eq 77 ] && { echo "  SKIP  no native assembler image (exit 77)"; exit 77; }
    [ "$asm_rc" -eq 0 ] || {
        echo "  FAIL  the program does not assemble at $1:"
        echo "$out" | sed 's/^/          /'
        exit 1; }
}

# ---- the WAV reader: what the HOST actually played --------------------------
# awk over `od` bytes: no interpreter beyond what the rest of this script uses.
wave_check() {  # $1 = wav path ; prints the report, rc 1 on any failure
    if [ ! -r "$1" ] || [ -d "$1" ]; then
        echo "  FAIL  the wav capture could not be read: [Errno 2] No such file or directory: '$1'"
        return 1
    fi
    LC_ALL=C od -An -v -t u1 "$1" | LC_ALL=C awk '
    function fail(m) { print "  FAIL  " m; bad = 1 }
    function note(m) { print "        " m }
    function lst(a, n,   i, s) { s = "["; for (i = 1; i <= n; i++) s = s (i > 1 ? ", " : "") a[i]; return s "]" }
    function s16(i,   x) { x = d[i] + 256 * d[i + 1]; return x >= 32768 ? x - 65536 : x }
    BEGIN { len = 0; AMP = 2000; RUN = 64; WANT = 4096 }
    { for (i = 1; i <= NF; i++) d[len++] = $i + 0 }
    END {
        if (len < 44 || d[0] != 82 || d[1] != 73 || d[2] != 70 || d[3] != 70 || \
            d[8] != 87 || d[9] != 65 || d[10] != 86 || d[11] != 69) {
            fail("the capture is not a RIFF/WAVE file (" len " bytes) \342\200\224 QEMU\047s wav backend")
            note("wrote nothing the device had pulled out of guest memory")
            exit 1
        }
        fmt = d[20] + 256 * d[21]; ch = d[22] + 256 * d[23]
        rate = d[24] + 256 * (d[25] + 256 * (d[26] + 256 * d[27]))
        bits = d[34] + 256 * d[35]
        n = int((len - 44) / 2)
        for (i = 0; i < n; i++) v[i] = s16(44 + 2 * i)
        print "  wav header    " rate " Hz, " ch " ch, " bits "-bit, " n * 2 " bytes of pcm"
        if (fmt != 1 || ch != 2 || bits != 16)
            fail("the capture is format " fmt ", " ch " channels, " bits " bits, not 16-bit stereo PCM")
        a = 0; b = n
        while (a < b && v[a] == 0) a++
        while (b > a && v[b - 1] == 0) b--
        pad = a + (n - b)
        if (pad) note(pad " samples of host-side silence trimmed from the edges")
        cn = b - a
        if (cn == 0) {
            fail("the capture holds no non-zero samples: the device consumed no frames")
            exit 1
        }
        for (i = 0; i < cn; i++) c[i] = v[a + i]
        nr = 0
        for (i = 0; i < cn; i++) {
            if (i == 0 || c[i] != c[i - 1]) { nr++; rv[nr] = c[i]; rl[nr] = 0 }
            rl[nr]++
        }
        nv = 0; nl = 0
        for (i = 1; i <= nr; i++) {
            if (!(rv[i] in sv)) { sv[rv[i]] = 1; va[++nv] = rv[i] + 0 }
            if (!(rl[i] in sl)) { sl[rl[i]] = 1; la[++nl] = rl[i] + 0 }
        }
        for (i = 2; i <= nv; i++) { x = va[i]; for (j = i - 1; j >= 1 && va[j] > x; j--) va[j + 1] = va[j]; va[j + 1] = x }
        for (i = 2; i <= nl; i++) { x = la[i]; for (j = i - 1; j >= 1 && la[j] > x; j--) la[j + 1] = la[j]; la[j + 1] = x }
        vs = ""; for (i = 1; i <= nv; i++) vs = vs (i > 1 ? ", " : "") va[i]
        ls = ""; for (i = 1; i <= nl; i++) ls = ls (i > 1 ? ", " : "") la[i]
        print "  samples       " cn " (" WANT " expected)"
        print "  values        " vs
        print "  runs          " nr ", lengths " ls
        print "  first sample  " c[0]
        if (cn != WANT) {
            fail("the host played " cn " samples; the guest submitted 4 periods of 2048")
            note("bytes, which is " WANT " samples of 16-bit audio")
        }
        if (!(nv == 2 && va[1] == -AMP && va[2] == AMP)) {
            fail("the played values are " lst(va, nv) ", not the -" AMP " / +" AMP " square wave the guest")
            note("stored into the buffer it handed the device")
        }
        if (!(nl == 1 && la[1] == RUN)) {
            fail("the sign flips every " lst(la, nl) " samples, not every " RUN)
            note("a run of any other length means the stream rate or the channel count")
            note("is not what SET_PARAMS asked for \342\200\224 QEMU resamples to the backend rate,")
            note("so 44100 would land here as 69.7, and mono would halve every run")
        }
        if (c[0] != -AMP) fail("the wave starts at " c[0] ", not the " (-AMP) " the guest wrote first")
        for (i = 1; i < nr; i++) if (rv[i] == rv[i + 1]) { fail("two adjacent runs carry the same value: the wave is not alternating"); break }
        exit bad
    }'
}

# ---- the empty-capture reader, for the negative run -------------------------
silent_check() {  # $1 = wav path
    LC_ALL=C od -An -v -t u1 "$1" | LC_ALL=C awk '
    BEGIN { len = 0 }
    { for (i = 1; i <= NF; i++) d[len++] = $i + 0 }
    END {
        n = len >= 44 ? int((len - 44) / 2) : 0
        loud = 0
        for (i = 0; i < n; i++) if (d[44 + 2 * i] != 0 || d[45 + 2 * i] != 0) loud++
        print "  capture       " len " bytes, " n " samples, " loud " of them non-zero"
        if (loud) {
            print "  FAIL  the negative run played " loud " non-zero samples with no virtio-sound"
            print "        device on the command line \342\200\224 the audio above is not the driver\047s."
            exit 1
        }
        exit 0
    }'
}

# One run: boot, wait for the machine to shut itself down, keep the transcript
# and the WAV.
#
# Bounded by an explicit SIGKILL from a background `sleep`, not by perl's alarm:
# QEMU runs its own timer subsystem and survives SIGALRM (`W-060`).
run() {  # $1 = elf, $2 = cleaned output, $3 = wav path, $4 = extra device args
    rm -f "$3" 2>/dev/null || true
    # timer-period=1000 (1 ms; QEMU's default is 10 ms) is what makes the
    # tail-of-stream assertion host-independent. The device returns a period
    # once its frames are in the mixer, not once they are played, and PCM_STOP
    # drops what has not played yet. A driver that STOPs on the last return
    # therefore passed or failed by host speed: measured 2026-10-06, the same
    # ELF kept 4096 samples on the Mac (QEMU 11.0.3) and 3852-3880 on
    # an x86_64 host (QEMU 10.1.0), and BOTH kept ~2160-2192 at 1 ms. At 1 ms
    # that driver fails on every host; one that waits for play-out (section
    # 14क of virtio-sound.sas) keeps all 4096 on both, at 1 ms, 10 ms, 100 ms
    # and under -icount. Nothing below is loosened.
    adev="wav,id=snd0,path=$3,out.frequency=48000,out.channels=2,timer-period=1000"
    # $4 is the only word-split argument, and it is this script's own literal.
    # shellcheck disable=SC2086
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$1" -audiodev "$adev" \
        $4 > "$tmp/raw" 2>&1 &
    qpid=$!
    # The watcher's own stderr is closed off, not just the kills': the SUBSHELL
    # is what announces "Terminated: 15  sleep 60" when its `sleep` is killed,
    # and it inherited this script's stderr when it was forked. Silencing the
    # `kill` instead does nothing, and the word Terminated lands in the middle
    # of this check's PASS report, where the next reader takes it for a failure.
    ( sleep 60; kill -9 "$qpid" 2>/dev/null ) 2>/dev/null & watcher=$!

    wait "$qpid" 2>/dev/null || true
    # `pkill -P` FIRST: killing the subshell alone orphans its `sleep` to init
    # (`W-096`). By PARENT pid, never by pattern — `W-044`/`W-047`/`W-049`;
    # another agent's QEMU has been seen live on this host.
    pkill -P "$watcher" 2>/dev/null || true
    kill "$watcher" 2>/dev/null || true
    wait "$watcher" 2>/dev/null || true
    # QEMU's serial line ends every line with CR. Without stripping it a
    # `grep -E '^VIRTIO-SOUND-DONE$'` matches NOTHING, which looks exactly like
    # a program that never got there. LC_ALL=C because macOS `tr` aborts on
    # invalid UTF-8 and TRUNCATES what it has copied so far.
    LC_ALL=C tr -d '\r' < "$tmp/raw" > "$2" 2>/dev/null || true
}

# The hex line the छापनम् helper prints, taken from immediately after a marker.
# Pairing by position is deliberate: it is the only way "SET_PARAMS said 0x8000"
# can be told apart from "some command somewhere said 0x8000".
answer() {  # $1 = output file, $2 = marker
    awk -v m="$2" 'seen { print; exit } $0 == m { seen = 1 }' "$1"
}

# Every line that follows a PERIOD marker, one per submitted period.
periods() {  # $1 = output file
    awk '/^VIRTIO-SOUND-PERIOD$/ { if ((getline line) > 0) print line }' "$1"
}

bit() {  # $1 = 16-hex-digit word, $2 = bit index -> prints 1 or 0
    d=$(printf '%d' "0x$1" 2>/dev/null || echo 0)
    echo $(( (d >> $2) & 1 ))
}

complain() {  # $1 = output file, $2 = which run
    for tag in NO-DEVICE NO-QUEUE NO-TXQUEUE NO-STREAM CTRL-TIMEOUT TX-TIMEOUT; do
        if grep -aq "^VIRTIO-SOUND-$tag\$" "$1"; then
            echo "  FAIL  run $2 -> the driver reported $tag"
            return 0
        fi
    done
    echo "  FAIL  run $2 -> the driver never reached VIRTIO-SOUND-DONE, and named"
    echo "        no failure of its own; last of what came out:"
    tail -3 "$1" | sed 's/^/          /'
    return 0
}

OK=0000000000008000        # VIRTIO_SND_S_OK, as छापनम् prints it
FILL=00000000ffffffff      # what the driver leaves there before submitting
COMMANDS="PCM-INFO SET-PARAMS PREPARE START STOP RELEASE"

fail=0

echo "Assembling at two link addresses..."
assemble ०षोड्८०२००००० "$tmp/snd-a.elf"
assemble ०षोड्८०४००००० "$tmp/snd-b.elf"

i=0
for pair in "a:0x80200000" "b:0x80400000"; do
    name=${pair%:*}; addr=${pair#*:}
    i=$((i + 1))
    echo
    echo "Run $i of 2 — linked at $addr..."
    run "$tmp/snd-$name.elf" "$tmp/$name.out" "$tmp/$name.wav" \
        "-device virtio-sound-device,audiodev=snd0"

    if ! grep -aq '^VIRTIO-SOUND-DONE$' "$tmp/$name.out"; then
        complain "$tmp/$name.out" "$i"; fail=1; continue
    fi

    printf "  device at     %s\n" "$(answer "$tmp/$name.out" VIRTIO-SOUND-BASE)"
    printf "  streams       %s\n" "$(answer "$tmp/$name.out" VIRTIO-SOUND-STREAMS)"

    for c in $COMMANDS; do
        got=$(answer "$tmp/$name.out" "VIRTIO-SOUND-$c")
        printf "  %-13s %s\n" "$c" "${got:-<nothing printed>}"
        if [ "$got" != "$OK" ]; then
            echo "  FAIL  run $i: $c came back ${got:-with no response line} —"
            echo "        the device did not answer VIRTIO_SND_S_OK (0x8000)."
            if [ "$got" = "$FILL" ]; then
                echo "        ffffffff is the fill the driver wrote before submitting:"
                echo "        the device never wrote a status at all."
            fi
            fail=1
        fi
    done

    # The four period statuses, each read out of an 0xff-filled buffer.
    np=0
    for got in $(periods "$tmp/$name.out"); do
        np=$((np + 1))
        printf "  period %-6d %s\n" "$np" "$got"
        if [ "$got" != "$OK" ]; then
            echo "  FAIL  run $i: period $np came back $got, not VIRTIO_SND_S_OK."
            [ "$got" = "$FILL" ] && echo "        that is the driver's own 0xff fill: the device wrote no status."
            fail=1
        fi
    done
    if [ "$np" -ne 4 ]; then
        echo "  FAIL  run $i: $np periods returned from the tx queue, 4 were submitted."
        fail=1
    fi

    # The device's own description of the stream, and the tie between what it
    # says it can do and what the driver then asked for.
    fmts=$(answer "$tmp/$name.out" VIRTIO-SOUND-FORMATS)
    rates=$(answer "$tmp/$name.out" VIRTIO-SOUND-RATES)
    dirn=$(answer "$tmp/$name.out" VIRTIO-SOUND-DIRECTION)
    printf "  formats       %s   (bit 5 = S16)\n" "$fmts"
    printf "  rates         %s   (bit 7 = 48000)\n" "$rates"
    printf "  dir/channels  %s   (low byte 0 = OUTPUT)\n" "$dirn"

    if [ "$(bit "$fmts" 5)" != "1" ]; then
        echo "  FAIL  run $i: the device's format bitmap is $fmts — bit 5 (S16) is"
        echo "        clear, yet SET_PARAMS asked for S16. The driver is asking for"
        echo "        a format this device never offered."
        fail=1
    fi
    if [ "$(bit "$rates" 7)" != "1" ]; then
        echo "  FAIL  run $i: the device's rate bitmap is $rates — bit 7 (48000) is"
        echo "        clear, yet SET_PARAMS asked for 48000."
        fail=1
    fi
    dird=$(printf '%d' "0x$dirn" 2>/dev/null || echo -1)
    if [ "$(( dird & 255 ))" -ne 0 ]; then
        echo "  FAIL  run $i: stream 0's direction byte is $(( dird & 255 )), not 0"
        echo "        (VIRTIO_SND_D_OUTPUT). Frames pushed at an input stream are"
        echo "        accepted and never played."
        fail=1
    fi
    if [ "$(( (dird >> 16) & 255 ))" -lt 2 ]; then
        echo "  FAIL  run $i: stream 0 takes at most $(( (dird >> 16) & 255 )) channels;"
        echo "        SET_PARAMS asked for 2."
        fail=1
    fi

    if [ ! -s "$tmp/$name.wav" ]; then
        echo "  FAIL  run $i: QEMU's wav backend left no capture at all."; fail=1; continue
    fi
    wave_check "$tmp/$name.wav" || fail=1
done

# ---- the negative control ---------------------------------------------------
# Same ELF, no device. Everything printed above has to stop being printed.
echo
echo "Control — the same image with no virtio-sound device attached..."
run "$tmp/snd-a.elf" "$tmp/none.out" "$tmp/none.wav" ""

if grep -aq '^VIRTIO-SOUND-NO-DEVICE$' "$tmp/none.out"; then
    echo "  tag           VIRTIO-SOUND-NO-DEVICE, as it should be"
else
    echo "  FAIL  with no device attached the driver did not report NO-DEVICE."
    echo "        Either the scan is not really scanning, or the failure tags are"
    echo "        unreachable — and an unreachable tag makes every failure above"
    echo "        read as silence. What came out:"
    tail -3 "$tmp/none.out" | sed 's/^/          /'
    fail=1
fi
for tag in DONE PERIOD START; do
    if grep -aq "^VIRTIO-SOUND-$tag\$" "$tmp/none.out"; then
        echo "  FAIL  with no device attached the driver still printed $tag."
        echo "        The transcript is being printed unconditionally, so none of"
        echo "        it is evidence of a device."
        fail=1
    fi
done
# Both branches are stated. A silent skip here would be the check quietly
# declining to make its own control, which is the thing this control is for.
if [ -f "$tmp/none.wav" ]; then
    silent_check "$tmp/none.wav" || fail=1
else
    echo "  capture       QEMU's wav backend created no file at all — the audio"
    echo "                above cannot have come from anywhere but the device"
fi

echo
[ "$fail" -eq 0 ] || { echo "F-009: the virtio-sound driver did not play."; exit 1; }

echo "ok  F-009: a waveform the guest wrote came out of the host's audio backend."
echo
echo "    virtio-sound was found by scanning the virtio-mmio window for DeviceID"
echo "    25, brought up over the LEGACY register set (QueuePFN/GuestPageSize/"
echo "    QueueAlign — this transport offers no VIRTIO_F_VERSION_1), and driven"
echo "    through PCM_INFO, PCM_SET_PARAMS, PCM_PREPARE, PCM_START, four periods"
echo "    on the tx queue, PCM_STOP and PCM_RELEASE. Every status word printed"
echo "    above was read out of a buffer the DEVICE wrote, over a buffer the"
echo "    driver had filled with 0xff first, and the S16/48000 the driver asked"
echo "    for were checked against the format and rate bitmaps the device itself"
echo "    reported in PCM_INFO."
echo
echo "    Then the host's own audio was read back. QEMU's wav backend captured"
echo "    4096 samples of 16-bit stereo: -2000 and +2000 and nothing else, in 64"
echo "    runs of exactly 64 samples, alternating, starting at -2000 — which is"
echo "    the square wave the guest stored into the pages it handed the device."
echo "    That file was written by the host out of frames the device pulled from"
echo "    guest memory, so no amount of printing by the guest could produce it."
echo
echo "    Both link addresses did it, and the same image with no device attached"
echo "    reported VIRTIO-SOUND-NO-DEVICE, played nothing, and printed no START,"
echo "    no PERIOD and no DONE."
