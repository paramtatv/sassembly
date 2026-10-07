#!/bin/sh
# A process runs in U-mode — task `C-002k`, doc 11 §7, words 1, 3, 5 and 6 of
# `tools/check-milestone-k2.sh`.
#
# ## The claim, and the one number that carries it
#
# `spec/user-mode.sas` builds a three-level Sv39 map, marks exactly one 4 KiB
# page `U`, copies two instructions onto it, installs `satp`, clears SPP and
# `sret`s. The page `ecall`s. Four words come back:
#
#   1  satp as installed             Sv39, mode nibble 8
#   3  the word at the shared VA     read through a window that is not the
#                                    image's own address
#   5  scause on that ecall          exactly 8
#   6  sstatus at the same moment    SPP (bit 8) clear
#
# Word 5 is the claim. Cause 8 is ENVIRONMENT CALL FROM U-MODE (RISC-V
# privileged §4.1.8); a supervisor `ecall` raises 9, and every `ecall` this tree
# has ever taken has raised 9. An 8 cannot be forged from S-mode, which is why
# it is the evidence rather than a print statement saying "userspace". Word 6 is
# the same fact read out of a different register, and both are demanded because
# a kernel that sets a variable can produce either one alone.
#
# Falling into a mode is not running in it. `spec/trap-seam.sas` reached U-level
# once by accident — a second `विघ्नप्रत्यागमः` returning with SPP already clear,
# seen as a `user_ecall` at 0x8020017c under `qemu -d int`. That accident printed
# nothing and returned nowhere. This one is entered on purpose and comes back.
#
# ## Why word 3 is checked against word 1 and not against a constant
#
# The program declares its layout: six page frames from one runtime-aligned
# base, in order — root table, megapage table, page table, window table, the
# user's page, the data page. `satp` holds the root table's page number, so the
# base is (word 1 & PPN mask) << 12, and the user's page is base + 4×4096. That
# is exactly the word the program stores in the data page and reads back through
# the window. So word 3 has an expected value that is DERIVED from word 1 and
# from nothing else this check was told.
#
# A program that printed the right-looking number without walking the window
# fails here, and so does one whose frames are laid out differently from what
# the source says. The base is also checked to be page-aligned and to lie inside
# the `.bss` the ELF reserves, read by `riscv64-elf-readelf` — an oracle that
# reads the file and has never run the program.
#
# ## Two link addresses
#
# MOVES  1 and 3. Both are derived from where the image was loaded. A word that
#        is the same at both addresses is a constant in the source, and the
#        derivation it stands for never happened.
# FIXED  5. A cause number is architectural. One that followed the link address
#        would not be coming from `scause` at all.
#
# Word 6 is neither: `sstatus` carries bits this program does not control (UXL,
# FS, SD), so only SPP is asserted and the rest is printed for a reader.
#
# ## What a green here does NOT prove
#
#   - Two address spaces. There is one `satp` and one map; the shared VA answers
#     one way because there is only one way for it to answer. `C-002l` adds the
#     second process and asks the same address to answer differently.
#   - Isolation. The user's page sits four frames from the kernel's tables in
#     the same `.bss`, reachable by the kernel and unreachable by nothing.
#   - Any exchange. Nothing crosses; that is `C-002m`.
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
  echo "CANNOT RUN: riscv64-elf-readelf is not installed, so the .bss the satp"
  echo "            page number has to fall inside was never read"
  exit 77; }

prog=$root/spec/user-mode.sas
[ -f "$prog" ] || { echo "CANNOT RUN: spec/user-mode.sas is missing"; exit 77; }

fail=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$addr" "$prog" "$tmp/um.elf" >/dev/null

    # The section is found by NAME: `kosha` emits only the sections a program
    # uses (`B-071`), so `.bss` is not always the same index.
    riscv64-elf-readelf -S -W "$tmp/um.elf" \
      | awk '{ for (i = 1; i <= NF; i++) if ($i == ".bss") print $(i+2), $(i+4) }' \
      > "$tmp/bss.$addr"

    # Bounded by an explicit SIGKILL, not perl's alarm: QEMU handles SIGALRM and
    # outlives it (`W-060`). A program that faults with no handler installed does
    # not stop — it takes the fault again on the handler and sits there.
    qemu-system-riscv64 -machine virt -smp 1 -m 256M -nographic \
        -bios default -kernel "$tmp/um.elf" </dev/null > "$tmp/out" 2>&1 &
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
    # No `tail -4`: it returns min(actual, 4), which makes a count assertion
    # one-sided by construction and blind to a surplus (`W-081`).
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' > "$tmp/l.$addr" || true

    if ! python3 - "$tmp/l.$addr" "$tmp/bss.$addr" > "$tmp/v" 2>&1 <<'PY'
import sys

v = [int(line, 16) for line in open(sys.argv[1]) if line.strip()]
NAME = ("satp as installed", "the word at the shared VA",
        "scause on the ecall", "sstatus at the same moment")
U_ECALL, SPP, PPN = 8, 1 << 8, (1 << 44) - 1
USER_FRAME = 4 * 4096

bad = []
if len(v) != 4:
    print(f"printed {len(v)} hex lines, expected 4: " +
          "; ".join(f"{i + 1} {n}" for i, n in enumerate(NAME)) +
          ". Fewer than four means the machine stopped where the line stops — "
          "a bad root table kills the fetch after the satp write, and a page "
          "without U kills the sret")
    sys.exit(1)

satp, shared, cause, sstatus = v
bss = open(sys.argv[2]).read().split()
bss_addr, bss_size = int(bss[0], 16), int(bss[1], 16)
base = (satp & PPN) << 12

# ---- the map ------------------------------------------------------------
if satp >> 60 != 8:
    bad.append(f"word 1, {NAME[0]}: mode nibble {satp >> 60:x}, not 8. Sv39 is "
               "the only paging mode this tree installs, and a satp that is not "
               "in it was never installed")
if base % 4096:
    bad.append(f"word 1: the root table is at {base:016x}, which is not page "
               "aligned — satp keeps a page NUMBER and the low bits are lost")
if not (bss_addr <= base and base + 6 * 4096 <= bss_addr + bss_size):
    bad.append(f"word 1: the root table is at {base:016x}, and the six frames "
               f"from it do not fit inside the .bss the ELF reserves "
               f"({bss_addr:016x} + {bss_size:x})")

# ---- the window ---------------------------------------------------------
if shared == 0:
    bad.append("word 3, the word at the shared VA: zero, which is what an "
               "unwritten frame reads. Nothing distinguishes 'the window "
               "resolved' from 'the window resolved to nothing'")
elif shared != base + USER_FRAME:
    bad.append(f"word 3, {NAME[1]}: {shared:016x}, and the fifth frame from the "
               f"root table is {base + USER_FRAME:016x}. The program stores the "
               "user's page address in the data frame and reads it back through "
               "a window at a virtual address that is not the frame's own, so a "
               "word that is not that address did not come through the window")

# ---- userspace ----------------------------------------------------------
if cause != U_ECALL:
    extra = (" — 9 is ENVIRONMENT CALL FROM S-MODE, which is every ecall this "
             "tree has ever taken and is the one number that says the process "
             "never left supervisor mode") if cause == 9 else ""
    bad.append(f"word 5, {NAME[2]}: cause {cause}, expected 8, ENVIRONMENT CALL "
               f"FROM U-MODE. The process did not trap from U-mode{extra}")
if sstatus & SPP:
    bad.append(f"word 6, {NAME[3]}: SPP is set ({sstatus:016x}), so the trap "
               "came from supervisor mode. scause and SPP are demanded together "
               "because a kernel that sets a variable can produce either alone")

if bad:
    print("\n".join(bad))
    sys.exit(1)
print(f"satp {satp:016x}, tables at {base:016x}, the window reads {shared:016x}, "
      f"cause {cause} with SPP clear in {sstatus:016x}")
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
NAME = ("satp as installed", "the word at the shared VA",
        "scause on the ecall", "sstatus at the same moment")

bad = []
for i in (0, 1):
    if lo[i] == hi[i]:
        bad.append(f"word {i + 1}, {NAME[i]}: {lo[i]:016x} at both link "
                   "addresses. It is derived from where the image was loaded, so "
                   "a value that did not move is a constant in the source and the "
                   "derivation it stands for never happened")
if lo[2] != hi[2]:
    bad.append(f"word 5, {NAME[2]}: {lo[2]:016x} then {hi[2]:016x}. A cause "
               "number is architectural. One that follows the link address is "
               "not coming from scause at all")
if bad:
    print("\n".join(bad))
    sys.exit(1)
print("satp and the windowed word both moved between the two link addresses and "
      "the cause number stayed put")
sys.exit(0)
PY
    then sed 's/^/  FAIL  /' "$tmp/m"; fail=1
    else printf "  both addresses -> %s\n" "$(cat "$tmp/m")"; fi
fi

[ "$fail" -eq 0 ] || {
    echo
    echo "U-mode is not on the record. What this asks is one thing: did a process"
    echo "run in user mode under its own satp and trap back? Cause 8 with SPP"
    echo "clear is the whole of it, and it is architectural — nothing in S-mode"
    echo "can produce that number. Two address spaces (C-002l) and an exchange"
    echo "across them (C-002m) are separate claims and are not tested here."
    exit 1; }

echo
echo "ok  at both link addresses: a three-level Sv39 map with U on exactly one"
echo "    page, entered by sret with SPP clear, and an ecall from that page"
echo "    raising cause 8 with SPP still clear. The shared VA read back the"
echo "    address of the user's own page through a window that is not it."
