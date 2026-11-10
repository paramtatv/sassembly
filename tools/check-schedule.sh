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
    "$root/tools/sadhana-native.sh" \
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
    if ! awk -f "$root/tools/check-schedule-oracle.awk" "$tmp/l" > "$tmp/v" 2>&1
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
