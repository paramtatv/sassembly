#!/bin/sh
# Row C-015 — VIRTIO-GPU 2D in .t1: C-009's picture, drawn by spec/darshaka.t1.
#
# ## What is being proved
#
# The .t1 driver (spec/darshaka.t1, unified on Uru's v0.5.0 driver by the
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
# tools/t1-qemu-args.sh applies yantra's own argument rule to a copy of the
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
# Exits 0 on PASS, 1 on any FAIL, 77 when QEMU or nc is absent.
set -eu

# subst1 IN OUT FROM TO [FROM2 TO2]: OUT = IN with the one occurrence of each
# FROM replaced; a FROM not found exactly once in IN refuses (rc 1, OUT unwritten).
subst1() {
  SUB_A=$3 SUB_B=$4 SUB_C=${5-} SUB_D=${6-} SUB_N=$# LC_ALL=C awk '
    function cnt(s, a,   n, i) { n = 0; if (a == "") return -1; while ((i = index(s, a)) > 0) { n++; s = substr(s, i + length(a)) } return n }
    function rep(s, a, b,   i) { i = index(s, a); return substr(s, 1, i - 1) b substr(s, i + length(a)) }
    BEGIN { RS = "\001" }
    { s = $0; got = 1 }
    END {
      a = ENVIRON["SUB_A"]; b = ENVIRON["SUB_B"]; c = ENVIRON["SUB_C"]; d = ENVIRON["SUB_D"]
      if (!got) s = ""
      if (cnt(s, a) != 1 || (ENVIRON["SUB_N"] > 4 && cnt(s, c) != 1)) { print "subst1: anchor not found once" > "/dev/stderr"; exit 1 }
      t = rep(s, a, b)
      if (ENVIRON["SUB_N"] > 4) t = rep(t, c, d)
      printf "%s", t
    }' "$1" > "$2.tmp" && mv "$2.tmp" "$2"
}
# devn N: N in Devanagari digits
devn() { printf '%s' "$1" | LC_ALL=C awk 'BEGIN { split("०,१,२,३,४,५,६,७,८,९", D, ",") } { o = ""; for (i = 1; i <= length($0); i++) o = o D[substr($0, i, 1) + 1]; printf "%s", o }'; }

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
# NOT in $root, for the same reason as check-virtio-gpu.sh: the coordinating
# session gates on a fingerprint of the working tree.
tmp=$(mktemp -d) || exit 77
trap 'rm -rf "$tmp"' EXIT

for t in qemu-system-riscv64 nc; do
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

echo "Resolving yantra-run (SC_YANTRA, else the L28 interpreter oracle)..."
. "$root/tools/ensure-yantra-bins.sh"; ensure_bins "$tmp/bin"   # the in-tree cargo build is gone with crates/yantra (R3-YAN)
T1="$root/tools/t1-build.sh"   # R3-SAD retired t1_image (Rust): same arguments, native route
Y="$tmp/bin/yantra-run"
# P3: T1_RUN picks the runner for the legs that only RUN an image (deferred-completion legs,
# shape legs). The placement oracle and the yantra-vs-QEMU scanout leg keep yantra-run: it IS
# the device under comparison there.
# The L28 interpreter oracle ($Y) has no slot-0 GPU scanout and drops YANTRA_SCANOUT, so the scanout and
# deferred-completion legs run on the native route (tools/t1-run.sh: YANTRA_SCANOUT + YANTRA_VIRTIO_DEFER are native
# on Linux); T1_RUN_REQUIRE_NATIVE=1 makes a silent fallback to an oracle that cannot dump a screen a refusal.
R="${T1_RUN:-$root/tools/t1-run.sh}"; export T1_RUN_REQUIRE_NATIVE=1
qemu-system-riscv64 --version | head -1

# ---- the QMP client --------------------------------------------------------
. "$root/tools/lib/qmp.sh"   # QMP client in sh + nc -U (N2; replaces the python heredoc)

# C-009's assertion on a screen: 64x32, every pixel 0x112233 but (1,1) 0xc47b2a
# (POSIX sh + od + awk; prints a verdict, exit 0 = that picture, 1 = not)
c009check() {
  _n=$(wc -c < "$1" | tr -d ' ')
  _h=$(LC_ALL=C head -c 13 "$1" | od -An -v -tx1 | tr -d ' \n')
  if [ "$_h" != 50360a36342033320a3235350a ] || [ "$_n" -ne 6157 ]; then
    echo "not a 64x32 P6 screen: header $_h, $_n octets"; return 1
  fi
  LC_ALL=C tail -c +14 "$1" | od -An -v -tx1 | awk '
    { for (j = 1; j <= NF; j++) { c = c $j; if (++m == 3) { i++
        if (c != "112233") { k++; if (k <= 5) o = o " (" (i - 1) % 64 "," int((i - 1) / 64) ")"; if (i == 66) w = c }
        c = ""; m = 0 } } }
    END {
      if (k != 1 || o != " (1,1)" || w != "c47b2a") {
        print "pixels off 0x112233 at" o " (want only (1,1), 0xc47b2a)"; exit 1 }
      print "64x32 of 0x112233, one 0xc47b2a at (1,1)" }'
}

# ---- C-009's screen, by C-009's driver --------------------------------------
echo
echo "C-009's screen: spec/virtio-gpu.sas under OpenSBI, QMP screendump..."
asm_rc=0
"$root/tools/sadhana-native.sh" --स्थान ०षोड्८०२००००० "$root/spec/virtio-gpu.sas" "$tmp/c009.elf" >/dev/null || asm_rc=$?
[ "$asm_rc" -eq 77 ] && { echo "  SKIP  no native assembler image (exit 77)"; exit 77; }
[ "$asm_rc" -eq 0 ] || exit "$asm_rc"
rm -f "$tmp/c009.sock"
qemu-system-riscv64 -machine virt -nographic -bios default -kernel "$tmp/c009.elf" \
    -device virtio-gpu-device \
    -qmp "unix:$tmp/c009.sock,server=on,wait=off" > "$tmp/c009.raw" 2>&1 &
qpid=$!
( sleep 40; kill -9 "$qpid" 2>/dev/null ) 2>/dev/null & watcher=$!
n=0
while [ "$n" -lt 30 ]; do
  LC_ALL=C tr -d '\r' < "$tmp/c009.raw" > "$tmp/c009.out" 2>/dev/null || true
  if grep -a -q '^VIRTIO-GPU-DONE$' "$tmp/c009.out"; then break; fi
  n=$((n + 1)); sleep 1
done
if grep -a -q '^VIRTIO-GPU-DONE$' "$tmp/c009.out"; then
  qmp_shot "$tmp/c009.sock" "$tmp" "$tmp/c009.ppm" ppm >/dev/null 2>&1 || true
fi
kill -9 "$qpid" 2>/dev/null || true
wait "$qpid" 2>/dev/null || true
# by PARENT pid, never by pattern (W-044/W-047/W-049, W-096)
pkill -P "$watcher" 2>/dev/null || true
kill "$watcher" 2>/dev/null || true
wait "$watcher" 2>/dev/null || true
if [ -s "$tmp/c009.ppm" ] && why=$(c009check "$tmp/c009.ppm"); then
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
  addr=$(sh "$root/tools/t1-qemu-args.sh" --ram $RAM "$1" "$1.q" "$1.args" "$1" screen \
      | sed -n 's/^addr //p')
  rm -f "$tmp/t1.sock" "$2"
  qemu-system-riscv64 -machine virt -bios none -m 768M -display none -kernel "$1.q" \
      $gpu -device "loader,file=$1.args,addr=$addr" \
      -action shutdown=pause -serial "file:$1.serial" -monitor none \
      -qmp "unix:$tmp/t1.sock,server=on,wait=off" > /dev/null 2>&1 &
  qpid=$!
  ( sleep 90; kill -9 "$qpid" 2>/dev/null ) 2>/dev/null & watcher=$!
  qmp_pause "$tmp/t1.sock" "$tmp" "$2" > "$1.qmp" 2>&1 || true
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
grep -a '^predict:' "$tmp/draw.elf.build.log" | sed 's/^/  build /'

# the injector, against yantra-run's own placement of the same arguments
want=$(YANTRA_RAM=$RAM "$Y" "$tmp/draw.elf" screen 2>&1 | sed -n 's/.*run at \(0x[0-9a-f]*\).*/\1/p')
YANTRA_RAM=$RAM YANTRA_SCANOUT="$tmp/y.ppm" "$R" "$tmp/draw.elf" screen > "$tmp/y.out" 2>&1 || true
got=$(sh "$root/tools/t1-qemu-args.sh" --ram $RAM "$tmp/draw.elf" "$tmp/x.elf" "$tmp/x.bin" \
    "$tmp/draw.elf" screen | sed -n 's/^run //p')
if [ -n "$want" ] && [ "$want" = "$got" ]; then pass "t1-qemu-args.sh places the run where yantra-run does ($got)"
else red "t1-qemu-args.sh says ${got:-nothing}, yantra-run says ${want:-nothing}"; fi

qemu_t1 "$tmp/draw.elf" "$tmp/t1.ppm" draw
if grep -a -q '^legacy true$' "$tmp/draw.elf.qmp"; then
  echo "  (QEMU's own default: force-legacy = true, not set by this script)"
fi
[ "$G" = 0 ] && pass "QEMU: the driver says G 0" || red "QEMU: the driver says G ${G:-<none>}"
if [ -s "$tmp/t1.ppm" ] && why=$(c009check "$tmp/t1.ppm"); then
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

# ---- deferred completion (the asynchronous device is QEMU) -------------------
# yantra's device used to complete a chain INSIDE the notify store, which hid the
# driver's single read of the used index; QEMU completes on its own clock and
# failed it (G 11). The native slot-0 GPU (tools/native-spike/scanout_dev.py) is
# synchronous BY DESIGN: it REFUSES when YANTRA_VIRTIO_DEFER is in the environment
# (scanout_dev.py "YANTRA_VIRTIO_DEFER is set"), and t1-run.sh passes the deferral
# clock only to the frames device (--net), never to slot 0's GPU. So "yantra with
# deferred completion" has no native twin (GAP, below); the hazard is proven where the
# deferral is real: the polling driver is G 0 on QEMU (the draw leg above), and the
# single-read driver, its bound cut to one read, must go red on QEMU. The control —
# the same single-read driver on the synchronous native device — must say G 0,
# with YANTRA_VIRTIO_DEFER UNSET (set to 0 it still refuses).
# GAP: yantra's deferred-GPU model (YANTRA_VIRTIO_DEFER + YANTRA_SCANOUT) is not native.
echo
echo "deferred completion: the single-read driver on QEMU, the control on the synchronous native device..."
onedrv="$tmp/one-read.drv.t1"
subst1 "$root/spec/darshaka.t1" "$onedrv" "यावत् पठनानि न्यूनम् १००००००० आदि" "यावत् पठनानि न्यूनम् १ आदि"
if build "$root/spec/virtio-gpu-draw.t1" "$tmp/one-read.elf" "$onedrv"; then
  rm -f "$tmp/one-sync.ppm"
  env -u YANTRA_VIRTIO_DEFER YANTRA_RAM=$RAM YANTRA_SCANOUT="$tmp/one-sync.ppm" "$R" "$tmp/one-read.elf" screen > "$tmp/one-sync.out" 2>&1 || true
  G=$(sed -n 's/^G //p' "$tmp/one-sync.out")
  [ "$G" = 0 ] && pass "one-read driver, synchronous native device: G 0 — the hazard a synchronous device hides" \
    || red "one-read driver, synchronous native device: G ${G:-<none>}, want 0 (the control)"
  qemu_t1 "$tmp/one-read.elf" "$tmp/one-qemu.ppm" one-read
  case "$G" in
    1[0-5]) pass "one-read driver, QEMU's own clock: caught as G $G (no used entry), not drawn" ;;
    *) red "one-read driver, QEMU's own clock: G ${G:-<none>}, want 10+k" ;;
  esac
else red "the one-read driver did not build"; fi

mutant() {  # $1 = label, $2 = draw|driver (the file edited), $3 = from, $4 = to
  label=$1; which=$2; shift 2
  src="$root/spec/virtio-gpu-draw.t1"; drv="$root/spec/darshaka.t1"
  if [ "$which" = driver ]; then orig=$drv; drv="$tmp/m-$label.drv.t1"; out=$drv
  else orig=$src; src="$tmp/m-$label.t1"; out=$src; fi
  set -- "$label" "$1" "$2"
  subst1 "$orig" "$out" "$2" "$3"
  if ! build "$src" "$tmp/m-$1.elf" "$drv"; then red "mutant $1 did not build"; return; fi
  qemu_t1 "$tmp/m-$1.elf" "$tmp/m-$1.ppm" "$1"
  if [ "$G" = 0 ] && [ -s "$tmp/m-$1.ppm" ] && cmp -s "$tmp/m-$1.ppm" "$tmp/c009.ppm"; then
    red "mutant $1 NOT caught: G 0 and C-009's screen"
  else
    pass "mutant $1 caught: G ${G:-<none>}, $(c009check "$tmp/m-$1.ppm" 2>&1 || true)"
  fi
}
echo
echo "Refusals, each by its own code..."
qemu_t1 "$tmp/draw.elf" "$tmp/nogpu.ppm" no-gpu nogpu
[ "$G" = 3 ] && pass "no GPU on the machine: G 3" || red "no GPU on the machine: G ${G:-<none>}, want 3"
# width 65 on a 64x32 picture is the 65x32 shape with a 2048-word array, so
# it is made by hand: the array is C-009's, the call one column wider
src="$tmp/w65.t1"
subst1 "$root/spec/virtio-gpu-draw.t1" "$src" "आरभ्य पटः ऽ ६४ ऽ ३२ समाप्तम्" "आरभ्य पटः ऽ ६५ ऽ ३२ समाप्तम्"
if build "$src" "$tmp/w65.elf"; then
  qemu_t1 "$tmp/w65.elf" "$tmp/w65.ppm" width-65
  [ "$G" = 9 ] && pass "width 65 on a 64x32 picture: G 9" || red "width 65 on a 64x32 picture: G ${G:-<none>}, want 9"
else red "the width-65 program did not build"; fi

# a picture of W x H words (C-009's colours, its odd pixel still word 65) on
# both devices, each of which must answer the code expected
shape() {  # $1 = W, $2 = H, $3 = expected G
  lbl="${1}x${2}"; src="$tmp/s-$lbl.t1"
  subst1 "$root/spec/virtio-gpu-draw.t1" "$src" "न्यूनम् २०४८ आदि" "न्यूनम् $(devn $(($1 * $2))) आदि" "आरभ्य पटः ऽ ६४ ऽ ३२ समाप्तम्" "आरभ्य पटः ऽ $(devn "$1") ऽ $(devn "$2") समाप्तम्"
  if ! build "$src" "$tmp/s-$lbl.elf"; then red "shape $lbl did not build"; return; fi
  qemu_t1 "$tmp/s-$lbl.elf" "$tmp/s-$lbl.ppm" "$lbl"
  qg=$G
  yg=$(YANTRA_RAM=$RAM YANTRA_STEPS=4000000000 YANTRA_SCANOUT="$tmp/s-$lbl.y.ppm" "$R" "$tmp/s-$lbl.elf" screen 2>&1 \
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
