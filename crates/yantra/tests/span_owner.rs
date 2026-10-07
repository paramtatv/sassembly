//! **NAMING THE WINDOW — and the case where it must refuse to name just one.**
//!
//! `W-306` measured `shrinkhala.t1`'s run as ONE HOT LOOP: nine tenths of its
//! steps inside 6,064 octets of a 456,852-octet text. That is a shape without a
//! site. [`yantra::profile::StepProfile::owners`] maps the window back through
//! the image's symbol table so the finding can say WHICH ROUTINE, which is what
//! turns "there is a loop at `+0x6e0d8`" into somewhere to go read.
//!
//! The reading is only worth having if it can come back with more than one
//! answer, so the refusal is here beside the finding:
//! [`a_span_across_two_routines_names_both_and_does_not_pick_the_nearer`] is a
//! window that genuinely straddles a boundary, and it must report BOTH routines
//! with the steps split between them. An instrument that always named exactly
//! one routine would name one whether or not that were true — the two-state
//! instrument `W-306`'s own predecessor removed — and
//! [`text_no_symbol_claims_is_reported_as_unnamed`] is the third state: a
//! window over code the symbol table does not cover is not "the nearest name",
//! it is no name at all.
//!
//! # Its own loader, deliberately
//!
//! As in `step_profile.rs`: the machine here is this file's. The placement (text
//! at [`BASE`], nothing else) is part of what the addresses in these assertions
//! mean, and a shared loader that grew a field would change this test without
//! anyone reading it.

use yantra::profile::{Owner, Routine, Span, routines, run_profiled};
use yantra::{FINISHER, Halt, Machine, Privilege};

const BASE: u64 = 0x8000_0000;

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
/// The four words that write `0x5555` to the finisher and stop the machine.
fn finish(addr_reg: u32, val_reg: u32) -> [u32; 4] {
    [
        u_type(0x37, addr_reg, FINISHER as u32),
        u_type(0x37, val_reg, 0x5000),
        addi(val_reg, val_reg, 0x555),
        s_type(0x23, 0x2, addr_reg, val_reg, 0),
    ]
}

/// A word index in [`program`]'s text, as an address.
fn at(word: u64) -> u64 {
    BASE + word * 4
}

/// **THE FIXTURE.** Eleven words in two halves, and the halves are what every
/// assertion below is about.
///
/// ```text
///   word 0   addi x1, x0, 5     प्रथमम् begins here
///   word 1   addi x2, x0, 0
///   word 2   addi x1, x1, -1    the loop body: five trips
///   word 3   addi x2, x2, 1
///   word 4   bne  x1, x0, -8
///   word 5   addi x3, x0, 1     द्वितीयम् begins here
///   word 6   addi x4, x0, 2
///   words 7..10                 the finisher
/// ```
///
/// 23 steps: 2 before the loop, 15 in it (three words, five trips), 2 after, and
/// the finisher's 4. The counts are written out because the assertions name
/// them, and a fixture whose step count nobody can derive by hand cannot be
/// checked against the profile it produces.
fn program() -> Vec<u32> {
    let mut text = vec![
        addi(1, 0, 5),
        addi(2, 0, 0),
        addi(1, 1, -1),
        addi(2, 2, 1),
        bne(1, 0, -8),
        addi(3, 0, 1),
        addi(4, 0, 2),
    ];
    text.extend(finish(5, 6));
    text
}

/// The two routines the fixture's symbol table declares: `प्रथमम्` owns words
/// 0..=4, `द्वितीयम्` owns word 5 to the end of the text.
fn fixture_routines(text_words: u64) -> Vec<Routine> {
    routines(
        &[("प्रथमम्".to_string(), at(0)), ("द्वितीयम्".to_string(), at(5))],
        BASE,
        at(text_words) - 1,
    )
}

/// Run the fixture and hand back its profile, having checked it really finished
/// — a profile of a run that hit the budget is a profile of the budget.
fn profiled() -> (yantra::profile::StepProfile, u64) {
    let text = program();
    let mut m = machine(&text);
    let mut out = Vec::new();
    let (halt, p) = run_profiled(&mut m, 1_000, &mut out);
    assert!(
        matches!(halt, Halt::Finisher { .. }),
        "the fixture must reach the finisher, got {halt:?}"
    );
    assert_eq!(p.steps(), 23, "the fixture's step count, derived by hand");
    (p, text.len() as u64)
}

/// A span over an address range, with its steps and sites read off the profile
/// rather than asserted into it.
fn span_over(p: &yantra::profile::StepProfile, lo: u64, hi: u64) -> Span {
    let (steps, sites) = p.steps_in(lo, hi);
    Span {
        lo,
        hi,
        steps,
        sites,
    }
}

fn named(o: &Owner) -> &str {
    o.name.as_deref().unwrap_or("<unnamed>")
}

#[test]
fn a_symbol_table_becomes_routines_that_end_where_the_next_one_begins() {
    // The extent is an inference — the linker's `Symbol` carries an address and
    // no size — so the rule it infers by is worth pinning: up to the next name,
    // and the last name to the end of the text.
    let rs = fixture_routines(11);
    assert_eq!(rs.len(), 2, "two names, two routines: {rs:?}");
    assert_eq!(rs[0].name, "प्रथमम्");
    assert_eq!((rs[0].lo, rs[0].hi), (at(0), at(5) - 1));
    assert_eq!(rs[0].width(), 20, "words 0..=4, twenty octets");
    assert_eq!(rs[1].name, "द्वितीयम्");
    assert_eq!(
        (rs[1].lo, rs[1].hi),
        (at(5), at(11) - 1),
        "the last routine runs to the end of the text, not to its own first word"
    );
}

#[test]
fn a_symbol_outside_the_text_is_dropped_and_two_names_at_one_address_collapse() {
    // Dropped rather than clamped: a name that is not in THIS text cannot be
    // given a piece of it. And two names at one address are an alias, not two
    // routines — two entries with identical ranges would count the same steps
    // twice, which is the one thing `owners` must never do.
    let rs = routines(
        &[
            ("अ".to_string(), at(0)),
            ("आ".to_string(), at(0)),
            ("बहिः".to_string(), at(99)),
            ("अधः".to_string(), BASE - 4),
            (String::new(), at(2)),
        ],
        BASE,
        at(11) - 1,
    );
    assert_eq!(
        rs.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        ["अ"],
        "one routine: the alias collapses, both out-of-text names go, and the \
         empty name is not a name"
    );
    assert_eq!((rs[0].lo, rs[0].hi), (BASE, at(11) - 1));
}

#[test]
fn the_hot_span_of_a_loop_names_the_one_routine_that_owns_it() {
    // The finding `W-306` could not make: not "half the steps at +0x8" but half
    // the steps in प्रथमम्.
    let (p, words) = profiled();
    let rs = fixture_routines(words);
    let half = p.hot_span(1, 2).expect("a run with steps has a hot span");
    assert_eq!(
        (half.lo, half.hi, half.steps),
        (at(2), at(4), 15),
        "the loop body is the narrowest window holding twelve of 23 steps"
    );

    let owners = p.owners(&half, &rs);
    assert_eq!(
        owners.len(),
        1,
        "the window is inside one routine: {:?}",
        owners.iter().map(named).collect::<Vec<_>>()
    );
    assert_eq!(owners[0].name.as_deref(), Some("प्रथमम्"));
    assert_eq!(owners[0].steps, 15, "all of the window's steps");
    assert_eq!(owners[0].sites, 3);
    assert_eq!(
        (owners[0].lo, owners[0].hi),
        (half.lo, half.hi),
        "the overlap is the SPAN's range here, not the routine's twenty octets"
    );
}

#[test]
fn a_span_across_two_routines_names_both_and_does_not_pick_the_nearer() {
    // **THE REFUSAL.** This window runs from the loop's last word into the
    // second routine. An instrument that answered with one name — the nearer
    // symbol, the one holding the most steps, the one the span starts in —
    // would answer with one name for every window ever handed to it, and could
    // never be caught being wrong. Two routines, two entries, steps split.
    let (p, words) = profiled();
    let rs = fixture_routines(words);
    let straddle = span_over(&p, at(4), at(6));
    assert_eq!(
        straddle.steps, 7,
        "five trips through word 4, then two words"
    );

    let owners = p.owners(&straddle, &rs);
    assert_eq!(
        owners.iter().map(named).collect::<Vec<_>>(),
        ["प्रथमम्", "द्वितीयम्"],
        "both, in address order"
    );
    assert_eq!(
        (owners[0].lo, owners[0].hi, owners[0].steps),
        (at(4), at(5) - 1, 5),
        "प्रथमम्'s share stops at its own last octet"
    );
    assert_eq!(
        (owners[1].lo, owners[1].hi, owners[1].steps),
        (at(5), at(6), 2),
        "द्वितीयम्'s share starts at its first"
    );
    assert_eq!(
        owners.iter().map(|o| o.steps).sum::<u64>(),
        straddle.steps,
        "the shares account for the whole window — a split that lost or \
         duplicated steps would make the narrower reading a lie"
    );
    assert_eq!(
        owners.iter().map(|o| o.sites).sum::<usize>(),
        straddle.sites
    );
}

#[test]
fn text_no_symbol_claims_is_reported_as_unnamed() {
    // The third state. A window over code the symbol table does not cover is
    // not "the nearest name": a profile that answered `द्वितीयम्` for the loop
    // because `द्वितीयम्` was the only name it had would be reporting the
    // symbol table's gap as a finding about the program.
    let (p, words) = profiled();
    let only_second = routines(&[("द्वितीयम्".to_string(), at(5))], BASE, at(words) - 1);
    let wide = span_over(&p, at(2), at(6));

    let owners = p.owners(&wide, &only_second);
    assert_eq!(
        owners.iter().map(named).collect::<Vec<_>>(),
        ["<unnamed>", "द्वितीयम्"],
        "the unclaimed stretch comes first and has no name"
    );
    assert_eq!(owners[0].name, None);
    assert_eq!((owners[0].lo, owners[0].hi), (at(2), at(5) - 1));
    assert_eq!(owners[0].steps, 15, "the loop's steps, owned by nobody");
    assert_eq!(
        owners.iter().map(|o| o.steps).sum::<u64>(),
        wide.steps,
        "an unnamed stretch still accounts for its steps"
    );
}

#[test]
fn an_unnamed_gap_the_run_never_entered_is_not_reported() {
    // Padding is not a finding. The span below ends past the last instruction
    // the run reached; the octets after it are covered by no routine and hold
    // no steps, and an entry for them would be a line of output per alignment
    // hole.
    let (p, words) = profiled();
    let head = routines(&[("प्रथमम्".to_string(), at(0))], BASE, at(2) - 1);
    let wide = span_over(&p, at(0), at(6));

    let owners = p.owners(&wide, &head);
    assert_eq!(
        owners.iter().map(named).collect::<Vec<_>>(),
        ["प्रथमम्", "<unnamed>"],
        "the reached gap is reported"
    );
    assert_eq!(
        owners[1].steps, 17,
        "words 2..=6, the loop and the two after"
    );

    // Now the same span against a routine table that covers all of it: no gap
    // entry at all, because there is nothing unclaimed.
    let whole = fixture_routines(words);
    let covered = p.owners(&wide, &whole);
    assert!(
        covered.iter().all(|o| o.name.is_some()),
        "nothing unclaimed, so nothing unnamed: {:?}",
        covered.iter().map(named).collect::<Vec<_>>()
    );
}

#[test]
fn steps_in_answers_zero_for_an_empty_range_rather_than_wrapping() {
    // `lo > hi` is an empty range, and a `BTreeMap::range` over one panics. The
    // caller that hands it over is `owners` closing a gap of width zero.
    let (p, _) = profiled();
    assert_eq!(
        p.steps_in(at(4), at(2)),
        (0, 0),
        "an inverted range is empty"
    );
    assert_eq!(p.steps_in(at(99), at(200)), (0, 0), "past the text");
    assert_eq!(p.steps_in(at(2), at(4)), (15, 3), "the loop body");
    assert_eq!(
        p.steps_in(BASE, at(11)).0,
        p.steps(),
        "the whole text holds every step"
    );
}

#[test]
fn a_routine_list_out_of_order_reads_the_same() {
    // `owners` sorts what it is given: a symbol table arriving in object order
    // rather than address order is ordinary, and a reading that depended on it
    // would be a different answer for the same image.
    let (p, words) = profiled();
    let mut rs = fixture_routines(words);
    rs.reverse();
    let owners = p.owners(&span_over(&p, at(4), at(6)), &rs);
    assert_eq!(
        owners.iter().map(named).collect::<Vec<_>>(),
        ["प्रथमम्", "द्वितीयम्"]
    );
}

#[test]
fn a_routine_inside_the_window_the_run_never_entered_is_not_an_owner() {
    // **THE COUNT IS THE FINDING, SO WHAT COUNTS MUST BE THE RUN.** `owners`
    // already drops an unclaimed stretch the run never entered — padding is not
    // a finding — but it kept a NAMED routine on the same terms, and the census
    // prints `{n} routine(s)` as the answer to "one hot loop, or spread?". A
    // dead routine the window happens to span holds no steps and is not part of
    // the spread; counting it would report a tighter loop as a looser one.
    //
    // The two runs below are the SAME eleven words and the SAME symbol table,
    // differing only in whether the branch over `मृतम्` is taken. That is the
    // falsifier: a rule that dropped `मृतम्` by its width, its position or its
    // name would drop it from both.
    //
    // ```text
    //   word 0   addi x1, x0, <1 or 0>   प्रथमम् begins here
    //   word 1   bne  x1, x0, +8         taken when x1 is 1: word 2 is skipped
    //   word 2   addi x5, x0, 99         मृतम् begins here
    //   word 3   addi x2, x0, 2          द्वितीयम् begins here
    //   words 4..7                       the finisher
    // ```
    let run = |x1: i32| {
        let mut text = vec![addi(1, 0, x1), bne(1, 0, 8), addi(5, 0, 99), addi(2, 0, 2)];
        text.extend(finish(6, 7));
        let words = text.len() as u64;
        let mut m = machine(&text);
        let mut out = Vec::new();
        let (halt, p) = run_profiled(&mut m, 1_000, &mut out);
        assert!(matches!(halt, Halt::Finisher { .. }), "got {halt:?}");
        let rs = routines(
            &[
                ("प्रथमम्".to_string(), at(0)),
                ("मृतम्".to_string(), at(2)),
                ("द्वितीयम्".to_string(), at(3)),
            ],
            BASE,
            at(words) - 1,
        );
        assert_eq!(rs.len(), 3, "three names, three routines: {rs:?}");
        let span = span_over(&p, at(0), at(3));
        (p, rs, span)
    };

    // Branch taken: word 2 never runs, so `मृतम्` owns nothing.
    let (p, rs, span) = run(1);
    assert_eq!(span.steps, 3, "words 0, 1 and 3");
    let owners = p.owners(&span, &rs);
    assert_eq!(
        owners.iter().map(named).collect::<Vec<_>>(),
        ["प्रथमम्", "द्वितीयम्"],
        "the window spans three routines and TWO of them ran"
    );
    assert_eq!((owners[0].steps, owners[1].steps), (2, 1));
    assert_eq!(
        owners.iter().map(|o| o.steps).sum::<u64>(),
        span.steps,
        "dropping a dead routine must not drop any steps"
    );
    assert!(
        !owners.iter().any(|o| o.name.is_none()),
        "and it must not leave an unnamed hole where the dead routine was: {:?}",
        owners.iter().map(named).collect::<Vec<_>>()
    );

    // **THE CASE THAT MUST STILL BE REPORTED.** Same text, same symbols, branch
    // NOT taken. One step in `मृतम्` is enough to make it an owner again.
    let (p, rs, span) = run(0);
    assert_eq!(span.steps, 4, "words 0, 1, 2 and 3");
    let owners = p.owners(&span, &rs);
    assert_eq!(
        owners.iter().map(named).collect::<Vec<_>>(),
        ["प्रथमम्", "मृतम्", "द्वितीयम्"],
        "entered once, so it is part of the spread"
    );
    assert_eq!(
        (owners[0].steps, owners[1].steps, owners[2].steps),
        (2, 1, 1)
    );
    assert_eq!(owners.iter().map(|o| o.steps).sum::<u64>(), span.steps);
}
