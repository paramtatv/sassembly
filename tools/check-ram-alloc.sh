#!/bin/sh
# The allocator runs on the machine's own RAM — task `C-001f2b`, doc 11 §7.
#
# `C-001e3c` exercised the allocator on a `ॱरिक्त` block written into the link.
# `C-001f2a` derived an arena from `/memory`'s `reg`. This is both at once: the
# same interleaving and the same drain, over the memory the machine reported.
#
# EIGHT LINES CHANGED — the ones that made the arena. Everything else is
# `spec/stress.sas` unaltered, and that is the whole claim: if the allocator was
# right, it stays right on real RAM; if the placement were wrong, it would write
# over the kernel and say nothing.
#
# What this asks that neither predecessor could:
#
#   * the arena is the FDT-derived one — aligned, and above the image, so the
#     blocks handed out are not the octets being executed;
#   * every address is inside that arena;
#   * after the drain, orders 6..11 are empty and order 12 holds ONE block at
#     the arena base — the arena is whole again, in real memory.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    "$root/tools/sadhana-native.sh" \
        --स्थान "$1" "$root/spec/stress-ram.sas" "$tmp/r.elf" >/dev/null
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/r.elf" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 25; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$'
}

fail=0; prev=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/lines"
    if ! LC_ALL=C awk '
function hx(s,  i, v) { v = 0; for (i = 1; i <= length(s); i++) v = v * 16 + index("0123456789abcdef", substr(s, i, 1)) - 1; return v }
function dec(h,  i, j, c, n, r, carry, s, D) {
    n = 1; D[1] = 0
    for (i = 1; i <= length(h); i++) {
        carry = index("0123456789abcdef", substr(h, i, 1)) - 1
        for (j = 1; j <= n; j++) { r = D[j] * 16 + carry; D[j] = r % 10; carry = int(r / 10) }
        while (carry > 0) { n++; D[n] = carry % 10; carry = int(carry / 10) }
    }
    s = ""; for (j = n; j >= 1; j--) s = s D[j]
    return s
}
function nz(h) { return h != "0000000000000000" }
function hd(h) { return (h in HEAD) ? HEAD[h] : "0000000000000000" }
{ if ($0 ~ /[^ \t\r\n]/) v[++nv] = $0 }
END {
    if (nv < 3) { print "the program printed almost nothing — it did not run"; exit 1 }
    arena = v[1]; nr = nv - 1
    if (nr % 2) print nr " words after the arena base — events come in (descriptor, value) PAIRS, so an odd count means a line was lost"
    nb = 0; nlive = 0; anyhead = 0
    for (i = 0; i + 1 < nr; i += 2) {
        d = v[2 + i]; val = v[3 + i]
        ord = hx(substr(d, 15, 2)); kind = substr(d, 1, 14)
        if (kind == "00000000000004") { H[ord] = val; anyhead = 1 }
        else if (kind == "00000000000005") C[ord] = val
        else if (kind == "00000000000001" && nz(val)) live[++nlive] = val
    }
    ah = hx(substr(arena, 1, 8)); al = hx(substr(arena, 9, 8))
    if (substr(arena, 14, 3) != "000") bad[++nb] = "the arena " arena " is not 4096-aligned"
    if (arena < "0000000080000000") bad[++nb] = "the arena " arena " is below this machine\047s RAM"
    if (!anyhead) bad[++nb] = "no free-list state was printed, so the drain cannot be read"
    for (k = 6; k < 12; k++) {
        hk = (k in H) ? H[k] : "0000000000000000"; ck = (k in C) ? C[k] : "0000000000000000"
        if (nz(hk) || nz(ck))
            bad[++nb] = "order " k " is not empty after the drain: head " hk ", " dec(ck) " block(s) — the arena did not come back whole"
    }
    h12 = (12 in H) ? H[12] : "0000000000000000"; c12 = (12 in C) ? C[12] : "0000000000000000"
    if (!(12 in H) || H[12] != arena || !(12 in C) || C[12] != "0000000000000001")
        bad[++nb] = "order 12 holds head " h12 " count " dec(c12) ", expected one block at the arena base " arena
    eh = ah; el = al + 4096; if (el >= 4294967296) { el -= 4294967296; eh++ }
    nout = 0
    for (i = 1; i <= nlive; i++) {
        x = live[i]; xh = hx(substr(x, 1, 8)); xl = hx(substr(x, 9, 8))
        inside = (xh > ah || (xh == ah && xl >= al)) && (xh < eh || (xh == eh && xl < el))
        if (!inside) { nout++; if (nout == 1) first = x }
    }
    endhex = sprintf("%08x%08x", eh, el)
    if (nout) bad[++nb] = nout " allocation(s) fell outside the arena " arena ".." endhex ", e.g. " first " — the allocator handed out memory it was never given"
    if (nb) { for (i = 1; i <= nb; i++) print bad[i]; exit 1 }
    print "arena " arena " from the device tree, " nlive " allocations all inside it, orders 6..11 empty and order 12 whole after the drain"
}
' "$tmp/lines" > "$tmp/verdict" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/verdict"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/verdict")"
    a=$(sed -n 1p "$tmp/lines")
    [ -n "$prev" ] && [ "$a" = "$prev" ] && { echo "  FAIL  arena $a at both link addresses — a constant"; fail=1; }
    prev=$a
done
[ "$fail" -eq 0 ] || { echo; echo "the allocator on real RAM is wrong."; exit 1; }
echo
echo "ok  the allocator runs on the RAM /memory reported — every block inside the"
echo "    device-tree-derived arena, and the arena whole again after the drain"
