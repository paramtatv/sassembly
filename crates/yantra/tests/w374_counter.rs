//! **`W-374`: THE RETIRED-INSTRUCTION COUNTER, EXPOSED AS A DIAGNOSTIC LOAD ONLY.**
//!
//! ADR-0040, "The counter, and the trap that is not the obvious one": a load from one MMIO
//! address answers [`Machine::time`], the count `yantra-run` prints as `steps:`. Reading
//! it is one instruction like any other, so the answer stays a function of the program
//! and its input. The hazard is coupling to codegen, which is why it is DIAGNOSTIC and
//! why `W-372`'s ratchet refuses a fixpoint image that reads it.
//!
//! Driven by real RV64 words through `Machine::run`. The `.t1` path is `W-350`'s seam,
//! `अष्टकॱउपकरणचतुरष्टकाहारः`, which lowers to `lw` at the absolute address and a mask to
//! 32 bits (`ir.t1`, the `उपकरणाहारमिदम्` arm) — falsifier (a) uses exactly that shape.
//!
//! THE ROW'S FALSIFIER: a program reading the counter twice with `k` instructions
//! between sees a difference of exactly `k + 1` — the `k`, plus the first read itself.

use yantra::{COUNTER, FINISHER, Halt, Machine, Privilege};

const BASE: u64 = 0x8000_0000;

fn machine(text: &[u32]) -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX,
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
    for (n, w) in text.iter().enumerate() {
        m.mem[n * 4..n * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    m
}

// Encoders written out, not taken from `sadhana::encode` (`interpreter.rs`'s rule).
fn r(op: u32, rd: u32, f3: u32, rs1: u32, rs2: u32, f7: u32) -> u32 {
    op | rd << 7 | f3 << 12 | rs1 << 15 | rs2 << 20 | f7 << 25
}
fn i(op: u32, rd: u32, f3: u32, rs1: u32, imm: i32) -> u32 {
    op | rd << 7 | f3 << 12 | rs1 << 15 | ((imm as u32) & 0xfff) << 20
}
fn s(op: u32, f3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    op | (imm & 0x1f) << 7 | f3 << 12 | rs1 << 15 | rs2 << 20 | (imm >> 5 & 0x7f) << 25
}
fn lui(rd: u32, imm: u32) -> u32 {
    0x37 | rd << 7 | (imm & 0xffff_f000)
}
fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    i(0x13, rd, 0, rs1, imm)
}
const NOP: u32 = 0x13;
const LW: u32 = 2;
const LD: u32 = 3;
const LWU: u32 = 6;

/// `x5 = COUNTER`. Asserted, so a move of the constant reds here.
fn counter_address() -> [u32; 2] {
    assert_eq!(
        COUNTER, 0x1000_0108,
        "the encoding below spells this address"
    );
    [lui(5, 0x1000_0000), addi(5, 5, 0x108)]
}

/// The SUCCESS finisher, four words.
fn finish() -> [u32; 4] {
    [
        lui(10, FINISHER as u32),
        lui(11, 0x5000),
        addi(11, 11, 0x555),
        s(0x23, 2, 10, 11, 0),
    ]
}

/// Read the counter into `x6` with `load` (funct3), `k` NOPs, read it into `x7`, finish.
/// `mask` adds the seam's `and` with 0xffff_ffff after each read (`x28` holds the mask),
/// which is what `अष्टकॱउपकरणचतुरष्टकाहारः` lowers to.
fn two_reads(load: u32, k: usize, mask: bool) -> Vec<u32> {
    let [a, b] = counter_address();
    let mut t = vec![a, b];
    if mask {
        // x28 = 0xffff_ffff: addi x28, x0, -1 then srli x28, x28, 32.
        t.push(addi(28, 0, -1));
        t.push(i(0x13, 28, 5, 28, 32));
    }
    t.push(i(0x03, 6, load, 5, 0));
    if mask {
        t.push(r(0x33, 6, 7, 6, 28, 0));
    }
    t.extend(std::iter::repeat_n(NOP, k));
    t.push(i(0x03, 7, load, 5, 0));
    if mask {
        t.push(r(0x33, 7, 7, 7, 28, 0));
    }
    t.extend(finish());
    t
}

fn run_to_finisher(m: &mut Machine) {
    match m.run(10_000, &mut Vec::new()) {
        Halt::Finisher {
            status: Some(0), ..
        } => {}
        other => panic!("the program must finish, got {other:?}"),
    }
}

/// (a) THE ROW'S FALSIFIER, through the `.t1` seam's own shape (`lw` + mask): the two
/// readings differ by exactly `k + 1`, for several `k`. With a mask between the reads
/// the instructions between them are `and`, `k` NOPs — so the `k` here counts them.
#[test]
fn two_reads_k_instructions_apart_differ_by_k_plus_one_through_the_seam_shape() {
    for k in [0usize, 1, 5, 40] {
        let mut m = machine(&two_reads(LW, k, true));
        run_to_finisher(&mut m);
        // Between the two loads: the mask `and` after the first read, then k NOPs.
        let between = k as u64 + 1;
        assert_eq!(
            m.x[7] - m.x[6],
            between + 1,
            "k = {k}: first {} second {}",
            m.x[6],
            m.x[7]
        );
    }
}

/// (a') The same with a bare `ld`, the full sixty-four bits, `k` NOPs and nothing else
/// between: exactly `k + 1`. And the reading is the machine's own `time` at that
/// instruction — the first read is the 3rd instruction begun.
#[test]
fn a_full_width_read_answers_the_retired_count_at_that_instruction() {
    for k in [0usize, 3, 17] {
        let mut m = machine(&two_reads(LD, k, false));
        run_to_finisher(&mut m);
        assert_eq!(
            m.x[6], 3,
            "lui, addi, then the read itself: the third instruction"
        );
        assert_eq!(m.x[7] - m.x[6], k as u64 + 1, "k = {k}");
    }
}

/// (b) The window is eight octets: `COUNTER` is the low word and `COUNTER + 4` the high
/// word, as two 32-bit reads of a 64-bit register take them. Checked across 2^32.
#[test]
fn the_high_word_is_at_counter_plus_four() {
    let [a, b] = counter_address();
    let mut t = vec![
        a,
        b,
        i(0x03, 6, LWU, 5, 0),
        i(0x03, 7, LWU, 5, 4),
        i(0x03, 8, LD, 5, 0),
    ];
    t.extend(finish());
    let mut m = machine(&t);
    let start = 0x1_ffff_fffd_u64;
    m.time = start;
    run_to_finisher(&mut m);
    // The three reads are instructions 3, 4 and 5 after `start`.
    assert_eq!(m.x[6], (start + 3) & 0xffff_ffff, "low word");
    assert_eq!(m.x[7], (start + 4) >> 32, "high word");
    assert_eq!(m.x[8], start + 5, "full width");
}

/// (c) DIAGNOSTIC LOAD ONLY: a store to the counter is refused, not swallowed and not
/// taken as a way to set the clock.
#[test]
fn a_store_to_the_counter_is_refused() {
    let [a, b] = counter_address();
    let mut t = vec![a, b, s(0x23, 3, 5, 0, 0)];
    t.extend(finish());
    let halt = machine(&t).run(100, &mut Vec::new());
    assert_eq!(
        halt,
        Halt::BadAccess {
            pc: BASE + 8,
            addr: COUNTER
        }
    );
}

/// (d) A read that straddles the window's end, or is misaligned within it, is refused by
/// name rather than answered with a shifted half of the count.
#[test]
fn a_misaligned_or_straddling_read_is_refused_by_name() {
    let [a, b] = counter_address();
    for (off, f3) in [(2, LW), (4, LD), (1, LWU)] {
        let mut t = vec![a, b, i(0x03, 6, f3, 5, off)];
        t.extend(finish());
        let halt = machine(&t).run(100, &mut Vec::new());
        assert!(
            matches!(halt, Halt::Device { pc, addr, .. } if pc == BASE + 8 && addr == COUNTER + off as u64),
            "offset {off}, funct3 {f3}: {halt:?}"
        );
    }
}

/// (e) The count is a function of the program: two runs of the same image read the same
/// two values.
#[test]
fn two_runs_read_the_same_values() {
    let mut one = machine(&two_reads(LW, 7, true));
    let mut two = machine(&two_reads(LW, 7, true));
    run_to_finisher(&mut one);
    run_to_finisher(&mut two);
    assert_eq!((one.x[6], one.x[7]), (two.x[6], two.x[7]));
}
