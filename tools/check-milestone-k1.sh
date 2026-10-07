#!/bin/sh
# MILESTONE M-K1, gate C1 — task `C-001`, doc 11 §7.
#
# `C-001f1` put boot, paging, the higher half, traps and the timer into one
# image. `C-001f2a` derived an arena from the device tree and `C-001f2b` ran the
# allocator on it — both without paging. This is all of it at once.
#
# THREE LINES, PRINTED FROM INSIDE THE INTERRUPT:
#
#   8000000000000005   bit 63 set: an interrupt, cause 5, the clock
#   ffffffc0…          the handler's own address — paging on, higher half
#   the arena base     read back inside the handler, derived from /memory
#
# No two of them can be produced by any earlier program. `spec/timer.sas` gives
# the first with no paging at all; `spec/higher-half.sas` gives the second and
# takes no interrupt; `spec/arena.sas` gives the third and neither pages nor
# traps. Together they say the clock stopped a PAGED, HIGH kernel and it could
# still name the memory the machine had told it about.
#
# The third line is the arena BASE, not an allocation. The allocator itself is
# `C-001f2b` and is not repeated here — saying otherwise would be claiming a
# thing this image does not do.
#
# ## What moves and what does not
#
# The handler address and the arena are DERIVED from where we were loaded, so
# both must move between link addresses. `scause` is architectural and must not.
# A check that treated them alike would pass a program printing constants.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$root/spec/milestone-k1.sas" "$tmp/m.elf" >/dev/null
    # SIGKILL, not perl's alarm — QEMU handles SIGALRM (W-060). The bound is
    # load-bearing: the wait loop has no exit that does not go through the
    # handler, so a kernel whose interrupt never arrives spins by design.
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/m.elf" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 20; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -3
}

fail=0; prev_h=""; prev_a=""; prev_c=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    n=$(wc -l < "$tmp/l" | tr -d ' ')
    if [ "$n" -ne 3 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 3. Nothing at all has"
        echo "        TWO causes here and they look identical from outside:"
        echo "          - the interrupt never arrived, and the machine spun until"
        echo "            this check killed it (STIE, SIE, stvec, set_timer); or"
        echo "          - the kernel shut down BEFORE the timer, because it could"
        echo "            not find /memory in the device tree and took अमापः."
        echo "        Bisect: print at the first instruction. If that appears, the"
        echo "        program started and the fault is later."
        fail=1; continue
    fi
    cause=$(sed -n 1p "$tmp/l"); handler=$(sed -n 2p "$tmp/l"); arena=$(sed -n 3p "$tmp/l")

    [ "$cause" = "8000000000000005" ] || {
        echo "  FAIL  scause is $cause, expected 8000000000000005 (interrupt, cause 5)"
        [ "$cause" = "0000000000000003" ] && echo "        cause 3 is a breakpoint — that is C-001d1's trap, not the clock"
        fail=1; continue; }

    case "$handler" in ffffffc0*) ;; *)
        echo "  FAIL  the handler ran at $handler, not in the higher half — the timer"
        echo "        fired, but with the kernel still at its physical address, so"
        echo "        this is spec/timer.sas with extra steps"
        fail=1; continue ;;
    esac

    case "$arena" in
        0000000000000000)
            echo "  FAIL  the arena is 0. Either /memory's reg was never found, or it"
            echo "        was found and never stored — the handler reads the pointer"
            echo "        back from memory, so a missing STORE and a missing READ give"
            echo "        the same zero. Both mean the kernel cannot name its own RAM."
            fail=1; continue ;;
    esac
    msg=$(python3 - "$arena" <<'PYA'
import sys
a = int(sys.argv[1], 16)
if a % 4096:
    print(f"the arena {a:016x} is not 4096-aligned, so buddy arithmetic would walk outside it")
    sys.exit(1)
if not (0x80000000 <= a < 0x88000000):
    print(f"the arena {a:016x} is outside this machine's RAM")
    sys.exit(1)
PYA
) || { echo "  FAIL  $msg"; fail=1; continue; }

    printf "  %s -> scause %s, handler %s, arena %s\n" "$addr" "$cause" "$handler" "$arena"

    [ -n "$prev_h" ] && [ "$handler" = "$prev_h" ] && { echo "  FAIL  handler is $handler at both link addresses — a constant"; fail=1; }
    [ -n "$prev_a" ] && [ "$arena" = "$prev_a" ]   && { echo "  FAIL  arena is $arena at both link addresses — a constant"; fail=1; }
    [ -n "$prev_c" ] && [ "$cause" != "$prev_c" ]  && { echo "  FAIL  scause changed between link addresses — it is architectural and must not"; fail=1; }
    prev_h=$handler; prev_a=$arena; prev_c=$cause
done

[ "$fail" -eq 0 ] || { echo; echo "milestone M-K1 is not met."; exit 1; }
echo
echo "ok  MILESTONE M-K1 / GATE C1: the kernel boots under OpenSBI, reads its RAM"
echo "    from the device tree, turns on Sv39, runs at the higher half, and from"
echo "    inside a timer interrupt prints its cause, its own high address, and the"
echo "    arena it derived — at two link addresses, so what is derived moves and"
echo "    what is architectural does not"
