#!/bin/sh
# build-sadhana.sh [OUT.wasm] -- the wasm32 build of spec/entry/assemble.t1 with the sadhana_* ABI, EMPTY import object.
#   t1-build.sh(assemble.t1) -> ANUVADA_RUN (wasm32 translate) -> ZEROIMP_RUN (stubs) -> abi.sh with sadhana-abi.wat.
#   ANUVADA_RUN / ZEROIMP_RUN: commands taking IN OUT (the T1 ports). Defaults: tools/t1-interp/wasm/anuvada-run.sh and
#   tools/native-spike/t1/authoring/zeroimp.sh. Exit 77 ONLY when node is absent.
#   The module is stamped --rev=$(tools/src-rev.sh --t1-assembler) (assemble.t1 + crates/sadhana-t1/src/*.t1); unknown if that cannot be taken.
#   env: ABI_CACHE, TMPDIR
set -e
H=$(cd "$(dirname "$0")" && pwd)
R=$(cd "$H/../../.." && pwd)
OUT=${1:-${TMPDIR:-/tmp}/sadhana.wasm}
ANUVADA_RUN=${ANUVADA_RUN:-"bash $H/anuvada-run.sh"}
ZEROIMP_RUN=${ZEROIMP_RUN:-"bash $R/tools/native-spike/t1/authoring/zeroimp.sh"}
command -v node >/dev/null 2>&1 || { echo "build-sadhana: SKIPPED: node absent" >&2; exit 77; }
REV=$(cd "$R" && sh tools/src-rev.sh --t1-assembler 2>/dev/null) || REV=unknown
W=$(mktemp -d "${TMPDIR:-/tmp}/sadhana-wasm.XXXXXX")
bash "$R/tools/t1-build.sh" --entry पाठसेतुः मुख्यम् -o "$W/as.elf" "$R"/crates/sadhana-t1/src/*.t1 "$R/spec/entry/assemble.t1"
$ANUVADA_RUN --target wasm32 "$W/as.elf" "$W/raw.wasm"
$ZEROIMP_RUN "$W/raw.wasm" "$W/zi.wasm"
ABI_WAT="$H/sadhana-abi.wat" bash "$H/abi.sh" "$W/zi.wasm" "$OUT" --rev="$REV"
node -e 'const m=new WebAssembly.Module(require("fs").readFileSync(process.argv[1]));const i=WebAssembly.Module.imports(m);console.log("imports",i.length);process.exit(i.length?1:0)' "$OUT"
cp "$W/as.elf" "${OUT%.wasm}.elf"
rm -rf "$W"
