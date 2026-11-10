#!/bin/sh
# build-yantra-wasm.sh [out.wasm] -- build the .t1 RV64 interpreter's wasm module (y.wasm) from THIS tree.
#   mk.sh -> t1-build (cached Stage 1) -> anuvada --target wasm32 -> zeroimp -> abi.sh (stamped with rev.sh).
# Out: arg 1, else $YANTRA_WASM_OUT, else target/yantra.wasm. Keeps intermediates in $YANTRA_WASM_WORK if set
#   (y.wasm, z.wasm, YI.elf, build.log), else a scratch dir removed on exit.
# Exit 77 when it cannot build: no node / python3, or no cached Stage 1 (T1_BUILD_CACHED_ONLY, see t1-build.sh).
# On a dirty tools/t1-interp rev.sh refuses and the module is stamped `unknown`.
set -eu
cd "$(dirname "$0")/.."
command -v node >/dev/null 2>&1 || { echo "SKIPPED: no node"; exit 77; }
command -v python3 >/dev/null 2>&1 || { echo "SKIPPED: no python3"; exit 77; }
OUT=${1:-${YANTRA_WASM_OUT:-target/yantra.wasm}}
if [ -n "${YANTRA_WASM_WORK:-}" ]; then O=$YANTRA_WASM_WORK; mkdir -p "$O"
else O=$(mktemp -d "${TMPDIR:-/tmp}/yantra-wasm.XXXXXX"); trap 'rm -rf "$O"' EXIT; fi
if REV=$(sh tools/t1-interp/wasm/rev.sh 2>/dev/null); then :; else REV=unknown; fi
sh tools/t1-interp/mk.sh "$O/m" >/dev/null 2>&1
rc=0
T1_BUILD_CACHED_ONLY=${T1_BUILD_CACHED_ONLY:-$([ "${T1_BUILD_COLD:-0}" = 1 ] && echo 0 || echo 1)} bash tools/t1-build.sh --entry यन्त्रकेन्द्र मुख्यम् -o "$O/YI.elf" $(cat "$O/m/MODULES") >"$O/build.log" 2>&1 || rc=$?
[ "$rc" -ne 77 ] || { echo "SKIPPED: no cached Stage 1 for the interpreter build"; exit 77; }
[ "$rc" -eq 0 ] || { tail -5 "$O/build.log"; echo "FAIL interpreter image build rc=$rc"; exit 1; }
bash tools/t1-interp/wasm/anuvada-run.sh --target wasm32 "$O/YI.elf" "$O/z.raw"
bash tools/native-spike/t1/authoring/zeroimp.sh "$O/z.raw" "$O/z.wasm" >/dev/null
bash tools/t1-interp/wasm/abi.sh "$O/z.wasm" "$O/y.wasm" --rev="$REV" >/dev/null
mkdir -p "$(dirname "$OUT")"
cp "$O/y.wasm" "$OUT"
[ -s "$OUT" ] || { echo "FAIL build-yantra-wasm: $OUT missing or empty"; exit 1; }
echo "$OUT rev=$REV"
