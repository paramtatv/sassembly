#!/bin/bash
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"  # the verdict function below uses bash arithmetic and arrays
# Untyped retype — task `C-002b`, doc 11 §3.1, phase 11.2.2.
#
# The nucleus never allocates. A process donates its own `अव्यक्तम्` and objects
# are carved from it, so kernel memory cannot be exhausted and quotas are
# structural rather than policed: whoever asks, pays.
#
# ## Three properties, and the third is the quiet one
#
#   * Objects do not overlap. The watermark only moves forward, so overlap is
#     impossible by construction rather than by checking.
#
#   * A request past the end is REFUSED, not trimmed. Trimming hands back an
#     object smaller than asked for without saying so, and the caller writes it
#     at full size — over its neighbour.
#
#   * A REFUSAL COSTS NOTHING. If a failed request still advanced the watermark,
#     the region would shrink on every denial with no count ever looking wrong.
#     That is a leak, and nothing shows it at the moment it happens.
#
# The third is checked directly: after the refused 2048, the next 1024 must land
# exactly where it would have landed had the refusal never occurred.
#
# The answer is a pair — verdict, then address — for the reason `C-002a` found:
# 0 alone reads both as a valid answer and as a denial.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    "$root/tools/sadhana-native.sh" \
        --स्थान "$1" "$root/spec/untyped.sas" "$tmp/u.elf" >/dev/null
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/u.elf" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 20; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -13
}

# verdict_untyped FILE: 13 hex lines (base, then verdict+address per request); diagnostics on stdout, rc 1 on failure.
verdict_untyped() {
    local v=() l i=0 K=1024 REGION=4096 M=$((1<<63)) base rest n g vd a ok bad=() asked want_ok granted_i=() granted_g=()
    while IFS= read -r l || [ -n "$l" ]; do
        [ -n "${l//[[:space:]]/}" ] || continue
        v[i]=$((16#${l//[[:space:]]/})); i=$((i+1))
    done < "$1"
    base=${v[0]}
    asked=($K $K $K $((2*K)) $K 8); want_ok=(1 1 1 0 1 0)
    if [ $((base & (REGION-1))) -ne 0 ]; then
        bad+=("$(printf 'the region base %016x is not %d-aligned' "$base" $REGION)")
    fi
    for i in 0 1 2 3 4 5; do
        vd=${v[$((1+2*i))]}; g=${v[$((2+2*i))]}; ok=${want_ok[i]}; a=${asked[i]}
        if [ "$ok" = 1 ] && [ "$vd" -ne 1 ]; then
            bad+=("request $i for $a octets fits in the region and was REFUSED — nothing could ever be created")
        fi
        if [ "$ok" = 0 ] && [ "$vd" -ne 0 ]; then
            bad+=("$(printf 'request %d for %d octets does not fit and was GRANTED at %016x. If it was trimmed, the caller will write %d octets over whatever follows and nothing will have said so' "$i" "$a" "$g" "$a")")
        fi
        if [ "$vd" -eq 0 ] && [ "$g" -ne 0 ]; then
            bad+=("$(printf 'request %d was refused but returned %016x — a refusal must carry no address' "$i" "$g")")
        fi
        if [ "$vd" -eq 1 ]; then granted_i+=("$i"); granted_g+=("$g"); fi
    done
    n=0
    for i in "${!granted_g[@]}"; do
        g=${granted_g[i]}
        if ! { [ $((base ^ M)) -le $((g ^ M)) ] && [ $((g ^ M)) -lt $(((base+REGION) ^ M)) ]; }; then
            bad+=("$(printf 'object %d at %016x is outside the donated region %016x..%016x — it was never given that memory' "${granted_i[i]}" "$g" "$base" $((base+REGION)))")
        fi
        if [ "$g" -ne $((base + n*K)) ]; then
            bad+=("$(printf 'object %d is at %016x, expected %016x — the objects are not laid end to end, so two of them share octets' "${granted_i[i]}" "$g" $((base+n*K)))")
        fi
        n=$((n+1))
    done
    if [ "${#granted_g[@]}" -ge 4 ]; then
        g=${granted_g[3]}
        if [ "$g" -ne $((base + 3*K)) ]; then
            bad+=("$(printf 'after the refused %d-octet request the next object landed at %016x, not %016x — the DENIAL MOVED THE WATERMARK, so the region shrinks on every refusal and no count ever looks wrong' $((2*K)) "$g" $((base+3*K)))")
        fi
    fi
    if [ "${#bad[@]}" -gt 0 ]; then printf '%s\n' "${bad[@]}"; return 1; fi
    printf 'region %016x: four objects laid end to end, two requests refused with no address, and the refusal cost nothing — the next object resumed at %016x\n' "$base" $((base+3*K))
}

fail=0; prev_base=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    n=$(wc -l < "$tmp/l" | tr -d ' ')
    if [ "$n" -ne 13 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 13 (the region base, then"
        echo "        a verdict and an address for each of six requests)"
        fail=1; continue
    fi
    if ! verdict_untyped "$tmp/l" > "$tmp/v" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
    b=$(sed -n 1p "$tmp/l")
    [ -n "$prev_base" ] && [ "$b" = "$prev_base" ] && { echo "  FAIL  region base $b at both link addresses — a constant"; fail=1; }
    prev_base=$b
done
[ "$fail" -eq 0 ] || { echo; echo "untyped retype is wrong."; exit 1; }
echo
echo "ok  objects are carved from donated memory end to end and cannot overlap;"
echo "    a request past the end is refused whole rather than trimmed; and a"
echo "    refusal costs nothing, so denials cannot leak the region away"
