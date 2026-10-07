#!/bin/sh
# A thread that blocks INSIDE its trap handler comes back to its own faulting
# site — tasks `C-002e2a` (this check) and `C-002e2b` (the widening it gates),
# doc 11 §3.4.
#
# ## What is being proved, and why nothing else proves it
#
# `सञ्चारः` used to save thirteen words: twelve callee-saved registers and the
# return address. No privileged state. That was complete for as long as no
# thread ever blocked inside a trap handler, because `sepc`, `sstatus` and
# `stval` are only alive between the trap and the `sret`, and no switch happened
# in that window.
#
# A separate pager thread opens the window. The faulting thread has to wait for
# its page *inside* the handler; another thread runs meanwhile; that thread's
# own trap overwrites the same `sepc`. The first thread then returns to an
# address that is not its own.
#
# `C-002d` never traps, `C-002h` never switches, `C-002i` never blocks in a
# handler. The gap is exactly in the seam between three landed rows and is
# invisible from all of them — so this is a check, not an extension of one.
#
# ## Why the evidence is two markers and not one
#
# "returned and re-ran its own site" and "did not return, landed somewhere else"
# look identical from outside unless something COUNTS. So each fault site prints
# a DIFFERENT marker from the instruction right after it: `a1` for thread 1,
# `b2` for thread 2. A correct switch prints each once. A leaking `sepc` sends
# thread 1 to thread 2's site, so `b2` prints and `a1` never does — the marker
# that appears where the other one should names the defect by itself.
#
# `c3` is the fourth line: a value thread 1 parks in a callee-saved register and
# carries across two switches. It is here so that WIDENING `सञ्चारः` cannot
# quietly break the twelve words it already saves correctly.
#
# ## IT WAS RED FIRST, AND WHAT IT SAID WHEN IT WAS RED
#
# It was written to go red against the unmodified thirteen-word switch and it
# did. Measured 2026-08-18 at link address 0x80200000: one line, `b2`, then the
# machine hangs. `qemu -d int` showed why, and showed two arms, not one —
# breakpoints at 0x8020003c and 0x8020006c, seventeen `supervisor_ecall`s (one
# printed line), then a `user_ecall`. The second `sret` fell to U-mode because
# `sret` clears `SPP` and nothing restored it. So `sepc` and `sstatus.SPP` are
# both exercised here.
#
# `stval` is NOT exercised by either arm. On `ebreak` it holds the same faulting
# address `sepc` does, so nothing here can tell the two apart. A green run of
# this check is not evidence about `stval`, and it was deliberately NOT added to
# `सञ्चारः`: a word saved on the strength of a check that cannot see it is
# armour, not evidence. Blocking inside a handler is not covered anywhere else.
#
# `C-002e2b` widened `सञ्चारः` to fifteen words — `sepc` at +160 and `sstatus`
# at +168 of the frame — and this check went green at both link addresses. Each
# arm was then removed on its own and watched to fail on its own: without the
# `sepc` restore, `a1` vanishes and `b2` takes its place; without the `sstatus`
# restore, `a1` and `b2` both print and the machine still dies at the second
# `sret`. Neither word is carried by the other, and neither is decoration.
#
# A failure here is now a REGRESSION in `सञ्चारः`, not an expected state.
#
# ## Absence is loud
#
# `W-087` found 27 `check-*.sh` that exit 0 when their tooling is missing, so a
# machine with nothing installed reports a green tree. This one exits **77** when
# it cannot run and **1** when it runs and fails, after `check-line-budget.sh`.
# It never exits 0 without having watched a machine.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "CANNOT RUN: qemu-system-riscv64 is not installed, so nothing was measured"
  exit 77; }

# The routine under test must be the SHIPPED routine. `spec/trap-seam.sas`
# carries a copy of `spec/schedule.sas`'s `सञ्चारः` so that what blocks inside a
# handler here is character for character what runs there — and a copy is only
# evidence for as long as it is still a copy. Nothing else compares them, and a
# drift would leave this check green about a routine no thread uses.
switch_of() {
    awk '/^सञ्चारःॱॱ$/ { f = 1 }
         f            { print }
         f && /^सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।$/ { exit }' "$1"
}
switch_of "$root/spec/trap-seam.sas" > "$tmp/a"
switch_of "$root/spec/schedule.sas"  > "$tmp/b"
[ -s "$tmp/a" ] && [ -s "$tmp/b" ] || {
  echo "  FAIL  सञ्चारः could not be found in one of the two files, so the copy"
  echo "        cannot be compared and nothing here is about the shipped routine"
  exit 1; }
cmp -s "$tmp/a" "$tmp/b" || {
  echo "  FAIL  spec/trap-seam.sas's सञ्चारः has drifted from spec/schedule.sas's:"
  diff "$tmp/b" "$tmp/a" | sed 's/^/          /'
  echo "        What blocks inside a handler here is then not what runs there, and"
  echo "        a green run below would be about a routine nothing else uses."
  exit 1; }

run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$root/spec/trap-seam.sas" "$tmp/s.elf" >/dev/null
    # Bounded by an explicit SIGKILL, not perl's alarm: QEMU handles SIGALRM and
    # survives it (W-060). A leaking sepc HANGS the machine rather than printing
    # something wrong, so an unbounded read hangs this check.
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/s.elf" > "$tmp/out" 2>&1 &
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
    # No `tail -N` — it returns min(actual, N), which makes a count assertion
    # one-sided by construction and blind to a surplus. See W-081.
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' || true
}

fail=0
# Two link addresses, because every address here is computed with auipc and one
# run cannot tell a computed address from a lucky constant.
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    if ! python3 - "$tmp/l" > "$tmp/v" 2>&1 <<'PY'
import sys

lines = [l.strip() for l in open(sys.argv[1]) if l.strip()]
vals = [int(l, 16) for l in lines]
MARK = {0xa1: "a1 — thread 1 returned to its OWN faulting site",
        0xb2: "b2 — thread 2 returned to its own faulting site",
        0xc3: "c3 — thread 1's callee-saved value survived both switches"}

# W-090: assert the impossible-value invariants of the INPUT before reading it
# as evidence about the subject. These cannot fire on any run of this program,
# correct or incorrect; they fire when the output is not this program's output.
for v in vals:
    if v not in MARK and v > 16:
        print(f"the output is not this program's: {v:#x} is neither a marker "
              f"({', '.join(hex(m) for m in MARK)}) nor a small fault count. "
              "Nothing is being claimed about the switch — read the raw log")
        sys.exit(1)

want = [0xa1, 0xb2, 2, 0xc3]
if vals == want:
    print("a1 then b2, two faults, and c3 across both switches: each thread "
          "resumed at its own site and neither lost its registers")
    sys.exit(0)

why = []
if 0xa1 not in vals and 0xb2 in vals:
    why.append("thread 1 never printed a1, and b2 appears in its place. Thread 1 "
               "blocked inside its handler; while it waited, thread 2's own trap "
               "overwrote sepc; thread 1 woke, read sepc, and returned to THREAD "
               "2's faulting site. sepc is not saved across सञ्चारः")
elif vals.count(0xb2) > 1:
    why.append(f"b2 printed {vals.count(0xb2)} times — one site was returned to "
               "more than once, which is the same sepc leak seen from the other end")
if 2 not in vals:
    why.append("the fault count never printed, so thread 1 did not reach its end. "
               "The machine most likely died after the second sret: sret clears "
               "sstatus.SPP, nothing restored it, and the second return therefore "
               "landed in U-mode")
if 0xc3 in vals and 0xa1 not in vals:
    why.append("c3 DID print, so the twelve callee-saved words are intact — what "
               "is missing is privileged state, not general state")
if not why:
    why.append(f"expected {[hex(w) for w in want]}, got {[hex(v) for v in vals]}")

print("\n".join(why))
sys.exit(1)
PY
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
done

[ "$fail" -eq 0 ] || {
    echo
    echo "a thread that blocks inside its trap handler does not come back to its"
    echo "own faulting site. This was the state BEFORE C-002e2b and is a"
    echo "REGRESSION now: सञ्चारः carries sepc at +160 and sstatus at +168, and"
    echo "spec/schedule.sas must carry the same routine character for character."
    exit 1; }

echo
echo "ok  a thread blocked inside its trap handler resumed at ITS OWN faulting"
echo "    site with its own registers, across two link addresses. This says"
echo "    nothing about stval, which ebreak leaves equal to sepc."
