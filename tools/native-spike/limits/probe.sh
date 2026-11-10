#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# probe.sh — classify what the native runtime does with each LIMITS.md row on THIS host.
#   tools/native-spike/limits/probe.sh [OUT.tsv]      (prints and writes: probe<TAB>class<TAB>detail)
# class: native (ran natively to the end) | native-halt (ran natively, halted with a named halt/refusal that
#   t1-run does NOT fall back on) | fallback (t1-run takes yantra-run; detail = the reason text) | refused
# METHOD: T1_RUN_REQUIRE_NATIVE=1, so t1-run.sh refuses every fallback with "REFUSED — native required
#   (T1_RUN_REQUIRE_NATIVE) but: <reason>" (class fallback, detail = the reason); SAS_BIN also points at an EMPTY
#   directory, so even a t1-run that ignored the switch could not reach a yantra-run ("route yantra-run — <reason>",
#   the same class). Needs no cargo and no yantra-run. native-halt = ran natively and ended in one of yantra's
#   halts other than a finisher (Unimplemented, Breakpoint, BadAccess, BeyondRam, Undelivered,
#   Wait, Csr) or a named refusal. Translation is by anuvada.py (SASSEMBLY_NATIVE_CACHE
#   is a fresh temp dir, so no stage1/translator cache is used): the oracle, byte-identical to the .t1 translator.
set -uo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
OUT=${1:-/dev/stdout}
W=$(mktemp -d "${TMPDIR:-/tmp}/limits.XXXXXX"); trap 'rm -rf "$W"' EXIT
mkdir -p "$W/nobin" "$W/cache" "$W/img"
awk -f "$HERE/../rvgen.awk" -f "$HERE/rvenc.awk" -f "$HERE/gen.awk" -v OUTDIR="$W/img" >/dev/null || exit 1
TMO=""; command -v timeout >/dev/null && TMO="timeout 60"
export SAS_BIN="$W/nobin" SASSEMBLY_NATIVE_CACHE="$W/cache" T1_RUN_REQUIRE_NATIVE=1 SAS="$ROOT"
: > "$W/rows"
row() { # name, extra env/flags handled by caller via $@ ; runs t1-run on image $IMG
    local name=$1 img=$2; shift 2
    $TMO env "$@" "$ROOT/tools/t1-run.sh" ${FLAGS:-} "$img" > "$W/o" 2> "$W/e"; local rc=$?
    local cls detail
    if grep -a -q 'native required (T1_RUN_REQUIRE_NATIVE) but:' "$W/e"; then
        # t1-run refused the fallback T1_RUN_REQUIRE_NATIVE forbids: that IS a fallback, the reason after "but: "
        cls=fallback; detail=$(grep -a 'native required (T1_RUN_REQUIRE_NATIVE) but:' "$W/e" | head -1 | sed 's/^.*native required (T1_RUN_REQUIRE_NATIVE) but: //')
    elif grep -a -q '^t1-run: route yantra-run' "$W/e"; then
        cls=fallback; detail=$(grep -a '^t1-run: route yantra-run\|^t1-run: the native run reached' "$W/e" | sed 's/^t1-run: //; s/^route yantra-run — //' | head -2 | tr '\n' ' ')
    elif grep -a -q '^t1-run: route native' "$W/e"; then
        if grep -a -q '^native: \|^halt: \(Beyond\|Unimplemented\|Breakpoint\|BadAccess\|Undelivered\|Wait\|Csr\)\|^input: ' "$W/e"; then cls=native-halt; detail="rc=$rc $(grep -a '^native: \|^halt: \|^input: ' "$W/e" | head -1)"
        else cls=native; detail="rc=$rc out=$(head -c 40 "$W/o" | tr '\n' '|') $(grep -a '^halt: ' "$W/e" | head -1)"; fi
    else cls=refused; detail=$(head -2 "$W/e" | tr '\n' ' ' | cut -c1-200); fi
    printf '%s\t%s\t%s\n' "$name" "$cls" "$detail" >> "$W/rows"
}
for f in "$W"/img/*.elf; do n=$(basename "$f" .elf); row "$n" "$f"; done
B="$W/img/uart_finisher.elf"
for v in YANTRA_STEPS YANTRA_VERDICT YANTRA_SCANOUT YANTRA_VIRTIO_DEFER YANTRA_WATERMARK; do row "env:$v" "$B" "$v=1"; done
row "env:YANTRA_RAM=64M" "$B" YANTRA_RAM=67108864
row "env:YANTRA_RAM=bogus" "$B" YANTRA_RAM=bogus
row "env:YANTRA_INPUT (image has no input tag)" "$B" YANTRA_INPUT=/dev/null YANTRA_INPUT_NAME=x
row "env:YANTRA_INPUT_ENTRY malformed" "$B" YANTRA_INPUT=/dev/null YANTRA_INPUT_ENTRY=x
row "env:T1_RUN_ROUTE=yantra" "$B" T1_RUN_ROUTE=yantra
for fl in "--events $W/log" "--record-events $W/log" "--record-events $W/log --listen 127.0.0.1:0" "--record-events $W/log --net-peer x" "--files $W" "--smp $W/log" "--version" "--source-stamp"; do
    FLAGS=$fl row "flag:${fl%% $W*}" "$B"
done
sort "$W/rows" > "$W/sorted"; cp "$W/sorted" "$OUT"
[ "$OUT" = /dev/stdout ] || { echo "host $(uname -s) $(uname -m): $(cut -f2 "$W/sorted" | sort | uniq -c | tr '\n' ' ')" >&2; }
