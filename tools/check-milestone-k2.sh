#!/bin/sh
# MILESTONE M-K2, gate C2 — task `C-002j`, doc 11 §7.
#
# THIS CHECK IS RED ON PURPOSE AND `spec/milestone-k2.sas` DOES NOT EXIST YET.
# It is written first for the reason `C-002e2a` and `C-002e3a` were written
# first: a check composed after the program it checks is a check whose author
# already knew the answer, and it tests the answer rather than the claim. The
# rows that turn it green are `C-002k` (U-mode), `C-002l` (the second address
# space) and `C-002m` (the exchange). Until then this exits 1 and says which
# line of the log is missing.
#
# ## What M-K2 claims, and the one word of it that is NOT claimed here
#
# "Two userspace processes exchange messages." Three separate things, and the
# tree today has none of them:
#
#   USERSPACE — nothing has ever entered U-mode deliberately. It has been
#     reached exactly once and it was an ACCIDENT: `spec/trap-seam.sas` records
#     that a second `विघ्नप्रत्यागमः` returns with SPP already cleared and the
#     machine falls to U-level, which `qemu -d int` showed as a `user_ecall` at
#     0x8020017c. Falling into a mode is not running in it.
#   TWO PROCESSES — `C-002c` fixed a page's rights AT THE MOMENT IT IS MAPPED
#     and proved W^X by refusing, with a reason. It never installed the result
#     in `satp` and never asked what a second address space would answer.
#   EXCHANGE — `C-002e` and `C-002e3` carry a message between two THREADS, in
#     one address space, both in supervisor mode. The words cross a rendezvous;
#     they have never crossed an address-space boundary.
#
# The fourth word of the milestone — "under 2 microseconds" — IS NOT CHECKED
# HERE AND MAY NOT BE ADDED. `C-002` decided that before any harness existed:
# QEMU is not cycle-accurate and `rdtime` is not the host clock, so a
# microsecond produced here would be meaningless and official-looking at once.
# The round trip is bounded instead by the dispatcher-entry count that
# `tools/check-ipc-fastpath.sh` reports, which is a number QEMU tells the truth
# about. The wall-clock claim lands EXPLICITLY OUTSTANDING and needs `W-012b`'s
# measurement host, which does not exist. If a microsecond ever appears in this
# file, it is wrong however plausible it looks.
#
# ## The nine words, and why no earlier program can print them
#
# `spec/milestone-k2.sas` prints nine 16-hex-digit words, in this order:
#
#   1  satp as installed for process A      Sv39: mode nibble 8, nonzero
#   2  satp as installed for process B      Sv39, and DIFFERENT from 1
#   3  the word at the shared VA, read under A's satp
#   4  the word at that SAME VA, under B's  and DIFFERENT from 3
#   5  scause on A's ecall                  exactly 8
#   6  sstatus on A's ecall                 SPP (bit 8) clear
#   7  the message, read out of B's inbox   nonzero, and it MOVES (below)
#   8  scause on B's ecall                  exactly 8
#   9  the reply, read out of A's inbox     exactly word 7 plus one
#
# Words 3 and 4 are the whole of "two processes" and are the reason the shared
# virtual address is in the contract at all: two different `satp` values prove
# two different root tables and nothing more — a pair of page tables can be
# distinct objects describing an identical map. ONE ADDRESS ANSWERING TWO WAYS
# is separation that has been observed rather than arranged.
#
# Word 5 is the whole of "userspace", and it is architectural: cause 8 is
# ENVIRONMENT CALL FROM U-MODE (RISC-V privileged §4.1.8). A supervisor-mode
# `ecall` raises 9, and every `ecall` in this tree so far has raised 9. There is
# no way to forge an 8 from S-mode, which is exactly why it is the evidence.
# Word 6 is the same fact read from the other register, and both are demanded
# because either one alone can be produced by a kernel that sets a variable.
#
# Word 9 is the round trip. A reply equal to word 7 plus one cannot be a
# constant compiled into B — word 7 moves with the link address — so B computed
# on a word that reached it from A's address space.
#
# ## What MOVES and what MUST NOT, at two link addresses
#
# `tools/check-milestone-k1.sh`'s discipline, and it is what separates this from
# a program that prints constants:
#
#   MOVES  1, 2, 3, 4, 7, 9 — every one is derived from where we were loaded.
#          A program printing baked-in words is caught here and nowhere else.
#   FIXED  5, 8 — architectural cause numbers. A cause that moved with the link
#          address would mean the number is not coming from `scause` at all.
#
# Word 6 is neither: `sstatus` carries bits this program does not control, so
# only SPP is asserted and the rest is printed for a reader, not tested.
#
# ## What this does NOT prove, stated so the green is not over-read
#
#   - WHO computed word 9. The check sees a reply, not the mode it was computed
#     in; a kernel that answered on B's behalf would pass. B's cause-8 in word 8
#     says B ran in U-mode, not that B ran THIS.
#   - That the exchange used `द्वारम्`'s fast path. Cost is
#     `tools/check-ipc-fastpath.sh`'s and is not duplicated here.
#   - Isolation. Two address spaces answering differently at one address is not
#     the claim that neither can reach the other's frames; nothing here tries.
#
# ## Absence is loud
#
# `W-087`: 27 check scripts exit 0 when their tooling is missing, so a bare
# machine reports a green tree. This exits **77** when it cannot run and **1**
# when it runs and fails, and never 0 without having watched a machine.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
# NOT in $root: the gate fingerprints modified and untracked files, so a scratch
# file dropped in the repo is a file that exists while the gate is looking.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "CANNOT RUN: qemu-system-riscv64 is not installed, so nothing was measured"
  exit 77; }

prog=$root/spec/milestone-k2.sas
[ -f "$prog" ] || {
  echo "RED, as written: spec/milestone-k2.sas does not exist."
  echo
  echo "  C-002j put this contract on the record before the program that has to"
  echo "  satisfy it. The nine words it demands are in the header above; the rows"
  echo "  that produce them are C-002k (U-mode: words 1, 3, 5, 6), C-002l (the"
  echo "  second address space: 2, 4, 8) and C-002m (the exchange: 7, 9)."
  echo
  echo "  Nothing is wrong with the tree. This is the red a milestone check is"
  echo "  supposed to be while its milestone is open."
  exit 1; }

run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$prog" "$tmp/k2.elf" >/dev/null
    # Bounded by an explicit SIGKILL, not perl's alarm: QEMU handles SIGALRM and
    # outlives it (`W-060`). A process that never traps runs forever.
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/k2.elf" > "$tmp/out" 2>&1 &
    qpid=$!
    ( sleep 25; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    wait "$qpid" 2>/dev/null || true
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    pkill -P "$watcher" 2>/dev/null || true
    kill "$watcher" 2>/dev/null || true
    # No `tail -9`: it returns min(actual, 9), which makes a count assertion
    # one-sided by construction and blind to a surplus (`W-081`).
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' || true
}

fail=0
# Two link addresses, because every address in that program is computed with
# auipc and one run cannot tell a computed address from a lucky constant.
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l.$addr"
    if ! python3 - "$tmp/l.$addr" > "$tmp/v" 2>&1 <<'PY'
import sys

v = [int(line, 16) for line in open(sys.argv[1]) if line.strip()]
NAME = ("satp for process A", "satp for process B",
        "the shared VA under A", "the shared VA under B",
        "scause on A's ecall", "sstatus on A's ecall",
        "the message in B's inbox", "scause on B's ecall",
        "the reply in A's inbox")
U_ECALL, SPP = 8, 1 << 8

bad = []
if len(v) != 9:
    print(f"printed {len(v)} hex lines, expected 9. The words that should be "
          "there, in order, are: " + "; ".join(f"{i + 1} {n}"
                                               for i, n in enumerate(NAME)))
    sys.exit(1)

satp_a, satp_b, shared_a, shared_b, cause_a, sstatus_a, msg, cause_b, reply = v

# ---- two address spaces -------------------------------------------------
for i, s in ((1, satp_a), (2, satp_b)):
    if s >> 60 != 8:
        bad.append(f"word {i}, {NAME[i - 1]}: mode nibble {s >> 60:x}, not 8. "
                   "Sv39 is the only paging mode this tree installs, and a satp "
                   "that is not in it was never installed")
if satp_a == satp_b:
    bad.append(f"words 1 and 2 are the same satp ({satp_a:016x}), so both "
               "processes are in ONE address space and the milestone's 'two' "
               "is not on the record")
if shared_a == shared_b:
    bad.append(f"words 3 and 4: the shared virtual address answers {shared_a:016x} "
               "under both satps. Two root tables describing an identical map are "
               "two objects, not two address spaces — the separation has to be "
               "observed at an address, not inferred from a pointer")

# ---- userspace ----------------------------------------------------------
for i, c in ((5, cause_a), (8, cause_b)):
    if c != U_ECALL:
        who = "A" if i == 5 else "B"
        extra = (" — 9 is ENVIRONMENT CALL FROM S-MODE, which is every ecall "
                 "this tree has ever taken and is the one number that says the "
                 "process never left supervisor mode") if c == 9 else ""
        bad.append(f"word {i}, {NAME[i - 1]}: cause {c}, expected 8, ENVIRONMENT "
                   f"CALL FROM U-MODE. Process {who} did not trap from U-mode"
                   + extra)
if sstatus_a & SPP:
    bad.append(f"word 6, {NAME[5]}: SPP is set ({sstatus_a:016x}), so the trap "
               "came from supervisor mode. scause and SPP are demanded together "
               "because a kernel that sets a variable can produce either alone")

# ---- the exchange -------------------------------------------------------
if msg == 0:
    bad.append("word 7, the message in B's inbox: zero, which is what an inbox "
               "reads before anything arrives. Nothing distinguishes 'A sent 0' "
               "from 'A sent nothing'")
if reply != (msg + 1) & 0xffffffffffffffff:
    bad.append(f"word 9, the reply in A's inbox: {reply:016x}, and word 7 plus "
               f"one is {(msg + 1) & 0xffffffffffffffff:016x}. The round trip is "
               "the claim: B must compute on the word that reached it, and a "
               "reply that is not derived from word 7 is a constant B already had")

if bad:
    print("\n".join(bad))
    sys.exit(1)
print(f"satp {satp_a:016x}/{satp_b:016x}, one VA answering {shared_a:016x} and "
      f"{shared_b:016x}, both ecalls cause 8 with SPP clear, {msg:016x} out and "
      f"{reply:016x} back")
sys.exit(0)
PY
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
done

# ---- what moved, and what had better not --------------------------------
# Only reached when both runs are internally well formed; otherwise the words
# below are not the words this compares.
if [ "$fail" -eq 0 ]; then
    if ! python3 - "$tmp/l.०षोड्८०२०००००" "$tmp/l.०षोड्८०४०००००" > "$tmp/m" 2>&1 <<'PY'
import sys

lo = [int(x, 16) for x in open(sys.argv[1]) if x.strip()]
hi = [int(x, 16) for x in open(sys.argv[2]) if x.strip()]
NAME = ("satp for process A", "satp for process B",
        "the shared VA under A", "the shared VA under B",
        "scause on A's ecall", "sstatus on A's ecall",
        "the message in B's inbox", "scause on B's ecall",
        "the reply in A's inbox")
MOVES, FIXED = (0, 1, 2, 3, 6, 8), (4, 7)

bad = []
for i in MOVES:
    if lo[i] == hi[i]:
        bad.append(f"word {i + 1}, {NAME[i]}: {lo[i]:016x} at both link "
                   "addresses. It is derived from where the image was loaded, so "
                   "a value that did not move is a constant in the source and the "
                   "derivation it stands for never happened")
for i in FIXED:
    if lo[i] != hi[i]:
        bad.append(f"word {i + 1}, {NAME[i]}: {lo[i]:016x} then {hi[i]:016x}. "
                   "A cause number is architectural. One that follows the link "
                   "address is not coming from scause at all")
if bad:
    print("\n".join(bad))
    sys.exit(1)
print("every derived word moved between the two link addresses and both cause "
      "numbers stayed put")
sys.exit(0)
PY
    then sed 's/^/  FAIL  /' "$tmp/m"; fail=1
    else printf "  both addresses -> %s\n" "$(cat "$tmp/m")"; fi
fi

[ "$fail" -eq 0 ] || {
    echo
    echo "M-K2 is not on the record. Two userspace processes exchanging messages"
    echo "is four claims and this reports which one is missing: cause 8 with SPP"
    echo "clear is userspace, one virtual address answering two ways is two"
    echo "processes, and a reply derived from the word that crossed is the"
    echo "exchange. The fourth — under 2 microseconds — is NOT here by decision:"
    echo "QEMU cannot honestly produce it, tools/check-ipc-fastpath.sh bounds the"
    echo "round trip by dispatcher entries instead, and the wall-clock claim stays"
    echo "outstanding until W-012b's measurement host exists."
    exit 1; }

echo
echo "ok  at both link addresses: both processes trapped from U-mode (cause 8,"
echo "    SPP clear), one virtual address answered differently under each satp,"
echo "    and a word crossed from A to B and came back computed on. No"
echo "    microsecond is claimed here and none may be added — see the header."
