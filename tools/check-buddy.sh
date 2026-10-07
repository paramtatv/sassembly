#!/bin/sh
# The buddy equation, checked alone — task `C-001e1`, doc 11 §7.1.5.
#
# A buddy allocator rests entirely on one identity:
#
#     buddy(addr, k) = addr XOR (1 << k)
#
# Splitting a block finds both halves with it; merging asks it whether the
# neighbour is really the other half or just a block that happens to sit next
# door. Get it wrong and nothing fails at once: the allocator merges two
# unrelated blocks and memory is handed out twice, much later, somewhere else.
#
# That delay is why this is checked on its own rather than inside a finished
# allocator, where the defect appears far from its cause.
#
# ## Two properties, because one is not enough
#
# 1. Each printed buddy equals what an independent Python computation gives —
#    the same identity, written in another language, from the base the program
#    itself printed.
#
# 2. The buddy of the buddy is the original. This is the property that separates
#    XOR from ADD, and it is not optional: the base's bits 12..19 are ZERO, so
#    `addr XOR (1<<k)` and `addr + (1<<k)` produce the SAME answer on every
#    single-application case. Property 1 alone cannot tell the two apart. Applied
#    twice, XOR returns and addition keeps going.
#
# Run at two link addresses, so the base is not a constant either.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$root/spec/buddy.sas" "$tmp/b.elf" >/dev/null
    # SIGKILL, not perl's alarm — QEMU handles SIGALRM (W-060).
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/b.elf" > "$tmp/out" 2>&1 &
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
    # tr first: -nographic serial output ends lines with CR, so an anchored
    # pattern matches nothing and the check reports "0 hex lines" about a
    # program that printed seventeen.
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -17
}

fail=0
prev_base=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/lines"
    n=$(wc -l < "$tmp/lines" | tr -d ' ')
    if [ "$n" -ne 17 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 17 (a base and eight pairs)"
        fail=1; continue
    fi
    base=$(sed -n 1p "$tmp/lines")

    # The oracle is a second implementation, not a re-read of the program's own
    # output: it takes only the base and recomputes every value independently.
    # The lines go in as an ARGUMENT, not on stdin: the heredoc below is
    # already stdin, and a second redirection silently wins, so the script
    # reads itself and every index is out of range.
    if ! python3 - "$base" "$tmp/lines" > "$tmp/verdict" <<'PY'
import sys
base = int(sys.argv[1], 16)
lines = [l.strip() for l in open(sys.argv[2]) if l.strip()]
bad = []
for i, k in enumerate(range(12, 20)):
    got_buddy = int(lines[1 + 2 * i], 16)
    got_back  = int(lines[2 + 2 * i], 16)
    want = base ^ (1 << k)
    if got_buddy != want:
        bad.append(f"order {k}: buddy is {got_buddy:016x}, the identity gives {want:016x}")
    if got_back != base:
        bad.append(f"order {k}: buddy of buddy is {got_back:016x}, not the base {base:016x} "
                   "— that is addition, not exclusive or")
if bad:
    print("\n".join(bad)); sys.exit(1)
print(f"base {base:016x}, eight orders, buddy and buddy-of-buddy both agree")
PY
    then
        sed 's/^/  FAIL  /' "$tmp/verdict"
        fail=1; continue
    fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/verdict")"

    if [ -n "$prev_base" ] && [ "$base" = "$prev_base" ]; then
        echo "  FAIL  both runs used base $base — it is a constant, not the region's address"
        fail=1
    fi
    prev_base=$base
done

[ "$fail" -eq 0 ] || { echo; echo "the buddy equation is wrong."; exit 1; }
echo
echo "ok  buddy(addr,k) = addr XOR (1<<k) for orders 12..19, against an"
echo "    independent computation, and buddy(buddy(x)) = x — which is what"
echo "    tells exclusive-or from addition when the bit starts clear"
