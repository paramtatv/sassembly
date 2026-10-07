#!/bin/sh
# Two address spaces — task `C-002l`, doc 11 §7, words 2, 4 and 8 of
# `tools/check-milestone-k2.sh`.
#
# ## The claim, and why a second `satp` is not it
#
# `spec/second-space.sas` builds TWO root page tables, runs a process under
# each, and reads ONE virtual address under both. Six words come back, in the
# order the program runs:
#
#   1  satp as installed for A       Sv39, mode nibble 8
#   2  the shared VA under A         A's own data frame
#   3  scause on A's ecall           exactly 8
#   4  satp as installed for B       Sv39, and different from 1
#   5  the same shared VA under B    different from 2
#   6  scause on B's ecall           exactly 8
#
# Words 1 and 4 being different proves two root tables and NOTHING MORE. Two
# page tables can be distinct objects describing an identical map, and then
# "two address spaces" is inferred from a pointer rather than observed. Words 2
# and 5 are the claim: one address, two answers. That is separation that was
# watched, not arranged.
#
# Word 6 is not word 3 repeated. The first process running in U-mode is not
# evidence that the second did; B fetches from its own root table, under its own
# `satp`, and returns by an `ecall` raising cause 8 — ENVIRONMENT CALL FROM
# U-MODE (RISC-V privileged §4.1.8), a number S-mode cannot forge, where every
# `ecall` this tree took before `C-002k` raised 9.
#
# ## Why words 2 and 5 are checked against word 1 and not against constants
#
# The program declares its layout: ten page frames from one runtime-aligned
# base, in order — A's root, B's root, A's megapage table, B's megapage table,
# the image megapage's page table (shared), A's window table, B's window table,
# the user's page (shared, and the only page carrying U), A's data frame, B's
# data frame. `satp` holds A's root page number, so the base is
# (word 1 & PPN mask) << 12, and the two data frames are base + 8×4096 and
# base + 9×4096 — which is exactly what each program stores in its own frame and
# reads back through the window at megapage 500.
#
# So every expected value here is DERIVED from word 1 and from the declared
# layout, including word 4: B's root is the frame after A's, so its page number
# is A's plus one. A program that printed plausible numbers without walking two
# maps fails here, and so does one whose frames are laid out differently from
# what the source says. The base is also checked to be page-aligned and to lie
# inside the `.bss` the ELF reserves, read by `riscv64-elf-readelf` — an oracle
# that reads the file and never runs the program.
#
# ## Two link addresses
#
# MOVES  1, 2, 4, 5. Every one is derived from where the image was loaded. A
#        word that is the same at both addresses is a constant in the source and
#        the derivation it stands for never happened.
# FIXED  3, 6. Cause numbers are architectural. One that followed the link
#        address would not be coming from `scause` at all.
#
# ## What a green here does NOT prove
#
#   - Isolation. One address answering two ways is not the claim that neither
#     process can reach the other's frames. Both maps carry the kernel's
#     identity mapping and nothing here tries to remove it.
#   - Different code. The two processes share the user page — the same two
#     instructions run under each map. The claim is about the address, not the
#     text.
#   - Any exchange. Not one word crosses between them; that is `C-002m`, and
#     words 7 and 9 of the milestone.
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

prog=$root/spec/second-space.sas
[ -f "$prog" ] || { echo "CANNOT RUN: spec/second-space.sas is missing"; exit 77; }

fail=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$addr" "$prog" "$tmp/ss.elf" >/dev/null

    # The section is found by NAME: `kosha` emits only the sections a program
    # uses (`B-071`), so `.bss` is not always the same index.
    riscv64-elf-readelf -S -W "$tmp/ss.elf" \
      | awk '{ for (i = 1; i <= NF; i++) if ($i == ".bss") print $(i+2), $(i+4) }' \
      > "$tmp/bss.$addr"

    # Bounded by an explicit SIGKILL, not perl's alarm: QEMU handles SIGALRM and
    # outlives it (`W-060`). A program that faults with no handler installed does
    # not stop — it takes the fault again on the handler and sits there.
    qemu-system-riscv64 -machine virt -smp 1 -m 256M -nographic \
        -bios default -kernel "$tmp/ss.elf" </dev/null > "$tmp/out" 2>&1 &
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
NAME = ("satp for A", "the shared VA under A", "scause on A's ecall",
        "satp for B", "the shared VA under B", "scause on B's ecall")
U_ECALL, PPN = 8, (1 << 44) - 1
FRAMES, DATA_A, DATA_B = 10, 8 * 4096, 9 * 4096

bad = []
if len(v) != 6:
    print(f"printed {len(v)} hex lines, expected 6: " +
          "; ".join(f"{i + 1} {n}" for i, n in enumerate(NAME)) +
          ". Fewer than six means the machine stopped where the line stops — a "
          "root table that does not map the kernel kills the fetch after its own "
          "satp write, and a window table reached through the wrong megapage "
          "entry kills the load that follows")
    sys.exit(1)

satp_a, shared_a, cause_a, satp_b, shared_b, cause_b = v
bss = open(sys.argv[2]).read().split()
bss_addr, bss_size = int(bss[0], 16), int(bss[1], 16)
base = (satp_a & PPN) << 12

# ---- two root tables ----------------------------------------------------
for i, s in ((1, satp_a), (4, satp_b)):
    if s >> 60 != 8:
        bad.append(f"word {i}, {NAME[i - 1]}: mode nibble {s >> 60:x}, not 8. "
                   "Sv39 is the only paging mode this tree installs, and a satp "
                   "that is not in it was never installed")
if satp_a == satp_b:
    bad.append(f"words 1 and 4 are the same satp ({satp_a:016x}), so both "
               "processes ran in ONE address space and there is no second root "
               "table to have observed anything about")
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

# ---- one address, two answers -------------------------------------------
if shared_a == shared_b:
    bad.append(f"words 2 and 5: the shared virtual address answers "
               f"{shared_a:016x} under BOTH satps. Two root tables describing an "
               "identical map are two objects, not two address spaces — the "
               "separation has to be observed at an address, not inferred from a "
               "pointer. Check that each megapage table sends megapage 500 to its "
               "OWN window table")
for i, (word, want, who) in ((2, (shared_a, base + DATA_A, "A")),
                             (5, (shared_b, base + DATA_B, "B"))):
    if word == 0:
        bad.append(f"word {i}, {NAME[i - 1]}: zero, which is what an unwritten "
                   "frame reads. Nothing distinguishes 'the window resolved' from "
                   "'the window resolved to nothing'")
    elif word != want:
        bad.append(f"word {i}, {NAME[i - 1]}: {word:016x}, and {who}'s data frame "
                   f"is {want:016x} by the declared layout. Each process stores "
                   "its own frame's address in that frame and reads it back "
                   "through a window at a virtual address that is not the frame's "
                   "own, so a word that is not that address did not come through "
                   f"{who}'s window")

# ---- both of them in U-mode ---------------------------------------------
for i, c, who in ((3, cause_a, "A"), (6, cause_b, "B")):
    extra = (" — 9 is ENVIRONMENT CALL FROM S-MODE, which is every ecall this "
             "tree took before C-002k and is the one number that says the "
             "process never left supervisor mode") if c == 9 else ""
    if c != U_ECALL:
        bad.append(f"word {i}, {NAME[i - 1]}: cause {c}, expected 8, ENVIRONMENT "
                   f"CALL FROM U-MODE. Process {who} did not trap from U-mode"
                   + extra)

if bad:
    print("\n".join(bad))
    sys.exit(1)
print(f"satp {satp_a:016x}/{satp_b:016x}, tables at {base:016x}, one VA reading "
      f"{shared_a:016x} then {shared_b:016x}, both ecalls cause 8")
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
NAME = ("satp for A", "the shared VA under A", "scause on A's ecall",
        "satp for B", "the shared VA under B", "scause on B's ecall")
MOVES, FIXED = (0, 1, 3, 4), (2, 5)

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
print("both satps and both windowed words moved between the two link addresses "
      "and both cause numbers stayed put")
sys.exit(0)
PY
    then sed 's/^/  FAIL  /' "$tmp/m"; fail=1
    else printf "  both addresses -> %s\n" "$(cat "$tmp/m")"; fi
fi

[ "$fail" -eq 0 ] || {
    echo
    echo "The second address space is not on the record. Two satps is not the"
    echo "claim — a pair of page tables can be distinct objects describing an"
    echo "identical map. The claim is one virtual address answering differently"
    echo "under each of them, with both processes trapping back from U-mode."
    echo "The exchange across that boundary is C-002m and is not tested here."
    exit 1; }

echo
echo "ok  at both link addresses: two root tables, one virtual address reading"
echo "    A's data frame under A's satp and B's under B's, and both processes"
echo "    returning from U-mode with cause 8. Nothing crossed between them —"
echo "    that is C-002m."
