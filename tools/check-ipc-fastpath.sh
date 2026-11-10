#!/bin/sh
# THE COST OF AN IPC ROUND TRIP — `C-002e3a` wrote it red, `C-002e3` turned it
# green, doc 11 §7. It was written BEFORE the fast path existed for the reason
# `C-002e2a` was: a check that passes against today's program is testing nothing.
#
# ## What the fast path is, and what the rendezvous costs without it
#
# `C-002e` finished द्वारम् in both directions, and in both directions the
# rendezvous crossed the DISPATCHER. Read off the log of `spec/schedule.sas`:
#
#   third run   turn 10  thread 0 hands three words to the waiting thread 2
#               turn 11  thread 2 runs — and चयनम् NAMED it (intended 2, actual 2)
#   fourth run  turn 10  thread 3 takes the parked sender's words
#               turn 11  thread 0 comes back — again as चयनम्'s named choice
#
# So the second half of that round trip costs ONE MORE DISPATCHER ENTRY: the
# transferring thread switches out, `चयनम्` is entered again, and the woken
# thread is switched in. `C-002e3`'s claim is that it should cost NONE — the
# transferring thread switches DIRECTLY into its counterpart, one `सञ्चारः`, no
# `चयनम्` between them.
#
# `C-002e3` installs that in the SIXTH run and only there, so the runs before it
# stay word for word what `C-002i`, `C-002e1`, `C-002e4`, `C-002e5`, `C-002e` and
# `C-002e6` proved. Those earlier runs are therefore not a regression when they
# still cost one entry — they are the BASELINE, and this check demands they keep
# costing exactly that. The claim is a difference, and a difference measured
# against a baseline that moved is about nothing.
#
# ## The number, and the number this does NOT produce
#
# What is measured here is DISPATCHER ENTRIES, which is what the log shows: one
# word per iteration of `प्रेषणचक्रम्`, and `चयनम्` is called exactly once in
# that loop. Switches are DERIVED from entries and not counted independently —
# each entry is one `सञ्चारः` in and one out — and the derivation is printed
# beside the count so a reader can see which of the two was observed.
#
# No microsecond appears here, and none may be added. `C-002` decided before
# anyone wrote a harness that the <2 µs claim is not to be measured under QEMU:
# QEMU is not cycle-accurate and `rdtime` is not the host clock, so a number
# produced there would be meaningless and official-looking at once. The round
# trip is bounded instead by a count QEMU reports honestly. The wall-clock claim
# stays EXPLICITLY OUTSTANDING and needs `W-012b`'s host, which does not exist.
#
# ## The two arms, and the one this check does not own
#
# ARM 1 — THE COST. In the run that installs the fast path, there must be NO
# dispatcher entry between the turn on which the words cross and the turn on
# which the counterpart resumes. In every other run there must be exactly ONE,
# which is the number `C-002e3a` put on the record before the implementation
# existed. Which run is which is read off the log — a handoff declares itself
# with event 4 — and not from a run number kept in this file. The failure names
# the pair of turns rather than reporting a number.
#
# ARM 2 — INTENDED EQUALS ACTUAL EVERYWHERE ELSE, and it must stay
# green. A fast-path turn is the one place the dispatcher's law is relaxed: the
# handoff word carries चयनम्'s choice as INTENDED and the thread that actually
# ran as ACTUAL, and they differ — that difference is the only evidence in the
# log that a thread ran without being named. Which is exactly why the relaxation
# must not be general: every word that is not a handoff must still show the two
# equal, or `C-002i`'s whole "control went where the policy sent it" claim is
# weakened to buy latency. This arm can be violated by an implementation that
# takes the shortcut everywhere, and it fires if one does.
#
# FAIRNESS IS NOT THIS CHECK'S. `गणनम्` must still be called on the fast path or
# the round-robin timestamps go stale, and a fast path that skips both halves of
# the vector PASSES A LATENCY TEST AND SILENTLY BREAKS FAIRNESS. The instrument
# for that already exists and is not duplicated here: `चक्रम्` decides by
# last-run alone, so a receiver whose `गणनम्` was skipped is picked again
# immediately and `tools/check-schedule.sh`'s round-robin pass breaks. That check
# must be green with the fast path taken. This one says nothing about fairness
# and must not be read as if it did.
#
# ## Absence is loud
#
# `W-087` found 27 `check-*.sh` that exit 0 when their tooling is missing, so a
# machine with nothing installed reports a green tree. This one exits **77** when
# it cannot run and **1** when it runs and fails. It never exits 0 without having
# watched a machine.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
# NOT in $root: the gate fingerprints modified and untracked files, so a scratch
# file dropped in the repo is a file that exists while the gate is looking.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "CANNOT RUN: qemu-system-riscv64 is not installed, so nothing was measured"
  exit 77; }

run_at() {
    "$root/tools/sadhana-native.sh" \
        --स्थान "$1" "$root/spec/schedule.sas" "$tmp/s.elf" >/dev/null
    # Bounded by an explicit SIGKILL, not perl's alarm: QEMU handles SIGALRM and
    # outlives it (`W-060`). A scheduler that never returns -1 runs forever.
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/s.elf" > "$tmp/out" 2>&1 &
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
    # No `tail -N`: it returns min(actual, N), which makes a count assertion
    # one-sided by construction and blind to a surplus (`W-081`). The number of
    # turns is part of what is measured here.
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' || true
}

# ipc_verdict FILE: the arms of the IPC check over the printed words (16 lowercase hex digits each).
# Fields are cut out of the hex text: want = digits 15-16, got = 13-14, event = digit 1; only the
# run's turn count is a number, and it is converted exactly (decimal by digit arithmetic) for messages.
ipc_verdict() {
    LC_ALL=C awk '
    function h2d(s,  i, v) { v = 0; for (i = 1; i <= length(s); i++) v = v * 16 + index("0123456789abcdef", substr(s, i, 1)) - 1; return v }
    function h2dec(s,  i, j, d, c, t, n, r) {   # exact decimal of a hex string
        n = 1; dg[1] = 0
        for (i = 1; i <= length(s); i++) {
            c = index("0123456789abcdef", substr(s, i, 1)) - 1
            for (j = 1; j <= n; j++) { t = dg[j] * 16 + c; dg[j] = t % 10; c = int(t / 10) }
            while (c > 0) { dg[++n] = c % 10; c = int(c / 10) }
        }
        r = ""; for (j = n; j >= 1; j--) r = r dg[j]
        return r
    }
    function runname(n) { return (n < 7) ? ORD[n + 1] : (n + 1) "th" }
    function ev(e) { return h2d(substr(e, 1, 1)) }
    function want(e) { return h2d(substr(e, 15, 2)) }
    function got(e) { return h2d(substr(e, 13, 2)) }
    function die(m) { print m; dead = 1; exit 1 }
    NF { v[++nv] = $1 "" }
    END {
        if (dead) exit 1
        split("first second third fourth fifth sixth seventh", ORD, " ")
        N = 4; WORDS = 4; RELEASE = 2; WOKE = 3; HANDOFF = 4; SCHEDULED = 5
        nruns = 0; i = 0
        while (i < nv) {
            if (i + 3 > nv) die((nv - i) " lines left after " nruns " runs \342\200\224 not even a header")
            nhex = v[i + 3]; n = h2d(nhex)
            if (n > nv) die("the " runname(nruns) " run says " h2dec(nhex) " turns and only " nv " lines were printed in total, so the count is not this program\047s output")
            need = 3 + n + N * WORDS
            if (i + need > nv) die("the " runname(nruns) " run says " n " turns and then " N * WORDS " inbox words, which is " need " lines, but only " (nv - i) " follow it")
            nruns++; rname[nruns] = runname(nruns - 1); rs[nruns] = i + 3; rn[nruns] = n
            i += need
        }
        if (nruns == 0) die("nothing was printed: the program did not reach its first run")
        nlaw = 0; ncost = 0
        for (r = 1; r <= nruns; r++)
            for (t = 1; t <= rn[r]; t++) {
                e = v[rs[r] + t]; w = want(e); g = got(e); x = ev(e)
                if (w != g && x != HANDOFF && x != SCHEDULED)
                    msg[++nlaw] = rname[r] " run turn " t ": \340\244\232\340\244\257\340\244\250\340\244\256\340\245\215 named thread " w " and thread " g " ran, on a turn that is not a handoff (event " x "). Control went somewhere the policy did not send it"
                if (x == HANDOFF && w == g)
                    msg[++nlaw] = rname[r] " run turn " t ": a handoff whose intended and actual are both " w ". A handoff is the one word that proves a thread ran without being named, and this one proves nothing"
            }
        seen = 0; ni = 0; anyfast = 0
        lim = (nruns < 7) ? nruns : 7
        for (r = 1; r <= lim; r++) {
            hasrel = 0; hf = 0
            for (t = 1; t <= rn[r]; t++) { x = ev(v[rs[r] + t]); if (x == RELEASE) hasrel = 1; if (x == HANDOFF) hf = 1 }
            if (hasrel) { ni++; ir[ni] = r; ifast[ni] = hf; if (hf) anyfast = 1 }
        }
        if (!anyfast) die("no run carries a handoff word, so no run installs the fast path and the cost below would be the baseline reported against itself")
        badord = 0; for (a = 1; a < ni; a++) if (ifast[a] && !ifast[a + 1]) badord = 1
        if (badord) {
            s = ""; for (a = 1; a <= ni; a++) s = s (a > 1 ? ", " : "") rname[ir[a]] " " (ifast[a] ? "fast" : "baseline")
            die("a run with the fast path installed is followed by one without it: " s ". The baseline has to be what the runs before the fast path were, and a run after it cannot be that")
        }
        for (a = 1; a <= ni; a++) {
            r = ir[a]; nt = rn[r]; isf = ifast[a]
            for (t = 1; t <= nt; t++) E[t] = ev(v[rs[r] + t])
            for (t0 = 1; t0 <= nt; t0++) {
                if (E[t0] != RELEASE) continue
                j = 0; for (k = t0 + 1; k <= nt; k++) if (E[k] == WOKE || E[k] == HANDOFF) { j = k; break }
                if (!j) { cmsg[++ncost] = rname[r] " run turn " t0 ": the words crossed and nobody ever came back for them \342\200\224 no later turn is a resumption"; continue }
                seen++
                entries = 0; for (k = t0 + 1; k <= j; k++) if (E[k] != HANDOFF) entries++
                wn = isf ? 0 : 1
                if (entries != wn) {
                    e0 = v[rs[r] + t0]; e1 = v[rs[r] + j]
                    where = "thread " got(e0) " passed the words on turn " t0 " and thread " got(e1) " came back on turn " j " (intended " want(e1) ", actual " got(e1) ", event " ev(e1) "), costing " entries " dispatcher " (entries == 1 ? "entry" : "entries") " \342\200\224 " entries + 1 " \340\244\270\340\244\236\340\245\215\340\244\232\340\244\276\340\244\260\340\244\203 switches derived from it"
                    if (isf) cmsg[++ncost] = rname[r] " run, which installs the fast path: " where ", where the fast path is one switch and no \340\244\232\340\244\257\340\244\250\340\244\256\340\245\215 at all"
                    else cmsg[++ncost] = rname[r] " run, which is a baseline: " where ", and the baseline C-002e3a put on the record is exactly one entry and two switches. The difference the fast path claims is measured against this number, so a baseline that moved makes the difference about nothing"
                }
            }
        }
        if (!seen) die("no rendezvous happened in any run: no turn carries a transfer, so there is nothing here to measure and the number below would be about nothing")
        if (ncost == 0 && nlaw == 0) {
            quick = 0
            for (a = 1; a <= ni; a++) if (ifast[a]) for (t = 1; t <= rn[a]; t++) if (ev(v[rs[a] + t]) == HANDOFF) quick++
            print quick " of " seen " rendezvous handed straight over \342\200\224 no dispatcher entry between the words crossing and the counterpart running, one \340\244\270\340\244\236\340\245\215\340\244\232\340\244\276\340\244\260\340\244\203 derived per handoff \342\200\224 and the other " seen - quick " still cost one entry and two switches each, which is the baseline on the record and is what the difference is measured against. intended equals actual on every turn that is not a handoff"
            exit 0
        }
        for (a = 1; a <= nlaw; a++) print msg[a]
        for (a = 1; a <= ncost; a++) print cmsg[a]
        print "measured over " seen " rendezvous in " nruns " " (nruns == 1 ? "run" : "runs") ". No microsecond is claimed: C-002 decided the <2 \302\265s round trip is not to be measured under QEMU, and this counts dispatcher entries instead"
        exit 1
    }' "$1"
}

fail=0
# Two link addresses, because every address in that program is computed with
# auipc and one run cannot tell a computed address from a lucky constant.
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    if ! ipc_verdict "$tmp/l" > "$tmp/v" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
done

[ "$fail" -eq 0 ] || {
    echo
    echo "The IPC round trip is not what C-002e3 landed. Either a rendezvous in"
    echo "the run that installs the fast path still crosses the dispatcher, or a"
    echo "BASELINE run no longer costs the one entry C-002e3a put on the record —"
    echo "and a difference measured against a moved baseline is about nothing."
    exit 1; }

echo
echo "ok  at both link addresses: every rendezvous in the run that installs the"
echo "    fast path handed straight over, and every rendezvous in a baseline run"
echo "    still costs the one dispatcher entry C-002e3a put on the record. The"
echo "    difference is the fast path and it is measured against a baseline that"
echo "    did not move. This says nothing about fairness: गणनम् being called on"
echo "    the fast path is tools/check-schedule.sh's round-robin pass, which must"
echo "    be green too."
