#!/bin/bash
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"  # the verdict function below uses bash arithmetic and arrays
# Capability derivation attenuates, never amplifies — task `C-002a`, doc 11 §3.1.
#
# `अधिकारः` is an unforgeable reference to an object plus rights, and doc 11
# §3.1 says it is THE ONLY WAY TO NAME ANYTHING. The whole model rests on one
# one-way property: a derived capability may drop rights and may never gain one
# its parent lacked. If a capability could exceed its parent, the system is a
# suggestion.
#
# ## Refusal must be distinguishable from an empty grant
#
# The first version of this program returned rights alone, so "asked for nothing
# and got it" and "asked and was refused" were both 0 — the same silence this
# file argues against. A difference the caller cannot read is not a difference.
# The answer is now two words: a verdict, then the rights.
#
#   parent 7             read|write|exec, NO grant
#   (1, 3) (1, 1) (1, 0) three attenuations, the last granting nothing
#   (0, 0) (0, 0)        two amplifications, both REFUSED
#
# The third attenuation is the one that earns the pair: it is granted and empty,
# and reads (1, 0) where a refusal reads (0, 0).
#
# Partial grants are refused whole. Asking for read|write|grant when the parent
# has read|write is not "here is read|write" — that is the silent masking the
# model forbids, and the caller would go on believing it holds grant.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    "$root/tools/sadhana-native.sh" \
        --स्थान "$1" "$root/spec/capability.sas" "$tmp/c.elf" >/dev/null
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
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -11
}

# bin N: unsigned binary of a 64-bit word, no leading zeros ("0" for zero).
bin() {
    local x=$1 i s=""
    for ((i = 63; i >= 0; i--)); do
        if [ -n "$s" ] || [ $(((x >> i) & 1)) -eq 1 ]; then s="$s$(((x >> i) & 1))"; fi
    done
    printf '%s' "${s:-0}"
}
# verdict_capability FILE: 11 hex lines (parent rights, then verdict+rights for five attempts); diagnostics on stdout, rc 1 on failure.
verdict_capability() {
    local v=() l i=0 parent gv gr wv wr a bad=() READ=1 WRITE=2 EXEC=4 GRANT=8 s
    local want_v=(1 1 1 0 0) want_r=(3 1 0 0 0) asked
    while IFS= read -r l || [ -n "$l" ]; do
        [ -n "${l//[[:space:]]/}" ] || continue
        v[i]=$((16#${l//[[:space:]]/})); i=$((i+1))
    done < "$1"
    asked=(3 1 0 $GRANT $((READ | WRITE | GRANT)))
    parent=${v[0]}
    if [ "$parent" -ne $((READ | WRITE | EXEC)) ]; then
        bad+=("the parent holds $(bin "$parent"), expected 111 — read, write, exec and NO grant")
    fi
    for i in 0 1 2 3 4; do
        gv=${v[$((1 + 2*i))]}; gr=${v[$((2 + 2*i))]}; wv=${want_v[i]}; wr=${want_r[i]}; a=${asked[i]}
        if [ "$gv" -ne "$wv" ] || [ "$gr" -ne "$wr" ]; then
            if [ "$wv" -eq 0 ] && [ "$gv" -eq 1 ]; then
                bad+=("attempt $i asked for $(bin "$a"), which the parent ($(bin "$parent")) does not hold, and it was GRANTED as $(bin "$gr"). A capability exceeded its parent, so the model is a suggestion")
            elif [ "$wv" -eq 0 ] && [ "$gr" -ne 0 ]; then
                bad+=("attempt $i was refused but returned rights $(bin "$gr") — a refusal must carry nothing")
            elif [ "$wv" -eq 1 ] && [ "$gv" -eq 0 ]; then
                bad+=("attempt $i asked for $(bin "$a"), entirely within the parent, and was REFUSED — attenuation must be allowed or nothing can be delegated")
            else
                bad+=("attempt $i asked $(bin "$a"): got verdict $(printf %u "$gv") rights $(bin "$gr"), expected verdict $wv rights $(bin "$wr")")
            fi
        fi
    done
    local ge="($(printf %u "${v[5]}"), $(printf %u "${v[6]}"))" rf="($(printf %u "${v[7]}"), $(printf %u "${v[8]}"))"
    if [ "${v[5]}" -eq "${v[7]}" ] && [ "${v[6]}" -eq "${v[8]}" ]; then
        bad+=("a granted-but-empty derivation reads $ge and a REFUSAL reads $rf — identical, so the caller cannot tell being given nothing from being turned down, which is the ambiguity the two-word answer exists to remove")
    fi
    if [ "${#bad[@]}" -gt 0 ]; then printf '%s\n' "${bad[@]}"; return 1; fi
    printf 'parent %s; attenuations [%s, %s, %s] all granted, amplifications both refused, and granted-empty %s differs from refused %s\n' \
        "$(bin "$parent")" "$(printf %u "${v[2]}")" "$(printf %u "${v[4]}")" "$(printf %u "${v[6]}")" "$ge" "$rf"
}

fail=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    n=$(wc -l < "$tmp/l" | tr -d ' ')
    if [ "$n" -ne 11 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 11 (the parent's rights,"
        echo "        then a verdict and a rights word for each of five attempts)"
        fail=1; continue
    fi
    if ! verdict_capability "$tmp/l" > "$tmp/v" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
done
[ "$fail" -eq 0 ] || { echo; echo "capability derivation is wrong."; exit 1; }
echo
echo "ok  a derived capability may drop rights and never gains one its parent"
echo "    lacked; an amplification is REFUSED whole rather than silently masked,"
echo "    and a refusal is distinguishable from a grant of nothing"
