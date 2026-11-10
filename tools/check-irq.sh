#!/bin/bash
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"  # the verdict function below uses bash arithmetic and arrays
# A hardware interrupt becomes a `सूचना` — `C-002g`, doc 11 §3.1, phase 11.2.7.
#
# `विघ्नग्राहकः` turns a device interrupt into a signal. Doc 11 §3.2 says why
# that matters: it is what lets a device driver live in userspace. The kernel
# does not handle the device. It acknowledges the interrupt, converts it into a
# bit, and returns — and the thread waiting on that `सूचना` does the handling.
#
# ## What is checked
#
#   * THE BIT COMES FROM THE CAUSE. The timer is cause 5 and a software
#     interrupt is cause 1, so the word gets 0x20 and 0x2. One device cannot
#     show this: a handler that sets one constant bit passes every
#     single-device test ever written. So there are two devices here.
#   * AN INTERRUPT WITH NOBODY WAITING IS NOT LOST. The bit stands, and a
#     later wait finds it — `C-002f`'s set property meeting real hardware.
#   * THE HANDLER MUST ACKNOWLEDGE, AND ACKNOWLEDGE THE RIGHT DEVICE. The
#     timer is silenced through SBI, the software interrupt by clearing SSIP
#     in `sip`. Acknowledging one and leaving the other re-arms on the way
#     out, forever.
#   * DELIVERY IS READ FROM THE WORD, NEVER FROM THE WAKEUP COUNT. Two
#     interrupts can produce one wakeup, and one wakeup can stand over an
#     empty word.
#
# ## The count is what makes "it did not queue twice" readable
#
# Two interrupts of one device leaving one bit in the word is correct. It is
# also EXACTLY what a second interrupt that never fired looks like. Only the
# count separates them, so the interrupt count is printed at every turn and
# every claim about queueing is read against it. On `C-002f` that lesson was
# about the test apparatus; here it is about the claim itself.
#
# ## The instant is chosen, and that has to be said
#
# Parts क through ङ arm the device with `sstatus.SIE` clear — the interrupt
# goes pending but is not taken — and then open SIE for one instruction pair.
# That window is the instant. It is not a cheat; it is how kernels build
# critical sections. But what it proves is that the handler is correct AT THE
# INSTANTS NAMED HERE, not at any instant.
#
# Part च closes as much of that as this machine allows: SIE is left open, the
# timer is set to a real future deadline, and a tight loop spins until the word
# changes. The interrupt lands on some instruction of that loop and nothing
# here knows which. What can be checked at an instant nobody chose is the one
# thing an interrupt owes the code it lands on: the code made no promises, so
# every register comes back. `C-002d` proves that for a switch that was ASKED
# for. An interrupt is not asked for, and that is the whole difference.
#
# ## A named failure beats a line count
#
# The handler prints a marker and stops instead of spinning: 0x7f1 if the
# interrupt count runs past the guard (something returned without
# acknowledging), 0x7f2 if the trap was not an interrupt at all, 0x7f3 for an
# interrupt cause it does not know. This check looks for those markers FIRST.
# Without that, a re-arm storm reports "printed 6 lines, expected 32" — true,
# and it sends the reader to count print statements instead of to the
# acknowledge.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

# `perl -e 'alarm'` does not bound QEMU: QEMU installs its own SIGALRM handler
# and outlives it. An explicit `kill -9` watcher does.
run_at() {
    "$root/tools/sadhana-native.sh" \
        --स्थान "$1" "$root/spec/irq.sas" "$2" >/dev/null
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$2" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 20; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    # `-nographic` ends every line with CR. An anchored grep matches nothing
    # against it and reports zero lines rather than a mismatch.
    # No `tail -N` — it returns min(actual, N) and hides any surplus. See W-081.
    #
    # This comment twice claimed something about HOW this check reads its values.
    # Both times the conclusion was defensible and the CAUSE was wrong, and a
    # right conclusion with a wrong cause is how the next reader re-derives the
    # original mistake — they reason from the stated mechanism, not the outcome.
    # All four cells have now been run (sansos-1d), on this file and on
    # check-notification.sh:
    #
    #                    surplus APPENDED             surplus PREPENDED
    #   notification      8 confident false FAILs     confident ok, exit 0
    #   irq              25 confident false FAILs     confident ok, exit 0
    #
    # There is no robust-versus-exposed distinction between the two files. They
    # are indistinguishable, and ONE mechanism explains every cell: `tail -N`
    # realigns the window. A prepended line is discarded and the check certifies
    # a changed program; an appended one is not, so every index shifts and the
    # check names a specific, plausible, wrong defect in the subject — here, that
    # the handler clobbered all seven borrowed registers. How either check reads
    # its values plays no part; 0x77 is not a marker, and the marker grep runs
    # over the already-truncated stream. The count assertion could not see either
    # direction. Capturing without `tail` is what closes both.
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$'
}

fail=0; i=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    i=$((i + 1))
    run_at "$addr" "$tmp/q$i.elf" > "$tmp/l$i"
    # This check had NO count assertion at all until W-081. It reads by marker,
    # so a surplus did not corrupt its verdict — but it ran a CHANGED program to
    # a confident `ok` and said nothing. Reading robustly is not the same as
    # noticing. 32 is measured, not assumed: both link addresses emit exactly 32.
    n=$(wc -l < "$tmp/l$i" | tr -d ' ')
    if [ "$n" -ne 32 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 32. The comparisons"
        echo "        below read by marker and would still have produced a"
        echo "        verdict; it would have been a verdict about a program"
        echo "        that is not the one this check describes."
        fail=1
    fi
done
[ "$fail" -eq 0 ] || { echo; echo "The program did not print what this check expects."; exit 1; }

# The handler's own refusals, before anything else. Each says which rule broke.
for i in 1 2; do
    if grep -aq '^00000000000007f1$' "$tmp/l$i"; then
        # The guard prints its own bound after the count, because a guard set
        # below what a correct run takes trips with NOTHING wrong — and then
        # this marker means the opposite of what it says. `honest` is the total
        # a correct run takes; it is `sum(want_fired)` in the block below.
        honest=9
        bound=$(sed -n '/^00000000000007f1$/{n;n;p;}' "$tmp/l$i" | head -1)
        bound=$((0x$bound))
        if [ "$bound" -le "$honest" ]; then
            echo "  FAIL  the re-arm guard trips at $bound and a correct run of this program"
            echo "        takes $honest interrupts, so the guard fired on its own account and"
            echo "        nothing here says a device went unacknowledged. Raise the guard;"
            echo "        the marker cannot mean anything while it sits below the total."
        else
            echo "  FAIL  the interrupt count ran past the guard at $bound: a handler returned"
            echo "        without acknowledging its device, so the same interrupt was taken"
            echo "        again on the way out and would have spun forever. Acknowledging is"
            echo "        per device — the timer through SBI, a software interrupt by"
            echo "        clearing SSIP — and doing one does not do the other."
        fi
        fail=1
    fi
    if grep -aq '^00000000000007f2$' "$tmp/l$i"; then
        echo "  FAIL  the trap was not an interrupt: scause has bit 63 clear, so this was"
        echo "        an exception and nothing about विघ्नग्राहकः was exercised. The cause"
        echo "        is the line after the marker."
        fail=1
    fi
    if grep -aq '^00000000000007f3$' "$tmp/l$i"; then
        echo "  FAIL  an interrupt arrived with a cause the handler does not know, and it"
        echo "        refused rather than derive a bit from it. The cause is the line"
        echo "        after the marker."
        fail=1
    fi
done
[ "$fail" -eq 0 ] || { echo; echo "विघ्नग्राहकः did not deliver."; exit 1; }

for i in 1 2; do
    n=$(wc -l < "$tmp/l$i" | tr -d ' ')
    if [ "$n" -ne 32 ]; then
        echo "  FAIL  printed $n hex lines, expected 32 (seven for the timer with nobody"
        echo "        waiting, three for the software interrupt, two for both at once,"
        echo "        three for the same one twice, six for wakeups-are-not-counts, nine"
        echo "        for the unchosen instant, then the total and the guard)"
        fail=1
    fi
done
[ "$fail" -eq 0 ] || { echo; echo "विघ्नग्राहकः did not deliver."; exit 1; }

# What moves must move, and what is architectural must not. The image really is
# built twice at two addresses — otherwise running it twice proves nothing —
# and not one of the thirty-two words may depend on where it was built. This
# comes BEFORE the properties on purpose: a word that changes with the link
# address is an address, and reading a property off an address is a wrong
# answer arrived at confidently.
if cmp -s "$tmp/q1.elf" "$tmp/q2.elf"; then
    echo "  FAIL  the two link addresses produced identical images, so the program"
    echo "        never moved and running it twice asserted nothing"
    fail=1
elif ! cmp -s "$tmp/l1" "$tmp/l2"; then
    echo "  FAIL  the two link addresses disagree about what विघ्नग्राहकः does, so some"
    echo "        word below is an address and not a signal:"
    diff "$tmp/l1" "$tmp/l2" | sed 's/^/          /'
    fail=1
fi

hx() { printf "0x%x" "$1"; }
# verdict_irq FILE: 32 hex lines; diagnostics on stdout (first line INPUT-NOT-EVIDENCE when the words are impossible), rc 1 on failure.
verdict_irq() {
    local v=() l i=0 M=$((1<<63)) TIMER=$((1<<5)) SOFT=$((1<<1)) GUARD took=() fired=() imp=() bad=() j k got wnt s
    local want_fired=(1 1 2 2 2 1) where=("the timer, nobody waiting" "the software interrupt" "both devices" "the same device twice" "two while one waiter was announced" "the unchosen instant")
    local w what ix saved=() distinct
    while IFS= read -r l || [ -n "$l" ]; do
        [ -n "${l//[[:space:]]/}" ] || continue
        v[i]=$((16#${l//[[:space:]]/})); i=$((i+1))
    done < "$1"
    GUARD=${v[31]}
    took=("${v[0]}" "${v[3]}" "${v[8]}" "${v[10]}" "${v[12]}" "${v[15]}" "${v[21]}")
    s=0
    for j in 0 1 2 3 4 5; do
        if [ $((took[j+1] ^ M)) -lt $((took[j] ^ M)) ]; then s=1; fi
    done
    if [ $s -eq 1 ]; then
        s="["; for j in 0 1 2 3 4 5 6; do s="$s$(printf %u "${took[j]}")"; [ $j -lt 6 ] && s="$s, "; done; s="$s]"
        imp+=("the interrupt count runs BACKWARDS across the run: $s. The count is cumulative and only ever incremented, so no ordering of interrupts can produce this")
    fi
    for w in "part 1:5" "part 5:18" "part 5, second wait:20"; do
        what=${w%:*}; ix=${w##*:}
        if [ "${v[ix]}" -ne 0 ] && [ "${v[ix]}" -ne 1 ]; then
            imp+=("the slept flag for $what is $(hx "${v[ix]}"), which is neither 0 nor 1")
        fi
    done
    if [ "${v[30]}" -ne "${took[6]}" ]; then
        imp+=("the run reports $(printf %u "${v[30]}") interrupts in total but its last per-turn count is $(printf %u "${took[6]}") — the two are read from the same word and cannot differ")
    fi
    if [ "${#imp[@]}" -gt 0 ]; then
        echo "INPUT-NOT-EVIDENCE"
        printf '%s\n' "${imp[@]}"
        echo "THESE WORDS CANNOT BE EVIDENCE ABOUT विघ्नग्राहकः, so nothing below was evaluated."
        echo "Two causes produce this and THIS CHECK CANNOT TELL THEM APART, so it does not guess:"
        echo '  * the stream is misaligned — a surplus or missing line shifts every index, and the'
        echo '    comparisons below then describe a program that was never run (`W-081`);'
        echo '  * the instrumentation is broken — the counters themselves are wrong, in which case'
        echo '    the fault is real but is in the harness, not in the handler the check names.'
        echo "Either way the per-part findings below would be detailed, specific and wrong."
        return 1
    fi
    if [ "${v[0]}" -ne 0 ]; then
        bad+=("a window opened with nothing armed took $(printf %u "${v[0]}") interrupt(s), so no count in this run is evidence of anything — the instants and the interrupts are not in correspondence")
    fi
    for j in 0 1 2 3 4 5; do fired[j]=$((took[j+1] - took[j])); done
    for k in 0 1 2 3 4 5; do
        got=${fired[k]}; wnt=${want_fired[k]}
        if [ "$got" -ne "$wnt" ]; then
            bad+=("part $((k + 1)) (${where[k]}) took $(printf %u "$got") interrupt(s), expected $wnt. Everything that part claims is read against that number")
        fi
    done
    if [ "${v[1]}" -ne $TIMER ] || [ "${v[7]}" -ne $SOFT ]; then
        bad+=("the timer left $(hx "${v[1]}") and the software interrupt left $(hx "${v[7]}") in the word, expected $(hx $TIMER) and $(hx $SOFT) — one shifted left by the cause number of each")
    fi
    if [ "${v[1]}" -eq "${v[7]}" ]; then
        bad+=("both devices left the same bit $(hx "${v[1]}"), so the word does not say WHICH device interrupted. A handler that sets one constant bit passes every test that only ever fires one device")
    fi
    if [ "${v[2]}" -ne 0 ]; then
        bad+=("$(printf %u "${v[2]}") wakeup(s) were delivered with no waiter announced")
    fi
    if [ "${v[4]}" -ne $TIMER ]; then
        bad+=("the wait after the interrupt got $(hx "${v[4]}"), expected $(hx $TIMER): the interrupt arrived while nobody was waiting and was not still standing when somebody finally did")
    fi
    if [ "${v[5]}" -ne 0 ]; then
        bad+=("the waiter slept even though the interrupt's bit was standing in the word")
    fi
    if [ "${v[6]}" -ne 0 ]; then
        bad+=("a second wait returned $(hx "${v[6]}") from a word that had just been taken")
    fi
    if [ "${v[9]}" -ne $SOFT ]; then
        bad+=("the wait after the software interrupt got $(hx "${v[9]}"), expected $(hx $SOFT)")
    fi
    if [ "${v[11]}" -ne $((TIMER | SOFT)) ]; then
        bad+=("with both devices signalled, one wait got $(hx "${v[11]}"), expected $(hx $((TIMER | SOFT))) — a wait takes the whole word, not one device")
    fi
    if ! { [ "${fired[3]}" -eq 2 ] && [ "${v[13]}" -eq $TIMER ] && [ "${v[14]}" -eq 0 ]; }; then
        bad+=("the same device interrupted $(printf %u "${fired[3]}") time(s) and one wait then got $(hx "${v[13]}") with $(hx "${v[14]}") left behind; expected 2, $(hx $TIMER) and 0. Both halves matter: a single bit after two interrupts is correct, and is indistinguishable from a second interrupt that never happened")
    fi
    if ! { [ "${fired[4]}" -eq 2 ] && [ "${v[16]}" -eq 1 ] && [ "${v[17]}" -eq $((TIMER | SOFT)) ]; }; then
        bad+=("$(printf %u "${fired[4]}") interrupts reached one announced waiter, delivering $(printf %u "${v[16]}") wakeup(s), and the wait then got $(hx "${v[17]}"); expected 2, 1 and $(hx $((TIMER | SOFT))). A waiter that counted wakeups would conclude that $(printf %u "${v[16]}") interrupt(s) arrived — delivery is in the word, and the wakeup is only the nudge that makes somebody read it")
    fi
    if [ "${v[18]}" -ne 0 ]; then
        bad+=("the waiter slept with both devices' bits in hand")
    fi
    if ! { [ "${v[19]}" -eq 0 ] && [ "${v[20]}" -eq 1 ]; }; then
        if [ "${v[20]}" -ne 0 ]; then s=slept; else s="stayed awake"; fi
        bad+=("after the word was emptied, the next wait got $(hx "${v[19]}") and $s; expected 0 and slept. A wakeup left standing over an empty word is spurious, and the waiter has to be able to see that it is")
    fi
    if [ "${v[22]}" -ne $TIMER ]; then
        bad+=("the wait after the unchosen instant got $(hx "${v[22]}"), expected $(hx $TIMER): either nothing landed inside the spin, and then the registers below crossed no trap at all, or something landed carrying a bit that is not the device this part armed")
    fi
    saved=("${v[@]:23:7}")
    for j in 0 1 2 3 4 5 6; do
        got=${saved[j]}; wnt=$((0xa00 + j))
        if [ "$got" -ne "$wnt" ]; then
            bad+=("क्षणिक$j came back $(hx "$got") from a trap taken at an instant it did not choose, expected $(hx "$wnt"). The interrupted code made no promises — it did not call anything — so the handler owes it every register it borrowed")
        fi
    done
    distinct=$(printf '%s\n' "${saved[@]}" | sort -u | wc -l | tr -d ' ')
    if [ "$distinct" -ne 7 ]; then
        bad+=("the seven registers do not all hold distinct values, so a handler that restored them in the wrong order could not be seen")
    fi
    if [ "${v[30]}" -ne "${took[6]}" ] || [ "${v[30]}" -ne 9 ]; then
        bad+=("the run took $(printf %u "${v[30]}") interrupts in total, expected 9")
    fi
    if [ $((GUARD ^ M)) -le $((v[30] ^ M)) ]; then
        bad+=("the re-arm guard trips at $(printf %u "$GUARD") and the run legitimately takes $(printf %u "${v[30]}") — the guard would fire on a correct program, and its marker would then mean nothing")
    fi
    if [ "${#bad[@]}" -gt 0 ]; then printf '%s\n' "${bad[@]}"; return 1; fi
    printf 'timer→%s and software→%s from the cause alone; %u interrupts across six parts, none lost with nobody waiting, two of one device leaving %s; two interrupts delivering %u wakeup and %s in the word; and seven registers returned intact from a trap taken at an instant nothing chose\n' \
        "$(hx $TIMER)" "$(hx $SOFT)" "${v[30]}" "$(hx "${v[13]}")" "${v[16]}" "$(hx "${v[17]}")"
}

i=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    i=$((i + 1))
    if ! verdict_irq "$tmp/l$i" > "$tmp/v$i" 2>&1
    then
        if grep -a -q '^INPUT-NOT-EVIDENCE$' "$tmp/v$i"; then notevidence=1; fi
        grep -a -v '^INPUT-NOT-EVIDENCE$' "$tmp/v$i" | sed 's/^/  FAIL  /'
        fail=1; continue
    fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v$i")"
done

# The verdict must not name विघ्नग्राहकः when the guard has just refused to. A
# footer that blames the subject anyway reintroduces, in one line, the whole
# defect the input guard exists to prevent (`W-090`).
if [ "${notevidence:-0}" -eq 1 ]; then
    echo
    echo "This run says nothing about विघ्नग्राहकः, for or against."
    exit 1
fi
[ "$fail" -eq 0 ] || { echo; echo "विघ्नग्राहकः did not deliver."; exit 1; }
echo
echo "ok  a device interrupt becomes a सूचना bit chosen by its cause, is not lost"
echo "    when nobody is waiting, does not queue when it repeats, is acknowledged"
echo "    per device so nothing re-arms on the way out, and hands back every"
echo "    register it borrowed from code that never agreed to lend one"
