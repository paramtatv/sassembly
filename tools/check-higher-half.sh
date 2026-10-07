#!/bin/sh
# The kernel runs at the higher half — task `C-001c3`, doc 11 §7.1.3.
#
# ## What the identity map could not prove
#
# `C-001c2` turned Sv39 on with 512 gigapages, each mapping its own address.
# That is real, and it is also the one arrangement where translation cannot be
# observed: if `satp` had done nothing at all, an identity-mapped machine runs
# exactly the same. The proof there was survival — the fetch after the write
# had to be translated for the program to continue — but survival says only
# "not broken", never "different".
#
# Here the kernel's gigapage is mapped a SECOND time at 0xffffffc0_00000000,
# and the program jumps to its own code through that second mapping. What it
# prints is its PC, and the PC starts `ffffffc0…` rather than `8…`. One address
# now reaches the same instructions by two routes, which is the thing an
# identity map can never show.
#
# ## Why two link addresses
#
# A single run proves nothing about arithmetic: `ffffffc0…` could be a constant
# someone typed. So the same source is built at two load addresses and both are
# run. The high half of the PC must be identical (the alias base is fixed), and
# the low half must differ by exactly the difference in load address — because
# the program derives its own gigapage index from `PC>>30` and its alias base
# from `-1 << 38`, neither of which is written down anywhere.
#
# Measured 2026-08-17: 0x80200000 -> ffffffc00020008c,
#                      0x80400000 -> ffffffc00040008c. Difference 0x200000,
# exactly the difference in load address.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

# Two load addresses, in the Devanagari numerals the flag takes.
run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$2" "$root/spec/higher-half.sas" "$tmp/hh.elf" >/dev/null
    # Bounded by an explicit kill, NOT by perl's alarm.
    #
    # A kernel that jumps to an unmapped address traps, and with no trap handler
    # the machine spins — so a broken program hangs QEMU and would hang this
    # check with it. That is not theoretical: the first mutation test of this
    # file hung, twice.
    #
    # `perl -e 'alarm 20; exec @ARGV'` — the trick loop-agent.sh uses — does not
    # work here. QEMU installs its own SIGALRM handler for its timer subsystem,
    # so it catches the signal and carries on: measured, two QEMU processes
    # still running after ten minutes under `alarm 20`. SIGKILL cannot be
    # handled, so that is what this sends.
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/hh.elf" > "$tmp/out" 2>&1 &
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
    tail -1 "$tmp/out" | tr -d ' \r'
}

pc1=$(run_at 0x80200000 ०षोड्८०२००००० )
pc2=$(run_at 0x80400000 ०षोड्८०४००००० )

fail=0
printf "  0x80200000 -> %s\n  0x80400000 -> %s\n" "$pc1" "$pc2"

for pc in "$pc1" "$pc2"; do
    case "$pc" in
        ffffffc0*) ;;
        8*) echo "  FAIL  PC is $pc — still running through the identity map, not the alias"; fail=1 ;;
        *)  echo "  FAIL  PC is $pc — neither the alias nor the identity map; the jump went nowhere"; fail=1 ;;
    esac
done

# The arithmetic: the gap between the two PCs must equal the gap between the
# two load addresses. A hardcoded alias would print the same PC twice.
if [ "$fail" -eq 0 ]; then
    if [ "$pc1" = "$pc2" ]; then
        echo "  FAIL  both runs printed $pc1 — the address is a constant, not a computation"
        fail=1
    else
        gap=$(python3 -c "print(hex(int('$pc2',16)-int('$pc1',16)))")
        if [ "$gap" != "0x200000" ]; then
            echo "  FAIL  the PCs differ by $gap; the load addresses differ by 0x200000"
            fail=1
        fi
    fi
fi

[ "$fail" -eq 0 ] || { echo; echo "the kernel is not running at the higher half."; exit 1; }
echo
echo "ok  the kernel runs at 0xffffffc0_00000000: its gigapage is mapped twice,"
echo "    it jumped to its own code through the alias, and the PC it prints"
echo "    tracks the load address — so the alias is computed, not typed"
