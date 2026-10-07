//! **A SASSEMBLY PROGRAM NAMES A FILE AND READS IT — RUN, NOT JUST COMPILED.**
//!
//! `t1_file_window.rs` tests the protocol by calling `patra::serve` directly.
//! That leaves two things unproven, and they are the two that decide whether
//! the capability exists:
//!
//! * the machine's THREE-ARM ADDRESS DECODE (remember path, remember buffer,
//!   go) — reachable only by a program executing three stores;
//! * the STATUS ROUND TRIP — the host writing into a run the program owns, and
//!   the program reading it back with ordinary indexing.
//!
//! So this compiles a `.t1` program with `अष्टकॱपत्रम्`, runs the image, and
//! checks the SUM OF THE OCTETS it read. The sum, not the count: a window that
//! reported the right length and copied nothing would pass a count check, and
//! that is precisely the failure an MMIO channel makes quietly.
//!
//! **THE PROGRAM IS ORDINARY.** It declares three runs, fills one with a path
//! written `उक्तम् … इति`, and reads the status with `स्थितिः अङ्कः ० अन्तः`.
//! Nothing here is a compiler intrinsic except the one call — which is the
//! claim: file reading is reachable from the language, not from a test harness.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::Machine;

const FUEL: u64 = 80_000_000_000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

/// Reads `कपत्रम्`, sums its octets, and answers the sum.
///
/// `स्थितिः` starts at ९९९ — a value no status can be — so "the host never
/// ran" is distinguishable from every real answer. The guard on `गणना` above
/// the buffer size catches a refusal (`u64::MAX - k`, a very large number)
/// before it is used as a loop bound.
const READS: &str = "मण्डलम् पठकः ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः मार्गः ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् कपत्रम् इति ।
    चरः पात्रम् ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् ६४ आदि
        पात्रम् अङ्कः क्रमः अन्तः भवति ० ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    चरः स्थितिः ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    स्थितिः अङ्कः ० अन्तः भवति ९९९ ।
    चरः अवगणना ॱॱ न६४ भवति अष्टकॱपत्रम् आरभ्य मार्गः ऽ पात्रम् ऽ स्थितिः समाप्तम् ।
    चरः गणना ॱॱ न६४ भवति स्थितिः अङ्कः ० अन्तः ।
    यदि गणना अधिकम् ६४ आदि
        प्रत्यागमनम् गणना ।
    इति
    चरः योगफलम् ॱॱ न६४ भवति ० ।
    चरः ज ॱॱ न६४ भवति ० ।
    यावत् ज न्यूनम् गणना आदि
        योगफलम् भवति योगफलम् योगः पात्रम् अङ्कः ज अन्तः ।
        ज भवति ज योगः १ ।
    इति
    प्रत्यागमनम् योगफलम् ।
इति
";

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
fn a_sassembly_program_reads_a_file_it_named() {
    let dir = std::env::temp_dir().join(format!(
        // pid AND a clock: a pid is REUSED and these roots are never
        // removed, so a pid-only name is unique inside this process and
        // not on disk (W-301).
        "patra-e2e-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let body = b"abc\n"; // 97 + 98 + 99 + 10
    std::fs::write(dir.join("कपत्रम्"), body).expect("fixture");
    let want: u64 = body.iter().map(|b| u64::from(*b)).sum();
    assert_eq!(want, 304, "the fixture's own arithmetic moved");

    let image = build(READS, "पठकः");
    assert!(!image.is_empty(), "the program built no image");

    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    // THE OPT-IN. Without this the window refuses everything, which is the
    // default at all fifteen construction sites in this workspace.
    m.patra_root = Some(dir.clone());

    let mut out: Vec<u8> = Vec::new();
    let status = match m.run(400_000_000, &mut out) {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => s,
        other => panic!("the program did not finish: {other:?}"),
    };

    assert_ne!(
        status, 999,
        "the status run still holds its sentinel — the program ran, the three \
         stores were emitted, and the HOST NEVER SERVED THEM. That is the \
         address decode in lib.rs, not the protocol in patra.rs"
    );
    assert_eq!(
        status,
        want % 65536,
        "the program summed {status}; `abc\\n` is {want}. A window that \
         reported the right count and copied nothing answers ०, and a decode \
         that served the wrong run answers something else again — the SUM is \
         what tells those apart",
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn without_a_root_the_same_program_is_refused_and_says_so() {
    // The negative control, and it is the one that proves the status round trip
    // rather than assuming it: a refusal is a LARGE number written by the host
    // into the program's own run, so seeing it means the write happened.
    let image = build(READS, "पठकः");
    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    // patra_root deliberately left None.
    let mut out: Vec<u8> = Vec::new();
    let status = match m.run(400_000_000, &mut out) {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => s,
        other => panic!("the program did not finish: {other:?}"),
    };
    assert_ne!(
        status, 999,
        "the sentinel survived: the host never wrote a refusal, so nothing was \
         served at all"
    );
    assert_ne!(
        status, 304,
        "a machine with no root READ THE FILE. The default must refuse"
    );
}

/// Writes four octets to `खपत्रम्`, then READS THEM BACK with the other
/// intrinsic and sums them. A round trip, because it is strictly stronger than
/// either half alone:
///
/// * a write that reported success and wrote nothing fails at the read,
/// * a read that answered from stale memory fails because nothing else ever
///   put those octets there,
/// * and the two intrinsics are proven to agree on the layout — the same three
///   runs, the same two remembered stores, differing only in which address the
///   third store names.
///
/// The host is never asked whether the file appeared. The PROGRAM answers, and
/// the answer is the sum of what came back.
const ROUND_TRIP: &str = "मण्डलम् लेखकः ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः मार्गः ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् खपत्रम् इति ।
    चरः पात्रम् ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    पात्रम् अङ्कः ० अन्तः भवति १० ।
    पात्रम् अङ्कः १ अन्तः भवति २० ।
    पात्रम् अङ्कः २ अन्तः भवति ३० ।
    पात्रम् अङ्कः ३ अन्तः भवति ४० ।
    चरः स्थितिः ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    स्थितिः अङ्कः ० अन्तः भवति ९९९ ।
    चरः अवगणनक ॱॱ न६४ भवति अष्टकॱपत्रलेखनम् आरभ्य मार्गः ऽ पात्रम् ऽ स्थितिः समाप्तम् ।
    चरः लिखितम् ॱॱ न६४ भवति स्थितिः अङ्कः ० अन्तः ।
    यदि लिखितम् अधिकम् ४ आदि
        प्रत्यागमनम् लिखितम् ।
    इति
    ॰ AND BACK. A fresh buffer, so nothing the write left behind can be read.
    चरः पुनःपात्रम् ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् ३२ आदि
        पुनःपात्रम् अङ्कः क्रमः अन्तः भवति ० ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    स्थितिः अङ्कः ० अन्तः भवति ९९९ ।
    चरः अवगणनख ॱॱ न६४ भवति अष्टकॱपत्रम् आरभ्य मार्गः ऽ पुनःपात्रम् ऽ स्थितिः समाप्तम् ।
    चरः गणना ॱॱ न६४ भवति स्थितिः अङ्कः ० अन्तः ।
    यदि गणना अधिकम् ३२ आदि
        प्रत्यागमनम् गणना ।
    इति
    चरः योगफलम् ॱॱ न६४ भवति ० ।
    चरः ज ॱॱ न६४ भवति ० ।
    यावत् ज न्यूनम् गणना आदि
        योगफलम् भवति योगफलम् योगः पुनःपात्रम् अङ्कः ज अन्तः ।
        ज भवति ज योगः १ ।
    इति
    प्रत्यागमनम् योगफलम् ।
इति
";

#[test]
fn a_sassembly_program_writes_a_file_and_reads_it_back() {
    let dir = std::env::temp_dir().join(format!(
        // pid AND a clock: a pid is REUSED and these roots are never
        // removed, so a pid-only name is unique inside this process and
        // not on disk (W-301).
        "patra-rt-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let target = dir.join("खपत्रम्");
    std::fs::remove_file(&target).ok();

    let image = build(ROUND_TRIP, "लेखकः");
    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    m.patra_root = Some(dir.clone());

    let mut out: Vec<u8> = Vec::new();
    let status = match m.run(400_000_000, &mut out) {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => s,
        other => panic!("the program did not finish: {other:?}"),
    };

    assert_ne!(
        status, 999,
        "a sentinel survived — one of the two calls was never served"
    );
    assert_eq!(
        status, 100,
        "the program wrote १०+२०+३०+४० and read back {status}. 0 means the write          reported success and produced nothing; anything else means the two          intrinsics disagree about the layout they share"
    );

    // AND THE HOST'S OWN VIEW, asserted separately so a disagreement between
    // what the program saw and what is on disk is named as that.
    let on_disk = std::fs::read(&target).expect("the file the program named exists");
    assert_eq!(
        on_disk,
        vec![10u8, 20, 30, 40],
        "the program read back the right sum from a file whose contents are {on_disk:?}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Writes the six octets of `कख` to the console, one call per octet.
///
/// **DEVANAGARI AND NOT ASCII, AND THAT IS NOT CONVENTION.** An earlier version
/// of this fixture used `SAS`; the NATIVE path built and printed it, and the
/// INTERPRETED path refused to load the module at all — "`S` is outside the doc
/// 15 repertoire". The two engines receive the source differently: `build`
/// passes it as OCTETS to the `.t1` compiler, whose lexer accepts it, while
/// `Interpreter::load` lexes it as a MODULE with the Rust front end, which does
/// not. A twin gate must be written in the language BOTH accept or it measures
/// the lexers instead of the channel.
///
/// `कख` is six octets because Devanagari is three per letter, which is why the
/// loop bound is ६ rather than the letter count.
///
/// The loop bound is the literal `३` rather than a length expression, so the
/// only construct under test is the console write itself. `उपेक्षा` holds the
/// call's result because a call is an expression here and the corpus's own
/// fixtures always bind one.
const PRINTS: &str = "मण्डलम् मुद्रकः ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः पाठः ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् कख इति ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    चरः उपेक्षा ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् ६ आदि
        उपेक्षा भवति अष्टकॱमुद्रणम् आरभ्य पाठः अङ्कः क्रमः अन्तः समाप्तम् ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    प्रत्यागमनम् ० ।
इति
";

/// **A COMPILED PROGRAM'S TEXT REACHES THE CONSOLE — asserted on the NATIVE
/// path, which nothing asserted before.** SAS-012.
///
/// # Why this was missing, and why the gap was invisible
///
/// `t1_output_channel.rs` holds nineteen tests on this channel and every one of
/// them runs through `Interpreter`: they test the INTERCEPTION, where the
/// interpreter recognises `अष्टकॱमुद्रणम्` and fills a sink itself. The native
/// path is a different mechanism entirely — `ir.t1` sets `मुद्रणमिदम्` (:2333,
/// :2403) and lowers the call to a STORE to `yantra::UART` (:2597), and nothing
/// in the tree checked that the store arrives.
///
/// That is the shape this project has been caught by repeatedly: the interpreter
/// executing a source hides a defect in the compiler's output. Nineteen green
/// tests said the channel worked; none of them ran the compiler's own emission.
///
/// # What is asserted, and what each assertion rules out
///
/// The expected octets are taken FROM THE SOURCE STRING rather than written out
/// again, so the fixture and the expectation cannot drift apart — change the
/// literal and the assertion follows.
///
/// `out` is not merely compared: it is first asserted NON-EMPTY, because a
/// channel that emitted nothing would satisfy a subset comparison and read as a
/// pass. And the halt is checked, because a program that faulted before its loop
/// would also write nothing.
/// **THE DIRECT TWIN GATE: one source, both engines, the same octets.** SAS-012
/// asked for this and it did not exist.
///
/// The two paths are genuinely different mechanisms, not two spellings of one:
///
/// * INTERPRETED — `nirvahana.rs:2035` recognises `अष्टकॱमुद्रणम्` and fills
///   `Interpreter::sink()` itself. The body at `ashtaka.t1:204` never runs.
/// * NATIVE — `ir.t1` sets `मुद्रणमिदम्` (:2333, :2403) and lowers the call to a
///   STORE to `yantra::UART` (:2597). The emulator's store arm catches it.
///
/// `t1_output_channel.rs` has nineteen tests on the first and, until SAS-012,
/// nothing had run the second. Each half being separately green is weaker than
/// this: two implementations can each satisfy their own test and still disagree,
/// and agreement is the property the row actually wanted.
///
/// # What is NOT asserted here, and why
///
/// `t1_output_channel` checks that an intercepted call answers `०` while
/// `268435456` means the body was REACHED — a sharp falsifier for a lowering
/// that stopped firing. **It does not transfer to the native side.** The
/// finisher's status is SIXTEEN BITS and `268435456 & 0xFFFF == 0`, so the device
/// address truncates to exactly the value it would need to be distinguished
/// from. Returning it would look like a strengthening and check nothing, so this
/// program returns `०` and the octets carry the whole claim.
#[test]
fn both_engines_write_the_same_console_octets_for_one_source() {
    // NATIVE first, so a failure here is attributed before the twin is involved.
    let image = build(PRINTS, "मुद्रकः");
    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let mut native: Vec<u8> = Vec::new();
    let halt = m.run(400_000_000, &mut native);
    match halt {
        yantra::Halt::Finisher {
            status: Some(0), ..
        } => {}
        other => panic!("the native run did not end cleanly: {other:?}"),
    }

    // INTERPRETED: the same source, loaded beside the chain that carries
    // `ashtaka.t1` (chain.rs:114) so the interception has something to intercept.
    let mut chain: Vec<(&str, &str)> = CHAIN.to_vec();
    chain.push(("मुद्रकः.t1", PRINTS));
    let mut it = Interpreter::load(&chain, &spec_root()).expect("the chain and the program load");
    it.call("मुद्रकःॱमुख्यम्", vec![], FUEL)
        .expect("the program runs interpreted");
    let interpreted = it.sink().to_vec();

    // NEITHER MAY BE EMPTY. Two empty sinks are equal, and that comparison would
    // pass while both channels were dead — the failure this whole file exists to
    // make impossible to report as coverage.
    assert!(
        !native.is_empty(),
        "the NATIVE console sink is empty; the store to yantra::UART did not arrive"
    );
    assert!(
        !interpreted.is_empty(),
        "the INTERPRETED sink is empty; the interception did not fire"
    );
    assert_eq!(
        interpreted,
        native,
        "THE TWO ENGINES DISAGREE on one source. interpreted {:?} vs native {:?} \
         — the interception and the `ir.t1` store lowering are emitting different \
         octets for the same program",
        String::from_utf8_lossy(&interpreted),
        String::from_utf8_lossy(&native)
    );
}

#[test]
fn a_compiled_program_writes_text_to_the_console() {
    // FROM THE SOURCE, so the two cannot drift. `उक्तम् … इति` delimits the
    // literal, and this fixture's is plain ASCII by construction.
    let lit = PRINTS
        .split("उक्तम् ")
        .nth(1)
        .and_then(|t| t.split(" इति").next())
        .expect("the fixture declares one `उक्तम् … इति` literal");
    assert_eq!(lit, "कख", "the fixture's literal moved");
    assert_eq!(
        lit.len(),
        6,
        "the loop bound in the fixture is the OCTET count, and Devanagari is \
         three octets per letter — if the literal changes, ६ must change with it"
    );

    let image = build(PRINTS, "मुद्रकः");
    assert!(!image.is_empty(), "the program built no image");

    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(400_000_000, &mut out);

    // THE HALT FIRST: a fault before the loop writes nothing, and "wrote
    // nothing" must not be reportable as "wrote the right thing".
    match halt {
        yantra::Halt::Finisher {
            status: Some(0), ..
        } => {}
        other => panic!(
            "the program did not end cleanly: {other:?} — the console octets \
             below say nothing until it does"
        ),
    }

    assert!(
        !out.is_empty(),
        "the console sink is EMPTY. The program halted cleanly, so it ran — the \
         store to yantra::UART that `ir.t1` lowers `अष्टकॱमुद्रणम्` to did not \
         arrive"
    );
    assert_eq!(
        out,
        lit.as_bytes(),
        "the console octets are not the source's literal: got {:?}, want {:?}",
        String::from_utf8_lossy(&out),
        lit
    );
}
