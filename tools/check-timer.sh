#!/bin/sh
# The clock interrupts the kernel — task `C-001d3`, doc 11 §7.1.4.
#
# ## What is different from C-001d1
#
# That trap was asked for: the program executed `ebreak` and stopped itself.
# An interrupt is not asked for by any instruction. The clock arrives from
# outside and cuts in wherever the program happens to be.
#
# So the program's last loop is deliberately infinite, and there is deliberately
# no path out of it that does not go through the handler. If the interrupt never
# arrives the machine spins, nothing is printed, and this check kills it and says
# so. A `shutdown` fall-through would have been a silent pass — the failure shape
# that fooled this project four times in one day (W-035, W-040, W-046, W-057).
#
# ## The evidence is the top bit
#
# `scause` for a supervisor timer is 5, but on an INTERRUPT bit 63 is also set,
# so the handler prints 0x8000000000000005 and not 5. That single bit is the
# whole difference between an interrupt and an exception, and `C-001d1`'s `3`
# is the other end of the same column: same register, same handler shape,
# different cause entirely.
#
# The value is architectural rather than derived, so unlike check-higher-half.sh
# there is no second link address that would prove anything — the cause of a
# timer interrupt does not move. What proves it instead is that REMOVING any one
# of the three enabling steps produces no output at all: the mutations are the
# evidence that this number was read from the machine and not printed by a
# program that would have printed it regardless.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
    --स्थान ०षोड्८०२००००० "$root/spec/timer.sas" "$tmp/timer.elf" >/dev/null

# Bounded by SIGKILL, not perl's alarm: QEMU handles SIGALRM and survives it
# (W-060). Here the bound is load-bearing rather than defensive — a kernel whose
# interrupt never arrives spins for ever by design.
qemu-system-riscv64 -machine virt -nographic -bios default \
    -kernel "$tmp/timer.elf" > "$tmp/out" 2>&1 &
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

got=$(tail -1 "$tmp/out" | tr -d ' \r')
printf "  scause -> %s\n" "$got"

# NEGATIVE CONTROL. The positive run above is not enough on its own, and I only
# found that out by mutating: deleting the set_timer call left the check passing.
# QEMU's mtimecmp starts at 0, so a timer interrupt is already pending at boot —
# opening STIE and SIE is sufficient to take one, and the deadline never had to
# be set at all.
#
# So the deadline is made to matter here. The same program is rebuilt with its
# deadline moved to the far future instead of the past, and it must then print
# NOTHING and be killed by the bound. If it prints anyway, this file is watching
# a timer that fires regardless of what it is told, and the positive run above
# proves only that interrupts can be enabled.
sed 's/योगः अर्थ०म् शून्यःन ०न ।/योगः अर्थ०म् शून्यःन ऋण१न ।/' \
    "$root/spec/timer.sas" > "$tmp/far.sas"
cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
    --स्थान ०षोड्८०२००००० "$tmp/far.sas" "$tmp/far.elf" >/dev/null
qemu-system-riscv64 -machine virt -nographic -bios default \
    -kernel "$tmp/far.elf" > "$tmp/farout" 2>&1 &
fpid=$!
( sleep 15; kill -9 "$fpid" 2>/dev/null ) & fwatch=$!
wait "$fpid" 2>/dev/null || true
# `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
# to init, still holding whatever it inherited (`W-096`). By PARENT pid,
# never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
# been seen live on this host. Ordered after the `wait`: the sleep dying
# lets the subshell run its kill, a no-op only because the process it
# would kill has already been reaped.
pkill -P "$fwatch" 2>/dev/null || true
kill "$fwatch" 2>/dev/null || true
far=$(tail -1 "$tmp/farout" | tr -d ' \r')
if [ "$far" = "8000000000000005" ]; then
    echo "  FAIL  a far-future deadline fired anyway — the timer is not being"
    echo "        controlled by set_timer, so the positive run proves only that"
    echo "        STIE and SIE can be opened"
    exit 1
fi
printf "  far deadline -> silent, as it must be\n"

case "$got" in
    8000000000000005)
        echo
        echo "ok  the clock interrupted the kernel: SBI set_timer with a deadline"
        echo "    already past, STIE and SIE opened, and the handler entered from"
        echo "    an infinite loop with no other way out. scause has bit 63 set —"
        echo "    an interrupt, cause 5 — not the 3 of a breakpoint"
        exit 0 ;;
    0000000000000005)
        echo "  FAIL  cause 5 without bit 63: that is an exception, not an interrupt"
        exit 1 ;;
    *)
        echo "  FAIL  expected 8000000000000005."
        echo "        Nothing printed means the interrupt never arrived and the"
        echo "        machine spun until this check killed it — check STIE (sie"
        echo "        bit 5), SIE (sstatus bit 1), and that set_timer was called."
        exit 1 ;;
esac
