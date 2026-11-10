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
    "$root/tools/sadhana-native.sh" \
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

    if ! LC_ALL=C awk '
function lz(s) { sub(/^0+/, "", s); return (s == "") ? "0" : s }
function pad(s) { while (length(s) < 16) s = "0" s; return s }
function hv(c) { return index("0123456789abcdef", c) - 1 }
function hadd(a, b,  i, la, lb, n, da, db, s, c, r) {
    la = length(a); lb = length(b); n = (la > lb) ? la : lb; c = 0; r = ""
    for (i = 0; i < n; i++) {
        da = (i < la) ? hv(substr(a, la - i, 1)) : 0
        db = (i < lb) ? hv(substr(b, lb - i, 1)) : 0
        s = da + db + c; c = (s >= 16) ? 1 : 0; if (s >= 16) s -= 16
        r = substr("0123456789abcdef", s + 1, 1) r
    }
    if (c) r = "1" r
    return r
}
function hcmp(a, b) { a = lz(a); b = lz(b); if (length(a) != length(b)) return (length(a) < length(b)) ? -1 : 1; return (a < b) ? -1 : (a > b) ? 1 : 0 }
function norm(s) { s = tolower(s); sub(/^0x/, "", s); return s }
function bad_(m) { bad[++nb] = m }
FILENAME == ARGV[1] { if ($0 ~ /[^ \t]/) v[++nv] = $0; next }
{ for (i = 1; i <= NF; i++) bss[++nbss] = $i }
END {
    NAME[1] = "satp for A"; NAME[2] = "the shared VA under A"; NAME[3] = "scause on A\047s ecall"
    NAME[4] = "satp for B"; NAME[5] = "the shared VA under B"; NAME[6] = "scause on B\047s ecall"
    if (nv != 6) {
        s = ""
        for (i = 1; i <= 6; i++) s = s (i > 1 ? "; " : "") i " " NAME[i]
        printf "printed %d hex lines, expected 6: %s. Fewer than six means the machine stopped where the line stops — a root table that does not map the kernel kills the fetch after its own satp write, and a window table reached through the wrong megapage entry kills the load that follows\n", nv, s
        exit 1
    }
    satp_a = v[1]; shared_a = v[2]; cause_a = v[3]; satp_b = v[4]; shared_b = v[5]; cause_b = v[6]
    if (nbss < 2) { print "bss file malformed"; exit 1 }
    bss_addr = lz(norm(bss[1])); bss_size = lz(norm(bss[2]))
    ppn_a = lz(substr(satp_a, 6)); ppn_b = lz(substr(satp_b, 6))
    base = lz(substr(satp_a, 6) "000")
    for (k = 1; k <= 2; k++) {
        i = (k == 1) ? 1 : 4; s = (k == 1) ? satp_a : satp_b
        if (substr(s, 1, 1) != "8")
            bad_(sprintf("word %d, %s: mode nibble %s, not 8. Sv39 is the only paging mode this tree installs, and a satp that is not in it was never installed", i, NAME[i], substr(s, 1, 1)))
    }
    if (satp_a == satp_b)
        bad_(sprintf("words 1 and 4 are the same satp (%s), so both processes ran in ONE address space and there is no second root table to have observed anything about", satp_a))
    else if (ppn_b != hadd(ppn_a, "1"))
        bad_(sprintf("word 4, %s: page number %s, and the declared layout puts B\047s root in the frame after A\047s, %s. The satp that was installed is not the table the source says was built", NAME[4], ppn_b, hadd(ppn_a, "1")))
    # base is a page number shifted by 12, so its low 12 bits are zero by construction
    if (!(hcmp(bss_addr, base) <= 0 && hcmp(hadd(base, "a000"), hadd(bss_addr, bss_size)) <= 0))
        bad_(sprintf("word 1: the frames start at %s, and the ten of them do not fit inside the .bss the ELF reserves (%s + %s)", pad(base), pad(bss_addr), bss_size))
    if (shared_a == shared_b)
        bad_(sprintf("words 2 and 5: the shared virtual address answers %s under BOTH satps. Two root tables describing an identical map are two objects, not two address spaces — the separation has to be observed at an address, not inferred from a pointer. Check that each megapage table sends megapage 500 to its OWN window table", shared_a))
    for (k = 1; k <= 2; k++) {
        i = (k == 1) ? 2 : 5; word = (k == 1) ? shared_a : shared_b
        want = pad(hadd(base, (k == 1) ? "8000" : "9000")); who = (k == 1) ? "A" : "B"
        if (lz(word) == "0")
            bad_(sprintf("word %d, %s: zero, which is what an unwritten frame reads. Nothing distinguishes \047the window resolved\047 from \047the window resolved to nothing\047", i, NAME[i]))
        else if (word != want)
            bad_(sprintf("word %d, %s: %s, and %s\047s data frame is %s by the declared layout. Each process stores its own frame\047s address in that frame and reads it back through a window at a virtual address that is not the frame\047s own, so a word that is not that address did not come through %s\047s window", i, NAME[i], word, who, want, who))
    }
    for (k = 1; k <= 2; k++) {
        i = (k == 1) ? 3 : 6; c = (k == 1) ? cause_a : cause_b; who = (k == 1) ? "A" : "B"
        extra = (lz(c) == "9") ? " — 9 is ENVIRONMENT CALL FROM S-MODE, which is every ecall this tree took before C-002k and is the one number that says the process never left supervisor mode" : ""
        if (lz(c) != "8") {
            cd = lz(c); n = 0
            for (j = 1; j <= length(cd); j++) n = n * 16 + hv(substr(cd, j, 1))
            bad_(sprintf("word %d, %s: cause %s, expected 8, ENVIRONMENT CALL FROM U-MODE. Process %s did not trap from U-mode%s", i, NAME[i], dec(cd), who, extra))
        }
    }
    if (nb) { for (i = 1; i <= nb; i++) print bad[i]; exit 1 }
    printf "satp %s/%s, tables at %s, one VA reading %s then %s, both ecalls cause 8\n", satp_a, satp_b, pad(base), shared_a, shared_b
}
function dec(h,  i, j, d, n, c, carry, s, t) {
    n = 1; d[1] = 0
    for (i = 1; i <= length(h); i++) {
        c = hv(substr(h, i, 1)); carry = c
        for (j = 1; j <= n; j++) { t = d[j] * 16 + carry; d[j] = t % 10; carry = int(t / 10) }
        while (carry > 0) { d[++n] = carry % 10; carry = int(carry / 10) }
    }
    s = ""; for (j = n; j >= 1; j--) s = s d[j]
    return s
}
' "$tmp/l.$addr" "$tmp/bss.$addr" > "$tmp/v" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
done

# ---- what moved, and what had better not --------------------------------
# Only reached when both runs are internally well formed; otherwise the words
# below are not the words this compares.
if [ "$fail" -eq 0 ]; then
    if ! LC_ALL=C awk '
FILENAME == ARGV[1] { if ($0 ~ /[^ \t]/) lo[++nl] = $0; next }
{ if ($0 ~ /[^ \t]/) hi[++nh] = $0 }
END {
    NAME[1] = "satp for A"; NAME[2] = "the shared VA under A"; NAME[3] = "scause on A\047s ecall"
    NAME[4] = "satp for B"; NAME[5] = "the shared VA under B"; NAME[6] = "scause on B\047s ecall"
    MOVES[1] = 0; MOVES[2] = 1; MOVES[3] = 3; MOVES[4] = 4; FIXED[1] = 2; FIXED[2] = 5
    for (k = 1; k <= 4; k++) { i = MOVES[k]
        if (lo[i + 1] == hi[i + 1])
            bad[++nb] = sprintf("word %d, %s: %s at both link addresses. It is derived from where the image was loaded, so a value that did not move is a constant in the source and the derivation it stands for never happened", i + 1, NAME[i + 1], lo[i + 1]) }
    for (k = 1; k <= 2; k++) { i = FIXED[k]
        if (lo[i + 1] != hi[i + 1])
            bad[++nb] = sprintf("word %d, %s: %s then %s. A cause number is architectural. One that follows the link address is not coming from scause at all", i + 1, NAME[i + 1], lo[i + 1], hi[i + 1]) }
    if (nb) { for (i = 1; i <= nb; i++) print bad[i]; exit 1 }
    print "both satps and both windowed words moved between the two link addresses and both cause numbers stayed put"
}
' "$tmp/l.०षोड्८०२०००००" "$tmp/l.०षोड्८०४०००००" > "$tmp/m" 2>&1
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
