#!/bin/sh
# The crossing — task `C-002m1`, doc 11 §7, on the way to word 7 of
# `tools/check-milestone-k2.sh`.
#
# ## The claim
#
# `spec/crossing.sas` carries ONE word out of process A's address space into
# process B's. Six words come back, in the order the program runs:
#
#   1  satp as installed for A       Sv39, mode nibble 8
#   2  scause on A's ecall           exactly 8
#   3  the word A sent               A's own inbox frame's address
#   4  satp as installed for B       Sv39, and different from 1
#   5  scause on B's ecall           exactly 8
#   6  the word B sent back          exactly word 3
#
# `C-002l` built two maps and read one address under each; not a word moved
# between them. Here A loads a word from its inbox through the window at
# megapage 500, hands it to the kernel by `ecall`, the kernel writes it into
# B's inbox, and B loads it from THE SAME virtual address under its own `satp`
# and hands it back. Word 6 equalling word 3 is the crossing.
#
# ## The control, which is what makes word 6 mean anything
#
# A word that is equal on both sides can be equal because it crossed or because
# both sides were looking at one frame. So the kernel EMPTIES A's inbox in the
# same breath as it fills B's, before B runs. If the two window tables were
# accidentally one — the exact confusion `C-002l` exists to refuse — B would
# read that zero and word 6 would come back `0000000000000000`. Run: pointing
# B's megapage 500 at A's window table does precisely that, leaving words 1, 2,
# 4 and 5 untouched.
#
# ## Why the expected values are derived and not written down
#
# The program declares its layout: ten page frames from one runtime-aligned
# base, in order — A's root, B's root, A's megapage table, B's megapage table,
# the image megapage's page table (shared), A's window table, B's window table,
# the user's page (shared, and the only page carrying X and U), A's inbox, B's
# inbox. `satp` holds A's root page number, so the base is
# (word 1 & PPN mask) << 12 and A's inbox is base + 8×4096 — which is exactly
# the word the program stores into that inbox before paging is on, and exactly
# what word 3 has to be. Word 4 is A's frame plus one. Nothing here is a
# constant a passing program could have been written around.
#
# The base is also checked to be page-aligned and to lie inside the `.bss` the
# ELF reserves, read by `riscv64-elf-readelf` — an oracle that reads the file
# and never runs the program.
#
# ## Two link addresses
#
# MOVES  1, 3, 4, 6. Every one is derived from where the image was loaded. A
#        word that is the same at both addresses is a constant in the source and
#        the derivation it stands for never happened — and for words 3 and 6
#        that is the whole guard: a message that did not move is a message B
#        could have been born holding.
# FIXED  2, 5. Cause numbers are architectural. One that followed the link
#        address would not be coming from `scause` at all.
#
# ## What a green here does NOT prove
#
#   - An exchange. Nothing comes back the other way, and B echoes the word
#     rather than computing on it. A reply derived from the word that crossed —
#     word 7 plus one, landing in A's inbox and read there — is `C-002m2` and
#     words 7 and 9 of the milestone.
#   - That B is the only reader. Both maps still carry the kernel's identity
#     mapping, and the kernel reaches both inboxes through it on purpose: the
#     inbox pages carry U, so a window read from S-mode would need SUM, and the
#     same frames are mapped a second time at their identity addresses without
#     U. Nothing here tries to take a mapping away from anyone.
#   - Cost. The round trip's bound is `tools/check-ipc-fastpath.sh`'s
#     dispatcher-entry count and is not duplicated here. No microsecond is
#     produced under QEMU, by `C-002`'s decision.
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
command -v riscv64-elf-readelf >/dev/null 2>&1 || {
  echo "CANNOT RUN: riscv64-elf-readelf is not installed, so the .bss the ten"
  echo "            frames have to fall inside was never read"
  exit 77; }

prog=$root/spec/crossing.sas
[ -f "$prog" ] || { echo "CANNOT RUN: spec/crossing.sas is missing"; exit 77; }

fail=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$addr" "$prog" "$tmp/cr.elf" >/dev/null

    # The section is found by NAME: `kosha` emits only the sections a program
    # uses (`B-071`), so `.bss` is not always the same index.
    riscv64-elf-readelf -S -W "$tmp/cr.elf" \
      | awk '{ for (i = 1; i <= NF; i++) if ($i == ".bss") print $(i+2), $(i+4) }' \
      > "$tmp/bss.$addr"

    # Bounded by an explicit SIGKILL, not perl's alarm: QEMU handles SIGALRM and
    # outlives it (`W-060`). The user's last instruction is a branch to itself,
    # so a process that is never trapped out of runs forever.
    qemu-system-riscv64 -machine virt -smp 1 -m 256M -nographic \
        -bios default -kernel "$tmp/cr.elf" </dev/null > "$tmp/out" 2>&1 &
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
    wait "$watcher" 2>/dev/null || true
    # No `tail -6`: it returns min(actual, 6), which makes a count assertion
    # one-sided by construction and blind to a surplus (`W-081`).
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' > "$tmp/l.$addr" || true

    if ! python3 - "$tmp/l.$addr" "$tmp/bss.$addr" > "$tmp/v" 2>&1 <<'PY'
import sys

v = [int(line, 16) for line in open(sys.argv[1]) if line.strip()]
NAME = ("satp for A", "scause on A's ecall", "the word A sent",
        "satp for B", "scause on B's ecall", "the word B sent back")
U_ECALL, PPN = 8, (1 << 44) - 1
FRAMES, INBOX_A = 10, 8 * 4096

bad = []
if len(v) != 6:
    print(f"printed {len(v)} hex lines, expected 6: " +
          "; ".join(f"{i + 1} {n}" for i, n in enumerate(NAME)) +
          ". Fewer than six means the machine stopped where the line stops — a "
          "window page without U kills the user's load before its ecall, and a "
          "root table that does not map the kernel kills the fetch right after "
          "its own satp write")
    sys.exit(1)

satp_a, cause_a, msg, satp_b, cause_b, back = v
bss = open(sys.argv[2]).read().split()
bss_addr, bss_size = int(bss[0], 16), int(bss[1], 16)
base = (satp_a & PPN) << 12

# ---- two address spaces to cross between --------------------------------
for i, s in ((1, satp_a), (4, satp_b)):
    if s >> 60 != 8:
        bad.append(f"word {i}, {NAME[i - 1]}: mode nibble {s >> 60:x}, not 8. "
                   "Sv39 is the only paging mode this tree installs, and a satp "
                   "that is not in it was never installed")
if satp_a == satp_b:
    bad.append(f"words 1 and 4 are the same satp ({satp_a:016x}), so both "
               "processes ran in ONE address space and nothing crossed a "
               "boundary because there was no boundary")
elif (satp_b & PPN) != (satp_a & PPN) + 1:
    bad.append(f"word 4, {NAME[3]}: page number {satp_b & PPN:x}, and the "
               f"declared layout puts B's root in the frame after A's, "
               f"{(satp_a & PPN) + 1:x}. The satp that was installed is not the "
               "table the source says was built")
if base % 4096:
    bad.append(f"word 1: A's root table is at {base:016x}, which is not page "
               "aligned — satp keeps a page NUMBER and the low bits are lost")
if not (bss_addr <= base and base + FRAMES * 4096 <= bss_addr + bss_size):
    bad.append(f"word 1: the frames start at {base:016x}, and the ten of them do "
               f"not fit inside the .bss the ELF reserves ({bss_addr:016x} + "
               f"{bss_size:x})")

# ---- both of them in U-mode ---------------------------------------------
for i, c, who in ((2, cause_a, "A"), (5, cause_b, "B")):
    extra = (" — 9 is ENVIRONMENT CALL FROM S-MODE, which is every ecall this "
             "tree took before C-002k and is the one number that says the "
             "process never left supervisor mode") if c == 9 else ""
    if c != U_ECALL:
        bad.append(f"word {i}, {NAME[i - 1]}: cause {c}, expected 8, ENVIRONMENT "
                   f"CALL FROM U-MODE. Process {who} did not trap from U-mode"
                   + extra)

# ---- the crossing -------------------------------------------------------
if msg == 0:
    bad.append("word 3, the word A sent: zero, which is what an unwritten frame "
               "reads. Nothing distinguishes 'A sent 0' from 'A's load never "
               "reached its inbox'")
elif msg != base + INBOX_A:
    bad.append(f"word 3, {NAME[2]}: {msg:016x}, and A's inbox frame is "
               f"{base + INBOX_A:016x} by the declared layout. That address is "
               "what the program writes into that frame before paging is on, so "
               "a word that is not it did not come through A's window")
if back == 0:
    bad.append("word 6, the word B sent back: zero, and zero is exactly what A's "
               "inbox is emptied to before B runs. B read a frame A had just "
               "been cleared out of — its window is reaching A's inbox, not its "
               "own, so the two window tables are one and nothing crossed")
elif back != msg:
    bad.append(f"word 6, {NAME[5]}: {back:016x}, and the word A sent was "
               f"{msg:016x}. B loaded from the same virtual address under its own "
               "satp, so a different word means what the kernel wrote into B's "
               "inbox is not what B found there")

if bad:
    print("\n".join(bad))
    sys.exit(1)
print(f"satp {satp_a:016x}/{satp_b:016x}, tables at {base:016x}, {msg:016x} out "
      f"of A's inbox and {back:016x} out of B's, both ecalls cause 8")
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
NAME = ("satp for A", "scause on A's ecall", "the word A sent",
        "satp for B", "scause on B's ecall", "the word B sent back")
MOVES, FIXED = (0, 2, 3, 5), (1, 4)

bad = []
for i in MOVES:
    if lo[i] == hi[i]:
        bad.append(f"word {i + 1}, {NAME[i]}: {lo[i]:016x} at both link "
                   "addresses. It is derived from where the image was loaded, so "
                   "a value that did not move is a constant in the source and the "
                   "derivation it stands for never happened")
for i in FIXED:
    if lo[i] != hi[i]:
        bad.append(f"word {i + 1}, {NAME[i]}: {lo[i]:016x} then {hi[i]:016x}. A "
                   "cause number is architectural. One that follows the link "
                   "address is not coming from scause at all")
if bad:
    print("\n".join(bad))
    sys.exit(1)
print("both satps and the word on both sides of the crossing moved between the "
      "two link addresses, and both cause numbers stayed put")
sys.exit(0)
PY
    then sed 's/^/  FAIL  /' "$tmp/m"; fail=1
    else printf "  both addresses -> %s\n" "$(cat "$tmp/m")"; fi
fi

[ "$fail" -eq 0 ] || {
    echo
    echo "The crossing is not on the record. A word equal on both sides is not"
    echo "the claim on its own — it is the claim because A's inbox is emptied"
    echo "before B runs, so a B reading A's frame reads zero. Both processes"
    echo "have to trap from U-mode, the two satps have to be the two the layout"
    echo "says, and the word has to move with the link address. The reply"
    echo "derived from it is C-002m2 and is not tested here."
    exit 1; }

echo
echo "ok  at both link addresses: a word left A's inbox through the window at"
echo "    megapage 500, crossed into B's address space, and came back out of B's"
echo "    inbox at the same virtual address — with A's inbox emptied in between,"
echo "    so a shared window would have returned zero. No reply is computed;"
echo "    that is C-002m2."
