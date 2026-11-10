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
    "$root/tools/sadhana-native.sh" \
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

# The verdict on the four printed words, in awk (no interpreter needed). Values
# are held as 32-bit halves joined into a double: exact below 2^53, which covers
# every address this machine can have.
arena_verdict() {
    LC_ALL=C awk '
    function hx(s) { return (index("0123456789abcdef", substr(s, 1, 1)) - 1) }
    function val(s,   i, v) {
        v = 0
        for (i = 1; i <= length(s); i++)
            v = v * 16 + index("0123456789abcdef", substr(s, i, 1)) - 1
        return v
    }
    function h16(v,   s, d) {
        s = ""
        while (v > 0) { d = v - 16 * int(v / 16); s = substr("0123456789abcdef", d + 1, 1) s; v = int(v / 16) }
        while (length(s) < 16) s = "0" s
        return s
    }
    function hmin(v,   s) { s = h16(v); sub(/^0+/, "", s); return (s == "" ? "0" : s) }
    function add(m) { bad[++nb] = m }
    { if ($0 ~ /[^ \t]/) { n++; w[n] = $0 } }
    END {
        ram_base = val(w[1]); ram_size = val(w[2]); image_end = val(w[3]); arena = val(w[4])
        ALIGN = 4096
        if (ram_base == 0 || ram_size == 0) {
            add("RAM base or size is 0 \342\200\224 /memory'"'"'s reg was not found, so nothing below describes this machine. Zero is not where RAM starts.")
        } else {
            ram_end = ram_base + ram_size
            if (!(ram_base <= image_end && image_end < ram_end))
                add("the image ends at " h16(image_end) ", outside the reported RAM " h16(ram_base) ".." h16(ram_end) " \342\200\224 one of the two readings is wrong")
            if (arena < image_end)
                add("the arena starts at " h16(arena) ", BELOW the image end " h16(image_end) " \342\200\224 it would hand out the octets the kernel is executing from, and the allocation would look perfectly ordinary")
            if (!(ram_base <= arena && arena < ram_end))
                add("the arena at " h16(arena) " is outside RAM " h16(ram_base) ".." h16(ram_end) " \342\200\224 those addresses answer nothing: reads give zero, writes go nowhere, and no trap is taken")
            if (arena % ALIGN != 0)
                add("the arena at " h16(arena) " is not " ALIGN "-aligned, so buddy arithmetic walks outside it (C-001e3a)")
            if (arena - image_end >= ALIGN)
                add("the arena at " h16(arena) " is " sprintf("%.0f", arena - image_end) " octets above the image end " h16(image_end) " \342\200\224 more than one alignment step, so it was not rounded up from the image but placed somewhere else")
        }
        if (nb > 0) { for (i = 1; i <= nb; i++) print bad[i]; exit 1 }
        print "RAM " h16(ram_base) "+" hmin(ram_size) ", image ends " h16(image_end) ", arena " h16(arena) " \342\200\224 aligned, above the image, inside RAM"
    }' "$1"
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

    if ! arena_verdict "$tmp/lines" > "$tmp/verdict" 2>&1
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
