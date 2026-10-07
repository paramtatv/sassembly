//! **AN ORDINARY PROGRAM ALLOCATES — THE HEAP IS ALREADY THERE.**
//!
//! Measured 2026-09-24, and it corrects a reading this repository invites.
//! `lib.t1` is fifteen lines of comments; a compiled program reaches the
//! machine through two SBI calls; so "no standard library" reads as "no heap,
//! no allocator, nothing but the stack". The obvious plan follows: write a
//! bump allocator, carve a region, export `_heap_start`/`_heap_end`.
//!
//! **All of that exists.** `ir.t1:604`: the emitters own the label
//! `रचनासूचकः`, and "run growth bumps it with LoadAt/StoreAt through
//! `वैश्विकस्थानाज्ञाभेद` — no new kind". `ir.t1:610`: the cursor holds an
//! OFFSET into `रचनाक्षेत्रम्`, and "the allocator answers region + offset".
//! The region is `यन्त्ररचनाष्टकाः` = ५३६८७०९१२ octets (512 MiB) at
//! `yantrotsarjana.t1:1834`, sized on 2026-09-14 from a measurement: the first
//! whole-corpus native compile reached 353,242,600 octets of high water, and
//! 512 MiB is 1.5x that.
//!
//! So the image's ~537 MB `.bss` is not a defect and not accumulated object
//! `.bss` — the 21 objects contribute 211,248 octets, 0.04% of it. **THE
//! `.bss` IS THE HEAP**, counted in the layout and never materialised, which is
//! why `yantra::ram_for` hands the image about 1.6 GB.
//!
//! **THE SIXTEEN-BIT TRAP.** A finisher status is sixteen bits. This fixture
//! sums 0..4999 = 12,497,500 and the machine reports 45,660, which is
//! 12,497,500 mod 65536. Comparing the raw numbers reads as a wrong answer
//! from a working allocator — so the assertion below compares MODULO 65536 and
//! says so, and the fixture's expected value is written out in full.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::Machine;

const FUEL: u64 = 80_000_000_000;

/// 0 + 1 + … + 4999. Chosen to EXCEED sixteen bits, so a fixture that silently
/// stopped growing the run early would answer a different residue rather than
/// a coincidentally equal one.
const N: u64 = 5000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

/// Grows a run to `N` elements, then walks it and sums. BOTH halves matter:
/// growth exercises the allocator, and the sum proves every element survived —
/// a run that reallocated and lost its earlier contents would still have the
/// right length. `ॱ दैर्घ्य` is read for the walk rather than `N`, so a run
/// that grew short is caught by the sum rather than passing a length it set
/// itself.
const GROWS_AND_SUMS: &str = "मण्डलम् चयन ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः सञ्चयः ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् ५००० आदि
        सञ्चयः अङ्कः क्रमः अन्तः भवति क्रमः ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    चरः योगः ॱॱ न६४ भवति ० ।
    चरः ज ॱॱ न६४ भवति ० ।
    यावत् ज न्यूनम् सञ्चयः ॱ दैर्घ्य आदि
        योगः भवति योगः योगः सञ्चयः अङ्कः ज अन्तः ।
        ज भवति ज योगः १ ।
    इति
    प्रत्यागमनम् योगः ।
इति
";

/// The fixture's answer **under the interpreter**, for comparison with the
/// native run.
///
/// WHY THIS EXISTS. The first version of this file ran the image natively and
/// stopped there. Every defect this corpus has paid for is the two engines
/// DISAGREEING — five in one day on 2026-09-22, each invisible to the
/// interpreter and visible only when the compiled compiler ran itself — so a
/// native-only number is the weak form of the evidence. `t1_image` has carried
/// the cross-check all along behind `--load`, which puts a source into the
/// INTERPRETER's program without adding it to the build; omitting it is why the
/// predict answered "no routine named …" and I read a correct refusal as noise.
///
/// It also removes the sixteen-bit ambiguity rather than reasoning around it:
/// the interpreter answers 12497500 where the finisher can only carry 45660.
fn interpreted(src: &str, routine: &str) -> u64 {
    let mut srcs: Vec<(&str, &str)> = CHAIN.to_vec();
    srcs.push(("चयन.t1", src));
    let mut it = Interpreter::load(&srcs, &spec_root()).expect("the chain plus the fixture loads");
    it.call(routine, vec![], FUEL)
        .expect("the fixture's routine runs under the interpreter")
        .as_int()
        .expect("it answers a number") as u64
}

fn build(src: &str, module: &str) -> Vec<u8> {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets(module.as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    it.call(
        "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
        vec![
            arena(vec![octets(src.as_bytes())]),
            arena(vec![octets(module.as_bytes())]),
            Value::Int(1),
        ],
        FUEL,
    )
    .expect("मण्डलानिप्रतिबिम्बम् runs")
    .octets()
    .map(|o| o.as_slice().to_vec())
    .unwrap_or_default()
}

#[test]
fn a_program_grows_a_run_to_five_thousand_and_every_element_survives() {
    let image = build(GROWS_AND_SUMS, "चयन");
    assert!(!image.is_empty(), "the fixture built no image");

    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let status = match m.run(200_000_000, &mut out) {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => s,
        other => panic!("the fixture did not finish: {other:?}"),
    };

    let want: u64 = (0..N).sum();
    assert_eq!(want, 12_497_500, "the fixture's own arithmetic moved");

    // THE CROSS-ENGINE CHECK. The interpreter answers untruncated, so this
    // pins the full value as well as agreement; a defect that moved only the
    // native side lands here and not on the assertion below.
    let interp = interpreted(GROWS_AND_SUMS, "चयनॱमुख्यम्");
    assert_eq!(
        interp, want,
        "the INTERPRETER summed {interp} where 0..{N} is {want} — the fixture \
         is wrong before either engine is in question"
    );
    assert_eq!(
        interp, status,
        "the two engines DISAGREE: interpreted {interp}, native {status}. This \
         is the class every defect in this corpus has been — invisible to the \
         interpreter, visible when the compiled code runs — so it is asserted \
         separately from the arithmetic below"
    );
    // WHOLE, not `% 65536`: until `W-341` the finisher status was sixteen bits
    // and this test compared residues. The full value asserts more — a run that
    // stopped growing early or lost elements on reallocation cannot hide in a
    // matching residue.
    assert_eq!(
        status, want,
        "summed {status}; 0..{N} is {want}. A run that stopped growing early, \
         or that lost its earlier elements when it reallocated, lands on a \
         different sum — this is not a length check"
    );
}

#[test]
fn the_image_declares_the_region_it_allocates_from() {
    let image = build(GROWS_AND_SUMS, "चयन");
    let m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");

    // `kosha.t1:138` sets `p_memsz = filesz + शून्यक्षेत्रम्`, and
    // `yantra::ram_for` sizes RAM from the extent. So the reservation is a
    // REQUEST the host honours, not a file cost: a 512 MiB region on a default
    // 20 MiB RAM is not a contradiction, and this pins that the loader really
    // does hand over enough to allocate in.
    let ram = yantra::ram_for(&image);
    assert!(
        ram > 512 * 1024 * 1024,
        "the loader gave {ram} octets of RAM, which is under the 512 MiB \
         record region `यन्त्ररचनाष्टकाः` declares. Allocation would fail on \
         the first growth past the region's start"
    );
    assert!(
        m.mem.len() >= ram,
        "the machine's memory is smaller than the RAM the image asked for"
    );
}
