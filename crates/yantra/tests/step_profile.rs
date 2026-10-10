//! **THE STEP DISTRIBUTION, AND THE CASE IT MUST REFUSE TO CALL A HOT LOOP.**
//!
//! `yantra::profile::run_profiled` exists to answer the one question the census's
//! `METRIC t1_run_steps` cannot: `shrinkhala.t1` spends 487,371 of a million steps
//! — is that one loop or a long program? The instrument is only worth its output
//! if it says NO to the second shape, so the refusal is here beside the finding:
//! [`straight_line_work_is_not_read_as_one_hot_loop`] is a program with no loop at
//! all, and the narrowest window holding half its steps must come back HALF ITS
//! TEXT wide. An instrument that answered "narrow" there would report every
//! program as a hot loop and could never be wrong.
//!
//! # Its own loader, deliberately
//!
//! The machine built here is this file's, not another test's helper: the placement
//! (text at `base`, registers zeroed, no file window) is part of what the
//! assertions mean, and a shared loader that grew a field would change this test
//! without anyone reading it.

use yantra::profile::{Span, run_profiled};
use yantra::{FINISHER, Halt, Machine, Privilege};

const BASE: u64 = 0x8000_0000;

/// A machine with `text` at [`BASE`] and nothing else.
fn machine(text: &[u32]) -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX, // W-363: no store bound beyond `mem` — this machine has no injected input above it
        // A machine nobody asked to serve files must not be able to.
        patra_root: None,
        patra_mem: None,
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
    for (i, w) in text.iter().enumerate() {
        m.mem[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    m
}

// Encoders written out rather than taken from `sadhana::encode`: a test that
// derived its words from the same place the interpreter does would agree with it
// while both disagreed with the ISA.
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

/// `addi rd, rs1, imm`.
fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    i_type(0x13, rd, 0x0, rs1, imm)
}
/// `bne rs1, rs2, off`.
fn bne(rs1: u32, rs2: u32, off: i32) -> u32 {
    b_type(0x1, rs1, rs2, off)
}

/// The three words that write `0x5555` to the finisher and stop the machine, given
/// two scratch registers. Appended to every program here so each run ends at a
/// `Finisher` halt rather than at the budget — a profile of a run that hit the
/// step limit is a profile of the limit.
fn finish(addr_reg: u32, val_reg: u32) -> [u32; 4] {
    [
        u_type(0x37, addr_reg, FINISHER as u32),
        u_type(0x37, val_reg, 0x5000),
        addi(val_reg, val_reg, 0x555),
        s_type(0x23, 0x2, addr_reg, val_reg, 0),
    ]
}

#[test]
fn the_profile_accounts_for_every_step_the_clock_counted() {
    // The two instruments must agree or one of them is broken, and the clock is
    // the one the census already reads (`row.steps = m.time`). A profile whose
    // total drifted from it would attribute a real run's steps to nothing.
    let mut text = vec![addi(1, 0, 7), addi(2, 0, 9), addi(3, 1, 1)];
    text.extend(finish(4, 5));
    let mut m = machine(&text);
    let mut out = Vec::new();
    let (halt, p) = run_profiled(&mut m, 100, &mut out);

    assert!(
        matches!(
            halt,
            Halt::Finisher {
                status: Some(0),
                ..
            }
        ),
        "the fixture must reach the finisher, not the budget: {halt:?}"
    );
    assert_eq!(p.steps(), m.time, "the profile's total is the hart's clock");
    assert_eq!(
        p.steps(),
        text.len() as u64,
        "every word runs exactly once here"
    );
    assert_eq!(
        p.counts_sum(),
        p.steps(),
        "the per-site counts must sum to the total"
    );
}

#[test]
fn an_address_the_run_never_reached_is_absent_not_zero() {
    // `0` as a count would be indistinguishable from "ran and was counted zero
    // times", which is not a state a run can be in. Absence is the honest answer,
    // and it is what makes `sites()` a coverage reading.
    let mut text = vec![addi(1, 0, 1)];
    text.extend(finish(2, 3));
    let unreached = BASE + (text.len() as u64) * 4;
    text.push(addi(9, 0, 9)); // past the finisher store: never executed.

    let mut m = machine(&text);
    let mut out = Vec::new();
    let (_, p) = run_profiled(&mut m, 100, &mut out);

    assert_eq!(p.at(unreached), 0, "the word after the store never ran");
    assert!(
        !p.reached(unreached),
        "and the profile must not carry a site for it"
    );
    assert_eq!(
        p.sites(),
        text.len() - 1,
        "one word of the text is unreached, so sites is one short of it"
    );
}

#[test]
fn a_loop_reads_as_one_narrow_hot_span() {
    // A two-instruction body run 200 times, behind a prologue as long as the body
    // is short. Half the steps live in eight octets, and that is what the
    // instrument must say.
    const TRIPS: i32 = 200;
    let mut text = vec![addi(1, 0, TRIPS), addi(2, 0, 0)];
    let body_at = BASE + (text.len() as u64) * 4;
    text.push(addi(1, 1, -1)); //          body: countdown
    text.push(bne(1, 0, -4)); //           body: branch back to the countdown
    text.extend(finish(3, 4));

    let mut m = machine(&text);
    let mut out = Vec::new();
    let (halt, p) = run_profiled(&mut m, 10_000, &mut out);
    assert!(
        matches!(
            halt,
            Halt::Finisher {
                status: Some(0),
                ..
            }
        ),
        "{halt:?}"
    );

    let trips = u64::try_from(TRIPS).unwrap();
    assert_eq!(p.at(body_at), trips, "the countdown runs once per trip");
    assert_eq!(p.at(body_at + 4), trips, "so does the branch");
    let half = p.hot_span(1, 2).expect("a run with steps has a hot span");
    assert_eq!(
        half,
        Span {
            lo: body_at,
            hi: body_at + 4,
            steps: 2 * trips,
            sites: 2,
        },
        "half the steps must come back as the loop body and nothing else"
    );
    assert_eq!(
        half.width(),
        8,
        "eight octets of a {}-word text",
        text.len()
    );
    // And the concentration is what a reader would act on: two sites out of nine
    // hold 98% of the run.
    assert_eq!(
        (half.steps * 100 / p.steps(), half.sites, p.sites()),
        (98, 2, text.len()),
        "two sites of eight hold 98% of the run, and every word of the text ran"
    );
    assert_eq!(
        p.hottest(2),
        vec![(body_at, trips), (body_at + 4, trips)],
        "the two busiest program counters are the body, earlier address first"
    );
}

#[test]
fn straight_line_work_is_not_read_as_one_hot_loop() {
    // THE CASE THAT MUST STILL BE REFUSED. Forty instructions, each run exactly
    // once — the shape of a program that is merely long. If `hot_span` came back
    // narrow here it would call every run a hot loop, which is the same blindness
    // `W-306` removed from `ran()`: an instrument that cannot say no says nothing.
    const RUN: usize = 40;
    let mut text: Vec<u32> = (0..RUN).map(|k| addi(1, 1, (k % 16) as i32)).collect();
    text.extend(finish(2, 3));
    let steps = text.len() as u64;

    let mut m = machine(&text);
    let mut out = Vec::new();
    let (halt, p) = run_profiled(&mut m, 1_000, &mut out);
    assert!(
        matches!(
            halt,
            Halt::Finisher {
                status: Some(0),
                ..
            }
        ),
        "{halt:?}"
    );

    assert_eq!(p.sites() as u64, steps, "every word ran, and ran once");
    let half = p.hot_span(1, 2).expect("a run with steps has a hot span");
    assert_eq!(
        half.sites as u64,
        steps.div_ceil(2),
        "reaching half the steps costs half the SITES when nothing repeats"
    );
    assert_eq!(
        half.width(),
        4 * steps.div_ceil(2),
        "so the window is half the text wide — the refusal this test exists for"
    );
    // Stated as the ratio a caller would branch on: the window is HALF the text,
    // against the 4.5% (8 octets of 176) the loop above gives at the same 50% of
    // its steps. That gap is the instrument's whole signal.
    assert_eq!(
        half.width() * 100 / (steps * 4),
        50,
        "half the steps need half the text when nothing repeats"
    );
}

#[test]
fn a_run_with_no_steps_has_no_hot_span_and_a_zero_denominator_is_refused() {
    // Two ways to ask an unanswerable question. Neither divides by zero, and
    // neither invents a window: a span is a claim about where steps went, and
    // there is no such claim to make here.
    let mut m = machine(&[addi(1, 0, 1)]);
    let mut out = Vec::new();
    let (halt, p) = run_profiled(&mut m, 0, &mut out);
    assert_eq!(
        halt,
        Halt::StepLimit { pc: BASE },
        "a zero budget stops before the first instruction"
    );
    assert_eq!(p.steps(), 0);
    assert_eq!(p.sites(), 0);
    assert_eq!(p.hot_span(1, 2), None, "no steps, no span");

    let mut text = vec![addi(1, 0, 1)];
    text.extend(finish(2, 3));
    let mut m = machine(&text);
    let (_, p) = run_profiled(&mut m, 100, &mut out);
    assert!(p.steps() > 0);
    assert_eq!(p.hot_span(1, 0), None, "a zero denominator is refused");
    assert_eq!(
        p.hot_span(1, 1).map(|s| s.steps),
        Some(p.steps()),
        "and all of them is the whole text"
    );
}
