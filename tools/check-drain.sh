#!/bin/sh
# The drain — task `C-001e3c2`, doc 11 §7.1.5.
#
# `C-001e3c1` drives 64 interleaved allocations and releases and asks that no two
# live blocks ever share an octet. That property is satisfied by an allocator
# that NEVER COALESCES AT ALL: one which always hands back untouched ground can
# never overlap, and it passes every check in `check-stress.sh` while leaking the
# arena away one block at a time.
#
# ## The property
#
# After the interleaving, releasing every block still held must return the arena
# to ONE WHOLE BLOCK: orders 6..11 empty, order 12 holding exactly one block, and
# that block at the arena base. Nothing fails at the moment a merge is missed —
# the failure is a later large request that cannot be met out of memory that is
# entirely free.
#
# The head alone does not say "one block": a list holding one address twice has
# the same head. So the program walks each chain and prints its LENGTH too,
# stopping at 64, and a cyclic list reports 64 rather than 1.
#
# Same program and same trace as `check-stress.sh`; the two read different halves
# of it. Every number below is computed from what the program printed. This file
# states no address of its own — `head[12]` is compared against the base the
# program reported, not against a constant.
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
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/lines"

    if ! python3 - "$tmp/lines" > "$tmp/verdict" 2>&1 <<'PY'
import sys
v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]
if not v:
    print("the program printed nothing"); sys.exit(1)
base, events = v[0], v[1:]
TOP = 12                           # the whole arena is one block at this order
LOW, HIGH = 6, TOP                 # the seven free lists
bad = []

live = {}                          # address -> order, replayed from the trace
drained = []                       # the drain's own releases
held_at_drain = None               # how many blocks the interleaving left behind
head = {}
chain = {}

if len(events) % 2:
    bad.append(f"{len(events)} words after the base — every event is two words, "
               "so the trace is truncated and the tail is the drain")

for i in range(0, len(events) - 1, 2):
    hdr, p = events[i], events[i + 1]
    kind, k = hdr >> 8, hdr & 0xff
    n = i // 2

    if kind in (1, 2, 3) and not (LOW <= k <= HIGH):
        bad.append(f"event {n}: header {hdr:016x} names order {k}, outside "
                   f"{LOW}..{HIGH}"); break

    if kind == 1:                  # allocate — C-001e3c1 checks these; here they
        if p:                      # only build the table the drain must empty
            live[p] = k
    elif kind == 2:
        live.pop(p, None)
    elif kind == 3:
        if held_at_drain is None:
            held_at_drain = len(live)
        if p not in live:
            bad.append(f"event {n}: the drain released {p:016x} at order {k}, "
                       "which was not live — the driver is releasing something "
                       "it does not hold and the final state means nothing")
        elif live[p] != k:
            bad.append(f"event {n}: the drain released {p:016x} at order {k}, "
                       f"taken at order {live[p]} — returned to the wrong free "
                       "list it is either lost or larger than it owns")
        else:
            del live[p]
            drained.append(p)
    elif kind == 4:
        head[k] = p
    elif kind == 5:
        chain[k] = p
    else:
        bad.append(f"event {n}: header {hdr:016x} is a kind this check cannot "
                   "read"); break

if held_at_drain is None:
    bad.append("no drain event in the trace at all — the program ran the "
               "interleaving and stopped, so nothing here was asked")
elif held_at_drain < 3:
    bad.append(f"the interleaving left only {held_at_drain} blocks held, so the "
               "drain merged almost nothing — with one block the arena comes "
               "back whole even from an allocator that never split it")
if live:
    bad.append(f"{len(live)} blocks were still live after the drain: "
               + ", ".join(f"{p:016x}/order {k}" for p, k in sorted(live.items()))
               + " — the drain did not release what it was holding")

want = set(range(LOW, HIGH + 1))
if set(head) != want or set(chain) != want:
    bad.append(f"the trace reports heads for {sorted(head)} and chains for "
               f"{sorted(chain)}; all of {sorted(want)} are needed to say the "
               "arena is whole")
else:
    for k in range(LOW, TOP):
        if head[k] or chain[k]:
            bad.append(f"order {k} is not empty: head {head[k]:016x}, "
                       f"{chain[k]} block(s) on the list. Every one of them is a "
                       "half whose buddy was free and was not merged with it, so "
                       "the arena can no longer answer a large request out of "
                       "memory that is entirely free")
    if head[TOP] != base:
        bad.append(f"order {TOP} holds {head[TOP]:016x}, not the arena base "
                   f"{base:016x} — the whole region is one block at this order "
                   "and there is nowhere else for it to start")
    if chain[TOP] != 1:
        bad.append(f"order {TOP} has {chain[TOP]} blocks on its list, not one. "
                   "The arena is one block wide at this order, so any other "
                   "count is the same address twice or a block outside the "
                   "region")

if bad:
    print("\n".join(bad)); sys.exit(1)
print(f"{held_at_drain} blocks still held after the interleaving, all released; "
      f"orders {LOW}..{TOP-1} empty and order {TOP} holding one block at "
      f"{head[TOP]:016x}, the arena base")
PY
    then
        sed 's/^/  FAIL  /' "$tmp/verdict"
        fail=1; continue
    fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/verdict")"
done

[ "$fail" -eq 0 ] || { echo; echo "released memory does not come back whole."; exit 1; }
echo
echo "ok  the arena drains back to one block. An allocator that never coalesces"
echo "    passes C-001e3a, C-001e3b and C-001e3c1 — it splits correctly, it"
echo "    merges the one chain those check, and it never overlaps because every"
echo "    request comes off untouched ground — and fails HERE, with the arena"
echo "    entirely free and unable to answer for any of it"
