#!/bin/bash
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"  # the verdict function below uses bash arithmetic and arrays
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
    "$root/tools/sadhana-native.sh" \
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

# verdict_notification FILE: 32 hex lines (set: 5, whole word: 3, then 6 per point x4); diagnostics on stdout, rc 1 on failure.
hx() { printf '0x%x' "$1"; }
verdict_notification() {
    local v=() l i=0 M=$((1<<63)) BIT=1 ALL=11 ARRIVED=4 POINTS=4 bad=() k b s extra missing seen="" nseen=0
    local took again whole standing second point passed hand left woke slept inhand inword sleepers=0 key
    while IFS= read -r l || [ -n "$l" ]; do
        [ -n "${l//[[:space:]]/}" ] || continue
        v[i]=$((16#${l//[[:space:]]/})); i=$((i+1))
    done < "$1"
    took=${v[3]}; again=${v[4]}
    if [ "${v[0]}" -ne 0 ] || [ "${v[1]}" -ne $BIT ] || [ "${v[2]}" -ne $BIT ]; then
        bad+=("signalling bit $(hx $BIT) three times into an empty word reported previous words $(hx "${v[0]}"), $(hx "${v[1]}"), $(hx "${v[2]}"), expected 0x0 then $(hx $BIT) twice — the previous word is where the caller learns 'this was already pending', and there is no queue for it to learn that from")
    fi
    if [ "$took" -ne $BIT ]; then
        extra=""
        if [ $((took ^ M)) -gt $((BIT ^ M)) ]; then extra=" — the word COUNTED the signals instead of setting a bit"; fi
        bad+=("three signals of bit $(hx $BIT) came back from one wait as $(hx "$took"), expected $(hx $BIT)$extra. सूचना is a set: signalling twice must not queue twice")
    fi
    if [ "$again" -ne 0 ]; then
        bad+=("a second wait returned $(hx "$again") after the word had been taken — something was queued behind the first signal")
    fi
    whole=${v[5]}; standing=${v[6]}; second=${v[7]}
    if [ "$whole" -ne $ALL ]; then
        missing=$((ALL & ~whole))
        s="one wait returned $(hx "$whole") of the three signals $(hx $ALL)"
        if [ "$missing" -ne 0 ]; then
            s="$s, missing $(hx "$missing") — a signal was dropped between the read and the clear"
        else
            s="$s — it returned bits nobody sent"
        fi
        bad+=("$s")
    fi
    if [ "$standing" -ne 0 ]; then
        bad+=("the word still holds $(hx "$standing") after a wait took it: the wait read without clearing, so the next waiter is handed the same signal again")
    fi
    if [ "$second" -ne 0 ]; then
        bad+=("a second wait returned $(hx "$second") from a word that had just been emptied")
    fi
    for k in 0 1 2 3; do
        b=$((8 + 6*k))
        point=${v[b]}; passed=${v[b+1]}; hand=${v[b+2]}; left=${v[b+3]}; woke=${v[b+4]}; slept=${v[b+5]}
        sleepers=$((sleepers + slept))
        if [ "$point" -ne "$k" ]; then
            bad+=("trial $k reported point $(printf %u "$point"): the trials are not the four points in order, so some point was never exercised")
            continue
        fi
        if [ "$passed" -ne $POINTS ]; then
            bad+=("trial $k: the wait passed $(printf %u "$passed") points, expected $POINTS. The point counter did not start this trial at zero, so the signal for this trial was never fired at all — nothing here is evidence about a lost signal")
            continue
        fi
        key="$((hand & ARRIVED)),$((left & ARRIVED))"
        case " $seen " in *" $key "*) ;; *) seen="$seen $key"; nseen=$((nseen+1));; esac
        if [ "$slept" -ne 0 ] && [ "$slept" -ne 1 ]; then
            bad+=("point $k: slept is $(hx "$slept"), which is neither 0 nor 1")
        elif { [ "$slept" -eq 1 ] && [ "$hand" -ne 0 ]; } || { [ "$slept" -eq 0 ] && [ "$hand" -eq 0 ]; }; then
            if [ "$slept" -eq 1 ]; then s=slept; else s="stayed awake"; fi
            bad+=("point $k: the waiter $s with $(hx "$hand") in hand — the decision to sleep must come from what the take returned and from nothing else")
        fi
        inhand=0; inword=0
        [ $((hand & ARRIVED)) -ne 0 ] && inhand=1
        [ $((left & ARRIVED)) -ne 0 ] && inword=1
        if [ $inhand = 1 ] && [ $inword = 1 ]; then
            bad+=("point $k: signal $(hx $ARRIVED) is BOTH in the waiter's hand and standing in the word — one signal was delivered twice")
        elif [ $inhand = 0 ] && [ $inword = 0 ]; then
            bad+=("point $k: signal $(hx $ARRIVED) is in neither the waiter's hand nor the word. It was sent and it is gone — the take cleared it away without ever returning it")
        fi
        if [ "$slept" -ne 0 ] && [ "$left" -ne 0 ] && [ "$woke" -eq 0 ]; then
            bad+=("point $k: the waiter went to sleep with $(hx "$left") standing in the word and no wakeup delivered. The signal arrived between the check and the sleep, found nobody announced, and woke nobody. Nothing that has already happened will wake this waiter — announce BEFORE checking, not after")
        fi
    done
    if [ "$nseen" -eq 1 ]; then
        bad+=("all four points produced the same outcome, so the injected signal never moved relative to the wait — the interleaving was not constructed and these trials assert nothing")
    fi
    if [ "${#bad[@]}" -gt 0 ]; then printf '%s\n' "${bad[@]}"; return 1; fi
    printf 'a set not a counter (%s from three signals of one bit); one wait took all of %s and left %s; and at all four points the signal was delivered exactly once, with %d of 4 waiters sleeping and every one of them holding a wakeup\n' "$(hx "$took")" "$(hx "$whole")" "$(hx "$standing")" "$sleepers"
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
    if ! verdict_notification "$tmp/l$i" > "$tmp/v$i" 2>&1
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
