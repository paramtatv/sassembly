#!/bin/sh
# A pixel reaches the SCREEN — row `C-009`, gate C9.
#
# ## What is being proved
#
# `spec/virtio-gpu.sas` finds a virtio-gpu over the virtio-mmio window, brings it
# up through the §3.1.1 handshake, and drives the control queue through the whole
# 2D path: RESOURCE_CREATE_2D, RESOURCE_ATTACH_BACKING, SET_SCANOUT,
# TRANSFER_TO_HOST_2D, RESOURCE_FLUSH.
#
# Two different things are asserted, and they fail independently:
#
#   1. THE DEVICE'S OWN ANSWER. Each command's response type is read back out of
#      the buffer the DEVICE wrote and must be 0x1100 — VIRTIO_GPU_RESP_OK_NODATA.
#      The program fills that buffer with 0xff before every submission, so "the
#      device never touched it" reads back as ffffffff and cannot be mistaken for
#      success. This is why the check does not merely assert that the program
#      printed something: a program that prints five lines and drives no queue at
#      all looks identical from outside, and doc 11 §2's rule is that a check
#      whose failure mode is silence is indistinguishable from one that did not run.
#
#   2. THE PIXELS. QEMU is asked, over QMP, for a `screendump` of the console
#      while the guest is still up, and the PPM that comes back is inspected byte
#      by byte. The guest paints 64x32 pixels of 0x112233 and exactly one pixel
#      of 0xc47b2a at (1,1); the check requires that histogram exactly, and
#      requires the odd pixel to be at (1,1). That last part is what makes the
#      pitch arithmetic falsifiable — a driver with the wrong stride still paints
#      a screen, but it paints the odd pixel somewhere else.
#
# (2) is the row. (1) alone would prove the device accepted five commands, which
# a device will happily do for a resource nobody ever looks at. Only the
# screendump is read off the HOST's copy of the framebuffer, and it is the only
# assertion here that the guest cannot fake by printing.
#
# ## Why the guest does not shut down
#
# The framebuffer exists only while the machine does. `spec/virtio-gpu.sas`
# therefore prints `VIRTIO-GPU-DONE` and spins instead of calling SBI shutdown;
# this script waits for that marker — it is printed after the used ring returned
# the flush, so the screen is painted before the photograph is taken — then takes
# the screendump and kills QEMU itself.
#
# ## Both link addresses
#
# The queue region, the command page and the framebuffer are all computed with
# `auipc`, and the framebuffer address is handed to the DEVICE as a guest
# physical address in RESOURCE_ATTACH_BACKING. Running at 0x80200000 and
# 0x80400000 is what shows that address is computed rather than a constant that
# happened to be right once — a hard-coded one would have the device scribbling
# on, or reading, the wrong page at the second address.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
# NOT in $root: the coordinating session gates on a fingerprint of the working
# tree, so a scratch file dropped in the repo is a file that exists while the
# gate is looking, even if it is deleted a line later.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

prog=$root/spec/virtio-gpu.sas
[ -f "$prog" ] || { echo "RED, as written: spec/virtio-gpu.sas does not exist."; exit 1; }

# The screendump half needs a QMP client. There is no shell one — `nc` is not
# universally present and cannot be relied on to speak a unix socket — so this
# is python3, and its absence is reported as 77 at the END, after the response
# codes have still been checked and printed. Exiting 77 up here would throw away
# a measurement this host can perfectly well make.
have_python=1
command -v python3 >/dev/null 2>&1 || have_python=0

# ---- the QMP client: connect, negotiate, screendump, leave ------------------
cat > "$tmp/shot.py" <<'PY'
import json, socket, sys, time
sock_path, out = sys.argv[1], sys.argv[2]
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
for _ in range(100):                       # QEMU may not have bound it yet
    try:
        s.connect(sock_path)
        break
    except OSError:
        time.sleep(0.1)
else:
    sys.exit("could not connect to the QMP socket")
f = s.makefile('rw', encoding='utf-8', newline='\n')
f.readline()                               # the greeting
def cmd(c):
    f.write(json.dumps(c) + '\n')
    f.flush()
    while True:
        line = f.readline()
        if not line:
            sys.exit("QMP closed mid-command")
        m = json.loads(line)
        if 'error' in m:
            sys.exit("QMP error: %s" % m['error'])
        if 'return' in m:
            return m['return']
cmd({'execute': 'qmp_capabilities'})
cmd({'execute': 'screendump', 'arguments': {'filename': out}})
PY

# ---- the PPM reader: what is actually on the screen -------------------------
#
# Asserts the WHOLE histogram, not just one probe. A driver that painted the
# right pixel and left the rest of the surface as QEMU found it would pass a
# single-pixel probe; it cannot pass "2047 of this colour and 1 of that".
cat > "$tmp/pixels.py" <<'PY'
import sys
from collections import Counter

BG, FG = (0x11, 0x22, 0x33), (0xc4, 0x7b, 0x2a)
W, H, AT = 64, 32, (1, 1)

data = open(sys.argv[1], 'rb').read()
fields, i = [], 0
while len(fields) < 4:                      # P6, width, height, maxval
    while i < len(data) and data[i:i+1].isspace():
        i += 1
    j = i
    while j < len(data) and not data[j:j+1].isspace():
        j += 1
    if j == i:
        sys.exit("  the screendump is not a readable PPM")
    fields.append(data[i:j]); i = j
i += 1
px = data[i:]

bad = 0
def bad_news(msg):
    global bad
    print("  FAIL  " + msg); bad = 1

def note(msg):                              # a continuation of the line above
    print("        " + msg)

if fields[0] != b'P6':
    bad_news("the screendump is %r, not the P6 PPM screendump returns" % fields[0])
w, h, mx = int(fields[1]), int(fields[2]), int(fields[3])
if (w, h) != (W, H):
    bad_news("the screen is %dx%d; SET_SCANOUT asked for %dx%d" % (w, h, W, H))
if len(px) < w * h * 3:
    bad_news("the screendump holds %d bytes of pixels, %d expected"
             % (len(px), w * h * 3))
if bad:
    sys.exit(1)

def at(x, y):
    o = (y * w + x) * 3
    return tuple(px[o:o + 3])

hist = Counter(tuple(px[k:k+3]) for k in range(0, w * h * 3, 3))
def hexc(c):
    return "%02x%02x%02x" % c

print("  screen        %dx%d, maxval %d" % (w, h, mx))
print("  colours       " + ", ".join("%s x%d" % (hexc(c), n)
                                     for c, n in hist.most_common(4)))
print("  pixel (1,1)   " + hexc(at(*AT)))
print("  pixel (0,0)   " + hexc(at(0, 0)))

want = {BG: w * h - 1, FG: 1}
if dict(hist) != want:
    bad_news("the screen is not what the guest painted; expected %s x%d"
             % (hexc(BG), w * h - 1))
    note("and %s x1, and nothing else" % hexc(FG))
if at(*AT) != FG:
    bad_news("(1,1) is %s, not the %s the guest stored at byte offset 260 —"
             % (hexc(at(*AT)), hexc(FG)))
    note("the odd pixel landed somewhere else, so the pitch is wrong")
if at(0, 0) != BG:
    bad_news("(0,0) is %s, not the background %s" % (hexc(at(0, 0)), hexc(BG)))
sys.exit(bad)
PY

assemble() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$prog" "$2" >/dev/null
}

# One run: boot, wait for the driver to say it is finished, photograph the
# screen, kill the machine.
#
# Bounded by an explicit SIGKILL from a background `sleep`, not by perl's alarm:
# QEMU runs its own timer subsystem and survives SIGALRM (`W-060`). This guest
# never exits on its own — it spins so the framebuffer stays alive — so an
# unbounded wait here would hang the check rather than fail it.
run() {  # $1 = elf, $2 = cleaned output, $3 = where to leave the screendump
    rm -f "$tmp/qmp.sock" "$3" 2>/dev/null || true
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$1" \
        -device virtio-gpu-device \
        -qmp "unix:$tmp/qmp.sock,server=on,wait=off" > "$tmp/raw" 2>&1 &
    qpid=$!
    # The watcher's own stderr is closed off, not just the kills': the SUBSHELL
    # is what announces "Terminated: 15  sleep 40" when its `sleep` is killed,
    # and it inherited this script's stderr when it was forked. Silencing the
    # `kill` instead does nothing, and the word Terminated lands in the middle
    # of this check's PASS report, where the next reader takes it for a failure.
    ( sleep 40; kill -9 "$qpid" 2>/dev/null ) 2>/dev/null & watcher=$!

    # Poll for the marker rather than sleeping a fixed time: DONE is printed
    # only after the used ring returned RESOURCE_FLUSH, so the screen is painted
    # by the time it appears, and photographing before it would race the device.
    n=0
    while [ "$n" -lt 30 ]; do
        # QEMU's serial line ends every line with CR. Without stripping it a
        # `grep -E '^VIRTIO-GPU-DONE$'` matches NOTHING, which looks exactly
        # like a program that never got there. LC_ALL=C because macOS `tr`
        # aborts on invalid UTF-8 and TRUNCATES what it has copied so far.
        LC_ALL=C tr -d '\r' < "$tmp/raw" > "$2" 2>/dev/null || true
        if grep -q '^VIRTIO-GPU-DONE$' "$2"; then break; fi
        n=$((n + 1))
        sleep 1
    done

    shot_err=""
    if [ "$want_shot" -eq 1 ] && grep -q '^VIRTIO-GPU-DONE$' "$2"; then
        shot_err=$(python3 "$tmp/shot.py" "$tmp/qmp.sock" "$3" 2>&1) || true
    fi

    kill -9 "$qpid" 2>/dev/null || true
    wait "$qpid" 2>/dev/null || true
    # `pkill -P` FIRST: killing the subshell alone orphans its `sleep` to init
    # (`W-096`). By PARENT pid, never by pattern — `W-044`/`W-047`/`W-049`;
    # another agent's QEMU has been seen live on this host.
    pkill -P "$watcher" 2>/dev/null || true
    kill "$watcher" 2>/dev/null || true
    wait "$watcher" 2>/dev/null || true
    LC_ALL=C tr -d '\r' < "$tmp/raw" > "$2" 2>/dev/null || true
}

# The hex line the छापनम् helper prints, taken from immediately after a marker.
# Pairing by position is deliberate: it is the only way "SET_SCANOUT said 0x1100"
# can be told apart from "some command somewhere said 0x1100".
answer() {  # $1 = output file, $2 = marker
    awk -v m="$2" 'seen { print; exit } $0 == m { seen = 1 }' "$1"
}

complain() {  # $1 = output file, $2 = which run
    for tag in NO-DEVICE NO-QUEUE; do
        if grep -q "^VIRTIO-GPU-$tag$" "$1"; then
            echo "  FAIL  run $2 -> the driver reported $tag"
            return 0
        fi
    done
    echo "  FAIL  run $2 -> the driver never reached VIRTIO-GPU-DONE, and named"
    echo "        no failure of its own; last of what came out:"
    tail -3 "$1" | sed 's/^/          /'
    return 0
}

OK=0000000000001100        # VIRTIO_GPU_RESP_OK_NODATA, as छापनम् prints it
COMMANDS="CREATE-2D ATTACH-BACKING SET-SCANOUT TRANSFER-TO-HOST-2D RESOURCE-FLUSH"

fail=0

echo "Assembling at two link addresses..."
assemble ०षोड्८०२००००० "$tmp/gpu-a.elf"
assemble ०षोड्८०४००००० "$tmp/gpu-b.elf"

i=0
for pair in "a:0x80200000" "b:0x80400000"; do
    name=${pair%:*}; addr=${pair#*:}
    i=$((i + 1))
    echo
    echo "Run $i of 2 — linked at $addr..."
    want_shot=$have_python
    run "$tmp/gpu-$name.elf" "$tmp/$name.out" "$tmp/$name.ppm"

    if ! grep -q '^VIRTIO-GPU-DONE$' "$tmp/$name.out"; then
        complain "$tmp/$name.out" "$i"; fail=1; continue
    fi

    printf "  device at     %s\n" "$(answer "$tmp/$name.out" VIRTIO-GPU-BASE)"
    for c in $COMMANDS; do
        got=$(answer "$tmp/$name.out" "VIRTIO-GPU-$c")
        printf "  %-20s %s\n" "$c" "${got:-<nothing printed>}"
        if [ "$got" != "$OK" ]; then
            echo "  FAIL  run $i: $c came back ${got:-with no response line} —"
            echo "        the device did not answer OK_NODATA (0x1100)."
            if [ "$got" = "00000000ffffffff" ]; then
                echo "        ffffffff is the fill the driver wrote before submitting:"
                echo "        the device never wrote a response at all."
            fi
            fail=1
        fi
    done

    [ "$have_python" -eq 1 ] || continue
    if [ -n "$shot_err" ]; then
        echo "  FAIL  run $i: the screendump could not be taken: $shot_err"; fail=1; continue
    fi
    if [ ! -s "$tmp/$name.ppm" ]; then
        echo "  FAIL  run $i: QMP reported success but left no screendump."; fail=1; continue
    fi
    python3 "$tmp/pixels.py" "$tmp/$name.ppm" || fail=1
done

echo
[ "$fail" -eq 0 ] || { echo "the first pixel is not on the screen."; exit 1; }

if [ "$have_python" -eq 0 ]; then
    echo "SKIPPED: python3 is not installed, so no QMP client could ask QEMU for a"
    echo "         screendump. The five control-queue commands were checked and the"
    echo "         device answered OK_NODATA to each, at both link addresses — but"
    echo "         nothing here has looked at the HOST's framebuffer, and that is"
    echo "         the row. Exiting 77 rather than 0."
    exit 77
fi

echo "ok  C-009: a pixel the guest wrote is on the host's screen."
echo
echo "    virtio-gpu was found by scanning the virtio-mmio window for DeviceID 16,"
echo "    brought up over the LEGACY register set (QueuePFN/GuestPageSize/"
echo "    QueueAlign — this transport offers no VIRTIO_F_VERSION_1), and driven"
echo "    through RESOURCE_CREATE_2D, RESOURCE_ATTACH_BACKING, SET_SCANOUT,"
echo "    TRANSFER_TO_HOST_2D and RESOURCE_FLUSH. Every response code printed"
echo "    above was read out of the buffer the DEVICE wrote, over a buffer the"
echo "    driver had filled with 0xff first."
echo
echo "    Then QEMU was asked for the screen itself. It is 64x32 — the size"
echo "    SET_SCANOUT asked for — and it holds 2047 pixels of 112233 and exactly"
echo "    one of c47b2a, at (1,1), which is where the guest stored it. That"
echo "    picture came off the host's copy of the framebuffer, so no amount of"
echo "    printing by the guest could have produced it."
