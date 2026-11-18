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
# Every pass is bounded by the step ceiling, never by a clock or an optional `timeout` binary (absent on macOS):
# LIMSTEPS = the lim/ family budget of tools/t1-interp/vectors/golden-rest.sh:15 (ysof lim/*) = the frozen oracle StepLimit rows.
LIMSTEPS=${LIMSTEPS:-3000000}
export YANTRA_STEPS=$LIMSTEPS
export SAS_BIN="$W/nobin" SASSEMBLY_NATIVE_CACHE="$W/cache" T1_RUN_REQUIRE_NATIVE=1 SAS="$ROOT"
: > "$W/rows"
row() { # name, extra env/flags handled by caller via $@ ; runs t1-run on image $IMG
    local name=$1 img=$2; shift 2
    env "$@" "$ROOT/tools/t1-run.sh" ${FLAGS:-} "$img" > "$W/o" 2> "$W/e"; local rc=$?
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
# ---- PARITY (HOLE-PROBE-EXIT, v1.2.3): the class column above says WHICH route ran; it never compared WHAT it printed.
# For every lim/ probe, run the DEFAULT route (fallback allowed: native, else the L28 interp oracle behind t1-run) and compare
# rc + halt line + stdout length + stdout sha256 with the frozen oracle row in vectors/rest/golden-rest.tsv (the same four
# fields golden-rest.sh check compares). A REQUIRE_NATIVE refusal (rc 3) is a class, not a divergence: the five probes
# csr_satp_write csr_sie_read csr_stvec_write csr_time store_into_code are rc 3 above and MATCH here (fallback parity).
# Output rows: parity:<name> TAB MATCH|DIFF|NOGOLD|XDIV|XPASS TAB route TAB detail. Exit 1 on any DIFF/NOGOLD/XPASS or an empty golden.
# PARITY_NEG=1 flips one expected sha: every row must DIFF (negative control). PARITY=0 skips the pass.
if [ "${PARITY:-1}" = 1 ]; then
    GOLD=${GOLDEN_REST:-$ROOT/tools/t1-interp/vectors/rest/golden-rest.tsv}
    [ -s "$GOLD" ] || { echo "probe: parity: golden $GOLD missing or empty" >&2; exit 1; }
    s256() { (sha256sum <"$1" 2>/dev/null || shasum -a 256 <"$1") | cut -c1-64; }
    # expected divergence (HOLE-PROBE-EXIT): rows in vectors/rest/expected-divergence.tsv, ONE rule shared with golden-rest.sh check
    # (vectors/rest/xdiv.sh): listed + exact native rc/len = XDIV (counted, not a DIFF); listed but now MATCH = XPASS = exit 1.
    # PARITY_XDIV=<tsv> overrides the list (negative control: an empty list must give DIFF for every listed divergence).
    . "$ROOT/tools/t1-interp/vectors/rest/xdiv.sh"
    XD=${PARITY_XDIV:-$ROOT/tools/t1-interp/vectors/rest/expected-divergence.tsv}; [ "${PARITY_NEG:-}" = 1 ] && XD=/dev/null
    : > "$W/par"; pd=0; pm=0; pn=0; px=0; pp=0
    for f in "$W"/img/*.elf; do
        n=$(basename "$f" .elf)
        g=$(grep -a -m1 "^lim/$n	" "$GOLD")
        if [ -z "$g" ]; then printf 'parity:%s\tNOGOLD\t-\tno lim/%s row in %s\n' "$n" "$n" "${GOLD##*/}" >> "$W/par"; pn=$((pn+1)); continue; fi
        grc=$(printf '%s' "$g" | cut -f2); ghalt=$(printf '%s' "$g" | cut -f3); glen=$(printf '%s' "$g" | cut -f4); gsha=$(printf '%s' "$g" | cut -f5)
        [ "${PARITY_NEG:-}" = 1 ] && gsha=0000$gsha
        env -u T1_RUN_REQUIRE_NATIVE -u SAS_BIN YANTRA_STEPS=$LIMSTEPS YANTRA_RAM=268435456 "$ROOT/tools/t1-run.sh" "$f" > "$W/po" 2> "$W/pe"; prc=$?
        route=$(grep -a -m1 '^t1-run: route ' "$W/pe" | sed 's/^t1-run: route //; s/ —.*//' | tr -d '\n'); [ -n "$route" ] || route=unknown
        ph=$(grep -a -m1 '^halt:' "$W/pe" | tr '\t' ' '); pl=$(wc -c < "$W/po" | tr -d ' '); ps=$(s256 "$W/po")
        xdiv_classify "$XD" "lim/$n" "$prc" "$ph" "$pl" "$ps" "$grc" "$ghalt" "$glen" "$gsha"
        case $XV in MATCH) v=MATCH; pm=$((pm+1));; XDIV) v=XDIV; px=$((px+1));; XPASS) v=XPASS; pp=$((pp+1));; *) v=DIFF; pd=$((pd+1));; esac
        printf 'parity:%s\t%s\t%s\trc=%s/%s len=%s/%s halt=[%s]/[%s]\n' "$n" "$v" "$route" "$prc" "$grc" "$pl" "$glen" "$ph" "$ghalt" >> "$W/par"
    done
    sort "$W/par" > "$W/parsorted"
    [ "$OUT" = /dev/stdout ] && cat "$W/parsorted" || cat "$W/parsorted" >> "$OUT"
    echo "parity: MATCH=$pm DIFF=$pd NOGOLD=$pn XDIV=$px XPASS=$pp (lim/ probes vs golden-rest.tsv: rc+halt+stdout len+sha)" >&2
    [ "$pd" = 0 ] && [ "$pn" = 0 ] && [ "$pp" = 0 ] && [ "$pm" -gt 0 ] || exit 1
fi
