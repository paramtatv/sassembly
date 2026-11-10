#!/bin/sh
# A page fault is handed out whole, and handed out once — `C-002h`, doc 11 §3.3.
#
# Doc 11 §3.3 puts page faults in the hands of a USERSPACE PAGER, and says that
# is what makes demand paging, memory-mapped files and copy-on-write userspace
# policies. Read the other way round, it says something sharper: the kernel does
# not decide what a fault MEANS. It packages the fault faithfully and hands it
# out. So what is provable here is not paging policy — it is the honesty of the
# package.
#
# ## Three claims
#
#   1. The package carries the faulting address (`stval`), the cause (`scause`)
#      and the instruction that faulted (`sepc`), and each is the REAL value.
#      The program prints what each should be BEFORE it touches anything; the
#      handler prints what it actually got. The claim is the match. A handler
#      that hands back the page base, or the PC in place of the address, or a
#      plausible constant, dies on one of those comparisons.
#
#   2. Read, write and execute are TELLABLE APART — 13, 15 and 12 in RISC-V. A
#      pager that cannot tell them apart cannot implement copy-on-write, which
#      is the entire reason this lives in userspace. All three faults here are
#      taken on the SAME gigapage, so the cause cannot be coming from the
#      address; only the kind of touch is left.
#
#   3. The fault is delivered ONCE. The handler never advances `sepc` — it
#      RESOLVES, by installing an entry, and the faulting instruction runs
#      again and succeeds. Returning without resolving would re-run the same
#      instruction forever and print nothing at all, which is why the handler
#      counts and stops itself at a fourth fault: the failure gets a face
#      instead of a hang.
#
# ## The hole, and why it is an alias
#
# The identity map is 512 gigapages with the fourth left EMPTY — 4 GiB to 5 GiB
# unmapped. The pager fills it, and fills it onto the KERNEL'S OWN gigapage (2),
# so a resolved address lands on the same physical memory the low address does.
# That is what lets this check say the pager installed the RIGHT entry and not
# merely an entry: a byte written through the alias is read back from below.
#
# ## W^X survives here too
#
#   13 load        -> V R A       = 0x43
#   15 store       -> V R W A D   = 0xc7
#   12 instruction -> V R X A     = 0x4b
#
# No entry ever holds W and X together (`C-002c`). Resolving the store fault
# drops X and resolving the instruction fault drops W, and the entry is printed
# so that this is SHOWN rather than asserted.
#
# ## What this row does NOT contain
#
# The pager is not a separate thread. Threads arrived in `C-002d`, but द्वारम्
# (`C-002e`) has not, and without synchronous IPC there is no way to carry a
# fault to another thread at all. So the handler stands in the pager's place and
# only what can be proved is proved: the package and its once-ness. The HANDOFF
# is a row after `C-002e`. Policy is absent on purpose too — where a page comes
# from is the pager's question, and this row is about the interface.
#
# One more thing this does not prove: the `sfence.vma` the pager issues after
# installing the entry. Deleting it was tried as a mutation and THIS CHECK STILL
# PASSED. QEMU never holds the stale translation that would punish its absence —
# it does not cache a failed walk, and it keeps the write permit separately — so
# on this machine the fence is unfalsifiable. It stays in the program because
# the privileged spec requires it and real hardware does hold that entry; it is
# named here so nobody reads a passing check as evidence for it.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    "$root/tools/sadhana-native.sh" \
        --स्थान "$1" "$root/spec/pager.sas" "$tmp/p.elf" >/dev/null
    # Bounded by an explicit SIGKILL, never perl's alarm: QEMU handles SIGALRM
    # and survives it (W-060). The bound is load-bearing here — an unresolved
    # fault is an infinite loop BY DESIGN, and it is silent while it runs.
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/p.elf" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 20; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -21
}

# verify_pager FILE — the verdict on the 21 hex words; prints what the pager
# check prints, exits 1 on any finding. POSIX sh, 64-bit $(( )), no python.
verify_pager() {
    vp_n=0
    while read -r vp_l; do
        [ -n "$vp_l" ] || continue
        vp_h=${vp_l%????????}; vp_o=${vp_l#????????}
        eval "v$vp_n=\$(( (0x$vp_h << 32) | 0x$vp_o ))"
        vp_n=$((vp_n + 1))
    done < "$1"
    vp_V=1; vp_R=2; vp_W=4; vp_X=8
    vp_FRAME=$((2 << 30)); vp_HOLE=$((4 << 30))
    vp_SEED=$((0x6c1 << 32 | 0x000006c1)); vp_MARK=$((0x652 << 32 | 0x00000652))
    vp_bad=""
    vp_add() { if [ -n "$vp_bad" ]; then vp_bad="$vp_bad
$1"; else vp_bad=$1; fi; }
    vp_hx() { printf '0x%x' "$1"; }
    vp_cn() {
        case $1 in
            13) printf 'load page fault (13)' ;;
            15) printf 'store/AMO page fault (15)' ;;
            12) printf 'instruction page fault (12)' ;;
            *) printf '%u' "$1" ;;
        esac
    }
    # vp_entry name pte grant deny gname dname
    vp_entry() {
        ve_name=$1; ve_pte=$2; ve_grant=$3; ve_deny=$4; ve_gname=$5; ve_dname=$6
        if [ $((ve_pte & vp_V)) -eq 0 ]; then
            vp_add "the $ve_name fault was resolved with entry $(vp_hx "$ve_pte"), which has V clear — the pager answered with a mapping that does not map"
            return
        fi
        ve_pf=$(( (ve_pte >> 10) & ((1 << 54) - 1) ))
        ve_hi=$((ve_pf >> 52)); ve_lo=$(( (ve_pf & ((1 << 52) - 1)) << 12 ))
        if [ "$ve_hi" -eq 0 ]; then ve_fr=$(vp_hx "$ve_lo"); else ve_fr=$(printf '0x%x%016x' "$ve_hi" "$ve_lo"); fi
        if [ "$ve_hi" -ne 0 ] || [ "$ve_lo" -ne "$vp_FRAME" ]; then
            vp_add "the $ve_name fault was resolved onto frame $ve_fr, not the kernel's own gigapage $(vp_hx "$vp_FRAME") — the pager installed an entry but not the one it meant, and the alias no longer names the memory below it"
        fi
        if [ $((ve_pte & ve_grant)) -eq 0 ]; then
            vp_add "the $ve_name fault was resolved with entry $(vp_hx "$ve_pte"), which does not grant $ve_gname — the pager did not act on the cause it was given, so the very same fault is about to arrive again"
        fi
        if [ $((ve_pte & ve_deny)) -ne 0 ]; then
            vp_add "the $ve_name fault was resolved with entry $(vp_hx "$ve_pte"), which grants $ve_dname as well as $ve_gname"
        fi
        if [ $((ve_pte & vp_W)) -ne 0 ] && [ $((ve_pte & vp_X)) -ne 0 ]; then
            vp_add "the $ve_name fault was resolved with entry $(vp_hx "$ve_pte"): writable AND executable. W^X is the rule \`C-002c\` fixed globally, and a pager that breaks it while resolving breaks it everywhere"
        fi
    }

    # ---- fault 1: a load, on an entry that is not there at all ----
    addr=$v0; pc=$v1; cause=$v2; stval=$v3; sepc=$v4; pte=$v5; seen=$v6
    if [ "$cause" -ne 13 ]; then
        vp_add "reading an unmapped address reported $(vp_cn "$cause"), expected $(vp_cn 13)"
    fi
    if [ "$stval" -ne "$addr" ]; then
        vp_x=""
        if [ "$stval" -eq $((addr & ~0xfff)) ]; then vp_x=" — that is the base of the page, not the address, and a pager cannot place a byte it was never told about"; fi
        vp_add "the load fault reported stval $(vp_hx "$stval"), but the address touched was $(vp_hx "$addr")$vp_x"
    fi
    if [ "$sepc" -ne "$pc" ]; then
        vp_x=""
        if [ "$sepc" -eq "$addr" ]; then vp_x=" — that is the faulting ADDRESS, not the faulting instruction"; fi
        vp_add "the load fault reported sepc $(vp_hx "$sepc"), but the faulting load is at $(vp_hx "$pc")$vp_x"
    fi
    if [ "$stval" -eq "$sepc" ]; then
        vp_add "the load fault reported the same value for stval and sepc; a data fault has an address and an instruction and they are two different things"
    fi
    vp_entry load "$pte" $vp_R $vp_X R X
    if [ "$seen" -ne "$vp_SEED" ]; then
        vp_add "reading through the resolved alias gave $(vp_hx "$seen"), expected $(vp_hx "$vp_SEED") — the fault was made to go away without the mapping reaching the right memory"
    fi
    sepc1=$sepc; pte1=$pte

    # ---- fault 2: a store, on an entry that is valid and readable but not writable ----
    addr2=$v7; pc=$v8; cause2=$v9; stval=$v10; sepc=$v11; pte=$v12; back=$v13
    if [ "$cause2" -ne 15 ]; then
        vp_add "writing a mapped read-only address reported $(vp_cn "$cause2"), expected $(vp_cn 15) — this is exactly the fault copy-on-write is built on, and a pager that sees it as a read fault will hand back a shared page"
    fi
    if [ "$stval" -ne "$addr2" ]; then
        vp_add "the store fault reported stval $(vp_hx "$stval"), but the address written was $(vp_hx "$addr2")"
    fi
    if [ "$sepc" -ne "$pc" ]; then
        vp_add "the store fault reported sepc $(vp_hx "$sepc"), but the faulting store is at $(vp_hx "$pc")"
    fi
    if [ "$stval" -eq "$sepc" ]; then
        vp_add "the store fault reported the same value for stval and sepc"
    fi
    vp_entry store "$pte" $vp_W $vp_X W X
    if [ "$back" -ne "$vp_MARK" ]; then
        vp_add "the byte written through the alias read back as $(vp_hx "$back") from the low address, expected $(vp_hx "$vp_MARK") — the alias and the low address are not the same frame, so the pager resolved onto memory nobody else can see"
    fi
    sepc2=$sepc; pte2=$pte

    # ---- fault 3: an instruction fetch, on an entry that is readable and writable ----
    addr3=$v14; cause3=$v15; stval=$v16; sepc=$v17; pte=$v18; landed=$v19
    if [ "$cause3" -ne 12 ]; then
        vp_add "jumping into a mapped non-executable page reported $(vp_cn "$cause3"), expected $(vp_cn 12)"
    fi
    if [ "$stval" -ne "$addr3" ]; then
        vp_add "the instruction fault reported stval $(vp_hx "$stval"), expected $(vp_hx "$addr3")"
    fi
    if [ "$sepc" -ne "$addr3" ]; then
        vp_add "the instruction fault reported sepc $(vp_hx "$sepc"), expected $(vp_hx "$addr3")"
    fi
    if [ "$stval" -ne "$sepc" ]; then
        vp_add "the instruction fault reported stval $(vp_hx "$stval") and sepc $(vp_hx "$sepc"). On an instruction fault the faulting address IS the PC, and a handler that reports two different things for them has invented one of them"
    fi
    vp_entry instruction "$pte" $vp_X $vp_W X W
    if [ "$landed" -ne "$addr3" ]; then
        vp_add "after the instruction fault was resolved, execution continued at $(vp_hx "$landed"), not $(vp_hx "$addr3") — the pager did not put control back where the fault was taken"
    fi
    sepc3=$sepc; pte3=$pte

    # ---- the three causes are the point: they must not collapse into one ----
    if [ "$cause" -eq "$cause2" ]; then
        vp_add "the load fault and the store fault both reported cause $(printf '%u' "$cause") — they are one fault to the pager, and copy-on-write needs them to be two"
    fi
    if [ "$cause" -eq "$cause3" ]; then
        vp_add "the load fault and the instruction fault both reported cause $(printf '%u' "$cause") — they are one fault to the pager, and copy-on-write needs them to be two"
    fi
    if [ "$cause2" -eq "$cause3" ]; then
        vp_add "the store fault and the instruction fault both reported cause $(printf '%u' "$cause2") — they are one fault to the pager, and copy-on-write needs them to be two"
    fi

    # ---- all three on one gigapage, so the cause cannot have come from the address ----
    for vp_k in "load $addr" "store $addr2" "instruction $addr3"; do
        set -- $vp_k
        if [ "$2" -lt "$vp_HOLE" ] || [ "$2" -ge $((vp_HOLE + (1 << 30))) ]; then
            vp_add "the $1 fault was taken at $(vp_hx "$2"), which is outside the empty gigapage $(vp_hx "$vp_HOLE")..$(vp_hx $((vp_HOLE + (1 << 30)))) the pager owns"
        fi
    done

    # ---- delivered once, and exactly once ----
    if [ "$v20" -ne 3 ]; then
        vp_x=""
        if [ "$v20" -gt 3 ] || [ "$v20" -lt 0 ]; then vp_x=" — a fault was handed out again, so the handler returned without resolving it"; fi
        vp_add "$(printf '%u' "$v20") faults were delivered, expected 3$vp_x"
    fi

    if [ -n "$vp_bad" ]; then
        printf '%s\n' "$vp_bad"
        return 1
    fi
    echo "stval, scause and sepc are the real ones at all three faults; load, store and instruction stay distinct on one gigapage; each was delivered exactly once"
    printf 'MOVES %016x %016x %016x %016x %016x %016x\n' "$v0" "$v7" "$v14" "$sepc1" "$sepc2" "$sepc3"
    printf 'FIXED %016x %016x %016x %016x %016x %016x\n' "$v2" "$v9" "$v15" "$pte1" "$pte2" "$pte3"
}

fail=0; first=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    n=$(wc -l < "$tmp/l" | tr -d ' ')
    if [ "$n" -ne 21 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 21 (six for each of"
        echo "        three faults, then the count of faults delivered)"
        if [ "$(tail -1 "$tmp/l")" = "0000000000000004" ]; then
            echo "        the last line is 4: a FOURTH fault arrived, so one of the"
            echo "        three was delivered again — the handler returned without"
            echo "        resolving it and the faulting instruction re-ran"
        fi
        fail=1; continue
    fi
    if ! verify_pager "$tmp/l" > "$tmp/v" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(sed -n 1p "$tmp/v")"

    moves=$(sed -n 's/^MOVES //p' "$tmp/v"); fixed=$(sed -n 's/^FIXED //p' "$tmp/v")
    if [ -z "$first" ]; then first="$moves|$fixed"; continue; fi
    # What is DERIVED must move with the link address: the addresses touched and
    # the instructions that touched them are computed, and a single run cannot
    # tell a computation from a lucky constant.
    if [ "$moves" = "${first%|*}" ]; then
        echo "  FAIL  the faulting addresses and instruction pointers are identical at both"
        echo "        link addresses — they are constants, not values read out of the machine"
        fail=1
    fi
    # What is ARCHITECTURAL must not: 13, 15 and 12 are RISC-V's numbers, and
    # the rights the pager grants are fixed by the cause, not by where it links.
    if [ "$fixed" != "${first#*|}" ]; then
        echo "  FAIL  the causes or the installed entries differ between link addresses:"
        echo "          $(printf '%s' "${first#*|}")"
        echo "          $fixed"
        echo "        13, 15 and 12 are architectural and the rights follow the cause; if"
        echo "        either moved with the link address, something is being read wrong"
        fail=1
    fi
done
[ "$fail" -eq 0 ] || { echo; echo "the fault is not being handed out faithfully."; exit 1; }
echo
echo "ok  a page fault carries its address, its cause and its instruction unaltered;"
echo "    read, write and execute arrive as 13, 15 and 12 from one and the same"
echo "    gigapage; each is resolved by an entry that answers the cause and never"
echo "    grants W and X together, and each is delivered exactly once"
