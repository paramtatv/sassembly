#!/bin/sh
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
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
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

fail=0
prev_first=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/lines"
    n=$(wc -l < "$tmp/lines" | tr -d ' ')
    if [ "$n" -ne 10 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 10 (eight blocks, an exhaustion, a reuse)"
        fail=1; continue
    fi

    if ! python3 - "$tmp/lines" > "$tmp/verdict" 2>&1 <<'PY'
import sys
v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]
blocks, exhausted, reused = v[:8], v[8], v[9]
SIZE, N = 64, 8
bad = []

if len(set(blocks)) != N:
    bad.append(f"only {len(set(blocks))} distinct addresses out of {N} — "
               "the same block was handed out twice")
base = min(blocks)
for b in blocks:
    if not (base <= b < base + SIZE * N):
        bad.append(f"{b:016x} is outside the region {base:016x}..{base+SIZE*N:016x}")
    if (b - base) % SIZE:
        bad.append(f"{b:016x} is not on a {SIZE}-octet boundary from the base")
if exhausted != 0:
    bad.append(f"the ninth allocation returned {exhausted:016x}, not 0 — "
               "exhaustion was wrapped rather than reported")
if reused != blocks[-1]:
    bad.append(f"after freeing {blocks[-1]:016x} the next allocation gave "
               f"{reused:016x} — the block was not returned to the list")

if bad:
    print("\n".join(bad)); sys.exit(1)
print(f"8 blocks {base:016x}..{base+SIZE*(N-1):016x}, exhaustion reported, "
      f"{blocks[-1]:016x} freed and reused")
PY
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
