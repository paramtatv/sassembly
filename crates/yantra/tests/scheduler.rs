//! `C-014a` — a scheduler over `C-014`'s process model, and the half the row is about:
//! **a process that never yields is taken off the hart anyway.**
//!
//! # What `C-014` left, measured rather than taken from the row
//!
//! `crates/yantra/src/process.rs` held two address spaces, saved contexts and a channel,
//! and its own module header said *"It is **not** a scheduler and does not pretend to be
//! one"*. ADR-0021's "What this does NOT settle" said the same in its first clause:
//! *"There is no scheduler, no preemption and no concurrency."* Every test in
//! `tests/process.rs` names the pid it wants — `k.run(&mut m, 1, BUDGET, &mut uart)` — so
//! the switch was the test's decision and not the kernel's.
//!
//! The Sassembly side is the other way round and neither half is the other.
//! `spec/schedule.sas` (`C-002i`, `done`) is a real dispatcher with two policies behind
//! one seam, MLFQ promotion, blocking and scheduler activations — and it says in its own
//! text, at the point where it explains why an upcall lands where it does, that it cannot
//! take the hart back: *"इस नाभिक के पास छीनने का कोई साधन नहीं — न घड़ी, न व्यवधान"*, this
//! nucleus has no means of taking, neither clock nor interrupt. **So preemption existed
//! nowhere in the tree, in either language**, and that is what this file is for. What it
//! does *not* build is `spec/schedule.sas`'s policy work: no priorities, no MLFQ, no
//! swappable policy, no blocking. See ADR-0025.
//!
//! # The negative half, and why a budget is not a quantum
//!
//! The row states the trap it was filed against: *"a cooperative-only scheduler passes
//! every test a preemptive one does until something spins"*. So the fixture spins —
//! [`spinner`] is two instructions, `addi t0, t0, 1` and a jump back to it, with no
//! `ecall` anywhere in the program and no way to reach one.
//!
//! Two things keep the green honest:
//!
//! 1. **`Kernel::schedule` hands each slice the whole remaining budget**, never the
//!    quantum. Nothing but the process itself or the timer can end a slice, so a passing
//!    preemption test cannot be a budget running out wearing a scheduler's name.
//! 2. **The control is an assertion, not a note.**
//!    [`without_a_quantum_the_spinner_keeps_the_hart_and_the_other_process_never_runs`]
//!    runs the same two programs with the same budget under `Kernel::new`, and requires
//!    that the second process retire **zero** instructions — its saved `sepc` still its
//!    entry point. One word of difference between the two kernels; opposite outcomes.
//!
//! And the preemption is measured in the process's own arithmetic rather than in the
//! kernel's opinion of it:
//! [`a_quantum_is_ticks_of_the_clock_and_two_of_them_are_the_kernel_s`] counts how many
//! times the spinner got round its loop, from `t0` in its saved register file.

use sadhana::kosha::write_debuggable_at;
use yantra::process::{Kernel, Slice};
use yantra::supervisor::{Ended, install};
use yantra::{Csrs, Halt, Machine, Privilege};

/// Physical RAM, and the supervisor's own address.
const BASE: u64 = 0x8000_0000;
/// 4 MiB. Three address spaces take about forty frames between them.
const RAM: usize = 1 << 22;
/// Where the loader may start taking frames — the first pool's start.
const FREE: u64 = BASE + 0x1_0000;
/// Where every fixture is linked. Any gigabyte but the supervisor's would do.
const APP: u64 = 0x2000_0000;

/// The slice. Small enough that a whole schedule is short, odd enough that an off-by-one
/// in the arming shows up as a wrong number rather than as a coincidence.
const QUANTUM: u64 = 9;
/// Enough for several slices and then some.
const BUDGET: u64 = 400;
/// What [`exiter`] reports. Not 0, so "it exited" cannot be confused with a zeroed field.
const STATUS: u64 = 37;

// --- encoders, the same dozen `tests/process.rs` uses ----------------------------------

fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    0x13 | rd << 7 | rs1 << 15 | ((imm as u32) & 0xfff) << 20
}
/// `jal rd, imm` — J-type, whose immediate is scattered across four fields. Written out
/// because a wrong one here would look like the scheduler losing a process.
fn jal(rd: u32, imm: i32) -> u32 {
    let i = imm as u32;
    0x6f | rd << 7
        | ((i >> 12) & 0xff) << 12
        | ((i >> 11) & 1) << 20
        | ((i >> 1) & 0x3ff) << 21
        | ((i >> 20) & 1) << 31
}
const ECALL: u32 = 0x0000_0073;

/// An ELF from the project's own writer holding `words`, linked at [`APP`].
fn image(words: &[u32]) -> Vec<u8> {
    let text: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    write_debuggable_at(&text, &[], &[], 0, &[], APP)
}

/// A machine with the supervisor's one `sret` installed at [`BASE`].
fn machine() -> Machine {
    let mut m = Machine {
        // Added with the `patra` file window: a machine that was never asked
        // to serve files must not be able to.
        patra_root: None,
        patra_path: None,
        patra_buffer: None,
        x: [0; 32],
        f: [0; 32],
        fcsr: 0,
        pc: 0,
        base: BASE,
        mem: vec![0; RAM],
        reservation: None,
        csr: Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
    };
    m.csr.sstatus = 1 << 8; // SPP, so a loader that forgets to clear it is caught
    install(&mut m, BASE).expect("the supervisor's word is inside RAM");
    m
}

// --- the fixtures ----------------------------------------------------------------------

/// **The program that never yields.** Two instructions: count, and jump back to counting.
///
/// There is no `ecall` in it, so it cannot make a call; and `ecall`, `sret`, every CSR
/// instruction and `wfi` are illegal in U-mode (`tests/user.rs`), so it could not hand the
/// hart back if it wanted to. It is not a `jal x0, .` either — that self-jump is
/// [`yantra::Halt::SpinForever`] on the bare machine and, since `W-219`, an idle under the
/// kernel ([`parker`]); this one *works* between interruptions, which is what makes it a
/// spinner and not a parked process. `t0` is the odometer: it counts loops, so the
/// register file says how far this process got without the kernel being asked.
fn spinner() -> Vec<u8> {
    image(&[
        addi(5, 5, 1), // addi t0, t0, 1
        jal(0, -4),    // back to it, forever
    ])
}

/// Exits with [`STATUS`] at once — the process that would like a turn.
fn exiter() -> Vec<u8> {
    image(&[
        addi(17, 0, 0),             // a7 = EXIT
        addi(10, 0, STATUS as i32), // a0 = STATUS
        ECALL,
    ])
}

/// A kernel with `programs` spawned in order, pid 0 first.
fn spawned(mut k: Kernel, programs: &[Vec<u8>]) -> (Machine, Kernel) {
    let mut m = machine();
    for (i, p) in programs.iter().enumerate() {
        let pid = k
            .spawn(&mut m, p, Vec::new(), Vec::new())
            .expect("every fixture here is a well-formed ELF the loader accepts");
        assert_eq!(pid, i, "pids are handed out in spawn order");
    }
    (m, k)
}

/// The pid of each slice, in order — the ledger read as a sequence of decisions.
fn order(ledger: &[Slice]) -> Vec<usize> {
    ledger.iter().map(|s| s.pid).collect()
}

// --- the row ---------------------------------------------------------------------------

/// **`C-014a`'s acceptance.** A process with no `ecall` in it loses the hart to a process
/// that has one, and is still alive afterwards.
#[test]
fn a_process_that_never_yields_is_preempted_anyway() {
    let (mut m, mut k) = spawned(Kernel::preemptive(FREE, QUANTUM), &[spinner(), exiter()]);
    let mut uart = Vec::new();
    let ledger = k.schedule(&mut m, BUDGET, &mut uart);

    // The first turn is the spinner's, and it did not end it.
    assert_eq!(
        ledger[0],
        Slice {
            pid: 0,
            steps: QUANTUM,
            ended: Ended::Preempted,
        },
        "the spinner asked for nothing and was taken off the hart after exactly one \
         quantum. `Ended::Preempted` and not `OutOfBudget`: the slice was handed the whole \
         remaining budget of {BUDGET}, so the only thing that could have ended it is the \
         timer"
    );

    // And the second turn went to the process that could not have got one otherwise.
    assert_eq!(
        ledger[1].pid, 1,
        "the hart went to the other process, not back to the one just interrupted"
    );
    assert_eq!(
        ledger[1].ended,
        Ended::Exited { status: STATUS },
        "and it ran to its own exit"
    );
    assert!(
        ledger[1].steps < QUANTUM,
        "a slice that ends on the program ends early: {} ticks, quantum {QUANTUM}",
        ledger[1].steps
    );

    // The preempted process is interrupted, not finished.
    assert!(
        k.processes[0].ended.is_none(),
        "preemption is not a death sentence: the spinner has no `ended` and is runnable"
    );
    assert!(k.ready().contains(&0), "and it is back on the run queue");
    assert!(
        !k.ready().contains(&1),
        "the process that exited is off the queue and cannot be scheduled again"
    );
    assert_eq!(
        order(&ledger).iter().filter(|&&p| p == 1).count(),
        1,
        "and it took exactly one turn — a scheduler that re-ran a dead process would \
         take more"
    );

    // It really executed. `t0` is the spinner's own odometer, and the kernel never wrote it.
    assert!(
        k.processes[0].x[5] > 0,
        "the spinner got round its loop before it was interrupted"
    );
    assert_ne!(
        k.processes[1].sepc, k.processes[1].entry,
        "and the exiter moved off its entry point, so it executed instructions rather \
         than being marked done"
    );
}

/// **The control the row demanded.** Same two programs, same budget, a kernel with no
/// quantum — and the second process never executes an instruction.
///
/// This is the test that fails if preemption is deleted from the preemptive kernel, and it
/// is the reason a green above is not a budget running out under another name.
#[test]
fn without_a_quantum_the_spinner_keeps_the_hart_and_the_other_process_never_runs() {
    let (mut m, mut k) = spawned(Kernel::new(FREE), &[spinner(), exiter()]);
    let mut uart = Vec::new();
    let ledger = k.schedule(&mut m, BUDGET, &mut uart);

    assert_eq!(ledger.len(), 1, "one slice, and it never ended: {ledger:?}");
    assert_eq!(
        ledger[0],
        Slice {
            pid: 0,
            steps: BUDGET,
            ended: Ended::OutOfBudget,
        },
        "the spinner ate the entire budget in one turn, because nothing armed a timer"
    );
    assert_eq!(
        k.processes[1].sepc, k.processes[1].entry,
        "and the exiter is still standing on its entry point — it retired NOTHING. This \
         is the whole difference the row was filed for: a cooperative scheduler is \
         indistinguishable from a preemptive one until a process refuses to yield"
    );
    assert_eq!(
        k.ready(),
        &[0, 1],
        "both are still queued: one never finished its turn, the other never got one"
    );
}

/// The run queue is a queue. Three spinners take the hart in turn, and the one just
/// interrupted goes to the back rather than straight back on.
#[test]
fn the_run_queue_is_round_robin_and_not_a_stack() {
    let (mut m, mut k) = spawned(
        Kernel::preemptive(FREE, QUANTUM),
        &[spinner(), spinner(), spinner()],
    );
    let mut uart = Vec::new();
    let ledger = k.schedule(&mut m, QUANTUM * 6, &mut uart);

    assert_eq!(
        order(&ledger),
        vec![0, 1, 2, 0, 1, 2],
        "front takes the hart and goes to the back. A scheduler that requeued at the \
         front would read 0,0,0,0,0,0 and would be a preemption that changed nothing; one \
         that scanned for the lowest runnable pid would read the same"
    );
    assert!(
        ledger.iter().all(|s| s.ended == Ended::Preempted),
        "and every one of the six turns ended on the clock: {ledger:?}"
    );
    for pid in 0..3 {
        assert_eq!(
            k.processes[pid].x[5], k.processes[0].x[5],
            "three identical programs given equal turns are equally far along — this is \
             what fair means here, and it is measured in the processes' own registers"
        );
    }
}

/// The quantum's arithmetic, stated in the process's own instructions rather than in the
/// kernel's account of them.
///
/// A quantum of `q` ticks spends two on the kernel — the supervisor's `sret` on the way
/// in, and the step on which the interrupt is taken instead of an instruction — so the
/// process retires `q - 2`. [`spinner`]'s loop is two instructions and it counts in `t0`,
/// so `t0` after one slice must be `(q - 1) / 2`.
#[test]
fn a_quantum_is_ticks_of_the_clock_and_two_of_them_are_the_kernel_s() {
    for quantum in [3, 4, 9, 20, 21] {
        let (mut m, mut k) = spawned(Kernel::preemptive(FREE, quantum), &[spinner()]);
        let mut uart = Vec::new();
        let ledger = k.schedule(&mut m, quantum, &mut uart);

        assert_eq!(
            ledger[0].steps, quantum,
            "a preempted slice is exactly the quantum long, at q={quantum}"
        );
        assert_eq!(
            k.processes[0].x[5],
            (quantum - 1) / 2,
            "at q={quantum} the process retires {} instructions of its own, which is {} \
             turns of a two-instruction loop",
            quantum - 2,
            (quantum - 1) / 2
        );
    }
}

/// A preempted process resumes where it was, in U-mode, and keeps counting — so the
/// context saved by the timer is a resume and not a restart.
#[test]
fn a_preempted_process_resumes_and_does_not_begin_again() {
    let (mut m, mut k) = spawned(Kernel::preemptive(FREE, QUANTUM), &[spinner()]);
    let mut uart = Vec::new();

    let first = k.schedule(&mut m, QUANTUM, &mut uart);
    assert_eq!(first[0].ended, Ended::Preempted);
    let after_one = k.processes[0].x[5];

    let second = k.schedule(&mut m, QUANTUM, &mut uart);
    assert_eq!(second[0].ended, Ended::Preempted);
    let after_two = k.processes[0].x[5];

    // Two slices retire `2 * (QUANTUM - 2)` of the process's instructions between them,
    // and the loop's first instruction is the `addi`, so the odometer reads that many
    // rounded up over two. **Not `after_one * 2`**: the first slice stopped in the middle
    // of a loop and the second resumed on the *jump*, which is itself evidence that the
    // saved `sepc` was an arbitrary point inside the program and not a boundary.
    assert_eq!(
        after_two,
        (2 * (QUANTUM - 2)).div_ceil(2),
        "the second slice carried on from the first: {after_one} loops became {after_two}. \
         A restart would have left it at {after_one}, and a context restored into S-mode \
         would have left it at 0"
    );
    assert!(
        after_two > after_one,
        "and it moved: {after_one} then {after_two}"
    );
    assert!(
        k.processes[0].ended.is_none(),
        "and after two preemptions it is still runnable"
    );
}

/// A slice may end on the program. The timer is not the only thing that stops one, which
/// is what keeps `Ended::Preempted` a measurement rather than the only answer.
#[test]
fn a_process_that_exits_ends_its_own_slice_and_leaves_the_queue() {
    let (mut m, mut k) = spawned(Kernel::preemptive(FREE, QUANTUM), &[exiter()]);
    let mut uart = Vec::new();
    let ledger = k.schedule(&mut m, BUDGET, &mut uart);

    assert_eq!(
        ledger,
        vec![Slice {
            pid: 0,
            steps: ledger[0].steps,
            ended: Ended::Exited { status: STATUS },
        }],
        "one slice, ended by the program"
    );
    assert!(
        ledger[0].steps < QUANTUM,
        "and it ended before the clock could: {} ticks against a quantum of {QUANTUM}",
        ledger[0].steps
    );
    assert!(
        k.ready().is_empty(),
        "the queue is empty, so a schedule with nothing runnable returns nothing rather \
         than spinning"
    );
    assert!(
        k.schedule(&mut m, BUDGET, &mut uart).is_empty(),
        "and asking again is not an error and is not a turn"
    );
}

/// A quantum that would hand out an empty slice is refused, because a scheduler that makes
/// no progress is a hang and not a fast one.
#[test]
#[should_panic(expected = "retires no instruction of the process")]
fn a_quantum_too_short_to_retire_an_instruction_is_refused() {
    let _ = Kernel::preemptive(FREE, 2);
}

// --- W-220: a slice the budget cut resumes where the hart stood ---------------------------

/// A program that counts every `addi t0` it retires, makes one call in the middle that the
/// kernel answers and resumes, and exits with `t0` as its status. So the exit status is the
/// number of counting instructions retired — [`COUNTED`] — and an instruction retired
/// twice reads as a status one too high. That is the side effect a wrong resume makes
/// visible twice.
fn counter() -> Vec<u8> {
    image(&[
        addi(5, 5, 1),   // 0: t0 += 1
        addi(17, 0, 99), // 1: a7 = 99, a number no table assigns
        ECALL,           // 2: answered असमर्थितम्, and the process resumes past it
        addi(5, 5, 1),   // 3: t0 += 1
        addi(5, 5, 1),   // 4: t0 += 1
        addi(10, 5, 0),  // 5: a0 = t0
        addi(17, 0, 0),  // 6: a7 = EXIT
        ECALL,           // 7
    ])
}
/// How many `addi t0` [`counter`] holds, and therefore the only correct exit status.
const COUNTED: u64 = 3;
/// Ticks from the switch to [`counter`]'s exit: `sret`, two `addi`, the `ecall`'s trap,
/// the `sret` back, three `addi`, one `addi`, and the exit's `ecall`.
const COUNTER_TICKS: u64 = 10;

/// **`W-220`.** The budget ends a slice in U-mode two instructions past the last trap. The
/// saved program counter must be where the hart stood — not `sepc`, which no trap has
/// moved since the `ecall` — or the resume re-executes word 3 and `t0` counts it twice.
#[test]
fn an_out_of_budget_slice_resumes_where_the_hart_stood_and_not_at_the_last_trap() {
    let (mut m, mut k) = spawned(Kernel::new(FREE), &[counter()]);
    let mut uart = Vec::new();

    // Six ticks: `sret`, addi, addi, the `ecall` (trapped and answered), `sret`, addi.
    // The hart is now in U-mode on word 4, and the CSR still names word 3.
    let first = k.run(&mut m, 0, 6, &mut uart);
    assert_eq!(first, Ended::OutOfBudget);
    assert_eq!(
        m.mode,
        Privilege::User,
        "the slice ended in U-mode, without a trap"
    );
    assert_eq!(m.pc, APP + 16, "the hart stands on word 4");
    assert_eq!(
        m.csr.sepc,
        APP + 12,
        "and the CSR was last written by the ecall's trap"
    );
    assert_eq!(k.processes[0].x[5], 2, "two counts so far");
    assert_eq!(
        k.processes[0].sepc, m.pc,
        "the saved program counter is where the hart stood ({:#x}), not the last trap's \
         sepc ({:#x}) — ADR-0025 says a later run *resumes the turn*",
        m.pc, m.csr.sepc
    );

    let second = k.run(&mut m, 0, BUDGET, &mut uart);
    assert_eq!(
        second,
        Ended::Exited { status: COUNTED },
        "the second slice started exactly where the first stopped. A status of {} is word \
         3 retired twice: the old save copied `sepc` from the CSR, which the trap had left \
         at the ecall's successor",
        COUNTED + 1
    );
}

/// Every place the budget can cut [`counter`] — in U-mode between instructions, in S-mode
/// standing on the trap's `sret`, on the entry before the first `sret` returns — the resume
/// finishes the program with exactly [`COUNTED`] counts. The cut at four ticks is the one
/// that ends on the trap: there the CSR *is* the right answer, and the fix must not
/// prefer the hart's `pc`, which is `stvec`.
#[test]
fn every_cut_of_a_slice_resumes_exactly_once_per_instruction() {
    for cut in 1..COUNTER_TICKS {
        let (mut m, mut k) = spawned(Kernel::new(FREE), &[counter()]);
        let mut uart = Vec::new();
        let first = k.run(&mut m, 0, cut, &mut uart);
        assert_eq!(first, Ended::OutOfBudget, "cut at {cut}");
        if cut == 4 {
            assert_eq!(
                m.mode,
                Privilege::Supervisor,
                "the fourth tick took the trap"
            );
            assert_eq!(
                k.processes[0].sepc,
                APP + 12,
                "and the saved program counter is the ecall's successor, not stvec"
            );
        }
        let second = k.run(&mut m, 0, BUDGET, &mut uart);
        assert_eq!(
            second,
            Ended::Exited { status: COUNTED },
            "cut at {cut}: the resume retired every instruction once"
        );
    }
    // And the uncut program agrees on the count, so the constant is measured, not assumed.
    let (mut m, mut k) = spawned(Kernel::new(FREE), &[counter()]);
    let before = m.time;
    assert_eq!(
        k.run(&mut m, 0, BUDGET, &mut Vec::new()),
        Ended::Exited { status: COUNTED }
    );
    assert_eq!(m.time - before, COUNTER_TICKS);
}

/// The same defect through the scheduler: a turn cut by the *schedule's* budget keeps the
/// front of the queue (ADR-0025) and a later schedule resumes it — on the instruction it
/// stopped at, not on the one the last trap named. [`spinner`] never traps, so the last
/// trap's `sepc` is its entry, and the old behaviour restarted it.
#[test]
fn a_turn_cut_by_the_schedules_budget_resumes_where_it_stopped() {
    let (mut m, mut k) = spawned(Kernel::preemptive(FREE, QUANTUM), &[spinner()]);
    let mut uart = Vec::new();

    // Four ticks: `sret`, addi, jal, addi — stopped on the jump with `t0 = 2`.
    let first = k.schedule(&mut m, 4, &mut uart);
    assert_eq!(
        first,
        vec![Slice {
            pid: 0,
            steps: 4,
            ended: Ended::OutOfBudget,
        }]
    );
    assert_eq!(k.processes[0].x[5], 2);
    assert_eq!(
        k.ready(),
        &[0],
        "still at the front, so the next schedule resumes it"
    );

    // Four more: `sret`, jal, addi, jal — one more count, not two.
    let second = k.schedule(&mut m, 4, &mut uart);
    assert_eq!(second[0].ended, Ended::OutOfBudget);
    assert_eq!(
        k.processes[0].x[5], 3,
        "the turn resumed on the jump it stopped at. A reading of 4 is the first `addi` \
         retired again — the slice restarted from the entry, which was the last (and \
         only) thing the CSR ever held"
    );
}

// --- W-219: a U-mode `jal x0, .` idles, and the clock takes the slice ---------------------

/// **The idle idiom.** `jal x0, .` — how both demo programs and every boot proof park once
/// they are done. On the bare machine it is [`Halt::SpinForever`]; under the kernel the
/// program is a process, and a process cannot stop the machine (ADR-0015 A3).
fn parker() -> Vec<u8> {
    image(&[jal(0, 0)])
}

/// **`W-219`'s decision, (a).** A parked process idles: it retires its jump once per tick,
/// changes nothing, and the clock takes the slice exactly as it takes a [`spinner`]'s. The
/// process behind it runs, and the parked one keeps taking turns until the budget is gone
/// — a program's end under a scheduler is a re-entry (research/22 §7, ह-14 → ल), not a
/// halt.
#[test]
fn a_user_mode_self_jump_idles_and_the_clock_takes_the_slice() {
    let (mut m, mut k) = spawned(Kernel::preemptive(FREE, QUANTUM), &[parker(), exiter()]);
    let mut uart = Vec::new();
    let ledger = k.schedule(&mut m, BUDGET, &mut uart);

    assert_eq!(
        ledger[0],
        Slice {
            pid: 0,
            steps: QUANTUM,
            ended: Ended::Preempted,
        },
        "the parked process idled for exactly one quantum and the clock took the slice. \
         `Stopped(SpinForever)` here is the whole machine halting on a user program's \
         word, which ADR-0025 and A3 both refuse: {ledger:?}"
    );
    assert_eq!(
        ledger[1].ended,
        Ended::Exited { status: STATUS },
        "and the process behind it got the hart and ran to its exit"
    );
    assert!(
        ledger.iter().all(|s| !matches!(s.ended, Ended::Stopped(_))),
        "nothing a process did stopped the machine: {ledger:?}"
    );
    assert!(
        ledger.len() > 2
            && ledger[2..]
                .iter()
                .all(|s| s.pid == 0 && s.ended == Ended::Preempted),
        "every later turn is the parked process's, and every one ends on the clock"
    );
    assert_eq!(
        ledger.iter().map(|s| s.steps).sum::<u64>(),
        BUDGET,
        "idling costs a tick per jump like any instruction — the ledger accounts for the \
         whole budget"
    );
    assert!(
        k.processes[0].ended.is_none(),
        "the parked process is idle, not ended"
    );
    assert_eq!(
        k.processes[0].sepc, APP,
        "and it is still standing on its jump"
    );
    assert_eq!(
        k.processes[0].x[0], 0,
        "x0 is still zero: the jump's link write did not stick to the zero register"
    );
}

/// **The refused case, twice.** The same word, not in U-mode, still halts the machine.
/// On the bare machine — `yantra-run`, no kernel above it — a `jal x0, .` is the end of
/// the program and [`Halt::SpinForever`] is the whole truth (research/22 §7: "on a bare
/// machine ह at 14 really is the end"); 0% re-entry there is by construction and stays.
/// And under the kernel, if the *supervisor's* own word were a self-jump, that is the
/// kernel spinning in S-mode and the machine is rightly stopped: the key is the privilege
/// the jump executed in, not whether a scheduler happens to exist.
#[test]
fn a_self_jump_that_is_not_a_processs_still_halts_the_machine() {
    // Bare: this interpreter is the firmware, so its bare mode is S-mode (there is no M).
    let mut m = machine();
    m.mem[..4].copy_from_slice(&jal(0, 0).to_le_bytes());
    assert_eq!(m.mode, Privilege::Supervisor);
    assert_eq!(
        m.run(BUDGET, &mut Vec::new()),
        Halt::SpinForever { pc: BASE },
        "the bare machine halts on the spin, as it always did"
    );

    // Under the kernel, in S-mode: the supervisor's `sret` replaced by the spin.
    let (mut m, mut k) = spawned(Kernel::preemptive(FREE, QUANTUM), &[exiter()]);
    m.mem[..4].copy_from_slice(&jal(0, 0).to_le_bytes());
    let ledger = k.schedule(&mut m, BUDGET, &mut Vec::new());
    assert_eq!(
        ledger,
        vec![Slice {
            pid: 0,
            steps: 1,
            ended: Ended::Stopped(Halt::SpinForever { pc: BASE }),
        }],
        "a spin in S-mode is the machine's to report, and the schedule ends on it"
    );
    assert_eq!(
        k.processes[0].ended,
        Some(Ended::Stopped(Halt::SpinForever { pc: BASE }))
    );
}

/// Under the cooperative kernel there is no clock to take the slice, so the parked process
/// keeps the hart until the caller's budget runs out — exactly what [`spinner`] does there
/// (`without_a_quantum_the_spinner_keeps_the_hart_and_the_other_process_never_runs`). The
/// one-word loop and the two-word loop are the same case to a kernel, and neither stops
/// the machine.
#[test]
fn under_the_cooperative_kernel_a_self_jump_burns_the_budget_and_stops_nothing() {
    let (mut m, mut k) = spawned(Kernel::new(FREE), &[parker(), exiter()]);
    let ledger = k.schedule(&mut m, BUDGET, &mut Vec::new());
    assert_eq!(
        ledger,
        vec![Slice {
            pid: 0,
            steps: BUDGET,
            ended: Ended::OutOfBudget,
        }],
        "one slice, the whole budget, and the caller's limit ended it — not the machine"
    );
    assert_eq!(
        k.processes[1].sepc, k.processes[1].entry,
        "the exiter never ran: nothing armed a timer"
    );
    assert_eq!(k.ready(), &[0, 1]);
    assert_eq!(k.processes[0].sepc, APP, "parked, and resumable");
}
