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

# The audio half needs a WAV reader. There is no shell one, so this is python3,
# and its absence is reported as 77 at the END, after the status words have
# still been checked and printed. Exiting 77 up here would throw away a
# measurement this host can perfectly well make.
have_python=1
command -v python3 >/dev/null 2>&1 || have_python=0

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
    out=$(cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$prog" "$2" 2>&1) || {
        echo "  FAIL  the program does not assemble at $1:"
        echo "$out" | sed 's/^/          /'
        exit 1; }
}

# ---- the WAV reader: what the HOST actually played --------------------------
cat > "$tmp/wave.py" <<'PY'
import itertools, struct, sys

AMP    = 2000     # the guest stores +/-2000 as int16
RUN    = 64       # samples between sign flips: 32 stereo frames
FIRST  = -AMP     # the guest's first sample
WANT   = 4096     # 4 periods x 2048 bytes / 2 bytes per sample

path = sys.argv[1]
try:
    data = open(path, 'rb').read()
except OSError as e:
    print("  FAIL  the wav capture could not be read: %s" % e)
    sys.exit(1)

bad = 0
def fail(msg):
    global bad
    print("  FAIL  " + msg); bad = 1
def note(msg):
    print("        " + msg)

if len(data) < 44 or data[0:4] != b'RIFF' or data[8:12] != b'WAVE':
    fail("the capture is not a RIFF/WAVE file (%d bytes) — QEMU's wav backend"
         % len(data))
    note("wrote nothing the device had pulled out of guest memory")
    sys.exit(1)

audiofmt, ch, rate, byterate, align, bits = struct.unpack('<HHIIHH', data[20:36])
pcm = data[44:]
n = len(pcm) // 2
v = list(struct.unpack('<%dh' % n, pcm[:n * 2])) if n else []

print("  wav header    %d Hz, %d ch, %d-bit, %d bytes of pcm" % (rate, ch, bits, n * 2))

if (audiofmt, ch, bits) != (1, 2, 16):
    fail("the capture is format %d, %d channels, %d bits, not 16-bit stereo PCM"
         % (audiofmt, ch, bits))

# Silence at the very edges is the HOST's, not the guest's: the backend voice can
# be running for a moment before the first period lands. Strip it and say so, but
# any silence INSIDE the wave still shows up as a broken run below.
a, b = 0, len(v)
while a < b and v[a] == 0:
    a += 1
while b > a and v[b - 1] == 0:
    b -= 1
pad = a + (len(v) - b)
core = v[a:b]
if pad:
    note("%d samples of host-side silence trimmed from the edges" % pad)

if not core:
    fail("the capture holds no non-zero samples: the device consumed no frames")
    sys.exit(1)

runs = [(k, len(list(g))) for k, g in itertools.groupby(core)]
vals = sorted(set(k for k, _ in runs))
lens = sorted(set(L for _, L in runs))

print("  samples       %d (%d expected)" % (len(core), WANT))
print("  values        %s" % ", ".join(str(x) for x in vals))
print("  runs          %d, lengths %s" % (len(runs), ", ".join(str(x) for x in lens)))
print("  first sample  %d" % core[0])

if len(core) != WANT:
    fail("the host played %d samples; the guest submitted 4 periods of 2048"
         % len(core))
    note("bytes, which is %d samples of 16-bit audio" % WANT)
if vals != [-AMP, AMP]:
    fail("the played values are %s, not the -%d / +%d square wave the guest"
         % (vals, AMP, AMP))
    note("stored into the buffer it handed the device")
if lens != [RUN]:
    fail("the sign flips every %s samples, not every %d" % (lens, RUN))
    note("a run of any other length means the stream rate or the channel count")
    note("is not what SET_PARAMS asked for — QEMU resamples to the backend rate,")
    note("so 44100 would land here as 69.7, and mono would halve every run")
if core[0] != FIRST:
    fail("the wave starts at %d, not the %d the guest wrote first" % (core[0], FIRST))
if any(runs[i][0] == runs[i + 1][0] for i in range(len(runs) - 1)):
    fail("two adjacent runs carry the same value: the wave is not alternating")
sys.exit(bad)
PY

# ---- the empty-capture reader, for the negative run -------------------------
cat > "$tmp/silent.py" <<'PY'
import struct, sys
data = open(sys.argv[1], 'rb').read() if len(sys.argv) > 1 else b''
pcm = data[44:] if len(data) >= 44 else b''
n = len(pcm) // 2
v = struct.unpack('<%dh' % n, pcm[:n * 2]) if n else ()
loud = sum(1 for x in v if x != 0)
print("  capture       %d bytes, %d samples, %d of them non-zero" % (len(data), n, loud))
if loud:
    print("  FAIL  the negative run played %d non-zero samples with no virtio-sound" % loud)
    print("        device on the command line — the audio above is not the driver's.")
    sys.exit(1)
sys.exit(0)
PY

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
    # A Linux x86-64 host (QEMU 10.1.0), and BOTH kept ~2160-2192 at 1 ms. At 1 ms
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

    [ "$have_python" -eq 1 ] || continue
    if [ ! -s "$tmp/$name.wav" ]; then
        echo "  FAIL  run $i: QEMU's wav backend left no capture at all."; fail=1; continue
    fi
    python3 "$tmp/wave.py" "$tmp/$name.wav" || fail=1
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
if [ "$have_python" -eq 0 ]; then
    echo "  capture       not read: python3 is absent"
elif [ -f "$tmp/none.wav" ]; then
    python3 "$tmp/silent.py" "$tmp/none.wav" || fail=1
else
    echo "  capture       QEMU's wav backend created no file at all — the audio"
    echo "                above cannot have come from anywhere but the device"
fi

echo
[ "$fail" -eq 0 ] || { echo "F-009: the virtio-sound driver did not play."; exit 1; }

if [ "$have_python" -eq 0 ]; then
    echo "SKIPPED: python3 is not installed, so nothing here could read the WAV"
    echo "         QEMU's audio backend wrote. Every control-queue command and"
    echo "         every period was checked and the device answered"
    echo "         VIRTIO_SND_S_OK to each, at both link addresses — but nothing"
    echo "         has listened to the HOST's copy of the audio, and that is the"
    echo "         row. Exiting 77 rather than 0."
    exit 77
fi

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
