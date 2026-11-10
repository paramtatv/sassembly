#!/bin/sh
# The pager HANDOFF — task `C-002h2a`, doc 11 §3.3 and §7, extended to a SECOND
# fault answered with a DIFFERENT frame by `C-002h2c1`, to a THIRD fault the
# pager REFUSES by `C-002h2c2a`, to a FOURTH where the pager names a frame that
# is not its and the KERNEL refuses by `C-002h2c3a`, and to a FIFTH the pager
# does not answer AT ALL by `C-002h2c2c1`.
#
# ## What `C-002h` left open, in its own words
#
# `tools/check-pager.sh` proved the PACKAGE: a page fault carries `stval`,
# `scause` and `sepc` unaltered, load, store and instruction stay tellable
# apart on one gigapage, and each fault is delivered exactly once. It then said
# what it could not reach:
#
#     The pager is not a separate thread. […] without synchronous IPC there is
#     no way to carry a fault to another thread at all. So the handler stands
#     in the pager's place […]. The HANDOFF is a row after `C-002e`.
#
# `C-002e` is done, and `C-002m1`/`C-002m2` carried a word across an
# address-space boundary and a reply back. So the way is open, and this is the
# contract for walking it: the fault package must leave the faulting process's
# address space and be read, in another one, by a pager that is NOT the thread
# that faulted — and the faulting instruction must then run again and succeed.
#
# ## The claim, as sixty-two words in the order the program runs
#
#    1  satp as installed for F, the faulter    Sv39, mode nibble 8
#    2  satp as installed for P, the pager      Sv39, and the frame after F's
#    3  scause the kernel took at F's fault     13, load page fault
#    4  stval  the kernel took                  the address F touched
#    5  sepc   the kernel took                  F's faulting instruction
#    6  the cause P read out of ITS OWN inbox   exactly word 3
#    7  the stval P read                        exactly word 4
#    8  the sepc  P read                        exactly word 5
#    9  scause on P's ecall answer              exactly 8 — P ran in U-mode
#   10  the frame P NAMED, as the kernel got it frame 11
#   11  the leaf the kernel then installed      frame 11, V R U A, no W, no X
#   12  scause at F's SECOND fault              13 again
#   13  stval  at the second fault              the next leaf — NOT word 4
#   14  sepc   at the second fault              the next load — NOT word 5
#   15  the word F read on the first resume     frame 11's own address
#   16  the cause P read the second time        exactly word 12
#   17  the stval P read                        exactly word 13
#   18  the sepc  P read                        exactly word 14
#   19  scause on P's second ecall              exactly 8
#   20  the frame P named the second time       frame 12 — NOT word 10
#   21  the leaf the kernel then installed      frame 12 — NOT the frame in word 11
#   22  scause at F's THIRD fault               13 again
#   23  stval  at the third fault               the leaf after word 13
#   24  sepc   at the third fault               the third load
#   25  the word F read on the second resume    frame 12's own address
#   26  the cause P read the third time         exactly word 22
#   27  the stval P read                        exactly word 23
#   28  the sepc  P read                        exactly word 24
#   29  scause on P's third ecall               exactly 8 — a refusal is an ANSWER
#   30  the frame P named the third time        0 — THE REFUSAL
#   31  that leaf, read back after the refusal  0 — nothing was installed
#   32  scause when F next trapped              8 — F ran, and called
#   33  sepc there                              F's own refusal entry, NOT word 24
#   34  what F wrote down about the refusal     exactly word 23
#   35  scause at F's FOURTH fault              13 again
#   36  stval  at the fourth fault              the leaf after word 23
#   37  sepc   at the fourth fault              the fourth load
#   38  the cause P read the fourth time        exactly word 35
#   39  the stval P read                        exactly word 36
#   40  the sepc  P read                        exactly word 37
#   41  scause on P's fourth ecall              exactly 8 — P DID answer
#   42  the frame P named the fourth time       NOT zero — a real frame, and
#                                               frame 0, F's own root table
#   43  the first frame the kernel says P owns  frame 11, out of its own record
#   44  the first frame it says P does NOT own  frame 13, and word 42 is
#                                               outside [word 43, word 44)
#   45  that leaf, read back after the refusal  0 — the KERNEL installed nothing
#   46  scause when F next trapped              8 — F ran, and called
#   47  sepc there                              exactly word 33 — F's own
#                                               refusal entry, a second time
#   48  what F wrote down about the distrust    exactly word 36
#   49  scause at F's FIFTH fault               13 again
#   50  stval  at the fifth fault               the leaf after word 36
#   51  sepc   at the fifth fault               the fifth load
#   52  scause when the DEADLINE expired        a supervisor TIMER interrupt,
#                                               8000000000000005
#   53  satp at that moment                     exactly word 2 — P was running
#   54  sepc at that moment                     on the user page, and none of
#                                               F's instructions
#   55  that leaf, read back after the deadline 0 — nothing was installed
#   56  scause when F next trapped              8 — F ran, and called
#   57  sepc there                              exactly word 33 — the same entry
#                                               a third time
#   58  what F wrote down about the silence     exactly word 50
#   59  faults delivered                        exactly 5
#   60  deadlines that expired                  exactly 1
#   61  answers the pager gave                  exactly 4 — two frames, one
#                                               refusal, and one frame that was
#                                               not its to give
#   62  answers the KERNEL refused              exactly 1 — word 42, and only it
#
# Words 6, 7 and 8 are the handoff. Words 3, 4 and 5 are what the KERNEL saw;
# 6, 7 and 8 are what a process under a DIFFERENT `satp` found in its own
# inbox. Their equality is a package that crossed a boundary. Word 15 is the
# resumption: F re-runs the very load that faulted and this time it reads the
# frame the pager named.
#
# ## The word for what the pager SAID — the debt `C-002h2c1` wrote down
#
# `C-002h2c1` found that two different defects printed byte-identical output:
# a kernel that threw the pager's answer away and mapped the frame it would
# have chosen anyway, and a pager that answered twice with the same frame
# because it never bumped its own slot. It said why, and refused to guess:
#
#     The answer travels only in a register and nothing in these twenty-one
#     words watches it in flight. […] Watching the answer in flight would need
#     a word for what P said, and that is a contract change, not a message
#     change.
#
# This is that contract change. Words 10, 20 and 30 are the frame P NAMED, as
# the kernel received it, printed BEFORE the kernel acts on it. With them the
# two defects separate: a kernel that ignores the answer prints a named frame
# that its installed leaf does not match, and a pager that does not bump prints
# word 20 equal to word 10. Neither can any longer wear the other's message.
#
# ## The third fault, which the pager REFUSES
#
# P owns two frames and hands them out in order, bumping its own slot by 4096
# before each answer. After two answers the slot has walked past the last frame
# it owns, and the honest thing for a pager in that position is to say so. So
# the third fault is answered with NO FRAME: word 30 is zero.
#
# Zero is not a frame a pager could be handing out. Frame 0 of this layout is
# F's own root table, so a pager naming it is naming something it does not own
# under any reading; zero is therefore free to mean the refusal, and it means
# only that. `C-002h2c3a` takes that sentence at its word and makes the pager
# name frame 0 anyway — see the fourth fault below. The two do not collide:
# frame 0 is at `base`, and `base` is not zero.
#
# ## What F is owed, which is a defined outcome and not a hang
#
# A refusal that only the kernel knows about is a hang with better manners: if
# the kernel resumes F at the faulting load, that load faults again, forever.
# So the contract demands three things F can be held to.
#
#   - **Nothing was installed** (word 31). The kernel reads the leaf back at
#     its identity address AFTER the refusal and prints it, and it must be
#     zero. A refusal that quietly maps something is a resolution wearing the
#     word "no".
#   - **F ran again, and not at the load** (words 32 and 33). F is resumed at
#     an address it chose in advance and left where the kernel could find it,
#     and it gets there and calls. `sepc` at that call is on the shared user
#     page and is NOT word 24, so F did not re-run the load that was refused.
#   - **F was told WHICH address** (word 34), and told it in a way that only
#     F's own instructions could produce: F writes the refused address into a
#     slot of its own inbox, through its own map, and the kernel reads that
#     slot back. A register the kernel set and then printed would prove
#     nothing — it would be the kernel reading back its own handwriting.
#
# Together those are a refusal F can tell apart from a resolution: on a
# resolution the load re-runs and reads a page (words 15 and 25); on a refusal
# the load never runs again and F writes down what it was refused.
#
# ## The fourth fault, where the pager names a frame that is not its
#
# Everything above is a pager that is honest. It hands out the two frames it
# owns and then says so when it has nothing left, and the kernel installs
# whatever it is told — words 11 and 21 are asserted to be EXACTLY the answer,
# and that is the whole point of them. So the kernel of the program above is a
# kernel that does whatever the pager says, and the only reason F's memory is
# safe from P is that the tables happen not to reach it. That is a property of
# the layout. Nothing checks.
#
# `C-002h2c3` is the row that makes it checked, and this is its contract. The
# fourth fault is answered — word 41 is 8, so P ran and called, and word 42 is a
# real page-aligned frame and not the zero that spells a refusal — and the frame
# it names is **frame 0, F's own root page table**.
#
# Frame 0 is chosen and not picked at random. It is real memory that is
# currently in use, it belongs to F and to the kernel both, and a kernel that
# installed it would hand F a readable mapping of its own page tables at a user
# address: the confused deputy in its most concrete form, where the damage is
# done by the kernel, with full privilege, on the word of a thread that had
# none. It is also the frame `C-002h2c2a` already named as the one no pager
# could be handing out, which is why zero was free to mean refusal — so the
# claim that was made in passing there is made a test here.
#
# ## What the kernel has to KNOW before it can refuse
#
# A kernel cannot refuse a frame on the strength of finding it suspicious. It
# has to hold a record of what this pager may hand out, and the record has to
# be the KERNEL's in exactly the sense the deadline is: written before P ever
# ran, out of P's reach, and not a copy of anything P can edit. P's own fifth
# inbox slot already holds the first frame it does not own — but that slot is in
# P's window and P bumps its neighbour every round, so a kernel that consulted
# it would be asking the accused.
#
# So words 43 and 44 are the record itself, printed at the moment it is used
# and read out of where the kernel keeps it: the first frame P owns (frame 11)
# and the first it does not (frame 13). Word 42 has to fall outside that
# half-open range, and it does — frame 0 is below it.
#
# A first frame and a limit is the SMALLEST thing that can be called ownership,
# and it is deliberately all this contract asks for. It is not an allocator, not
# a per-frame owner table, and not a way to say that two pagers own disjoint
# sets; there is one pager here and one range. What it does establish is the
# shape: the kernel consults a record of its own and refuses on the answer,
# rather than installing and hoping. A real owner map is a later row.
#
# ## What F is owed when the KERNEL is the one refusing
#
# The same three things a pager's refusal owes it, and the same three words —
# because F is not supposed to be able to tell which of the two happened. It
# asked for a page and did not get one; whether P had nothing to give or gave
# something it had no right to give is between the kernel and P.
#
#   - **Nothing was installed** (word 45), the leaf read back at the distrusted
#     address. This is the word that separates a kernel that REFUSED from one
#     that merely lost the answer somewhere: word 42 shows the kernel received a
#     frame, and word 45 shows it did not use it.
#   - **F ran again, at its own refusal entry** (words 46 and 47), and word 47
#     must EQUAL word 33 for the reason word 43 used to: one ending, not three.
#   - **F was told WHICH address** (word 48), in F's own hand, and it is word 36
#     — the distrusted address, not word 34's refused one.
#
# And word 62 is the enforcement counted: exactly one answer was refused by the
# kernel, across five faults. A kernel that refuses two has refused something it
# should have installed, and words 11 and 21 would already have said so; a
# kernel that refuses none installed frame 0.
#
# ## Why distrust comes BEFORE the silence, which is not a preference
#
# The fifth fault is the one nobody answers, and P reaches that state by walking
# its own slot past the bound and then waiting on an inbox slot nothing ever
# fills. That wait does not end. So a fault P has to ANSWER cannot be ordered
# after a fault P never answers — there would be nobody left to answer it. The
# distrusted answer is fault four because it is the last round in which the
# pager is still running, and the silence keeps the last place because it is
# the state the machine does not come back from.
#
# ## The fifth fault, which the pager does not answer AT ALL
#
# A refusal is an answer. Silence is not, and the difference is the whole of
# `C-002h2c2c`. Under the program `C-002h2c2b` left green, a pager that stopped
# replying would leave F's fault unresolved and the machine spinning in U-mode
# with nothing left to print, and the only thing that would end the run is the
# `kill -9` further down this file. That bound belongs to the harness. A kernel
# whose liveness is a property of the shell script that started it has no
# liveness at all: on a device there is no shell script.
#
# So the fifth fault is delivered to a pager that never comes back, and what
# is demanded is that the machine ends it ITSELF, at a bound the KERNEL set,
# and that F — who did nothing wrong and is not the one that stalled — gets the
# same defined outcome a refusal would have given it.
#
# ## Why the bound has to be an interrupt, which is not a choice
#
# A kernel that is not running cannot count. Once the kernel `sret`s into P,
# every instruction executed is P's, and P is under no obligation to give the
# processor back — that is what "never answers" MEANS. No loop counter in the
# kernel, no check after the ecall and no bound written in the pager's own code
# reaches a pager that has stopped executing the code the bound is written in.
# The only thing that takes the processor away from a U-mode thread that will
# not yield is a trap the kernel armed BEFORE it left, and for a deadline that
# is the supervisor timer.
#
# That is why word 52 is `8000000000000005` and not a number this contract was
# free to pick: bit 63 says interrupt, and code 5 is the supervisor timer.
# `spec/irq.sas` already arms it the same way — `time` (CSR 0xc01) plus a
# delta, handed to SBI `set_timer`, with `sie.STIE` open — so the mechanism is
# a thing this tree has done rather than a thing this contract invented.
#
# The deadline is therefore also the first thing in this program that is a
# POLICY: how long a pager may take is a number someone chose. The contract
# does not fix it. What it fixes is that the number is the kernel's, that it is
# armed before control leaves and disarmed when the answer arrives (words 60
# and 61: one deadline expired across five faults, and four answers given —
# so the four pagers that DID answer were not killed by the clock), and that
# expiry is survivable rather than fatal.
#
# ## Whose fault it is, and who pays for it — words 53 and 54
#
# A timer that fires is not by itself evidence that the PAGER was slow. It
# proves only that time passed. The word that makes it evidence is 53: `satp`
# as the kernel found it at the trap, which is the address space that was
# running when the deadline expired, and it must be word 2 — P's. A deadline
# that expired in F's address space is a deadline that fired at the wrong
# thread, and would resolve a fault F was still in the middle of taking.
#
# Word 54 is `sepc` at that trap: an instruction on the shared user page, and
# not any instruction of F's — not words 5, 14, 24, 37 or 51, and not word 33,
# which is where F waits. P is somewhere in its own code, and that is the
# picture of a stall rather than of a jump into the weeds.
#
# And then F stops paying for it. Words 55 to 58 are the refusal's own three
# demands, made a third time and for a third reason:
#
#   - **Nothing was installed** (word 55). A deadline is not permission to map
#     a frame nobody named. The kernel reads the leaf back at the silent
#     address, and it is zero — for the same reason words 31 and 45 are.
#   - **F ran again, at its own refusal entry** (words 56 and 57). This is
#     stronger than word 33 was: word 57 must EQUAL word 33. F chose that
#     address in advance for a fault that will not be resolved, and a silence it
#     can tell apart from a refusal by where it resumes is a silence it would
#     have to have a second handler for. The kernel does not invent a
#     destination for F; it uses the one F already left it.
#   - **F was told WHICH address** (word 58), written by F's own instructions
#     under F's own map into F's own inbox, and it is word 50 — the address
#     that went unanswered, not the one that was refused (word 34) and not the
#     one that was distrusted (word 48). Three reports from one entry, naming
#     three different addresses, is what says F learned each of them rather
#     than repeating itself.
#
# What this does NOT claim is that the pager is punished, replaced, or ever
# runs again. It is not descheduled, not killed, and not retried with another
# pager — and that is true of the pager that named frame 0 as much as of the one
# that said nothing: word 42 is answered with a refusal, not with a revocation.
# After word 58 the machine prints its counts and shuts down with P
# exactly as stuck as it was. Charging a stall to the thread that caused it,
# and giving F a pager that still works, needs a scheduler this program does
# not have — `C-002i2`, and a row after it.
#
# ## The second fault, which is the whole of `C-002h2c1`
#
# `C-002h2b` went green and could not tell an installed leaf the pager NAMED
# from one the kernel chose by itself: a kernel that threw the answer away and
# mapped frame 11 because frame 11 is what the layout says it hands back passes
# every word above identically. The difference has to be made to show, and the
# only way to make it show is to answer a second fault with a DIFFERENT frame.
#
# So the pager's fourth inbox slot is not a constant it reads: it is the frame
# it will hand out next, and P adds 4096 to it and writes it back BEFORE it
# answers. Frame 11 goes out the first time and frame 12 the second, and the
# kernel neither writes that slot nor reads it — the bump is the pager's, in
# U-mode, through its own window.
#
# **Words 11 and 21 must therefore install different frames, and words 15 and
# 25 must read different seeds.** A kernel that ignores the answer produces
# `11 == 21` and `15 == 25`, and this check names that case by name rather than
# reporting a mismatch, because it is the ONE failure `C-002h2c1` exists to
# catch. Since `C-002h2c2a` it also has words 10 and 20 to separate that case
# from a pager that never bumped its slot.
#
# The second faulting address is derived, not chosen: leaf `(image megapage +
# 1) & 511` of the same empty megapage 400. The `& 511` is counted rather than
# assumed — without it an image linked in the last megapage of a gigapage puts
# the second address in megapage 401, which IS mapped, and there would be no
# second fault to answer. The third is `(image megapage + 2) & 511` of the same
# table, the fourth `+ 3` and the fifth `+ 4`, by the same reasoning and the
# same mask.
#
# ## The control, which is what makes words 6, 7 and 8 mean something
#
# A package that matches can match because it crossed or because P is reading
# the frame the kernel wrote and F would read the same one. So F's inbox is
# filled, before paging is on, with a DECOY — its own frame address in all
# three slots — and the package goes only into P's. If the two window tables
# were accidentally one (the confusion `C-002l` exists to refuse) P would read
# the decoy, and words 6, 7 and 8 would come back as three copies of one
# address. This check names that case rather than reporting a mismatch.
#
# ## The layout the words are derived from — thirteen frames, one aligned base
#
#    0 F's root table            1 P's root table
#    2 F's megapage table        3 P's megapage table
#    4 the image megapage's page table (shared)
#    5 F's window table          6 P's window table
#    7 the user page (shared, U and X, no W)
#    8 F's inbox (U, no X)       9 P's inbox (U, no X)
#   10 F's page table for the EMPTY megapage 400 — a valid non-leaf whose 512
#      leaves are all zero, which is what makes the fault a fault
#   11 the page the pager hands back FIRST, seeded with its own address before
#      paging is on
#   12 the page it hands back SECOND, seeded the same way. Two distinct seeds
#      are what make words 15 and 25 able to disagree
#
# There is no frame 13. That is not an omission: the third fault is REFUSED,
# and a layout that kept a spare frame ready for it would make the refusal a
# choice about policy rather than the only honest answer left. It is also the
# limit in words 43 and 44 — the first frame P does not own is the first frame
# there ISN'T, and the two facts are the same fact. The fourth fault needs no
# new frame either: the one P names on it is frame 0, which already exists and
# already belongs to somebody. Nor does the fifth, for a third reason — nobody
# names one at all.
#
# `satp` holds F's root frame number, so base = (word 1 & PPN) << 12 and every
# other frame is base + n×4096. F faults at megapage 400 of the image gigapage,
# at the leaf whose index is the image's OWN megapage index — an address that is
# EMPTY IN F'S MAP AND MAPPED IN NOBODY'S, so the fault cannot be coming from
# anywhere else. Nothing here is a constant a passing program could have been
# written around: change where the image links and every one of those addresses
# changes with it.
#
# ## The leaf index, and why it is not 7 — found by `C-002h2b`
#
# This row wrote `leaf 7`, and a fixed leaf makes the faulting address a
# function of the GIGAPAGE alone. Both link addresses below are inside gigapage
# 2, so word 4 came back identical at both — and `MOVES` demands it differ. The
# contract contradicted itself: no program could satisfy both halves, and the
# synthetic run it was validated against never noticed because it was built from
# two bases in two different gigapages.
#
# The repair keeps the demand and fixes the derivation. The leaf is `(base >> 21)
# & 511`, the image's own megapage index, which DOES follow the link address —
# 0x80200000 and 0x80400000 are one megapage apart by construction. Weakening
# `MOVES` to call words 4 and 7 fixed was the other way out and is worse: word 7
# moving is this check's whole guard against a package P was born holding.
#
# ## Two link addresses
#
# MOVES  1, 2, 4, 5, 7, 8, 10, 11, 13, 14, 15, 17, 18, 20, 21, 23, 24, 25, 27,
#        28, 33, 34, 36, 37, 39, 40, 42, 43, 44, 47, 48, 50, 51, 53, 54, 57,
#        58. All derived from where the image was loaded. For 7, 8, 17, 18, 27,
#        28, 39 and 40 that is the whole guard: a package that is identical at
#        both link addresses is a package P could have been born holding. Words
#        34, 48 and 58 are the same guard for the refusal, the distrust and the
#        silence: an address F wrote down that is the same at both link
#        addresses is one F did not learn. Word 53 moves because it is a satp,
#        and word 54 because P's code moves with the image. Words 43 and 44 move
#        because the kernel's record of what P owns is a pair of addresses in
#        this image's own frames — a record that did not move is a constant
#        compiled in rather than a record the kernel keeps.
# FIXED  3, 6, 9, 12, 16, 19, 22, 26, 29, 30, 31, 32, 35, 38, 41, 45, 46, 49,
#        52, 55, 56, 59, 60, 61, 62. A cause number is architectural —
#        including word 52, where the architecture names the timer — a count is
#        a count, and the refusal and the three leaves read back after a fault
#        that was not resolved are absences: the words here that are fixed
#        because there is nothing there, rather than because they are constants.
#
# ## What a green here does NOT prove, and which row owes it
#
#   - That the BAD PAGER IS DEALT WITH. Its answer is refused and F is let go;
#     P is not. A pager caught naming a frame it does not own keeps running,
#     keeps its inbox, and is asked again on the very next fault — and the one
#     that stalls is not descheduled either. Nothing here revokes a pager,
#     retries a fault with another one, or charges the deadline to the thread
#     that burned it, because there is no scheduler to hand the processor to
#     anyone else and no second pager to hand it to. `C-002i2`, and a row after
#     it.
#   - That the deadline is the RIGHT length. It is long enough that four
#     answers arrive inside it and short enough that one run ends before the
#     harness's own kill, and that is all this proves about it. Choosing it
#     against a measured pager is `W-012b`'s host and `C-002`'s microsecond.
#   - That the OWNERSHIP RECORD is out of P's reach. Words 43 and 44 are what
#     the kernel used, not where it got it from, and a kernel that read P's own
#     fifth inbox slot would print the same two numbers — that slot holds frame
#     13 by `C-002h2c2b`'s construction. The demand is in the prose above and
#     the check cannot see it; catching it would need a run where P's slot and
#     the kernel's record DISAGREE, which is a pager that lies about its own
#     bound, and that is a row after this one.
#   - That ownership is more than a RANGE. One pager, one interval. Two pagers
#     owning disjoint sets, a frame changing hands, or a frame owned by nobody
#     are all outside what a first-and-limit can say. `C-002i2`, and an owner
#     map after it.
#   - Cost. No microsecond is produced under QEMU, by `C-002`'s decision.
#
# ## Absence is loud
#
# `W-087`: 27 check scripts exited 0 when their tooling was missing, so a bare
# machine reported a green tree. This exits **77** when it cannot run and **1**
# when it runs and fails, and never 0 without having watched a machine.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
# NOT in $root: the gate fingerprints modified and untracked files, so a scratch
# file dropped in the repo is a file that exists while the gate is looking.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "CANNOT RUN: qemu-system-riscv64 is not installed, so nothing was measured"
  exit 77; }
command -v riscv64-elf-readelf >/dev/null 2>&1 || {
  echo "CANNOT RUN: riscv64-elf-readelf is not installed, so the .bss the thirteen"
  echo "            frames have to fall inside was never read"
  exit 77; }

prog=$root/spec/pager-handoff.sas
[ -f "$prog" ] || {
  echo "RED, as written: spec/pager-handoff.sas does not exist."
  echo
  echo "  C-002h2a puts this contract on the record before the program that has"
  echo "  to satisfy it, the way C-002j did for the milestone. The sixty-two"
  echo "  words it demands are in the header above; C-002h2b is the program that"
  echo "  produces the first ten, C-002h2c1 the second fault answered with a"
  echo "  different frame, C-002h2c2b the third fault the pager refuses,"
  echo "  C-002h2c3b the fourth where the KERNEL refuses, and C-002h2c2c2 the"
  echo "  fifth the pager never answers."
  echo
  echo "  Nothing is wrong with the tree. This is the red a contract is supposed"
  echo "  to be while the row that satisfies it is still open."
  exit 1; }

# ---- the word contract, in POSIX sh ----------------------------------------
# sh arithmetic is signed 64-bit; printf %x and %u print the unsigned value and
# ph_ult compares unsigned. Hex words are parsed in 32-bit halves.
ph_names() { cat <<'EOF'
satp for F
satp for P
scause at F's fault
stval at F's fault
sepc at F's fault
the cause P read
the stval P read
the sepc P read
scause on P's ecall
the frame P named
the leaf the kernel installed
scause at F's second fault
stval at the second fault
sepc at the second fault
the word F read on the first resume
the cause P read the second time
the stval P read the second time
the sepc P read the second time
scause on P's second ecall
the frame P named the second time
the second leaf the kernel installed
scause at F's third fault
stval at the third fault
sepc at the third fault
the word F read on the second resume
the cause P read the third time
the stval P read the third time
the sepc P read the third time
scause on P's third ecall
the frame P named the third time
the refused leaf read back
scause when F next trapped
sepc when F next trapped
what F wrote down about the refusal
scause at F's fourth fault
stval at the fourth fault
sepc at the fourth fault
the cause P read the fourth time
the stval P read the fourth time
the sepc P read the fourth time
scause on P's fourth ecall
the frame P named the fourth time
the first frame the kernel says P owns
the first frame the kernel says P does not own
the distrusted leaf read back
scause when F trapped after that
sepc when F trapped after that
what F wrote down about the distrust
scause at F's fifth fault
stval at the fifth fault
sepc at the fifth fault
scause when the deadline expired
satp when the deadline expired
sepc when the deadline expired
the unanswered leaf read back
scause when F trapped after that
sepc when F trapped after that
what F wrote down about the silence
faults delivered
deadlines that expired
answers the pager gave
answers the kernel refused
EOF
}
ph_nm() { ph_names | awk -v n="$1" 'NR==n { printf "%s", $0 }'; }
ph_h() { printf '%016x' "$1"; }
ph_x() { if [ "$1" -eq 0 ]; then printf 0x0; else printf '%#x' "$1"; fi; }
ph_cause() {
    case $1 in
        12) printf 'instruction page fault (12)';;
        13) printf 'load page fault (13)';;
        15) printf 'store/AMO page fault (15)';;
        8) printf 'environment call from U-mode (8)';;
        9) printf 'environment call from S-mode (9)';;
        *) printf '%u' "$1";;
    esac
}
# the frame a leaf names: (leaf >> 10) << 12, the shift made logical
ph_fr() { echo $(( (($1 >> 10) & 18014398509481983) << 12 )); }
ph_fh() { ph_t=$(( ($1 >> 62) & 3 )); if [ "$ph_t" -ne 0 ]; then printf "%x%016x" "$ph_t" "$(ph_fr "$1")"; else printf "%016x" "$(ph_fr "$1")"; fi; }
ph_fe() { [ $(( ($1 >> 62) & 3 )) -eq 0 ] && [ "$(ph_fr "$1")" -eq "$2" ]; }
ph_ugt() { [ $(( $1 ^ (-9223372036854775807 - 1) )) -gt $(( $2 ^ (-9223372036854775807 - 1) )) ]; }
ph_bad() { ph_BAD="$ph_BAD$1
"; }
ph_ult() { [ $(( ($1 ^ ph_MIN) < ($2 ^ ph_MIN) )) -eq 1 ]; }
# lo <= x < lo + 4096, unsigned
ph_inpage() { if ph_ult "$1" "$2"; then return 1; fi; ph_ult "$1" $(( $2 + 4096 )); }
# one hex word (up to 16 digits) into ph_PV
ph_parse() {
    ph_s=0000000000000000${1#0x}
    ph_a=${ph_s%????????}
    ph_lo=${ph_s#"$ph_a"}
    ph_hi=${ph_a#"${ph_a%????????}"}
    ph_PV=$(( (0x$ph_hi << 32) | 0x$ph_lo ))
}

ph_took() {  # w cause stval sepc want_va want_leaf
    w=$1; c=$2; sv=$3; sp=$4; wv=$5; wl=$6
    if [ "$c" -ne 13 ]; then
        ph_bad "word $w, $(ph_nm "$w"): $(ph_cause "$c"), expected $(ph_cause 13). F's touch is a load"
    fi
    if [ "$sv" -ne "$wv" ]; then
        extra=""
        if [ "$sv" -eq $(( wv & ~4095 )) ]; then
            extra=" — that is the base of the page, not the address, and a pager cannot place a byte it was never told about"
        elif [ "$sv" -eq 0 ]; then
            extra=" — zero is what stval reads when nothing wrote it, so nothing distinguishes this from a package assembled without looking"
        fi
        ph_bad "word $((w + 1)), $(ph_nm $((w + 1))): $(ph_h "$sv"), and the address F touches is $(ph_h "$wv") by the declared layout — megapage $ph_HOLE_MEGA of the image gigapage $(ph_h "$giga"), leaf $wl$extra"
    fi
    if ! ph_inpage "$sp" $(( base + ph_USER )); then
        extra=""
        if [ "$sp" -eq "$sv" ]; then extra=" — it is the faulting ADDRESS"; fi
        ph_bad "word $((w + 2)), $(ph_nm $((w + 2))): $(ph_h "$sp"), which is outside the shared user page $(ph_h $((base + ph_USER)))..$(ph_h $((base + ph_USER + 4096))). The faulting load is one of the instructions on that page, and an sepc pointing anywhere else is not the instruction that faulted$extra"
    fi
    if [ "$sv" -eq "$sp" ]; then
        ph_bad "words $((w + 1)) and $((w + 2)) are the same value; a load fault has an address and an instruction and they are two different things"
    fi
}

ph_crossed() {  # w p_cause p_stval p_sepc ecall cause stval sepc k
    w=$1; pc=$2; ps=$3; pp=$4; ec=$5; kc=$6; ks=$7; kp=$8; k=$9
    decoy=$(( base + ph_INBOX_F ))
    if [ "$pc" -eq "$decoy" ] && [ "$ps" -eq "$decoy" ] && [ "$pp" -eq "$decoy" ]; then
        ph_bad "words $w, $((w + 1)) and $((w + 2)) are three copies of $(ph_h "$decoy"), which is F's own inbox frame and the decoy planted in it before paging was on. P read F's inbox, not its own — the two window tables are reaching one frame and the package never crossed"
    else
        i=0
        for pair in "$pc $kc" "$ps $ks" "$pp $kp"; do
            set -- $pair; got=$1; want=$2
            if [ "$got" -ne "$want" ]; then
                if [ "$got" -eq 0 ]; then
                    ph_bad "word $((w + i)), $(ph_nm $((w + i))): zero, which is what an unwritten inbox reads. Nothing distinguishes 'the kernel wrote a zero' from 'the package never arrived'"
                else
                    ph_bad "word $((w + i)), $(ph_nm $((w + i))): $(ph_h "$got"), and the kernel took $(ph_h "$want") (word $((k + i))). P is reading its inbox under its own satp, so a different value means what the kernel wrote is not what P found there — the package did not survive the crossing intact, which is the one thing this row is about"
                fi
            fi
            i=$((i + 1))
        done
    fi
    if [ "$ec" -ne 8 ]; then
        ph_bad "word $((w + 3)), $(ph_nm $((w + 3))): $(ph_cause "$ec"), expected $(ph_cause 8). P answered from supervisor mode, so it is not a userspace pager — doc 11 §3.3 puts the policy OUTSIDE the kernel and a pager inside it is the arrangement this row exists to replace"
    fi
}

ph_answered() {  # w named want_frame
    w=$1; nd=$2; wf=$3
    if [ "$nd" -eq 0 ]; then
        ph_bad "word $w, $(ph_nm "$w"): zero, which this contract spells REFUSAL. The pager still owns a frame at this point and had no ground to refuse — a refusal here is not a policy, it is a pager that lost track of what it owns"
        return 0
    fi
    if [ $(( nd % 4096 )) -ne 0 ]; then
        ph_bad "word $w, $(ph_nm "$w"): $(ph_h "$nd") is not page aligned, so it is not a frame; whatever the kernel installs from it will not be the page the pager meant"
    fi
    if [ "$nd" -ne "$wf" ]; then
        ph_bad "word $w, $(ph_nm "$w"): $(ph_h "$nd"), and the page the pager hands back at this point is $(ph_h "$wf") by the declared layout. The kernel received an answer, but not the one the pager was supposed to give — look at the pager's own slot, not at the kernel's install path"
    fi
}

ph_installed() {  # w leaf named want_frame
    w=$1; lf=$2; nd=$3; wf=$4
    if [ $(( lf & 1 )) -eq 0 ]; then
        ph_bad "word $w, $(ph_nm "$w"): $(ph_x "$lf"), which has V clear — the fault was answered with a mapping that does not map, so the load is about to fault again"
        return 0
    fi
    fr=$(ph_fr "$lf")
    if ! ph_fe "$lf" "$nd"; then
        ph_bad "word $w, $(ph_nm "$w"): frame $(ph_fh "$lf"), and the pager NAMED $(ph_h "$nd") (word $((w - 1))). The kernel installed something other than the answer it was given — the answer travels in a register between those two words and this is where it was dropped. This comparison is why word 10 exists: without it a kernel that ignores the answer and a pager that repeats itself print the same picture"
    elif [ "$fr" -ne "$wf" ]; then
        ph_bad "word $w, $(ph_nm "$w"): frame $(ph_fh "$lf"), and the page the pager hands back is $(ph_h "$wf") by the declared layout. The kernel installed exactly what it was told, so the kernel is not the fault here — the pager named the wrong frame"
    fi
    if [ $(( lf & 2 )) -eq 0 ]; then
        ph_bad "word $w, $(ph_nm "$w"): $(ph_x "$lf") does not grant R, and the fault was a LOAD. The cause said what was needed and the answer did not act on it"
    fi
    if [ $(( lf & 16 )) -eq 0 ]; then
        ph_bad "word $w, $(ph_nm "$w"): $(ph_x "$lf") does not carry U. F is in U-mode, so resuming it onto a page without U hands it the same fault again under a different name"
    fi
    if [ $(( lf & 4 )) -ne 0 ]; then
        ph_bad "word $w, $(ph_nm "$w"): $(ph_x "$lf") grants W for a load fault — more than the cause asked for, and copy-on-write is exactly the policy that breaks when a read fault is answered with a writable page"
    fi
    if [ $(( lf & 8 )) -ne 0 ]; then
        ph_bad "word $w, $(ph_nm "$w"): $(ph_x "$lf") grants X for a load fault"
    fi
    if [ $(( lf & 4 )) -ne 0 ] && [ $(( lf & 8 )) -ne 0 ]; then
        ph_bad "word $w, $(ph_nm "$w"): $(ph_x "$lf") is writable AND executable. W^X is the rule C-002c fixed globally, and a pager that breaks it while resolving breaks it everywhere"
    fi
}

ph_resumed() {  # w read_back want_frame
    w=$1; rb=$2; wf=$3
    if [ "$rb" -ne "$wf" ]; then
        extra=""
        if [ "$rb" -eq 0 ]; then extra=" — zero is an untouched frame"; fi
        ph_bad "word $w, $(ph_nm "$w"): $(ph_h "$rb"), and the page the pager handed back holds its own address $(ph_h "$wf"), written into it before paging was on. The fault was made to go away without the mapping reaching the memory the pager named$extra"
    fi
}

ph_dups() {  # kind word:value ...
    kind=$1; shift
    while [ $# -gt 1 ]; do
        a=$1; shift
        for b in "$@"; do
            if [ "${a#*:}" -eq "${b#*:}" ]; then
                if [ "$kind" = addr ]; then
                    ph_bad "words ${a%%:*} and ${b%%:*} are the same address $(ph_h "${a#*:}"). Each fault is supposed to be at the next leaf of the same empty megapage; two faults at one address are one address faulting twice, which means the answer before it never took"
                else
                    ph_bad "words ${a%%:*} and ${b%%:*} are the same instruction $(ph_h "${a#*:}"). F is supposed to reach a further load each time; the same instruction faulting twice is the previous one re-running, and re-running is what happens when nothing was installed"
                fi
            fi
        done
    done
}

# ph_verify <hex-lines-file> <bss-file>: one run's sixty-two words
ph_verify() (
    ph_F1=$1; ph_F2=$2
    ph_BAD=""
    ph_MIN=$(( 1 << 63 ))
    ph_PPN=$(( (1 << 44) - 1 ))
    ph_S_TIMER=$(( (1 << 63) | 5 ))
    ph_FRAMES=13
    ph_USER=$(( 7 * 4096 )); ph_INBOX_F=$(( 8 * 4096 ))
    ph_GIVEN=$(( 11 * 4096 )); ph_GIVEN2=$(( 12 * 4096 ))
    ph_OWN_FIRST=$(( 11 * 4096 )); ph_OWN_LIMIT=$(( 13 * 4096 ))
    ph_NOT_OWNED=0
    ph_HOLE_MEGA=400
    ph_MEGA=$(( 1 << 21 ))

    set --
    while read -r ph_line || [ -n "$ph_line" ]; do
        ph_line=$(printf '%s' "$ph_line" | tr -d ' \t\r')
        [ -n "$ph_line" ] || continue
        ph_parse "$ph_line"
        set -- "$@" "$ph_PV"
    done < "$ph_F1"
    n=$#
    if [ "$n" -ne 62 ]; then
        why="Stopping short means the machine stopped where the line stops — an unresolved fault re-runs its instruction forever and prints nothing more, and a package that never reached P leaves P blocked on an inbox that stays empty"
        if [ "$n" -eq 21 ]; then
            why="Twenty-one is exactly what C-002h2c1's program prints, so nothing is wrong with the machine: this is the red a contract is supposed to be while the row that satisfies it is open. C-002h2c2a added words 10, 20 and 30 for what the pager SAID and words 22-34 for a third fault it REFUSES; C-002h2c2b is the program that owes them"
        fi
        if [ "$n" -eq 35 ]; then
            why="Thirty-five is exactly what C-002h2c2b's program prints, and it ran to its own shutdown, so nothing is wrong with the machine: this is the red a contract is supposed to be while the row that satisfies it is open. C-002h2c2c1 added words for a fault the pager never answers, bounded by a deadline the kernel armed rather than by the kill in this script, and C-002h2c3a added words 35-48 for one where the pager names a frame it does not own and the KERNEL refuses; C-002h2c2c2 and C-002h2c3b are the programs that owe them"
        fi
        if [ "$n" -eq 47 ]; then
            why="Forty-seven is exactly what C-002h2c2c2's program prints, and it ran to its own shutdown, so nothing is wrong with the machine: this is the red a contract is supposed to be while the row that satisfies it is open. C-002h2c3a added words 35-48 for a FOURTH fault where the pager names a frame that is not its — frame 0, F's own root table — and the KERNEL refuses it against a record of what P owns that P cannot reach, plus word 62 counting that refusal; the silence moved to words 49-58 because a fault the pager must ANSWER cannot come after one it never answers. C-002h2c3b is the program that owes them"
        fi
        printf '%s\n' "printed $n hex lines, expected 62: $(ph_names | awk '{ printf "%s%d %s", (NR > 1 ? "; " : ""), NR, $0 }'). $why"
        exit 1
    fi

    satp_f=$1 satp_p=$2 cause=$3 stval=$4 sepc=$5
    p_cause=$6 p_stval=$7 p_sepc=$8 ecall=$9 named=${10} leaf=${11}
    cause2=${12} stval2=${13} sepc2=${14} read_back=${15}
    p_cause2=${16} p_stval2=${17} p_sepc2=${18} ecall2=${19} named2=${20} leaf2=${21}
    cause3=${22} stval3=${23} sepc3=${24} read_back2=${25}
    p_cause3=${26} p_stval3=${27} p_sepc3=${28} ecall3=${29} named3=${30} leaf3=${31}
    f_cause=${32} f_sepc=${33} f_wrote=${34}
    cause4=${35} stval4=${36} sepc4=${37}
    p_cause4=${38} p_stval4=${39} p_sepc4=${40} ecall4=${41} named4=${42}
    own_first=${43} own_limit=${44} leaf4=${45}
    f_cause2=${46} f_sepc2=${47} f_wrote2=${48}
    cause5=${49} stval5=${50} sepc5=${51}
    late_cause=${52} late_satp=${53} late_sepc=${54} leaf5=${55}
    f_cause3=${56} f_sepc3=${57} f_wrote3=${58}
    count=${59} deadlines=${60} answers=${61} refused_by_kernel=${62}

    set -- $(tr -s ' \t\r\n' ' ' < "$ph_F2")
    ph_parse "$1"; bss_addr=$ph_PV
    ph_parse "$2"; bss_size=$ph_PV
    base=$(( (satp_f & ph_PPN) << 12 ))
    giga=$(( (base >> 30) << 30 ))
    hole_leaf=$(( (base >> 21) & 511 ))
    fault_va=$(( giga + ph_HOLE_MEGA * ph_MEGA + hole_leaf * 4096 ))
    hole_leaf2=$(( (hole_leaf + 1) & 511 ))
    fault_va2=$(( giga + ph_HOLE_MEGA * ph_MEGA + hole_leaf2 * 4096 ))
    hole_leaf3=$(( (hole_leaf + 2) & 511 ))
    fault_va3=$(( giga + ph_HOLE_MEGA * ph_MEGA + hole_leaf3 * 4096 ))
    hole_leaf4=$(( (hole_leaf + 3) & 511 ))
    fault_va4=$(( giga + ph_HOLE_MEGA * ph_MEGA + hole_leaf4 * 4096 ))
    hole_leaf5=$(( (hole_leaf + 4) & 511 ))
    fault_va5=$(( giga + ph_HOLE_MEGA * ph_MEGA + hole_leaf5 * 4096 ))

    for pair in "1 $satp_f" "2 $satp_p"; do
        set -- $pair
        if [ $(( ($2 >> 60) & 15 )) -ne 8 ]; then
            ph_bad "word $1, $(ph_nm "$1"): mode nibble $(printf '%x' $(( ($2 >> 60) & 15 ))), not 8. Sv39 is the only paging mode this tree installs, and a satp that is not in it was never installed"
        fi
    done
    if [ "$satp_f" -eq "$satp_p" ]; then
        ph_bad "words 1 and 2 are the same satp ($(ph_h "$satp_f")), so the pager ran in the faulter's own address space. There is no handoff without a boundary to hand across, and C-002h already proved the package inside one address space"
    elif [ $(( satp_p & ph_PPN )) -ne $(( (satp_f & ph_PPN) + 1 )) ]; then
        ph_bad "word 2, $(ph_nm 2): page number $(printf '%x' $(( satp_p & ph_PPN ))), and the declared layout puts P's root in the frame after F's, $(printf '%x' $(( (satp_f & ph_PPN) + 1 ))). The satp that was installed is not the table the source says was built"
    fi
    if [ $(( base % 4096 )) -ne 0 ]; then
        ph_bad "word 1: F's root table is at $(ph_h "$base"), which is not page aligned — satp keeps a page NUMBER and the low bits are lost"
    fi
    if ph_ult "$base" "$bss_addr" || ph_ult $(( bss_addr + bss_size )) $(( base + ph_FRAMES * 4096 )); then
        ph_bad "word 1: the frames start at $(ph_h "$base"), and the $ph_FRAMES of them do not fit inside the .bss the ELF reserves ($(ph_h "$bss_addr") + $(printf '%x' "$bss_size"))"
    fi
    if [ $(( (base >> 21) & 2047 )) -ne $(( ((base + ph_FRAMES * 4096 - 1) >> 21) & 2047 )) ]; then
        ph_bad "the $ph_FRAMES frames from $(ph_h "$base") straddle a megapage boundary, so they are not all mapped by the one shared page table the layout says maps them"
    fi

    ph_took 3 "$cause" "$stval" "$sepc" "$fault_va" "$hole_leaf"
    ph_crossed 6 "$p_cause" "$p_stval" "$p_sepc" "$ecall" "$cause" "$stval" "$sepc" 3
    ph_answered 10 "$named" $((base + ph_GIVEN))
    ph_installed 11 "$leaf" "$named" $((base + ph_GIVEN))
    ph_took 12 "$cause2" "$stval2" "$sepc2" "$fault_va2" "$hole_leaf2"
    ph_resumed 15 "$read_back" $((base + ph_GIVEN))
    ph_crossed 16 "$p_cause2" "$p_stval2" "$p_sepc2" "$ecall2" "$cause2" "$stval2" "$sepc2" 12
    ph_answered 20 "$named2" $((base + ph_GIVEN2))
    ph_installed 21 "$leaf2" "$named2" $((base + ph_GIVEN2))
    ph_took 22 "$cause3" "$stval3" "$sepc3" "$fault_va3" "$hole_leaf3"
    ph_resumed 25 "$read_back2" $((base + ph_GIVEN2))
    ph_crossed 26 "$p_cause3" "$p_stval3" "$p_sepc3" "$ecall3" "$cause3" "$stval3" "$sepc3" 22
    ph_took 35 "$cause4" "$stval4" "$sepc4" "$fault_va4" "$hole_leaf4"
    ph_crossed 38 "$p_cause4" "$p_stval4" "$p_sepc4" "$ecall4" "$cause4" "$stval4" "$sepc4" 35
    ph_took 49 "$cause5" "$stval5" "$sepc5" "$fault_va5" "$hole_leaf5"

    ph_dups addr "4:$stval" "13:$stval2" "23:$stval3" "36:$stval4" "50:$stval5"
    ph_dups pc "5:$sepc" "14:$sepc2" "24:$sepc3" "37:$sepc4" "51:$sepc5"

    if [ "$named" -eq "$named2" ]; then
        ph_bad "words 10 and 20: the pager NAMED the same frame $(ph_h "$named") twice. Its fourth inbox slot is the frame it hands out next and it is supposed to add 4096 to it and write it back, in U-mode through its own window, BEFORE it answers. Look at the pager's store to slot 4. The kernel is not implicated: these two words are what the kernel received, printed before it acted on either"
    fi
    if [ $(( leaf & 1 )) -ne 0 ] && [ $(( leaf2 & 1 )) -ne 0 ] && [ "$(( (leaf >> 10) & 18014398509481983 ))" -eq "$(( (leaf2 >> 10) & 18014398509481983 ))" ] && [ "$named" -ne "$named2" ]; then
        ph_bad "words 11 and 21 install the SAME frame $(ph_fh "$leaf") for two different faults, and the pager named two DIFFERENT frames (words 10 and 20). So this is the kernel: it threw the answer away and mapped the frame it would have chosen anyway. Look at the kernel's leaf-installing path. C-002h2b's green could not tell a leaf the pager named from one the kernel chose, and until C-002h2c2a added word 10 this message had to name two causes at once"
    fi
    if [ "$read_back" -eq "$read_back2" ]; then
        ph_bad "words 15 and 25 read the same seed $(ph_h "$read_back") after two faults answered with two different frames. Each handed-back page holds its own address and nothing else does, so one seed read twice means the second mapping reached the first frame"
    fi

    if [ "$named3" -ne 0 ]; then
        ph_bad "word 30, $(ph_nm 30): $(ph_h "$named3"), and the pager owns two frames and has handed out both. A third frame from a pager that owns two is a pager handing out memory that is not its to give, which is the failure C-002h2c3 is about; here it means the refusal never happened and every word after this one is about something else"
    fi
    if [ "$leaf3" -ne 0 ]; then
        ph_bad "word 31, $(ph_nm 31): $(ph_x "$leaf3"), read back at the refused address after the refusal, and it should be zero. Something was installed for a fault that was REFUSED — a refusal that quietly maps a page is a resolution wearing the word 'no', and F would be resumed onto memory nobody agreed to give it"
    fi
    if [ "$f_cause" -ne 8 ]; then
        ph_bad "word 32, $(ph_nm 32): $(ph_cause "$f_cause"), expected $(ph_cause 8). After a refusal F is supposed to RUN — at an address it chose in advance — and call. Another page fault here means F was resumed onto the load that was refused and faulted on it again, which is the infinite loop this row exists to replace"
    fi
    if ! ph_inpage "$f_sepc" $((base + ph_USER)); then
        ph_bad "word 33, $(ph_nm 33): $(ph_h "$f_sepc"), which is outside the shared user page $(ph_h $((base + ph_USER)))..$(ph_h $((base + ph_USER + 4096))). F's refusal entry is one of the instructions on that page"
    elif [ "$f_sepc" -eq "$sepc3" ]; then
        ph_bad "word 33, $(ph_nm 33): $(ph_h "$f_sepc"), which is word 24 — the load that was refused. F was resumed at the faulting instruction after being told there would be no page for it, so it can only fault again. A refusal has to send F somewhere ELSE"
    fi
    if [ "$f_wrote" -ne "$stval3" ]; then
        extra=""
        if [ "$f_wrote" -eq 0 ]; then extra=" — zero is the slot untouched, so F never reached the store at all"; fi
        ph_bad "word 34, $(ph_nm 34): $(ph_h "$f_wrote"), and the address that was refused is $(ph_h "$stval3") (word 23). F writes this into its own inbox with its own instructions through its own map, so what is here is what F learned — and a refusal F cannot name the address of is one it cannot act on$extra"
    fi
    if [ "$named4" -eq 0 ]; then
        ph_bad "word 42, $(ph_nm 42): zero, which this contract spells REFUSAL, and the pager is supposed to ANSWER this fault — with frame 0, which is not its to give. A refusal here is the pager declining to do the one thing this round exists to catch the kernel at, and words 43 to 45 would then be about nothing"
    elif [ $(( named4 % 4096 )) -ne 0 ]; then
        ph_bad "word 42, $(ph_nm 42): $(ph_h "$named4") is not page aligned, so it is not a frame at all. The answer this round is about is a frame that is REAL and belongs to somebody else — a misaligned word would be refused by arithmetic and would prove nothing about ownership"
    elif [ "$named4" -ne $((base + ph_NOT_OWNED)) ]; then
        extra=""
        if ! ph_ult "$named4" $((base + ph_OWN_FIRST)) && ph_ult "$named4" $((base + ph_OWN_LIMIT)); then
            extra=" — and that IS a frame P owns, so a kernel that installed it would be right to; there is nothing here for the record to refuse"
        fi
        ph_bad "word 42, $(ph_nm 42): $(ph_h "$named4"), and the frame the pager names on this fault is $(ph_h $((base + ph_NOT_OWNED))) by the declared layout — frame 0, F's own root page table$extra. It is chosen because it is real memory in use, belongs to F and the kernel both, and a kernel that installed it would hand F a readable mapping of its own page tables"
    fi
    if [ "$own_first" -ne $((base + ph_OWN_FIRST)) ]; then
        extra=""
        if [ "$own_first" -eq 0 ]; then extra=" — zero is an unwritten record, and a record nobody wrote refuses everything or nothing"; fi
        ph_bad "word 43, $(ph_nm 43): $(ph_h "$own_first"), and the first frame P owns is $(ph_h $((base + ph_OWN_FIRST))) by the declared layout — frame 11, the one it handed out on the first fault. The kernel is enforcing a record that does not describe this pager$extra"
    fi
    if [ "$own_limit" -ne $((base + ph_OWN_LIMIT)) ]; then
        extra=""
        if [ "$own_limit" -eq 0 ]; then extra=" — zero is an unwritten record"; fi
        ph_bad "word 44, $(ph_nm 44): $(ph_h "$own_limit"), and the first frame P does NOT own is $(ph_h $((base + ph_OWN_LIMIT))) — frame 13, which this layout does not have. P owns frames 11 and 12 and nothing else, so the limit is the frame after the last one it handed out$extra"
    fi
    if ! ph_ult "$own_first" "$own_limit"; then
        ph_bad "words 43 and 44: the kernel says P owns [$(ph_h "$own_first"), $(ph_h "$own_limit")), which is empty or inverted. A pager that owns nothing could not have answered the first two faults, and words 11 and 21 say it did"
    elif ! ph_ult "$named4" "$own_first" && ph_ult "$named4" "$own_limit"; then
        ph_bad "word 42, $(ph_nm 42): $(ph_h "$named4") falls INSIDE the range the kernel says P owns, [$(ph_h "$own_first"), $(ph_h "$own_limit")) (words 43 and 44). Then there was nothing to refuse, and word 45 being zero means the kernel dropped an answer it should have installed rather than refused one it should not have"
    fi
    if [ "$leaf4" -ne 0 ]; then
        extra=""
        if [ $(( leaf4 & 1 )) -ne 0 ] && ph_fe "$leaf4" "$named4"; then
            extra=" — and that is exactly the frame P named, so the kernel installed what it was told without consulting the record at all. This is the confused deputy: F now has a readable mapping of its own root page table, put there with full privilege on the word of a thread that had none"
        fi
        ph_bad "word 45, $(ph_nm 45): $(ph_x "$leaf4"), read back at the distrusted address, and it should be zero$extra. A frame the pager had no right to give is not made right by the kernel installing it"
    fi
    if [ "$f_cause2" -ne 8 ]; then
        ph_bad "word 46, $(ph_nm 46): $(ph_cause "$f_cause2"), expected $(ph_cause 8). After the kernel refuses an answer F is supposed to RUN and call, exactly as it does after the pager's own refusal. Another page fault here means F was resumed onto the load, which is the infinite loop word 32 already ruled out for the other refusal"
    fi
    if [ "$f_sepc2" -ne "$f_sepc" ]; then
        extra=""
        if [ "$f_sepc2" -eq "$sepc4" ]; then
            extra=" — that is word 37, the load whose answer was refused, so F is about to take the same fault again"
        fi
        ph_bad "word 47, $(ph_nm 47): $(ph_h "$f_sepc2"), and F's refusal entry is $(ph_h "$f_sepc") (word 33)$extra. F is not supposed to be able to tell the kernel's refusal from the pager's: it asked for a page and did not get one, and which of the two was at fault is between the kernel and P. One ending, not three"
    fi
    if [ "$f_wrote2" -ne "$stval4" ]; then
        extra=""
        if [ "$f_wrote2" -eq "$f_wrote" ]; then
            extra=" — and that is word 34, the address that was REFUSED by the pager. F wrote the same address down twice, so the second report is the first one repeated and F never learned which address was distrusted"
        elif [ "$f_wrote2" -eq 0 ]; then
            extra=" — zero is the slot untouched, so F never reached the store at all"
        fi
        ph_bad "word 48, $(ph_nm 48): $(ph_h "$f_wrote2"), and the address whose answer was distrusted is $(ph_h "$stval4") (word 36)$extra. F writes this into its own inbox with its own instructions through its own map, so what is here is what F learned"
    fi
    if [ "$late_cause" -ne "$ph_S_TIMER" ]; then
        if [ $(( (late_cause >> 63) & 1 )) -eq 0 ]; then
            ph_bad "word 52, $(ph_nm 52): $(ph_cause "$late_cause"), which is an EXCEPTION — bit 63 is clear. Something the pager did trapped; nothing took the processor away from a pager that was still running. A kernel that is not running cannot count, so the only bound on a thread that will not yield is a trap armed before control left: a supervisor timer, scause $(ph_h "$ph_S_TIMER")"
        else
            ph_bad "word 38, $(ph_nm 38): interrupt $(( late_cause & ~(1 << 63) )), expected 5, the supervisor timer ($(ph_h "$ph_S_TIMER")). A software or external interrupt arrived instead; the deadline this row is about is a clock the kernel set with SBI set_timer before it handed the processor to the pager"
        fi
    fi
    if [ "$late_satp" -ne "$satp_p" ]; then
        why="A timer that fires proves only that time passed. What makes it evidence that the PAGER stalled is the address space that was running when it expired"
        if [ "$late_satp" -eq "$satp_f" ]; then
            why="That is word 1, F's own satp: the deadline expired while the FAULTER was running, so it fired at the wrong thread and resolved a fault F was still in the middle of taking"
        fi
        ph_bad "word 53, $(ph_nm 53): $(ph_h "$late_satp"), and P's satp is $(ph_h "$satp_p") (word 2). $why"
    fi
    if ! ph_inpage "$late_sepc" $((base + ph_USER)); then
        ph_bad "word 54, $(ph_nm 54): $(ph_h "$late_sepc"), which is outside the shared user page $(ph_h $((base + ph_USER)))..$(ph_h $((base + ph_USER + 4096))). The pager that failed to answer is supposed to be somewhere in its own code, and a deadline that expired anywhere else is not a stalled pager"
    else
        for ent in "5:$sepc:F's first load" "14:$sepc2:F's second load" "24:$sepc3:the load that was refused" "37:$sepc4:the load whose answer was distrusted" "51:$sepc5:the load nobody answered" "33:$f_sepc:F's refusal entry"; do
            ew=${ent%%:*}; er=${ent#*:}; epc=${er%%:*}; ewho=${er#*:}
            if [ "$late_sepc" -eq "$epc" ]; then
                ph_bad "word 40, $(ph_nm 40): $(ph_h "$late_sepc"), which is word $ew — $ewho. F was running when the deadline expired, so the processor was never handed to the pager at all and there was no silence to bound"
            fi
        done
    fi
    if [ "$leaf5" -ne 0 ]; then
        ph_bad "word 55, $(ph_nm 55): $(ph_x "$leaf5"), read back at the unanswered address after the deadline, and it should be zero. A deadline is not permission to map a frame nobody named: no pager chose that page, so installing one makes the kernel the pager and the stall invisible"
    fi
    if [ "$f_cause3" -ne 8 ]; then
        ph_bad "word 56, $(ph_nm 56): $(ph_cause "$f_cause3"), expected $(ph_cause 8). After the deadline F is supposed to RUN and call, the same way it does after a refusal. Another page fault here means F was resumed onto the load nobody answered, which is the infinite loop the deadline exists to end"
    fi
    if [ "$f_sepc3" -ne "$f_sepc" ]; then
        extra=""
        if [ "$f_sepc3" -eq "$sepc5" ]; then
            extra=" — that is word 51, the load nobody answered, so F is about to take the same fault again"
        fi
        ph_bad "word 57, $(ph_nm 57): $(ph_h "$f_sepc3"), and F's refusal entry is $(ph_h "$f_sepc") (word 33)$extra. F chose ONE address in advance for a fault that will not be resolved, and left it where the kernel could find it. A silence that resumes F somewhere else is one F would need a second handler for, and the kernel would be inventing a destination rather than using the one F gave it"
    fi
    if [ "$f_wrote3" -ne "$stval5" ]; then
        extra=""
        if [ "$f_wrote3" -eq "$f_wrote2" ]; then
            extra=" — and that is word 48, the address whose answer was DISTRUSTED. F wrote the same address down twice, so the third report is the second one repeated and F never learned which address went unanswered"
        elif [ "$f_wrote3" -eq "$f_wrote" ]; then
            extra=" — and that is word 34, the address the PAGER refused, two rounds back. F is reporting an address it learned long before this one"
        elif [ "$f_wrote3" -eq 0 ]; then
            extra=" — zero is the slot untouched, so F never reached the store at all"
        fi
        ph_bad "word 58, $(ph_nm 58): $(ph_h "$f_wrote3"), and the address that went unanswered is $(ph_h "$stval5") (word 50)$extra. F writes this into its own inbox with its own instructions through its own map, so what is here is what F learned"
    fi
    if [ "$count" -ne 5 ]; then
        extra=" — F did not reach all five of its loads, so the fault the missing round is about never happened"
        if ph_ugt "$count" 5; then extra=" — a fault was handed out again, so F was resumed without the leaf being installed and the same load re-ran"; fi
        ph_bad "word 59, $(ph_nm 59): $(printf '%u' "$count") faults delivered, expected 5$extra"
    fi
    if [ "$deadlines" -ne 1 ]; then
        extra=" — the clock kept firing, so it was not disarmed when the answer came, and a deadline that expires under a pager which IS answering will eventually cut one off mid-reply"
        if [ "$deadlines" -eq 0 ]; then extra=" — zero means nothing was armed, or it was armed for a moment that never arrives, and the fifth fault ended some other way"; fi
        ph_bad "word 60, $(ph_nm 60): $(printf '%u' "$deadlines") deadlines expired, expected 1$extra"
    fi
    if [ "$answers" -ne 4 ]; then
        extra=" — fewer than four means an answer that arrived in the first four rounds was lost, so the silence here is not the only one"
        if ph_ugt "$answers" 4; then extra=" — two frames, a refusal and a frame that was not the pager's to give are four answers, and a fifth means the pager DID answer the fault the silence is about: it answered late, which is a slow pager and not a silent one, and the deadline then fired on a fault that was already resolved"; fi
        ph_bad "word 61, $(ph_nm 61): $(printf '%u' "$answers") answers from the pager, expected 4$extra"
    fi
    if [ "$refused_by_kernel" -ne 1 ]; then
        extra=" — more than one means an answer in the first three rounds was refused too, and words 11 and 21 say those two were installed, so the record is refusing frames the pager does own"
        if [ "$refused_by_kernel" -eq 0 ]; then extra=" — zero means the record was never consulted, or consulting it changed nothing: the pager named frame 0 and the kernel took it. Distrust that never refuses anything is the layout again, which is what this row exists to replace"; fi
        ph_bad "word 62, $(ph_nm 62): the kernel refused $(printf '%u' "$refused_by_kernel") answers, expected 1$extra"
    fi

    if [ -n "$ph_BAD" ]; then
        printf '%s' "$ph_BAD"
        exit 1
    fi
    printf '%s\n' "satp $(ph_h "$satp_f")/$(ph_h "$satp_p"), tables at $(ph_h "$base"); F faulted at $(ph_h "$stval") from $(ph_h "$sepc"), P read that package out of its own inbox and answered from U-mode naming $(ph_h "$named"), leaf $(ph_x "$leaf") went in, and F resumed and read $(ph_h "$read_back"); it then faulted at $(ph_h "$stval2"), P named the NEXT frame $(ph_h "$named2"), leaf $(ph_x "$leaf2") went in, and F read $(ph_h "$read_back2"); at $(ph_h "$stval3") P had nothing left and REFUSED, nothing was installed, and F ran on at $(ph_h "$f_sepc") and wrote down $(ph_h "$f_wrote"); at $(ph_h "$stval4") P named $(ph_h "$named4"), which is outside the [$(ph_h "$own_first"), $(ph_h "$own_limit")) the kernel says it owns, and the KERNEL refused it — nothing was installed and F came back to $(ph_h "$f_sepc2") and wrote down $(ph_h "$f_wrote2"); at $(ph_h "$stval5") P said nothing at all and the kernel's own deadline expired on it at $(ph_h "$late_sepc") under P's satp, nothing was installed there either, and F came back to the same entry $(ph_h "$f_sepc3") and wrote down $(ph_h "$f_wrote3")"
    exit 0
)

# ph_moves <low-run-lines> <high-run-lines>: what moved, and what had better not
ph_moves() (
    ph_BAD=""
    ph_load() {  # file prefix
        ph_k=0
        while read -r ph_line || [ -n "$ph_line" ]; do
            ph_line=$(printf '%s' "$ph_line" | tr -d ' \t\r')
            [ -n "$ph_line" ] || continue
            ph_parse "$ph_line"
            eval "$2$ph_k=\$ph_PV"
            ph_k=$((ph_k + 1))
        done < "$1"
    }
    ph_load "$1" lo_
    ph_load "$2" hi_
    for i in 0 1 3 4 6 7 9 10 12 13 14 16 17 19 20 22 23 24 26 27 32 33 35 36 38 39 41 42 43 46 47 49 50 52 53 56 57; do
        eval "a=\$lo_$i b=\$hi_$i"
        if [ "$a" -eq "$b" ]; then
            why="It is derived from where the image was loaded, so a value that did not move is a constant in the source and the derivation it stands for never happened"
            case $i in 6|7|16|17|26|27|38|39) why="$why — and for the package P read, that is the whole guard: a package identical at both link addresses is one P could have been born holding";; esac
            case $i in 33|47|57) why="$why — and for the address F wrote down, that is the whole guard: an address identical at both link addresses is one F did not learn from the refusal, from the distrust or from the silence";; esac
            case $i in 42|43) why="$why — and the kernel's record of what P owns is a pair of addresses in this image's own frames, so one that did not move is a constant compiled in rather than a record the kernel keeps";; esac
            if [ "$i" -eq 52 ]; then why="$why — and a satp that did not move is not the one word 2 printed, so it is not evidence about which address space was running"; fi
            ph_bad "word $((i + 1)), $(ph_nm $((i + 1))): $(ph_h "$a") at both link addresses. $why"
        fi
    done
    for i in 2 5 8 11 15 18 21 25 28 29 30 31 34 37 40 44 45 48 51 54 55 58 59 60 61; do
        eval "a=\$lo_$i b=\$hi_$i"
        if [ "$a" -ne "$b" ]; then
            ph_bad "word $((i + 1)), $(ph_nm $((i + 1))): $(ph_h "$a") then $(ph_h "$b"). A cause number is architectural, a count is a count, and a refusal and the leaf read back after it are absences; one that follows the link address is not being read from the machine at all"
        fi
    done
    if [ -n "$ph_BAD" ]; then
        printf '%s' "$ph_BAD"
        exit 1
    fi
    printf '%s\n' "both satps, all four packages on both sides of the crossing, the frames the pager named, both installed leaves, both words read through them, the kernel's record of what the pager owns, all five faulting addresses, the satp and the instruction the deadline expired on and all three addresses F wrote down all moved between the two link addresses, and the cause numbers, the timer, the refusal, the three leaves read back after a fault that was not resolved and the four counts stayed put"
    exit 0
)

fail=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    "$root/tools/sadhana-native.sh" \
        --स्थान "$addr" "$prog" "$tmp/ph.elf" >/dev/null

    # The section is found by NAME: `kosha` emits only the sections a program
    # uses (`B-071`), so `.bss` is not always the same index.
    riscv64-elf-readelf -S -W "$tmp/ph.elf" \
      | awk '{ for (i = 1; i <= NF; i++) if ($i == ".bss") print $(i+2), $(i+4) }' \
      > "$tmp/bss.$addr"

    # An explicit SIGKILL, not perl's alarm: QEMU handles SIGALRM and outlives
    # it (`W-060`). Since `C-002h2c2c1` this kill is a FAILURE DETECTOR rather
    # than the bound. The fifth fault is delivered to a pager that never
    # answers, and the machine is required to end that itself, at a deadline
    # the kernel armed; if it is still running when the harness comes for it,
    # the thing being tested is exactly what did not happen, and a truncated
    # word count would report it as some other defect. So the watcher leaves a
    # mark when it actually had to kill, and that mark is read first.
    rm -f "$tmp/killed"
    qemu-system-riscv64 -machine virt -smp 1 -m 256M -nographic \
        -bios default -kernel "$tmp/ph.elf" </dev/null > "$tmp/out" 2>&1 &
    qpid=$!
    ( sleep 25; kill -9 "$qpid" 2>/dev/null && : > "$tmp/killed" ) & watcher=$!
    wait "$qpid" 2>/dev/null || true
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    pkill -P "$watcher" 2>/dev/null || true
    kill "$watcher" 2>/dev/null || true
    wait "$watcher" 2>/dev/null || true
    if [ -f "$tmp/killed" ]; then
        echo "  FAIL  $addr: still running after 25 seconds, and the harness killed it."
        echo "        That is the failure this row is about, named as itself: the bound"
        echo "        that ended this run was a sleep in a shell script, not a deadline"
        echo "        the kernel armed before it handed the processor to the pager. On a"
        echo "        device there is no shell script. Look for the timer: 'time' plus a"
        echo "        delta into SBI set_timer, sie.STIE open, and the trap handler"
        echo "        branching on scause bit 63 before it reads the cause. Words 52-62"
        echo "        are what a machine that defends itself prints."
        fail=1; continue
    fi
    # No `tail -12`: it returns min(actual, 12), which makes a count assertion
    # one-sided by construction and blind to a surplus (`W-081`).
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' > "$tmp/l.$addr" || true

    if ! ph_verify "$tmp/l.$addr" "$tmp/bss.$addr" > "$tmp/v" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
done

# ---- what moved, and what had better not ----------------------------------
# Only reached when both runs are internally well formed; otherwise the words
# below are not the words this compares.
if [ "$fail" -eq 0 ]; then
    if ! ph_moves "$tmp/l.०षोड्८०२०००००" "$tmp/l.०षोड्८०४०००००" > "$tmp/m" 2>&1
    then sed 's/^/  FAIL  /' "$tmp/m"; fail=1
    else printf "  both addresses -> %s\n" "$(cat "$tmp/m")"; fi
fi

[ "$fail" -eq 0 ] || {
    echo
    echo "The handoff is not on the record. C-002h proved the package inside one"
    echo "address space; what is owed here is that the package LEAVES it — that a"
    echo "process under a different satp reads stval, scause and sepc out of its"
    echo "own inbox and finds the values the kernel took, that it answers from"
    echo "U-mode, and that the faulting load then runs again and reads the page"
    echo "the answer named — twice, with the pager naming a DIFFERENT frame the"
    echo "second time, which is what tells a kernel that read the answer from one"
    echo "that chose for itself. And a THIRD time, where the pager has nothing"
    echo "left to give and REFUSES: nothing is installed, and F is owed a defined"
    echo "outcome it can tell apart from a resolution rather than a load that"
    echo "faults forever. And a FOURTH time, where the pager DOES answer and"
    echo "names a frame that is not its to give -- F's own root page table --"
    echo "and the kernel refuses it against a record of what that pager owns,"
    echo "held where the pager cannot reach it, rather than installing it and"
    echo "handing F a readable window onto its own page tables. And a FIFTH"
    echo "time, where the pager says nothing at all:"
    echo "there the machine has to end its own wait, at a deadline the kernel"
    echo "armed before it handed over the processor, and F -- which is not the"
    echo "thread that stalled -- has to come back to the same entry it chose for"
    echo "a refusal and be told which address went unanswered. The kill in this"
    echo "script is a failure detector, not the bound."
    exit 1; }

echo
echo "ok  at both link addresses: five load faults in F's address space were"
echo "    packaged and delivered into P's, where a userspace pager read stval,"
echo "    scause and sepc unaltered out of its own inbox and answered by ecall"
echo "    from U-mode. The first two answers NAMED a frame each and the kernel"
echo "    installed exactly what it was named — a different frame the second"
echo "    time, because the pager bumped its own slot before it replied — and F"
echo "    resumed at the instruction that faulted and read each page through."
echo "    The third answer was a REFUSAL: the pager had handed out both frames"
echo "    it owns, named none, and nothing was installed. F did not re-run the"
echo "    load; it ran on at an address it had chosen in advance and wrote down"
echo "    the address it had been refused. On the FOURTH the pager answered and"
echo "    named a frame that was not its to give -- F's own root page table --"
echo "    and the KERNEL refused it: the frame fell outside the record the"
echo "    kernel holds of what that pager owns, nothing was installed, and F"
echo "    came back to the same entry and wrote down that address too. The"
echo "    fifth answer never came at all, and the machine ended that wait"
echo "    ITSELF: a supervisor timer the kernel had armed expired while the"
echo "    PAGER was running, nothing was installed for a page nobody named, and"
echo "    F returned once more to the same entry. One deadline expired across"
echo "    five faults and four answers, one of which the kernel refused, and"
echo "    the harness never had to kill it."
