#!/bin/sh
# The allocator runs on the machine's own RAM — task `C-001f2b`, doc 11 §7.
#
# `C-001e3c` exercised the allocator on a `ॱरिक्त` block written into the link.
# `C-001f2a` derived an arena from `/memory`'s `reg`. This is both at once: the
# same interleaving and the same drain, over the memory the machine reported.
#
# EIGHT LINES CHANGED — the ones that made the arena. Everything else is
# `spec/stress.sas` unaltered, and that is the whole claim: if the allocator was
# right, it stays right on real RAM; if the placement were wrong, it would write
# over the kernel and say nothing.
#
# What this asks that neither predecessor could:
#
#   * the arena is the FDT-derived one — aligned, and above the image, so the
#     blocks handed out are not the octets being executed;
#   * every address is inside that arena;
#   * after the drain, orders 6..11 are empty and order 12 holds ONE block at
#     the arena base — the arena is whole again, in real memory.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$root/spec/stress-ram.sas" "$tmp/r.elf" >/dev/null
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/r.elf" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 25; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$'
}

fail=0; prev=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/lines"
    if ! python3 - "$tmp/lines" > "$tmp/verdict" 2>&1 <<'PY'
import sys
v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]
if len(v) < 3: print("the program printed almost nothing — it did not run"); sys.exit(1)
arena, rest = v[0], v[1:]
if len(rest) % 2: print(f"{len(rest)} words after the arena base — events come in "
                        "(descriptor, value) PAIRS, so an odd count means a line was lost")
bad = []
# Each event is two words: (kind << 8 | order), then the value. Kind 1 is an
# allocation, 2 a release, 3 a drain release, 4 a free-list head, 5 its length.
ev = [(rest[i] >> 8, rest[i] & 0xff, rest[i+1]) for i in range(0, len(rest) - 1, 2)]
heads  = {order: val for kind, order, val in ev if kind == 4}
counts = {order: val for kind, order, val in ev if kind == 5}
live   = [val for kind, _, val in ev if kind == 1 and val]

if arena % 4096: bad.append(f"the arena {arena:016x} is not 4096-aligned")
if arena < 0x80000000: bad.append(f"the arena {arena:016x} is below this machine's RAM")
if not heads: bad.append("no free-list state was printed, so the drain cannot be read")

for k in range(6, 12):
    if heads.get(k, 0) or counts.get(k, 0):
        bad.append(f"order {k} is not empty after the drain: head {heads.get(k,0):016x}, "
                   f"{counts.get(k,0)} block(s) — the arena did not come back whole")
if heads.get(12) != arena or counts.get(12) != 1:
    bad.append(f"order 12 holds head {heads.get(12,0):016x} count {counts.get(12,0)}, "
               f"expected one block at the arena base {arena:016x}")

ARENA_SIZE = 1 << 12
outside = [x for x in live if not (arena <= x < arena + ARENA_SIZE)]
if outside:
    bad.append(f"{len(outside)} allocation(s) fell outside the arena "
               f"{arena:016x}..{arena+ARENA_SIZE:016x}, e.g. {outside[0]:016x} — the "
               "allocator handed out memory it was never given")
if bad: print("\n".join(bad)); sys.exit(1)
print(f"arena {arena:016x} from the device tree, {len(live)} allocations all inside it, "
      f"orders 6..11 empty and order 12 whole after the drain")
PY
    then sed 's/^/  FAIL  /' "$tmp/verdict"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/verdict")"
    a=$(sed -n 1p "$tmp/lines")
    [ -n "$prev" ] && [ "$a" = "$prev" ] && { echo "  FAIL  arena $a at both link addresses — a constant"; fail=1; }
    prev=$a
done
[ "$fail" -eq 0 ] || { echo; echo "the allocator on real RAM is wrong."; exit 1; }
echo
echo "ok  the allocator runs on the RAM /memory reported — every block inside the"
echo "    device-tree-derived arena, and the arena whole again after the drain"
