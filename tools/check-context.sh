#!/bin/sh
# A context switch restores ALL twelve — `C-002d`, doc 11 §3.1, phase 11.2.4.
#
# `तन्तुः` holds an execution context, and the whole claim of a switch is one
# sentence: every register belonging to the caller comes back. All of them —
# twelve, not eleven.
#
# ## Why the check has the shape it has
#
# A switch that saves eleven of twelve PASSES every test that does not look at
# the twelfth. And the twelfth only fails when another thread happens to be
# holding something in it — which is to say occasionally, much later, and in
# somebody else's code.
#
# So each register carries a DIFFERENT value and every one is printed. Filling
# them all with one value would be as useless as filling none: an off-by-one in
# the save order would not show, and an off-by-one is the easiest defect to
# write here.
#
# ## Two contexts, because one cannot show a leak
#
# Saving and restoring a single context does not reveal whether two threads
# bleed into each other. So two are filled — pattern A = 0xa00+j, pattern B =
# 0xb00+j — and both are restored in turn. If the switch drops a register, the
# restored A shows B's value in that slot, and that is the leak.
#
# In hexadecimal here the digits 10..15 are the VOWELS अ आ इ ई उ ऊ in alphabet
# order, so the leading digit of each value says which pattern it belongs to.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$root/spec/context.sas" "$tmp/c.elf" >/dev/null
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/c.elf" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 20; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    # No `tail -N` — it returns min(actual, N), which makes the count assertion
    # below one-sided by construction and blind to a surplus. See W-081.
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$'
}

fail=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    n=$(wc -l < "$tmp/l" | tr -d ' ')
    if [ "$n" -ne 24 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 24 (twelve registers"
        echo "        restored from context A, then twelve from context B)"
        fail=1; continue
    fi
    if ! python3 - "$tmp/l" > "$tmp/v" 2>&1 <<'PY'
import sys
v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]
A, B = v[:12], v[12:]
bad = []
for name, got, base, other in (("A", A, 0xa00, 0xb00), ("B", B, 0xb00, 0xa00)):
    for j, g in enumerate(got):
        want = base + j
        if g == want:
            continue
        if g == other + j:
            bad.append(f"context {name}, register {j}: holds {g:#x}, which is the OTHER "
                       f"context's value for that slot. The switch did not save or restore "
                       f"register {j}, so the two threads share it — a leak that only shows "
                       "when something is actually living there")
        elif g in (other + k for k in range(12)):
            bad.append(f"context {name}, register {j}: holds {g:#x} — another context's "
                       "value from a DIFFERENT slot, so the save order is shifted")
        else:
            bad.append(f"context {name}, register {j}: {g:#x}, expected {want:#x}")
if len(set(A)) != 12 or len(set(B)) != 12:
    bad.append("some registers hold identical values, so an off-by-one in the save order "
               "could not be seen — the patterns must stay distinct per register")
if bad:
    print("\n".join(bad)); sys.exit(1)
print(f"context A restored {A[0]:#x}..{A[-1]:#x} and B restored {B[0]:#x}..{B[-1]:#x}; "
      "all twelve distinct in each, and neither context holds a value from the other")
PY
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
done
[ "$fail" -eq 0 ] || { echo; echo "the context switch is wrong."; exit 1; }
echo
echo "ok  every one of the twelve callee-saved registers survives a switch in"
echo "    both directions, and no register carries a value from the other"
echo "    context — which is what a switch saving eleven of twelve would show"
