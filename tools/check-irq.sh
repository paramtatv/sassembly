#!/bin/sh
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
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
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
    # All four cells have now been run (a peer session), on this file and on
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

i=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    i=$((i + 1))
    if ! python3 - "$tmp/l$i" > "$tmp/v$i" 2>&1 <<'PY'
import sys

v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]
bad = []

TIMER, SOFT = 1 << 5, 1 << 1      # cause 5 and cause 1
GUARD = v[31]
took = [v[0], v[3], v[8], v[10], v[12], v[15], v[21]]   # the count at each turn

# ---- impossible values in the INPUT, before it is read as evidence (`W-090`)
# Everything below reads these thirty-two words as statements about
# विघ्नग्राहकः. These four establish they are statements about anything at all.
# None can fail on a correct run whatever the handler does, because their
# subject is the DATA and not the program: a count that runs backwards, or a
# flag outside {0,1}, is not a wrong answer but an impossible one, and an
# impossible value can only mean these are not the words this check thinks it
# is reading.
#
# `W-081` produced exactly that. One surplus print shifted every index, and
# this file reported `part 2 took -30 interrupt(s), expected 1` and
# `-31 interrupts reached one announced waiter` — twenty-five confident
# findings against a handler nobody had touched, including the accusation that
# it clobbered all seven registers. It held proof that its own input was
# corrupt and spent it blaming the subject.
impossible = []
if any(b < a for a, b in zip(took, took[1:])):
    impossible.append(f"the interrupt count runs BACKWARDS across the run: {took}. The count is "
                      "cumulative and only ever incremented, so no ordering of interrupts can "
                      "produce this")
for what, ix in (("part 1", 5), ("part 5", 18), ("part 5, second wait", 20)):
    if v[ix] not in (0, 1):
        impossible.append(f"the slept flag for {what} is {v[ix]:#x}, which is neither 0 nor 1")
if v[30] != took[-1]:
    impossible.append(f"the run reports {v[30]} interrupts in total but its last per-turn count "
                      f"is {took[-1]} — the two are read from the same word and cannot differ")
if impossible:
    print("INPUT-NOT-EVIDENCE")
    print("\n".join(impossible))
    print("THESE WORDS CANNOT BE EVIDENCE ABOUT विघ्नग्राहकः, so nothing below was evaluated.")
    print("Two causes produce this and THIS CHECK CANNOT TELL THEM APART, so it does not guess:")
    print("  * the stream is misaligned — a surplus or missing line shifts every index, and the")
    print("    comparisons below then describe a program that was never run (`W-081`);")
    print("  * the instrumentation is broken — the counters themselves are wrong, in which case")
    print("    the fault is real but is in the harness, not in the handler the check names.")
    print("Either way the per-part findings below would be detailed, specific and wrong.")
    sys.exit(1)

# ---- the window itself, before anything is read off it --------------------
# Every count below is evidence only if an instant with nothing armed takes
# nothing. Check the apparatus before the claim.
if v[0] != 0:
    bad.append(f"a window opened with nothing armed took {v[0]} interrupt(s), so no count "
               "in this run is evidence of anything — the instants and the interrupts "
               "are not in correspondence")

# ---- how many interrupts each part actually fired ------------------------
fired = [b - a for a, b in zip(took, took[1:])]
want_fired = [1, 1, 2, 2, 2, 1]
where = ["the timer, nobody waiting", "the software interrupt", "both devices",
         "the same device twice", "two while one waiter was announced",
         "the unchosen instant"]
for k, (got, wnt) in enumerate(zip(fired, want_fired)):
    if got != wnt:
        bad.append(f"part {k + 1} ({where[k]}) took {got} interrupt(s), expected {wnt}. "
                   "Everything that part claims is read against that number")

# ---- the bit comes from the cause ----------------------------------------
if v[1] != TIMER or v[7] != SOFT:
    bad.append(f"the timer left {v[1]:#x} and the software interrupt left {v[7]:#x} in the "
               f"word, expected {TIMER:#x} and {SOFT:#x} — one shifted left by the cause "
               "number of each")
if v[1] == v[7]:
    bad.append(f"both devices left the same bit {v[1]:#x}, so the word does not say WHICH "
               "device interrupted. A handler that sets one constant bit passes every "
               "test that only ever fires one device")

# ---- an interrupt with nobody waiting is not lost -------------------------
if v[2] != 0:
    bad.append(f"{v[2]} wakeup(s) were delivered with no waiter announced")
if v[4] != TIMER:
    bad.append(f"the wait after the interrupt got {v[4]:#x}, expected {TIMER:#x}: the "
               "interrupt arrived while nobody was waiting and was not still standing "
               "when somebody finally did")
if v[5] != 0:
    bad.append("the waiter slept even though the interrupt's bit was standing in the word")
if v[6] != 0:
    bad.append(f"a second wait returned {v[6]:#x} from a word that had just been taken")

# ---- one wait takes every device at once ---------------------------------
if v[9] != SOFT:
    bad.append(f"the wait after the software interrupt got {v[9]:#x}, expected {SOFT:#x}")
if v[11] != TIMER | SOFT:
    bad.append(f"with both devices signalled, one wait got {v[11]:#x}, expected "
               f"{TIMER | SOFT:#x} — a wait takes the whole word, not one device")

# ---- the same interrupt twice does not queue twice -----------------------
# The two halves are one claim and must be stated together: "one bit in the
# word" is also what a second interrupt that never fired looks like.
if not (fired[3] == 2 and v[13] == TIMER and v[14] == 0):
    bad.append(f"the same device interrupted {fired[3]} time(s) and one wait then got "
               f"{v[13]:#x} with {v[14]:#x} left behind; expected 2, {TIMER:#x} and 0. "
               "Both halves matter: a single bit after two interrupts is correct, and is "
               "indistinguishable from a second interrupt that never happened")

# ---- a wakeup is a hint; the word is the fact -----------------------------
if not (fired[4] == 2 and v[16] == 1 and v[17] == TIMER | SOFT):
    bad.append(f"{fired[4]} interrupts reached one announced waiter, delivering {v[16]} "
               f"wakeup(s), and the wait then got {v[17]:#x}; expected 2, 1 and "
               f"{TIMER | SOFT:#x}. A waiter that counted wakeups would conclude that "
               f"{v[16]} interrupt(s) arrived — delivery is in the word, and the wakeup "
               "is only the nudge that makes somebody read it")
if v[18] != 0:
    bad.append("the waiter slept with both devices' bits in hand")
if not (v[19] == 0 and v[20] == 1):
    bad.append(f"after the word was emptied, the next wait got {v[19]:#x} and "
               f"{'slept' if v[20] else 'stayed awake'}; expected 0 and slept. A wakeup "
               "left standing over an empty word is spurious, and the waiter has to be "
               "able to see that it is")

# ---- the unchosen instant -------------------------------------------------
if v[22] != TIMER:
    bad.append(f"the wait after the unchosen instant got {v[22]:#x}, expected {TIMER:#x}: "
               "either nothing landed inside the spin, and then the registers below "
               "crossed no trap at all, or something landed carrying a bit that is not "
               "the device this part armed")
saved = v[23:30]
want_saved = [0xa00 + j for j in range(7)]
if saved != want_saved:
    for j, (got, wnt) in enumerate(zip(saved, want_saved)):
        if got != wnt:
            bad.append(f"क्षणिक{j} came back {got:#x} from a trap taken at an instant it did "
                       f"not choose, expected {wnt:#x}. The interrupted code made no "
                       "promises — it did not call anything — so the handler owes it every "
                       "register it borrowed")
if len(set(saved)) != 7:
    bad.append("the seven registers do not all hold distinct values, so a handler that "
               "restored them in the wrong order could not be seen")

# ---- the guard is above the real total ------------------------------------
if v[30] != took[-1] or v[30] != sum(want_fired):
    bad.append(f"the run took {v[30]} interrupts in total, expected {sum(want_fired)}")
if GUARD <= v[30]:
    bad.append(f"the re-arm guard trips at {GUARD} and the run legitimately takes "
               f"{v[30]} — the guard would fire on a correct program, and its marker "
               "would then mean nothing")

if bad:
    print("\n".join(bad)); sys.exit(1)
print(f"timer→{TIMER:#x} and software→{SOFT:#x} from the cause alone; {v[30]} interrupts "
      f"across six parts, none lost with nobody waiting, two of one device leaving "
      f"{v[13]:#x}; two interrupts delivering {v[16]} wakeup and {v[17]:#x} in the word; "
      "and seven registers returned intact from a trap taken at an instant nothing chose")
PY
    then
        if grep -q '^INPUT-NOT-EVIDENCE$' "$tmp/v$i"; then notevidence=1; fi
        grep -v '^INPUT-NOT-EVIDENCE$' "$tmp/v$i" | sed 's/^/  FAIL  /'
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
