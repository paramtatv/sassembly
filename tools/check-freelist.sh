#!/bin/bash
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"  # the verdict function below uses bash arithmetic and arrays
# Memory is actually handed out — task `C-001e2`, doc 11 §7.1.5.
#
# `C-001e1` checked the buddy identity in isolation. This is the first program
# that allocates: eight blocks of sixty-four octets at a single order, with the
# free list living inside the free blocks themselves — each holds the address of
# the next, and a block that is in use is simply not in the list.
#
# ## Three properties, and the third is the one that matters
#
#   1. Eight distinct addresses, all inside the region, evenly spaced.
#   2. The ninth allocation returns 0 — exhaustion is SAID, not wrapped around
#      silently into memory that belongs to something else.
#   3. A freed block comes back on the next request.
#
# Without (3) an allocator that never frees would pass: it hands out eight
# correct blocks, then reports exhaustion forever, and every property above is
# satisfied while memory leaks. That is the shape of leak that looks like
# working code, so it is checked directly rather than inferred.
#
# Run at two link addresses, so no address in the output can be a constant.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    "$root/tools/sadhana-native.sh" \
        --स्थान "$1" "$root/spec/freelist.sas" "$tmp/f.elf" >/dev/null
    # SIGKILL, not perl's alarm — QEMU handles SIGALRM (W-060).
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/f.elf" > "$tmp/out" 2>&1 &
    qpid=$!
    ( sleep 20; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    wait "$qpid" 2>/dev/null || true
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    pkill -P "$watcher" 2>/dev/null || true
    kill "$watcher" 2>/dev/null || true
    # CR-stripped: -nographic ends its lines with one, and an anchored pattern
    # silently matches nothing without this. `-a` is the same defect one
    # level down: a single NUL anywhere in the guest's output makes grep
    # decline to look and report zero lines, exit 1, in silence (`W-098`).
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -10
}

# verdict_freelist FILE: 10 hex lines (eight blocks, an exhaustion, a reuse); diagnostics on stdout, rc 1 on failure.
verdict_freelist() {
    local v=() l i=0 SIZE=64 N=8 M=$((1<<63)) blocks=() exhausted reused base b nd bad=()
    while IFS= read -r l || [ -n "$l" ]; do
        [ -n "${l//[[:space:]]/}" ] || continue
        v[i]=$((16#${l//[[:space:]]/})); i=$((i+1))
    done < "$1"
    blocks=("${v[@]:0:8}"); exhausted=${v[8]}; reused=${v[9]}
    nd=$(printf '%s\n' "${blocks[@]}" | sort -u | wc -l | tr -d ' ')
    if [ "$nd" -ne "$N" ]; then
        bad+=("only $nd distinct addresses out of $N — the same block was handed out twice")
    fi
    base=${blocks[0]}
    for b in "${blocks[@]}"; do [ $((b ^ M)) -lt $((base ^ M)) ] && base=$b; done
    for b in "${blocks[@]}"; do
        if ! [ $((b ^ M)) -lt $(((base + SIZE*N) ^ M)) ]; then
            bad+=("$(printf '%016x is outside the region %016x..%016x' "$b" "$base" $((base+SIZE*N)))")
        fi
        if [ $(((b - base) & (SIZE-1))) -ne 0 ]; then
            bad+=("$(printf '%016x is not on a %d-octet boundary from the base' "$b" $SIZE)")
        fi
    done
    if [ "$exhausted" -ne 0 ]; then
        bad+=("$(printf 'the ninth allocation returned %016x, not 0 — exhaustion was wrapped rather than reported' "$exhausted")")
    fi
    if [ "$reused" -ne "${blocks[7]}" ]; then
        bad+=("$(printf 'after freeing %016x the next allocation gave %016x — the block was not returned to the list' "${blocks[7]}" "$reused")")
    fi
    if [ "${#bad[@]}" -gt 0 ]; then printf '%s\n' "${bad[@]}"; return 1; fi
    printf '8 blocks %016x..%016x, exhaustion reported, %016x freed and reused\n' "$base" $((base+SIZE*(N-1))) "${blocks[7]}"
}

fail=0
prev_first=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/lines"
    n=$(wc -l < "$tmp/lines" | tr -d ' ')
    if [ "$n" -ne 10 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 10 (eight blocks, an exhaustion, a reuse)"
        fail=1; continue
    fi

    if ! verdict_freelist "$tmp/lines" > "$tmp/verdict" 2>&1
    then
        sed 's/^/  FAIL  /' "$tmp/verdict"
        fail=1; continue
    fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/verdict")"

    first=$(sed -n 1p "$tmp/lines")
    if [ -n "$prev_first" ] && [ "$first" = "$prev_first" ]; then
        echo "  FAIL  both runs began at $first — the region is a constant"
        fail=1
    fi
    prev_first=$first
done

[ "$fail" -eq 0 ] || { echo; echo "the allocator is wrong."; exit 1; }
echo
echo "ok  eight distinct blocks inside the region, exhaustion reported as 0"
echo "    rather than wrapped, and a freed block handed back on the next"
echo "    request — at two link addresses, so none of it is a constant"
