#!/bin/sh
# A trap is taken and the handler runs — task `C-001d1`, doc 11 §7.1.4.
#
# ## What is being proved
#
# Every instruction so far did what it said. A trap is the opposite: control
# arrives somewhere no branch sent it. `spec/trap.sas` puts a handler address in
# `stvec`, executes `ebreak`, and the next instruction that runs is the
# handler's first — not the one after the breakpoint.
#
# The evidence is `scause`, READ rather than written: a breakpoint's cause is 3
# (RISC-V privileged spec §4.1.8). Three things are cut at once by printing it:
#
#   * `stvec` never took     — OpenSBI's own handler runs and the machine dies,
#                              so nothing of ours prints at all;
#   * `stvec` took but wrong — some other address runs and prints something else,
#                              or hangs;
#   * the trap never fired   — the fall-through path prints ffff…ffff, which the
#                              program writes deliberately so that "no trap" can
#                              never look like silence.
#
# That last one matters most. A test whose failure mode is *nothing printed* is
# indistinguishable from a test that did not run, and this project has been
# caught by that four times.
#
# Returning from the handler is NOT proved here: that needs `sret`, which is not
# in the mnemonic registry yet (`C-001d2`). Arriving first, returning second.
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
        --स्थान "$1" "$root/spec/trap.sas" "$tmp/trap.elf" >/dev/null
    # Bounded by an explicit SIGKILL, not perl's alarm: QEMU handles SIGALRM
    # (its own timer subsystem) and survives it — see W-060. A program whose
    # trap goes wrong hangs the machine, so an unbounded read hangs this check.
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/trap.elf" > "$tmp/out" 2>&1 &
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
    tail -2 "$tmp/out" | tr -d ' \r' | tr '\n' ' '
}

fail=0
prev_pc=""
# Two link addresses, because the handler's address is computed with auipc and a
# single run cannot tell a computed address from a lucky constant.
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    got=$(run_at "$addr")
    cause=$(echo "$got" | awk '{print $1}')
    pc=$(echo "$got" | awk '{print $2}')

    # TWO lines are required, and which one is missing says which half failed.
    # One line alone is the dangerous case: it looks like output, so it must be
    # named rather than pattern-matched loosely.
    if [ "$cause" != "0000000000000003" ]; then
        echo "  FAIL  $addr -> first line '$cause' is not scause 3; the handler was never reached"
        fail=1; continue
    fi
    if [ -z "$pc" ]; then
        echo "  FAIL  $addr -> scause 3 printed but nothing after it: the handler never returned"
        fail=1; continue
    fi
    # The commonest way to get this wrong: return without advancing sepc, so the
    # same breakpoint traps again forever. The machine then prints scause 3 until
    # it is killed, and BOTH of the last two lines are that cause.
    if [ "$pc" = "0000000000000003" ] || [ "$pc" = "$cause" ]; then
        echo "  FAIL  $addr -> scause 3 twice: the handler returned to the same"
        echo "        breakpoint, so sepc was never advanced past it"
        fail=1; continue
    fi
    case "$pc" in
        0000000080*) printf "  %s -> scause 3, returned to %s\n" "$addr" "$pc" ;;
        *)  echo "  FAIL  $addr -> returned to '$pc', which is not in the loaded image (expected 0000000080…)"
            fail=1; continue ;;
    esac

    # The resume address must track the link address, or it is a constant.
    if [ -n "$prev_pc" ] && [ "$pc" = "$prev_pc" ]; then
        echo "  FAIL  both runs resumed at $pc — the address is a constant, not sepc+4"
        fail=1
    fi
    prev_pc=$pc
done

[ "$fail" -eq 0 ] || { echo; echo "the trap is not being taken."; exit 1; }
echo
echo "ok  stvec holds our handler, ebreak transfers to it, the handler reads"
echo "    scause = 3, advances sepc past the breakpoint and returns — and the"
echo "    resume address tracks the link address, so both the handler's address"
echo "    and the resume point are computed rather than constants"
