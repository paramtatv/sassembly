#!/bin/sh
# Host entropy for Sassembly programs (runtime completeness item 1): spec/entropy/*.t1 under yantra (through
# tools/yantra-entropy.sh) and as a native translation on THIS host. See the header of
# spec/entropy/yadrucchika-srotah.t1 for the protocol (the reserved name ".entropy" in the file window).
#
# usage: tools/check-native-entropy.sh [ORACLE.py]   (BIN=dir holding t1_image and yantra-run; default target/release)
# Exits 0 on PASS, 1 on FAIL, 77 when a tool is absent. A native leg that cannot run natively is a FAIL. Entropy is not deterministic: nothing here compares
# two live runs for equality except a REPLAY with its record.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
BIN=${BIN:-$root/target/release}
ORACLE=${1:-$root/tools/native-spike/anuvada.py}
for t in "$BIN/t1_image" "$BIN/yantra-run"; do [ -x "$t" ] || { echo "SKIPPED: $t absent"; exit 77; }; done
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
d=$root/spec/entropy
build() { # NAME FILES...
  n=$1; shift; load=""; for f in "$@"; do load="$load --load $f"; done
  # shellcheck disable=SC2086
  "$BIN/t1_image" --spec-root "$root/spec" --compiler "$root/crates/sadhana-t1/src" $load \
    --entry यादृच्छिकपरीक्षा मुख्यम् -o "$tmp/$n.elf" "$@" > "$tmp/$n.build.log" 2>&1 || { echo "FAIL build $n"; tail -5 "$tmp/$n.build.log"; exit 1; }
}
build ent "$d/yadrucchika-srotah.t1" "$d/yadrucchika-pariksha.t1"
build gpu "$d/gpu-probe.t1"
fail=0
ok() { echo "  PASS  $1"; }
bad() { echo "  FAIL  $1"; fail=1; }
dist() { tail -n +2 "$1" | sort -u | wc -l; }
halves_differ() { python3 - "$1" <<'PY'
import sys
v = [l for l in open(sys.argv[1]).read().split("\n") if l.strip()][1:]
sys.exit(0 if len(v) == 64 and v[:32] != v[32:] else 1)
PY
}
mkdir -p "$tmp/root"; Y=$BIN/yantra-run

# ---- yantra, through the launcher
sh "$root/tools/yantra-entropy.sh" --record "$tmp/rec.bin" "$tmp/root" "$Y" "$tmp/ent.elf" x > "$tmp/y.out" 2>/dev/null || true
[ "$(head -1 "$tmp/y.out")" = 0 ] && [ "$(dist "$tmp/y.out")" -gt 8 ] && ok "yantra: status 0, 64 octets, $(dist "$tmp/y.out") distinct" || bad "yantra draw"
halves_differ "$tmp/y.out" && ok "yantra: the second draw differs from the first (the cursor)" || bad "yantra cursor"
[ ! -e "$tmp/root/.entropy" ] && ok "yantra: the pool is removed after the run" || bad "pool left behind"
sh "$root/tools/yantra-entropy.sh" --replay "$tmp/rec.bin" "$tmp/root" "$Y" "$tmp/ent.elf" x > "$tmp/y2.out" 2>/dev/null || true
cmp -s "$tmp/y.out" "$tmp/y2.out" && ok "yantra: replay of the record is exact" || bad "yantra replay"
sh "$root/tools/yantra-entropy.sh" --pool 40 "$tmp/root" "$Y" "$tmp/ent.elf" x > "$tmp/ys.out" 2>/dev/null || true
[ "$(head -1 "$tmp/ys.out")" = 64172851 ] && ok "yantra: a short pool is refusal 0x3D33333, no zeros" || bad "yantra short pool"
"$Y" --files "$tmp/root" "$tmp/ent.elf" x > "$tmp/ym.out" 2>/dev/null || true
[ "$(head -1 "$tmp/ym.out")" = 64107315 ] && ok "yantra: a missing pool is refusal 0x3D23333" || bad "yantra missing pool"

# ---- native on this host
case "$(uname -s) $(uname -m)" in
  "Linux x86_64") tgt=x86_64-linux;; "Linux aarch64") tgt=aarch64-linux;;
  "Darwin arm64") tgt=aarch64-macos;; "Darwin x86_64") tgt=x86_64-macos;; *) echo "no native target here"; exit 77;;
esac
python3 "$ORACLE" --target $tgt "$tmp/gpu.elf" "$tmp/gpu.nat" "$tmp/gpu.elf" > /dev/null 2>&1; chmod +x "$tmp/gpu.nat"
# Native entropy is the SAME launcher with tools/t1-run.sh as the runner: the pool is an ordinary file in the
# granted root, read through the native file window, so native == yantra by construction (no special case in
# the runtime). T1_RUN_REQUIRE_NATIVE=1 makes a run that would fall back to yantra-run refuse (exit 3) instead.
T=$root/tools/t1-run.sh
nat() { SAS_BIN=$BIN T1_RUN_REQUIRE_NATIVE=1 sh "$root/tools/yantra-entropy.sh" "$@"; }
nat --replay "$tmp/rec.bin" "$tmp/root" "$T" "$tmp/ent.elf" x > "$tmp/n.out" 2> "$tmp/n.err" && rc=0 || rc=$?
if [ "$rc" -eq 3 ]; then
  bad "$tgt: tools/t1-run.sh did not run the pool natively ($(grep -a REFUSED "$tmp/n.err" | head -1)); a native leg that cannot run is a FAIL, not a skip"
else
  cmp -s "$tmp/y.out" "$tmp/n.out" && ok "$tgt: native == yantra on the same pool (replayed record)" || bad "$tgt differs from yantra on the same pool"
  nat --record "$tmp/nrec.bin" "$tmp/root" "$T" "$tmp/ent.elf" x > "$tmp/n1.out" 2>/dev/null || true
  [ "$(head -1 "$tmp/n1.out")" = 0 ] && [ "$(dist "$tmp/n1.out")" -gt 8 ] && ok "$tgt: fresh pool, status 0, $(dist "$tmp/n1.out") distinct" || bad "$tgt fresh draw"
  cmp -s "$tmp/n1.out" "$tmp/n.out" && bad "$tgt: a fresh pool repeated the replayed one" || ok "$tgt: a fresh pool differs"
  "$Y" --files "$tmp/root" "$tmp/ent.elf" x > /dev/null 2>&1 || true   # (the pool is gone again: the launcher removed it)
  [ ! -e "$tmp/root/.entropy" ] && ok "$tgt: the pool is removed after the run" || bad "$tgt pool left behind"
  nat --pool 40 "$tmp/root" "$T" "$tmp/ent.elf" x > "$tmp/ns.out" 2>/dev/null || true
  [ "$(head -1 "$tmp/ns.out")" = "$(head -1 "$tmp/ys.out")" ] && ok "$tgt: a short pool refuses as yantra does ($(head -1 "$tmp/ns.out"))" || bad "$tgt short pool"
  SAS_BIN=$BIN T1_RUN_REQUIRE_NATIVE=1 sh "$T" --files "$tmp/root" "$tmp/ent.elf" x > "$tmp/nm.out" 2>/dev/null || true
  cmp -s "$tmp/ym.out" "$tmp/nm.out" && ok "$tgt: a missing pool refuses as yantra does (0x3D23333)" || bad "$tgt missing pool"
fi

# ---- the GPU probe: slot 0 is yantra's GPU; native does not emulate it, it FAULTS to the fallback
"$Y" "$tmp/gpu.elf" x > "$tmp/gy.out" 2>/dev/null || true
[ "$(head -1 "$tmp/gy.out")" = 16 ] && ok "yantra: the GPU probe answers 16" || bad "yantra gpu probe"
"${tmp}/gpu.nat" x > "$tmp/gn.out" 2> "$tmp/gn.err" && rc=0 || rc=$?
[ "$rc" -ne 0 ] && [ ! -s "$tmp/gn.out" ] && grep -aq '^halt: BadAccess' "$tmp/gn.err" \
  && ok "$tgt: the GPU probe faults (BadAccess) and answers nothing: tools/t1-run.sh falls back to yantra, status 16 either way" \
  || bad "$tgt gpu probe did not fault to the fallback"
[ $fail -eq 0 ] && echo PASS || { echo FAIL; exit 1; }
