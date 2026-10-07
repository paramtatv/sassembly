#!/bin/sh
# A page fault is handed out whole, and handed out once — `C-002h`, doc 11 §3.3.
#
# Doc 11 §3.3 puts page faults in the hands of a USERSPACE PAGER, and says that
# is what makes demand paging, memory-mapped files and copy-on-write userspace
# policies. Read the other way round, it says something sharper: the kernel does
# not decide what a fault MEANS. It packages the fault faithfully and hands it
# out. So what is provable here is not paging policy — it is the honesty of the
# package.
#
# ## Three claims
#
#   1. The package carries the faulting address (`stval`), the cause (`scause`)
#      and the instruction that faulted (`sepc`), and each is the REAL value.
#      The program prints what each should be BEFORE it touches anything; the
#      handler prints what it actually got. The claim is the match. A handler
#      that hands back the page base, or the PC in place of the address, or a
#      plausible constant, dies on one of those comparisons.
#
#   2. Read, write and execute are TELLABLE APART — 13, 15 and 12 in RISC-V. A
#      pager that cannot tell them apart cannot implement copy-on-write, which
#      is the entire reason this lives in userspace. All three faults here are
#      taken on the SAME gigapage, so the cause cannot be coming from the
#      address; only the kind of touch is left.
#
#   3. The fault is delivered ONCE. The handler never advances `sepc` — it
#      RESOLVES, by installing an entry, and the faulting instruction runs
#      again and succeeds. Returning without resolving would re-run the same
#      instruction forever and print nothing at all, which is why the handler
#      counts and stops itself at a fourth fault: the failure gets a face
#      instead of a hang.
#
# ## The hole, and why it is an alias
#
# The identity map is 512 gigapages with the fourth left EMPTY — 4 GiB to 5 GiB
# unmapped. The pager fills it, and fills it onto the KERNEL'S OWN gigapage (2),
# so a resolved address lands on the same physical memory the low address does.
# That is what lets this check say the pager installed the RIGHT entry and not
# merely an entry: a byte written through the alias is read back from below.
#
# ## W^X survives here too
#
#   13 load        -> V R A       = 0x43
#   15 store       -> V R W A D   = 0xc7
#   12 instruction -> V R X A     = 0x4b
#
# No entry ever holds W and X together (`C-002c`). Resolving the store fault
# drops X and resolving the instruction fault drops W, and the entry is printed
# so that this is SHOWN rather than asserted.
#
# ## What this row does NOT contain
#
# The pager is not a separate thread. Threads arrived in `C-002d`, but द्वारम्
# (`C-002e`) has not, and without synchronous IPC there is no way to carry a
# fault to another thread at all. So the handler stands in the pager's place and
# only what can be proved is proved: the package and its once-ness. The HANDOFF
# is a row after `C-002e`. Policy is absent on purpose too — where a page comes
# from is the pager's question, and this row is about the interface.
#
# One more thing this does not prove: the `sfence.vma` the pager issues after
# installing the entry. Deleting it was tried as a mutation and THIS CHECK STILL
# PASSED. QEMU never holds the stale translation that would punish its absence —
# it does not cache a failed walk, and it keeps the write permit separately — so
# on this machine the fence is unfalsifiable. It stays in the program because
# the privileged spec requires it and real hardware does hold that entry; it is
# named here so nobody reads a passing check as evidence for it.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$root/spec/pager.sas" "$tmp/p.elf" >/dev/null
    # Bounded by an explicit SIGKILL, never perl's alarm: QEMU handles SIGALRM
    # and survives it (W-060). The bound is load-bearing here — an unresolved
    # fault is an infinite loop BY DESIGN, and it is silent while it runs.
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/p.elf" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 20; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -21
}

fail=0; first=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    n=$(wc -l < "$tmp/l" | tr -d ' ')
    if [ "$n" -ne 21 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 21 (six for each of"
        echo "        three faults, then the count of faults delivered)"
        if [ "$(tail -1 "$tmp/l")" = "0000000000000004" ]; then
            echo "        the last line is 4: a FOURTH fault arrived, so one of the"
            echo "        three was delivered again — the handler returned without"
            echo "        resolving it and the faulting instruction re-ran"
        fi
        fail=1; continue
    fi
    if ! python3 - "$tmp/l" > "$tmp/v" 2>&1 <<'PY'
import sys
v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]

V, R, W, X, A, D = 1, 2, 4, 8, 64, 128
FRAME = 2 << 30                 # the kernel's own gigapage, which the hole aliases
HOLE  = 4 << 30                 # 4 GiB .. 5 GiB, the entry left empty on purpose
SEED  = 0x6c1000006c1           # 1729 in both halves, planted below, read via the alias
MARK  = 0x65200000652           # 1618 in both halves, written via the alias, read below

# name, cause, the right the pager must grant, the right it must NOT grant
KINDS = [("load",        13, R, X),
         ("store",       15, W, X),
         ("instruction", 12, X, W)]
CAUSE = {13: "load page fault (13)", 15: "store/AMO page fault (15)",
         12: "instruction page fault (12)"}

want, told, sepcs, ptes = [v[0], v[7], v[14]], [v[2], v[9], v[15]], [], []
bad = []

def entry(i, name, pte, grant, deny, gname, dname):
    if not pte & V:
        bad.append(f"the {name} fault was resolved with entry {pte:#x}, which has V clear "
                   "— the pager answered with a mapping that does not map")
        return
    if (pte >> 10) << 12 != FRAME:
        bad.append(f"the {name} fault was resolved onto frame {(pte >> 10) << 12:#x}, not the "
                   f"kernel's own gigapage {FRAME:#x} — the pager installed an entry but not "
                   "the one it meant, and the alias no longer names the memory below it")
    if not pte & grant:
        bad.append(f"the {name} fault was resolved with entry {pte:#x}, which does not grant "
                   f"{gname} — the pager did not act on the cause it was given, so the very "
                   "same fault is about to arrive again")
    if pte & deny:
        bad.append(f"the {name} fault was resolved with entry {pte:#x}, which grants {dname} "
                   f"as well as {gname}")
    if pte & W and pte & X:
        bad.append(f"the {name} fault was resolved with entry {pte:#x}: writable AND "
                   "executable. W^X is the rule `C-002c` fixed globally, and a pager that "
                   "breaks it while resolving breaks it everywhere")

# ---- fault 1: a load, on an entry that is not there at all ----
addr, pc, cause, stval, sepc, pte, seen = v[0:7]
if cause != 13:
    bad.append(f"reading an unmapped address reported {CAUSE.get(cause, cause)}, "
               f"expected {CAUSE[13]}")
if stval != addr:
    bad.append(f"the load fault reported stval {stval:#x}, but the address touched was "
               f"{addr:#x}" + (" — that is the base of the page, not the address, and a "
               "pager cannot place a byte it was never told about"
               if stval == addr & ~0xfff else ""))
if sepc != pc:
    bad.append(f"the load fault reported sepc {sepc:#x}, but the faulting load is at "
               f"{pc:#x}" + (" — that is the faulting ADDRESS, not the faulting "
               "instruction" if sepc == addr else ""))
if stval == sepc:
    bad.append("the load fault reported the same value for stval and sepc; a data fault "
               "has an address and an instruction and they are two different things")
entry(0, "load", pte, R, X, "R", "X")
if seen != SEED:
    bad.append(f"reading through the resolved alias gave {seen:#x}, expected {SEED:#x} — the "
               "fault was made to go away without the mapping reaching the right memory")
sepcs.append(sepc); ptes.append(pte)

# ---- fault 2: a store, on an entry that is valid and readable but not writable ----
addr, pc, cause, stval, sepc, pte, back = v[7:14]
if cause != 15:
    bad.append(f"writing a mapped read-only address reported {CAUSE.get(cause, cause)}, "
               f"expected {CAUSE[15]} — this is exactly the fault copy-on-write is built "
               "on, and a pager that sees it as a read fault will hand back a shared page")
if stval != addr:
    bad.append(f"the store fault reported stval {stval:#x}, but the address written was "
               f"{addr:#x}")
if sepc != pc:
    bad.append(f"the store fault reported sepc {sepc:#x}, but the faulting store is at {pc:#x}")
if stval == sepc:
    bad.append("the store fault reported the same value for stval and sepc")
entry(1, "store", pte, W, X, "W", "X")
if back != MARK:
    bad.append(f"the byte written through the alias read back as {back:#x} from the low "
               f"address, expected {MARK:#x} — the alias and the low address are not the "
               "same frame, so the pager resolved onto memory nobody else can see")
sepcs.append(sepc); ptes.append(pte)

# ---- fault 3: an instruction fetch, on an entry that is readable and writable ----
addr, cause, stval, sepc, pte, landed = v[14:20]
if cause != 12:
    bad.append(f"jumping into a mapped non-executable page reported {CAUSE.get(cause, cause)}, "
               f"expected {CAUSE[12]}")
if stval != addr:
    bad.append(f"the instruction fault reported stval {stval:#x}, expected {addr:#x}")
if sepc != addr:
    bad.append(f"the instruction fault reported sepc {sepc:#x}, expected {addr:#x}")
if stval != sepc:
    bad.append(f"the instruction fault reported stval {stval:#x} and sepc {sepc:#x}. On an "
               "instruction fault the faulting address IS the PC, and a handler that "
               "reports two different things for them has invented one of them")
entry(2, "instruction", pte, X, W, "X", "W")
if landed != addr:
    bad.append(f"after the instruction fault was resolved, execution continued at "
               f"{landed:#x}, not {addr:#x} — the pager did not put control back where the "
               "fault was taken")
sepcs.append(sepc); ptes.append(pte)

# ---- the three causes are the point: they must not collapse into one ----
for i in range(3):
    for j in range(i + 1, 3):
        if told[i] == told[j]:
            bad.append(f"the {KINDS[i][0]} fault and the {KINDS[j][0]} fault both reported "
                       f"cause {told[i]} — they are one fault to the pager, and copy-on-write "
                       "needs them to be two")

# ---- all three on one gigapage, so the cause cannot have come from the address ----
for (name, _, _, _), a in zip(KINDS, want):
    if not HOLE <= a < HOLE + (1 << 30):
        bad.append(f"the {name} fault was taken at {a:#x}, which is outside the empty "
                   f"gigapage {HOLE:#x}..{HOLE + (1 << 30):#x} the pager owns")

# ---- delivered once, and exactly once ----
if v[20] != 3:
    bad.append(f"{v[20]} faults were delivered, expected 3" + (" — a fault was handed out "
               "again, so the handler returned without resolving it" if v[20] > 3 else ""))

if bad:
    print("\n".join(bad)); sys.exit(1)
print("stval, scause and sepc are the real ones at all three faults; load, store and "
      "instruction stay distinct on one gigapage; each was delivered exactly once")
# machine-readable tail for the cross-run comparison
print("MOVES " + " ".join(f"{x:016x}" for x in want + sepcs))
print("FIXED " + " ".join(f"{x:016x}" for x in told + ptes))
PY
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(sed -n 1p "$tmp/v")"

    moves=$(sed -n 's/^MOVES //p' "$tmp/v"); fixed=$(sed -n 's/^FIXED //p' "$tmp/v")
    if [ -z "$first" ]; then first="$moves|$fixed"; continue; fi
    # What is DERIVED must move with the link address: the addresses touched and
    # the instructions that touched them are computed, and a single run cannot
    # tell a computation from a lucky constant.
    if [ "$moves" = "${first%|*}" ]; then
        echo "  FAIL  the faulting addresses and instruction pointers are identical at both"
        echo "        link addresses — they are constants, not values read out of the machine"
        fail=1
    fi
    # What is ARCHITECTURAL must not: 13, 15 and 12 are RISC-V's numbers, and
    # the rights the pager grants are fixed by the cause, not by where it links.
    if [ "$fixed" != "${first#*|}" ]; then
        echo "  FAIL  the causes or the installed entries differ between link addresses:"
        echo "          $(printf '%s' "${first#*|}")"
        echo "          $fixed"
        echo "        13, 15 and 12 are architectural and the rights follow the cause; if"
        echo "        either moved with the link address, something is being read wrong"
        fail=1
    fi
done
[ "$fail" -eq 0 ] || { echo; echo "the fault is not being handed out faithfully."; exit 1; }
echo
echo "ok  a page fault carries its address, its cause and its instruction unaltered;"
echo "    read, write and execute arrive as 13, 15 and 12 from one and the same"
echo "    gigapage; each is resolved by an entry that answers the cause and never"
echo "    grants W and X together, and each is delivered exactly once"
