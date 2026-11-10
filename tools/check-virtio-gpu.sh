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

# ---- the QMP client: connect, negotiate, screendump, leave ------------------
. "$root/tools/lib/qmp.sh"   # QMP client in sh + nc -U (N2; replaces the python heredoc)

# ---- the PPM reader: what is actually on the screen -------------------------
#
# Asserts the WHOLE histogram, not just one probe. A driver that painted the
# right pixel and left the rest of the surface as QEMU found it would pass a
# single-pixel probe; it cannot pass "2047 of this colour and 1 of that".
#
# Plain od + awk (no interpreter): od turns the file into decimal byte values,
# awk parses the header and tallies the pixels. Prints to stdout, rc 1 on a bad
# screen, as the reader it replaces did.
pixels() {
    od -An -v -tu1 "$1" | LC_ALL=C awk '
    function sp(c) { return c == 32 || (c >= 9 && c <= 13) }
    function hexc(r, g, b) { return sprintf("%02x%02x%02x", r, g, b) }
    function brepr(s,   q, i, c, o, n) {          # python bytes repr
        q = "\047"
        if (index(s, "\047") && !index(s, "\042")) q = "\042"
        o = ""
        for (i = 1; i <= length(s); i++) {
            c = substr(s, i, 1); n = ord[c]
            if (c == "\\") o = o "\\\\"
            else if (c == q) o = o "\\" c
            else if (n == 9) o = o "\\t"
            else if (n == 10) o = o "\\n"
            else if (n == 13) o = o "\\r"
            else if (n < 32 || n >= 127) o = o sprintf("\\x%02x", n)
            else o = o c
        }
        return "b" q o q
    }
    function bad_news(msg) { print "  FAIL  " msg; bad = 1 }
    function note(msg) { print "        " msg }
    function at(x, y) { o = (y * w + x) * 3; return hexc(px[o], px[o + 1], px[o + 2]) }
    BEGIN { for (i = 0; i < 256; i++) ord[sprintf("%c", i)] = i }
    { for (i = 1; i <= NF; i++) d[n++] = $i + 0 }
    END {
        i = 0; nf = 0
        while (nf < 4) {
            while (i < n && sp(d[i])) i++
            j = i
            while (j < n && !sp(d[j])) j++
            if (j == i) { print "  the screendump is not a readable PPM" > "/dev/stderr"; exit 1 }
            s = ""
            for (k = i; k < j; k++) s = s sprintf("%c", d[k])
            f[nf++] = s; i = j
        }
        i++
        npx = n - i; if (npx < 0) npx = 0
        for (k = 0; k < npx; k++) px[k] = d[i + k]
        W = 64; H = 32
        if (f[0] != "P6")
            bad_news("the screendump is " brepr(f[0]) ", not the P6 PPM screendump returns")
        if (f[1] !~ /^[0-9]+$/ || f[2] !~ /^[0-9]+$/ || f[3] !~ /^[0-9]+$/) {
            print "  the screendump header is not numeric" > "/dev/stderr"; exit 1 }
        w = f[1] + 0; h = f[2] + 0; mx = f[3] + 0
        if (w != W || h != H)
            bad_news(sprintf("the screen is %dx%d; SET_SCANOUT asked for %dx%d", w, h, W, H))
        if (npx < w * h * 3)
            bad_news(sprintf("the screendump holds %d bytes of pixels, %d expected", npx, w * h * 3))
        if (bad) exit 1
        nc = 0
        for (k = 0; k < w * h * 3; k += 3) {
            c = hexc(px[k], px[k + 1], px[k + 2])
            if (!(c in cnt)) { cnt[c] = 0; ord_c[nc++] = c }
            cnt[c]++
        }
        printf "  screen        %dx%d, maxval %d\n", w, h, mx
        line = "  colours       "
        for (t = 0; t < 4; t++) {
            best = ""; bn = 0
            for (k = 0; k < nc; k++) {
                c = ord_c[k]
                if (!(c in used) && cnt[c] > bn) { best = c; bn = cnt[c] }
            }
            if (best == "") break
            used[best] = 1
            line = line (t ? ", " : "") best " x" bn
        }
        print line
        print "  pixel (1,1)   " at(1, 1)
        print "  pixel (0,0)   " at(0, 0)
        BG = "112233"; FG = "c47b2a"
        if (!(nc == 2 && cnt[BG] == w * h - 1 && cnt[FG] == 1)) {
            bad_news(sprintf("the screen is not what the guest painted; expected %s x%d", BG, w * h - 1))
            note(sprintf("and %s x1, and nothing else", FG))
        }
        if (at(1, 1) != FG) {
            bad_news(sprintf("(1,1) is %s, not the %s the guest stored at byte offset 260 —", at(1, 1), FG))
            note("the odd pixel landed somewhere else, so the pitch is wrong")
        }
        if (at(0, 0) != BG)
            bad_news(sprintf("(0,0) is %s, not the background %s", at(0, 0), BG))
        exit bad
    }'
}

assemble() {
    rc=0
    "$root/tools/sadhana-native.sh" --स्थान "$1" "$prog" "$2" >/dev/null || rc=$?
    [ "$rc" -eq 77 ] && { echo "  SKIP  no native assembler image (exit 77)"; exit 77; }
    [ "$rc" -eq 0 ] || exit "$rc"
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
        if grep -aq '^VIRTIO-GPU-DONE$' "$2"; then break; fi
        n=$((n + 1))
        sleep 1
    done

    shot_err=""
    if [ "$want_shot" -eq 1 ] && grep -aq '^VIRTIO-GPU-DONE$' "$2"; then
        shot_err=$(qmp_shot "$tmp/qmp.sock" "$tmp" "$3" 2>&1) || true
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
        if grep -aq "^VIRTIO-GPU-$tag$" "$1"; then
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
    want_shot=1
    run "$tmp/gpu-$name.elf" "$tmp/$name.out" "$tmp/$name.ppm"

    if ! grep -a -q '^VIRTIO-GPU-DONE$' "$tmp/$name.out"; then
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

    if [ -n "$shot_err" ]; then
        echo "  FAIL  run $i: the screendump could not be taken: $shot_err"; fail=1; continue
    fi
    if [ ! -s "$tmp/$name.ppm" ]; then
        echo "  FAIL  run $i: QMP reported success but left no screendump."; fail=1; continue
    fi
    pixels "$tmp/$name.ppm" || fail=1
done

echo
[ "$fail" -eq 0 ] || { echo "the first pixel is not on the screen."; exit 1; }

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
