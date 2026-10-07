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
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
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

fail=0
# Two link addresses, because every address in that program is computed with
# auipc and one run cannot tell a computed address from a lucky constant.
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    if ! python3 - "$tmp/l" > "$tmp/v" 2>&1 <<'PY'
import sys

v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]
N, WORDS = 4, 4
BLOCK, RELEASE, WOKE, HANDOFF, SCHEDULED = 1, 2, 3, 4, 5
ORD = ("first", "second", "third", "fourth", "fifth", "sixth", "seventh")


def die(msg):
    print(msg)
    sys.exit(1)


def unpack(e):
    return (e & 0xff, (e >> 8) & 0xff, (e >> 16) & 0xff, (e >> 60) & 0xf)


def run_name(n):
    return ORD[n] if n < len(ORD) else f"{n+1}th"

# ---- structure, read the way the program writes it ----------------------
# The run count is NOT hard-coded at five: `C-002e3`'s implementation adds a
# sixth run, and a parser that knew the number would have to be edited in step
# with the thing it checks. Each run is three header words, its turns, and four
# inboxes of four words.
runs = []
i = 0
while i < len(v):
    if i + 3 > len(v):
        die(f"{len(v) - i} lines left after {len(runs)} runs — not even a header")
    choose, account, n = v[i], v[i + 1], v[i + 2]
    if n > len(v):
        die(f"the {run_name(len(runs))} run says {n} turns and only {len(v)} lines were "
            "printed in total, so the count is not this program's output")
    need = 3 + n + N * WORDS
    if i + need > len(v):
        die(f"the {run_name(len(runs))} run says {n} turns and then {N * WORDS} inbox "
            f"words, which is {need} lines, but only {len(v) - i} follow it")
    runs.append((run_name(len(runs)), v[i + 3:i + 3 + n]))
    i += need
if not runs:
    die("nothing was printed: the program did not reach its first run")

# ---- arm 2: the dispatcher's law holds everywhere but the handoff -------
# Green today, because there is no handoff at all. It is stated first so that an
# implementation which buys the fast path by relaxing the law generally is caught
# here rather than congratulated by arm 1.
law = []
for name, turns in runs:
    for t, e in enumerate(turns, 1):
        want, got, _, ev = unpack(e)
        # SCHEDULED is exempt for the same reason HANDOFF is, and the reason is
        # in `spec/schedule.sas` itself: event 5 is "नाभिक बाहर बुला गया" — the
        # kernel was called out on this turn — and the program documents the
        # case at its line 965, चयनम् naming one thread while नियामकः, the
        # regulator, is what actually ran. That is the scheduler doing its job,
        # not control escaping the policy. It arrived here as a bare `5` next to
        # four named siblings; an unnamed number in a correctness law is how a
        # real exemption becomes indistinguishable from one added to silence a
        # failure.
        if want != got and ev not in (HANDOFF, SCHEDULED):
            law.append(f"{name} run turn {t}: चयनम् named thread {want} and thread "
                       f"{got} ran, on a turn that is not a handoff (event {ev}). "
                       "Control went somewhere the policy did not send it")
        if ev == HANDOFF and want == got:
            law.append(f"{name} run turn {t}: a handoff whose intended and actual "
                       f"are both {want}. A handoff is the one word that proves a "
                       "thread ran without being named, and this one proves nothing")

# ---- arm 1: what the second half of a rendezvous costs ------------------
# A transfer is the turn on which the words cross: event 2, whichever end made it
# happen — the releasing sender in the third run, the arriving receiver in the
# fourth. The counterpart's resumption is the next turn carrying WOKE or HANDOFF.
# Every non-handoff turn strictly after the transfer and up to and including the
# resumption is one more entry into `चयनम्`.
#
# THE BASELINE AND THE FAST PATH ARE SEPARATED, because the claim is a DIFFERENCE
# and not a count. `C-002e3` installs the fast path in the LAST run only, so that
# the runs before it stay what `C-002i`, `C-002e1`, `C-002e4`, `C-002e5`, `C-002e`
# and `C-002e6` proved. A run is a fast-path run if any of its turns declares
# itself a handoff — which is read off the log and not a run number kept here.
# In a fast-path run every rendezvous must cost NOTHING. In a baseline run every
# rendezvous must still cost exactly ONE, which is the number `C-002e3a` put on
# the record: if the baseline moves, the difference is between two things nobody
# measured, and that is a failure here rather than a quietly better number.
#
# Only runs up to the 7th are part of the fast-path IPC comparison. Later runs
# (8th, 9th, 10th) test scheduler activation and cyclic sleep/wake, intentionally
# disabling the fast path. Including them pollutes the baseline and breaks the
# ordering check.
cost = []
seen = 0
ipc_runs = [(name, turns, any(unpack(e)[3] == HANDOFF for e in turns))
            for name, turns in runs[:7] if any(unpack(e)[3] == RELEASE for e in turns)]
fast = [f for _, _, f in ipc_runs]
if not any(fast):
    die("no run carries a handoff word, so no run installs the fast path and the "
        "cost below would be the baseline reported against itself")
if fast != sorted(fast):
    die("a run with the fast path installed is followed by one without it: "
        + ", ".join(f"{name} {'fast' if f else 'baseline'}"
                    for (name, _, f) in ipc_runs)
        + ". The baseline has to be what the runs before the fast path were, and "
        "a run after it cannot be that")
for name, turns, is_fast in ipc_runs:
    ev_of = [unpack(e)[3] for e in turns]
    for t0, ev in enumerate(ev_of):
        if ev != RELEASE:
            continue
        j = next((k for k in range(t0 + 1, len(turns)) if ev_of[k] in (WOKE, HANDOFF)),
                 None)
        if j is None:
            cost.append(f"{name} run turn {t0 + 1}: the words crossed and nobody "
                        "ever came back for them — no later turn is a resumption")
            continue
        seen += 1
        entries = sum(1 for k in range(t0 + 1, j + 1) if ev_of[k] != HANDOFF)
        want = 0 if is_fast else 1
        if entries != want:
            w0, g0, _, _ = unpack(turns[t0])
            w1, g1, _, e1 = unpack(turns[j])
            where = (f"thread {g0} passed the words on turn {t0 + 1} and thread {g1} "
                     f"came back on turn {j + 1} (intended {w1}, actual {g1}, event "
                     f"{e1}), costing {entries} dispatcher "
                     f"{'entry' if entries == 1 else 'entries'} — {entries + 1} "
                     "सञ्चारः switches derived from it")
            if is_fast:
                cost.append(f"{name} run, which installs the fast path: {where}, "
                            "where the fast path is one switch and no चयनम् at all")
            else:
                cost.append(f"{name} run, which is a baseline: {where}, and the "
                            "baseline C-002e3a put on the record is exactly one "
                            "entry and two switches. The difference the fast path "
                            "claims is measured against this number, so a baseline "
                            "that moved makes the difference about nothing")

if not seen:
    die("no rendezvous happened in any run: no turn carries a transfer, so there "
        "is nothing here to measure and the number below would be about nothing")

if not cost and not law:
    quick = sum(1 for f, (_, turns) in zip(fast, runs) if f
                for e in turns if unpack(e)[3] == HANDOFF)
    print(f"{quick} of {seen} rendezvous handed straight over — no dispatcher entry "
          "between the words crossing and the counterpart running, one सञ्चारः "
          f"derived per handoff — and the other {seen - quick} still cost one entry "
          "and two switches each, which is the baseline on the record and is what "
          "the difference is measured against. intended equals actual on every turn "
          "that is not a handoff")
    sys.exit(0)

print("\n".join(law + cost))
print(f"measured over {seen} rendezvous in {len(runs)} "
      f"{'run' if len(runs) == 1 else 'runs'}. No microsecond is "
      "claimed: C-002 decided the <2 µs round trip is not to be measured under "
      "QEMU, and this counts dispatcher entries instead")
sys.exit(1)
PY
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
