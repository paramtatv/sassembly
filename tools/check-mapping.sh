#!/bin/sh
# W^X, and a refusal that says which rule it broke — `C-002c`, doc 11 §3.3.
#
# A frame is mapped into a `स्मृतिक्षेत्रम्` and its rights are fixed at that
# moment. The heaviest rule is W^X: NO PAGE IS EVER WRITABLE AND EXECUTABLE.
# That is what makes doc 09's D-09-C (no JIT) coherent, and what forces doc 16's
# Tier 2 patcher to remap transiently — a rare, audited operation.
#
# ## Two refusals, and they are not the same refusal
#
#   1  W^X          writable and executable together — OUR rule
#   2  W without R  reserved by RISC-V itself — THE MACHINE's rule
#
# Returning 0 for both would repeat a mistake made earlier the same day: two
# mutations at milestone M-K1 were caught correctly and explained wrongly, and a
# wrong explanation sends the next reader to the wrong place. So the answer
# carries a reason: 0 granted, 1 refused by W^X, 2 refused as reserved.
#
# The distinction is not academic. A W^X refusal means the CALLER asked for
# something the system forbids; a reserved-encoding refusal means the caller
# asked for something the HARDWARE will fault on — and a reserved PTE does not
# fault when written, it faults when someone eventually touches the page.
#
# Sv39 bits: V=1 R=2 W=4 X=8 U=16 G=32 A=64 D=128, PPN shifted left by 10.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
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
# map_why N : the reason name for 0/1/2, '?' otherwise.
map_why() { case $1 in 0) echo granted ;; 1) echo "refused by W^X" ;; 2) echo "refused as reserved" ;; *) echo '?' ;; esac; }
# verify_map LINES : page + five (reason, entry) pairs; rc 0/1.
verify_map() {
    loadvals m "$1"
    page=$m1
    bad=""
    if [ $(( page % 4096 )) -ne 0 ]; then
        add "$(printf 'the frame %016x is not 4096-aligned' "$page")"
    fi
    ppn=$(( (page >> 12) & ((1 << 52) - 1) ))
    i=0
    for spec in "2 0" "6 0" "10 0" "14 1" "4 2"; do
        set -- $spec; a=$1; wr=$2
        eval "reason=\$m$((2 + 2 * i)) pte=\$m$((3 + 2 * i))"
        if [ "$reason" -ne "$wr" ]; then
            if [ "$wr" -eq 1 ] && [ "$reason" -eq 0 ]; then
                add "$(printf 'request %s asked for R|W|X and was GRANTED as 0x%x. W^X is not enforced, so a page can be written and then executed — which is the whole shape of attack the rule exists to remove' "$i" "$pte")"
            elif [ "$wr" -eq 2 ] && [ "$reason" -eq 0 ]; then
                add "$(printf 'request %s asked for W without R and was GRANTED as 0x%x. That encoding is RESERVED in RISC-V: the fault does not arrive when the entry is written, it arrives when someone touches the page' "$i" "$pte")"
            elif [ "$wr" -eq 0 ] && [ "$reason" -ne 0 ]; then
                add "$(printf 'request %s asked for 0x%x, which breaks no rule, and was refused with reason %u (%s)' "$i" "$a" "$reason" "$(map_why "$reason")")"
            else
                add "$(printf 'request %s: reason %u (%s), expected %s (%s)' "$i" "$reason" "$(map_why "$reason")" "$wr" "$(map_why "$wr")")"
            fi
        elif [ "$reason" -ne 0 ]; then
            if [ "$pte" -ne 0 ]; then
                add "$(printf 'request %s was refused but returned entry 0x%x — a refusal must not hand back a usable mapping' "$i" "$pte")"
            fi
        else
            wp=$(( (ppn << 10) | a | 1 | 64 ))
            if [ "$pte" -ne "$wp" ]; then
                add "$(printf 'request %s: entry 0x%x, expected 0x%x — the frame or the rights are not what was asked for' "$i" "$pte" "$wp")"
            fi
        fi
        i=$((i + 1))
    done
    if [ "$m8" -eq "$m10" ]; then
        add "$(printf 'the W^X refusal and the reserved-encoding refusal both report %u — the caller cannot tell a rule it broke from a rule the machine imposes, and the two need different fixes' "$m8")"
    fi
    if [ -n "$bad" ]; then printf '%s\n' "$bad"; return 1; fi
    printf 'frame %016x: R, R|W and R|X mapped with correct entries; R|W|X refused by W^X (1) and W-without-R refused as reserved (2), and the two are distinguishable\n' "$page"
}

tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    "$root/tools/sadhana-native.sh" \
        --स्थान "$1" "$root/spec/mapping.sas" "$tmp/m.elf" >/dev/null
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/m.elf" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 20; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -11
}

fail=0; prev=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    n=$(wc -l < "$tmp/l" | tr -d ' ')
    if [ "$n" -ne 11 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 11 (the page, then a"
        echo "        reason and an entry for each of five requests)"
        fail=1; continue
    fi
    if ! verify_map "$tmp/l" > "$tmp/v" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
    b=$(sed -n 1p "$tmp/l")
    [ -n "$prev" ] && [ "$b" = "$prev" ] && { echo "  FAIL  frame $b at both link addresses — a constant"; fail=1; }
    prev=$b
done
[ "$fail" -eq 0 ] || { echo; echo "mapping is wrong."; exit 1; }
echo
echo "ok  no page is both writable and executable; a reserved encoding is refused"
echo "    separately from a W^X violation, so the caller learns which rule it hit"
