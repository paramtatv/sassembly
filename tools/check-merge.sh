#!/bin/sh
# Coalescing on release — task `C-001e3b`, doc 11 §7.1.5.
#
# `C-001e3a` broke the arena down. This runs the other direction: a released
# block merges with its buddy, that with its buddy, and so on up until the whole
# region is one block again.
#
# A merge that stops one order early breaks nothing at the time. Every octet is
# free, no count is wrong, nothing is printed — and the arena stays split into
# halves that will never satisfy a large request again.
#
# ## The question one block cannot ask
#
# Taking a single block and releasing it does NOT test this: with everything
# else free, an allocator that merges without ever looking at its buddy gives
# the right answer too. So this takes TWO order-6 blocks — the second is the
# half the first one's split left behind, so the two are each other's buddy —
# and releases them one at a time.
#
#   After the FIRST release the buddy is still in use, so nothing may merge:
#
#       head[6]  = p1                    it goes on the list alone
#       head[k]  = p1 XOR (1 << k)       k in 7..11, exactly as the split left them
#       head[12] = 0
#
#   After the SECOND the buddy IS on the list, and the chain runs to the top:
#
#       head[k]  = 0                     k in 6..11
#       head[12] = p1                    the arena is whole again
#
# An allocator that merges without checking collapses the arena on the FIRST
# release — and that whole-arena block contains the second block, which someone
# else is still holding. Memory has been handed out twice and nothing says so.
#
# Every address below is recomputed from the one the program returned, by
# `C-001e1`'s identity. This file states no address of its own.
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
        --स्थान "$1" "$root/spec/merge.sas" "$tmp/m.elf" >/dev/null
    # SIGKILL, not perl's alarm — QEMU handles SIGALRM (W-060).
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/m.elf" > "$tmp/out" 2>&1 &
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
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -16
}

fail=0
prev_block=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/lines"
    n=$(wc -l < "$tmp/lines" | tr -d ' ')
    if [ "$n" -ne 16 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 16 (two blocks and"
        echo "        seven list heads after each of the two releases)"
        fail=1; continue
    fi

    if ! python3 - "$tmp/lines" > "$tmp/verdict" 2>&1 <<'PY'
import sys
v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]
p1, p2, after_one, after_two = v[0], v[1], v[2:9], v[9:16]
LO, HI = 6, 12
bad = []

if p1 == 0 or p2 == 0:
    bad.append("an allocation returned 0 — the arena was empty before anything "
               "was released, so nothing below is about coalescing")
elif p1 % (1 << HI):
    bad.append(f"{p1:016x} is not aligned to {1<<HI} — buddy arithmetic walks "
               "outside a region not aligned to its own size")
elif p2 != p1 ^ (1 << LO):
    bad.append(f"the second block is {p2:016x}, not {p1 ^ (1<<LO):016x} — the two "
               "are supposed to be each other's buddy, which is the only reason "
               "the release order below asks anything")

for i, k in enumerate(range(LO, HI + 1)):
    got = after_one[i]
    if k == LO:
        want, why = p1, "the released block, alone, because its buddy is in use"
    elif k == HI:
        want, why = 0, "the arena was handed out and only half of it is back"
    else:
        want, why = p1 ^ (1 << k), f"untouched, exactly as splitting order {k+1} left it"
    if got != want:
        if k == HI and got != 0:
            bad.append(f"after ONE release order {HI} holds {got:016x} — the arena "
                       "was merged back whole while the second block is STILL IN "
                       "USE. The allocator merged without checking that the buddy "
                       "was free, so that address is now inside a free block and "
                       "will be handed out a second time")
        else:
            bad.append(f"after ONE release, order {k}: {got:016x}, expected "
                       f"{want:016x} ({why})")

for i, k in enumerate(range(LO, HI + 1)):
    got = after_two[i]
    want = p1 if k == HI else 0
    if got != want:
        if k == HI:
            bad.append(f"after BOTH releases order {HI} holds {got:016x}, not "
                       f"{p1:016x} — the arena did not come back whole. A merge "
                       "that stops one order early fails nothing at the time and "
                       "leaves the region permanently unable to satisfy a large "
                       "request")
        else:
            bad.append(f"after BOTH releases order {k} still holds {got:016x} — "
                       "that block was merged into a larger one and left on the "
                       "smaller list as well, so the same octets are now reachable "
                       "from two free lists at once")

if bad:
    print("\n".join(bad)); sys.exit(1)
print(f"blocks {p1:016x} and {p2:016x} are buddies; releasing the first merges "
      f"nothing, releasing the second returns orders {LO}..{HI-1} to empty and "
      f"order {HI} to one whole block")
PY
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

[ "$fail" -eq 0 ] || { echo; echo "coalescing is wrong."; exit 1; }
echo
echo "ok  a released block merges with its buddy at every order in turn and the"
echo "    arena comes back as ONE whole block — and a block whose buddy is still"
echo "    in use merges with nothing, which is the property that separates a"
echo "    coalescer from one that hands the same memory out twice"
