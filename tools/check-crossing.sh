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
# Verdict functions (formerly python3 heredocs). Words are the 16-digit lowercase hex lines the
# run printed; compared as strings, so no 64-bit overflow. Only the .bss address/size go through $(( )).
cr_name() {
    case $1 in
        1) printf '%s' "satp for A";; 2) printf '%s' "scause on A's ecall";; 3) printf '%s' "the word A sent";;
        4) printf '%s' "satp for B";; 5) printf '%s' "scause on B's ecall";; 6) printf '%s' "the word B sent back";;
    esac
}
cr_dec() {  # lowercase hex word -> unsigned decimal, as python's {c}
    printf '%s\n' "$1" | LC_ALL=C awk '{
        n = 0
        for (i = 1; i <= length($0); i++) {
            c = index("0123456789abcdef", substr($0, i, 1)) - 1
            for (j = 1; j <= n; j++) { x = d[j] * 16 + c; d[j] = x % 10; c = int(x / 10) }
            while (c > 0) { n++; d[n] = c % 10; c = int(c / 10) }
        }
        if (n == 0) { print 0; exit }
        s = ""; for (j = n; j >= 1; j--) s = s d[j]; print s }'
}
cr_word() { sed -n "${2}p" "$1"; }
cr_bad() { cr_b="$cr_b$1
"; }
crossing_verdict() {  # LINES BSS -> message on stdout, rc 0/1
    cr_f=$1; cr_n=$(awk 'NF { k++ } END { print k + 0 }' "$cr_f")
    if [ "$cr_n" -ne 6 ]; then
        printf '%s\n' "printed $cr_n hex lines, expected 6: 1 satp for A; 2 scause on A's ecall; 3 the word A sent; 4 satp for B; 5 scause on B's ecall; 6 the word B sent back. Fewer than six means the machine stopped where the line stops — a window page without U kills the user's load before its ecall, and a root table that does not map the kernel kills the fetch right after its own satp write"
        return 1
    fi
    cr_sa=$(cr_word "$cr_f" 1); cr_ca=$(cr_word "$cr_f" 2); cr_msg=$(cr_word "$cr_f" 3)
    cr_sb=$(cr_word "$cr_f" 4); cr_cb=$(cr_word "$cr_f" 5); cr_back=$(cr_word "$cr_f" 6)
    set -- $(cat "$2")
    if [ $# -lt 2 ]; then echo "bss file malformed"; return 1; fi
    cr_baddr=$((0x$1)); cr_bsize=$((0x$2))
    cr_ppna=$((0x${cr_sa#?????})); cr_ppnb=$((0x${cr_sb#?????}))
    cr_base=$((cr_ppna * 4096)); cr_basex=$(printf '%016x' "$cr_base")
    cr_b=""
    for cr_i in 1 4; do
        if [ "$cr_i" = 1 ]; then cr_s=$cr_sa; else cr_s=$cr_sb; fi
        cr_nib=$(printf '%s' "$cr_s" | cut -c1)
        if [ "$cr_nib" != 8 ]; then
            cr_bad "word $cr_i, $(cr_name $cr_i): mode nibble $cr_nib, not 8. Sv39 is the only paging mode this tree installs, and a satp that is not in it was never installed"
        fi
    done
    if [ "$cr_sa" = "$cr_sb" ]; then
        cr_bad "words 1 and 4 are the same satp ($cr_sa), so both processes ran in ONE address space and nothing crossed a boundary because there was no boundary"
    elif [ "$cr_ppnb" -ne $((cr_ppna + 1)) ]; then
        cr_bad "word 4, $(cr_name 4): page number $(printf '%x' "$cr_ppnb"), and the declared layout puts B's root in the frame after A's, $(printf '%x' $((cr_ppna + 1))). The satp that was installed is not the table the source says was built"
    fi
    if [ $((cr_base % 4096)) -ne 0 ]; then
        cr_bad "word 1: A's root table is at $cr_basex, which is not page aligned — satp keeps a page NUMBER and the low bits are lost"
    fi
    if ! { [ "$cr_baddr" -le "$cr_base" ] && [ $((cr_base + 10 * 4096)) -le $((cr_baddr + cr_bsize)) ]; }; then
        cr_bad "word 1: the frames start at $cr_basex, and the ten of them do not fit inside the .bss the ELF reserves ($(printf '%016x' "$cr_baddr") + $(printf '%x' "$cr_bsize"))"
    fi
    for cr_i in 2 5; do
        if [ "$cr_i" = 2 ]; then cr_c=$cr_ca; cr_who=A; else cr_c=$cr_cb; cr_who=B; fi
        cr_d=$(cr_dec "$cr_c"); cr_x=""
        if [ "$cr_d" = 9 ]; then cr_x=" — 9 is ENVIRONMENT CALL FROM S-MODE, which is every ecall this tree took before C-002k and is the one number that says the process never left supervisor mode"; fi
        if [ "$cr_d" != 8 ]; then
            cr_bad "word $cr_i, $(cr_name $cr_i): cause $cr_d, expected 8, ENVIRONMENT CALL FROM U-MODE. Process $cr_who did not trap from U-mode$cr_x"
        fi
    done
    cr_inbox=$(printf '%016x' $((cr_base + 8 * 4096)))
    if [ "$cr_msg" = 0000000000000000 ]; then
        cr_bad "word 3, the word A sent: zero, which is what an unwritten frame reads. Nothing distinguishes 'A sent 0' from 'A's load never reached its inbox'"
    elif [ "$cr_msg" != "$cr_inbox" ]; then
        cr_bad "word 3, $(cr_name 3): $cr_msg, and A's inbox frame is $cr_inbox by the declared layout. That address is what the program writes into that frame before paging is on, so a word that is not it did not come through A's window"
    fi
    if [ "$cr_back" = 0000000000000000 ]; then
        cr_bad "word 6, the word B sent back: zero, and zero is exactly what A's inbox is emptied to before B runs. B read a frame A had just been cleared out of — its window is reaching A's inbox, not its own, so the two window tables are one and nothing crossed"
    elif [ "$cr_back" != "$cr_msg" ]; then
        cr_bad "word 6, $(cr_name 6): $cr_back, and the word A sent was $cr_msg. B loaded from the same virtual address under its own satp, so a different word means what the kernel wrote into B's inbox is not what B found there"
    fi
    if [ -n "$cr_b" ]; then printf '%s' "$cr_b"; return 1; fi
    printf '%s\n' "satp $cr_sa/$cr_sb, tables at $cr_basex, $cr_msg out of A's inbox and $cr_back out of B's, both ecalls cause 8"
}
crossing_moves() {  # LOFILE HIFILE
    cm_b=""
    for cm_i in 0 2 3 5; do
        cm_l=$(cr_word "$1" $((cm_i + 1))); cm_h=$(cr_word "$2" $((cm_i + 1)))
        if [ "$cm_l" = "$cm_h" ]; then
            cm_b="${cm_b}word $((cm_i + 1)), $(cr_name $((cm_i + 1))): $cm_l at both link addresses. It is derived from where the image was loaded, so a value that did not move is a constant in the source and the derivation it stands for never happened
"
        fi
    done
    for cm_i in 1 4; do
        cm_l=$(cr_word "$1" $((cm_i + 1))); cm_h=$(cr_word "$2" $((cm_i + 1)))
        if [ "$cm_l" != "$cm_h" ]; then
            cm_b="${cm_b}word $((cm_i + 1)), $(cr_name $((cm_i + 1))): $cm_l then $cm_h. A cause number is architectural. One that follows the link address is not coming from scause at all
"
        fi
    done
    if [ -n "$cm_b" ]; then printf '%s' "$cm_b"; return 1; fi
    echo "both satps and the word on both sides of the crossing moved between the two link addresses, and both cause numbers stayed put"
}

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
    "$root/tools/sadhana-native.sh" \
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

    if ! crossing_verdict "$tmp/l.$addr" "$tmp/bss.$addr" > "$tmp/v" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
done

# ---- what moved, and what had better not --------------------------------
# Only reached when both runs are internally well formed; otherwise the words
# below are not the words this compares.
if [ "$fail" -eq 0 ]; then
    if ! crossing_moves "$tmp/l.०षोड्८०२०००००" "$tmp/l.०षोड्८०४०००००" > "$tmp/m" 2>&1
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
