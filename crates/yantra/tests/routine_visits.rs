//! **HOW MANY TIMES, NOT JUST HOW LONG — and the count the inner loop must not
//! inflate.**
//!
//! `W-306` localised `shrinkhala.t1`'s run to `खण्डवृद्धिः`: 282,302 steps of
//! 438,794 in the nine-tenths window, under one name. That reading cannot say
//! WHICH FINDING it is. A routine holding 282,302 steps ran either once and
//! spun — a loop to go fix — or thousands of times and did a few dozen steps
//! each, which is the work the source asked for and a question about
//! `DEFAULT_STEPS` instead. Steps alone spell those identically.
//!
//! [`yantra::profile::StepProfile::visit`] adds the reading that separates
//! them: ENTRIES, the count at the routine's first address, beside the steps.
//!
//! # The case that must still be refused
//!
//! [`a_routines_entries_are_its_calls_and_not_its_inner_loops_trips`] is a
//! callee entered THREE times that spins FOUR on each entry. Three numbers are
//! available inside its extent and only one is right: the entry count is `3`,
//! the busiest site in the range carries `12`, and the steps sum to `30`. An
//! implementation reaching for the maximum, or for the sum, passes every
//! assertion about steps and fails here — which is the point, because both are
//! the obvious way to write it and both would report the hot callee as a hot
//! loop.
//!
//! [`a_routine_whose_first_address_the_run_never_reached_reads_no_ratio`] is
//! the third state. `routines` infers an extent as reaching up to the NEXT
//! NAME, so an unnamed neighbour is annexed silently; when that happens the
//! steps are real and the entries are zero, and `steps_per_entry` answers
//! `None` rather than dividing by it or dropping the row.
//!
//! # Its own loader, deliberately
//!
//! As in `step_profile.rs` and `span_owner.rs`: the machine here is this
//! file's, and the word addresses in these assertions are what its placement
//! (text at [`BASE`], nothing else) means.

use yantra::profile::{Routine, routines, run_profiled};
use yantra::{FINISHER, Halt, Machine, Privilege};

const BASE: u64 = 0x8000_0000;

/// The address of word `n` of the text.
fn w(n: u64) -> u64 {
    BASE + n * 4
}

/// A machine with `text` at [`BASE`] and nothing else.
fn machine(text: &[u32]) -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX, // W-363: no store bound beyond `mem` — this machine has no injected input above it
        // A machine nobody asked to serve files must not be able to.
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
        csr: yantra::Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
        socket: None,
    };
    for (i, word) in text.iter().enumerate() {
        m.mem[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    m
}

// Encoders written out rather than taken from `sadhana::encode`: a test that
// derived its words from the same place the interpreter does would agree with
// it while both disagreed with the ISA.
fn i_type(op: u32, rd: u32, f3: u32, rs1: u32, imm: i32) -> u32 {
    op | rd << 7 | f3 << 12 | rs1 << 15 | ((imm as u32) & 0xfff) << 20
}
fn s_type(op: u32, f3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    op | (imm & 0x1f) << 7 | f3 << 12 | rs1 << 15 | rs2 << 20 | (imm >> 5 & 0x7f) << 25
}
fn b_type(f3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    0x63 | (imm >> 11 & 1) << 7
        | (imm >> 1 & 0xf) << 8
        | f3 << 12
        | rs1 << 15
        | rs2 << 20
        | (imm >> 5 & 0x3f) << 25
        | (imm >> 12 & 1) << 31
}
fn u_type(op: u32, rd: u32, imm: u32) -> u32 {
    op | rd << 7 | (imm & 0xffff_f000)
}
fn j_type(rd: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    0x6f | rd << 7
        | (imm >> 12 & 0xff) << 12
        | (imm >> 11 & 1) << 20
        | (imm >> 1 & 0x3ff) << 21
        | (imm >> 20 & 1) << 31
}
/// `addi rd, rs1, imm`.
fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x13, rd, 0x0, rs1, imm)
}
/// `bne rs1, rs2, off`.
fn bne(rs1: u32, rs2: u32, off: i32) -> u32 {
    b_type(0x1, rs1, rs2, off)
}
/// `jal rd, off`.
fn jal(rd: u32, off: i32) -> u32 {
    j_type(rd, off)
}
/// `jalr x0, rs1, 0` — return through `rs1`.
fn ret(rs1: u32) -> u32 {
    i_type(0x67, 0, 0x0, rs1, 0)
}
/// The four words that write `0x5555` to the finisher and stop the machine.
/// Every program here ends at a `Finisher` halt rather than at the budget: a
/// profile of a run that hit the step limit is a profile of the limit.
fn finish(addr_reg: u32, val_reg: u32) -> [u32; 4] {
    [
        u_type(0x37, addr_reg, FINISHER as u32),
        u_type(0x37, val_reg, 0x5000),
        addi(val_reg, val_reg, 0x555),
        s_type(0x23, 0x2, addr_reg, val_reg, 0),
    ]
}

/// A caller that calls `callee` THREE times; `callee` spins FOUR on each entry;
/// a `dead` routine nobody calls; and one PAD word at `w(8)` the run can never
/// reach, since the finisher store at `w(7)` stops the machine.
///
/// ```text
///  w0  addi x5, x0, 3      outer count
///  w1  addi x5, x5, -1     LOOPTOP
///  w2  jal  x1, callee
///  w3  bne  x5, x0, LOOPTOP
///  w4..w7  finish          halts here
///  w8  addi x0, x0, 0      PAD — unreachable
///  w9  addi x7, x0, 4      callee: ENTRY
///  w10 addi x7, x7, -1     INNER
///  w11 bne  x7, x0, INNER
///  w12 jalr x0, x1, 0
///  w13 addi x0, x0, 0      dead: never called
///  w14 addi x0, x0, 0
/// ```
fn caller_callee_dead() -> Vec<u32> {
    let f = finish(10, 11);
    vec![
        addi(5, 0, 3),
        addi(5, 5, -1),
        jal(1, 7 * 4),
        bne(5, 0, -2 * 4),
        f[0],
        f[1],
        f[2],
        f[3],
        addi(0, 0, 0),
        addi(7, 0, 4),
        addi(7, 7, -1),
        bne(7, 0, -4),
        ret(1),
        addi(0, 0, 0),
        addi(0, 0, 0),
    ]
}

/// The symbol table for [`caller_callee_dead`], as the image would carry it.
fn symbols() -> Vec<(String, u64)> {
    vec![
        ("caller".to_string(), w(0)),
        ("callee".to_string(), w(9)),
        ("dead".to_string(), w(13)),
    ]
}

fn run() -> (Vec<Routine>, yantra::profile::StepProfile) {
    let text = caller_callee_dead();
    let mut m = machine(&text);
    let mut out = Vec::new();
    let (halt, p) = run_profiled(&mut m, 10_000, &mut out);
    // A budget halt would make every count below a count of the limit.
    assert!(
        matches!(
            halt,
            Halt::Finisher {
                status: Some(0),
                ..
            }
        ),
        "the fixture must reach its finisher, not the budget: {halt:?}"
    );
    let hi = w(text.len() as u64) - 1;
    (routines(&symbols(), BASE, hi), p)
}

fn named<'a>(rs: &'a [Routine], name: &str) -> &'a Routine {
    rs.iter()
        .find(|r| r.name == name)
        .unwrap_or_else(|| panic!("no routine {name} in {rs:?}"))
}

#[test]
fn a_routines_entries_are_its_calls_and_not_its_inner_loops_trips() {
    let (rs, p) = run();
    let callee = named(&rs, "callee");
    let v = p.visit(callee);

    // THE THREE NUMBERS AVAILABLE INSIDE THE EXTENT, spelled out so the two
    // wrong ones are on the page beside the right one. Entered 3 times; 10
    // steps per entry (`w9` once, `w10` and `w11` four times each, `w12`
    // once); so 30 steps, and the busiest site in the range carries 12.
    assert_eq!(v.entries, 3, "entries is the count at the routine's own lo");
    assert_eq!(v.steps, 30, "steps is everything inside the extent");
    assert_eq!(v.steps_per_entry(), Some(10));

    let busiest_site = (callee.lo..=callee.hi)
        .step_by(4)
        .map(|pc| p.at(pc))
        .max()
        .expect("the extent holds at least one word");
    assert_eq!(busiest_site, 12, "the inner loop's own site, over 3 calls");
    // THE REFUSAL. An implementation taking the maximum count in the range
    // would answer 12 and one taking the sum would answer 30, and both would
    // report this hot CALLEE as a hot LOOP — the exact confusion `visit`
    // exists to end. Neither may equal the entry count.
    assert_ne!(v.entries, busiest_site);
    assert_ne!(v.entries, v.steps);

    // And the caller, for contrast: entered ONCE, and every step it holds is
    // its own — the pad at `w(8)` its inferred extent annexes never ran.
    let caller = p.visit(named(&rs, "caller"));
    assert_eq!(caller.entries, 1);
    assert_eq!(caller.steps, 14, "1 + 3 + 3 + 3 outside, 4 in the finisher");
    assert_eq!(caller.steps_per_entry(), Some(14));
}

#[test]
fn a_routine_whose_first_address_the_run_never_reached_reads_no_ratio() {
    let (_rs, p) = run();
    // `routines` reads an extent as reaching up to the next NAME, so an
    // unnamed stretch in front of a routine is annexed to it silently. This is
    // that shape built on purpose: a name on the unreachable PAD at `w(8)`,
    // covering the whole of `callee`.
    let annexed = Routine {
        name: "annexed".to_string(),
        lo: w(8),
        hi: w(12),
    };
    let v = p.visit(&annexed);
    assert_eq!(v.steps, 30, "the annexed body's steps are real");
    assert_eq!(v.sites, 4, "and it reached four of them");
    assert_eq!(v.entries, 0, "but the run never arrived at w(8)");
    // NOT `Some(0)`, and NOT a dropped row. Steps without entries is the state
    // that says the extent is wrong, and it only says so if it can be seen.
    assert_eq!(v.steps_per_entry(), None);
}

#[test]
fn a_routine_the_run_never_entered_at_all_is_not_among_the_busiest() {
    let (rs, p) = run();
    let dead = p.visit(named(&rs, "dead"));
    assert_eq!((dead.entries, dead.steps, dead.sites), (0, 0, 0));

    let busiest = p.busiest(&rs, 10);
    let names: Vec<&str> = busiest.iter().map(|v| v.name.as_str()).collect();
    // Steps order, and `dead` dropped — reached nothing, so it is not part of
    // the spread. `callee` leads on steps (30) over `caller` (14).
    assert_eq!(names, ["callee", "caller"]);
    assert_eq!(busiest[0].entries, 3);
    assert_eq!(busiest[1].entries, 1);
    // The whole run is accounted for by the two that ran.
    assert_eq!(
        busiest.iter().map(|v| v.steps).sum::<u64>(),
        p.steps(),
        "44 steps, all of them inside a named routine that ran"
    );
}

#[test]
fn the_busiest_list_is_truncated_to_what_was_asked_for() {
    let (rs, p) = run();
    assert_eq!(p.busiest(&rs, 1).len(), 1, "the hottest, and only it");
    assert_eq!(p.busiest(&rs, 1)[0].name, "callee");
    assert!(p.busiest(&rs, 0).is_empty());
    assert_eq!(p.busiest(&[], 5).len(), 0, "no routines, no rows");
}
