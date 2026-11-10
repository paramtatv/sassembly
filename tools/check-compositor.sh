#!/bin/sh
# The compositor presents whole frames — row `C-010`, gate C10.
#
# ## What is being proved, and what is not
#
# `compositor.sas` composites three layers into a 1920x1080 B8G8R8X8 surface and
# presents it through virtio-gpu, double buffered: the composite is written into
# whichever of two backing stores is NOT on the scanout, and SET_SCANOUT for that
# resource is issued only after the composite has finished. This script measures
# whether the picture the HOST holds is ever a mix of two composites.
#
# It can measure that because the guest signs every pixel. In frame k, every
# pixel of every layer carries k (mod 256) in its BLUE byte, and its layer
# identity in its RED byte. So:
#
#   * a screendump of one whole composite has EXACTLY ONE distinct blue value
#     across all 2073600 pixels;
#   * a screendump of a torn frame has two, and says where the seam was.
#
# The screendump is taken over QMP while the guest is running flat out and is not
# synchronised with it in any way: QEMU serves it from its main loop while the
# vCPU keeps compositing. Every sample is therefore an unannounced look at the
# host's copy of the framebuffer at a moment the guest did not choose.
#
# ## The control
#
# A check that only ever sees one blue value has not shown that it could see two.
# So the guest tears ONCE, on purpose, before the loop starts: it composites frame
# 1 into the front buffer, presents it, then repaints the top half of that same
# still-scanned-out buffer with frame 2's blue and transfers it — the exact
# mistake double buffering exists to prevent — and holds the result for about 8
# seconds, announcing the window on the serial line. This script screendumps
# inside that window and REQUIRES two blue values there. If the detector cannot
# see the deliberate tear, nothing it says about the loop is worth anything, and
# the run is failed by name rather than passed.
#
# ## The layer areas, which are the compositing half of the row
#
# Every clean sample is also required to hold exactly
#
#     1173600 pixels of red 0x11   background
#      300000 pixels of red 0xc4   layer 1, 900x500 at (100,100), partly covered
#      600000 pixels of red 0x2a   layer 2, 1000x600 at (500,300), on top
#
# and nothing else. Compositing the two rectangles in the other order swaps the
# second and third counts, so the painter order is falsifiable from the picture.
# A driver that painted one correct probe pixel cannot pass this.
#
# ## What "1080p60" is NOT proved to be
#
# The geometry is the row's geometry — a real 1920x1080 surface, a real 8294400
# byte transfer per presented frame. The RATE is not asserted. The guest prints
# its frame number and the elapsed `time` ticks after every present, and this
# script reports the measured frames, the measured seconds and the quotient. That
# number is composites-and-presents per second under QEMU's interpreter on this
# host; there is no display refresh, no vsync and no timing device in this
# picture, so it is not a demonstration that a 60 Hz screen was fed on time. It
# is the only rate this arrangement can honestly report.
#
# Being reported and not asserted makes it the one number here that nothing is
# watching, so the ledger read is written to fail loudly rather than to print.
# If the elapsed time cannot be read off a WHOLE record, this check says so by
# name and reds. A measurement it did not take is never printed as a 0.0.
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

root=$(CDPATH= cd -- "$here/.." && pwd)
prog=$root/spec/compositor.sas
[ -f "$prog" ] || { echo "RED, as written: $prog does not exist."; exit 1; }

# `cargo run`, like every other check in this tree.
#
# This was drafted against a PREBUILT binary at the repository root, so it
# could run while a gate held the cargo lock. That is dropped deliberately: the
# override resolves to a path this check does not own, and a stale binary left
# there would assemble with an assembler that is not the one in the tree — and
# still print `ok`. Waiting for a lock is cheaper than measuring the wrong
# build and believing it.
# (assemble() below calls tools/sadhana-native.sh.)

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

. "$root/tools/lib/qmp.sh"   # QMP client in sh + nc -U (N2; replaces the python heredoc)

# ---- the PPM reader ---------------------------------------------------------
#
# pixels <mode> <file> <label>.  mode `clean` demands one composite; mode `torn`
# demands the control's deliberate mix. POSIX sh + od/awk, no interpreter:
#   1. pixels-hdr.awk reads the first bytes (od -tu1) and prints
#      "<fields> <pixel offset> <magic> <width> <height>" the way a PPM
#      tokenises: four whitespace-separated fields, then ONE whitespace byte;
#   2. the pixel bytes are cut with tail/head, dumped by od -An -v -tu1, joined
#      48 bytes (= 16 whole pixels, so channel = position mod 3) per line by
#      paste, and collapsed by sort | uniq -c, so awk only walks the DISTINCT
#      48-byte groups — a composite is a few thousand, not 2073600 pixels;
#   3. pixels-cnt.awk keeps per-channel histograms and prints the verdict.
cat > "$tmp/pixels-hdr.awk" <<'AWK'
BEGIN { for (i = 33; i < 127; i++) chr[i] = sprintf("%c", i) }
{
    for (k = 1; k <= NF; k++) {
        v = $k + 0; pos++
        ws = (v == 32 || (v >= 9 && v <= 13))
        if (infield) {
            if (ws) { nf++; fld[nf] = cur; cur = ""; infield = 0; if (nf == 4) { print nf, pos, fld[1], fld[2], fld[3]; done = 1; exit } }
            else cur = cur ((v in chr) ? chr[v] : "?")
        } else if (!ws) { infield = 1; cur = chr[v] }
    }
}
END {
    if (done) exit
    if (infield) { nf++; fld[nf] = cur }
    if (nf == 4) print nf, pos + 1, fld[1], fld[2], fld[3]; else print nf + 0
}
AWK
cat > "$tmp/pixels-cnt.awk" <<'AWK'
function fail(msg) { print "  FAIL  " label ": " msg; bad = 1 }
function note(msg) { print "        " msg }
function blist(   b, s, first, k) {       # blue values ascending, with counts
    s = ""; first = 1
    for (b = 0; b < 256; b++) { k = sprintf("%02x", b)
        if (bc[k] > 0) { s = s (first ? "" : ", ") k " x" bc[k]; first = 0 } }
    return s
}
BEGIN {
    lr[1] = "11"; lg[1] = "22"; lw[1] = 1173600; ln[1] = "background"
    lr[2] = "c4"; lg[2] = "7b"; lw[2] = 300000;  ln[2] = "layer 1 900x500 at (100,100)"
    lr[3] = "2a"; lg[3] = "d0"; lw[3] = 600000;  ln[3] = "layer 2 1000x600 at (500,300)"
    for (i = 0; i < 256; i++) hx[i] = sprintf("%02x", i)
}
{                                          # uniq -c: count, then up to 48 bytes
    n = $1
    for (k = 2; k <= NF; k++) {
        ch = (k - 2) % 3; h = hx[$k + 0]
        if (ch == 0) rc_[h] += n; else if (ch == 1) gc[h] += n; else bc[h] += n
    }
}
END {
    w = WW + 0; h = HH + 0
    nb = 0; for (b = 0; b < 256; b++) if (bc[hx[b]] > 0) { nb++; b1 = hx[b] }
    nr = 0; for (b = 0; b < 256; b++) if (rc_[hx[b]] > 0) nr++
    if (mode == "torn") {
        # The CONTROL. Two blue values is the whole point of this sample.
        print "  control       " w "x" h ", blue values on screen: " blist()
        if (nb < 2) {
            fail("the deliberately torn frame shows " nb " blue value(s), not 2.")
            note("The guest repainted half of the buffer it was scanning out and")
            note("said so on the serial line, and this screendump did not see it.")
            note("The detector is blind, so its verdict on the loop means nothing.")
        }
        exit bad
    }
    # mode == "clean"
    s = ""
    for (j = 1; j <= 3; j++) s = s (j > 1 ? "/" : "") (rc_[lr[j]] + 0)
    printf "  %-12s blue %s   layers %s\n", label, (nb == 1 ? b1 : "00"), s
    if (nb != 1) {
        fail("TORN — " nb " distinct blue values on one presented frame: " blist())
        note("Every pixel of composite k carries k in its blue byte, so more than")
        note("one blue value means the host is showing parts of two composites.")
    }
    for (j = 1; j <= 3; j++) {
        if (rc_[lr[j]] + 0 != lw[j])
            fail(ln[j] ": " (rc_[lr[j]] + 0) " pixels of red " lr[j] ", " lw[j] " expected")
        if (gc[lg[j]] + 0 != lw[j])
            fail(ln[j] ": " (gc[lg[j]] + 0) " pixels of green " lg[j] ", " lw[j] " expected")
    }
    if (nr != 3)
        fail("the screen holds " nr " red values, not the 3 the three layers paint")
    if (!bad) print "RESULT " b1
    exit bad
}
AWK
pixels() {  # $1 = mode, $2 = ppm, $3 = label; rc 1 on any failure
    px_info=$(head -c 256 "$2" | od -An -v -tu1 | awk -f "$tmp/pixels-hdr.awk")
    # shellcheck disable=SC2086
    set -- "$1" "$2" "$3" $px_info
    if [ "$#" -lt 8 ]; then
        echo "  FAIL  $3: the screendump is not a readable PPM"; return 1
    fi
    if [ "$6" != "P6" ]; then
        echo "  FAIL  $3: the screendump is b'$6', not the P6 a screendump returns"; return 1
    fi
    case $7$8 in *[!0-9]*|'') return 1 ;; esac
    if [ "$7" -ne 1920 ] || [ "$8" -ne 1080 ]; then
        echo "  FAIL  $3: the screen is ${7}x$8; SET_SCANOUT asked for 1920x1080"; return 1
    fi
    px_have=$(( $(wc -c < "$2") - $5 ))
    px_need=$(( $7 * $8 * 3 ))
    if [ "$px_have" -lt "$px_need" ]; then
        echo "  FAIL  $3: the screendump holds $px_have bytes of pixels, $px_need expected"; return 1
    fi
    head -c $(( $5 + 1 - 1 + $px_need )) "$2" | tail -c +$(($5 + 1)) | od -An -v -tu1 | paste - - - |
        LC_ALL=C sort | uniq -c |
        awk -v mode="$1" -v label="$3" -v WW="$7" -v HH="$8" -f "$tmp/pixels-cnt.awk"
}

# The assembler's diagnostics are in Sanskrit and are the whole story when it
# refuses, so they are kept and shown — but only when it refuses. On success it
# announces the entry point, which would otherwise sit in the middle of the
# report where the next reader takes it for output of the check.
assemble() {  # $1 = link address, $2 = output elf
    asm_rc=0  # set -e: capture the rc instead of dying on it
    "$root/tools/sadhana-native.sh" \
            --स्थान "$1" "$prog" "$2" > "$tmp/asm.log" 2>&1 || asm_rc=$?
    if [ "$asm_rc" -eq 77 ]; then
        echo "SKIPPED: sadhana-native.sh exited 77 (no Stage 1 / wrapper image)"
        exit 77
    fi
    if [ "$asm_rc" -ne 0 ]; then
        echo "  FAIL  compositor.sas does not assemble at $1:"
        sed 's/^/          /' "$tmp/asm.log"
        exit 1
    fi
}

shot() {  # $1 = destination ppm; sets shot_err
    rm -f "$1" 2>/dev/null || true
    shot_err=$(qmp_shot "$tmp/qmp.sock" "$tmp" "$1" 2>&1) || true
}

# Wait for a marker on the guest's serial line. QEMU ends every line with CR;
# without `tr -d '\r'` an anchored match finds NOTHING, which looks exactly like
# a guest that never got there. LC_ALL=C because macOS `tr` aborts on invalid
# UTF-8 and truncates what it has already copied.
await() {  # $1 = marker, $2 = seconds; 0 if seen
    n=0
    while [ "$n" -lt "$2" ]; do
        LC_ALL=C tr -d '\r' < "$tmp/raw" > "$out" 2>/dev/null || true
        if grep -aq "^$1\$" "$out"; then return 0; fi
        n=$((n + 1))
        sleep 1
    done
    LC_ALL=C tr -d '\r' < "$tmp/raw" > "$out" 2>/dev/null || true
    grep -aq "^$1\$" "$out"
}

# Named failures the guest can report about itself. A compositor that dies
# quietly is indistinguishable from one that was never started.
complain() {  # $1 = run number
    for tag in NO-DEVICE NO-QUEUE CMD-FAIL; do
        if grep -aq "^COMPOSITOR-$tag\$" "$out"; then
            echo "  FAIL  run $1 -> the guest reported COMPOSITOR-$tag"
            return 0
        fi
    done
    echo "  FAIL  run $1 -> the guest named no failure of its own; last output:"
    tail -6 "$out" | sed 's/^/          /'
    return 0
}

SAMPLES=12
fail=0

echo "Assembling compositor.sas at two link addresses..."
echo "  assembler     tools/sadhana-native.sh"
assemble ०षोड्८०२००००० "$tmp/comp-a.elf"
assemble ०षोड्८०४००००० "$tmp/comp-b.elf"
echo "  ok            both link addresses assembled"

run_no=0
for pair in "a:0x80200000" "b:0x80400000"; do
    name=${pair%:*}; addr=${pair#*:}
    run_no=$((run_no + 1))
    out=$tmp/$name.out
    echo
    echo "Run $run_no of 2 — linked at $addr"

    rm -f "$tmp/qmp.sock" "$tmp/raw" 2>/dev/null || true
    : > "$tmp/raw"
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/comp-$name.elf" \
        -device virtio-gpu-device \
        -qmp "unix:$tmp/qmp.sock,server=on,wait=off" > "$tmp/raw" 2>&1 &
    qpid=$!
    # Bounded by an explicit SIGKILL from a background `sleep`, not by an alarm:
    # QEMU runs its own timer subsystem and survives SIGALRM. This guest never
    # exits — the framebuffer only exists while the machine does — so an
    # unbounded wait would hang the check instead of failing it. The watcher's
    # own stderr is closed off: the SUBSHELL is what announces "Terminated:
    # sleep 120", and it inherited this script's stderr when it was forked.
    ( sleep 150; kill -9 "$qpid" 2>/dev/null ) 2>/dev/null & watcher=$!

    stop() {
        kill -9 "$qpid" 2>/dev/null || true
        wait "$qpid" 2>/dev/null || true
        # `pkill -P` FIRST: killing the subshell alone orphans its `sleep` to
        # init. By PARENT pid, never by pattern — another agent's QEMU has been
        # seen live on this host.
        pkill -P "$watcher" 2>/dev/null || true
        kill "$watcher" 2>/dev/null || true
        wait "$watcher" 2>/dev/null || true
        LC_ALL=C tr -d '\r' < "$tmp/raw" > "$out" 2>/dev/null || true
    }

    # ---- the control: photograph the deliberate tear ------------------------
    if ! await COMPOSITOR-TEAR-WINDOW 40; then
        complain "$run_no"; fail=1; stop; continue
    fi
    if grep -aq '^COMPOSITOR-TEAR-CLOSED$' "$out"; then
        echo "  FAIL  run $run_no: the torn window had already closed when it was"
        echo "        noticed, so the control could not be photographed. This is"
        echo "        inconclusive, not a pass."
        fail=1; stop; continue
    fi
    shot "$tmp/$name-torn.ppm"
    if [ -n "$shot_err" ] || [ ! -s "$tmp/$name-torn.ppm" ]; then
        echo "  FAIL  run $run_no: no screendump of the control window: ${shot_err:-empty file}"
        fail=1; stop; continue
    fi
    pixels torn "$tmp/$name-torn.ppm" "run $run_no control" || fail=1

    # ---- the measurement: many unsynchronised looks at the loop -------------
    if ! await COMPOSITOR-READY 40; then
        complain "$run_no"; fail=1; stop; continue
    fi

    blues=""
    torn=0
    taken=0
    k=0
    while [ "$k" -lt "$SAMPLES" ]; do
        k=$((k + 1))
        shot "$tmp/shot.ppm"
        if [ -n "$shot_err" ] || [ ! -s "$tmp/shot.ppm" ]; then
            echo "  FAIL  run $run_no sample $k: screendump failed: ${shot_err:-empty file}"
            fail=1; continue
        fi
        taken=$((taken + 1))
        if res=$(pixels clean "$tmp/shot.ppm" "sample $k"); then
            b=$(echo "$res" | sed -n 's/^RESULT //p')
            echo "$res" | grep -v '^RESULT ' || true
            blues="$blues $b"
        else
            echo "$res"
            torn=$((torn + 1))
            fail=1
        fi
    done

    stop

    if [ "$taken" -eq 0 ]; then
        echo "  FAIL  run $run_no: no sample was taken at all."
        fail=1; continue
    fi

    # Was the guest actually compositing while it was being photographed? If the
    # screen were frozen every sample would trivially hold one blue value, and
    # "no tearing" would be a statement about a still picture. Distinct blues
    # across samples is what rules that out.
    distinct=$(echo "$blues" | tr ' ' '\n' | grep -v '^$' | sort -u | wc -l | tr -d ' ')
    echo "  samples       $taken taken, $torn torn, $distinct distinct frame indices seen"
    if [ "$distinct" -lt 2 ]; then
        echo "  FAIL  run $run_no: every sample showed the same frame index, so the"
        echo "        screen never changed while it was being watched. A frozen"
        echo "        screen cannot tear, and proves nothing about one that moves."
        fail=1
    fi

    # The guest's own frame ledger: index and elapsed 10 MHz `time` ticks,
    # printed after every present, as a three-line record.
    #
    # QEMU is stopped with SIGKILL above while the guest is still printing, so
    # the LAST record on the serial line is routinely cut in half — about one
    # run in eight, measured over 24 kills on this host. The record is only ~54
    # bytes and the frame that precedes it is ~8.5 ms of compositing, so the cut
    # is rare per frame and certain to happen eventually. Reading "whatever
    # followed the last tag" therefore reads a HALF-WRITTEN line, and there are
    # two ways that lies rather than fails:
    #
    #   * The elapsed value is printed as sixteen hex digits and is around 4.5
    #     seconds, so its first EIGHT digits are zeros. A tick line cut anywhere
    #     inside that half parses as a perfectly plausible zero — which is how
    #     `in 0.00 s = 0.0 composites+presents/s` reached this report as though
    #     something had measured it. Observed: a run ending on `...00000000b5`
    #     then `00000` and nothing more.
    #   * macOS awk (BWK, 20200816) does not leave a variable untouched when
    #     `getline var` hits EOF; it comes back aliased to its neighbour. A
    #     record cut before its tick line thus returned the index in BOTH
    #     fields, which is the only thing the old `frames_hex = ticks_hex`
    #     guard was ever catching — and it reported "no complete frame record"
    #     for a guest that had printed five hundred of them.
    #
    # So: take the last record that is COMPLETE — the tag, then two lines of
    # exactly sixteen hex digits — and never read past the end of one. A cut
    # tail is not a failure, it is a partial record to be skipped; the record
    # before it was fully written and says the same thing one frame earlier.
    # No getline, so nothing can alias at EOF.
    ledger=$(awk '
        /^COMPOSITOR-FRAME$/ { st = 1; next }
        st == 1 && length($0) == 16 && $0 ~ /^[0-9a-f]+$/ { idx = $0; st = 2; next }
        st == 2 && length($0) == 16 && $0 ~ /^[0-9a-f]+$/ { f = idx; t = $0; st = 0; next }
        { st = 0 }
        END { if (f != "") print f, t }' "$out")
    frames_hex=${ledger%% *}; ticks_hex=${ledger##* }
    if [ -z "$ledger" ] || [ -z "$frames_hex" ] || [ -z "$ticks_hex" ]; then
        echo "  FAIL  run $run_no: the guest printed no complete frame record — no"
        echo "        COMPOSITOR-FRAME tag on the serial line was followed by two"
        echo "        whole 16-digit lines, so there is no ledger to read and the"
        echo "        rate is unmeasured. This is inconclusive, not a pass."
        fail=1; continue
    fi
    # Frame numbering in the loop starts at 3 — frames 1 and 2 belong to the
    # control — so the ledger's last index minus 3 is what the loop presented.
    #
    # The rate is REPORTED, not asserted against a threshold: there is no vsync
    # in this picture to be on time for, as the closing note says. But "reported
    # rather than asserted" is not a licence to print a number nothing measured.
    # A zero or backwards elapsed time is not a fast run, it is an unmeasured
    # one, and it fails here by name instead of being divided into.
    if ! awk -v fh="$frames_hex" -v th="$ticks_hex" -v run="$run_no" '
        function hv(s,  i, v) {
            v = 0
            for (i = 1; i <= length(s); i++) v = v * 16 + index("0123456789abcdef", substr(s, i, 1)) - 1
            return v
        }
        BEGIN {
            f = hv(fh) - 3; t = hv(th)
            if (t <= 0) {
                printf "  FAIL  run %s: the last complete frame record reports %d elapsed\n", run, t
                print "        ticks of the 10 MHz time counter, so how long this run took is"
                print "        not known. The rate is reported here and never asserted, but an"
                print "        unmeasured rate is not 0.0 composites+presents/s and is not"
                print "        printed as one. Inconclusive, not a pass."
                exit 1
            }
            if (f <= 0) {
                printf "  FAIL  run %s: the frame ledger ends at index 0x%s, which is\n", run, fh
                printf "        %d frames past the two the control presented. The tear-free loop\n", f
                print "        presented nothing that this ledger can account for."
                exit 1
            }
            s = t / 1e7
            printf "  presented     %d frames of 1920x1080 in %.2f s = %.1f composites+presents/s\n", f, s, f / s
        }'; then
        fail=1
    fi
    if grep -aq '^COMPOSITOR-CMD-FAIL$' "$out"; then
        echo "  FAIL  run $run_no: the guest reported COMPOSITOR-CMD-FAIL — some"
        echo "        virtio-gpu command was not answered with OK_NODATA (0x1100)."
        fail=1
    fi
done

echo
[ "$fail" -eq 0 ] || {
    echo "the compositor does not present whole frames, or could not be measured."
    exit 1; }

echo "ok  C-010 (the no-tearing half): every presented frame was one composite."
echo
echo "    Two 1920x1080 B8G8R8X8 resources were created over virtio-gpu and backed"
echo "    by two separate 8294400-byte regions of guest memory. Every frame was"
echo "    composited — three layers, painter order — into the region belonging to"
echo "    the resource that was NOT on the scanout, and SET_SCANOUT for it was"
echo "    issued only after the composite finished. Every one of the device's"
echo "    answers was compared against OK_NODATA over a response buffer the guest"
echo "    had filled with 0xff first, in the loop as well as in setup."
echo
echo "    $((SAMPLES * 2)) unsynchronised screendumps were then read off the HOST's copy of the"
echo "    framebuffer, at two link addresses. Each held exactly one distinct blue"
echo "    value across 2073600 pixels — one composite, never a mix — and exactly"
echo "    1173600/300000/600000 pixels of the three layer colours, which is the"
echo "    painter order as well as the geometry. The frame index differed between"
echo "    samples, so the screen was moving while it was watched."
echo
echo "    The control tore on purpose first, and was seen to tear: the same"
echo "    detector read two blue values off the frame the guest had half-repainted"
echo "    under its own scanout."
echo
echo "    NOT PROVED: 60 Hz. The rate printed above is composites-and-presents per"
echo "    second under QEMU's interpreter on this host; there is no display"
echo "    refresh and no vsync anywhere in this arrangement to be on time for."
