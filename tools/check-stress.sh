#!/bin/sh
# Interleaved allocation and release — task `C-001e3c1`, doc 11 §7.1.5.
#
# `C-001e3a` checks splitting alone and `C-001e3b` checks coalescing alone, each
# against a sequence written by hand. Both are right about what they ask. What
# neither reaches is the two working against each other: allocate, release,
# allocate again out of the ground the release just changed.
#
# ## The property
#
# NO TWO LIVE BLOCKS SHARE AN OCTET. That is the whole promise of an allocator,
# and breaking it is silent — no message, no failure, no wrong number anywhere
# near the defect. Two owners write into one buffer and the fault appears later,
# elsewhere, in a third piece of code.
#
# The sequence is pseudo-random because the SHAPE of the defect is not known in
# advance. A hand-written order is chosen by the same reasoning that wrote the
# allocator (doc 03 §4.4), so it asks what the author already thought of.
# xorshift64 from seed 1 is deterministic: this is the same sequence on every
# machine, every run, so a failure can be reproduced rather than glimpsed.
#
# The program prints two words per event — `(kind << 8) | order`, then the
# address — and its arena base first. It states no verdict. Every check below is
# computed here from what the program returned; this file states no address of
# its own either.
#
# The same trace continues past the interleaving with the drain and the seven
# free lists (kinds 3, 4 and 5), which is `C-001e3c2` and is read by
# `tools/check-drain.sh`. This check stops at the first drain event: the two
# halves fail differently, so they are asked separately.
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

# verify_stress FILE — the verdict on the allocator trace; prints what the
# stress check prints, exits 1 on any finding. POSIX sh, 64-bit $(( )), no python.
# Addresses are unsigned 64-bit: ordering uses sign-flip compares, sums carry
# into a 65th bit (vs_hi) so no wrap can hide an overrun.
verify_stress() {
    vs_n=0
    while read -r vs_l; do
        [ -n "$vs_l" ] || continue
        vs_h=${vs_l%????????}; vs_o=${vs_l#????????}
        eval "vs_v$vs_n=\$(( (0x$vs_h << 32) | 0x$vs_o ))"
        vs_n=$((vs_n + 1))
    done < "$1"
    vs_MIN=$((1 << 63)); vs_ARENA=4096; vs_LO=6; vs_HI=9
    vs_bad=""
    vs_add() { if [ -n "$vs_bad" ]; then vs_bad="$vs_bad
$1"; else vs_bad=$1; fi; }
    # vs_ult A B: unsigned A < B
    vs_ult() { vs_ua=$1; vs_ub=$2; [ $((vs_ua ^ vs_MIN)) -lt $((vs_ub ^ vs_MIN)) ]; }
    # vs_sum A B (B small, > 0): sets vs_sl (low 64) and vs_sh (carry 0/1)
    vs_sum() { vs_sa=$1; vs_sb=$2; vs_sl=$((vs_sa + vs_sb)); if vs_ult "$vs_sl" "$vs_sa"; then vs_sh=1; else vs_sh=0; fi; }
    # vs_lt H1 L1 H2 L2: 65-bit unsigned less-than
    vs_lt() {
        if [ "$1" -ne "$3" ]; then [ "$1" -lt "$3" ]; else vs_ult "$2" "$4"; fi
    }
    # vs_x H L: hex of a 65-bit value, 16 digits (17 on carry)
    vs_x() { if [ "$1" -eq 1 ]; then printf '1%016x' "$2"; else printf '%016x' "$2"; fi; }

    vs_base=$vs_v0
    if [ $((vs_base & (vs_ARENA - 1))) -ne 0 ]; then
        vs_add "the arena base $(printf '%016x' "$vs_base") is not aligned to $vs_ARENA — buddy arithmetic walks outside a region not aligned to its own size, so nothing below would mean anything"
    fi
    vs_sum "$vs_base" "$vs_ARENA"; vs_beh=$vs_sh; vs_bel=$vs_sl

    vs_ln=0
    vs_allocs=0; vs_frees=0; vs_refusals=0; vs_most=0; vs_recycled=0
    vs_freed=" "; vs_orders=" "; vs_seen=0
    vs_m=$((vs_n - 1)); vs_i=0
    while [ "$vs_i" -lt $((vs_m - 1)) ]; do
        eval "vs_hdr=\$vs_v$((vs_i + 1)); vs_p=\$vs_v$((vs_i + 2))"
        vs_kind=$(( (vs_hdr >> 8) & 0x00ffffffffffffff )); vs_k=$((vs_hdr & 0xff))
        vs_ev=$((vs_i / 2))
        vs_i=$((vs_i + 2))
        case $vs_kind in 3|4|5) break ;; esac
        vs_seen=$((vs_seen + 1))
        if { [ "$vs_kind" -ne 1 ] && [ "$vs_kind" -ne 2 ]; } || [ "$vs_k" -lt "$vs_LO" ] || [ "$vs_k" -gt "$vs_HI" ]; then
            vs_add "event $vs_ev: header $(printf '%016x' "$vs_hdr") is neither an allocation nor a release of an order between $vs_LO and $vs_HI — the trace is not the trace this check knows how to read"
            break
        fi
        case $vs_orders in *" $vs_k "*) ;; *) vs_orders="$vs_orders$vs_k " ;; esac
        vs_size=$((1 << vs_k))
        vs_ps=$(printf '%016x' "$vs_p")

        if [ "$vs_kind" -eq 1 ]; then
            if [ "$vs_p" -eq 0 ]; then vs_refusals=$((vs_refusals + 1)); continue; fi
            vs_allocs=$((vs_allocs + 1))
            if [ $((vs_p & (vs_size - 1))) -ne 0 ]; then
                vs_add "event $vs_ev: order $vs_k block $vs_ps is not aligned to $vs_size — a buddy computed from it lands outside its own half of the region"
            fi
            vs_sum "$vs_p" "$vs_size"; vs_peh=$vs_sh; vs_pel=$vs_sl
            if vs_ult "$vs_p" "$vs_base" || vs_lt "$vs_beh" "$vs_bel" "$vs_peh" "$vs_pel"; then
                vs_add "event $vs_ev: order $vs_k block $vs_ps..$(vs_x "$vs_peh" "$vs_pel") is outside the arena $(printf '%016x' "$vs_base")..$(vs_x "$vs_beh" "$vs_bel") — the allocator handed out memory it does not own"
            fi
            vs_j=0; vs_at=-1
            while [ "$vs_j" -lt "$vs_ln" ]; do
                eval "vs_q=\$vs_lp$vs_j; vs_qk=\$vs_lk$vs_j"
                vs_sum "$vs_q" $((1 << vs_qk))
                if vs_lt 0 "$vs_p" "$vs_sh" "$vs_sl" && vs_lt 0 "$vs_q" "$vs_peh" "$vs_pel"; then
                    vs_add "event $vs_ev: order $vs_k block $vs_ps..$(vs_x "$vs_peh" "$vs_pel") OVERLAPS the live order $vs_qk block $(printf '%016x' "$vs_q")..$(vs_x "$vs_sh" "$vs_sl"). Two owners now hold the same octets. Nothing fails here: they will write over each other later, somewhere else, and the trail back to this moment will be gone"
                    break
                fi
                vs_j=$((vs_j + 1))
            done
            case $vs_freed in *" $vs_p "*) vs_recycled=$((vs_recycled + 1)) ;; esac
            vs_j=0; vs_at=-1
            while [ "$vs_j" -lt "$vs_ln" ]; do
                eval "vs_q=\$vs_lp$vs_j"
                if [ "$vs_q" -eq "$vs_p" ]; then vs_at=$vs_j; break; fi
                vs_j=$((vs_j + 1))
            done
            if [ "$vs_at" -lt 0 ]; then vs_at=$vs_ln; vs_ln=$((vs_ln + 1)); eval "vs_lp$vs_at=\$vs_p"; fi
            eval "vs_lk$vs_at=\$vs_k"
            if [ "$vs_ln" -gt "$vs_most" ]; then vs_most=$vs_ln; fi
        else
            vs_frees=$((vs_frees + 1))
            vs_j=0; vs_at=-1
            while [ "$vs_j" -lt "$vs_ln" ]; do
                eval "vs_q=\$vs_lp$vs_j"
                if [ "$vs_q" -eq "$vs_p" ]; then vs_at=$vs_j; break; fi
                vs_j=$((vs_j + 1))
            done
            if [ "$vs_at" -lt 0 ]; then
                vs_add "event $vs_ev: released $vs_ps at order $vs_k, which is not a live block — the driver's own table disagrees with what it was handed"
            else
                eval "vs_qk=\$vs_lk$vs_at"
                if [ "$vs_qk" -ne "$vs_k" ]; then
                    vs_add "event $vs_ev: released $vs_ps at order $vs_k, taken at order $vs_qk — a block returned to the wrong free list is lost or hands out more memory than it owns"
                else
                    vs_j=$vs_at
                    while [ "$vs_j" -lt $((vs_ln - 1)) ]; do
                        eval "vs_lp$vs_j=\$vs_lp$((vs_j + 1)); vs_lk$vs_j=\$vs_lk$((vs_j + 1))"
                        vs_j=$((vs_j + 1))
                    done
                    vs_ln=$((vs_ln - 1))
                    vs_freed="$vs_freed$vs_p "
                fi
            fi
        fi
    done

    # A run that never interleaves proves nothing, and it would pass every check
    # above by doing almost nothing. So the trace has to show its own work.
    if [ "$vs_seen" -ne 64 ]; then
        vs_add "$vs_seen interleaved events before the drain, expected 64 — the driver stopped early or the drain started before it was done"
    fi
    if [ "$vs_allocs" -lt 20 ]; then
        vs_add "only $vs_allocs allocations succeeded of $((vs_allocs + vs_refusals)) asked — too few for the overlap check to have been asked anything"
    fi
    if [ "$vs_frees" -lt 15 ]; then
        vs_add "only $vs_frees releases — an interleaving needs both directions"
    fi
    if [ "$vs_most" -lt 3 ]; then
        vs_add "at most $vs_most blocks were live at once — with two live blocks a wrong answer has almost nowhere to hide"
    fi
    if [ "$vs_recycled" -lt 1 ]; then
        vs_add "no address was ever handed out again after being released — every allocation came off untouched ground, which is the one case splitting alone already covers"
    fi
    vs_omin=""; vs_omax=""; vs_on=0
    for vs_k in $vs_orders; do
        vs_on=$((vs_on + 1))
        if [ -z "$vs_omin" ] || [ "$vs_k" -lt "$vs_omin" ]; then vs_omin=$vs_k; fi
        if [ -z "$vs_omax" ] || [ "$vs_k" -gt "$vs_omax" ]; then vs_omax=$vs_k; fi
    done
    if [ "$vs_on" -lt 2 ]; then
        if [ "$vs_on" -eq 0 ]; then vs_os="set()"; else vs_os="{$vs_omin}"; fi
        vs_add "every event used order $vs_os — a single order is C-001e2"
    fi

    if [ -n "$vs_bad" ]; then
        printf '%s\n' "$vs_bad"
        return 1
    fi
    echo "$vs_allocs allocations, $vs_frees releases, $vs_refusals refusals, up to $vs_most live at once, orders $vs_omin..$vs_omax, $vs_recycled addresses handed out again after release — no two live blocks ever shared an octet"
}

fail=0
prev_base=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/lines"
    n=$(wc -l < "$tmp/lines" | tr -d ' ')
    if [ "$n" -lt 129 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected at least 129 (the"
        echo "        arena base, then two words for each of 64 events, and"
        echo "        after those the drain that C-001e3c2 reads)"
        fail=1; continue
    fi

    if ! verify_stress "$tmp/lines" > "$tmp/verdict" 2>&1
    then
        sed 's/^/  FAIL  /' "$tmp/verdict"
        fail=1; continue
    fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/verdict")"

    b=$(sed -n 1p "$tmp/lines")
    if [ -n "$prev_base" ] && [ "$b" = "$prev_base" ]; then
        echo "  FAIL  both runs used the arena at $b — the base is a constant"
        fail=1
    fi
    prev_base=$b
done

[ "$fail" -eq 0 ] || { echo; echo "the allocator hands out overlapping memory."; exit 1; }
echo
echo "ok  64 interleaved allocations and releases at four orders, driven by a"
echo "    seeded xorshift, and no two live blocks ever shared an octet. A"
echo "    coalescer that merges a block whose buddy is still in use passes both"
echo "    C-001e3a and C-001e3b and fails HERE, because the merged block is"
echo "    handed out again while its second half is still held"
