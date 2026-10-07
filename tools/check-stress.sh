#!/bin/sh
# Interleaved allocation and release — task `C-001e3c1`, doc 11 §7.1.5.
#
# `C-001e3a` checks splitting alone and `C-001e3b` checks coalescing alone, each
# against a sequence written by hand. Both are right about what they ask. What
# neither reaches is the two working against each other: allocate, release,
# allocate again out of the ground the release just changed.
#
# ## The property
#
# NO TWO LIVE BLOCKS SHARE AN OCTET. That is the whole promise of an allocator,
# and breaking it is silent — no message, no failure, no wrong number anywhere
# near the defect. Two owners write into one buffer and the fault appears later,
# elsewhere, in a third piece of code.
#
# The sequence is pseudo-random because the SHAPE of the defect is not known in
# advance. A hand-written order is chosen by the same reasoning that wrote the
# allocator (doc 03 §4.4), so it asks what the author already thought of.
# xorshift64 from seed 1 is deterministic: this is the same sequence on every
# machine, every run, so a failure can be reproduced rather than glimpsed.
#
# The program prints two words per event — `(kind << 8) | order`, then the
# address — and its arena base first. It states no verdict. Every check below is
# computed here from what the program returned; this file states no address of
# its own either.
#
# The same trace continues past the interleaving with the drain and the seven
# free lists (kinds 3, 4 and 5), which is `C-001e3c2` and is read by
# `tools/check-drain.sh`. This check stops at the first drain event: the two
# halves fail differently, so they are asked separately.
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
        --स्थान "$1" "$root/spec/stress.sas" "$tmp/s.elf" >/dev/null
    # SIGKILL, not perl's alarm — QEMU handles SIGALRM (W-060).
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/s.elf" > "$tmp/out" 2>&1 &
    qpid=$!
    ( sleep 30; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
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
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$'
}

fail=0
prev_base=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/lines"
    n=$(wc -l < "$tmp/lines" | tr -d ' ')
    if [ "$n" -lt 129 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected at least 129 (the"
        echo "        arena base, then two words for each of 64 events, and"
        echo "        after those the drain that C-001e3c2 reads)"
        fail=1; continue
    fi

    if ! python3 - "$tmp/lines" > "$tmp/verdict" 2>&1 <<'PY'
import sys
v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]
base, events = v[0], v[1:]
ARENA = 1 << 12
LO, HI = 6, 9                      # the orders the program asks for
bad = []

if base % ARENA:
    bad.append(f"the arena base {base:016x} is not aligned to {ARENA} — buddy "
               "arithmetic walks outside a region not aligned to its own size, so "
               "nothing below would mean anything")

live = {}                          # address -> order
allocs = frees = refusals = 0
most = 0
recycled = 0
freed_once = set()
orders = set()

seen = 0
for i in range(0, len(events) - 1, 2):
    hdr, p = events[i], events[i + 1]
    kind, k = hdr >> 8, hdr & 0xff
    n = i // 2
    if kind in (3, 4, 5):          # the drain and the free lists — C-001e3c2
        break
    seen += 1
    if kind not in (1, 2) or not (LO <= k <= HI):
        bad.append(f"event {n}: header {hdr:016x} is neither an allocation nor a "
                   f"release of an order between {LO} and {HI} — the trace is not "
                   "the trace this check knows how to read")
        break
    orders.add(k)
    size = 1 << k

    if kind == 1:
        if p == 0:
            refusals += 1
            continue
        allocs += 1
        if p % size:
            bad.append(f"event {n}: order {k} block {p:016x} is not aligned to "
                       f"{size} — a buddy computed from it lands outside its own "
                       "half of the region")
        if p < base or p + size > base + ARENA:
            bad.append(f"event {n}: order {k} block {p:016x}..{p+size:016x} is "
                       f"outside the arena {base:016x}..{base+ARENA:016x} — the "
                       "allocator handed out memory it does not own")
        for q, j in live.items():
            if p < q + (1 << j) and q < p + size:
                bad.append(f"event {n}: order {k} block {p:016x}..{p+size:016x} "
                           f"OVERLAPS the live order {j} block {q:016x}.."
                           f"{q+(1<<j):016x}. Two owners now hold the same octets. "
                           "Nothing fails here: they will write over each other "
                           "later, somewhere else, and the trail back to this "
                           "moment will be gone")
                break
        if p in freed_once:
            recycled += 1
        live[p] = k
        most = max(most, len(live))
    else:
        frees += 1
        if p not in live:
            bad.append(f"event {n}: released {p:016x} at order {k}, which is not a "
                       "live block — the driver's own table disagrees with what it "
                       "was handed")
        elif live[p] != k:
            bad.append(f"event {n}: released {p:016x} at order {k}, taken at order "
                       f"{live[p]} — a block returned to the wrong free list is lost "
                       "or hands out more memory than it owns")
        else:
            del live[p]
            freed_once.add(p)

# A run that never interleaves proves nothing, and it would pass every check
# above by doing almost nothing. So the trace has to show its own work.
if seen != 64:
    bad.append(f"{seen} interleaved events before the drain, expected 64 — the "
               "driver stopped early or the drain started before it was done")
if allocs < 20:
    bad.append(f"only {allocs} allocations succeeded of {allocs+refusals} asked — "
               "too few for the overlap check to have been asked anything")
if frees < 15:
    bad.append(f"only {frees} releases — an interleaving needs both directions")
if most < 3:
    bad.append(f"at most {most} blocks were live at once — with two live blocks a "
               "wrong answer has almost nowhere to hide")
if recycled < 1:
    bad.append("no address was ever handed out again after being released — every "
               "allocation came off untouched ground, which is the one case "
               "splitting alone already covers")
if len(orders) < 2:
    bad.append(f"every event used order {orders} — a single order is C-001e2")

if bad:
    print("\n".join(bad)); sys.exit(1)
print(f"{allocs} allocations, {frees} releases, {refusals} refusals, up to {most} "
      f"live at once, orders {min(orders)}..{max(orders)}, {recycled} addresses "
      "handed out again after release — no two live blocks ever shared an octet")
PY
    then
        sed 's/^/  FAIL  /' "$tmp/verdict"
        fail=1; continue
    fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/verdict")"

    b=$(sed -n 1p "$tmp/lines")
    if [ -n "$prev_base" ] && [ "$b" = "$prev_base" ]; then
        echo "  FAIL  both runs used the arena at $b — the base is a constant"
        fail=1
    fi
    prev_base=$b
done

[ "$fail" -eq 0 ] || { echo; echo "the allocator hands out overlapping memory."; exit 1; }
echo
echo "ok  64 interleaved allocations and releases at four orders, driven by a"
echo "    seeded xorshift, and no two live blocks ever shared an octet. A"
echo "    coalescer that merges a block whose buddy is still in use passes both"
echo "    C-001e3a and C-001e3b and fails HERE, because the merged block is"
echo "    handed out again while its second half is still held"
