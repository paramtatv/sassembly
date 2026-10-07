#!/bin/sh
# Row C-015 — VIRTIO-GPU 2D in .t1: C-009's picture, drawn by spec/darshaka.t1.
#
# ## What is being proved
#
# The .t1 driver (spec/darshaka.t1, unified on the GPU-driver project's v0.5.0 driver by the
# owner's ruling of 2026-10-06), called by spec/virtio-gpu-draw.t1 with C-009's
# own picture — 64x32 of 0x112233 and one pixel of 0xc47b2a at (1,1) — puts on
# QEMU's REAL virtio-gpu a screen BYTE-IDENTICAL to the one C-009's .sas driver
# (spec/virtio-gpu.sas) puts there. Both screens are QMP screendumps of QEMU's
# own copy of the framebuffer, taken in this run, so neither is a number written
# down once and trusted since. The comparison is of octets, which subsumes
# C-009's histogram-and-place assertion; that assertion is ALSO made on both
# screens, so a failure says which half of the picture is wrong.
#
# Then the same image on yantra's device model (YANTRA_SCANOUT) must give the
# same octets: two devices that share no code, one picture.
#
# ## Why the GPU is not pinned to a slot
#
# QEMU's virt machine fills its eight virtio-mmio slots from the TOP, so an
# unpinned `-device virtio-gpu-device` sits in the last slot, as in C-009's own
# run. The driver must find it by scanning; yantra's GPU is in slot 0. One
# driver passing both is the scan's falsifier.
#
# ## The transport
#
# The driver speaks LEGACY virtio-mmio (row C-015's port decision of
# 2026-10-02). This script does NOT set virtio-mmio's force-legacy: QEMU's own
# default stands, and the run prints it from `info qtree`.
#
# ## Arguments, and why the guest does not spin
#
# The draw program reaches the device only when given an argument — the
# build-time interpreter must not touch a device — and QEMU has no argv, so
# tools/t1-qemu-args.py applies yantra's own argument rule to a copy of the
# image; it is first checked against yantra-run's "run at" for the same image
# and RAM. Unlike C-009's guest, this one halts through the finisher;
# `-action shutdown=pause` keeps the display, and the screendump is taken from
# the paused machine.
#
# ## Mutants, each of which must go red
#
#   odd pixel one word late   — C-009's stride falsifier, moved into the picture
#   CREATE_2D one row short   — the driver's resource height off by one (QEMU
#                               then refuses SET_SCANOUT's larger rectangle)
#   ONE READ of the used index — the driver C-015's poll replaced, on yantra with
#                               YANTRA_VIRTIO_DEFER=1000: G 10+k (and, as the
#                               control, G 0 on a synchronous yantra)
#
# ## Refusals, each of which must name its own code
#
#   no GPU on the machine     — G 3 (no virt slot holds DeviceID 16), and the
#                               scan reads all eight slots without a fault
#   width 65 on a 64x32 frame — G 9 (the shape refused before any store)
#
# ## Shape bounds, the SAME code required of BOTH devices
#
#   16x128   — G 0: 16 is the smallest side QEMU's virtio-gpu takes
#   15x32    — G 9: a side under 16 (QEMU would refuse SET_SCANOUT as G 23
#              where yantra's model might draw — the driver refuses first)
#   4096x4096 — G 9: the frame would pass 0xAA00_0000, past the 672 MiB the
#              driver states (W x H at most 8,372,224)
#
# Exits 0 on PASS, 1 on any FAIL, 77 when QEMU or python3 is absent.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
# NOT in $root, for the same reason as check-virtio-gpu.sh: the coordinating
# session gates on a fingerprint of the working tree.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

for t in qemu-system-riscv64 python3; do
  command -v "$t" >/dev/null 2>&1 || {
    echo "SKIPPED: $t is not installed"
    echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
    exit 77; }
done

RAM=738197504                     # the driver's region is at 0xA800_0000
fail=0
# printf, not echo: dash's echo expands the backslashes in a repr()
pass() { printf '  PASS  %s\n' "$*"; }
red()  { printf '  FAIL  %s\n' "$*"; fail=1; }

echo "Building t1_image and yantra-run (release)..."
cargo build --quiet --release --manifest-path "$root/Cargo.toml" --bin t1_image --bin yantra-run
T1="$root/target/release/t1_image"
Y="$root/target/release/yantra-run"
qemu-system-riscv64 --version | head -1

# ---- the QMP client --------------------------------------------------------
cat > "$tmp/qmp.py" <<'PY'
import json, socket, sys, time
mode, sock_path, out = sys.argv[1], sys.argv[2], sys.argv[3]
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
for _ in range(200):
    try:
        s.connect(sock_path); break
    except OSError:
        time.sleep(0.05)
f = s.makefile("rw")
json.loads(f.readline())
def cmd(name, **args):
    f.write(json.dumps({"execute": name, "arguments": args}) + "\n"); f.flush()
    while True:
        r = json.loads(f.readline())
        if "return" in r or "error" in r:
            return r
cmd("qmp_capabilities")
if mode == "pause":
    # the transport's force-legacy, from QEMU's own device tree: the
    # virtio-mmio block whose bus carries the GPU
    tree = cmd("human-monitor-command", **{"command-line": "info qtree"})["return"]
    legacy, block = "unknown", []
    lines = tree.splitlines()
    for i, line in enumerate(lines):
        t = line.strip()
        if t.startswith("dev: virtio-mmio"):
            block = []
        block.append(t)
        if t.startswith("dev: virtio-gpu-device"):
            legacy = next((b.split("= ")[1] for b in block if b.startswith("force-legacy")), "unknown")
    print("legacy", legacy)
    t0 = time.time()
    status = "?"
    while time.time() - t0 < 60:
        status = cmd("query-status")["return"]["status"]
        if status != "running":
            break
        time.sleep(0.2)
    print("status", status)
r = cmd("screendump", filename=out, format="ppm")
print("dump", "ok" if "return" in r else json.dumps(r.get("error")))
if mode == "pause":
    cmd("quit")
PY

# C-009's assertion on a screen: 64x32, every pixel 0x112233 but (1,1) 0xc47b2a
cat > "$tmp/c009.py" <<'PY'
import sys
b = open(sys.argv[1], "rb").read()
hdr = b"P6\n64 32\n255\n"
if not b.startswith(hdr) or len(b) != len(hdr) + 64 * 32 * 3:
    print("not a 64x32 P6 screen: header %r, %d octets" % (b[:16], len(b))); sys.exit(1)
px = b[len(hdr):]
odd = [(i % 64, i // 64) for i in range(64 * 32) if px[3 * i:3 * i + 3] != b"\x11\x22\x33"]
if odd != [(1, 1)] or px[3 * 65:3 * 65 + 3] != b"\xc4\x7b\x2a":
    print("pixels off 0x112233 at %s (want only (1,1), 0xc47b2a)" % odd[:5]); sys.exit(1)
print("64x32 of 0x112233, one 0xc47b2a at (1,1)")
PY

# ---- C-009's screen, by C-009's driver --------------------------------------
echo
echo "C-009's screen: spec/virtio-gpu.sas under OpenSBI, QMP screendump..."
cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
    --स्थान ०षोड्८०२००००० "$root/spec/virtio-gpu.sas" "$tmp/c009.elf" >/dev/null
rm -f "$tmp/c009.sock"
qemu-system-riscv64 -machine virt -nographic -bios default -kernel "$tmp/c009.elf" \
    -device virtio-gpu-device \
    -qmp "unix:$tmp/c009.sock,server=on,wait=off" > "$tmp/c009.raw" 2>&1 &
qpid=$!
( sleep 40; kill -9 "$qpid" 2>/dev/null ) 2>/dev/null & watcher=$!
n=0
while [ "$n" -lt 30 ]; do
  LC_ALL=C tr -d '\r' < "$tmp/c009.raw" > "$tmp/c009.out" 2>/dev/null || true
  if grep -q '^VIRTIO-GPU-DONE$' "$tmp/c009.out"; then break; fi
  n=$((n + 1)); sleep 1
done
if grep -q '^VIRTIO-GPU-DONE$' "$tmp/c009.out"; then
  python3 "$tmp/qmp.py" shot "$tmp/c009.sock" "$tmp/c009.ppm" >/dev/null 2>&1 || true
fi
kill -9 "$qpid" 2>/dev/null || true
wait "$qpid" 2>/dev/null || true
# by PARENT pid, never by pattern (W-044/W-047/W-049, W-096)
pkill -P "$watcher" 2>/dev/null || true
kill "$watcher" 2>/dev/null || true
wait "$watcher" 2>/dev/null || true
if [ -s "$tmp/c009.ppm" ] && why=$(python3 "$tmp/c009.py" "$tmp/c009.ppm"); then
  pass "C-009's own screen: $why"
else
  red "C-009's own screen could not be taken or is not its picture: ${why:-no screendump}"
  echo "RED: there is no reference screen to compare with."; exit 1
fi

# ---- the .t1 image, and its mutants ----------------------------------------
build() {  # $1 = draw source, $2 = elf [, $3 = driver source]
  drv=${3:-$root/spec/darshaka.t1}
  "$T1" --spec-root "$root/spec" --compiler "$root/crates/sadhana-t1/src" \
      --load "$drv" --load "$1" --entry प्रथमबिन्दु मुख्यम् \
      -o "$2" "$drv" "$1" > "$2.build.log" 2>&1
  [ -s "$2" ]
}

# one QEMU run of a .t1 image: G line, transport, screen at $2; a fourth
# argument "nogpu" leaves the GPU off the machine
qemu_t1() {  # $1 = elf, $2 = ppm, $3 = label [, nogpu]
  gpu="-device virtio-gpu-device"
  [ "${4:-}" = nogpu ] && gpu=
  addr=$(python3 "$root/tools/t1-qemu-args.py" --ram $RAM "$1" "$1.q" "$1.args" "$1" screen \
      | sed -n 's/^addr //p')
  rm -f "$tmp/t1.sock" "$2"
  qemu-system-riscv64 -machine virt -bios none -m 768M -display none -kernel "$1.q" \
      $gpu -device "loader,file=$1.args,addr=$addr" \
      -action shutdown=pause -serial "file:$1.serial" -monitor none \
      -qmp "unix:$tmp/t1.sock,server=on,wait=off" > /dev/null 2>&1 &
  qpid=$!
  ( sleep 90; kill -9 "$qpid" 2>/dev/null ) 2>/dev/null & watcher=$!
  python3 "$tmp/qmp.py" pause "$tmp/t1.sock" "$2" > "$1.qmp" 2>&1 || true
  wait "$qpid" 2>/dev/null || true
  pkill -P "$watcher" 2>/dev/null || true
  kill "$watcher" 2>/dev/null || true
  wait "$watcher" 2>/dev/null || true
  G=$(LC_ALL=C tr -d '\r' < "$1.serial" | sed -n 's/^G //p')
  echo "  qemu $3: $(tr '\n' ' ' < "$1.qmp")G ${G:-<none>}"
}

echo
echo "C-015: spec/virtio-gpu-draw.t1 + spec/darshaka.t1..."
build "$root/spec/virtio-gpu-draw.t1" "$tmp/draw.elf" || {
  red "the draw program did not build:"; tail -20 "$tmp/draw.elf.build.log"; exit 1; }
grep '^predict:' "$tmp/draw.elf.build.log" | sed 's/^/  build /'

# the injector, against yantra-run's own placement of the same arguments
want=$(YANTRA_RAM=$RAM YANTRA_SCANOUT="$tmp/y.ppm" "$Y" "$tmp/draw.elf" screen 2>&1 \
    | tee "$tmp/y.out" | sed -n 's/.*run at \(0x[0-9a-f]*\).*/\1/p')
got=$(python3 "$root/tools/t1-qemu-args.py" --ram $RAM "$tmp/draw.elf" "$tmp/x.elf" "$tmp/x.bin" \
    "$tmp/draw.elf" screen | sed -n 's/^run //p')
if [ -n "$want" ] && [ "$want" = "$got" ]; then pass "t1-qemu-args.py places the run where yantra-run does ($got)"
else red "t1-qemu-args.py says ${got:-nothing}, yantra-run says ${want:-nothing}"; fi

qemu_t1 "$tmp/draw.elf" "$tmp/t1.ppm" draw
if grep -q '^legacy true$' "$tmp/draw.elf.qmp"; then
  echo "  (QEMU's own default: force-legacy = true, not set by this script)"
fi
[ "$G" = 0 ] && pass "QEMU: the driver says G 0" || red "QEMU: the driver says G ${G:-<none>}"
if [ -s "$tmp/t1.ppm" ] && why=$(python3 "$tmp/c009.py" "$tmp/t1.ppm"); then
  pass "QEMU screen, C-009's assertion: $why"
else red "QEMU screen, C-009's assertion: ${why:-no screendump}"; fi
if [ -s "$tmp/t1.ppm" ] && cmp -s "$tmp/t1.ppm" "$tmp/c009.ppm"; then
  pass "QEMU screen BYTE-IDENTICAL to C-009's ($(wc -c < "$tmp/t1.ppm" | tr -d ' ') octets)"
else red "QEMU screen differs from C-009's: $(cmp "$tmp/t1.ppm" "$tmp/c009.ppm" 2>&1 | head -1)"; fi

YG=$(sed -n 's/^G //p' "$tmp/y.out")
[ "$YG" = 0 ] && pass "yantra: the driver says G 0" || red "yantra: the driver says G ${YG:-<none>}"
if [ -s "$tmp/y.ppm" ] && cmp -s "$tmp/y.ppm" "$tmp/c009.ppm"; then
  pass "yantra's scanout BYTE-IDENTICAL to C-009's QEMU screen"
else red "yantra's scanout differs from C-009's QEMU screen"; fi

# ---- deferred completion on yantra (YANTRA_VIRTIO_DEFER, b0232dcb) ----------
# yantra's device used to complete a chain INSIDE the notify store, which hid the
# driver's single read of the used index; QEMU completes on its own clock and
# failed it (G 11). With the device answering 1000 instructions after the
# notify, yantra shows the same hazard, so the bounded poll is proven on both
# devices and the single-read driver is red on yantra too.
echo
echo "yantra with deferred completion (YANTRA_VIRTIO_DEFER=1000)..."
yrun() {  # $1 = elf, $2 = ppm, $3 = defer; leaves the run in $2.out, prints G
  rm -f "$2"
  # yantra-run exits 1 on a non-zero finisher (G 10 is one); the G line is the
  # verdict, so its exit status must not end this script under set -e
  YANTRA_VIRTIO_DEFER=$3 YANTRA_RAM=$RAM YANTRA_SCANOUT="$2" "$Y" "$1" screen > "$2.out" 2>&1 || true
  sed -n 's/^G //p' "$2.out"
}
G=$(yrun "$tmp/draw.elf" "$tmp/yd.ppm" 1000)
if grep -q 'completions deferred by 1000 instruction' "$tmp/yd.ppm.out"; then
  pass "deferred: yantra says its completions are deferred by 1000 instructions"
else red "deferred: yantra did not announce the deferral — the run may have been synchronous"; fi
if [ "$G" = 0 ] && [ -s "$tmp/yd.ppm" ] && cmp -s "$tmp/yd.ppm" "$tmp/c009.ppm"; then
  pass "deferred: the polling driver says G 0 and its scanout is C-009's, byte for byte"
else red "deferred: the polling driver says G ${G:-<none>} (want 0 and C-009's screen)"; fi
# the SINGLE-READ driver: the poll's bound cut to one read
onedrv="$tmp/one-read.drv.t1"
python3 - "$root/spec/darshaka.t1" "$onedrv" <<'PY'
import sys
s = open(sys.argv[1]).read()
a = "यावत् पठनानि न्यूनम् १००००००० आदि"
assert s.count(a) == 1, "the poll's bound is not found once"
open(sys.argv[2], "w").write(s.replace(a, "यावत् पठनानि न्यूनम् १ आदि"))
PY
if build "$root/spec/virtio-gpu-draw.t1" "$tmp/one-read.elf" "$onedrv"; then
  G=$(yrun "$tmp/one-read.elf" "$tmp/one-sync.ppm" 0)
  [ "$G" = 0 ] && pass "one-read driver, synchronous yantra: G 0 — the hazard a synchronous device hides" \
    || red "one-read driver, synchronous yantra: G ${G:-<none>}, want 0 (the control)"
  G=$(yrun "$tmp/one-read.elf" "$tmp/one-defer.ppm" 1000)
  case "$G" in
    1[0-5]) pass "one-read driver, deferred yantra: caught as G $G (no used entry), not drawn" ;;
    *) red "one-read driver, deferred yantra: G ${G:-<none>}, want 10+k" ;;
  esac
else red "the one-read driver did not build"; fi

mutant() {  # $1 = label, $2 = draw|driver (the file edited), $3 = from, $4 = to
  label=$1; which=$2; shift 2
  src="$root/spec/virtio-gpu-draw.t1"; drv="$root/spec/darshaka.t1"
  if [ "$which" = driver ]; then orig=$drv; drv="$tmp/m-$label.drv.t1"; out=$drv
  else orig=$src; src="$tmp/m-$label.t1"; out=$src; fi
  set -- "$label" "$1" "$2"
  python3 - "$orig" "$out" "$2" "$3" <<'PY'
import sys
s = open(sys.argv[1]).read()
assert s.count(sys.argv[3]) == 1, "mutant anchor not found once"
open(sys.argv[2], "w").write(s.replace(sys.argv[3], sys.argv[4]))
PY
  if ! build "$src" "$tmp/m-$1.elf" "$drv"; then red "mutant $1 did not build"; return; fi
  qemu_t1 "$tmp/m-$1.elf" "$tmp/m-$1.ppm" "$1"
  if [ "$G" = 0 ] && [ -s "$tmp/m-$1.ppm" ] && cmp -s "$tmp/m-$1.ppm" "$tmp/c009.ppm"; then
    red "mutant $1 NOT caught: G 0 and C-009's screen"
  else
    pass "mutant $1 caught: G ${G:-<none>}, $(python3 "$tmp/c009.py" "$tmp/m-$1.ppm" 2>&1 || true)"
  fi
}
echo
echo "Refusals, each by its own code..."
qemu_t1 "$tmp/draw.elf" "$tmp/nogpu.ppm" no-gpu nogpu
[ "$G" = 3 ] && pass "no GPU on the machine: G 3" || red "no GPU on the machine: G ${G:-<none>}, want 3"
# width 65 on a 64x32 picture is the 65x32 shape with a 2048-word array, so
# it is made by hand: the array is C-009's, the call one column wider
src="$tmp/w65.t1"
python3 - "$root/spec/virtio-gpu-draw.t1" "$src" <<'PY'
import sys
s = open(sys.argv[1]).read()
a = "आरभ्य पटः ऽ ६४ ऽ ३२ समाप्तम्"
assert s.count(a) == 1
open(sys.argv[2], "w").write(s.replace(a, "आरभ्य पटः ऽ ६५ ऽ ३२ समाप्तम्"))
PY
if build "$src" "$tmp/w65.elf"; then
  qemu_t1 "$tmp/w65.elf" "$tmp/w65.ppm" width-65
  [ "$G" = 9 ] && pass "width 65 on a 64x32 picture: G 9" || red "width 65 on a 64x32 picture: G ${G:-<none>}, want 9"
else red "the width-65 program did not build"; fi

# a picture of W x H words (C-009's colours, its odd pixel still word 65) on
# both devices, each of which must answer the code expected
shape() {  # $1 = W, $2 = H, $3 = expected G
  lbl="${1}x${2}"; src="$tmp/s-$lbl.t1"
  python3 - "$root/spec/virtio-gpu-draw.t1" "$src" "$1" "$2" <<'PY'
import sys
s = open(sys.argv[1]).read()
dev = lambda n: "".join("०१२३४५६७८९"[int(c)] for c in str(n))
w, h = int(sys.argv[3]), int(sys.argv[4])
a, b = "न्यूनम् २०४८ आदि", "आरभ्य पटः ऽ ६४ ऽ ३२ समाप्तम्"
assert s.count(a) == 1 and s.count(b) == 1
s = s.replace(a, "न्यूनम् %s आदि" % dev(w * h))
s = s.replace(b, "आरभ्य पटः ऽ %s ऽ %s समाप्तम्" % (dev(w), dev(h)))
open(sys.argv[2], "w").write(s)
PY
  if ! build "$src" "$tmp/s-$lbl.elf"; then red "shape $lbl did not build"; return; fi
  qemu_t1 "$tmp/s-$lbl.elf" "$tmp/s-$lbl.ppm" "$lbl"
  qg=$G
  yg=$(YANTRA_RAM=$RAM YANTRA_STEPS=4000000000 YANTRA_SCANOUT="$tmp/s-$lbl.y.ppm" "$Y" "$tmp/s-$lbl.elf" screen 2>&1 \
      | sed -n 's/^G //p')
  if [ "$qg" = "$3" ] && [ "$yg" = "$3" ]; then pass "shape $lbl: G $3 on QEMU and on yantra"
  else red "shape $lbl: QEMU G ${qg:-<none>}, yantra G ${yg:-<none>}, want G $3 on both"; fi
}
shape 16 128 0
shape 15 32 9
shape 4096 4096 9

echo
echo "Mutants, each of which must go red on QEMU..."
mutant odd-pixel-late draw "पटः अङ्कः ६५ अन्तः भवति १२८७६५८६ ।" "पटः अङ्कः ६६ अन्तः भवति १२८७६५८६ ।"
mutant create-height-short driver "        शब्दाः अङ्कः ९ अन्तः भवति उच्चता ।
        प्रत्यागमनम् ४० ।" "        चरः न्यूनोच्चता ॱॱ न६४ भवति उच्चता वियोगः १ ।
        शब्दाः अङ्कः ९ अन्तः भवति न्यूनोच्चता ।
        प्रत्यागमनम् ४० ।"

echo
if [ "$fail" -eq 0 ]; then echo "PASS: C-015 — the .t1 driver puts C-009's screen on QEMU's virtio-gpu, and on yantra's"; exit 0; fi
echo "FAIL: C-015"; exit 1
