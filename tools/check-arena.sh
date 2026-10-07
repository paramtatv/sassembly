#!/bin/sh
# The arena comes from the machine's own RAM — task `C-001f2a`, doc 11 §7.
#
# Every allocator so far has run on a `ॱरिक्त` region: a block written into the
# link, rounded up at run time. This is the first time the arena is the memory
# the machine REPORTED — `/memory`'s `reg`, read out of the device tree the boot
# hart handed us.
#
# The traversal is `C-001b2b1..b3`'s, unchanged: same loop, same keys, same '@'
# check. What was removed is only its own reporting.
#
# ## Placement is the hard half, and it fails silently
#
# The arena must be INSIDE the reported RAM and ABOVE the kernel's own image.
# Neither mistake breaks anything at the moment it is made:
#
#   * an arena below the image hands out the octets the kernel is EXECUTING
#     FROM — the allocation succeeds, the address looks ordinary, and the
#     program writes over its own code;
#   * an arena past the end of RAM hands out addresses nothing answers — reads
#     give zero, writes go nowhere, and no trap is taken.
#
# So this check does not ask whether four numbers were printed. It asks for
# containment and non-overlap, and the program states its own image end rather
# than having this file assume one.
#
# Four lines, hexadecimal: RAM base, RAM size, image end, arena base. Run at two
# link addresses — the image end and the arena must MOVE, while the RAM the
# machine reports must NOT: one pair is derived from where we were loaded, the
# other is a property of the machine, and a check that treated them alike would
# be satisfied by a program that printed constants.
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
        --स्थान "$1" "$root/spec/arena.sas" "$tmp/a.elf" >/dev/null
    # SIGKILL, not perl's alarm — QEMU handles SIGALRM (W-060).
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/a.elf" > "$tmp/out" 2>&1 &
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
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -4
}

fail=0
prev_arena=""
prev_ram=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/lines"
    n=$(wc -l < "$tmp/lines" | tr -d ' ')
    if [ "$n" -ne 4 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 4 (RAM base, RAM size,"
        echo "        image end, arena base)"
        fail=1; continue
    fi

    if ! python3 - "$tmp/lines" > "$tmp/verdict" 2>&1 <<'PY'
import sys
v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]
ram_base, ram_size, image_end, arena = v
ALIGN = 4096
bad = []

if ram_base == 0 or ram_size == 0:
    bad.append("RAM base or size is 0 — /memory's reg was not found, so nothing "
               "below describes this machine. Zero is not where RAM starts.")
else:
    ram_end = ram_base + ram_size
    if not (ram_base <= image_end < ram_end):
        bad.append(f"the image ends at {image_end:016x}, outside the reported RAM "
                   f"{ram_base:016x}..{ram_end:016x} — one of the two readings is wrong")
    if arena < image_end:
        bad.append(f"the arena starts at {arena:016x}, BELOW the image end "
                   f"{image_end:016x} — it would hand out the octets the kernel is "
                   "executing from, and the allocation would look perfectly ordinary")
    if not (ram_base <= arena < ram_end):
        bad.append(f"the arena at {arena:016x} is outside RAM "
                   f"{ram_base:016x}..{ram_end:016x} — those addresses answer nothing: "
                   "reads give zero, writes go nowhere, and no trap is taken")
    if arena % ALIGN:
        bad.append(f"the arena at {arena:016x} is not {ALIGN}-aligned, so buddy "
                   "arithmetic walks outside it (C-001e3a)")
    if arena - image_end >= ALIGN:
        bad.append(f"the arena at {arena:016x} is {arena-image_end} octets above the "
                   f"image end {image_end:016x} — more than one alignment step, so it "
                   "was not rounded up from the image but placed somewhere else")

if bad:
    print("\n".join(bad)); sys.exit(1)
print(f"RAM {ram_base:016x}+{ram_size:x}, image ends {image_end:016x}, "
      f"arena {arena:016x} — aligned, above the image, inside RAM")
PY
    then
        sed 's/^/  FAIL  /' "$tmp/verdict"
        fail=1; continue
    fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/verdict")"

    ram=$(sed -n 1p "$tmp/lines"); arena=$(sed -n 4p "$tmp/lines")
    # The arena is DERIVED from where we were loaded, so it must move.
    if [ -n "$prev_arena" ] && [ "$arena" = "$prev_arena" ]; then
        echo "  FAIL  the arena is $arena at both link addresses — it is a constant,"
        echo "        not a placement computed from this image's own end"
        fail=1
    fi
    # The machine's RAM is not, so it must NOT move.
    if [ -n "$prev_ram" ] && [ "$ram" != "$prev_ram" ]; then
        echo "  FAIL  the reported RAM base changed from $prev_ram to $ram between"
        echo "        link addresses — RAM is a property of the machine, so this is"
        echo "        not being read from the device tree at all"
        fail=1
    fi
    prev_arena=$arena; prev_ram=$ram
done

[ "$fail" -eq 0 ] || { echo; echo "the arena is not placed correctly."; exit 1; }
echo
echo "ok  the kernel reads /memory from the device tree and places its arena"
echo "    inside that RAM and above its own image, aligned — with the arena"
echo "    moving between link addresses because it is derived, and the RAM not"
echo "    moving because it is the machine's"
