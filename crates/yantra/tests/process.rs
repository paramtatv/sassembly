//! `C-014` — two processes in separate address spaces, a channel between them, and the
//! half the row is actually about: **one process trying to read the other's memory and
//! being refused**.
//!
//! # What was already proved, and where, so this claims only what is new
//!
//! - `C-002l` is `done` and its evidence is `spec/second-space.sas` +
//!   `tools/check-second-space.sh`: two root tables, and **one virtual address answering
//!   differently under each `satp`**, with a negative control that collapses the two
//!   answers when B's megapage is pointed at A's window table.
//! - `C-002m1`/`C-002m2` are `done` — `spec/crossing.sas`, `spec/milestone-k2.sas`: a word
//!   leaves A's space and is read in B's, and a derived reply returns. **The channel half
//!   of this row is already proved there**, under QEMU, and is not re-claimed here.
//! - **Neither of them proves isolation, and both say so.**
//!   `tools/check-second-space.sh`'s "What a green here does NOT prove" reads: *"Isolation.
//!   One address answering two ways is not the claim that neither process can reach the
//!   other's frames. Both maps carry the kernel's identity mapping and nothing here tries
//!   to remove it."* `second-space.sas` says the same in its own `जो यह नहीं कहता`.
//!   **Nothing tried.** That is the gap, and it is the only clause of `C-014` that was
//!   genuinely open.
//! - `E-010`'s `spec/powerbox.sas` proved a delegated capability confines an app to one
//!   file. Its own closing text says what it did not prove: *"no IPC between separate
//!   address spaces, and no MMU — the confinement is a checked capability slot in one
//!   program, not hardware isolation."*
//! - `tests/user.rs` proves the `U` bit is the whole of the protection, and
//!   `tests/loader.rs` proves one address space is built correctly.
//!
//! So what is new here is stated narrowly: **two address spaces held at once inside
//! `crates/`, with saved contexts, a channel that copies between them, and eight attempts
//! to cross the boundary of which seven fail.**
//!
//! # The negative half, and why it is shaped like `E-010`'s
//!
//! `tools/check-powerbox.sh` fixed the rule this file obeys: *"A denial that is proved by
//! the absence of a read has proved nothing: a program that never asks is
//! indistinguishable from one that asked and was refused, and the second is the only one
//! that demonstrates a gate."* So [`eight_attempts_to_read_the_other_process_and_seven_denials`]
//! makes **eight real U-mode loads**, each in its own program, each aimed at a named
//! address, and reports what came back.
//!
//! One reaches what it is entitled to. Seven do not reach what they were aimed at: **six
//! are refused by the machine with a load page fault naming the address**, and one — the
//! sharpest — returns the *prober's own* zero rather than the secret that really is at
//! that virtual address in the other process. That last is the aliasing denial, and it is
//! the one a shared `satp` breaks.
//!
//! # The witness that is not the program's narration
//!
//! `E-010` corroborated its denials with the virtio available-ring index — a measurement
//! of the wire, not the program's opinion. The equivalent here is physical RAM itself:
//! [`the_secret_is_really_there_so_the_denial_is_a_denial`] scans `Machine::mem` and
//! requires the secret to be present **exactly once**, at an offset inside the first
//! process's frame range and outside the second's. Without it, "the prober never saw the
//! secret" would be satisfied by a secret that was never written.
//!
//! # `C-014c` — the write half, which is a separate property
//!
//! Everything above is a **load**. `C-014` proved a peer cannot *read* across the boundary
//! and did not prove it cannot *write* across it, and the two are not the same claim: the
//! machine checks `R` for a load and `W` for a store, so a page table that refuses every
//! read could still accept a store, and every denial above would still be green. The
//! addresses that make the difference are the ones a process **can** read and must not
//! write — its own text and its own handle vector — because an address that is not mapped
//! at all is refused by validity alone and says nothing about `W`.
//!
//! So [`nine_attempts_to_write_across_the_boundary_and_seven_denials`] repeats the shape
//! above with `sd` in place of `ld`, and
//! [`every_frame_the_other_process_was_built_from_is_refused_as_a_store`] repeats the
//! sweep. Two attempts open, and both are meant to: a process writing its own stack, and —
//! the sharp one — a process writing the *virtual* address at which the peer's secret
//! really lives, which lands in the prober's own frame and leaves the secret intact. RAM is
//! the witness again: after that store the secret is still in memory exactly once and the
//! stamp is inside the writer's frames and nowhere else.
//!
//! The second clause of `C-014c` is the channel's own boundary, and it is the mirror of
//! `C-014`'s confused-deputy test. There the attack was on `SEND`'s buffer — *whose memory
//! may the kernel read on a program's behalf*. Here it is on `RECV`'s — **whose memory may
//! the kernel write on a program's behalf**, and where the bytes of a message end up. The
//! destination is never the sender's to name: it is the receiver's `a1`, translated under
//! the receiver's own table with `W` and `U` required, so
//! [`the_channel_will_not_place_a_message_where_the_receiver_may_not_write`] aims that
//! pointer at the supervisor, at the sender's frames and at pages the receiver can read but
//! not write, and [`a_message_lands_only_where_the_receiver_named_it`] measures RAM before
//! and after a legitimate receive and requires **exactly one new copy**, inside the
//! receiver's frames.

use sadhana::kosha::write_debuggable_at;
use yantra::loader::{HANDLE_VECTOR, STACK_TOP};
use yantra::process::{Endpoint, Kernel, RECV, RIKTAH, SEND};
use yantra::supervisor::{
    ADHIKARABHAVAH, Ended, RIKTAPRAVESHAH, SIDDHAM, SIMATIKRAMAH, Surface, WRITE, install,
};
use yantra::{Csrs, Machine, Privilege};

/// Physical RAM, and the supervisor's own address.
const BASE: u64 = 0x8000_0000;
/// 4 MiB. Two address spaces take about thirty frames between them.
const RAM: usize = 1 << 22;
/// Where the loader may start taking frames — the first pool's start.
const FREE: u64 = BASE + 0x1_0000;
/// Where every fixture is linked. Any gigabyte but the supervisor's would do.
const APP: u64 = 0x2000_0000;
/// `scause` 13 — a load that could not be translated.
const LOAD_FAULT: u64 = 13;
/// `scause` 15 — a store that could not be translated. **A different cause from
/// [`LOAD_FAULT`] because it is a different question**: the machine checks `R` for one and
/// `W` for the other, which is the whole reason `C-014c` is not already answered by
/// `C-014`.
const STORE_FAULT: u64 = 15;
/// Enough steps for any fixture here; none is longer than fifteen instructions.
const BUDGET: u64 = 400;

/// What the first process writes into its own stack, and what the second must never see.
/// Distinctive on purpose: it is searched for byte-by-byte in physical RAM.
const SECRET: u64 = 0x5AFE_C0DE;
/// What the probing process writes into *its* stack, so that the attempt which opens is
/// visibly the prober's own memory and not the other's.
const MINE: u64 = 0x0BAD_F00D;

/// What the writing process stores, and what must never be found inside another process's
/// frames. Distinctive on purpose, for the same reason [`SECRET`] is: it is searched for
/// byte-by-byte in physical RAM.
const STAMP: u64 = 0x0DEF_ACED;

/// Where the secret sits in the first process's stack.
const SECRET_AT: u64 = STACK_TOP - 8;
/// Where the prober's own word sits. **A different offset from [`SECRET_AT`]**, which is
/// the whole design of the aliasing attempt: probing [`SECRET_AT`] then reads a word the
/// prober never wrote, so an isolated run returns zero and a shared one returns the
/// secret.
const MINE_AT: u64 = STACK_TOP - 16;

/// Handle numbers. Their values are the kernel's to choose and no program is told one —
/// that is what makes reading the handle vector the only way to obtain a capability (A4).
const SCREEN: u64 = 7;
const PIPE_A: u64 = 21;
const PIPE_B: u64 = 22;

/// The message that crosses the channel. Multi-byte on purpose: the ABI moves octets.
const MESSAGE: &[u8] = "नमस्ते".as_bytes();

// --- encoders, because a dozen instructions do not need an assembler --------------------

fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    0x13 | rd << 7 | rs1 << 15 | ((imm as u32) & 0xfff) << 20
}
fn lui(rd: u32, imm: u32) -> u32 {
    0x37 | rd << 7 | (imm & 0xf_ffff) << 12
}
fn auipc(rd: u32) -> u32 {
    0x17 | rd << 7
}
/// `ld rd, imm(rs1)` — the instruction every attempt in this file is made of.
fn ld(rd: u32, rs1: u32, imm: i32) -> u32 {
    0x03 | rd << 7 | 0x3 << 12 | rs1 << 15 | ((imm as u32) & 0xfff) << 20
}
/// `sd rs2, imm(rs1)`.
fn sd(rs2: u32, rs1: u32, imm: i32) -> u32 {
    let i = imm as u32;
    0x23 | (i & 0x1f) << 7 | 0x3 << 12 | rs1 << 15 | rs2 << 20 | ((i >> 5) & 0x7f) << 25
}
const ECALL: u32 = 0x0000_0073;

/// `lui`+`addi` for a value that fits in 32 sign-extended bits, with the `addi`'s sign
/// carried back into the `lui` — the standard `li` expansion, written out because a wrong
/// one here would look like a memory bug.
fn li(rd: u32, value: u64) -> [u32; 2] {
    let v = value as u32;
    let hi = v.wrapping_add(0x800) >> 12;
    let lo = (v & 0xfff) as i32;
    let lo = if lo >= 0x800 { lo - 0x1000 } else { lo };
    [lui(rd, hi), addi(rd, rd, lo)]
}

/// An ELF from the project's own writer holding `words` then `tail`, linked at [`APP`].
fn image(words: &[u32], tail: &[u8]) -> Vec<u8> {
    let mut text: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
    text.extend_from_slice(tail);
    write_debuggable_at(&text, &[], &[], 0, &[], APP)
}

/// A machine with the supervisor's one `sret` installed at [`BASE`].
fn machine() -> Machine {
    let mut m = Machine {
        x: [0; 32],
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

/// Writes [`SECRET`] to [`SECRET_AT`] and exits. The stack is the only writable memory an
/// application built by this toolchain has — `sadhana` emits one `PF_R | PF_X` segment —
/// so that is where a secret goes.
fn keeper() -> Vec<u8> {
    let [hi, lo] = li(7, SECRET);
    image(
        &[
            hi,
            lo,
            sd(7, 2, -8),   // sd t2, -8(sp)
            addi(17, 0, 0), // a7 = EXIT
            addi(10, 0, 0), // a0 = 0
            ECALL,
        ],
        &[],
    )
}

/// Writes [`MINE`] to [`MINE_AT`], then **loads eight bytes from `target`** and exits with
/// whatever came back.
///
/// The target is not an immediate: it is eight bytes in the program's own text, found with
/// `auipc`, so one fixture serves every attempt and the program still does not learn its
/// load address. The eight words before the tail make it 8-byte aligned, which `ld`
/// requires of nothing here but which keeps the offset a constant a reader can check.
fn prober(target: u64) -> Vec<u8> {
    let [hi, lo] = li(7, MINE);
    let words = [
        auipc(5), // t0 = the address of this instruction
        hi,
        lo,
        sd(7, 2, -16),  // sd t2, -16(sp)   — the prober's own word
        ld(6, 5, 32),   // t1 = the target, from the tail
        ld(10, 6, 0),   // THE ATTEMPT: a0 = *target
        addi(17, 0, 0), // a7 = EXIT
        ECALL,
    ];
    assert_eq!(words.len() * 4, 32, "the tail offset above is this size");
    image(&words, &target.to_le_bytes())
}

/// **Stores [`STAMP`] at `target`**, then loads the same address back and exits with what
/// it read — `C-014c`'s instrument, and the mirror of [`prober`].
///
/// The read-back is what makes a *successful* store observable: a store returns nothing, so
/// a fixture that only stored would exit with the same status whether the write landed or
/// was quietly dropped, and "quietly dropped" is exactly the failure a permission check
/// that returns instead of faulting would produce. The target travels in the tail for the
/// same reason [`prober`]'s does.
fn writer(target: u64) -> Vec<u8> {
    let [hi, lo] = li(7, STAMP);
    let words = [
        auipc(5), // t0 = the address of this instruction
        hi,
        lo,             // t2 = STAMP
        ld(6, 5, 32),   // t1 = the target, from the tail
        sd(7, 6, 0),    // THE ATTEMPT: *t1 = STAMP
        ld(10, 6, 0),   // a0 = *t1, read back, so a store that landed is visible
        addi(17, 0, 0), // a7 = EXIT
        ECALL,
    ];
    assert_eq!(words.len() * 4, 32, "the tail offset above is this size");
    image(&words, &target.to_le_bytes())
}

/// Sends [`MESSAGE`] — carried in its own text and found with `auipc` — on [`PIPE_A`], and
/// **exits with whatever the call returned in `a0`**.
///
/// The exit status is the send's verdict rather than a fixed 0, so a refusal is observed
/// the way the ABI states it — the program reporting its own result — instead of by the
/// test reading a register after the fact. The `nop` before the final `ecall` is where an
/// `addi a0, x0, 0` would otherwise clobber that verdict, and it is kept as a `nop` so the
/// message's offset stays the constant asserted below.
fn sender() -> Vec<u8> {
    let words = [
        auipc(5),                          // t0 = here
        addi(11, 5, 36),                   // a1 = the message, in this program's text
        addi(10, 0, PIPE_A as i32),        // a0 = the endpoint
        addi(12, 0, MESSAGE.len() as i32), // a2 = the length
        addi(17, 0, SEND as i32),          // a7 = प्रेषणम्
        ECALL,                             // a0 = the verdict, a1 = bytes sent
        addi(17, 0, 0),                    // a7 = EXIT
        addi(0, 0, 0),                     // nop — a0 still holds the verdict
        ECALL,
    ];
    assert_eq!(words.len() * 4, 36, "the message offset above is this size");
    image(&words, MESSAGE)
}

/// Asks the kernel to send eight bytes **from an address this program cannot read** — the
/// confused-deputy attack on the channel.
///
/// The channel is the one thing in this design that touches both address spaces, so it is
/// the one thing that could be talked into reading across the boundary on a process's
/// behalf. The buffer address is a runtime value and travels in the tail, exactly as
/// [`prober`]'s target does.
fn deputy(target: u64) -> Vec<u8> {
    let words = [
        auipc(5),                   // t0 = here
        ld(11, 5, 40),              // a1 = the buffer, from the tail
        addi(10, 0, PIPE_B as i32), // a0 = the endpoint
        addi(12, 0, 8),             // a2 = eight bytes
        addi(17, 0, SEND as i32),   // a7 = प्रेषणम्
        ECALL,                      // a0 = the verdict
        addi(17, 0, 0),             // a7 = EXIT
        addi(0, 0, 0),              // nop — a0 still holds the verdict
        ECALL,
        addi(0, 0, 0), // padding, so the tail below is 8-byte aligned
    ];
    assert_eq!(words.len() * 4, 40, "the tail offset above is this size");
    image(&words, &target.to_le_bytes())
}

/// Receives on [`PIPE_B`] into its own stack, writes what arrived to [`SCREEN`], and
/// **exits with the receive's verdict** — for the same reason [`sender`] does.
fn receiver() -> Vec<u8> {
    image(
        &[
            addi(10, 0, PIPE_B as i32), // a0 = the endpoint
            addi(11, 2, -64),           // a1 = a buffer on its own stack
            addi(12, 0, 64),            // a2 = the capacity offered
            addi(17, 0, RECV as i32),   // a7 = ग्रहणम्
            ECALL,                      // a0 = the verdict, a1 = bytes received
            addi(28, 11, 0),            // t3 = the length, before a1 is reused
            addi(29, 10, 0),            // t4 = the verdict, before a0 is reused
            addi(10, 0, SCREEN as i32),
            addi(11, 2, -64),
            addi(12, 28, 0),
            addi(17, 0, WRITE as i32),
            ECALL,
            addi(17, 0, 0),  // a7 = EXIT
            addi(10, 29, 0), // a0 = the receive's verdict
            ECALL,
        ],
        &[],
    )
}

/// Receives on [`PIPE_B`] into **an address given at run time** and exits with the
/// receive's verdict — the write-side [`deputy`].
///
/// [`receiver`] always names its own stack. This one names whatever is in its tail, which
/// is how the second clause of `C-014c` gets asked: `RECV` is the only path by which the
/// kernel writes bytes on a program's behalf, so it is the only place a message could be
/// made to land somewhere its recipient may not write.
fn taker(buffer: u64) -> Vec<u8> {
    let words = [
        auipc(5),                   // t0 = here
        ld(11, 5, 40),              // a1 = the buffer, from the tail
        addi(10, 0, PIPE_B as i32), // a0 = the endpoint
        addi(12, 0, 64),            // a2 = the capacity offered
        addi(17, 0, RECV as i32),   // a7 = ग्रहणम्
        ECALL,                      // a0 = the verdict
        addi(17, 0, 0),             // a7 = EXIT
        addi(0, 0, 0),              // nop — a0 still holds the verdict
        ECALL,
        addi(0, 0, 0), // padding, so the tail below is 8-byte aligned
    ];
    assert_eq!(words.len() * 4, 40, "the tail offset above is this size");
    image(&words, &buffer.to_le_bytes())
}

// --- the arrangement -------------------------------------------------------------------

/// A kernel holding the keeper as process 0 and `second` as process 1, with the keeper
/// already run to its exit so its secret is in RAM.
///
/// Both processes are granted a surface and one end of one channel, so every test below
/// differs only in the program it loads.
fn two_processes(second: &[u8]) -> (Machine, Kernel) {
    let mut m = machine();
    let mut k = Kernel::new(FREE);
    let c = k.channel();
    k.spawn(
        &mut m,
        &keeper(),
        vec![Surface::writable(SCREEN)],
        vec![Endpoint {
            handle: PIPE_A,
            channel: c,
            may_send: true,
            may_recv: false,
        }],
    )
    .expect("the keeper must load");
    k.spawn(
        &mut m,
        second,
        vec![Surface::writable(SCREEN)],
        vec![Endpoint {
            handle: PIPE_B,
            channel: c,
            may_send: false,
            may_recv: true,
        }],
    )
    .expect("the second program must load");
    let mut uart: Vec<u8> = Vec::new();
    assert_eq!(
        k.run(&mut m, 0, BUDGET, &mut uart),
        Ended::Exited { status: 0 },
        "the keeper writes its secret and exits"
    );
    assert!(
        uart.is_empty(),
        "an application cannot address a device (A4)"
    );
    (m, k)
}

/// Every offset in physical RAM at which `needle` appears.
///
/// The measurement that is not a program's narration: what is in memory, asked of the
/// machine's own RAM rather than of the process that claims to have been refused.
fn offsets_of(m: &Machine, needle: &[u8]) -> Vec<usize> {
    (0..=m.mem.len().saturating_sub(needle.len()))
        .filter(|&i| m.mem[i..i + needle.len()] == *needle)
        .collect()
}

/// Every offset in physical RAM at which [`SECRET`]'s eight bytes appear.
fn secret_offsets(m: &Machine) -> Vec<usize> {
    offsets_of(m, &SECRET.to_le_bytes())
}

// --- the two spaces exist and are disjoint ----------------------------------------------

#[test]
fn two_processes_have_two_root_tables_built_from_disjoint_frames() {
    let (_m, k) = two_processes(&prober(MINE_AT));
    let (a, b) = (&k.processes[0], &k.processes[1]);

    assert_eq!(a.satp >> 60, 8, "process 0 is Sv39");
    assert_eq!(b.satp >> 60, 8, "process 1 is Sv39");
    assert_ne!(
        a.satp, b.satp,
        "one satp for both processes is ONE address space wearing two names, and every \
         denial below would then be a denial of nothing"
    );

    // `C-002l` stops at the line above, and says so: distinct satps prove two root tables
    // and nothing more, since a pair of tables can describe an identical map. THIS is the
    // stronger structural claim, and it is the mechanism the denials rest on — the frames
    // one space was built from are not the frames the other was built from, so the entry
    // naming the other's memory is not in the table at all.
    assert!(
        a.frames.1 <= b.frames.0,
        "the pools must not overlap: process 0 took [{:#x},{:#x}) and process 1 \
         [{:#x},{:#x})",
        a.frames.0,
        a.frames.1,
        b.frames.0,
        b.frames.1
    );
    assert!(a.frames.0 < a.frames.1, "process 0 took at least one frame");
    assert!(b.frames.0 < b.frames.1, "process 1 took at least one frame");
    assert_eq!(
        (a.satp & ((1 << 44) - 1)) << 12,
        a.frames.0,
        "process 0's root table is the first frame of its own pool"
    );
}

#[test]
fn the_secret_is_really_there_so_the_denial_is_a_denial() {
    // The witness that is not the program's narration. `E-010` used the device's
    // available-ring index; the equivalent here is the physical memory of the machine.
    let (m, k) = two_processes(&prober(MINE_AT));
    let at = secret_offsets(&m);
    assert_eq!(
        at.len(),
        1,
        "the secret must be in RAM exactly once — zero times means the keeper never ran \
         and every refusal below is refusing nothing; twice means it is not the keeper's \
         alone and 'which process it belongs to' has no answer: {at:?}"
    );
    let pa = BASE + at[0] as u64;
    let (a, b) = (&k.processes[0], &k.processes[1]);
    assert!(
        pa >= a.frames.0 && pa < a.frames.1,
        "the secret at {pa:#x} is not inside process 0's frames [{:#x},{:#x})",
        a.frames.0,
        a.frames.1
    );
    assert!(
        pa < b.frames.0 || pa >= b.frames.1,
        "the secret at {pa:#x} is inside process 1's frames [{:#x},{:#x}) — the pools \
         overlap and there is no boundary here to prove",
        b.frames.0,
        b.frames.1
    );
}

// --- the negative half, which is the row ------------------------------------------------

/// Run the prober aimed at `target` as process 1 and say what came back.
fn attempt(target: u64) -> Ended {
    let (mut m, mut k) = two_processes(&prober(target));
    let mut uart: Vec<u8> = Vec::new();
    let ended = k.run(&mut m, 1, BUDGET, &mut uart);
    assert!(
        uart.is_empty(),
        "an application cannot address a device (A4)"
    );
    ended
}

#[test]
fn eight_attempts_to_read_the_other_process_and_seven_denials() {
    // The whole row. Eight U-mode loads, each in its own program, each aimed at a named
    // address, every one of them MADE — a denial proved by never asking has proved
    // nothing (`tools/check-powerbox.sh`).
    let (m, k) = two_processes(&prober(MINE_AT));
    let secret_pa = BASE + secret_offsets(&m)[0] as u64;
    let a_root = (k.processes[0].satp & ((1 << 44) - 1)) << 12;

    // ATTEMPT 1 — the prober's own stack word. THE ONE THAT OPENS, and the control: if
    // this did not come back the probe mechanism would be broken and the seven refusals
    // below would be refusals by a program that could not read anything at all.
    assert_eq!(
        attempt(MINE_AT),
        Ended::Exited { status: MINE },
        "attempt 1 must OPEN and return the prober's own word — a probe that cannot read \
         its own memory proves nothing by failing to read somebody else's"
    );

    // ATTEMPT 2 — THE SHARPEST ONE. The exact virtual address at which the other process's
    // secret really is. Not a fault: the address is mapped in BOTH spaces, because both
    // programs got a stack at the same place. What comes back is the prober's OWN frame,
    // which it never wrote, so it is zero. This is the denial a shared `satp` breaks, and
    // it is the assertion the mutation test fails on.
    assert_eq!(
        attempt(SECRET_AT),
        Ended::Exited { status: 0 },
        "attempt 2: {SECRET_AT:#x} holds {SECRET:#x} in process 0 and nothing in process \
         1. A prober that returned {SECRET:#x} here read the other process's memory, and \
         the two 'address spaces' are one"
    );

    // ATTEMPTS 3-8 — six addresses that are not in the prober's page table at all. Each is
    // a load page fault naming the address it was aimed at, which is the machine refusing
    // rather than this test asserting.
    let denied = [
        ("3: the secret's own physical address", secret_pa),
        ("4: the other process's root page table", a_root),
        ("5: the supervisor's gigapage", BASE),
        ("6: the other process's first frame", FREE),
        ("7: the guard page above its own stack", STACK_TOP),
        (
            "8: an unmapped hole in its own space",
            HANDLE_VECTOR + 0x1000,
        ),
    ];
    for (name, target) in denied {
        assert_eq!(
            attempt(target),
            Ended::Faulted {
                cause: LOAD_FAULT,
                tval: target,
                epc: APP + 20,
            },
            "attempt {name} at {target:#x} must be a load page fault naming that address"
        );
    }

    // And the sentence the row asks for, in the form E-010 put it: eight attempts, one
    // reached what it was entitled to, seven did not reach what they were aimed at.
    // Nothing in this test asserts an absence — every line above is an answer the machine
    // gave to a load it really executed.
}

#[test]
fn every_frame_the_other_process_was_built_from_is_refused_as_an_address() {
    // The sweep. Not one address chosen to fail, but every physical page process 0's
    // address space was built out of, offered to process 1 as a virtual address. A
    // boundary that held for six hand-picked addresses and leaked on the seventh would
    // show up here.
    let (_m, k) = two_processes(&prober(MINE_AT));
    let (start, end) = k.processes[0].frames;
    let mut refused = 0;
    let mut page = start;
    while page < end {
        assert!(
            matches!(
                attempt(page),
                Ended::Faulted {
                    cause: LOAD_FAULT,
                    ..
                }
            ),
            "process 0's frame at {page:#x} was reachable from process 1"
        );
        refused += 1;
        page += 4096;
    }
    assert!(
        refused >= 8,
        "a sweep of {refused} frame(s) is too few to be a sweep — process 0's pool is \
         [{start:#x},{end:#x})"
    );
}

// --- `C-014c`: the write half ------------------------------------------------------------

/// Run the writer aimed at `target` as process 1, and hand back the verdict **and the
/// machine**, because for a store the evidence that matters is in RAM rather than in a
/// status: a store that landed where it should not have is invisible to the program that
/// made it.
fn store_attempt_on(target: u64) -> (Ended, Machine, Kernel) {
    let (mut m, mut k) = two_processes(&writer(target));
    let mut uart: Vec<u8> = Vec::new();
    let ended = k.run(&mut m, 1, BUDGET, &mut uart);
    assert!(
        uart.is_empty(),
        "an application cannot address a device (A4)"
    );
    (ended, m, k)
}

/// The verdict alone, for the attempts whose whole answer is a fault.
fn store_attempt(target: u64) -> Ended {
    store_attempt_on(target).0
}

#[test]
fn nine_attempts_to_write_across_the_boundary_and_seven_denials() {
    // `C-014`'s eight loads, repeated as stores, plus the two addresses a load says nothing
    // about. Every attempt is a real U-mode `sd` that executed — a denial proved by never
    // storing has proved nothing, exactly as `tools/check-powerbox.sh` says of reads.
    let (m, k) = two_processes(&writer(MINE_AT));
    let secret_pa = BASE + secret_offsets(&m)[0] as u64;
    let a_root = (k.processes[0].satp & ((1 << 44) - 1)) << 12;

    // ATTEMPT 1 — its own stack. THE ONE THAT OPENS, and the control: a writer that cannot
    // write its own memory refuses nothing by failing to write somebody else's.
    assert_eq!(
        store_attempt(MINE_AT),
        Ended::Exited { status: STAMP },
        "attempt 1 must OPEN: the store landed in the writer's own stack and the load back \
         returned it"
    );

    // ATTEMPTS 2-7 — six addresses that are not in the writer's page table at all. Each is
    // a STORE page fault, cause 15, naming the address, at the `sd` and not at the load
    // after it.
    let unmapped = [
        ("2: the secret's own physical address", secret_pa),
        ("3: the other process's root page table", a_root),
        ("4: the supervisor's gigapage", BASE),
        ("5: the other process's first frame", FREE),
        ("6: the guard page above its own stack", STACK_TOP),
        (
            "7: an unmapped hole in its own space",
            HANDLE_VECTOR + 0x1000,
        ),
    ];
    for (name, target) in unmapped {
        assert_eq!(
            store_attempt(target),
            Ended::Faulted {
                cause: STORE_FAULT,
                tval: target,
                epc: APP + 16,
            },
            "attempt {name} at {target:#x} must be a store page fault naming that address"
        );
    }

    // ATTEMPTS 8 AND 9 — THE TWO THIS TEST EXISTS FOR. Everything above is refused by
    // validity: the address is in no leaf of the writer's table, so a machine that ignored
    // `W` entirely would still fault on all six and this file would still be green. These
    // two are addresses the writer CAN READ and must not write. They separate "not mapped"
    // from "not writable", and they are the only attempts here that do.
    assert_eq!(
        attempt(HANDLE_VECTOR),
        Ended::Exited { status: SCREEN },
        "attempt 8, first half: the handle vector is READABLE — it answers with the first \
         handle this process was granted. A fault here would make the store fault below \
         prove nothing about W"
    );
    assert_eq!(
        store_attempt(HANDLE_VECTOR),
        Ended::Faulted {
            cause: STORE_FAULT,
            tval: HANDLE_VECTOR,
            epc: APP + 16,
        },
        "attempt 8: the loader maps the handle vector `V R U A` and never W (A4). A \
         process that could write it could forge itself a capability"
    );
    assert!(
        matches!(attempt(APP), Ended::Exited { status } if status != 0),
        "attempt 9, first half: a program's own text is readable — every fixture in this \
         file loads a word out of it"
    );
    assert_eq!(
        store_attempt(APP),
        Ended::Faulted {
            cause: STORE_FAULT,
            tval: APP,
            epc: APP + 16,
        },
        "attempt 9: the text segment is `R X U` and not W, so a program cannot rewrite its \
         own instructions"
    );
}

#[test]
fn writing_its_own_stack_leaves_the_other_process_s_secret_where_it_was() {
    // The sharpest store, and the one that is ALLOWED. `SECRET_AT` is mapped in both
    // spaces, because both programs were given a stack at the same virtual address. The
    // writer stores there and the store succeeds — into ITS OWN frame. The witness is RAM:
    // the secret is still there, exactly once, and the stamp is inside the writer's frames
    // and nowhere else.
    //
    // A shared `satp` turns this store into a store through the peer's page table, and the
    // keeper's secret is overwritten — which is a corruption no read test could ever see,
    // and the reason the write half is a separate row.
    let (ended, m, k) = store_attempt_on(SECRET_AT);
    assert_eq!(
        ended,
        Ended::Exited { status: STAMP },
        "the store into its own stack must land, or the two assertions below hold \
         vacuously for a program that never wrote anything"
    );
    let (a, b) = (&k.processes[0], &k.processes[1]);

    let secret = secret_offsets(&m);
    assert_eq!(
        secret.len(),
        1,
        "the keeper's secret must still be in RAM exactly once after the peer stored at \
         the very address it lives at: {secret:?}"
    );
    let secret_pa = BASE + secret[0] as u64;
    assert!(
        secret_pa >= a.frames.0 && secret_pa < a.frames.1,
        "the surviving secret at {secret_pa:#x} must be the keeper's own, in [{:#x},{:#x})",
        a.frames.0,
        a.frames.1
    );

    let stamped = offsets_of(&m, &STAMP.to_le_bytes());
    assert!(
        !stamped.is_empty(),
        "the stamp must be somewhere in RAM — a store that reached no memory at all is a \
         denial wearing a success's clothes"
    );
    for offset in &stamped {
        let pa = BASE + *offset as u64;
        assert!(
            pa >= b.frames.0 && pa < b.frames.1,
            "the writer's stamp reached {pa:#x}, outside its own frames [{:#x},{:#x}) — a \
             store crossed the boundary",
            b.frames.0,
            b.frames.1
        );
    }
}

#[test]
fn every_frame_the_other_process_was_built_from_is_refused_as_a_store() {
    // The sweep, as a store. `C-014`'s sweep offered every one of process 0's physical
    // frames to process 1 as an address to LOAD from; this offers the same frames as
    // addresses to STORE to, and a boundary that refused every read while accepting a write
    // on one frame would show up here and nowhere else in this file.
    let (_m, k) = two_processes(&writer(MINE_AT));
    let (start, end) = k.processes[0].frames;
    let mut refused = 0;
    let mut page = start;
    while page < end {
        assert!(
            matches!(
                store_attempt(page),
                Ended::Faulted {
                    cause: STORE_FAULT,
                    ..
                }
            ),
            "process 0's frame at {page:#x} was writable from process 1"
        );
        refused += 1;
        page += 4096;
    }
    assert!(
        refused >= 8,
        "a sweep of {refused} frame(s) is too few to be a sweep — process 0's pool is \
         [{start:#x},{end:#x})"
    );
}

#[test]
fn the_channel_cannot_be_talked_into_reading_across_the_boundary() {
    // The ninth attempt, and the only one that does not use a load instruction. Everything
    // above asks the MACHINE for the other process's memory and is refused by the page
    // table. This asks the KERNEL for it — the one component that can reach both spaces —
    // by naming the other process's physical address as a send buffer.
    //
    // `crates/yantra/src/supervisor.rs` already had the argument in its header, about a
    // pointer into the supervisor's own gigapage: reading the buffer with `sstatus.SUM`
    // set asks *may S-mode read this?*, which is the wrong question. `send` therefore
    // reads the buffer with `read_as_the_program`, which requires `U` at the leaf of the
    // CALLER's table. A process's own page table is the only thing that decides what it
    // can put on a channel.
    let (m, _) = two_processes(&prober(MINE_AT));
    let secret_pa = BASE + secret_offsets(&m)[0] as u64;

    let mut m = machine();
    let mut k = Kernel::new(FREE);
    let c = k.channel();
    k.spawn(&mut m, &keeper(), vec![], vec![])
        .expect("the keeper must load");
    k.spawn(
        &mut m,
        &deputy(secret_pa),
        vec![],
        // Granted प्रेषणम् on purpose: the refusal below must be about the BUFFER and not
        // about the grant, or it would prove nothing about the boundary.
        vec![Endpoint {
            handle: PIPE_B,
            channel: c,
            may_send: true,
            may_recv: true,
        }],
    )
    .expect("the deputy must load");

    let mut uart: Vec<u8> = Vec::new();
    k.run(&mut m, 0, BUDGET, &mut uart);
    assert_eq!(
        k.run(&mut m, 1, BUDGET, &mut uart),
        Ended::Exited {
            status: RIKTAPRAVESHAH as u64
        },
        "रिक्तप्रवेशः — {secret_pa:#x} is not readable BY THIS PROGRAM, so the kernel \
         will not read it on the program's behalf. A सिद्धम् here is the confused deputy: \
         the channel would have carried the other process's memory out for it"
    );
    assert!(
        k.channels[c].messages.is_empty(),
        "and nothing was queued — a refused send must not have sent"
    );
    for message in &k.channels[c].messages {
        assert!(
            !message.windows(8).any(|w| w == SECRET.to_le_bytes()),
            "the secret crossed the channel"
        );
    }
}

// --- the channel, which is the other half of the acceptance -----------------------------

#[test]
fn a_message_crosses_the_channel_and_nothing_else_does() {
    let mut m = machine();
    let mut k = Kernel::new(FREE);
    let c = k.channel();
    k.spawn(
        &mut m,
        &sender(),
        vec![],
        vec![Endpoint {
            handle: PIPE_A,
            channel: c,
            may_send: true,
            may_recv: false,
        }],
    )
    .expect("the sender must load");
    k.spawn(
        &mut m,
        &receiver(),
        vec![Surface::writable(SCREEN)],
        vec![Endpoint {
            handle: PIPE_B,
            channel: c,
            may_send: false,
            may_recv: true,
        }],
    )
    .expect("the receiver must load");

    let mut uart: Vec<u8> = Vec::new();
    assert_eq!(
        k.run(&mut m, 0, BUDGET, &mut uart),
        Ended::Exited { status: 0 },
        "the sender sends and exits"
    );
    assert_eq!(
        k.channels[c].messages,
        vec![MESSAGE.to_vec()],
        "the message is in the KERNEL between the send and the receive — not in a page \
         either process can reach, which is why the channel does not weaken the boundary"
    );
    assert_eq!(
        k.run(&mut m, 1, BUDGET, &mut uart),
        Ended::Exited { status: 0 },
        "the receiver receives and exits"
    );
    assert_eq!(
        k.processes[1].surface(SCREEN).expect("granted").bytes,
        MESSAGE,
        "the sender's bytes reached the receiver's surface, having crossed only through \
         the kernel"
    );
    assert!(
        k.channels[c].messages.is_empty(),
        "a received message is consumed"
    );
    assert!(uart.is_empty(), "neither program can address a device (A4)");
}

/// A sender that has already sent [`MESSAGE`] as process 0, and a [`taker`] aimed at
/// `buffer` waiting as process 1. The channel index comes back with them.
fn sender_and_taker(buffer: u64) -> (Machine, Kernel, usize) {
    let mut m = machine();
    let mut k = Kernel::new(FREE);
    let c = k.channel();
    k.spawn(
        &mut m,
        &sender(),
        vec![],
        vec![Endpoint {
            handle: PIPE_A,
            channel: c,
            may_send: true,
            may_recv: false,
        }],
    )
    .expect("the sender must load");
    k.spawn(
        &mut m,
        &taker(buffer),
        vec![],
        // Granted ग्रहणम् on purpose: the refusals below must be about the BUFFER and not
        // about the grant, or they would prove nothing about where bytes can be put.
        vec![Endpoint {
            handle: PIPE_B,
            channel: c,
            may_send: false,
            may_recv: true,
        }],
    )
    .expect("the taker must load");
    let mut uart: Vec<u8> = Vec::new();
    assert_eq!(
        k.run(&mut m, 0, BUDGET, &mut uart),
        Ended::Exited { status: 0 },
        "the sender sends and exits"
    );
    assert_eq!(
        k.channels[c].messages,
        vec![MESSAGE.to_vec()],
        "the message waits in the kernel, so there is really something to be placed"
    );
    (m, k, c)
}

#[test]
fn the_channel_will_not_place_a_message_where_the_receiver_may_not_write() {
    // `C-014c`'s second clause, and the mirror of the confused-deputy test above. That one
    // asked whether the kernel can be made to READ a program's way across the boundary;
    // this asks whether it can be made to WRITE across it — the same deputy, the other
    // direction, and a separate function (`write_as_the_program`) doing the deciding.
    //
    // The address a message lands at is never the sender's: it is the receiver's `a1`,
    // translated under the RECEIVER's table with `W` and `U` required. So these four
    // pointers are aimed by the receiver, and each of them is a place a message must not
    // be able to reach.
    let attempts = [
        ("the supervisor's own text", BASE),
        ("the sender's first frame", FREE),
        ("its own handle vector, readable and never W", HANDLE_VECTOR),
        ("its own text, R X and never W", APP),
    ];
    for (name, buffer) in attempts {
        let (mut m, mut k, c) = sender_and_taker(buffer);
        let before = offsets_of(&m, MESSAGE);
        assert!(
            !before.is_empty(),
            "{name}: the message must already be in RAM inside the sender's text, or \
             'no new copy' below is satisfied by a message that never existed"
        );
        let mut uart: Vec<u8> = Vec::new();
        assert_eq!(
            k.run(&mut m, 1, BUDGET, &mut uart),
            Ended::Exited {
                status: RIKTAPRAVESHAH as u64
            },
            "रिक्तप्रवेशः — {buffer:#x} ({name}) is not writable BY THIS PROGRAM, so the \
             kernel will not write it on the program's behalf. A सिद्धम् here is the \
             confused deputy in reverse: the channel would have placed a sender's bytes \
             where the receiver could not have put them itself"
        );
        assert_eq!(
            k.channels[c].messages,
            vec![MESSAGE.to_vec()],
            "{name}: a refused receive must not consume the message — the program has no \
             way to ask for it again"
        );
        assert_eq!(
            offsets_of(&m, MESSAGE),
            before,
            "{name}: RAM must hold the message in exactly the places it held it before. A \
             new copy anywhere is bytes placed at an address chosen by a program that may \
             not write there"
        );
    }
}

#[test]
fn a_message_lands_only_where_the_receiver_named_it() {
    // The positive half of the same clause, and the one that says the placement is the
    // RECEIVER's choice rather than the sender's. The sender named a source and a length
    // and nothing else; the bytes end up at the one address the receiver asked for, and
    // there is exactly one new copy of them in the whole machine.
    let buffer = STACK_TOP - 64;
    let (mut m, mut k, c) = sender_and_taker(buffer);
    let before = offsets_of(&m, MESSAGE);
    let (a, b) = (k.processes[0].frames, k.processes[1].frames);
    for offset in &before {
        let pa = BASE + *offset as u64;
        assert!(
            pa >= a.0 && pa < a.1,
            "before the receive the message is the sender's alone, and this copy is at \
             {pa:#x}, outside [{:#x},{:#x})",
            a.0,
            a.1
        );
    }

    let mut uart: Vec<u8> = Vec::new();
    assert_eq!(
        k.run(&mut m, 1, BUDGET, &mut uart),
        Ended::Exited {
            status: SIDDHAM as u64
        },
        "सिद्धम् — the receiver named its own stack and the message was placed there"
    );
    assert!(
        k.channels[c].messages.is_empty(),
        "a received message is consumed"
    );

    let after = offsets_of(&m, MESSAGE);
    let fresh: Vec<usize> = after
        .iter()
        .copied()
        .filter(|o| !before.contains(o))
        .collect();
    assert_eq!(
        fresh.len(),
        1,
        "exactly one new copy of the message: none means the receive wrote nothing and \
         its सिद्धम् was a lie, more than one means the kernel scattered the bytes \
         somewhere besides the buffer it was given. before={before:?} after={after:?}"
    );
    let pa = BASE + fresh[0] as u64;
    assert!(
        pa >= b.0 && pa < b.1,
        "the new copy is at {pa:#x}, outside the receiver's own frames [{:#x},{:#x}) — the \
         message was placed somewhere that is not the receiver's memory",
        b.0,
        b.1
    );
    assert!(
        pa < a.0 || pa >= a.1,
        "the new copy is at {pa:#x}, inside the SENDER's frames [{:#x},{:#x}), so the \
         receive wrote into the peer",
        a.0,
        a.1
    );
    assert!(uart.is_empty(), "neither program can address a device (A4)");
}

#[test]
fn an_endpoint_is_a_capability_and_the_refusals_are_the_table_s() {
    // The channel is granted the same way a surface is, and refuses the same way. Without
    // this the permission fields would be decoration no run ever reaches.
    let mut m = machine();
    let mut k = Kernel::new(FREE);
    let c = k.channel();
    // A receive-only endpoint, asked to send.
    k.spawn(
        &mut m,
        &sender(),
        vec![],
        vec![Endpoint {
            handle: PIPE_A,
            channel: c,
            may_send: false,
            may_recv: true,
        }],
    )
    .expect("load");
    let mut uart: Vec<u8> = Vec::new();
    assert_eq!(
        k.run(&mut m, 0, BUDGET, &mut uart),
        Ended::Exited {
            status: ADHIKARABHAVAH as u64
        },
        "अधिकाराभावः — the grant does not carry प्रेषणम्. A different sentence from 'no \
         such handle', and it has to stay one"
    );
    assert!(
        k.channels[c].messages.is_empty(),
        "a refused send must not have sent"
    );

    // A handle that was never granted at all.
    let mut m = machine();
    let mut k = Kernel::new(FREE);
    k.spawn(&mut m, &sender(), vec![], vec![]).expect("load");
    assert_eq!(
        k.run(&mut m, 0, BUDGET, &mut uart),
        Ended::Exited {
            status: SIMATIKRAMAH as u64
        },
        "सीमातिक्रमः — a program cannot name an endpoint it was not handed, and guessing \
         the number does not help it"
    );

    // An empty channel is रिक्तः, which is neither success nor failure.
    let mut m = machine();
    let mut k = Kernel::new(FREE);
    let c = k.channel();
    k.spawn(
        &mut m,
        &receiver(),
        vec![Surface::writable(SCREEN)],
        vec![Endpoint {
            handle: PIPE_B,
            channel: c,
            may_send: false,
            may_recv: true,
        }],
    )
    .expect("load");
    assert_eq!(
        k.run(&mut m, 0, BUDGET, &mut uart),
        Ended::Exited {
            status: RIKTAH as u64
        },
        "रिक्तः — nothing has been sent. A सिद्धम् of zero bytes here would be \
         indistinguishable from a zero-length message that really was sent"
    );
    assert_eq!(
        k.processes[0].surface(SCREEN).expect("granted").bytes,
        Vec::<u8>::new(),
        "and nothing was written"
    );
}

#[test]
fn a_process_that_exited_is_not_run_again_and_the_other_still_runs() {
    // ADR-0015 A5, now with two of them: ending one process must not end the machine, the
    // kernel, or the other process. The keeper has already exited inside `two_processes`.
    let (mut m, mut k) = two_processes(&prober(MINE_AT));
    let mut uart: Vec<u8> = Vec::new();
    assert_eq!(
        k.run(&mut m, 0, BUDGET, &mut uart),
        Ended::Exited { status: 0 },
        "a process that exited reports its exit again rather than re-entering a program \
         that asked to end"
    );
    assert_eq!(
        k.run(&mut m, 1, BUDGET, &mut uart),
        Ended::Exited { status: MINE },
        "and the other process runs afterwards, in its own space"
    );
}
