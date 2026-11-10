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
# ---- portable hex-word verifiers (replace inline python; POSIX sh) ----
# ---- portable hex-word helpers (POSIX sh; 64-bit signed, bit-exact) -------
# h2i HEX16 -> $I : the 64-bit pattern of a 16-digit hex word (two 32-bit
# halves, so no shell has to parse a literal above INT64_MAX; shorter input
# is zero-padded on the left, like int(x, 16)).
h2i() {
    _s=0000000000000000$1; _s=${_s#${_s%????????????????}}
    I=$(( (0x${_s%????????} << 32) | 0x${_s#????????} ))
}
# loadvals PREFIX FILE -> PREFIX1..PREFIXn, PREFIX_n : one word per non-blank line.
loadvals() {
    _p=$1; _i=0
    set -f
    for _t in $(cat "$2"); do
        _i=$((_i + 1)); h2i "$_t"; eval "$_p$_i=\$I"
    done
    set +f
    eval "${_p}_n=\$_i"
}
# add MSG : append one line to the failure list $bad.
add() { if [ -n "$bad" ]; then bad="$bad
$1"; else bad=$1; fi; }
# verify_um LINES BSS : the six-word user-mode verdict; stdout = message, rc 0/1.
verify_um() {
    loadvals w "$1"
    if [ "$w_n" -ne 4 ]; then
        echo "printed $w_n hex lines, expected 4: 1 satp as installed; 2 the word at the shared VA; 3 scause on the ecall; 4 sstatus at the same moment. Fewer than four means the machine stopped where the line stops — a bad root table kills the fetch after the satp write, and a page without U kills the sret"
        return 1
    fi
    satp=$w1 shared=$w2 cause=$w3 sstatus=$w4
    set -f; set -- $(cat "$2"); set +f
    if [ $# -lt 2 ]; then echo "no .bss section in the ELF"; return 1; fi
    h2i "$1"; bss_addr=$I; h2i "$2"; bss_size=$I
    base=$(( (satp & ((1 << 44) - 1)) << 12 ))
    mn=$(( 1 << 63 ))
    bad=""
    nib=$(( (satp >> 60) & 15 ))
    if [ "$nib" -ne 8 ]; then
        add "$(printf 'word 1, satp as installed: mode nibble %x, not 8. Sv39 is the only paging mode this tree installs, and a satp that is not in it was never installed' "$nib")"
    fi
    if [ $(( base % 4096 )) -ne 0 ]; then
        add "$(printf 'word 1: the root table is at %016x, which is not page aligned — satp keeps a page NUMBER and the low bits are lost' "$base")"
    fi
    # bss_addr + bss_size is unbounded in the original: a carry out of bit 63
    # (end < addr, unsigned) means the true sum exceeds any base + 6*4096.
    bss_end=$(( bss_addr + bss_size ))
    if ! { [ $(( (bss_addr ^ mn) <= (base ^ mn) )) -eq 1 ] \
        && { [ $(( (bss_end ^ mn) < (bss_addr ^ mn) )) -eq 1 ] \
          || [ $(( ((base + 6 * 4096) ^ mn) <= (bss_end ^ mn) )) -eq 1 ]; }; }; then
        add "$(printf 'word 1: the root table is at %016x, and the six frames from it do not fit inside the .bss the ELF reserves (%016x + %x)' "$base" "$bss_addr" "$bss_size")"
    fi
    uf=$(( base + 4 * 4096 ))
    if [ "$shared" -eq 0 ]; then
        add "word 3, the word at the shared VA: zero, which is what an unwritten frame reads. Nothing distinguishes 'the window resolved' from 'the window resolved to nothing'"
    elif [ "$shared" -ne "$uf" ]; then
        add "$(printf "word 3, the word at the shared VA: %016x, and the fifth frame from the root table is %016x. The program stores the user's page address in the data frame and reads it back through a window at a virtual address that is not the frame's own, so a word that is not that address did not come through the window" "$shared" "$uf")"
    fi
    if [ "$cause" -ne 8 ]; then
        extra=""
        if [ "$cause" -eq 9 ]; then
            extra=" — 9 is ENVIRONMENT CALL FROM S-MODE, which is every ecall this tree has ever taken and is the one number that says the process never left supervisor mode"
        fi
        add "$(printf 'word 5, scause on the ecall: cause %u, expected 8, ENVIRONMENT CALL FROM U-MODE. The process did not trap from U-mode%s' "$cause" "$extra")"
    fi
    if [ $(( sstatus & 256 )) -ne 0 ]; then
        add "$(printf 'word 6, sstatus at the same moment: SPP is set (%016x), so the trap came from supervisor mode. scause and SPP are demanded together because a kernel that sets a variable can produce either alone' "$sstatus")"
    fi
    if [ -n "$bad" ]; then printf '%s\n' "$bad"; return 1; fi
    printf 'satp %016x, tables at %016x, the window reads %016x, cause %u with SPP clear in %016x\n' \
        "$satp" "$base" "$shared" "$cause" "$sstatus"
}
# verify_um_moved LO HI : the two link addresses must move words 1-2 and fix word 5.
verify_um_moved() {
    loadvals lo "$1"; loadvals hi "$2"
    bad=""
    if [ "$lo1" -eq "$hi1" ]; then
        add "$(printf 'word 1, satp as installed: %016x at both link addresses. It is derived from where the image was loaded, so a value that did not move is a constant in the source and the derivation it stands for never happened' "$lo1")"
    fi
    if [ "$lo2" -eq "$hi2" ]; then
        add "$(printf 'word 2, the word at the shared VA: %016x at both link addresses. It is derived from where the image was loaded, so a value that did not move is a constant in the source and the derivation it stands for never happened' "$lo2")"
    fi
    if [ "$lo3" -ne "$hi3" ]; then
        add "$(printf 'word 5, scause on the ecall: %016x then %016x. A cause number is architectural. One that follows the link address is not coming from scause at all' "$lo3" "$hi3")"
    fi
    if [ -n "$bad" ]; then printf '%s\n' "$bad"; return 1; fi
    echo "satp and the windowed word both moved between the two link addresses and the cause number stayed put"
}

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
    "$root/tools/sadhana-native.sh" \
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

    if ! verify_um "$tmp/l.$addr" "$tmp/bss.$addr" > "$tmp/v" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
done

# ---- what moved, and what had better not --------------------------------
# Only reached when both runs are internally well formed; otherwise the words
# below are not the words this compares.
if [ "$fail" -eq 0 ]; then
    if ! verify_um_moved "$tmp/l.०षोड्८०२०००००" "$tmp/l.०षोड्८०४०००००" > "$tmp/m" 2>&1
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
