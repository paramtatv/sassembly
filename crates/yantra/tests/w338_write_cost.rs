//! **`W-338`: AN ARENA ELEMENT WRITE IN A LOOP — ITS COST, RATCHETED.**
//!
//! THE RATCHETS ARE THE CLAIM, NOT THE FALSIFIER. The row's falsifier (a write
//! at most 2x a read) is asserted too, but on its own it PASSES THE OLD
//! LOWERING: origin/main measured 28 against a read of 14, and 28 <= 28. So
//! what this file refuses is a write dearer than `WRITE_RATCHET`, a raising
//! write dearer than `RAISE_RATCHET`, and a growing fill dearer than
//! `FILL_RATCHET_HUNDREDTHS` — each red on origin/main at 1456770f. The guarded
//! parameter store (`W-359`, a different lowering this change did not touch)
//! is pinned at exactly `GUARDED_PER_STORE`.
//!
//! The row's method, at a smaller count: one module, one loop, and the loop
//! body is the only thing that varies. The cost of a construct is its executed-
//! instruction delta against the empty body, per iteration, and it must divide
//! exactly. Each probe is run at two loop counts and only the DIFFERENCE is
//! used, because the code after the loop is allocated differently per body and
//! moves the one-off cost by a few instructions.
//!
//! ```text
//!   per iteration          origin/main 1456770f   W-338    W-381 stage 4
//!   read  a[j]                   14                  14        13
//!   write a[j] (pre-sized)       28                  19        19
//!   write a[j] (length raised)   28                  23        24
//!   growing fill a[j] from empty 50.47               45.47     46.47  (4,647,880 over 1e5)
//!   guarded parameter a[j]       13                  13        13     (W-359)
//! ```
//!
//! `W-381` STAGE 4 (O4, native bounds checks on reads and writes, owner-ruled),
//! measured here: a READ costs 13, one fewer than before — the bound (the length
//! word loaded at base − ८, an unsigned compare) adds two, and the read's dead
//! scratch slot, gone because every refusal now spins, saves three (its clear,
//! store and reload). A raising write 23 → 24 (a negative index must not raise
//! the length: one sign branch in the raise block), so the growing fill 45.47 →
//! 46.47 per write. The pre-sized write stays 19 (its length test only became
//! unsigned) and the guarded store 13.
//!
//! And one answer the cost must not buy: a write PAST the length in a loop
//! (index > length) sets the length to index + १ and leaves the gap's slots
//! ० — the same answer origin/main gives.
//!
//! The write is the inline growth path's fast path (`ir.t1`, the index-store
//! arm inside a `यावत्`): nil test, capacity test, then the length. It used to
//! raise the length branch-free and store it back on every write; it now
//! branches to the store when index < length, and only the raise block writes
//! index + १.
//!
//! The image is compiled HERE by the current `.t1` compiler running in the
//! interpreter (`CHAIN`), so it carries the lowering this tree says.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::{Halt, Machine};

const FUEL: u64 = 80_000_000_000;
const MODULE: &str = "लेखव्यय";
/// The loop runs `ज` from १ while `ज < bound`; the two bounds differ by
/// `ITERATIONS` and stay below १२८, so neither run grows either arena.
const LO: u64 = 21;
const HI: u64 = 121;
const ITERATIONS: u64 = HI - LO;

/// The falsifier the row registered: a write at most twice a read.
const FALSIFIER_FACTOR: u64 = 2;
/// Measured by this change; a write may get cheaper, never dearer. origin/main
/// at 1456770f measured 28.
const WRITE_RATCHET: u64 = 19;
/// The same write when every iteration raises the length (index == length,
/// capacity already fits). Measured by this change; 28 on origin/main; 23
/// until `W-381` stage 4 added the raise block's sign branch; 25 since stage 4's
/// review finding 1 (the far-store overflow bound in the growth path, +1).
const RAISE_RATCHET: u64 = 25;
/// A fresh arena filled from index ० to `FILL_HI` − १, per write in hundredths
/// of an instruction (capacity doublings make it fractional). Measured by this
/// change; origin/main measured FILL_MAIN_HUNDREDTHS. 4547 until `W-381` stage 4
/// (one sign branch per raising write: +1.00 exactly); 4647 until stage 4's review
/// finding 1 (the far-store overflow bound, one more compare per raising write: +1.00).
const FILL_RATCHET_HUNDREDTHS: u64 = 4747;
const FILL_MAIN_HUNDREDTHS: u64 = 5047;
const FILL_LO: u64 = 1;
const FILL_HI: u64 = 100_001;
/// `W-359`'s bounded store into a parameter, untouched by this change.
const GUARDED_PER_STORE: u64 = 13;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

/// `सूची` is sized to १२८ by one write of its last index; `नवा` holds one
/// element and a first block of १२८, so a write at index `ज` from १ raises its
/// length every iteration and never grows it. Answers सूची[९९] + |नवा| + स.
fn probe(body: &str, bound: u64) -> String {
    let bound = devanagari(bound);
    format!(
        "मण्डलम् लेखव्यय ॥\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n\
         \x20   चरः सूची ॱॱ अङ्कः अन्तः न६४ भवति ० ।\n\
         \x20   चरः नवा ॱॱ अङ्कः अन्तः न६४ भवति ० ।\n\
         \x20   सूची अङ्कः १२७ अन्तः भवति ० ।\n\
         \x20   नवा अङ्कः ० अन्तः भवति ० ।\n\
         \x20   चरः स ॱॱ न६४ भवति ० ।\n\
         \x20   चरः ज ॱॱ न६४ भवति १ ।\n\
         \x20   यावत् ज न्यूनम् {bound} आदि\n\
         {body}\
         \x20       ज भवति ज योगः १ ।\n\
         \x20   इति\n\
         \x20   चरः फलाङ्कः ॱॱ न६४ भवति सूची अङ्कः ९९ अन्तः ।\n\
         \x20   फलाङ्कः भवति फलाङ्कः योगः नवा ॱ दैर्घ्य ।\n\
         \x20   फलाङ्कः भवति फलाङ्कः योगः स ।\n\
         \x20   प्रत्यागमनम् फलाङ्कः ।\n\
         इति\n"
    )
}

fn compile(src: &str) -> Vec<u8> {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets(MODULE.as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(src.as_bytes())]),
                arena(vec![octets(MODULE.as_bytes())]),
                Value::Int(1),
            ],
            FUEL,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(!image.is_empty(), "the probe built no image");
    image
}

fn devanagari(n: u64) -> String {
    n.to_string()
        .chars()
        .map(|c| char::from_u32(0x966 + c.to_digit(10).expect("a digit")).expect("a digit"))
        .collect()
}

/// (status, executed instructions)
fn run_at(body: &str, bound: u64) -> (u64, u64) {
    execute(&probe(body, bound))
}

/// (status, executed instructions) of one compiled source.
fn execute(src: &str) -> (u64, u64) {
    let image = compile(src);
    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let mut steps: u64 = 0;
    loop {
        steps += 1;
        if let Some(h) = m.step(&mut out) {
            match h {
                Halt::Finisher {
                    status: Some(s), ..
                } => return (s, steps),
                other => panic!("the probe did not finish with a status: {other:?}"),
            }
        }
        assert!(steps < 4_000_000_000, "the image did not halt");
    }
}

/// (status at HI, status at LO, executed instructions HI − LO)
fn run(body: &str) -> (u64, u64, u64) {
    let (s_hi, hi) = run_at(body, HI);
    let (s_lo, lo) = run_at(body, LO);
    (s_hi, s_lo, hi - lo)
}

fn per_iteration(name: &str, steps: u64, empty: u64) -> u64 {
    let delta = steps - empty;
    assert_eq!(
        delta % ITERATIONS,
        0,
        "{name}: a construct's cost is a whole number of instructions per iteration \
         (delta {delta} over {ITERATIONS})"
    );
    delta / ITERATIONS
}

#[test]
fn an_arena_write_in_a_loop_costs_at_most_twice_a_read() {
    let (s_empty, l_empty, empty) = run("");
    let (s_read, l_read, read) = run("        स भवति सूची अङ्कः ज अन्तः ।\n");
    let (s_write, l_write, write) = run("        सूची अङ्कः ज अन्तः भवति ज ।\n");
    let (s_raise, l_raise, raise) = run("        नवा अङ्कः ज अन्तः भवति ज ।\n");
    // The answers first: a cheaper store that stores the wrong thing is no win.
    assert_eq!(
        (s_empty, l_empty),
        (1, 1),
        "empty: सूची[९९] ० + |नवा| १ + स ०"
    );
    assert_eq!(
        (s_read, l_read),
        (1, 1),
        "read: the slots read are ०, so स ends ०"
    );
    assert_eq!(
        (s_write, l_write),
        (100, 1),
        "write: सूची[९९] = ९९ once the loop reaches it, |नवा| = १"
    );
    assert_eq!(
        (s_raise, l_raise),
        (HI, LO),
        "raise: every write raised |नवा|, to the loop bound"
    );
    let read = per_iteration("read", read, empty);
    let write = per_iteration("write", write, empty);
    let raise = per_iteration("raise", raise, empty);
    eprintln!(
        "W-338 cost per iteration: read {read}, write {write} (pre-sized), \
         write {raise} (length raised)"
    );
    assert!(
        write <= FALSIFIER_FACTOR * read,
        "the row's falsifier: a write ({write}) must cost at most {FALSIFIER_FACTOR}x a read ({read})"
    );
    assert!(
        write <= WRITE_RATCHET,
        "a pre-sized write costs {write}, more than the ratchet's {WRITE_RATCHET} \
         (28 on origin/main at 1456770f)"
    );
    assert!(
        raise <= RAISE_RATCHET,
        "a length-raising write costs {raise}, more than the ratchet's {RAISE_RATCHET}"
    );
}

/// A FRESH arena filled at `ज` from ० while `ज < bound` — every write raises
/// the length and ~log2(bound) of them grow the block. Answers |नवा|.
fn fill_probe(write: bool, bound: u64) -> String {
    let body = if write {
        "        नवा अङ्कः ज अन्तः भवति ज ।\n"
    } else {
        ""
    };
    let bound = devanagari(bound);
    format!(
        "मण्डलम् लेखव्यय ॥\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n\
         \x20   चरः नवा ॱॱ अङ्कः अन्तः न६४ भवति ० ।\n\
         \x20   चरः ज ॱॱ न६४ भवति ० ।\n\
         \x20   यावत् ज न्यूनम् {bound} आदि\n\
         {body}\
         \x20       ज भवति ज योगः १ ।\n\
         \x20   इति\n\
         \x20   प्रत्यागमनम् नवा ॱ दैर्घ्य ।\n\
         इति\n"
    )
}

#[test]
fn a_growing_fill_costs_no_more_than_the_ratchet() {
    let (s_hi, w_hi) = execute(&fill_probe(true, FILL_HI));
    let (s_lo, w_lo) = execute(&fill_probe(true, FILL_LO));
    let (e_hi_s, e_hi) = execute(&fill_probe(false, FILL_HI));
    let (e_lo_s, e_lo) = execute(&fill_probe(false, FILL_LO));
    assert_eq!(
        (s_hi, s_lo),
        (FILL_HI, FILL_LO),
        "the fill answers its length"
    );
    assert_eq!(
        (e_hi_s, e_lo_s),
        (0, 0),
        "the empty loop leaves the arena empty"
    );
    let delta = (w_hi - w_lo) - (e_hi - e_lo);
    let hundredths = delta * 100 / (FILL_HI - FILL_LO);
    eprintln!(
        "W-338 growing fill: {delta} instructions over {} writes, {hundredths}/100 per write",
        FILL_HI - FILL_LO
    );
    assert!(
        hundredths <= FILL_RATCHET_HUNDREDTHS,
        "a growing fill costs {hundredths}/100 per write, more than the ratchet's \
         {FILL_RATCHET_HUNDREDTHS} (origin/main at 1456770f: {FILL_MAIN_HUNDREDTHS})"
    );
}

/// THE INDEX > LENGTH CASE, in a loop: writes at ५ and १० into a run of one.
/// The length must become ११ and the eight gap slots read ०. Answers
/// |नवा| × १००० + the sum of slots ० to १०, which is ११ × १००० + ० + १ + २.
const GAP: &str = "मण्डलम् लेखव्यय ॥
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः नवा ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    नवा अङ्कः ० अन्तः भवति ० ।
    चरः ज ॱॱ न६४ भवति १ ।
    यावत् ज न्यूनम् ३ आदि
        चरः क ॱॱ न६४ भवति ज गुणनम् ५ ।
        नवा अङ्कः क अन्तः भवति ज ।
        ज भवति ज योगः १ ।
    इति
    चरः योगफलम् ॱॱ न६४ भवति ० ।
    चरः म ॱॱ न६४ भवति ० ।
    यावत् म न्यूनम् नवा ॱ दैर्घ्य आदि
        योगफलम् भवति योगफलम् योगः नवा अङ्कः म अन्तः ।
        म भवति म योगः १ ।
    इति
    चरः फलाङ्कः ॱॱ न६४ भवति नवा ॱ दैर्घ्य गुणनम् १००० ।
    फलाङ्कः भवति फलाङ्कः योगः योगफलम् ।
    प्रत्यागमनम् फलाङ्कः ।
इति
";

#[test]
fn a_write_past_the_length_in_a_loop_sets_the_length_and_leaves_the_gap_zero() {
    let (status, _) = execute(GAP);
    assert_eq!(
        status, 11_003,
        "index > length: |नवा| must be ११ and the gap slots ० (origin/main answers 11003)"
    );
}

/// `W-359`'s probe shape: a non-returning callee writes its PARAMETER, sized
/// to १२८ by the caller, at `ज` while `ज < bound`.
fn guarded_probe(store: bool, bound: u64) -> String {
    let line = if store {
        "        सूची अङ्कः ज अन्तः भवति ज ।\n"
    } else {
        ""
    };
    let bound = devanagari(bound);
    format!(
        "मण्डलम् लेखव्यय ॥\n\
         वृत्तिः लेखकः आदाय सूची ॱॱ अङ्कः अन्तः न६४ ऽ परिमाणम् ॱॱ न६४ ददाति न६४ आदि\n\
         \x20   चरः ज ॱॱ न६४ भवति ० ।\n\
         \x20   यावत् ज न्यूनम् परिमाणम् आदि\n\
         {line}\
         \x20       ज भवति ज योगः १ ।\n\
         \x20   इति\n\
         \x20   प्रत्यागमनम् ज ।\n\
         इति\n\
         सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n\
         \x20   चरः सूची ॱॱ अङ्कः अन्तः न६४ भवति ० ।\n\
         \x20   सूची अङ्कः १२७ अन्तः भवति ० ।\n\
         \x20   चरः उ ॱॱ न६४ भवति लेखकः सूची {bound} ।\n\
         \x20   प्रत्यागमनम् उ ।\n\
         इति\n"
    )
}

#[test]
fn a_guarded_parameter_store_stays_at_thirteen() {
    let (s1, w_hi) = execute(&guarded_probe(true, HI));
    let (s2, w_lo) = execute(&guarded_probe(true, LO));
    let (s3, e_hi) = execute(&guarded_probe(false, HI));
    let (s4, e_lo) = execute(&guarded_probe(false, LO));
    assert_eq!(
        (s1, s2, s3, s4),
        (HI, LO, HI, LO),
        "the callee answers its count"
    );
    let delta = (w_hi - w_lo) - (e_hi - e_lo);
    assert_eq!(
        delta % ITERATIONS,
        0,
        "a whole number per store (delta {delta})"
    );
    assert_eq!(
        delta / ITERATIONS,
        GUARDED_PER_STORE,
        "W-359's bounded parameter store moved; this change must not touch it"
    );
}
