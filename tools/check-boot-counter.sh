#!/bin/sh
# The boot counter survives a POWER CYCLE — row `E-004b`, doc 11 §7.3.
#
# ## What is being proved
#
# `E-004` showed a counter that survived a watchdog reset. That counter lived in
# RAM, and RAM does not survive the power going off — so the row it proved is
# not the row claimed here. `spec/boot-counter.sas` keeps the counter in sector
# 0 of a virtio-blk device instead, and the property is that it goes UP across a
# full machine restart.
#
# One QEMU run cannot prove that. A program that prints `1` and touches no disk
# at all looks identical, run once. So the acceptance is a PAIR of runs against
# the same untouched backing file, and it is the second number being larger that
# is the whole row.
#
# Four runs, because "larger the second time" alone is still weak:
#
#   A  fresh disk, link 0x80200000  -> FRESH, some value
#   B  SAME disk,  link 0x80200000  -> SEEN,  larger    <- the row
#   C  fresh disk, link 0x80400000  -> FRESH, A's value <- control
#   D  disk from B, link 0x80400000 -> SEEN,  larger    <- the row again
#
# C is what stops a counter that is really a boot-count kept anywhere but the
# medium: same binary, same machine, a different disk, and the number falls back
# to where A started. It also runs the driver at a second link address, so the
# queue's address — built with `auipc` — is shown to be computed rather than a
# constant that happened to be right once.
#
# FRESH and SEEN come from a magic word the program writes ahead of the counter.
# Without them a failed write and a fresh disk both read back as zero, and doc
# 11 §2's rule is that a check whose failure mode is silence is indistinguishable
# from a check that did not run. SEEN on run B is the medium saying, in its own
# bytes, that run A's write landed.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

prog=$root/spec/boot-counter.sas
[ -f "$prog" ] || { echo "RED, as written: spec/boot-counter.sas does not exist."; exit 1; }

# A backing file whose counter is KNOWN to be zero. Not an image carried in the
# tree and not one left over from a previous run: the starting value has to be
# something this script established, or "it went up" is being read off a number
# nobody chose.
new_disk() {
    dd if=/dev/zero of="$1" bs=512 count=64 2>/dev/null
}

assemble() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$prog" "$2" >/dev/null
}

# One power cycle: a whole QEMU process, started and gone. Nothing is carried
# between calls except the file named in $2.
#
# Bounded by an explicit SIGKILL from a background `sleep`, not by perl's alarm:
# QEMU runs its own timer subsystem and survives SIGALRM (`W-060`). A driver
# that mis-programs the queue spins forever on the used ring, so an unbounded
# wait here hangs the check rather than failing it.
cycle() {  # $1 = elf, $2 = disk, $3 = where to leave the cleaned output
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$1" \
        -drive file="$2",format=raw,if=none,id=d0 \
        -device virtio-blk-device,drive=d0 > "$tmp/raw" 2>&1 &
    qpid=$!
    # The watcher's own stderr is closed off, not just the kills' — the SUBSHELL
    # is what announces "Terminated: 15  sleep 25" when its `sleep` is killed,
    # and it inherited this script's stderr when it was forked. Silencing the
    # `pkill` instead does nothing, and the word Terminated lands in the middle
    # of this check's PASS report, where the next person to read it will take it
    # for a failure.
    ( sleep 25; kill -9 "$qpid" 2>/dev/null ) 2>/dev/null & watcher=$!
    wait "$qpid" 2>/dev/null || true
    # `pkill -P` FIRST: killing the subshell alone orphans its `sleep` to init
    # (`W-096`). By PARENT pid, never by pattern — `W-044`/`W-047`/`W-049`;
    # another agent's QEMU has been seen live on this host.
    pkill -P "$watcher" 2>/dev/null || true
    kill "$watcher" 2>/dev/null || true
    wait "$watcher" 2>/dev/null || true
    # QEMU's serial line ends every line with CR. Without stripping it,
    # `grep -E '^[0-9a-f]{16}$'` matches NOTHING and every counter reads empty,
    # which looks exactly like a program that printed nothing at all.
    tr -d '\r' < "$tmp/raw" > "$3"
}

# The counter line: sixteen hex digits alone on a line, the shape
# the छापनम् helper prints and the one every check in this tree reads. (That
# helper came from the trap proof; its filename is deliberately not written
# here, because the checks-column scanner in yantra/tests/application.rs is
# textual and would read the mention as a claim that THIS check tests THAT
# program, which it does not.)
counter() { grep -E '^[0-9a-f]{16}$' "$1" | tail -1; }

# FRESH or SEEN, whichever the program said. Empty if it said neither.
#
# `grep -E` rather than `sed`, because alternation is an ERE and BSD sed reads a
# basic one: `\|` there matches a literal bar, so the pattern quietly matched
# nothing on this host and every verdict came back empty while the counters were
# perfectly correct.
verdict() {
    grep -E '^BOOT-COUNTER-(FRESH|SEEN)$' "$1" | tail -1 | sed 's/^BOOT-COUNTER-//'
}

# The program names each way it can fail; a named failure is reported as itself
# rather than as a missing number.
complain() {  # $1 = output file, $2 = which run
    for tag in NO-DEVICE NO-QUEUE READ-FAILED WRITE-FAILED; do
        if grep -q "^BOOT-COUNTER-$tag$" "$1"; then
            echo "  FAIL  run $2 -> the driver reported $tag"
            return 0
        fi
    done
    echo "  FAIL  run $2 -> no counter line printed, and the driver named no failure;"
    echo "        last of what came out:"
    tail -3 "$1" | sed 's/^/          /'
    return 0
}

echo "Assembling at two link addresses..."
assemble ०षोड्८०२००००० "$tmp/bc-a.elf"
assemble ०षोड्८०४००००० "$tmp/bc-b.elf"

new_disk "$tmp/kept.img"
new_disk "$tmp/other.img"

fail=0

echo "Power cycle 1 of 4 — fresh medium..."
cycle "$tmp/bc-a.elf" "$tmp/kept.img" "$tmp/a.out"
a=$(counter "$tmp/a.out"); av=$(verdict "$tmp/a.out")
echo "Power cycle 2 of 4 — SAME medium, machine restarted..."
cycle "$tmp/bc-a.elf" "$tmp/kept.img" "$tmp/b.out"
b=$(counter "$tmp/b.out"); bv=$(verdict "$tmp/b.out")
echo "Power cycle 3 of 4 — different medium, second link address..."
cycle "$tmp/bc-b.elf" "$tmp/other.img" "$tmp/c.out"
c=$(counter "$tmp/c.out"); cv=$(verdict "$tmp/c.out")
echo "Power cycle 4 of 4 — the kept medium again, second link address..."
cycle "$tmp/bc-b.elf" "$tmp/kept.img" "$tmp/d.out"
d=$(counter "$tmp/d.out"); dv=$(verdict "$tmp/d.out")
echo

for pair in "a:1" "b:2" "c:3" "d:4"; do
    name=${pair%:*}; n=${pair#*:}
    eval "v=\$$name"
    [ -n "$v" ] || { complain "$tmp/$name.out" "$n"; fail=1; }
done
[ "$fail" -eq 0 ] || { echo; echo "the counter never printed; nothing about persistence is settled."; exit 1; }

printf "  run 1  %-6s %s   (fresh medium)\n" "$av" "$a"
printf "  run 2  %-6s %s   (same medium, after a full restart)\n" "$bv" "$b"
printf "  run 3  %-6s %s   (different medium, link 0x80400000)\n" "$cv" "$c"
printf "  run 4  %-6s %s   (kept medium, link 0x80400000)\n" "$dv" "$d"
echo

# Both counters are sixteen zero-padded hex digits, so a string comparison IS
# the numeric one — and it needs no shell that can read hexadecimal, which
# `printf %d 0x…` quietly cannot everywhere this runs.
gt() { awk -v x="$1" -v y="$2" 'BEGIN { exit !(x > y) }'; }

# The row itself.
if ! gt "$b" "$a"; then
    echo "  FAIL  run 2 printed $b, which is not larger than run 1's $a —"
    echo "        the counter did not survive the power cycle"
    fail=1
fi
if ! gt "$d" "$b"; then
    echo "  FAIL  run 4 printed $d, which is not larger than run 2's $b"
    fail=1
fi

# The medium said so, not just the program. Without this, a write that never
# reached the disk and a disk that was never read look the same from outside.
[ "$av" = "FRESH" ] || { echo "  FAIL  run 1 said '$av' on a medium this script had just zeroed"; fail=1; }
[ "$bv" = "SEEN" ]  || { echo "  FAIL  run 2 said '$bv' — the magic word run 1 wrote was not found,"
                         echo "        so run 1's write never reached the medium"; fail=1; }
[ "$dv" = "SEEN" ]  || { echo "  FAIL  run 4 said '$dv' on the kept medium"; fail=1; }

# The control. A counter kept anywhere other than the medium would keep climbing
# here; one kept on the medium starts over, at exactly the value run 1 gave.
[ "$cv" = "FRESH" ] || { echo "  FAIL  run 3 said '$cv' on a DIFFERENT, freshly zeroed medium —"
                         echo "        the program is remembering something that is not the disk"; fail=1; }
if [ "$c" != "$a" ]; then
    echo "  FAIL  run 3 printed $c on a fresh medium where run 1 printed $a —"
    echo "        the value does not come from the medium alone"
    fail=1
fi

[ "$fail" -eq 0 ] || { echo; echo "the boot counter is not persistent."; exit 1; }

echo "ok  the counter lives in sector 0 of a virtio-blk device, not in RAM."
echo
echo "    Two separate QEMU processes against one untouched backing file:"
echo "        run 1  $a"
echo "        run 2  $b"
echo "    and run 2 found the magic word run 1 had left on the medium, so run"
echo "    1's write reached the disk rather than only its own RAM."
echo
echo "    A third process, linked at another address and given a different,"
echo "    freshly zeroed medium, printed $c — back to where run 1 began."
echo "    The number is read off the medium, not carried by the image."
