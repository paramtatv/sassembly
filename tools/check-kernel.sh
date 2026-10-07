#!/bin/sh
# The kernel prints from a timer interrupt — task `C-001f1`, GATE C1, doc 11 §7.
#
# Seven programs have been correct on their own: boot, the device tree, paging,
# the higher half, traps, the timer, the allocator. This is the first time they
# run as ONE kernel, and it is the gate's stated condition — *prints from a
# timer interrupt*.
#
# ## Two lines, because neither says it alone
#
#   scause = 0x8000000000000005   bit 63 set: an INTERRUPT, cause 5, the clock
#   handler = 0xffffffc0…         the handler's own address, in the higher half
#
# The first alone would be satisfied by `spec/timer.sas`, which takes a timer
# interrupt with no paging at all. The second alone would be satisfied by
# `spec/higher-half.sas`, which runs high and takes no interrupt. Together they
# say the interrupt was taken WHILE paging was on and the kernel was executing
# from its high alias — which is the integration, and is what neither program
# could report.
#
# ## What integration forced, and isolation never could
#
# Both new constraints are about ORDER:
#
#   * `stvec` is loaded only AFTER the jump to the higher half. Load it before
#     and it holds a physical address — which still works, because RAM is
#     identity-mapped, and that is exactly why it is worth stating: the wrong
#     order passes, and the kernel is then one unmapped page away from a trap
#     vector pointing at nothing.
#
#   * The timer deadline is set AFTER `stvec`. A deadline of 0 is already past,
#     so the interrupt goes pending immediately; set it first and the first
#     interrupt jumps to whatever `stvec` happened to contain.
#
# The handler address is DERIVED from the link address, so the check runs at two
# and requires it to move. `scause` is architectural and does not move — the
# check says which is which rather than treating both the same way.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

# $1 = link address, $2 = source file (so a mutant can be run through the same path)
run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$2" "$tmp/k.elf" >/dev/null
    # SIGKILL, not perl's alarm — QEMU handles SIGALRM (W-060). The bound is
    # load-bearing here rather than defensive: the wait loop has no exit that
    # does not go through the handler, so a kernel whose interrupt never
    # arrives spins for ever BY DESIGN. A shutdown fall-through would make
    # "the interrupt never came" look identical to success.
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/k.elf" > "$tmp/out" 2>&1 &
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
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -2
}

fail=0
prev_handler=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" "$root/spec/kernel.sas" > "$tmp/lines"
    n=$(wc -l < "$tmp/lines" | tr -d ' ')
    if [ "$n" -ne 2 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 2 (scause, then the"
        echo "        handler's own address). Nothing at all means the interrupt"
        echo "        never arrived and the machine spun until this check killed"
        echo "        it — check STIE (sie bit 5), SIE (sstatus bit 1), that"
        echo "        stvec was loaded, and that set_timer was called."
        fail=1; continue
    fi
    cause=$(sed -n 1p "$tmp/lines")
    handler=$(sed -n 2p "$tmp/lines")

    case "$cause" in
        8000000000000005) ;;
        0000000000000005)
            echo "  FAIL  cause 5 without bit 63: an exception, not an interrupt"
            fail=1; continue ;;
        0000000000000003)
            echo "  FAIL  cause 3 is a BREAKPOINT — that is C-001d1's trap, taken"
            echo "        because something executed अन्वेषणविरामः, not because the"
            echo "        clock fired"
            fail=1; continue ;;
        *)
            echo "  FAIL  scause is $cause, expected 8000000000000005"
            fail=1; continue ;;
    esac

    # The higher half is the half of the claim that isolation cannot make.
    case "$handler" in
        ffffffc0*) ;;
        *)
            echo "  FAIL  the handler ran at $handler, which is not in the higher"
            echo "        half (0xffffffc0…). The timer interrupt was taken, but"
            echo "        with the kernel still executing from its physical"
            echo "        address — so this is spec/timer.sas with extra steps and"
            echo "        the integration has not happened."
            fail=1; continue ;;
    esac

    printf "  %s -> scause %s, handler %s\n" "$addr" "$cause" "$handler"

    if [ -n "$prev_handler" ] && [ "$handler" = "$prev_handler" ]; then
        echo "  FAIL  the handler is at $handler at BOTH link addresses — it is a"
        echo "        constant, not this kernel's own address"
        fail=1
    fi
    prev_handler=$handler
done

[ "$fail" -eq 0 ] || { echo; echo "the kernel does not meet gate C1."; exit 1; }
echo
echo "ok  GATE C1: the kernel boots under OpenSBI, turns on Sv39, runs at the"
echo "    higher half, and PRINTS FROM A TIMER INTERRUPT — scause with bit 63"
echo "    set and cause 5, from a handler at 0xffffffc0…, at two link addresses"
echo "    so the address is derived rather than written down"
