#!/bin/sh
# A context switch restores ALL twelve — `C-002d`, doc 11 §3.1, phase 11.2.4.
#
# `तन्तुः` holds an execution context, and the whole claim of a switch is one
# sentence: every register belonging to the caller comes back. All of them —
# twelve, not eleven.
#
# ## Why the check has the shape it has
#
# A switch that saves eleven of twelve PASSES every test that does not look at
# the twelfth. And the twelfth only fails when another thread happens to be
# holding something in it — which is to say occasionally, much later, and in
# somebody else's code.
#
# So each register carries a DIFFERENT value and every one is printed. Filling
# them all with one value would be as useless as filling none: an off-by-one in
# the save order would not show, and an off-by-one is the easiest defect to
# write here.
#
# ## Two contexts, because one cannot show a leak
#
# Saving and restoring a single context does not reveal whether two threads
# bleed into each other. So two are filled — pattern A = 0xa00+j, pattern B =
# 0xb00+j — and both are restored in turn. If the switch drops a register, the
# restored A shows B's value in that slot, and that is the leak.
#
# In hexadecimal here the digits 10..15 are the VOWELS अ आ इ ई उ ऊ in alphabet
# order, so the leading digit of each value says which pattern it belongs to.
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
# verify_ctx LINES : 24 words, contexts A (0xa00+j) and B (0xb00+j); rc 0/1.
verify_ctx() {
    loadvals c "$1"
    bad=""
    for cn in A B; do
        if [ "$cn" = A ]; then off=0; base=2560; other=2816; else off=12; base=2816; other=2560; fi
        j=0
        while [ "$j" -lt 12 ]; do
            eval "g=\$c$((off + j + 1))"
            want=$((base + j)); d=$((g - other))
            if [ "$g" -eq "$want" ]; then
                :
            elif [ "$g" -eq $((other + j)) ]; then
                add "$(printf "context %s, register %s: holds 0x%x, which is the OTHER context's value for that slot. The switch did not save or restore register %s, so the two threads share it — a leak that only shows when something is actually living there" "$cn" "$j" "$g" "$j")"
            elif [ "$d" -ge 0 ] && [ "$d" -lt 12 ]; then
                add "$(printf "context %s, register %s: holds 0x%x — another context's value from a DIFFERENT slot, so the save order is shifted" "$cn" "$j" "$g")"
            else
                add "$(printf 'context %s, register %s: 0x%x, expected 0x%x' "$cn" "$j" "$g" "$want")"
            fi
            j=$((j + 1))
        done
    done
    dup=0; a=1
    while [ "$a" -le 24 ]; do
        b=$((a + 1))
        while [ "$b" -le $(( a <= 12 ? 12 : 24 )) ]; do
            eval "x=\$c$a y=\$c$b"
            [ "$x" -ne "$y" ] || dup=1
            b=$((b + 1))
        done
        a=$((a + 1))
    done
    if [ "$dup" -eq 1 ]; then
        add "some registers hold identical values, so an off-by-one in the save order could not be seen — the patterns must stay distinct per register"
    fi
    if [ -n "$bad" ]; then printf '%s\n' "$bad"; return 1; fi
    printf 'context A restored 0x%x..0x%x and B restored 0x%x..0x%x; all twelve distinct in each, and neither context holds a value from the other\n' \
        "$c1" "$c12" "$c13" "$c24"
}

tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    "$root/tools/sadhana-native.sh" \
        --स्थान "$1" "$root/spec/context.sas" "$tmp/c.elf" >/dev/null
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/c.elf" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 20; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    # No `tail -N` — it returns min(actual, N), which makes the count assertion
    # below one-sided by construction and blind to a surplus. See W-081.
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$'
}

fail=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    n=$(wc -l < "$tmp/l" | tr -d ' ')
    if [ "$n" -ne 24 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 24 (twelve registers"
        echo "        restored from context A, then twelve from context B)"
        fail=1; continue
    fi
    if ! verify_ctx "$tmp/l" > "$tmp/v" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
done
[ "$fail" -eq 0 ] || { echo; echo "the context switch is wrong."; exit 1; }
echo
echo "ok  every one of the twelve callee-saved registers survives a switch in"
echo "    both directions, and no register carries a value from the other"
echo "    context — which is what a switch saving eleven of twelve would show"
