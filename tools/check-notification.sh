#!/bin/sh
# A signal arriving between the check and the sleep is not lost — `C-002f`,
# doc 11 §3.1, phase 11.2.6.
#
# `सूचना` is the asynchronous counterpart to an endpoint: one signal word, and
# a binding discipline that is three sentences long.
#
#   * signalling sets a bit — signalling twice does not queue twice, because
#     this is a set and not a counter;
#   * waiting takes the WHOLE word and returns it, so nothing is lost between
#     the read and the clear: the read and the clear are one instruction;
#   * a signal arriving between the check and the sleep is not dropped.
#
# ## The third one is what this check is built around
#
# Get either of the first two wrong and any test notices at once. Not the
# third. A lost wakeup has no symptom at the time it happens: the waiter simply
# stays asleep, and the signal stands in the word with nobody looking at it. A
# design that gets it wrong PASSES every test that does not construct the
# interleaving deliberately — so this one constructs it deliberately.
#
# ## The interleaving is placed by hand, because there is one hart
#
# On a single hart the only way to say "the other hart signals at exactly this
# instant" is to put it there yourself. `प्रतीक्षणम्` therefore calls `अन्तरालः`
# at four points inside itself, and `अन्तरालः` signals at the one point that was
# asked for and at no other. The program runs all four points in turn.
#
#   point 0   before the waiter announces it is going to sleep
#   point 1   after the announcement, before the check
#   point 2   after the check, which found the word empty
#   point 3   after the decision to sleep
#
# A correct `सूचना` survives all four. The classic bug — check first, announce
# afterwards — survives THREE of them and drops the signal at exactly one. That
# ratio is the whole point: three passing points is what "it works" looks like.
#
# ## The rule the trials are read against
#
#   a sleeping waiter must never have a signal standing in the word and no
#   wakeup delivered
#
# A wakeup too many is not a defect — sleeping is always inside a loop, so a
# waiter woken for nothing checks again and goes back to sleep. A wakeup too
# few is a defect, because nothing happens afterwards. The asymmetry is the
# reason the announcement comes FIRST and the check second; the other order
# also works nearly always.
#
# ## What this check cannot construct, and why that is an answer
#
# There is no point inside the take itself, because the take is one instruction
# — `परमाणुविनिमयः`. There is no instant in it to place anything at. That is not
# a gap in the check but the shape of the design: the second property holds
# because the interval does not exist, not because something wins inside it. A
# design that read and then cleared would have that instant, and this one
# refuses to have it.
#
# ## The trials also report on themselves
#
# Each trial prints how many points the wait actually passed, and it must be
# four. That number is not about `सूचना`; it is the check auditing its own
# apparatus. Break the point counter and every trial comes out identical with
# the signal nowhere — which reads exactly like a lost signal, and is not one,
# because nothing was ever sent. A diagnosis that cannot tell "the signal was
# dropped" from "no signal was fired" sends the reader to the wrong file.
#
# ## One thing this deliberately does not assert
#
# The sender TAKES the announcement with `परमाणुविनिमयः` rather than reading and
# clearing it, so two senders arriving together cannot both claim one sleeper
# and hand out two wakeups for one sleep. Nothing here checks that: every trial
# fires exactly one signal, so no interleaving in this program can produce a
# second claimant. An assertion that cannot be made to fail is not an
# assertion, and one was removed from this file rather than left in looking
# like coverage.
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
        --स्थान "$1" "$root/spec/notification.sas" "$2" >/dev/null
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
    # NO `tail -N` HERE, DELIBERATELY. `tail -N` returns min(actual, N), so a
    # count assertion downstream of it can only ever fire on a SHORTFALL: the
    # surplus branch is unreachable, not merely untested. W-081 demonstrated
    # both halves on this very file — a print appended produced eight confident
    # failures naming a defect in सूचना that was never touched, and a print
    # PREPENDED made this check pass with its full success text on a program
    # whose output had changed. Emit every matching line; the caller counts.
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$'
}

fail=0; i=0; count_wrong=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    i=$((i + 1))
    run_at "$addr" "$tmp/n$i.elf" > "$tmp/l$i"
    n=$(wc -l < "$tmp/l$i" | tr -d ' ')
    if [ "$n" -ne 32 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 32 (five for the set,"
        echo "        three for the whole word, then six for each of four points)"
        if [ "$n" -gt 32 ]; then
            echo "        $((n - 32)) MORE than expected — a surplus, which the"
            echo "        \`tail -32\` that used to be here could not see at all."
        fi
        fail=1; count_wrong=1
    fi
done
# The line count is about the PROGRAM, not about सूचना. Blaming the notification
# word for a wrong count sends the reader to the atomics — which is precisely
# what happened in W-081, where a surplus print produced eight confident
# failures and the summary "सूचना drops a signal" about code nobody had touched.
if [ "$count_wrong" -ne 0 ]; then
    echo
    echo "The program did not print the expected number of lines, so nothing"
    echo "below was compared. This is a fault in the PROGRAM or in this check's"
    echo "expectation — it says nothing about whether सूचना drops a signal."
    exit 1
fi
[ "$fail" -eq 0 ] || { echo; echo "सूचना drops a signal."; exit 1; }

# What moves must move, and what is architectural must not. The image really is
# built twice at two addresses — if it were not, running twice would prove
# nothing — and not one of the thirty-two words may depend on where it was
# built. This comes BEFORE the properties on purpose: a word that changes with
# the link address is an address, and reading a property off an address is a
# wrong answer arrived at confidently.
if cmp -s "$tmp/n1.elf" "$tmp/n2.elf"; then
    echo "  FAIL  the two link addresses produced identical images, so the program"
    echo "        never moved and running it twice asserted nothing"
    fail=1
elif ! cmp -s "$tmp/l1" "$tmp/l2"; then
    echo "  FAIL  the two link addresses disagree about what सूचना does, so some word"
    echo "        below is an address and not a signal:"
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

# ---- a set, not a counter ------------------------------------------------
# One bit signalled three times. The previous word says whether the signal was
# new or already standing — that answer is what replaces a queue.
BIT = 1
before, took, again = v[0:3], v[3], v[4]
if before != [0, BIT, BIT]:
    bad.append("signalling bit {:#x} three times into an empty word reported previous "
               "words {}, expected 0x0 then {:#x} twice — the previous word is where the "
               "caller learns 'this was already pending', and there is no queue for it "
               "to learn that from"
               .format(BIT, ", ".join(f"{b:#x}" for b in before), BIT))
if took != BIT:
    extra = " — the word COUNTED the signals instead of setting a bit" if took > BIT else ""
    bad.append(f"three signals of bit {BIT:#x} came back from one wait as {took:#x}, "
               f"expected {BIT:#x}{extra}. सूचना is a set: signalling twice must not "
               "queue twice")
if again != 0:
    bad.append(f"a second wait returned {again:#x} after the word had been taken — "
               "something was queued behind the first signal")

# ---- the wait takes the whole word ---------------------------------------
# Three different bits from three senders. ONE wait returns all three, and the
# word is empty afterwards. Nothing is lost between the read and the clear,
# because the read and the clear are the same instruction.
ALL = 1 | 2 | 8
whole, standing, second = v[5], v[6], v[7]
if whole != ALL:
    missing = ALL & ~whole
    bad.append(f"one wait returned {whole:#x} of the three signals {ALL:#x}"
               + (f", missing {missing:#x} — a signal was dropped between the read and "
                  "the clear" if missing else " — it returned bits nobody sent"))
if standing != 0:
    bad.append(f"the word still holds {standing:#x} after a wait took it: the wait read "
               "without clearing, so the next waiter is handed the same signal again")
if second != 0:
    bad.append(f"a second wait returned {second:#x} from a word that had just been "
               "emptied")

# ---- a signal between the check and the sleep ----------------------------
# Per trial: the point, how many points the wait actually passed, what the wait
# had in hand, what stands in the word, how many wakeups were delivered, and
# whether the waiter went to sleep.
ARRIVED = 4
POINTS = 4
trials = [v[8 + 6 * k:14 + 6 * k] for k in range(4)]
seen = set()
for k, (point, passed, hand, left, woke, slept) in enumerate(trials):
    if point != k:
        bad.append(f"trial {k} reported point {point}: the trials are not the four "
                   "points in order, so some point was never exercised")
        continue
    # The apparatus before the claim. A trial whose points did not start at
    # zero fires its signal nowhere, and every assertion below would then be
    # reading an interleaving that was never constructed.
    if passed != POINTS:
        bad.append(f"trial {k}: the wait passed {passed} points, expected {POINTS}. The "
                   "point counter did not start this trial at zero, so the signal for "
                   "this trial was never fired at all — nothing here is evidence about "
                   "a lost signal")
        continue
    seen.add((hand & ARRIVED, left & ARRIVED))
    if slept not in (0, 1):
        bad.append(f"point {k}: slept is {slept:#x}, which is neither 0 nor 1")
    elif slept != (hand == 0):
        bad.append(f"point {k}: the waiter {'slept' if slept else 'stayed awake'} with "
                   f"{hand:#x} in hand — the decision to sleep must come from what the "
                   "take returned and from nothing else")
    inhand, inword = bool(hand & ARRIVED), bool(left & ARRIVED)
    if inhand and inword:
        bad.append(f"point {k}: signal {ARRIVED:#x} is BOTH in the waiter's hand and "
                   "standing in the word — one signal was delivered twice")
    elif not inhand and not inword:
        bad.append(f"point {k}: signal {ARRIVED:#x} is in neither the waiter's hand nor "
                   "the word. It was sent and it is gone — the take cleared it away "
                   "without ever returning it")
    # The rule. Everything above is bookkeeping; this is the claim.
    if slept and left != 0 and woke == 0:
        bad.append(f"point {k}: the waiter went to sleep with {left:#x} standing in the "
                   "word and no wakeup delivered. The signal arrived between the check "
                   "and the sleep, found nobody announced, and woke nobody. Nothing that "
                   "has already happened will wake this waiter — announce BEFORE checking, "
                   "not after")

# The trials only mean something if the injected signal actually landed at
# different instants. If every point produced the same outcome, the enumeration
# is decorative and proves nothing.
if len(seen) == 1:
    bad.append("all four points produced the same outcome, so the injected signal never "
               "moved relative to the wait — the interleaving was not constructed and "
               "these trials assert nothing")

if bad:
    print("\n".join(bad)); sys.exit(1)
sleepers = sum(t[5] for t in trials)
print(f"a set not a counter ({took:#x} from three signals of one bit); one wait took all "
      f"of {whole:#x} and left {standing:#x}; and at all four points the signal was "
      f"delivered exactly once, with {sleepers} of 4 waiters sleeping and every one of "
      "them holding a wakeup")
PY
    then sed 's/^/  FAIL  /' "$tmp/v$i"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v$i")"
done

[ "$fail" -eq 0 ] || { echo; echo "सूचना drops a signal."; exit 1; }
echo
echo "ok  signalling twice does not queue twice, a wait takes the whole word in"
echo "    one instruction, and a signal arriving at any of the four points"
echo "    between the check and the sleep is delivered exactly once — the"
echo "    interleaving a design that announces AFTER checking survives three"
echo "    times out of four"
