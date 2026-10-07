#!/bin/sh
# The scheduler, and the seam it can be replaced through — `C-002i`, doc 11 §3.4.
#
# Doc 11 §3.4 asks for three things at once: priority round-robin, a simple MLFQ
# for interactivity, and — the heaviest — an interface designed so a better
# policy can be dropped in later.
#
# The third is harder than the first two, because "it works" is not an answer to
# it. A scheduler that picks correctly but cannot be replaced fails the stated
# requirement, and no single run can show the difference. So the seam is tied to
# evidence here: two policies, ONE entry point, and not one instruction of
# difference in the dispatcher. Installing a policy means writing two words. The
# proof is that doing so CHANGES THE DECISIONS — the two traces differ. If they
# were the same, the seam would be decorative and the dispatcher would not be
# going through it at all.
#
# ## The interface
#
#     चयनम्(set)     -> which thread of `set` to run, or -1 if `set` is empty
#     गणनम्(i, used) -> i ran and ate `used` units; the policy updates itself
#
# That is all of it. Levels, priority, demotion — all inside the policy. The
# dispatcher lifts both out of `नीतिः` and calls through `jalr`, so it does not
# know which policy is installed. The two addresses it will actually call are
# READ BACK OUT of that vector and printed, so what is checked is what runs.
#
# ## Two policies
#
#   श्रेणी  priority + round-robin + MLFQ. Lowest level wins; among equals the
#           thread that ran longest ago wins; eating the whole slice demotes,
#           eating less promotes.
#   चक्रम्  round-robin alone. Levels are never read and `गणनम्` does nothing.
#
# ## The threads really run
#
# This is not a simulation. `C-002d` proved a twelve-register context switch;
# here that same switch gains one word — `ra` — and becomes a coroutine switch.
# Each thread writes its own identity into `वर्तमानः` when it runs, and BOTH the
# thread the policy named and the thread that actually wrote are recorded. Their
# being equal is what separates a context switch from a bookkeeping exercise.
#
# `sp` was deliberately not added to the context: nothing here touches a stack,
# and a slot that can never be wrong is armour-shaped and is not armour.
#
# ## The workload is not the policy
#
# The slice is two units for everyone — a property of the machine, and it lives
# inside the thread. A thread eats the smallest of its appetite, the slice, and
# its remaining work. Appetite is the thread's nature; the policy never reads it
# and sees only how much was eaten. That is why the interactivity check is not
# circular: what is proved is the policy's decision, and the policy's only
# evidence about interactivity is observed behaviour.
#
#   threads 0 1 2 — level 0, work 10, appetite 8 (bigger than the slice)
#   thread  3     — level 2, work 4,  appetite 1 (starts last, eats little)
#
# ## The log, and why the check needs no model of the policy
#
#   bits  0.. 7  thread the policy named
#   bits  8..15  thread that actually ran
#   bits 16..23  units eaten
#   bits 24..39  all four levels, four bits each
#   bits 40..55  all four remaining works, four bits each
#   bits 56..59  the set the policy was OFFERED — who could run at all
#
# Levels and works are AS OF THE MOMENT OF CHOICE. So the check can see, for
# itself, who was runnable and whose level was lowest, without replaying the
# policy. A check that re-implements the thing it checks can make the same
# mistake in both places, and this one does not have to.
#
# ## रोधः — runnability is the dispatcher's, not the policy's (`C-002e1`)
#
# Both policies used to read `+112` themselves to see whether a thread had work
# left. Adding blocking would have meant the same edit in both, and that is the
# argument that the line was in the wrong place: whether a thread CAN run is a
# property of the thread, and only who SHOULD run belongs to the policy. So the
# dispatcher now walks the four threads, builds a four-bit set, and hands it to
# `चयनम्`. A policy cannot tell a blocked thread from a finished one, and should
# not be able to.
#
# Thread 3 starts blocked with four units of work; thread 0 releases it when its
# own work falls to four. Both halves are required. Without the release, blocked
# and finished look identical from outside — the thread has work, never runs,
# the run ends — and the check says so by name rather than by a work-outstanding
# complaint that would not name the cause. The release was reverted and watched
# fire: `thread 3 was never offered on any of 15 turns`.
#
# The set is in the log, so every eligibility question here is asked of what the
# dispatcher OFFERED and not of `work > 0`. Where nothing is ever blocked — the
# other three threads — the check still demands the two agree exactly, so the
# set cannot become a second, unfalsifiable source of truth.
#
# ## द्वारम् — the endpoint and the order of the threads waiting (`C-002e4`)
#
# The two runs above never have a thread block ITSELF: the block is placed at
# startup and cleared by another thread's store, so there is no such thing as an
# ORDER over waiters to get wrong. A third run opens द्वारम्. Threads 1, 2 and 3
# sit down on one endpoint by themselves and thread 0 releases the head, one per
# turn, once all three are waiting.
#
# The order has to be falsifiable, so it is deliberately not index order: a
# per-thread countdown makes them block 2, 1, 3. FIFO is then the claim that they
# come back 2, 1, 3 — a stack would give 3, 1, 2 and a set would give 1, 2, 3, and
# both are orders this check rejects by name.
#
# THE EVIDENCE IS THE RESUMPTION, NOT THE QUEUE ARRAY. A queue nothing pops is a
# list, and an order held in memory that no behaviour depends on cannot be wrong.
# So the order is read off the turns on which a thread came back from INSIDE
# `सञ्चारः` — event 3 in the log — and never off the endpoint's own array, which
# this check does not read at all.
#
# The third run installs `चक्रम्`, which never reads a level. That is deliberate:
# what is under test is the order of a wait queue, and a policy that sorts by
# level could be the thing producing the order. It also means the MLFQ promotion
# a blocking thread earns — it eats 0 units on the turn it blocks, which is less
# than its slice — is NOT exercised here. That belongs to `C-002e`.
#
# ## The message (`C-002e5`)
#
# The releaser is now a SENDER. Each release carries four words — a fault
# package's shape, doc 11: cause, address, point, and who sent it — written into
# the receiver's SAVED CONTEXT, so the woken thread comes back holding them in
# `स्थिर२..५` and copies them into its own inbox before it reads any memory. No
# buffer, four registers, which is the shape `C-002e3`'s fast path has to carry.
#
# The three messages go out IN SEND ORDER, not by receiver, so which message a
# thread ends up with is the queue's answer to "who had waited longest" and not a
# lookup by name. A stack would hand the third message to the first waiter, and
# that shows up here as well as in the order of resumption.
#
# Every run prints four inboxes of four words after its turns. In runs one and
# two, where द्वारम् is shut, all sixteen must be zero — the same inertness
# demanded of the event field, and the reason the inboxes are cleared per run and
# not once: a word left from an earlier run reads exactly like a word just sent.
#
# Runs one and two must come out UNCHANGED, word for word, which is checked by
# this file continuing to make every assertion it made before. Their traces were
# diffed against the pre-`C-002e4` program and are identical; only the two policy
# addresses moved, because code was inserted ahead of them.
#
# ## The sender that BLOCKS (`C-002e`)
#
# Every send above found somebody already sitting on the endpoint, so the
# rendezvous was shown in one direction only: a receiver waits, a sender arrives,
# the receiver goes. A fourth run is the other direction. Thread 0 sends on its
# very first turn, when nothing in the run has ever blocked — so there is nobody
# to hand the words to, and the SENDER is the one that has to sit down. On the
# same queue, from the other end, which is what makes द्वारम् a rendezvous rather
# than a mailbox with a wakeup.
#
# The words travel in the sender's OWN saved context. It loads them into
# `स्थिर२..५` and calls `सञ्चारः`, which saves them exactly where the third run's
# sender used to write them — same four slots, opposite direction of the wire:
# there a sender wrote into the receiver's saved context, here a receiver reads
# out of the sender's.
#
# Thread 3 arrives two turns later, finds the sender waiting, copies the four
# words into its own inbox, clears the block and pops the queue — and does NOT
# block itself. That asymmetry is the claim: a receive that finds a sender
# completes in the turn it is made. Threads 1 and 2 never touch the endpoint at
# all; their running throughout is what says the parked sender did not stall the
# machine.
#
# THAT THE QUEUE WAS EMPTY when thread 0 sent is read off the events, not off the
# endpoint's array, which this check still never reads: joining the queue is the
# only thing in the program that records a BLOCK, and thread 0's block is the
# first event of the run.
#
# The run is `चक्रम्` again and starts from the same `द्वारसारणी` as the third,
# so the two runs differ by one word — `द्वारसक्रियः`, 1 against 2.
#
# ## The promotion a blocking thread earns (`C-002e6`)
#
# Both द्वारम् runs above install `चक्रम्`, which never reads a level — on
# purpose, because a policy that sorts could be the thing producing the wait
# order. The cost is that the lift a blocking thread is owed is invisible in
# them. A fifth run is the third one again with `श्रेणी` in `नीतिः` instead: one
# pair of words, the same table, the same endpoint, the same three waiters.
#
# `श्रेणी` has no notion of an interactive thread. It demotes whoever ate the
# whole slice and promotes whoever ate less, and a thread that sits down on
# द्वारम् ate nothing — so doc 11 §3.4's "keep the interactive thread in front"
# is a consequence of the MLFQ rule here rather than a rule beside it.
#
# The rise alone is not the claim. A level that moves and decides nothing is a
# number, so what is demanded is that the lifted thread is then CHOSEN over one
# standing at the level the block found it at — the same bar `C-002i` sets for
# thread 3 climbing out of the bottom, applied to a thread that climbed by
# waiting instead of by being small.
#
# ## The twin, and why a copy is worth a run (`C-002e3b`)
#
# `C-002e3`'s fast path is not a number, it is a DIFFERENCE: the second half of a
# rendezvous crosses the dispatcher today and must stop crossing it. A difference
# needs the same thing standing on both sides, so a sixth run is added BEFORE the
# fast path exists — the third run again, same `चक्रम्`, same `द्वारसारणी`, same
# `द्वारसक्रियः`, nothing else touched.
#
# `C-002e3` has since landed the fast path in that sixth run and ONLY there, so
# the arm has TIGHTENED, not been deleted: the two logs must be the same length
# and must agree word for word everywhere except the handoff words, and there
# must be at least one of those. Which positions may differ is read off the यमज
# run itself — a handoff declares itself with event 4 and an intended that differs
# from its actual — so this file keeps no list of turn numbers that would have to
# be edited in step with the thing it checks.
#
# An unchanged log is now the FAILURE it used to be the pass. Two things still
# fall out for free: `प्रतिष्ठापनम्` puts the machine back where it started even
# after five runs and a policy swap, and the diff between these two logs is the
# fast path and nothing else — every turn that is not a handoff, including every
# turn after one, is bit for bit what it was, which is what says गणनम् was called
# for BOTH threads that ran on a handoff entry and not only for the one चयनम्
# named.
#
# `C-002e3c` then adds the OTHER DIRECTION. The sixth run's rendezvous is a
# releasing sender handing straight to the waiter it just woke; the fourth run's
# is the reverse — a receiver arrives, finds a parked SENDER, takes its four
# words and clears its block — and that half still crossed the dispatcher. So a
# SEVENTH run is the fourth one again with द्रुतपथः set, and the twin arm above
# is a function applied to both pairs rather than written out twice. What the
# second pair adds is that the mechanism is direction-blind: `द्वारम्` names a
# counterpart and `मुक्त्यन्तः` reads `द्रुतपथः`, neither of them knowing which
# end of the rendezvous asked.
#
# No other arm is applied to a यमज run, and none needs to be: everything the run
# it copies proves holds of a log that agrees with it away from the handoffs, and
# asserting it twice would only make two chances to disagree with itself.
#
# ## उपकार्यम् — the upcall, written RED before the program (`C-002i2a`)
#
# Doc 11 §3.4 asks for one thing more than the seam: a latency-sensitive service
# — audio, doc 10 — must get BOUNDED scheduling WITHOUT A KERNEL CHANGE. That is
# a call from the kernel OUT to userspace, and until `C-002e` there was no
# mechanism for that direction at all. `द्वारम्` is the mechanism and `नीतिः` is
# where it attaches: a policy whose `चयनम्` DECIDES NOTHING AND INSTEAD ASKS.
#
# So an EIGHTH run — and it is written here BEFORE `spec/schedule.sas` can pass
# it, the order `C-002h2c2a` and `C-002h2c3a` stood in and for their reason: a
# contract written after the program it describes is a description, and a
# description has never refused anything. What follows is RED today. The red is
# THIS FILE'S, not the tree's — the program prints seven runs and is asked for
# eight, and `C-002i2b` is the row that owes the rest.
#
# ### The shape the eighth run has to have
#
# Thread 3 is the USERSPACE SCHEDULER, `नियामकः`. It starts blocked on the
# endpoint as a receiver — parked, waiting to be called — and it has no work of
# its own. Threads 0, 1 and 2 are ordinary work at level 0, all three equal, so
# that nothing in the order can be attributed to a level.
#
# `नीतिः` holds `उपकार्यचयनम्`/`उपकार्यगणनम्`, and the dispatcher is not touched
# by a single instruction. `उपकार्यचयनम्` takes the offered set, SENDS it through
# `द्वारम्` to the parked `नियामकः`, and returns what comes back.
#
# ### Why a rule no kernel policy implements
#
# The decision must be shown to have LEFT the kernel, and "it scheduled
# correctly" cannot show that — a kernel that computed the same answer itself
# would produce the same log. So `नियामकः` decides by a rule neither installed
# policy can express: THE HIGHEST-NUMBERED RUNNABLE THREAD WINS. `श्रेणी` sorts
# by level and then by who ran longest ago; `चक्रम्` by who ran longest ago
# alone. Neither has a notion of index order, let alone a reversed one. So every
# turn of the eighth run is checkable straight off the log — the set is in bits
# 56..59, the choice in bits 0..7 — against a rule that no line of kernel code
# contains. That is the same argument the two-policy seam makes, made once more
# where the second party is not in the image's kernel at all.
#
# ### The message is what says the call crossed
#
# An activation declares itself: EVENT 5, `उपकारः`, on a turn whose actual
# thread is `नियामकः` and whose intended thread is not — the self-declaring
# shape the handoff word already has, and for the same reason, so that this file
# keeps no list of turn numbers to be edited in step with the thing it checks.
#
# The four words that cross are `C-002e5`'s, read out of the RECEIVER's own
# registers: word 0 is the offered set the dispatcher had just built, and word 3
# is the sender's identity — which here is THE KERNEL, an identity that is not
# any of the four threads. A message whose sender is no thread is the mark of a
# call made in the other direction, and it is the one word that could not have
# been written by any party already in this program.
#
# Its limit, said out loud: an inbox is printed ONCE, at the end of the run, so
# what survives is the LAST activation's message and not all of them. The
# per-turn evidence is the event word; the inbox is what ties one of those turns
# to words that actually travelled. The other three inboxes must be zero —
# nobody but `नियामकः` is spoken to.
#
# ## मौनम् — the upcall that is never answered, written RED before the program
# (`C-002i2c1a`)
#
# The eighth run's `नियामकः` always replies. A userspace scheduler that does not
# is the `C-002h2c2c` question asked of this seam: silence is not an answer, and
# the only bound on it in the eighth run is THIS FILE'S OWN SIGKILL — the harness
# rescuing the machine rather than the machine defining an outcome. A kernel that
# hands its scheduling decision to a userspace thread has handed it somewhere
# that can stop answering, and if that is fatal then doc 11 §3.4's "without a
# kernel change" was bought by making every audio server a kernel-critical
# process.
#
# So a NINTH run, and it is written here BEFORE `spec/schedule.sas` can pass it,
# the order `C-002i2a` stood in and for its reason. What follows is RED today —
# the program prints eight runs and is asked for nine — and `C-002i2c1b` is the
# row that owes the rest.
#
# ### The one word that differs, and which side it is on
#
# The ninth run is the eighth again: the same table, `नियामकः` parked on the
# endpoint with no work of its own, threads 0, 1 and 2 equal at level 0, and THE
# SAME TWO WORDS in `नीतिः` — the third policy is re-installed, not replaced, and
# the check demands the two addresses be the pair the eighth run printed. Nothing
# in the kernel differs. What differs is one word inside `नियामकः`, which takes
# its turn and re-parks WITHOUT leaving an answer.
#
# That is deliberately which side the word is on. A kernel told in advance that
# this scheduler will be silent is a kernel that handled a case it was given; the
# claim here is that the kernel FINDS OUT, by asking and getting nothing back.
#
# ### What the log has to show, and none of it needs a model of the policy
#
# **The first question is word for word the eighth run's first question.** The
# kernel cannot yet know anything, so turn 1 of the two runs must be the same
# word — same carrier, same set, same event. The two runs diverge only after an
# answer failed to arrive, which is what says the silence and not the table is
# what changed.
#
# **Asking again IS the evidence of silence**, and it is read straight off the
# log. `उपकार्यचयनम्` consumes an answer when it uses one, so a second upcall
# turn with no work turn between it and the first can only mean the first
# question was never answered. So the run opens with `ASKS` upcall turns, back to
# back from turn 1, eating nothing.
#
# **The bound is a number in the program, not a timeout in the harness.** `ASKS`
# is 3: one ask says nothing about silence, a second is the first evidence that
# the first went unanswered, and a third is what makes it a BOUND rather than
# "ask twice". After the third the kernel gives up, and the giving-up is a turn
# that declares itself — EVENT 6, `उपकारभङ्गः` — exactly once in the run, with
# no upcall turn after it. A kernel that keeps asking a scheduler it has already
# declared dead has not defined an outcome, it has renamed the loop.
#
# **The run must end because the work ran out.** Not on the dispatcher's budget,
# and not on the SIGKILL: every thread runs, all the work is eaten, and the turn
# count is under the budget. That is the whole of what "the machine defines the
# outcome" means here, and it is the arm the eighth run cannot make about a
# silent scheduler.
#
# **After the give-up the choice is the LOWEST runnable index** — the exact
# inverse of `नियामकः`'s rule, so no fallback turn can be mistaken for an
# answered one, and it is a rule already written in this program (`चयनम्` names
# its carrier by it) rather than one invented for the fallback. The two runs'
# orders must therefore differ.
#
# ### Its limits, said out loud
#
# **A scheduler that never yields at all is out of reach**, and this file does
# not pretend otherwise. This kernel is cooperative — no clock, no interrupt — so
# a `नियामकः` that spins forever cannot be taken off the machine by anything in
# the image; what is bounded here is a scheduler that RETURNS and answers
# nothing, which is the wedged-scheduler case, not the runaway-thread case. The
# runaway needs preemption and preemption needs a timer, and that is not this
# row.
#
# **The fallback is a stand-in and not a good policy.** Lowest-runnable-index is
# chosen to be unmistakable, not to schedule well; which of the kernel's own two
# policies a real system should fall back to is a design question, and the seam
# already shows either can be written into those two words.
#
# ## अनशनम् — ordering over BLOCKED threads, written RED before the program
# (`C-002i2c2a`)
#
# Every arm above is about who was CHOSEN FROM THE OFFERED SET, and a thread
# that is not in that set is outside all of them. `starvation()` says so in its
# own body: a thread eligible at one turn and again at a later one must have run
# in between — and a thread that spent the interval BLOCKED was eligible at
# neither end, so the rule steps over it without a word. That is what `C-002i`
# could not reach: nothing could block then, and the fairness claim it left
# behind is a claim about threads that never leave.
#
# Starvation-by-blocking is the interesting case and the one a real system meets.
# A thread that blocks, is released and blocks AGAIN is what every consumer of a
# device queue does, doc 10's audio thread included. The question the nine runs
# above never ask is whether such a thread comes ROUND: whether the turn it lost
# by waiting is given back to it, or whether each release puts it at the end of a
# line it never reaches the front of.
#
# So a TENTH run, and it is written here BEFORE `spec/schedule.sas` can pass it,
# the order `C-002i2a` and `C-002i2c1a` stood in and for their reason: a contract
# written after the program it describes is a description, and a description has
# never refused anything. What follows is RED today — the program prints nine
# runs and is asked for ten — and `C-002i2c2b` is the row that owes the rest.
#
# ### The shape the tenth run has to have
#
# Thread 3 is the CYCLER. It starts parked on `द्वारम्` as a receiver, the way
# `नियामकः` does, and unlike `नियामकः` it has work of its own and is an ordinary
# candidate on every turn it is runnable. Thread `0` releases it by SENDING —
# `C-002e5`'s four words, the same three rows in send order — and it goes back
# onto the endpoint by its own action once it has eaten. Threads 1 and 2 never
# touch the endpoint: they are the ones that take its place if a release quietly
# costs it its turn.
#
# `चक्रम्` is installed, and the tenth run must print the two words the second
# run printed. That is deliberate: round-robin alone has no notion of a level, so
# nothing here can be laid at the door of the lift a blocking thread earns —
# `C-002e6` is that claim, made once, over a single block. What is under test is
# the ORDER, and a policy that sorts by a number the block itself moves could be
# the thing producing it.
#
# ### The bound, and why it is not `starvation()` again
#
# The clock starts when the cycler RE-ENTERS the offered set, not when it was
# last eligible. From that turn until the turn it is chosen, NO OTHER THREAD MAY
# RUN TWICE. That is the same fairness `starvation()` states for a thread that
# never leaves — a turn comes round before anyone takes a second — asked ACROSS a
# block instead of around one, and read off the same two log fields: the set at
# bits 56..59 and the choice at bits 0..7. Nothing is asked of the cycler while
# it is out of the set, which is the wait it asked for and not one it lost.
#
# It is a BOUND and not a "never", because a trace can carry a bound and cannot
# carry a never. A thread passed over once by each of its equals has waited its
# turn; a thread passed over while somebody takes a SECOND has lost one, and lost
# it for having waited.
#
# THREE CYCLES, not one. One block is an incident, two a coincidence, and it
# takes a third before "comes round" is a property rather than a thing that
# happened — the same reason `ASKS` is 3 above. At least two of them must be
# blocks the cycler performs ON ITSELF, event 1 on a turn it ran, because a
# thread that is only ever blocked at startup has not shown it can go back.
#
# ### And the work must finish
#
# All four threads' work reaches zero and the run ends under the dispatcher's
# budget, the cycler's own work included. A run in which the cycler is scheduled
# promptly and still never finishes has answered the fairness question and lost
# the machine anyway.
#
# ### The releases are messages, so one of them is tied to words that crossed
#
# The cycler's inbox is printed at the end and must hold the LAST release's four
# words with THREAD 0's identity in the fourth — not the kernel's, which is what
# separates a release from an activation, and not zero, which would say it was
# woken by a store rather than by a rendezvous. The other three inboxes are zero.
# Same limit as the eighth run, said again: an inbox printed once ties the last
# release to words that travelled, and the per-turn evidence is the event word.
#
# ### Its limits, said out loud
#
# **A bounded wait in one finite run is not a liveness proof.** What is shown is
# that no release in this run was followed by a lost turn, over three cycles and
# under one policy. A schedule this program does not run could still starve
# something, and no trace can say otherwise.
#
# **Deadlock is not starvation and is not here.** The cycler is released because
# thread 0 sends to it; a thread nobody ever releases is a different failure with
# a different fix, and this run does not reach it.
#
# **The same claim under `श्रेणी` is not made.** `C-002e6` covers the lift a
# single block earns; the repeated case under a level-sorting policy would be a
# second run, and it is not this row.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$root/spec/schedule.sas" "$tmp/s.elf" >/dev/null
    # Bounded by an explicit SIGKILL, not perl's alarm: QEMU handles SIGALRM and
    # outlives it (`W-060`). A scheduler that never returns -1 runs forever, so
    # the bound is not decoration.
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/s.elf" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 25; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    # Every matching line is taken, not a fixed tail: the number of turns is
    # what is under test, and a `tail -44` would quietly hide a run that went
    # long by cutting off its beginning.
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$'
}

fail=0; first=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    if ! python3 - "$tmp/l" > "$tmp/v" 2>&1 <<'PY'
import sys
v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]

N, QUANTUM, MAXLEVEL, BUDGET = 4, 2, 2, 64
INIT_LEVEL = [0, 0, 0, 2]
INIT_BLOCKED = [0, 0, 0, 1]
RELEASE_AT = 4
INIT_WORK  = [10, 10, 10, 4]
APPETITE   = [8, 8, 8, 1]
# The द्वारम् run: nobody starts blocked, every level starts at 0, and thread 0
# carries enough work to stay alive until it has released all three.
D_LEVEL, D_WORK, D_BLOCKED = [0, 0, 0, 0], [15, 10, 10, 10], [0, 0, 0, 0]
D_APPETITE = [8, 8, 8, 8]
WAITERS = [2, 1, 3]          # the order they sit down, which is not index order
BLOCK, RELEASE, WOKE, HANDOFF = 1, 2, 3, 4
# The three messages, IN THE ORDER THEY ARE SENT — the k-th send carries the k-th
# row, whoever is at the head when it goes. The fourth word is not in the table:
# the sender writes its own identity there, over a 999 that would show if it did
# not. `spec/schedule.sas` holds the same three rows.
WORDS = 4
MESSAGES = [[13, 4096, 32768], [15, 8192, 32800], [12, 12288, 32832]]
SENDER, UNSENT = 0, [0, 0, 0, 0]
ORD = ("first", "second", "third")
# The fourth run: thread 0 sends into an empty endpoint and blocks there, thread
# 3 comes along and takes the words. It carries the first message row, so what
# the receiver must end up holding is fixed by the same table.
SND, RCV = 0, 3
# The eighth run — the upcall (`C-002i2a`). Thread 3 is नियामकः, the userspace
# scheduler: parked on the endpoint from the first instant, no work of its own,
# and never a candidate. The other three are equal at level 0 so that nothing in
# the order can be laid at a level's door. UPCALL is a fifth event word, the
# self-declaring mark of a turn on which the kernel called OUT; KERNEL is the
# identity that arrives in the message's fourth word and is NOT one of the four
# threads, which is the whole of what it is there to say.
UPCALL, NIYAMAKA, KERNEL = 5, 3, 9
U_LEVEL, U_WORK, U_BLOCKED = [0, 0, 0, 0], [10, 10, 10, 0], [0, 0, 0, 1]
U_APPETITE = [8, 8, 8, 8]
# The ninth run — the upcall that is never answered (`C-002i2c1a`). The same
# table and the same two words in नीतिः; the one word that differs is inside
# नियामकः, which takes its turn and re-parks without leaving an answer. GIVEUP is
# a sixth event word, the self-declaring mark of the turn on which the kernel
# stopped asking. ASKS is the bound, and it is the program's, not this file's
# SIGKILL: one ask says nothing about silence, a second is the first evidence the
# first went unanswered, a third is what makes it a bound rather than "ask twice".
GIVEUP, ASKS = 6, 3
# The tenth run — ordering over BLOCKED threads (`C-002i2c2a`). Thread 3 is the
# CYCLER: parked on the endpoint from the first instant like नियामकः, but with
# work of its own and an ordinary candidate every turn it is runnable. Thread 0
# (SENDER) releases it, three times, with the three message rows in send order;
# it goes back onto the endpoint by its own action. Threads 1 and 2 never touch
# the endpoint and are the ones that would take its turn. CYCLES is 3 for the
# reason ASKS is: one block is an incident and two a coincidence.
CYCLER, CYCLES = 3, 3
S_LEVEL, S_WORK, S_BLOCKED = [0, 0, 0, 0], [12, 10, 10, 6], [0, 0, 0, 1]
S_APPETITE = [8, 8, 8, 2]
bad = []

def die(msg):
    print(msg); sys.exit(1)

# ---- structure ----------------------------------------------------------
if len(v) < 40:
    die(f"only {len(v)} lines printed; expected at least forty (two policy "
        "addresses, a turn count and at least one turn, ten times over)")
head = []
i = 0
for which in ("first", "second", "third", "fourth", "fifth", "sixth", "seventh",
              "eighth", "ninth", "tenth"):
    if i + 3 > len(v):
        die(f"the {which} run's header is missing after {i} lines")
    choose, account, n = v[i], v[i + 1], v[i + 2]
    # The turns are counted; the inboxes are not, because their length is fixed
    # by the program — four threads, four words. A printed count of a fixed-length
    # thing lets the run tell the check how much of it to expect, which is the one
    # number a check must not take on trust.
    if i + 3 + n + N * WORDS > len(v):
        die(f"the {which} run says {n} turns and then four inboxes of {WORDS} words, "
            f"which is {3 + n + N * WORDS} lines, but only {len(v) - i} follow it")
    box = v[i + 3 + n:i + 3 + n + N * WORDS]
    head.append((choose, account, v[i + 3:i + 3 + n],
                 [box[WORDS * k:WORDS * (k + 1)] for k in range(N)]))
    i += 3 + n + N * WORDS
if i != len(v):
    die(f"{len(v)} lines printed; the ten runs account for {i}")
(chooseA, accountA, A, boxA), (chooseB, accountB, B, boxB), \
    (chooseC, accountC, C, boxC), (chooseD, accountD, D, boxD), \
    (chooseE, accountE, E, boxE), (chooseF, accountF, F, boxF), \
    (chooseG, accountG, G, boxG), (chooseH, accountH, H, boxH), \
    (chooseI, accountI, I, boxI), (chooseJ, accountJ, J, boxJ) = head
nA, nB, nC, nD, nE, nF, nG, nH, nI, nJ = (len(A), len(B), len(C), len(D), len(E),
                                          len(F), len(G), len(H), len(I), len(J))

def unpack(e):
    return (e & 0xff, (e >> 8) & 0xff, (e >> 16) & 0xff,
            [(e >> (24 + 4 * i)) & 0xf for i in range(N)],
            [(e >> (40 + 4 * i)) & 0xf for i in range(N)],
            (e >> 56) & 0xf, (e >> 60) & 0xf)

# Who the DISPATCHER offered the policy, read off the log rather than
# recomputed. Every eligibility question below asks this and not `work > 0`:
# a thread with work left can be blocked, and the difference between the two
# is the whole of `C-002e1`.
def offered(e):
    return [i for i in range(N) if unpack(e)[5] >> i & 1]

# ---- things every later pass assumes, so they are fatal on their own ----
# Without this the out-of-range case reaches the priority pass and dies there
# with a Python traceback, which is a failure that does not say which rule it
# broke — the `C-002c` complaint, made by the check instead of the program.
def structural(name, tr):
    if not tr:
        die(f"{name}: no turns at all — the policy refused to schedule anything")
    for t, e in enumerate(tr, 1):
        want, got, _, _, _, _, _ = unpack(e)
        if want >= N or got >= N:
            die(f"{name} turn {t}: the log names thread {max(want, got)} and there are "
                f"only {N} — the index is not a thread at all")

# ---- what the INTERFACE guarantees, whatever policy is installed --------
# `init` is the run's starting state and `waiting(t)` names the threads that are
# legitimately out of the set on turn `t` — thread 3 for the first two runs,
# whoever is sitting on the endpoint for the third. Everything else here is a law
# of the interface and is asserted the same way for all three runs.
def invariants(name, tr, init=(INIT_LEVEL, INIT_WORK, INIT_BLOCKED, APPETITE),
               waiting=lambda t: {3}):
    init_lv, init_wk, init_bl, app = init
    if len(tr) >= BUDGET - 1:
        bad.append(f"{name}: ran {len(tr)} turns and stopped on the dispatcher's budget, "
                   "not because the work ran out — some thread was never finishing")
    _, _, _, lv0, wk0, m0, _ = unpack(tr[0])
    if lv0 != init_lv or wk0 != init_wk:
        bad.append(f"{name}: the run began at levels {lv0} works {wk0}, expected "
                   f"{init_lv} / {init_wk} — the thread table was not put back to "
                   "its starting state, so this run is the tail of the previous one")
    want0 = sum(1 << i for i in range(N) if not init_bl[i] and init_wk[i])
    if m0 != want0:
        bad.append(f"{name}: the first choice was made from {m0:#06b}, expected "
                   f"{want0:#06b} — the run's starting रोधः is {init_bl} against works "
                   f"{init_wk}, so the set the policy is first offered is fixed by it. A "
                   "run that starts from a different set has not been put back to its "
                   "starting state, and everything below it is about a different program")
    for t, e in enumerate(tr, 1):
        want, got, used, lv, wk, mask, ev = unpack(e)
        run = offered(e)
        if want != got:
            bad.append(f"{name} turn {t}: the policy chose thread {want} but thread {got} "
                       "is the one that ran — control did not arrive where it was sent, "
                       "so the switch and the decision have come apart")
            return
        if want not in run:
            bad.append(f"{name} turn {t}: thread {want} was chosen out of the set "
                       f"{run} it was never offered — the policy is picking from its own "
                       "idea of who can run rather than from the dispatcher's, which is "
                       "the one that knows about blocking")
        for i in range(N):
            if (mask >> i & 1) and wk[i] == 0:
                bad.append(f"{name} turn {t}: thread {i} was offered with no work left — "
                           "the dispatcher offered a thread that is finished")
            # Only thread 3 is ever blocked here, so for the other three the set
            # must agree with the work exactly. This is what keeps the mask from
            # becoming a second, unfalsifiable source of truth.
            if i not in waiting(t) and not (mask >> i & 1) and wk[i] > 0:
                bad.append(f"{name} turn {t}: thread {i} has work {wk[i]} and was not "
                           "offered, and nothing was blocking it — the dispatcher is "
                           "withholding a runnable thread")
        # A thread that blocks itself spends its turn waiting rather than working,
        # so it eats nothing. That is not a special case bolted on for the check:
        # it is what makes गणनम् see a thread that ate less than its slice.
        eat = 0 if ev == BLOCK else min(app[want], QUANTUM, wk[want])
        if used != eat:
            bad.append(f"{name} turn {t}: thread {want} ate {used} units, expected {eat} "
                       f"(appetite {app[want]}, slice {QUANTUM}, work left {wk[want]}"
                       + (", and it blocked itself on द्वारम् this turn)" if ev == BLOCK
                          else ")"))
        if any(l > MAXLEVEL for l in lv):
            bad.append(f"{name} turn {t}: levels {lv} — {MAXLEVEL} is the bottom")
        if t < len(tr):
            nxt = list(wk); nxt[want] -= used
            _, _, _, _, wk2, _, _ = unpack(tr[t])
            if wk2 != nxt:
                bad.append(f"{name} turn {t}: work went {wk} -> {wk2}, but thread {want} "
                           f"ate {used} so it should have gone {nxt} — work is appearing "
                           "or vanishing between turns")
        else:
            end = list(wk); end[want] -= used
            if any(x for x in end):
                bad.append(f"{name}: the run ended with work {end} still outstanding — the "
                           "policy stopped scheduling while threads were still runnable")
    ran = {unpack(e)[0] for e in tr}
    missing = [i for i in range(N) if i not in ran]
    if missing:
        bad.append(f"{name}: thread(s) {missing} never ran once in {len(tr)} turns — starved "
                   "outright, not merely delayed")

# ---- priority, read straight off the log -------------------------------
def priority_violations(tr):
    out = []
    for t, e in enumerate(tr, 1):
        want, _, _, lv, wk, _, _ = unpack(e)
        runnable = offered(e)
        if not runnable:
            continue
        lowest = min(lv[i] for i in runnable)
        if lv[want] != lowest:
            out.append((t, want, lv[want], lowest,
                        [i for i in runnable if lv[i] == lowest]))
    return out

# ---- no thread runs twice before its equals run once -------------------
# Stated over the log, not over a model of the policy: "eligible" is read from
# the recorded levels and works, and "ran" is read from the recorded choices.
def starvation(name, tr, by_level):
    seen = {}
    for t, e in enumerate(tr, 1):
        want, _, _, lv, wk, _, _ = unpack(e)
        runnable = offered(e)
        lowest = min(lv[i] for i in runnable) if runnable else 0
        elig = [i for i in runnable if (not by_level) or lv[i] == lowest]
        if want in seen:
            t0, elig0 = seen[want]
            between = {unpack(x)[0] for x in tr[t0:t - 1]}
            for y in elig:
                if y != want and y in elig0 and y not in between:
                    bad.append(f"{name}: thread {want} ran at turn {t0} and again at turn "
                               f"{t}, and thread {y} was eligible at both and ran at "
                               "neither — at one level the turn must come round before "
                               "anyone takes a second")
                    return
        seen[want] = (t, elig)

# ---- the accounting rule, checked against the levels it produced -------
def accounting(name, tr, moves):
    for t, e in enumerate(tr[:-1], 1):
        want, _, used, lv, _, _, _ = unpack(e)
        _, _, _, lv2, _, _, _ = unpack(tr[t])
        if not moves:
            if lv2 != lv:
                bad.append(f"{name} turn {t}: levels moved {lv} -> {lv2}, but this policy's "
                           "गणनम् does nothing — something outside the policy is editing "
                           "the level, which means the seam leaks")
                return
            continue
        if used >= QUANTUM:
            exp = min(lv[want] + 1, MAXLEVEL)
            why = f"ate its whole slice ({used} of {QUANTUM}) and must drop a level"
        else:
            exp = max(lv[want] - 1, 0)
            why = f"ate only {used} of {QUANTUM} and must rise a level"
        if lv2[want] != exp:
            bad.append(f"{name} turn {t}: thread {want} {why}, going {lv[want]} -> {exp}, "
                       f"but the next turn records it at {lv2[want]}")
            return

# ---- रोधः: a blocked thread waits, and the wait ENDS --------------------
# Blocking is only worth anything if it can be released. Without the release
# half, "blocked" and "finished" look the same from outside — the thread has
# work, never runs, and the run ends. So both halves are asserted, and the
# turn the release happens at is derived from the log's own record of thread
# 0's work rather than from a count of turns, which would differ per policy.
def blocking(name, tr):
    # Held means WITHHELD: out of the set with work still left. Once thread 3
    # drains, it is out of the set for the ordinary reason and that says
    # nothing about रोधः — counting those turns as blocked is how the first
    # draft of this check reported a re-block that never happened.
    held = [t for t, e in enumerate(tr, 1)
            if 3 not in offered(e) and unpack(e)[4][3] > 0]
    free = [t for t, e in enumerate(tr, 1) if 3 in offered(e)]
    if not held:
        bad.append(f"{name}: thread 3 was offered on every turn — रोधः never kept it out "
                   "of a single choice, so nothing here is about blocking")
        return
    if not free:
        bad.append(f"{name}: thread 3 was never offered on any of {len(tr)} turns — it was "
                   "blocked and stayed blocked, so the release never fired and a blocked "
                   "thread is indistinguishable from a finished one")
        return
    if held[0] != 1 or held[-1] >= free[0]:
        bad.append(f"{name}: thread 3 was withheld with work left on turns {held} and "
                   f"offered on {free} — it must be held from turn 1 and, once released, "
                   "stay released until its work runs out; a thread that goes back to "
                   "blocked is being re-blocked by something no one wrote")
        return
    ran = [t for t, e in enumerate(tr, 1) if unpack(e)[0] == 3]
    if any(t < free[0] for t in ran):
        bad.append(f"{name}: thread 3 ran at turn(s) {[t for t in ran if t < free[0]]}, "
                   f"before it was released at turn {free[0]} — the block did not hold")
    if not ran:
        bad.append(f"{name}: thread 3 was released at turn {free[0]} and still never ran — "
                   "the release changed a bit and not a decision")
    # The release is thread 0 dropping to RELEASE_AT, and the log records the
    # work of every thread at every choice. So this reads the CAUSE off the
    # same trace as the effect, and a release that fired for some other reason
    # is a different failure from a release that never fired.
    _, _, _, _, wk_at, _, _ = unpack(tr[free[0] - 1])
    if wk_at[0] > RELEASE_AT:
        bad.append(f"{name}: thread 3 became runnable at turn {free[0]}, when thread 0 "
                   f"still had {wk_at[0]} work left — the only writer of रोधः is thread 0 "
                   f"at {RELEASE_AT}, so something else cleared it")

# ---- द्वारम्: threads sit down by themselves, and get up in order ---------
# Everything below is read off the log. The endpoint's own array is never
# consulted: a queue that nothing pops is a list, and an order no behaviour
# depends on cannot be observed to be wrong.
def events(tr, code):
    return [(t, unpack(e)[0]) for t, e in enumerate(tr, 1) if unpack(e)[6] == code]

def dvaram(name, tr):
    for t, e in enumerate(tr, 1):
        if unpack(e)[6] > WOKE:
            die(f"{name} turn {t}: event {unpack(e)[6]} is not one of the four the log "
                "defines, so the top four bits are carrying something else")
    blocks, releases, wakes = (events(tr, c) for c in (BLOCK, RELEASE, WOKE))
    if [w for _, w in blocks] != WAITERS:
        bad.append(f"{name}: the threads that blocked THEMSELVES were "
                   f"{[w for _, w in blocks]} on turns {[t for t, _ in blocks]}, expected "
                   f"{WAITERS} — the countdowns say who sits down when, and if that order "
                   "is wrong every order below it is about a different run")
        return
    if [w for _, w in releases] != [0, 0, 0]:
        bad.append(f"{name}: the head was released by {[w for _, w in releases]}, and only "
                   "thread 0 releases anything here")
        return
    got = [w for _, w in wakes]
    if got != WAITERS:
        why = ("that is the order they were CHOSEN in, not the order they waited in — a "
               "set, not a queue" if got == sorted(WAITERS) else
               "that is last-in-first-out — a stack, not a queue" if got == WAITERS[::-1]
               else "which is neither the order they blocked in nor any order at all")
        bad.append(f"{name}: they blocked {WAITERS} and came back {got}. First in must be "
                   f"first out, so they must come back {WAITERS}; {why}")
        return
    # Each thread must come back INSIDE its own wait — after a release, and after
    # its own block. A wake before the release that caused it would mean the
    # thread resumed on its own and the endpoint decided nothing.
    for k, ((tb, w), (tr_, _), (tw, _)) in enumerate(zip(blocks, releases, wakes)):
        if not tb < tr_ < tw:
            bad.append(f"{name}: thread {w} blocked at turn {tb}, the {k + 1}th release "
                       f"was turn {tr_} and it came back at turn {tw} — a thread must "
                       "block, then be released, then run, in that order")
            return
        held = [t for t, e in enumerate(tr, 1)
                if tb < t < tw and w not in offered(e) and unpack(e)[4][w] > 0]
        ran = [t for t, e in enumerate(tr, 1) if tb < t < tw and unpack(e)[0] == w]
        if ran:
            bad.append(f"{name}: thread {w} blocked itself at turn {tb} but ran at "
                       f"turn(s) {ran} before it was released — the block did not hold")
        if len(held) != tw - tb - 1:
            bad.append(f"{name}: thread {w} was out of the offered set on {len(held)} of "
                       f"the {tw - tb - 1} turns between blocking at {tb} and coming back "
                       f"at {tw} — a thread on an endpoint is out of every one of them")
    # And nobody sits down twice: a thread that re-blocks would rewrite the queue
    # and the order coming out would no longer be the order that went in.
    twice = [w for w in set(x for _, x in blocks) if [x for _, x in blocks].count(w) > 1]
    if twice:
        bad.append(f"{name}: thread(s) {twice} blocked more than once")

# ---- the message: four words, and they arrived with the right thread (`C-002e5`)
# What is read here is the RECEIVER'S OWN RECORD. A woken thread comes back from
# inside `सञ्चारः` holding the message in `स्थिर२..५` — the sender wrote it into
# its saved context — and copies those four registers into its inbox before it
# touches memory. The sender's table is never read by this check, so what is
# compared is what a thread says it was holding, not what a sender says it sent.
#
# Messages are chosen BY SEND ORDER, not by receiver. So the pairing is the queue
# doing work: with a stack the third message would land on the first waiter, and
# that is caught here as well as by the order of resumption above — two
# independent readings of the same defect.
#
# What this does NOT show, and it should not be dressed up as if it did: the
# print cannot tell a word delivered through the saved context from a word a
# sender wrote straight into the inbox. That wire is visible in the source and
# is the saved context because `C-002e3`'s fast path has to carry it with no
# buffer. What the print does settle is WHICH four words reached WHICH thread.
def carried(name, tr, box):
    if box[SENDER] != UNSENT:
        bad.append(f"{name}: thread {SENDER} sent all three messages and was sent none, "
                   f"but its inbox holds {box[SENDER]} — something is writing into a "
                   "thread that never sat down on the endpoint")
    for k, w in enumerate(WAITERS):
        want, got = MESSAGES[k] + [SENDER], box[w]
        if got == want:
            continue
        if got == UNSENT:
            bad.append(f"{name}: thread {w} was released at the {ORD[k]} send and came "
                       f"back holding nothing, and it should have been {want} — the "
                       "release moved a bit and carried no message, which is exactly what "
                       "C-002e4 already did")
        elif got[:3] in MESSAGES:
            j = MESSAGES.index(got[:3])
            bad.append(f"{name}: thread {w} was the {ORD[k]} thread sent to and came back "
                       f"with the {ORD[j]} message {got[:3]}. Messages go out in send order, "
                       "so either the queue handed over a thread that had not waited longest "
                       "and the message followed it, or the sender is not advancing through "
                       "them — the wake order above says which")
        elif got[3] != SENDER:
            bad.append(f"{name}: thread {w} came back with {got}, whose fourth word is "
                       f"{got[3]} and not the sender {SENDER} — 999 is what the table "
                       "carries in that slot, so the sender never wrote its own identity "
                       "and the receiver cannot say who it heard from")
        else:
            bad.append(f"{name}: thread {w} came back holding {got}, which is not any of "
                       f"the three messages {MESSAGES} — the four words crossed the switch "
                       "as something other than what was sent")
    # A message is a message only if it says something the other two do not: three
    # identical rows would pass every pairing test above and prove nothing.
    if len({tuple(m) for m in MESSAGES}) != len(MESSAGES):
        die("the three messages are not three distinct words, so no pairing between them "
            "and the threads can be observed to be wrong")

# ---- the sender that blocks: the rendezvous from the other end (`C-002e`)
# The third run's सender never waits — it releases and carries on — so nothing
# there says what happens when a send finds the endpoint empty. Here that is the
# whole run, and it is read the same way: off the events and the inboxes, never
# off the queue.
def rendezvous(name, tr, box):
    blocks, releases, wakes = (events(tr, c) for c in (BLOCK, RELEASE, WOKE))
    if [w for _, w in blocks] != [SND]:
        bad.append(f"{name}: the threads that blocked themselves were "
                   f"{[w for _, w in blocks]}, and here exactly one must — thread {SND}, "
                   f"the SENDER, because it sends into an empty endpoint. Thread {RCV} "
                   "receiving must not block: it arrives to find the sender already "
                   "sitting there, and a receive that finds a sender completes in the "
                   "turn it is made")
        return
    tb = blocks[0][0]
    early = [(t, w, c) for c, evs in ((RELEASE, releases), (WOKE, wakes))
             for t, w in evs if t < tb]
    if early:
        bad.append(f"{name}: turn {early[0][0]} records event {early[0][2]} before "
                   f"anything had blocked at turn {tb} — the sender is only proved to "
                   "have found the endpoint EMPTY because its own block is the first "
                   "event in the run, and this run has one before it")
        return
    if [w for _, w in releases] != [RCV]:
        bad.append(f"{name}: the sender was let go by {[w for _, w in releases]}, and "
                   f"only thread {RCV} receives here — a wake that came from anywhere "
                   "else is not a receive")
        return
    if [w for _, w in wakes] != [SND]:
        bad.append(f"{name}: the threads that came back from inside सञ्चारः were "
                   f"{[w for _, w in wakes]}, expected just the sender {SND}")
        return
    trel, tw = releases[0][0], wakes[0][0]
    if not tb < trel < tw:
        bad.append(f"{name}: the sender blocked at turn {tb}, the receive was turn "
                   f"{trel} and it came back at turn {tw} — it must block, then be "
                   "received from, then run, in that order")
        return
    ran = [t for t, e in enumerate(tr, 1) if tb < t < tw and unpack(e)[0] == SND]
    if ran:
        bad.append(f"{name}: the sender blocked at turn {tb} but ran at turn(s) {ran} "
                   f"before the receive at turn {trel} — the block did not hold, and a "
                   "send that does not block is a mailbox")
    held = [t for t, e in enumerate(tr, 1)
            if tb < t < tw and SND not in offered(e) and unpack(e)[4][SND] > 0]
    if len(held) != tw - tb - 1:
        bad.append(f"{name}: the sender was out of the offered set on {len(held)} of the "
                   f"{tw - tb - 1} turns between blocking at {tb} and coming back at "
                   f"{tw} — a thread sitting on an endpoint is out of every one of them")
    # The machine must keep going while the sender is parked. Threads 1 and 2
    # never touch the endpoint, so if they do not run through the wait, what is
    # being shown is a stall and not a block.
    others = {unpack(e)[0] for e in tr[tb:tw - 1]} - {SND, RCV}
    if others != {1, 2}:
        bad.append(f"{name}: threads {sorted(others)} ran while the sender waited from "
                   f"turn {tb} to turn {tw}, expected [1, 2] — neither touches द्वारम्, "
                   "and their running is what separates a blocked sender from a stalled "
                   "dispatcher")
    want = MESSAGES[0] + [SND]
    for k in range(N):
        got, exp = box[k], want if k == RCV else UNSENT
        if got == exp:
            continue
        if k == SND and got == want:
            bad.append(f"{name}: the SENDER came back holding {got} — the words it sent. "
                       "A sender receives nothing here; an inbox filled on the sending "
                       "side means the wake path copied its own registers back, which is "
                       "what a receiver does and not what a sender does")
        elif k == RCV and got == UNSENT:
            bad.append(f"{name}: thread {RCV} received from a waiting sender and came "
                       f"back holding nothing, and it should have been {want} — the "
                       "receive cleared a block and took no words, so the queue was used "
                       "as a wakeup and not as a rendezvous")
        elif k == RCV and got[3] != SND:
            bad.append(f"{name}: thread {RCV} holds {got}, whose fourth word is {got[3]} "
                       f"and not the sender {SND} — 999 is what the table carries in that "
                       "slot, so the sender never wrote its own identity")
        else:
            bad.append(f"{name}: thread {k} holds {got} and should hold {exp} — the four "
                       f"words are supposed to go from thread {SND} to thread {RCV} and "
                       "nowhere else")

# ---- the promotion a BLOCKING thread earns (`C-002e6`)
# Doc 11 §3.4 wants an interactive thread kept in front. `श्रेणी` never asks
# whether a thread is interactive: its only rule is that eating the whole slice
# demotes and eating less promotes. A thread that sits down on द्वारम् eats
# NOTHING on the turn it does so — which is less than its slice — so the lift
# falls out of the MLFQ rule and is not bolted on beside it. Nothing has shown
# that until now, because the two द्वारम् runs install `चक्रम्`, which never
# reads a level; this is the third run again with one word changed.
#
# TWO THINGS ARE REQUIRED AND THE SECOND IS THE POINT. A level that moves and
# never decides anything is a number, and `C-002i`'s interactivity check already
# refuses to accept one — so it is not enough that the level rises: the thread
# has to be CHOSEN on the strength of the rise, ahead of somebody standing where
# the block found it.
#
# A block at level 0 is a legitimate no-op — 0 is the top and `max(lv-1, 0)` is
# the rule — so those are counted and not complained about. What would be a
# failure is every block landing there, because then nothing here is evidence.
def promotion(name, tr):
    rises = []
    for t, e in enumerate(tr[:-1], 1):
        want, _, _, lv, _, _, ev = unpack(e)
        if ev == BLOCK:
            rises.append((t, want, lv[want], unpack(tr[t])[3][want]))
    if not rises:
        bad.append(f"{name}: not one thread sat down on द्वारम्, so there was no thread "
                   "that ate less than its slice and no promotion to earn")
        return
    real = [r for r in rises if r[3] < r[2]]
    if not real:
        bad.append(f"{name}: {len(rises)} thread(s) blocked and not one level rose ("
                   + ", ".join(f"thread {w} stayed at {b} across its block on turn {t}"
                               for t, w, b, _ in rises)
                   + "). Either गणनम् is not lifting a thread that ate less than its "
                     "slice, or every block happened at level 0 where the lift is a "
                     "no-op — and then this run shows nothing either way")
        return
    for tb, w, before, after in real:
        later = [(t, e) for t, e in enumerate(tr, 1) if t > tb and unpack(e)[0] == w]
        if not later:
            bad.append(f"{name}: thread {w} rose {before} -> {after} by blocking at turn "
                       f"{tb} and then never ran again — a promotion the thread never "
                       "cashes in is a number that changed")
            return
        t2, e2 = later[0]
        lv = unpack(e2)[3]
        if lv[w] != after:
            bad.append(f"{name}: thread {w} was lifted to level {after} on turn {tb} and "
                       f"came back at level {lv[w]} on turn {t2} — nothing ran it in "
                       "between, so its level was edited by something outside गणनम्")
            return
        behind = [j for j in offered(e2) if j != w and lv[j] >= before]
        if not behind:
            bad.append(f"{name}: thread {w} rose {before} -> {after} by blocking at turn "
                       f"{tb}, and when it next ran at turn {t2} the runnable threads "
                       f"stood at {[(j, lv[j]) for j in offered(e2)]} — none of them was "
                       f"at {before} or worse, so the rise cannot have put it in front of "
                       "anybody and it changed a number and not a decision")
            return
    # What this does NOT settle, and it is not dressed up as if it did: that the
    # choice would have gone the other way without the lift. Without it the thread
    # would have stood at `before` — level with the thread it overtook, not behind
    # it — and among equals चक्रक्रम् picks whoever ran longest ago, which a thread
    # fresh off an endpoint generally is. What is settled is that PRIORITY, not
    # round-robin, is what ordered that turn, and priority could only order it
    # because the block moved the level.

structural("श्रेणी", A)
structural("चक्रम्", B)
structural("द्वारम्", C)
structural("प्रेषकः", D)
structural("उन्नतिः", E)
invariants("श्रेणी", A)
invariants("चक्रम्", B)

# Who is legitimately out of the set on turn t of the द्वारम् run: whoever has
# blocked and not yet come back. Derived from the log, so a thread withheld for
# any other reason is still caught by the same rule that catches it in A and B.
# Paired per THREAD and not by position: pairing the k-th block with the k-th
# wake is right only when the order is right, so a wrong order would report
# itself here as a withheld-runnable-thread complaint instead of as the wrong
# order it is — the check would be blaming the dispatcher for the queue.
def sitting_in(tr):
    def sitting(t):
        out = set()
        for tb, w in events(tr, BLOCK):
            wake = [tw for tw, x in events(tr, WOKE) if x == w and tw > tb]
            if tb < t < (wake[0] if wake else len(tr) + 1):
                out.add(w)
        return out
    return sitting

invariants("द्वारम्", C, (D_LEVEL, D_WORK, D_BLOCKED, D_APPETITE), sitting_in(C))
dvaram("द्वारम्", C)
carried("द्वारम्", C, boxC)
starvation("द्वारम्", C, False)
accounting("द्वारम्", C, False)

# The fourth run starts from the same table as the third and is held to the same
# laws — only the direction of the rendezvous differs.
invariants("प्रेषकः", D, (D_LEVEL, D_WORK, D_BLOCKED, D_APPETITE), sitting_in(D))
for t, e in enumerate(D, 1):
    if unpack(e)[6] > WOKE:
        die(f"प्रेषकः turn {t}: event {unpack(e)[6]} is not one of the four the log "
            "defines, so the top four bits are carrying something else")
rendezvous("प्रेषकः", D, boxD)
starvation("प्रेषकः", D, False)
accounting("प्रेषकः", D, False)

# The fifth run is the third one with `श्रेणी` installed instead of `चक्रम्`, so
# every law the third run is held to still applies — the queue is the queue
# whatever is choosing. What is new is that the policy now READS and WRITES
# levels, so `accounting` is asked for the MLFQ rule rather than for silence,
# and priority has to order every turn.
invariants("उन्नतिः", E, (D_LEVEL, D_WORK, D_BLOCKED, D_APPETITE), sitting_in(E))
dvaram("उन्नतिः", E)
carried("उन्नतिः", E, boxE)
starvation("उन्नतिः", E, True)
accounting("उन्नतिः", E, True)
promotion("उन्नतिः", E)
pvE = priority_violations(E)
if pvE:
    t, want, got, low, who = pvE[0]
    bad.append(f"उन्नतिः turn {t}: thread {want} ran at level {got} while thread(s) {who} "
               f"were runnable at level {low} — this run installs श्रेणी, and a policy "
               "that lets a worse level through is not the one whose promotion is under "
               "test here")

# The widening must be INERT where द्वारम् is shut. If the first two runs record
# an event, the four new bits are picking something up on their own and the third
# run's order is not evidence of anything.
for nm, tr, box in (("श्रेणी", A, boxA), ("चक्रम्", B, boxB)):
    noisy = [t for t, e in enumerate(tr, 1) if unpack(e)[6] != 0]
    if noisy:
        bad.append(f"{nm}: turn(s) {noisy} record a द्वारम् event, and द्वारम् is shut "
                   "for this run — the event field is not reporting what it says")
    # Nobody sends in these two runs, so every inbox must be empty — and empty at
    # the START of the third run for the same reason, which is why they are cleared
    # per run rather than once. A word left over from a previous run looks exactly
    # like a word that was sent.
    if box != [UNSENT] * N:
        bad.append(f"{nm}: the inboxes came out {box} with द्वारम् shut for this run — "
                   "nothing sent anything, so either they were not cleared between runs "
                   "or something is writing messages nobody sent")

# ---- policy श्रेणी: priority, fairness, and a promotion that BITES -----
pv = priority_violations(A)
if pv:
    t, want, got, low, who = pv[0]
    bad.append(f"श्रेणी turn {t}: thread {want} ran at level {got} while thread(s) {who} "
               f"were runnable at level {low} — priority is not ordering anything, and a "
               "count of runs would never have shown it")
starvation("श्रेणी", A, True)
starvation("चक्रम्", B, False)
accounting("श्रेणी", A, True)
accounting("चक्रम्", B, False)
blocking("श्रेणी", A)
blocking("चक्रम्", B)

# The interactive thread starts at the bottom and must climb by BEHAVING.
lv3 = [unpack(e)[3][3] for e in A]
drops = sum(1 for i in range(1, len(lv3)) if lv3[i] < lv3[i - 1])
if drops < 2:
    bad.append(f"श्रेणी: thread 3 started at level {INIT_LEVEL[3]} and its level fell "
               f"{drops} time(s) across the run — it eats less than its slice every turn, "
               "so the interactivity rule should have lifted it more than once")
overtook = [t for t, e in enumerate(A, 1)
            if unpack(e)[0] == 3 and any(j in offered(e) and unpack(e)[3][j] > unpack(e)[3][3]
                                         for j in range(N) if j != 3)]
if not overtook:
    bad.append("श्रेणी: thread 3 was never chosen while another thread was runnable at a "
               "worse level — its promotion changed a number and never changed an order, "
               "which is the only thing the promotion is for")

# ---- the seam ----------------------------------------------------------
seqA = [unpack(e)[0] for e in A]
seqB = [unpack(e)[0] for e in B]
seqC = [unpack(e)[0] for e in C]
seqE = [unpack(e)[0] for e in E]
if seqA == seqB:
    bad.append("both policies produced the same order of threads. Either the same policy "
               "was installed twice, or the dispatcher is not calling through नीतिः at all "
               "— and if it is not, the interface is decoration and nothing can be swapped "
               "in later")
if not priority_violations(B):
    bad.append("चक्रम् never once ran a thread while a better-placed one was runnable, so "
               "it is obeying levels it is supposed to ignore — the two policies are two "
               "names for one policy, and the seam has not been shown to carry anything")
if (chooseD, accountD) != (chooseB, accountB):
    bad.append(f"the प्रेषकः run installed {chooseD:#x}/{accountD:#x} and not चक्रम्'s "
               f"{chooseB:#x}/{accountB:#x} — the same reason as the द्वारम् run, and it "
               "is also what makes the two comparable")
if (chooseE, accountE) != (chooseA, accountA):
    bad.append(f"the उन्नतिः run installed {chooseE:#x}/{accountE:#x} and not श्रेणी's "
               f"{chooseA:#x}/{accountA:#x} — the whole of this run is that the policy "
               "which reads levels is the one on the endpoint, and it is not")
if seqE == seqC:
    bad.append("the उन्नतिः run scheduled exactly as the द्वारम् run did, and they differ "
               "by the two words in नीतिः — so either श्रेणी was not installed or the "
               "levels it keeps are not reaching a decision")
# ---- the twin: the other side of C-002e3's difference -------------------
# The fast path's claim is not a count, it is a DIFFERENCE — the second half of a
# rendezvous costs one dispatcher entry today and must cost none. A difference is
# only a difference if the same thing stands on both sides, so the sixth run is
# the third one again: same चक्रम्, same द्वारसारणी, same endpoint, nothing else
# touched. Today it is WORD FOR WORD the third, and that is what is demanded here
# — when the fast path lands, the diff between these two logs is the fast path
# and nothing else, and this arm tightens to "differs only in the handoff words"
# rather than being deleted.
#
# `C-002e3c` adds the SECOND HALF of that same claim: the sixth run proves the
# fast path in the RELEASE direction only (a sender hands straight to the waiter
# it just woke), and the seventh is the FOURTH run again with द्रुतपथः set, which
# is the other direction — a receiver that finds a parked sender hands straight
# back to it. The arm is therefore one function applied twice and not two copies
# of it: a twin that drifted from its own second instance would be two chances to
# disagree with itself, which is the reason no other arm is applied to a यमज run
# at all.
def twin(name, base_name, base, fast, n_base, n_fast, box_base, box_fast,
         id_base, id_fast, row):
    # `C-002e3` landed the fast path in the यमज runs and only there, so the arm
    # TIGHTENED rather than loosened: the two logs must still have the same
    # length and must still agree word for word EVERYWHERE EXCEPT the handoff
    # words. A handoff is self-declaring — event 4, and intended differs from
    # actual — so the set of positions allowed to differ is read off the यमज run
    # itself and is not a list kept here. Everything else is what it was: same
    # turns, same threads, same units, same levels, same work left, same offered
    # set.
    if id_fast != id_base:
        bad.append(f"the {name} run installed {id_fast[0]:#x}/{id_fast[1]:#x} and the "
                   f"{base_name} run {id_base[0]:#x}/{id_base[1]:#x} — the whole of "
                   "this run is that it is that one again, and a run under a "
                   "different policy is not a control for anything")
    handoffs = [t for t in range(n_fast) if unpack(fast[t])[6] == HANDOFF]
    if n_fast != n_base:
        bad.append(f"the {base_name} run took {n_base} turns and the {name} run "
                   f"{n_fast}. The fast path removes DISPATCHER ENTRIES, not turns: "
                   "the same threads run in the same order and the same number of "
                   "words are written, two of them on the entry that hands over")
    elif not handoffs:
        bad.append(f"the {name} run carries no handoff word at all, so the fast path "
                   f"did not fire in the run that installs it — and a control which "
                   "is identical to its subject measures nothing. This arm asserted "
                   f"word-for-word equality until {row}; it does not any more, and an "
                   "unchanged log is now the failure it used to be the pass")
    else:
        off = [t for t in range(n_base) if base[t] != fast[t]]
        stray = [t for t in off if t not in handoffs]
        if stray:
            t = stray[0]
            bad.append(f"the {name} run differs from the {base_name} run on turn "
                       f"{t + 1}, which is not a handoff: {base[t]:016x} against "
                       f"{fast[t]:016x}. The fast path is the only difference between "
                       "these two runs, so a turn that moves without being a handoff "
                       "is a scheduling decision that changed — either the दौड़ do not "
                       "start from the same state, or गणनम् is not being called for a "
                       "thread that ran")
        same = [t for t in handoffs if base[t] == fast[t]]
        if same:
            t = same[0]
            bad.append(f"turn {t + 1} of the {name} run is marked a handoff and is "
                       f"word for word the {base_name} run's turn: {fast[t]:016x}. A "
                       "handoff word is the one place the two logs may differ, and "
                       "one that does not differ is a label on nothing")
    if any(unpack(e)[6] == HANDOFF for e in base):
        bad.append(f"the {base_name} run carries a handoff word, so the fast path is "
                   "installed in the run that is supposed to be the baseline — the "
                   f"difference {row} measures would then be between two fast paths")
    if box_fast != box_base:
        bad.append(f"the {name} run's inboxes came out {box_fast} and the "
                   f"{base_name} run's {box_base}, and the two runs are the same run "
                   "— a message that lands differently in a copy of a run is a "
                   "message that was not decided by the queue")
    return handoffs


handoffs = twin("यमज", "द्वारम्", C, F, nC, nF, boxC, boxF,
                (chooseC, accountC), (chooseF, accountF), "C-002e3")
# And the same again in the OTHER DIRECTION. The fourth run is the rendezvous
# read backwards — the sender sits down and the receiver finds it — so its twin
# is what says the fast path is direction-blind rather than proved once on the
# side that happened to be built first.
handoffsG = twin("प्रेषकयमज", "प्रेषकः", D, G, nD, nG, boxD, boxG,
                 (chooseD, accountD), (chooseG, accountG), "C-002e3c")
if (chooseC, accountC) != (chooseB, accountB):
    bad.append(f"the द्वारम् run installed {chooseC:#x}/{accountC:#x} and not चक्रम्'s "
               f"{chooseB:#x}/{accountB:#x} — the order a wait queue comes out in must not "
               "be attributable to a policy that sorts, and चक्रम् is the one that cannot")
# ---- उपकार्यम्: the decision is made outside the kernel (`C-002i2a`) -----
# Written red before the program. Every arm here is read off the log and the
# inboxes; none of it replays a policy, and none of it could be satisfied by a
# kernel that answered its own question.
structural("उपकार्यम्", H)
seqH = [unpack(e)[0] for e in H]
if (chooseH, accountH) in ((chooseA, accountA), (chooseB, accountB)):
    bad.append(f"the उपकार्यम् run installed {chooseH:#x}/{accountH:#x}, which is one of "
               "the two policies already in the image. The seam's claim is that a THIRD "
               "thing can be written into those two words, and a policy that is already "
               "there has not been shown to be replaceable by anything new")
if seqH == seqA or seqH == seqB:
    bad.append("the उपकार्यम् run scheduled in exactly the order "
               + ("श्रेणी" if seqH == seqA else "चक्रम्") + " did. Then nothing "
               "distinguishes a decision taken in userspace from one the kernel took "
               "itself, which is the only thing this run exists to show")
acts = [t for t, e in enumerate(H, 1) if unpack(e)[6] == UPCALL]
if not acts:
    bad.append(f"no turn of the उपकार्यम् run carries event {UPCALL} — the kernel never "
               "called out at all, so whatever chose these threads was in the image's "
               "kernel and the upcall is a name for a routine")
for t in acts:
    want, got, _, _, _, _, _ = unpack(H[t - 1])
    if got != NIYAMAKA:
        bad.append(f"turn {t} of the उपकार्यम् run is marked an upcall and thread {got} "
                   f"ran on it, not नियामकः ({NIYAMAKA}) — the call went somewhere, but "
                   "not out to the scheduler it was addressed to")
    if want == NIYAMAKA:
        bad.append(f"turn {t} of the उपकार्यम् run is marked an upcall and चयनम् had "
                   "NAMED नियामकः. Then it is an ordinary turn wearing the label: an "
                   "upcall is the case where the thread that runs is the one the policy "
                   "went out to ASK, and it cannot have asked for itself")
for t, e in enumerate(H, 1):
    want, _, _, _, _, _, ev = unpack(e)
    if want == NIYAMAKA and ev != UPCALL:
        bad.append(f"turn {t} of the उपकार्यम् run chose नियामकः as ordinary work. It has "
                   "no work; it is the party being asked, and a scheduler that competes "
                   "for the slice it is handing out is not the seam doc 11 §3.4 asks for")
    if ev == UPCALL:
        continue
    cand = [i for i in offered(e) if i != NIYAMAKA]
    if cand and want != max(cand):
        bad.append(f"turn {t} of the उपकार्यम् run offered {cand} and चयनम् named {want}, "
                   f"not {max(cand)}. नियामकः decides by highest runnable index — a rule "
                   "neither श्रेणी nor चक्रम् can express, because neither has any notion "
                   "of index order — and a choice that does not follow it was not made by "
                   "the thread this run says made it")
if H:
    _, _, _, lv0, wk0, m0, _ = unpack(H[0])
    if lv0 != U_LEVEL or wk0 != U_WORK:
        bad.append(f"the उपकार्यम् run began at levels {lv0} works {wk0}, expected "
                   f"{U_LEVEL} / {U_WORK} — नियामकः must start with no work of its own "
                   "and the other three equal, or the order below is a level's doing")
    want0 = sum(1 << i for i in range(N) if not U_BLOCKED[i] and U_WORK[i])
    if m0 != want0:
        bad.append(f"the उपकार्यम् run's first choice was made from {m0:#06b}, expected "
                   f"{want0:#06b} — नियामकः starts PARKED on the endpoint, so it is never "
                   "in the set, and a run that starts elsewhere is a different program")
    if len(H) >= BUDGET - 1:
        bad.append(f"the उपकार्यम् run took {len(H)} turns and stopped on the dispatcher's "
                   "budget rather than because the work ran out")
    end_ran = {unpack(e)[0] for e in H}
    missing = [i for i in range(N) if i != NIYAMAKA and i not in end_ran]
    if missing:
        bad.append(f"the उपकार्यम् run never ran thread(s) {missing} — a userspace "
                   "scheduler that starves a thread outright is not bounded scheduling, "
                   "it is no scheduling")
# The message. An inbox is printed once, at the end, so this ties the LAST
# activation to words that actually crossed; the per-turn evidence is the event
# word above. Word 3 is the one word no party already in this program could have
# written — the sender is the kernel, and the kernel is not a thread.
if boxH[NIYAMAKA][3] != KERNEL:
    bad.append(f"नियामकः's inbox carries sender {boxH[NIYAMAKA][3]} and not {KERNEL}. The "
               "identity in the fourth word is what says the call came from the KERNEL "
               "rather than from another thread, and a sender that is one of the four "
               "threads is the rendezvous this program already had")
if acts:
    want_set = unpack(H[acts[-1] - 1])[5]
    if boxH[NIYAMAKA][0] != want_set:
        bad.append(f"the last upcall offered {want_set:#06b} and नियामकः's inbox holds "
                   f"{boxH[NIYAMAKA][0]:#06b} in its first word. The set the dispatcher "
                   "built and the set the scheduler was told are printed by two different "
                   "parties, and the whole claim is that the second is the first — if it "
                   "is not, नियामकः decided about a machine that does not exist")
for k in range(N):
    if k != NIYAMAKA and boxH[k] != UNSENT:
        bad.append(f"thread {k}'s inbox in the उपकार्यम् run came out {boxH[k]}, and "
                   "nobody sent it anything — the upcall speaks to नियामकः alone")
# ---- मौनम्: the upcall that is never answered (`C-002i2c1a`) ------------
# Written red before the program. The kernel is not told the scheduler will be
# silent — it finds out by asking and getting nothing back, and every arm here
# is read off the log the same way the eighth run's are.
structural("मौनम्", I)
if (chooseI, accountI) != (chooseH, accountH):
    bad.append(f"the मौनम् run installed {chooseI:#x}/{accountI:#x} and the उपकार्यम् run "
               f"{chooseH:#x}/{accountH:#x} — this run is the eighth one with a SILENT "
               "नियामकः, so the two words in नीतिः must be the same two words. A kernel "
               "given a different policy for the case is a kernel that was told in "
               "advance, and the claim is that it finds out")
if I and H and I[0] != H[0]:
    bad.append(f"the मौनम् run's first turn is {I[0]:016x} and the उपकार्यम् run's is "
               f"{H[0]:016x}. Nothing has been answered yet on turn 1 and the kernel "
               "cannot know anything, so the first question must be the same question — "
               "the two runs may diverge only after an answer failed to arrive, or what "
               "changed was the table and not the silence")
asks = [t for t, e in enumerate(I, 1) if unpack(e)[6] == UPCALL]
gaveup = [t for t, e in enumerate(I, 1) if unpack(e)[6] == GIVEUP]
if not asks:
    bad.append(f"no turn of the मौनम् run carries event {UPCALL} — the kernel never asked "
               "at all, so there is no silence here to survive, only a kernel deciding "
               "for itself under another name")
if asks and asks != list(range(1, len(asks) + 1)):
    bad.append(f"the मौनम् run's upcall turns are {asks} and not the first {len(asks)} "
               "turns of the run. उपकार्यचयनम् consumes an answer when it uses one, so "
               "nothing can run between an unanswered question and the next ask; a work "
               "turn in among them means something was answered and this run is not the "
               "silence it says it is")
if asks and len(asks) != ASKS:
    bad.append(f"the मौनम् run asked {len(asks)} times before giving up and the bound is "
               f"{ASKS}. Asking again is the only evidence in this log that the last "
               "question went unanswered, so one ask proves no silence — and a bound that "
               "is not the stated one is not a bound the program holds itself to")
for t in asks:
    want, got, used, _, _, _, _ = unpack(I[t - 1])
    if got != NIYAMAKA or want == NIYAMAKA:
        bad.append(f"turn {t} of the मौनम् run is marked an upcall, चयनम् named {want} and "
                   f"thread {got} ran — an upcall is the case where the thread that runs "
                   f"is the one that was asked ({NIYAMAKA}) and it cannot have asked "
                   "itself")
    if used:
        bad.append(f"turn {t} of the मौनम् run is an upcall and {used} units were eaten. "
                   "नियामकः has no work of its own; a scheduler that eats the slice it "
                   "was handed to allocate is doing something other than being asked")
if len(gaveup) != 1:
    bad.append(f"{len(gaveup)} turns of the मौनम् run carry event {GIVEUP} and exactly one "
               "must. Giving up is a thing that happens once: a run that never declares "
               "it is bounded only by this file's SIGKILL, which is the harness rescuing "
               "the machine, and a run that declares it repeatedly has renamed its loop")
if gaveup and asks and gaveup[0] != asks[-1] + 1:
    bad.append(f"the मौनम् run gave up on turn {gaveup[0]} and its last ask was turn "
               f"{asks[-1]} — the give-up is the turn the last unanswered question runs "
               "out on, and a gap between them is turns the kernel spent neither "
               "asking nor deciding")
if gaveup and [t for t in asks if t > gaveup[0]]:
    bad.append(f"the मौनम् run asked again on turn(s) {[t for t in asks if t > gaveup[0]]} "
               f"after declaring the upcall broken on turn {gaveup[0]} — a kernel that "
               "keeps asking a scheduler it has already given up on has not defined an "
               "outcome")
for t, e in enumerate(I, 1):
    want, _, _, _, _, _, ev = unpack(e)
    if want == NIYAMAKA and ev != UPCALL:
        bad.append(f"turn {t} of the मौनम् run chose नियामकः as ordinary work. It has no "
                   "work, and a silent scheduler is still not a candidate for the slice "
                   "it stopped handing out")
    if ev == UPCALL or not gaveup or t < gaveup[0]:
        continue
    cand = [i for i in offered(e) if i != NIYAMAKA]
    if cand and want != min(cand):
        bad.append(f"turn {t} of the मौनम् run is after the give-up, offered {cand}, and "
                   f"चयनम् named {want} rather than {min(cand)}. The fallback is the "
                   "LOWEST runnable index — the exact inverse of नियामकः's rule, so that "
                   "no fallback turn can be read as an answered one — and a choice that "
                   "follows neither rule was made by nobody this run names")
if I and [unpack(e)[0] for e in I] == seqH:
    bad.append("the मौनम् run scheduled in exactly the order the उपकार्यम् run did. The "
               "fallback is the inverse of the rule नियामकः answers by, so identical "
               "orders mean either the silence changed nothing or somebody is still "
               "answering")
if I:
    _, _, _, lv0, wk0, m0, _ = unpack(I[0])
    if lv0 != U_LEVEL or wk0 != U_WORK:
        bad.append(f"the मौनम् run began at levels {lv0} works {wk0}, expected {U_LEVEL} / "
                   f"{U_WORK} — it is the उपकार्यम् run's table with a silent नियामकः, and "
                   "a different table makes it a different program instead")
    if len(I) >= BUDGET - 1:
        bad.append(f"the मौनम् run took {len(I)} turns and stopped on the dispatcher's "
                   "budget. Running out of budget is the machine being switched off, not "
                   "an outcome it defined: the whole of this run is that the work still "
                   "finishes when the scheduler goes quiet")
    end_ran = {unpack(e)[0] for e in I}
    missing = [i for i in range(N) if i != NIYAMAKA and i not in end_ran]
    if missing:
        bad.append(f"the मौनम् run never ran thread(s) {missing} — a kernel whose fallback "
                   "starves a thread outright has survived the silence and lost the "
                   "machine anyway")
    want, _, used, _, wk, _, _ = unpack(I[-1])
    end = list(wk); end[want] -= used
    if any(x for x in end):
        bad.append(f"the मौनम् run ended with work {end} still outstanding — it stopped "
                   "scheduling while threads were still runnable, which is the silence "
                   "reaching the work rather than being contained")
# The question still crossed, ASKS times, and the last one is the one the inbox
# ties to words that travelled — the same limit the eighth run states.
if boxI[NIYAMAKA][3] != KERNEL:
    bad.append(f"in the मौनम् run नियामकः's inbox carries sender {boxI[NIYAMAKA][3]} and "
               f"not {KERNEL} — it was asked {ASKS} times before the kernel gave up, so "
               "the words of the last question must be there with the kernel's identity "
               "on them; an empty inbox means the silence was never even asked into")
if asks:
    want_set = unpack(I[asks[-1] - 1])[5]
    if boxI[NIYAMAKA][0] != want_set:
        bad.append(f"the मौनम् run's last question offered {want_set:#06b} and नियामकः's "
                   f"inbox holds {boxI[NIYAMAKA][0]:#06b} — the set the dispatcher built "
                   "and the set the scheduler was handed are printed by two parties and "
                   "the claim is that they are one set")
for k in range(N):
    if k != NIYAMAKA and boxI[k] != UNSENT:
        bad.append(f"thread {k}'s inbox in the मौनम् run came out {boxI[k]}, and nobody "
                   "sent it anything — the upcall speaks to नियामकः alone, silent or not")
# ---- अनशनम्: the turn a BLOCKED thread lost, and getting it back (`C-002i2c2a`)
# Written red before the program. The clock starts where `starvation()` stops
# asking — the turn the cycler is back in the set — and nothing here is asked of
# it while it is out, because that wait is the one it asked for.
structural("अनशनम्", J)
if (chooseJ, accountJ) != (chooseB, accountB):
    bad.append(f"the अनशनम् run installed {chooseJ:#x}/{accountJ:#x} and चक्रम् is "
               f"{chooseB:#x}/{accountB:#x}. Round-robin alone is what this run must be "
               "ordered by: it never reads a level, so no part of the order can be laid "
               "at the door of the lift a blocking thread earns — that is C-002e6's "
               "claim, made once, over a single block")
invariants("अनशनम्", J, init=(S_LEVEL, S_WORK, S_BLOCKED, S_APPETITE),
           waiting=lambda t: {CYCLER})
# Releases and self-blocks are read off the set and the event word, never off
# the endpoint's array — the same rule the द्वारम् runs are held to.
releases = [t for t, e in enumerate(J, 1)
            if CYCLER in offered(e) and (t == 1 or CYCLER not in offered(J[t - 2]))]
selfblocks = [t for t, e in enumerate(J, 1)
              if unpack(e)[0] == CYCLER and unpack(e)[6] == BLOCK]
if len(releases) != CYCLES:
    bad.append(f"thread {CYCLER} came back into the set {len(releases)} time(s) in the "
               f"अनशनम् run and {CYCLES} are asked for. One block is an incident and two "
               "a coincidence: it takes a third before coming round is a property of the "
               "scheduler rather than a thing that happened once")
if len(selfblocks) < CYCLES - 1:
    bad.append(f"thread {CYCLER} blocked itself on turn(s) {selfblocks} of the अनशनम् run "
               f"and at least {CYCLES - 1} are asked for. A thread blocked at startup and "
               "released has been shown to come back ONCE; that it goes back down again by "
               "its own action is the whole of 'blocks, is released and blocks again'")
for t in selfblocks:
    if not [x for x in range(t + 1, len(J) + 1) if CYCLER not in offered(J[x - 1])]:
        bad.append(f"thread {CYCLER} blocked itself on turn {t} of the अनशनम् run and was "
                   "in the set on every turn after it — a block that costs no turn is a "
                   "label, and there is then no lost turn to give back")
for t in releases:
    later = [x for x, e in enumerate(J, 1) if x >= t and unpack(e)[0] == CYCLER]
    if not later:
        bad.append(f"thread {CYCLER} was put back into the set on turn {t} of the अनशनम् "
                   f"run with work left and was never chosen again in {len(J)} turns — "
                   "starved outright, which is the case C-002i could not reach and the "
                   "reason this run exists")
        continue
    ran = [unpack(J[x - 1])[0] for x in range(t, later[0])]
    twice = sorted({i for i in ran if ran.count(i) > 1})
    if twice:
        bad.append(f"thread {CYCLER} came back into the set on turn {t} of the अनशनम् run "
                   f"and was not chosen until turn {later[0]}, and thread(s) {twice} ran "
                   "twice in between. The turn it lost by waiting went to somebody who "
                   "had already had one — that is the bound, and it is the same bound "
                   "starvation() states for a thread that never leaves the set, asked "
                   "across the block instead of around it")
# The releases are messages, so the last one is tied to words that crossed. The
# fourth word is thread 0's identity and not the kernel's: a release is not an
# activation, and a zero there would say a store woke it rather than a rendezvous.
if boxJ[CYCLER] != MESSAGES[CYCLES - 1] + [SENDER]:
    bad.append(f"thread {CYCLER}'s inbox in the अनशनम् run holds {boxJ[CYCLER]} and the "
               f"{ORD[CYCLES - 1]} message with its sender is {MESSAGES[CYCLES - 1] + [SENDER]}. "
               "The releases carry the three rows in send order, so the last one is fixed "
               "by the same table the द्वारम् run's are — an inbox that holds something "
               "else was filled by something other than the release this run counts")
for k in range(N):
    if k != CYCLER and boxJ[k] != UNSENT:
        bad.append(f"thread {k}'s inbox in the अनशनम् run came out {boxJ[k]}, and nobody "
                   f"sent it anything — thread {SENDER} speaks to the cycler alone here")
addrs = {"श्रेणीचयनम्": chooseA, "श्रेणीगणनम्": accountA,
         "चक्रचयनम्": chooseB, "चक्रगणनम्": accountB,
         "उपकार्यचयनम्": chooseH, "उपकार्यगणनम्": accountH}
for nm, a in addrs.items():
    if not 0x80000000 <= a < 0x81000000:
        bad.append(f"नीतिः held {a:#x} for {nm}, which is not an address in the loaded image")
if len(set(addrs.values())) != 6:
    bad.append(f"the six policy entry points are not six distinct addresses: "
               + ", ".join(f"{n}={a:#x}" for n, a in addrs.items()))

if bad:
    print("\n".join(bad)); sys.exit(1)
print(f"श्रेणी ran {nA} turns and चक्रम् {nB}, both finishing all the work; priority "
      f"ordered every श्रेणी turn, thread 3 climbed {drops} levels by behaving and then "
      f"ran ahead of threads still stuck below it, and the two orders differ. द्वारम् ran "
      f"{nC}: threads {WAITERS} sat down on the endpoint in that order, came back in it, "
      "and each came back holding the message sent to the head at the time. प्रेषकः ran "
      f"{nD}: thread {SND} sent into an empty endpoint and had to sit down on it itself, "
      f"and thread {RCV} took the four words out of it without waiting at all. उन्नतिः "
      f"ran {nE}: the same run as द्वारम् with श्रेणी installed, and every thread that "
      "sat down below the top level was lifted for eating nothing and then ran ahead of "
      f"a thread standing where the block had found it. यमज ran {nF}: the द्वारम् run "
      f"again with the fast path installed, word for word except on turns "
      f"{[t + 1 for t in handoffs]}, where a thread ran without चयनम् naming it — the "
      "same threads, in the same order, eating the same units, on fewer dispatcher "
      f"entries. प्रेषकयमज ran {nG}: the प्रेषकः run again with the fast path "
      f"installed, word for word except on turns {[t + 1 for t in handoffsG]} — the "
      "same difference in the receive direction, where it is the arriving receiver "
      f"that hands straight back to the sender it just unparked. उपकार्यम् ran {nH}: "
      f"a third policy in the same two words, {len(acts)} of those turns a call OUT to "
      "नियामकः, and every other turn the highest-numbered runnable thread — a rule no "
      f"kernel policy in this image can express. मौनम् ran {nI}: the same run with a "
      f"नियामकः that answers nothing, asked {len(asks)} times, given up on at turn "
      f"{gaveup[0] if gaveup else '-'} by the program's own bound rather than by this "
      f"file's SIGKILL, and every thread's work finished anyway. अनशनम् ran {nJ}: thread "
      f"{CYCLER} went down onto the endpoint and came back {len(releases)} times, and on "
      "none of those returns did another thread take a second turn ahead of it — the "
      "fairness the other runs state for a thread that never leaves the set, asked "
      "across the block")
print("MOVES " + " ".join(f"{x:016x}" for x in (chooseA, accountA, chooseB, accountB,
                                                chooseH, accountH)))
print("FIXED " + " ".join(f"{x:016x}" for x in
                          [nA] + A + [nB] + B + [nC] + C + [nD] + D + [nE] + E
                          + [nF] + F + [nG] + G + [nH] + H + [nI] + I + [nJ] + J
                          + sum(boxA + boxB + boxC + boxD + boxE + boxF + boxG
                                + boxH + boxI + boxJ, [])))
PY
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(sed -n 1p "$tmp/v")"

    moves=$(sed -n 's/^MOVES //p' "$tmp/v"); fixed=$(sed -n 's/^FIXED //p' "$tmp/v")
    if [ -z "$first" ]; then first="$moves|$fixed"; continue; fi
    # The four entry points are computed with auipc and stored into नीतिः, so
    # they must MOVE — a single run cannot tell a computed address from a lucky
    # constant, and these four are the whole seam.
    if [ "$moves" = "${first%|*}" ]; then
        echo "  FAIL  नीतिः held the same six addresses at both link addresses, so they are"
        echo "        constants rather than the entry points of this image — and a vector of"
        echo "        constants cannot have a new policy written into it"
        fail=1
    fi
    # And the schedule must NOT. Which thread runs when cannot depend on where
    # the image was linked; if it does, something is reading an address as data.
    if [ "$fixed" != "${first#*|}" ]; then
        echo "  FAIL  the two runs scheduled differently at the two link addresses — a"
        echo "        scheduling decision must not depend on where the image is linked"
        fail=1
    fi
done
[ "$fail" -eq 0 ] || { echo; echo "the scheduler is not doing what the row claims."; exit 1; }
echo
echo "ok  priority orders the run and round-robin keeps equals from starving; a thread"
echo "    that eats less than its slice climbs out of the bottom level and overtakes the"
echo "    ones still there; and the whole policy is two words in नीतिः — replacing them"
echo "    changes the order through a dispatcher that was not touched. A blocked thread"
echo "    is withheld by the dispatcher under BOTH policies, neither of which can see"
echo "    the block, and runs again once another thread clears it. And on द्वारम्"
echo "    three threads sat down on ONE endpoint by themselves, in an order that is"
echo "    not their index order, and came back in the order they sat down — read off"
echo "    the turns they resumed on, not off the queue they resumed from. And the"
echo "    releaser is now a SENDER: each of the three came back holding the four words"
echo "    that were sent to the head at the moment it was released — read off what the"
echo "    RECEIVER recorded out of its own registers, never off the sender's table."
echo "    And in the fourth run the rendezvous runs the other way: a send into an"
echo "    endpoint nobody was waiting on sat the SENDER down on that same queue, its"
echo "    four words parked in its own saved context, and the receive that came two"
echo "    turns later took them without waiting a turn itself. And the last two runs"
echo "    are those two runs again with the fast path installed — one per direction —"
echo "    each word for word its own original except on the turns that declare"
echo "    themselves handoffs, which is what says गणनम् was called for BOTH threads"
echo "    that ran on a handoff entry and not only for the one चयनम् named. And the"
echo "    eighth run is the upcall doc 11 §3.4 asks for by name: a THIRD policy in the"
echo "    same two words, whose चयनम् decides nothing and instead sends the offered set"
echo "    out through द्वारम् to नियामकः — a userspace thread — and returns what comes"
echo "    back. What says the decision left the kernel is that every choice follows a"
echo "    rule neither policy in the image can express, and that the four words नियामकः"
echo "    was holding name a sender that is not any thread. And the ninth run is that"
echo "    same run with a नियामकः that answers NOTHING: the kernel is not told, it finds"
echo "    out by asking again — which it can only do because the last answer never came"
echo "    — and after a bound written into the program it declares the upcall broken,"
echo "    falls back to a rule of its own and finishes every thread's work. The bound"
echo "    is the machine's, not this file's SIGKILL. And the tenth run asks the fairness"
echo "    question of a thread that is NOT in the offered set: one that blocks on the"
echo "    endpoint, is released, and blocks again, three times over. From the turn it is"
echo "    back in the set until the turn it is chosen, nobody takes a second — the turn"
echo "    it lost by waiting is given back to it, and every thread's work still finishes."
