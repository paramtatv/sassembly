//! `F-001c2b3` — the clock, the interrupt it raises, and the last three CSRs.
//!
//! # Two columns, as `F-001c2b2` had
//!
//! This row adds **no instruction**: `sie`, `sip` and `time` are reached through the six
//! CSR forms `F-001c2a` decoded, and `sbi_set_timer` through the `ecall` `F-001c1`
//! decoded. `tests/system.rs` already holds every one of them against
//! `spec/encodings-riscv64.tsv`. What moved is that a hart can now be interrupted, which
//! no encoding table records. So:
//!
//! 1. **The meaning**, written from the privileged specification by hand: what advances
//!    `time`, what `sip.STIP` is a function of, which of `sie`/`sstatus.SIE` gates what,
//!    the priority between two pending interrupts, and what `stvec`'s vectored mode
//!    spreads.
//! 2. **A whole program** — `spec/timer.sas`, at both link addresses, asserted against the
//!    *same word* `tools/check-timer.sh` demands of QEMU under real OpenSBI, **and against
//!    that script's negative control**: the same program with its deadline moved to the
//!    far future must print nothing at all.
//!
//! # The one bit that carries the claim
//!
//! `scause` for a supervisor timer is 5. On an **interrupt** bit 63 is also set, so the
//! handler prints `0x8000000000000005` and not `5`. That single bit is the whole
//! difference between an interrupt and an exception, and `spec/trap.sas`'s `3` is the
//! other end of the same column: same register, same handler shape, a cause that was
//! asked for by an instruction rather than arriving from outside it.
//!
//! # Why the negative control is not optional
//!
//! `tools/check-timer.sh` records that deleting the `set_timer` call left it passing:
//! QEMU's `mtimecmp` starts at zero, so a timer interrupt is *already pending at boot* and
//! opening `STIE` and `SIE` is enough to take one. A VM that fired on any enabled timer
//! would pass every positive assertion here and be wrong in exactly that way — so
//! [`a_far_deadline_never_fires_and_the_program_spins`] rebuilds the program with the
//! deadline the check script's `sed` writes, and demands silence.

use sadhana::encode::{Target, encode_object, layout_addresses};
use sadhana::kosha::{object, write_debuggable_at};
use sadhana::parse::assemble_program;
use sadhana::samyojana::link_at;
use sadhana::vastu::read;
use yantra::{Csrs, Halt, INTERRUPT, Machine, Privilege};

const BASE: u64 = 0x8000_0000;
/// 16 MiB, as `tests/user.rs` uses: `spec/timer.sas` links at `0x8020_0000` and its `.bss`
/// stays well inside this.
const RAM: usize = 1 << 24;

const SIP: u32 = 0x144;
const TIME: u32 = 0xc01;

/// `sstatus.SIE`, bit 1 — "not while I am in here", said by the kernel.
const SSTATUS_SIE: u64 = 1 << 1;
/// `sstatus.SPP`, bit 8.
const SPP: u64 = 1 << 8;

/// `sie`/`sip` bit 1 — supervisor software interrupt, cause 1.
const SSI: u64 = 1 << 1;
/// `sie`/`sip` bit 5 — supervisor timer interrupt, cause 5. The claim.
const STI: u64 = 1 << 5;

/// Where a handler is installed in the hand-built machines below.
const HANDLER: u64 = BASE + 0x800;

const ECALL: u32 = 0x0000_0073;

/// `csrrw`/`csrrs`/`csrrc` and their immediate forms: `funct3` selects which.
fn csr_insn(funct3: u32, rd: u32, csr: u32, rs1: u32) -> u32 {
    0x73 | rd << 7 | funct3 << 12 | rs1 << 15 | csr << 20
}

/// A machine holding `words` at the entry point and nothing else.
fn machine(words: &[u32]) -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX, // W-363: no store bound beyond `mem` — this machine has no injected input above it
        // Added with the `patra` file window: a machine that was never asked
        // to serve files must not be able to.
        patra_root: None,
        patra_path: None,
        patra_buffer: None,
        virtio: Default::default(),
        x: [0; 32],
        f: [0; 32],
        fcsr: 0,
        pc: BASE,
        base: BASE,
        mem: vec![0; 1 << 16],
        reservation: None,
        csr: Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
        socket: None,
    };
    for (i, w) in words.iter().enumerate() {
        m.mem[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    m
}

// ---------------------------------------------------------------------------------------
// Column 1: the meaning.

#[test]
fn time_counts_the_instructions_and_nothing_else() {
    // The decision this row had to make, and the one recorded in `.loop/ASSUMPTIONS.md`:
    // a browser tab's wall clock is not this hart's. Reading the host's clock would make
    // the SAME program take a different number of ticks on every run and fire its handler
    // at a different instruction each time — so `time` counts instructions, which is the
    // only quantity here that is a function of the program alone.
    //
    // `csrrs x5, time, x0` twice, with an `addi x0, x0, 0` between them.
    let read = csr_insn(0x2, 5, TIME, 0);
    let second = csr_insn(0x2, 6, TIME, 0);
    let mut m = machine(&[read, 0x0000_0013, second]);
    for _ in 0..3 {
        assert_eq!(m.step(&mut Vec::new()), None);
    }
    assert_eq!(
        m.x[6] - m.x[5],
        2,
        "two instructions began between the two reads, so the counter moved by two"
    );
    // And it is the same two on a second run. Determinism is the whole reason for the
    // choice, so it is asserted rather than assumed.
    let mut again = machine(&[read, 0x0000_0013, second]);
    for _ in 0..3 {
        assert_eq!(again.step(&mut Vec::new()), None);
    }
    assert_eq!((again.x[5], again.x[6]), (m.x[5], m.x[6]));
}

#[test]
fn a_write_to_time_is_an_illegal_instruction_from_supervisor_mode_too() {
    // Bits 11:10 of a CSR number are `11` for a read-only register. `time` is a counter,
    // and a kernel that could set it could run its own deadlines backwards — so this is
    // illegal from S-mode, not merely from U-mode like every other privileged access in
    // `tests/user.rs`.
    let word = csr_insn(0x1, 5, TIME, 6); // csrrw x5, time, x6
    let mut m = machine(&[word]);
    m.csr.stvec = HANDLER;
    m.x[6] = 0xdead_beef;
    assert_eq!(m.step(&mut Vec::new()), None, "delivered, not halted");
    assert_eq!(m.csr.scause, 2, "illegal instruction");
    assert_eq!(m.csr.stval, u64::from(word), "stval is the instruction");
    assert_eq!(m.pc, HANDLER);
    assert_eq!(
        m.csr.sstatus & SPP,
        SPP,
        "the trap came from supervisor mode"
    );
    assert_eq!(
        m.x[5], 0,
        "the WHOLE instruction is illegal, so rd is not written either — a machine that \
         refused the write but returned the count would have executed half of it"
    );
    // A bare read of the same register is not a write and must not trap.
    let mut m = machine(&[csr_insn(0x2, 5, TIME, 0)]);
    m.csr.stvec = HANDLER;
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(
        m.pc,
        BASE + 4,
        "a read of `time` runs to the next instruction"
    );
    assert_eq!(m.x[5], 1, "and it is the count, which is 1 after one step");
}

#[test]
fn stip_is_the_deadline_and_not_a_bit_software_owns() {
    // `sip.STIP` is written by the firmware on hardware. This interpreter IS the firmware,
    // and it computes the bit from the deadline every time rather than storing it — so
    // there is no moment where a stored bit and the clock disagree, and a write of STIP
    // by software is simply dropped.
    let read = csr_insn(0x2, 5, SIP, 0);
    let mut m = machine(&[read, read, read]);
    m.timecmp = Some(2);
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(
        m.x[5] & STI,
        0,
        "time is 1, the deadline is 2, nothing is due"
    );
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(
        m.x[5] & STI,
        STI,
        "time reached the deadline and the bit is set"
    );

    // A machine with no deadline at all is not a machine with a deadline of zero.
    let mut never = machine(&[read]);
    assert_eq!(never.step(&mut Vec::new()), None);
    assert_eq!(
        never.x[5] & STI,
        0,
        "an unarmed timer is never pending — QEMU's mtimecmp starts at zero and that is \
         exactly the difference tools/check-timer.sh's negative control was written for"
    );

    // Software may set SSIP and only SSIP.
    let mut m = machine(&[csr_insn(0x1, 0, SIP, 6), read]);
    m.x[6] = SSI | STI;
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(
        m.x[5], SSI,
        "SSIP took, STIP did not: it is read-only, and this is why `spec/timer.sas` stops \
         its clock with set_timer rather than by clearing the bit"
    );
}

#[test]
fn an_enabled_pending_timer_is_taken_and_scause_says_interrupt() {
    // The claim, in the smallest machine that can carry it: an infinite loop that is left
    // anyway. `jal x0, .` at the entry point — the idiom `spec/timer.sas` waits in — with
    // the timer already due.
    let mut m = machine(&[0x0000_006f]);
    m.csr.stvec = HANDLER;
    m.csr.sie = STI;
    m.csr.sstatus = SSTATUS_SIE;
    m.timecmp = Some(0);
    assert_eq!(m.step(&mut Vec::new()), None, "delivered, not halted");
    assert_eq!(
        m.csr.scause,
        INTERRUPT | 5,
        "cause 5 with bit 63 set. Without bit 63 this is a load-address-misaligned \
         exception, which is a different event entirely"
    );
    assert_eq!(m.pc, HANDLER);
    assert_eq!(
        m.csr.sepc, BASE,
        "sepc names the instruction that did NOT run, so an sret resumes the loop rather \
         than skipping past it"
    );
    assert_eq!(m.csr.stval, 0, "an interrupt has no faulting address");
    assert_eq!(m.csr.sstatus & SSTATUS_SIE, 0, "SIE is cleared on entry");

    // Without `sie.STIE` the same machine spins for ever: an interrupt the kernel never
    // enabled is not one it may be given.
    let mut deaf = machine(&[0x0000_006f]);
    deaf.csr.stvec = HANDLER;
    deaf.csr.sstatus = SSTATUS_SIE;
    deaf.timecmp = Some(0);
    assert_eq!(
        deaf.run(100, &mut Vec::new()),
        Halt::SpinForever { pc: BASE },
        "pending is not enabled"
    );
}

#[test]
fn sstatus_sie_holds_interrupts_off_in_s_mode_and_cannot_in_u_mode() {
    // `SIE` is the kernel saying "not while I am in here". A user program is never in
    // there — so an S-mode interrupt is taken from U-mode whatever `SIE` says, which is
    // what stops a user program holding the kernel's own clock off by looping.
    for (mode, taken) in [(Privilege::Supervisor, false), (Privilege::User, true)] {
        let mut m = machine(&[0x0000_006f]);
        m.csr.stvec = HANDLER;
        m.csr.sie = STI;
        m.csr.sstatus = 0; // SIE clear
        m.timecmp = Some(0);
        m.mode = mode;
        let halt = m.step(&mut Vec::new());
        if taken {
            assert_eq!(halt, None, "{mode:?}");
            assert_eq!(m.pc, HANDLER, "{mode:?}: taken despite SIE being clear");
            assert_eq!(m.csr.scause, INTERRUPT | 5);
            assert_eq!(m.mode, Privilege::Supervisor, "and it entered the kernel");
            assert_eq!(m.csr.sstatus & SPP, 0, "SPP says the trap came from U-mode");
        } else {
            // The `jal x0, .` executed instead, and this interpreter recognises a jump to
            // itself as the end of the program — which is precisely the wait `SIE` was
            // holding the interrupt off from interrupting.
            assert_eq!(halt, Some(Halt::SpinForever { pc: BASE }), "{mode:?}");
            assert_eq!(m.csr.scause, 0, "and nothing was delivered");
        }
    }
}

#[test]
fn two_pending_interrupts_are_taken_in_the_spec_s_priority() {
    // The privileged spec orders them external, software, timer. Both of the two this
    // machine can raise are made pending at once, and the software interrupt must win —
    // a machine that took them in bit order would take the timer and be wrong in a way
    // that only shows up when a kernel is doing both.
    let mut m = machine(&[0x0000_006f]);
    m.csr.stvec = HANDLER;
    m.csr.sie = SSI | STI;
    m.csr.sip = SSI;
    m.csr.sstatus = SSTATUS_SIE;
    m.timecmp = Some(0);
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(m.csr.scause, INTERRUPT | 1, "software before timer");
}

#[test]
fn vectored_stvec_spreads_interrupts_and_leaves_exceptions_at_the_base() {
    // `MODE = 1` sends interrupt `n` to `BASE + 4n` and every exception to `BASE`. Until
    // this row raised an interrupt the distinction could not be observed, so it was not
    // made; a machine that vectored exceptions too would send `spec/trap.sas`'s breakpoint
    // to the wrong address.
    let mut m = machine(&[0x0000_006f]);
    m.csr.stvec = HANDLER | 1;
    m.csr.sie = STI;
    m.csr.sstatus = SSTATUS_SIE;
    m.timecmp = Some(0);
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(m.pc, HANDLER + 4 * 5, "timer is cause 5, so BASE + 20");

    // The same `stvec`, an exception: `ebreak`.
    let mut m = machine(&[0x0010_0073]);
    m.csr.stvec = HANDLER | 1;
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(m.pc, HANDLER, "an exception enters at the base either way");
}

#[test]
fn set_timer_arms_an_absolute_deadline_and_moving_it_stops_the_clock() {
    // `a0` is an absolute deadline, not a delay — which is why `spec/timer.sas` can ask
    // for 0 and mean "now" and for -1 and mean "never" without reading the clock first.
    let mut m = machine(&[ECALL, ECALL]);
    m.x[17] = 0; // a7 = sbi_set_timer
    m.x[10] = 0; // a0 = a deadline already past
    assert_eq!(m.step(&mut Vec::new()), None, "it returns; it used to halt");
    assert_eq!(m.timecmp, Some(0));
    assert_eq!(m.x[10], 0, "the legacy call returns 0 in a0");

    // The handler's first act in `spec/timer.sas`: move the deadline to the furthest
    // moment there is. Returning with the old one still armed re-enters the handler on
    // the instruction after the `sret` — an infinite interrupt rather than an infinite
    // loop.
    m.x[17] = 0;
    m.x[10] = u64::MAX;
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(m.timecmp, Some(u64::MAX));
    let mut sip = machine(&[csr_insn(0x2, 5, SIP, 0)]);
    sip.timecmp = Some(u64::MAX);
    assert_eq!(sip.step(&mut Vec::new()), None);
    assert_eq!(
        sip.x[5] & STI,
        0,
        "moving the deadline cleared the pending bit"
    );
}

#[test]
fn an_interrupt_with_no_vector_stops_rather_than_running_the_rubble_at_zero() {
    // The same answer `ebreak`, a page fault and a user trap give. An interrupt is not
    // asked for by any instruction, so this is the one of the four that can arrive at a
    // machine which never installed a handler *and never executed anything privileged*.
    let mut m = machine(&[0x0000_006f]);
    m.csr.sie = STI;
    m.csr.sstatus = SSTATUS_SIE;
    m.timecmp = Some(0);
    assert_eq!(
        m.step(&mut Vec::new()),
        Some(Halt::Undelivered {
            pc: BASE,
            cause: INTERRUPT | 5
        })
    );
}

// ---------------------------------------------------------------------------------------
// Column 2: the whole program.

/// Assemble `source` and link it at `load`, as `tests/user.rs` does.
fn build(source: &str, load: u64) -> Vec<u8> {
    let program = assemble_program(source).unwrap_or_else(|e| panic!("{e:?}"));
    let (text, pending) = encode_object(&program).unwrap_or_else(|e| panic!("{e:?}"));
    let bytes = object(
        &text,
        &program,
        &pending,
        None,
        &layout_addresses(&program, Target::Uncompressed),
    );
    let objects = [read(&bytes).expect("the object reads back")];
    let image = link_at(&objects, load).unwrap_or_else(|e| panic!("{e:?}"));
    write_debuggable_at(&image.text, &image.data, &image.table, image.bss, &[], load)
}

/// The program `tools/check-timer.sh` boots under real OpenSBI: `set_timer` with a
/// deadline already past, `STIE` and `SIE` opened, and a wait loop with no other way out.
const TIMER: &str = include_str!("../../../spec/timer.sas");

/// Run `source` at `load` and return what it printed and why it stopped.
fn run(source: &str, load: u64) -> (String, Halt) {
    let elf = build(source, load);
    let mut m = Machine::load_elf(&elf, RAM).expect("the toolchain's own ELF must load");
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(5_000_000, &mut out);
    (
        String::from_utf8(out).expect("the program prints ASCII hex"),
        halt,
    )
}

#[test]
fn timer_sas_says_the_same_word_qemu_hears() {
    for load in [0x8020_0000u64, 0x8040_0000] {
        let (out, halt) = run(TIMER, load);
        assert!(
            matches!(halt, Halt::Shutdown { .. }),
            "the handler ends by asking the firmware to stop: {halt:?}"
        );
        let word = out.trim_end();
        assert_eq!(
            word, "8000000000000005",
            "`tools/check-timer.sh`'s only assertion, at load {load:#x}. \
             `0000000000000005` would be an exception, not an interrupt; nothing at all \
             would mean the clock never arrived"
        );
    }
}

#[test]
fn a_far_deadline_never_fires_and_the_program_spins() {
    // `tools/check-timer.sh`'s negative control, and its `sed` is reproduced exactly: the
    // deadline `0` becomes `-1`, read unsigned as the furthest moment there is. The check
    // script found that deleting `set_timer` altogether left it PASSING, because QEMU's
    // `mtimecmp` starts at zero and a timer was already pending at boot.
    //
    // Here the same mutation must produce silence and a machine still in its wait loop.
    // `Halt::SpinForever` is what a `jal x0, .` becomes in this interpreter and it is the
    // exact shape of "QEMU had to be killed by the bound".
    let far = TIMER.replace("योगः अर्थ०म् शून्यःन ०न ।", "योगः अर्थ०म् शून्यःन ऋण१न ।");
    assert_ne!(far, TIMER, "the mutation must actually apply");
    let (out, halt) = run(&far, 0x8020_0000);
    assert_eq!(out, "", "a deadline in the far future fired anyway");
    assert!(
        matches!(halt, Halt::SpinForever { .. }),
        "the program must still be waiting: {halt:?}"
    );
}
