#!/bin/sh
# Root task and supervisor with restart contracts — task `C-003`, doc 11 §7
# (phase 11.3.1 `आदिकर्ता` and 11.3.2 `अधिष्ठाता`), doc 11 §5.2.
#
# THIS CHECK IS WRITTEN BEFORE `spec/root-task.sas` EXISTS, for the reason
# `C-002j`, `C-002e2a` and `C-002e3a` were written first: a check composed after
# the program it checks is a check whose author already knew the answer, and it
# tests the answer rather than the claim. Until the program lands this exits 1
# and says what is missing.
#
# ## The row's done-when is four claims, and only the fourth is a CONTRACT
#
# "A killed driver restarts." Taken apart:
#
#   1 THE GRAPH CAME FROM THE DESCRIPTION — two address spaces exist because the
#     system description has two records, and the root task walked it.
#   2 THE DRIVER RAN, THEN DIED — it wrote a word derived from its own frame,
#     then took a fault it did not survive.
#   3 IT CAME BACK — it ran a second time, as a FRESH instance rather than a
#     resumed one.
#   4 THE RESTART HONOURED ITS CONTRACT — a second service takes the IDENTICAL
#     fault and, because its `पुनरारम्भ्यः` bit is clear, STAYS DOWN.
#
# Claim 4 is why this is a supervisor and not a retry loop. A supervisor that
# restarts unconditionally satisfies 1-3 and is not what doc 11 §5.2 describes;
# without 4 the green means "something restarted", which is exactly the
# plausible-looking pass this row was warned about. The two services execute the
# SAME user text — the only difference between them is a record in the
# description — so "one came back and one did not" is a property of the
# contract and cannot be a property of the code.
#
# ## The nine words, in the order the machine produces them
#
#   1  satp installed for service 0, the driver     Sv39: mode nibble 8
#   2  satp installed for service 1                 Sv39, and DIFFERENT from 1
#   3  the service count, walked out of description  exactly 2
#   4  scause of service 0's fault                  exactly 15, STORE PAGE FAULT
#   5  the word service 0 wrote in its first life   nonzero, and it MOVES
#   6  service 0's restart count, after the restart exactly 1
#   7  the word service 0 wrote in its second life  exactly word 5 plus one
#   8  scause of service 1's fault                  exactly 15 — the SAME death
#   9  service 1's restart count, after supervision exactly 0 — THE CONTRACT
#
# Word 7 is claim 3 and it is why the supervisor reseeds rather than resumes:
# the kernel writes the restart count into the service's data frame before every
# entry, the user text adds one and stores it, so the second life necessarily
# writes a word the first life could not have left behind. A resumed thread
# would re-print word 5.
#
# Words 4 and 8 must be EQUAL, and that is the load-bearing pair. Word 9 means
# "the supervisor declined to restart" only if service 1 actually died the same
# death as service 0. If word 8 differed from word 4, word 9 would be reporting
# a different situation and the contract would be untested.
#
# ## What MOVES and what MUST NOT, at two link addresses
#
# `tools/check-milestone-k1.sh`'s discipline, and what separates this from a
# program printing constants:
#
#   MOVES  1, 2, 5, 7 — every one derived from where the image was loaded.
#   FIXED  3, 4, 6, 8, 9 — a service count, two architectural cause numbers and
#          two restart counts. Any of these following the link address would
#          mean it is not being read from where it is claimed to be read.
#
# ## What this does NOT prove, stated so the green is not over-read
#
#   - NOT that the description was LOADED. There is no filesystem until `C-005`,
#     so the root task builds the description in memory and then walks it. What
#     is tested is that the graph is built by WALKING that data — the count in
#     word 3 is read back out of the table — not that it was parsed off a disk.
#     A program with two hand-written address spaces and no table would pass
#     words 1 and 2 and could not produce word 3 from anywhere honest.
#   - NOT a DEVICE driver. There is no device until `C-004`; "driver" here is the
#     supervised child, which is what the row's done-when is about.
#   - NOT the other three contract properties of doc 11 §5.2. `स्थितिबाह्यः`,
#     `आश्रिताः` (dependent-service notification) and `पुनर्योजनम्` (client
#     reconnection) are NOT exercised. Only `पुनरारम्भ्यः` is. A supervision
#     tree that notifies dependents is a later row and this green does not
#     reach it.
#   - NOT hot patching. Doc 16 Tier 1 sits on this; it is not this.
#   - NOT isolation beyond what `C-002l` established.
#   - NOT that the fault was DETECTED rather than expected. The supervisor reads
#     `scause`, but a kernel that assumed a fault would print the same 15. What
#     is tested is the CONSEQUENCE — one service back, one not — under one text.
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
# rt_name N : the name of word N (1-based) of the root-task contract.
rt_name() {
    case $1 in
    1) echo "satp for service 0" ;;
    2) echo "satp for service 1" ;;
    3) echo "the service count from the description" ;;
    4) echo "scause on service 0's fault" ;;
    5) echo "the word service 0's first life wrote" ;;
    6) echo "service 0's restart count" ;;
    7) echo "the word service 0's second life wrote" ;;
    8) echo "scause on service 1's fault" ;;
    9) echo "service 1's restart count" ;;
    esac
}
# verify_rt LINES : the nine-word root-task verdict; stdout = message, rc 0/1.
verify_rt() {
    loadvals w "$1"
    if [ "$w_n" -ne 9 ]; then
        extra=""
        if [ "$w_n" -gt 9 ]; then
            extra=" More than nine has exactly ONE meaning, and the program is arranged so that it does: only service 0's restart count is printed, so a service restarted against its contract changes word 9 rather than the line count. A surplus therefore means the supervision loop did not stop — a restart contract that never stops being satisfied."
        fi
        names=""; k=1
        while [ "$k" -le 9 ]; do
            if [ -n "$names" ]; then names="$names; $k $(rt_name $k)"; else names="$k $(rt_name $k)"; fi
            k=$((k + 1))
        done
        printf 'printed %s hex lines, expected 9.%s The words that should be there, in order, are: %s\n' "$w_n" "$extra" "$names"
        return 1
    fi
    bad=""
    for i in 1 2; do
        eval "s=\$w$i"
        nib=$(( (s >> 60) & 15 ))
        if [ "$nib" -ne 8 ]; then
            add "$(printf 'word %s, %s: mode nibble %x, not 8. Sv39 is the only paging mode this tree installs, and a satp that is not in it was never installed' "$i" "$(rt_name $i)" "$nib")"
        fi
    done
    if [ "$w1" -eq "$w2" ]; then
        add "$(printf 'words 1 and 2 are the same satp (%016x), so both services share ONE address space and there is no second service to hold a different contract' "$w1")"
    fi
    if [ "$w3" -ne 2 ]; then
        add "$(printf 'word 3, the service count from the description: %u, expected 2. This is read back out of the description the root task walked. Two address spaces with a count that does not say two means the spaces were written by hand and the description is decoration' "$w3")"
    fi
    if [ "$w4" -ne 15 ]; then
        add "$(printf "word 4, scause on service 0's fault: cause %u, expected 15, STORE/AMO PAGE FAULT (RISC-V privileged §4.1.8). The driver is killed by storing to an address its own space does not grant it, and a different cause means it died of something this program did not arrange" "$w4")"
    fi
    if [ "$w5" -eq 0 ]; then
        add "word 5, the word service 0's first life wrote: zero, which is what the frame reads before anything is written to it. Nothing distinguishes 'the service wrote 0' from 'the service never ran'"
    fi
    if [ "$w6" -ne 1 ]; then
        add "$(printf "word 6, service 0's restart count: %u, expected 1. The supervisor restarted the driver exactly once; 0 means it never did and anything higher means it did not stop" "$w6")"
    fi
    nx=$(( w5 + 1 ))
    if [ "$w7" -ne "$nx" ]; then
        add "$(printf "word 7, the word service 0's second life wrote: %016x, and word 5 plus one is %016x. The kernel writes the restart count into the frame before every entry and the user text adds one, so a second life MUST write one more than the first. A word equal to word 5 is a thread that was resumed, not a service that was restarted" "$w7" "$nx")"
    fi
    if [ "$w8" -ne "$w4" ]; then
        add "$(printf "word 8, scause on service 1's fault: cause %u, but service 0 died of %u. The two services run the SAME user text and must die the SAME death — otherwise word 9 is reporting on a different situation and the contract is untested" "$w8" "$w4")"
    fi
    if [ "$w9" -ne 0 ]; then
        add "$(printf "word 9, service 1's restart count: %u, expected 0. Service 1's पुनरारम्भ्यः bit is clear. It took the identical fault under the identical text, and a supervisor that brought it back anyway is restarting unconditionally rather than honouring a contract — which is the whole difference this row exists to establish" "$w9")"
    fi
    if [ -n "$bad" ]; then printf '%s\n' "$bad"; return 1; fi
    printf "satp %016x/%016x from a description of %u, driver died of %u writing %016x, came back %ux writing %016x, and service 1 died of %u and stayed down (%u restarts)\n" \
        "$w1" "$w2" "$w3" "$w4" "$w5" "$w6" "$w7" "$w8" "$w9"
}
# verify_rt_moved LO HI : derived words must move, fixed words must not.
verify_rt_moved() {
    loadvals lo "$1"; loadvals hi "$2"
    bad=""
    for i in 1 2 5 7; do
        eval "a=\$lo$i b=\$hi$i"
        if [ "$a" -eq "$b" ]; then
            add "$(printf 'word %s, %s: %016x at both link addresses. It is derived from where the image was loaded, so a value that did not move is a constant in the source and the derivation it stands for never happened' "$i" "$(rt_name $i)" "$a")"
        fi
    done
    for i in 3 4 6 8 9; do
        eval "a=\$lo$i b=\$hi$i"
        if [ "$a" -ne "$b" ]; then
            add "$(printf 'word %s, %s: %u then %u. A service count, a cause number and a restart count are none of them properties of where the image was loaded. One that follows the link address is not being read from where it is claimed to be' "$i" "$(rt_name $i)" "$a" "$b")"
        fi
    done
    if [ -n "$bad" ]; then printf '%s\n' "$bad"; return 1; fi
    echo "every derived word moved between the two link addresses, and the count, both causes and both restart counts stayed put"
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "CANNOT RUN: qemu-system-riscv64 is not installed, so nothing was measured"
  exit 77; }

prog=$root/spec/root-task.sas
[ -f "$prog" ] || {
  echo "RED, as written: spec/root-task.sas does not exist."
  echo
  echo "  C-003 puts this contract on the record before the program that has to"
  echo "  satisfy it. The nine words it demands are in the header above: two"
  echo "  address spaces built by walking a system description, one service that"
  echo "  dies and comes back reseeded, and a second service that dies the same"
  echo "  death under a cleared पुनरारम्भ्यः bit and stays down."
  echo
  echo "  Nothing is wrong with the tree. This is the red a contract check is"
  echo "  supposed to be while its row is open."
  exit 1; }

run_at() {
    "$root/tools/sadhana-native.sh" \
        --स्थान "$1" "$prog" "$tmp/rt.elf" >/dev/null
    # Bounded by an explicit SIGKILL, not perl's alarm: QEMU handles SIGALRM and
    # outlives it (`W-060`). A supervisor with a restart bug runs forever, which
    # is precisely the failure this row can produce, so the bound is not
    # decoration.
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/rt.elf" > "$tmp/out" 2>&1 &
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
    # No `tail -9`: it returns min(actual, 9), which makes a count assertion
    # one-sided by construction and blind to a surplus (`W-081`). A supervisor
    # that restarts forever prints MORE than nine words and that is a failure
    # this must be able to see.
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' || true
}

fail=0
# Two link addresses, because every address in that program is computed with
# auipc and one run cannot tell a computed address from a lucky constant.
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l.$addr"
    if ! verify_rt "$tmp/l.$addr" > "$tmp/v" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
done

# ---- what moved, and what had better not --------------------------------
# Only reached when both runs are internally well formed; otherwise the words
# below are not the words this compares.
if [ "$fail" -eq 0 ]; then
    if ! verify_rt_moved "$tmp/l.०षोड्८०२०००००" "$tmp/l.०षोड्८०४०००००" > "$tmp/m" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/m"; fail=1
    else printf "  both addresses -> %s\n" "$(cat "$tmp/m")"; fi
fi

[ "$fail" -eq 0 ] || {
    echo
    echo "C-003 is not on the record. A killed driver restarting is four claims"
    echo "and this reports which one is missing: a count read back out of the"
    echo "description is the graph coming from data, a fault of 15 after a word"
    echo "was written is a driver that ran and died, a second word one greater is"
    echo "a fresh instance rather than a resumed thread, and a second service"
    echo "dying the SAME death under a cleared पुनरारम्भ्यः bit and staying down"
    echo "is the difference between a supervisor and a retry loop."
    exit 1; }

echo
echo "ok  at both link addresses: two address spaces built by walking a"
echo "    description of two, the driver wrote a word and died of a store page"
echo "    fault, came back exactly once with its frame reseeded and wrote one"
echo "    more, and the service whose पुनरारम्भ्यः bit is clear took the same"
echo "    fault under the same text and was not brought back. Dependent-service"
echo "    notification and the other three contract properties are NOT claimed"
echo "    here — see the header."
