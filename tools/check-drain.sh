#!/bin/sh
# The drain — task `C-001e3c2`, doc 11 §7.1.5.
#
# `C-001e3c1` drives 64 interleaved allocations and releases and asks that no two
# live blocks ever share an octet. That property is satisfied by an allocator
# that NEVER COALESCES AT ALL: one which always hands back untouched ground can
# never overlap, and it passes every check in `check-stress.sh` while leaking the
# arena away one block at a time.
#
# ## The property
#
# After the interleaving, releasing every block still held must return the arena
# to ONE WHOLE BLOCK: orders 6..11 empty, order 12 holding exactly one block, and
# that block at the arena base. Nothing fails at the moment a merge is missed —
# the failure is a later large request that cannot be met out of memory that is
# entirely free.
#
# The head alone does not say "one block": a list holding one address twice has
# the same head. So the program walks each chain and prints its LENGTH too,
# stopping at 64, and a cyclic list reports 64 rather than 1.
#
# Same program and same trace as `check-stress.sh`; the two read different halves
# of it. Every number below is computed from what the program printed. This file
# states no address of its own — `head[12]` is compared against the base the
# program reported, not against a constant.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    "$root/tools/sadhana-native.sh" \
        --स्थान "$1" "$root/spec/stress.sas" "$tmp/s.elf" >/dev/null
    # SIGKILL, not perl's alarm — QEMU handles SIGALRM (W-060).
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/s.elf" > "$tmp/out" 2>&1 &
    qpid=$!
    ( sleep 30; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    wait "$qpid" 2>/dev/null || true
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    pkill -P "$watcher" 2>/dev/null || true
    kill "$watcher" 2>/dev/null || true
    # CR-stripped: -nographic ends its lines with one, and an anchored pattern
    # silently matches nothing without this. `-a` is the same defect one
    # level down: a single NUL anywhere in the guest's output makes grep
    # decline to look and report zero lines, exit 1, in silence (`W-098`).
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$'
}

# drain_verdict FILE: replays the trace in FILE (16-hex-digit lines). Addresses and headers stay
# strings (lowercase hex, same width, so string order is numeric order); only the small fields
# (order, chain length) are converted. Exact below 2^53, far above any value this program prints.
drain_verdict() {
    LC_ALL=C awk '
    function h2d(s,  i, v) { v = 0; for (i = 1; i <= length(s); i++) v = v * 16 + index("0123456789abcdef", substr(s, i, 1)) - 1; return v }
    function isz(s) { return s ~ /^0*$/ }
    function addbad(m) { bad[++nbad] = m }
    function lst(arr, n,  i, s) { s = "["; for (i = 1; i <= n; i++) s = s (i > 1 ? ", " : "") arr[i]; return s "]" }
    function sortkeys(arr, out,  k, n, i, j, t) {   # numeric keys ascending
        n = 0; for (k in arr) out[++n] = k + 0
        for (i = 2; i <= n; i++) { t = out[i]; for (j = i - 1; j >= 1 && out[j] > t; j--) out[j + 1] = out[j]; out[j + 1] = t }
        return n
    }
    NF { v[++nv] = $1 "" }
    END {
        if (nv == 0) { print "the program printed nothing"; exit 1 }
        base = v[1]; ne = nv - 1
        LOW = 6; HIGH = 12; TOP = 12
        held = -1; nlive = 0; nbad = 0
        if (ne % 2) addbad(ne " words after the base \342\200\224 every event is two words, so the trace is truncated and the tail is the drain")
        for (i = 0; i + 1 < ne; i += 2) {
            hdr = v[2 + i]; p = v[3 + i]
            ks = substr(hdr, 1, 14); k = h2d(substr(hdr, 15, 2)); n = i / 2
            kind = (ks ~ /^0*[1-5]$/) ? substr(ks, 14, 1) + 0 : 0
            if (kind >= 1 && kind <= 3 && (k < LOW || k > HIGH)) {
                addbad("event " n ": header " hdr " names order " k ", outside " LOW ".." HIGH); break }
            if (kind == 1) { if (!isz(p)) { if (!(p in live)) nlive++; live[p] = k } }
            else if (kind == 2) { if (p in live) { delete live[p]; nlive-- } }
            else if (kind == 3) {
                if (held < 0) held = nlive
                if (!(p in live)) addbad("event " n ": the drain released " p " at order " k ", which was not live \342\200\224 the driver is releasing something it does not hold and the final state means nothing")
                else if (live[p] != k) addbad("event " n ": the drain released " p " at order " k ", taken at order " live[p] " \342\200\224 returned to the wrong free list it is either lost or larger than it owns")
                else { delete live[p]; nlive-- }
            }
            else if (kind == 4) head[k] = p
            else if (kind == 5) chain[k] = p
            else { addbad("event " n ": header " hdr " is a kind this check cannot read"); break }
        }
        if (held < 0) addbad("no drain event in the trace at all \342\200\224 the program ran the interleaving and stopped, so nothing here was asked")
        else if (held < 3) addbad("the interleaving left only " held " blocks held, so the drain merged almost nothing \342\200\224 with one block the arena comes back whole even from an allocator that never split it")
        if (nlive > 0) {
            m = sortlive(live, la); s = ""
            for (i = 1; i <= m; i++) s = s (i > 1 ? ", " : "") la[i] "/order " live[la[i]]
            addbad(nlive " blocks were still live after the drain: " s " \342\200\224 the drain did not release what it was holding")
        }
        nh = sortkeys(head, hk); nc = sortkeys(chain, ck)
        ok = (nh == 7 && nc == 7)
        if (ok) for (k = LOW; k <= HIGH; k++) if (!(k in head) || !(k in chain)) ok = 0
        if (!ok) addbad("the trace reports heads for " lst(hk, nh) " and chains for " lst(ck, nc) "; all of " lst2() " are needed to say the arena is whole")
        else {
            for (k = LOW; k < TOP; k++)
                if (!isz(head[k]) || !isz(chain[k]))
                    addbad("order " k " is not empty: head " head[k] ", " h2d(chain[k]) " block(s) on the list. Every one of them is a half whose buddy was free and was not merged with it, so the arena can no longer answer a large request out of memory that is entirely free")
            if (head[TOP] != base) addbad("order " TOP " holds " head[TOP] ", not the arena base " base " \342\200\224 the whole region is one block at this order and there is nowhere else for it to start")
            if (chain[TOP] != "0000000000000001") addbad("order " TOP " has " h2d(chain[TOP]) " blocks on its list, not one. The arena is one block wide at this order, so any other count is the same address twice or a block outside the region")
        }
        if (nbad > 0) { for (i = 1; i <= nbad; i++) print bad[i]; exit 1 }
        print held " blocks still held after the interleaving, all released; orders " LOW ".." TOP - 1 " empty and order " TOP " holding one block at " head[TOP] ", the arena base"
    }
    function lst2(  s, i) { s = "["; for (i = LOW; i <= HIGH; i++) s = s (i > LOW ? ", " : "") i; return s "]" }
    function sortlive(arr, out,  k, n, i, j, t) {
        n = 0; for (k in arr) out[++n] = k
        for (i = 2; i <= n; i++) { t = out[i]; for (j = i - 1; j >= 1 && out[j] > t; j--) out[j + 1] = out[j]; out[j + 1] = t }
        return n
    }' "$1"
}

fail=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/lines"

    if ! drain_verdict "$tmp/lines" > "$tmp/verdict" 2>&1
    then
        sed 's/^/  FAIL  /' "$tmp/verdict"
        fail=1; continue
    fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/verdict")"
done

[ "$fail" -eq 0 ] || { echo; echo "released memory does not come back whole."; exit 1; }
echo
echo "ok  the arena drains back to one block. An allocator that never coalesces"
echo "    passes C-001e3a, C-001e3b and C-001e3c1 — it splits correctly, it"
echo "    merges the one chain those check, and it never overlaps because every"
echo "    request comes off untouched ground — and fails HERE, with the arena"
echo "    entirely free and unable to answer for any of it"
