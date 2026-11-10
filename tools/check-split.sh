#!/bin/bash
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"  # the verdict function below uses bash arithmetic and arrays
# Splitting across orders — task `C-001e3a`, doc 11 §7.1.5.
#
# `C-001e2` allocated at ONE order. This is the first program where the order
# changes: seven free lists, orders 6 through 12 — 64 octets up to 4096 — and
# asking for the smallest block breaks the whole arena down to reach it.
#
# ## What a correct split leaves behind
#
# Each split hands back a half. Breaking an order-j block makes two of order
# j-1; the second — the buddy — goes on that order's list, and the first is
# broken further. So after a single request for the smallest order, EXACTLY ONE
# free block remains at every intermediate order, each at its own buddy address:
#
#     head[k] = block XOR (1 << k)      for k in 6..11
#     head[12] = 0                      the whole block is gone
#
# That is a sharp question, and it is the one worth asking, because a split that
# stops one order early hands back a block LARGER than was asked for and loses
# the remainder silently — nothing fails at the time, and the arena is quietly
# smaller for the rest of the program's life.
#
# The oracle recomputes every head with `C-001e1`'s identity from the address
# the program itself returned, so this file states no address of its own.
#
# ## Alignment is checked, not assumed
#
# `buddy(addr,k) = addr XOR (1<<k)` only stays inside the region when the base
# is aligned to the region's own SIZE. `C-001e2` did not care — its list was
# flat — but here an unaligned base sends XOR outside the arena entirely.
# `संरेखः` is refused inside `ॱरिक्त` (B-068's rule; task `B-108`), so the
# program rounds its base up at run time, and the first assertion here is that
# it actually did.
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
        --स्थान "$1" "$root/spec/split.sas" "$tmp/s.elf" >/dev/null
    # SIGKILL, not perl's alarm — QEMU handles SIGALRM (W-060).
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/s.elf" > "$tmp/out" 2>&1 &
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
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -8
}

# verdict_split FILE: 8 hex lines (the block, then seven list heads for orders 6..12); diagnostics on stdout, rc 1 on failure.
verdict_split() {
    local v=() l i=0 LO=6 HI=12 block heads=() k got want why bad=()
    while IFS= read -r l || [ -n "$l" ]; do
        [ -n "${l//[[:space:]]/}" ] || continue
        v[i]=$((16#${l//[[:space:]]/})); i=$((i+1))
    done < "$1"
    block=${v[0]}; heads=("${v[@]:1}")
    if [ "$block" -eq 0 ]; then
        bad+=("the allocation returned 0 — the arena was empty at the start, so the search for a non-empty order never found the whole block")
    elif [ $((block & ((1<<HI)-1))) -ne 0 ]; then
        bad+=("$(printf '%016x is not aligned to %d — buddy arithmetic walks outside a region that is not aligned to its own size, so every address below is meaningless' "$block" $((1<<HI)))")
    fi
    for ((k=LO; k<=HI; k++)); do
        got=${heads[k-LO]}
        if [ "$k" -eq "$HI" ]; then
            want=0; why="the arena was handed out, so nothing whole is left"
        else
            want=$((block ^ (1<<k))); why="the half left behind when order $((k+1)) was split"
        fi
        if [ "$got" -ne "$want" ]; then
            if [ "$got" -eq 0 ]; then
                bad+=("$(printf 'order %d: list is EMPTY, expected %016x — the split stopped before reaching this order, so the block handed back is larger than was asked for and the remainder is lost' "$k" "$want")")
            else
                bad+=("$(printf 'order %d: list holds %016x, expected %016x (%s)' "$k" "$got" "$want" "$why")")
            fi
        fi
    done
    if [ "${#bad[@]}" -gt 0 ]; then printf '%s\n' "${bad[@]}"; return 1; fi
    printf 'block %016x aligned to %d, one free block at each of orders %d..%d on its buddy address, order %d empty\n' "$block" $((1<<HI)) $LO $((HI-1)) $HI
}

fail=0
prev_block=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/lines"
    n=$(wc -l < "$tmp/lines" | tr -d ' ')
    if [ "$n" -ne 8 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 8 (the block and seven list heads)"
        fail=1; continue
    fi

    if ! verdict_split "$tmp/lines" > "$tmp/verdict" 2>&1
    then
        sed 's/^/  FAIL  /' "$tmp/verdict"
        fail=1; continue
    fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/verdict")"

    block=$(sed -n 1p "$tmp/lines")
    if [ -n "$prev_block" ] && [ "$block" = "$prev_block" ]; then
        echo "  FAIL  both runs returned $block — the arena is a constant"
        fail=1
    fi
    prev_block=$block
done

[ "$fail" -eq 0 ] || { echo; echo "the split is wrong."; exit 1; }
echo
echo "ok  asking for the smallest order splits the arena the whole way down,"
echo "    leaving exactly one free block at every intermediate order and each"
echo "    on its buddy address — recomputed by C-001e1's identity from the"
echo "    address the program returned, at two link addresses"
