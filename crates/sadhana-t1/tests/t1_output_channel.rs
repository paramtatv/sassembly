//! THE OUTPUT CHANNEL'S ACCEPTANCE — asserted on BYTES, not on a return value.
//!
//! # Why this file exists rather than a mutation control
//!
//! `अष्टकॱमुद्रणम्` is declared in `ashtaka.t1` with a body that is never meant to
//! run: the interpreter intercepts the call in `nirvahana.rs`, and natively
//! `ir.t1` replaces it with a store at the device address. A body no path
//! reaches is a **claim that it is unreachable**, and the obvious guard for such
//! a claim is a mutation control — remove an interception, observe a failure.
//!
//! **That is the species that rots.** On 2026-09-14 six dead mutation controls
//! were found in two batteries, each mutating a routine nothing on the path
//! called any more, each passing by proving nothing. They died of DRIFT: the
//! control's subject and the readers' subject became different names with
//! nothing tying them together.
//!
//! So the guard here is not a control at all. It is the feature's own
//! acceptance, and it **cannot drift**, because its subject is the output
//! channel — the thing the feature exists to fill. Rename anything you like: if
//! the bytes stop arriving, this fails.
//!
//! # What it asserts, and what it deliberately does not
//!
//! It reads `Interpreter::sink()` and nothing else. Asserting on the intrinsic's
//! RETURN VALUE or on an exit status would pass while nothing reached the sink —
//! which is the one silent case the marker body cannot make loud:
//!
//! ```text
//! interception gone, path exercised, output OBSERVED    -> caught at point of use
//! interception gone, path exercised, output UNOBSERVED  -> the only silent case
//! interception gone, path unexercised                   -> benign, nothing depends on it
//! ```
//!
//! This file removes the silent case by being the observer.
//!
//! # The contract, pinned by both halves
//!
//! ```text
//! routine          सार्वजनिक वृत्तिः मुद्रणम् आदाय अष्टकम् ॱॱ अ८ ददाति न६४
//! qualified call   अष्टकॱमुद्रणम्          (the UNqualified form must not match)
//! intercepted      returns ०
//! body reached     returns २६८४३५४५६ = 0x10000000, the device address itself
//! ```
//!
//! The body returns the device address so that a caller sees `०` in every
//! correct configuration and `268435456` exactly when the wire is broken —
//! unmistakable, and it names its own cause. A body returning `०` would be
//! indistinguishable from success, which is the ambiguity that has twice forced
//! an answer to be offset (`१०००`+kind, `२०००`+length) elsewhere in this tree.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana-t1 has a grandparent")
        .to_path_buf()
}

fn source(name: &str) -> String {
    let p = repo_root().join("crates/sadhana-t1/src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} must be readable: {e}", p.display()))
}

fn spec_root() -> PathBuf {
    repo_root().join("spec")
}

/// A scratch module that calls the intrinsic with three known octets.
///
/// `अष्टक` has ZERO imports and nothing cycles through it, which is why the
/// intrinsic lives there; a caller therefore needs only its own declaration and
/// `अष्टक` itself.
const CALLER: &str = "\
मण्डलम् मुद्रणपरीक्षा ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः लेखनम् ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति अष्टकॱमुद्रणम् ७२ ।
    चरः ख ॱॱ न६४ भवति अष्टकॱमुद्रणम् ७३ ।
    चरः ग ॱॱ न६४ भवति अष्टकॱमुद्रणम् ७४ ।
    प्रत्यागमनम् क योगः ख योगः ग ।
इति
";

fn load_caller() -> Interpreter {
    let ashtaka = source("ashtaka.t1");
    Interpreter::load(
        &[("ashtaka.t1", ashtaka.as_str()), ("caller.t1", CALLER)],
        &spec_root(),
    )
    .expect("अष्टक and the caller load together")
}

/// **THE ACCEPTANCE.** The octets a program writes arrive in the sink, in order.
///
/// This is the whole of the channel's contract on the interpreted side. It fails
/// before the interception exists, passes once it does, and fails again the day
/// anyone removes it — without knowing anything about how the interception is
/// implemented.
#[test]
fn the_octets_a_program_writes_arrive_in_the_sink_in_order() {
    let mut it = load_caller();
    let answer = it
        .call("मुद्रणपरीक्षाॱलेखनम्", vec![], 5_000_000)
        .expect("the caller runs");

    assert_eq!(
        it.sink(),
        b"HIJ",
        "the three octets written (72, 73, 74) must reach the sink in order; \
         got {:?}. An empty sink means NOTHING was emitted — the channel is not \
         wired, and no return value or exit status would have said so.",
        it.sink()
    );

    // The return is the secondary check, and it is stated as secondary on
    // purpose: it can pass while the sink stays empty, which is exactly the
    // failure this file exists to catch.
    assert_eq!(
        answer,
        Value::Int(0),
        "an intercepted call answers ०; `268435456` means the body was REACHED, \
         i.e. neither the interpreter nor the lowering intercepted it"
    );
}

/// THE INTERCEPTION ALONE, against a synthetic `अष्टक`.
///
/// Separates the two halves of the contract so a failure says WHICH is missing.
/// Without this, "the sink is empty" is equally consistent with a broken
/// interception and an absent declaration, and a reader would have to bisect two
/// people's work to find out. It also means my half is verified before the other
/// half exists, rather than both landing on one untested green.
#[test]
fn the_interception_fills_the_sink_without_the_real_ashtaka() {
    const SYNTHETIC_ASHTAKA: &str = "\
मण्डलम् अष्टक ॥

सार्वजनिक वृत्तिः मुद्रणम् आदाय अष्टकम् ॱॱ अ८ ददाति न६४ आदि
    प्रत्यागमनम् २६८४३५४५६ ।
इति
";
    let mut it = Interpreter::load(
        &[("ashtaka.t1", SYNTHETIC_ASHTAKA), ("caller.t1", CALLER)],
        &spec_root(),
    )
    .expect("the synthetic अष्टक and the caller load together");

    let answer = it
        .call("मुद्रणपरीक्षाॱलेखनम्", vec![], 5_000_000)
        .expect("the caller runs");

    assert_eq!(
        it.sink(),
        b"HIJ",
        "the INTERPRETER's interception must fill the sink; got {:?}",
        it.sink()
    );
    assert_eq!(
        answer,
        Value::Int(0),
        "intercepted, so ० — not the device address, which would mean the \
         synthetic body ran and the interception did not"
    );
}

/// The marker body's value is what a broken wire looks like, and it is loud.
///
/// Not a mutation control — it asserts the CONTRACT's constant, so that the
/// number a caller would see is written down somewhere a reader will find it.
/// If `ashtaka.t1`'s body ever returns something else, this says so.
#[test]
fn the_marker_body_returns_the_device_address_so_a_broken_wire_is_unmistakable() {
    let ashtaka = source("ashtaka.t1");
    assert!(
        ashtaka.contains("मुद्रणम्"),
        "`अष्टक` must declare `मुद्रणम्` — the intrinsic's declaration is half of \
         the contract, and the interception is the other half"
    );
    // 0x10000000. No legitimate answer from "write one octet" resembles it,
    // where ० is the most plausible number in the language.
    assert!(
        ashtaka.contains("२६८४३५४५६") || ashtaka.contains("०x१००००००० "),
        "the marker body must return the DEVICE ADDRESS (268435456), not ०: a \
         caller that gets it back knows both interceptions are gone, and knows \
         where to look. A body returning ० is indistinguishable from success."
    );
}

/// The UNqualified form must not be captured.
///
/// The intrinsic is a contract about a cross-module call. Matching a bare
/// `मुद्रणम्` would silently capture any same-named routine `अष्टक` grew later —
/// a name collision that compiles clean and changes behaviour, which this tree
/// has already had once today.
#[test]
fn an_unqualified_call_is_not_the_intrinsic() {
    let src = std::fs::read_to_string(repo_root().join("crates/sadhana/src/t1/nirvahana.rs"))
        .expect("nirvahana.rs must be readable");
    assert!(
        src.contains(r#"m.as_deref() == Some("अष्टक")"#),
        "the interception must require the QUALIFIED module, so that an \
         unqualified call from inside `अष्टक` itself does not match"
    );
}

/// Turn a `&str` into the octet run the `.t1` routines take.
fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

/// **THE CONSUMER'S ACCEPTANCE — the octets that arrive are the object's, ALL of them.**
///
/// The channel landed unused: `ir.t1` lowers the call, the interpreter
/// intercepts it, and nothing in the corpus made the call. This drives
/// `शृङ्खलाॱवस्तुपाठ्यमुद्रणम्`, the first caller, and asserts the payload.
///
/// # Why the assertion is EQUALITY WITH THE OBJECT and not "the sink is non-empty"
///
/// The channel is unframed — one octet per call, no length, no terminator — so
/// a write that stops part way leaves a PREFIX, and a prefix of an object is
/// byte for byte a shorter object. `assert!(!sink.is_empty())` passes on one
/// octet of ninety thousand, and Stage B would then report a first-differing
/// offset that looks exactly like a finding about the compiler.
///
/// So the reference is the object's own `पाठ्यम्`, read from the record through
/// a SECOND call, and the comparison is byte for byte. That is the only form
/// of this test that can see a short write.
#[test]
fn the_consumer_writes_the_whole_object_text_and_nothing_else() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");

    // The front half: a real module's source to Sassembly text. `अष्टक` because
    // it has zero imports, so nothing about this depends on a load order.
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![
                octets(source("ashtaka.t1").as_bytes()),
                octets("अष्टक".as_bytes()),
            ],
            80_000_000_000,
        )
        .expect("मण्डलसङ्कलनम् runs");
    let text = text.octets().expect("मण्डलसङ्कलनम् answers octets");
    let text = text.as_slice().to_vec();
    assert!(
        !text.is_empty(),
        "the front half emitted nothing, so this test would be measuring an \
         empty object rather than the channel"
    );

    // THE REFERENCE, through the back half directly. `सङ्कलनारम्भः` first, as
    // every caller of the builder must: the front half above left `पदविभाग`
    // holding its tokens.
    it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], 8_000_000_000)
        .expect("सङ्कलनारम्भः runs");
    let obj = it
        .call("शृङ्खलाॱपाठवस्तुरचना", vec![octets(&text)], 40_000_000_000)
        .expect("पाठवस्तुरचना runs");
    let want: Vec<u8> = match &obj {
        Value::Record(r) => r
            .borrow()
            .get("पाठ्यम्")
            .and_then(|v| v.octets().map(|o| o.as_slice().to_vec()))
            .expect("the object carries पाठ्यम्"),
        other => panic!(
            "`पाठवस्तुरचना` refused on `अष्टक`'s own text ({other:?}); the \
             consumer cannot be measured against an object that was not built"
        ),
    };
    assert!(
        !want.is_empty(),
        "the object's text is empty, so equality below would be vacuous — the \
         both-sides-zero trap"
    );
    // The payload Stage B compares, named so the figure is recoverable. NOT
    // pinned: it moves with every lowering that reaches `ashtaka.t1`, and a pin
    // here would fail for reasons that have nothing to do with the channel.
    println!("METRIC t1_channel_object_text_octets {}", want.len());
    println!("METRIC t1_channel_emitted_text_octets {}", text.len());

    // THE CONSUMER. Its own reset runs inside it, which is the point of putting
    // the reset there: this call needs no ceremony from its caller.
    let answer = it
        .call("शृङ्खलाॱवस्तुपाठ्यमुद्रणम्", vec![octets(&text)], 40_000_000_000)
        .expect("वस्तुपाठ्यमुद्रणम् runs");

    assert_eq!(
        answer,
        Value::Int(0),
        "० means the whole object was written; १००x carries the builder's own \
         exit code (`वस्तुरचनाविरामभेद`), and १००३ is the caller failing to \
         tell an EMPTY object from a refusal — fixed at 82be3d5d, and NOT \
         clause nine, which is the गणना variant lowering"
    );
    // THE LENGTH, IN THREE STATES AND NOT TWO. This guard was written for the
    // SHORT WRITE — the consumer stopping early, which the octet stream cannot
    // show you. But `!=` has a second side, and on 2026-09-19 that is the side
    // that fired: fourteen debug markers committed into `encode.t1` by
    // `40b9c34a` put 84912 octets into the sink where the object's text is
    // 2928, and the message still said "a SHORT WRITE is the failure this test
    // exists for". A twenty-nine-fold OVER-write reported as a short write
    // sends the reader looking at the consumer's loop, which is correct code.
    // So the two directions are named separately, and each says where to look.
    let (got, want_len) = (it.sink().len(), want.len());
    match got.cmp(&want_len) {
        std::cmp::Ordering::Less => panic!(
            "SHORT WRITE: the sink holds {got} octets where the object's text \
             is {want_len}. The consumer stopped early — this is the failure \
             this test exists for, and it is invisible in the octet stream \
             itself. Look at `वस्तुपाठ्यमुद्रणम्`'s loop bound."
        ),
        std::cmp::Ordering::Greater => panic!(
            "OVER-WRITE: the sink holds {got} octets where the object's text \
             is {want_len} — {extra} octets that are not the object's. The \
             consumer did not stop early; something ELSE wrote into the same \
             channel. First suspect: a debug marker left in a product `.t1` \
             source. A `चरः ... भवति चतुरष्टकमुद्रणम् <n>` line writes four \
             octets per call into THIS sink, and `40b9c34a` shipped fourteen \
             of them in `encode.t1`. Census the corpus for `चतुरष्टकमुद्रणम्` \
             calls whose result is never read before blaming the consumer.",
            extra = got - want_len
        ),
        std::cmp::Ordering::Equal => {}
    }
    assert_eq!(
        it.sink(),
        want.as_slice(),
        "the octets that arrived are not the object's text"
    );
}

/// The module rung 73 uses, whose text reaches E22 natively.
// **THE `इति` IS THE LITERAL'S TERMINATOR, NOT PART OF THE SOURCE.** Rung 76
// writes `उक्तम् मण्डलम् क ॥ … भवति ७ । इति ।`, and a first version of this
// constant copied the whole tail into the string. The front half then emits
// text that splits into ZERO statements and `पाठवस्तुरचना` answers exit १
// (`वस्तुरचनावाक्यभेद`) — an empty sink that looks exactly like a dump site
// that was never reached.
const TINY: &str = "मण्डलम् क ॥ सार्वजनिक चरः ख ॱॱ न६४ भवति ७ ।";

/// Drive `पाठवस्तुरचना` over `TINY`, returning the sink.
fn build_tiny(dump: bool) -> (Interpreter, Value) {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(TINY.as_bytes()), octets("क".as_bytes())],
            80_000_000_000,
        )
        .expect("मण्डलसङ्कलनम् runs")
        .octets()
        .expect("octets")
        .as_slice()
        .to_vec();
    // THE SWITCH GOES AFTER THE RESET, and the first version put it before.
    // `सङ्कलनारम्भः` clears `सङ्केतन`'s module state, so a flag set ahead of it is
    // cleared with everything else and the sink comes back EMPTY — which reads
    // exactly like "the dump site is never reached", a different conclusion
    // about a different routine.
    it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], 8_000_000_000)
        .expect("सङ्कलनारम्भः runs");
    if dump {
        it.call("सङ्केतनॱविन्यासमुद्रणारम्भः", vec![Value::Bool(true)], 1_000_000)
            .expect("the switch is a routine, reachable from a native rung too");
        // READ IT BACK. Without this, "the sink is empty" cannot tell a switch
        // that did not take from a site that was not reached.
        assert_eq!(
            it.global("विन्यासमुद्रणार्हम्"),
            Some(&Value::Bool(true)),
            "the switch did not take — nothing below is about the dump site"
        );
    }
    let obj = it
        .call("शृङ्खलाॱपाठवस्तुरचना", vec![octets(&text)], 40_000_000_000)
        .expect("पाठवस्तुरचना runs");
    (it, obj)
}

/// Split the framed stream: each array is a little-endian u32 count then that
/// many little-endian u32 elements.
fn decode_items(sink: &[u8]) -> Vec<Vec<u32>> {
    let word =
        |s: &[u8], i: usize| -> u32 { u32::from_le_bytes([s[i], s[i + 1], s[i + 2], s[i + 3]]) };
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < sink.len() {
        assert!(
            at + 4 <= sink.len(),
            "the stream ended before an array's COUNT at octet {at} of {} — this \
             is the short-write case the length prefix exists to name",
            sink.len()
        );
        let n = word(sink, at) as usize;
        at += 4;
        assert!(
            at + 4 * n <= sink.len(),
            "an array announced {n} elements and the stream holds {} more octets",
            sink.len() - at
        );
        out.push((0..n).map(|k| word(sink, at + 4 * k)).collect::<Vec<u32>>());
        at += 4 * n;
    }
    out
}

/// Find the pair tagged `९१०` — the two layout arrays and the comparator.
///
/// **BY TAG, NOT BY POSITION.** These were indexed as `items[0..3]`, and adding a
/// diagnostic emit UPSTREAM in `स्थानविन्यासः` silently re-pointed them at a
/// marker. A reader that finds its payload by counting from the start of a
/// stream breaks whenever anything earlier learns to speak.
fn tagged_layouts(sink: &[u8]) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let items = decode_items(sink);
    let at = items
        .iter()
        .position(|i| i.len() == 1 && i[0] == 910)
        .unwrap_or_else(|| panic!("no ९१० tag in a {}-item stream: {items:?}", items.len()));
    (
        items[at + 1].clone(),
        items[at + 2].clone(),
        items[at + 3].clone(),
    )
}

/// **THE CLAUSE-NINE INSTRUMENT'S ACCEPTANCE, and its safety claim.**
///
/// Two things, and the second is the one that protects every reading already
/// taken: with the switch OFF not one octet is written, so the instrumented
/// tree measures exactly what the uninstrumented one did.
#[test]
fn the_layout_dump_is_silent_until_asked_and_then_frames_two_arrays() {
    // OFF — the claim that this costs nothing.
    let (quiet, obj_quiet) = build_tiny(false);
    assert!(
        quiet.sink().is_empty(),
        "the dump wrote {} octets with the switch OFF; every existing reading on \
         this path is then taken through a changed routine",
        quiet.sink().len()
    );

    // ON — the same path, and the object must still build exactly as before.
    let (loud, obj_loud) = build_tiny(true);
    assert_eq!(
        matches!(obj_quiet, Value::Nil),
        matches!(obj_loud, Value::Nil),
        "the dump CHANGED whether the object built — then the dump is the story \
         and nothing it prints is about clause nine"
    );

    // THE BUILDER'S OWN EXIT, BEFORE ANY CLAIM ABOUT THE SINK. An empty sink is
    // equally consistent with "the dump site was not reached" and "the object
    // path stopped earlier", and those want opposite fixes.
    println!(
        "METRIC t1_clause9_exit_interpreted {:?}",
        loud.global("वस्तुरचनाविरामभेद")
    );
    println!("METRIC t1_clause9_sink_octets {}", loud.sink().len());
    assert!(
        !matches!(obj_loud, Value::Nil),
        "`पाठवस्तुरचना` refused this exemplar INTERPRETED (exit {:?}), so the \
         dump site was never reached and the exemplar is wrong, not the dump",
        loud.global("वस्तुरचनाविरामभेद")
    );

    let (first, second, tuly) = tagged_layouts(loud.sink());
    // THE COMPARATOR, ASSERTED RATHER THAN PRINTED. Interpreted it must answer
    // सत्यम् on equal layouts; natively it does too, which is what established
    // that E22 never fires and the refusal is `उत्सर्जनक्रमः`. If this ever reads
    // [0] on equal arrays, the comparator itself is the defect.
    assert_eq!(
        tuly,
        vec![1],
        "`विन्याससाम्यम्` answered असत्यम् on layouts that agree — the comparator is \
         then the defect, not the emitter"
    );
    println!("METRIC t1_clause9_layout_pass1_elements {}", first.len());
    println!("METRIC t1_clause9_layout_pass2_elements {}", second.len());
    println!("  pass 1: {first:?}");
    println!("  pass 2: {second:?}");

    // INTERPRETED, THE TWO PASSES AGREE — that is rung 77's `६०११`, and it is
    // asserted here so the reference side of the comparison is pinned. A native
    // run that disagrees is then a difference against a KNOWN reference rather
    // than against an assumption.
    assert_eq!(
        first, second,
        "interpreted, the two layout passes must agree — this is the side that \
         does NOT raise E22, and it is the reference the native stream is read \
         against"
    );
}

/// **RUNG 79'S INTERPRETED READING, PINNED BEFORE THE NATIVE RUN.**
///
/// Registering the reference first is the whole point: a native `१००३` with two
/// differing arrays is then a difference against a MEASURED interpreted answer,
/// not against an assumption about what the interpreter would have said.
///
/// The status is the control and the octets are the payload. Rung 73 is the
/// same run with the dump off; if rung 79's status ever differs from rung 73's
/// on the same tree, the instrument perturbed what it measures.
#[test]
fn rung_79_interpreted_agrees_with_rung_73_and_emits_two_equal_layouts() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let quiet = it
        .call("शृङ्खलाॱस्वपरीक्षात्रिसप्ततिः", vec![], 80_000_000_000)
        .expect("rung 73 runs");
    assert!(
        it.sink().is_empty(),
        "rung 73 has the dump OFF and must write nothing"
    );

    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let loud = it
        .call("शृङ्खलाॱस्वपरीक्षैकोनाशीतिः", vec![], 80_000_000_000)
        .expect("rung 79 runs");

    println!("METRIC t1_rung73_interpreted {quiet:?}");
    println!("METRIC t1_rung79_interpreted {loud:?}");
    println!("METRIC t1_rung79_sink_octets {}", it.sink().len());

    assert_eq!(
        quiet, loud,
        "the dump MOVED the rung's own answer — then the instrument is the story"
    );
    let (first, second, tuly) = tagged_layouts(it.sink());
    // THE COMPARATOR, ASSERTED RATHER THAN PRINTED. Interpreted it must answer
    // सत्यम् on equal layouts; natively it does too, which is what established
    // that E22 never fires and the refusal is `उत्सर्जनक्रमः`. If this ever reads
    // [0] on equal arrays, the comparator itself is the defect.
    assert_eq!(
        tuly,
        vec![1],
        "`विन्याससाम्यम्` answered असत्यम् on layouts that agree — the comparator is \
         then the defect, not the emitter"
    );
    println!("  pass 1: {first:?}");
    println!("  pass 2: {second:?}");
    assert_eq!(
        first, second,
        "interpreted, the two passes agree; this is the reference the native \
         stream is read against"
    );
}

/// **THE INTERPRETED CONTROL FOR THE NATIVE `BadAccess`.**
///
/// The fast-path image faults `BadAccess { addr: 0 }` with zero octets on a
/// module carrying ten instructions, with the tables filled. That is either the
/// compiled back half or my scratch entry, and the two want opposite next steps.
/// This runs the SAME SEQUENCE interpreted. If it completes, the fault is
/// native; if it faults here too, the sequence is wrong and the image is
/// faithfully reproducing my own bug.
#[test]
fn the_back_half_completes_interpreted_on_a_module_with_instructions() {
    const WITH_CODE: &str = "मण्डलम् ग ॥\nसार्वजनिक वृत्तिः घ ददाति न६४ आदि\n    प्रत्यागमनम् ७ ।\nइति\n";
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(WITH_CODE.as_bytes()), octets("ग".as_bytes())],
            80_000_000_000,
        )
        .expect("मण्डलसङ्कलनम् runs")
        .octets()
        .expect("octets")
        .as_slice()
        .to_vec();
    println!("METRIC t1_withcode_emitted_octets {}", text.len());

    it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], 8_000_000_000)
        .expect("सङ्कलनारम्भः runs");
    let statements = it
        .call("वाक्यविभागॱसङ्कलनम्", vec![octets(&text)], 40_000_000_000)
        .expect("सङ्कलनम् runs");
    println!("METRIC t1_withcode_statements {statements:?}");
    let program = it
        .call("वाक्यविभागॱकार्यक्रमरचना", vec![], 8_000_000_000)
        .expect("कार्यक्रमरचना runs");
    let instrs = match &program {
        Value::Record(r) => r.borrow().get("आज्ञासंख्यान").cloned(),
        other => panic!("कार्यक्रम is a record, not {other:?}"),
    };
    println!("METRIC t1_withcode_instructions {instrs:?}");

    let obj = it
        .call("सङ्केतनॱवस्तुसङ्केतनम्", vec![program], 80_000_000_000)
        .expect(
            "वस्तुसङ्केतनम् must not FAULT interpreted — if it does, the \
                 native BadAccess is my sequence, not the compiler",
        );
    println!(
        "METRIC t1_withcode_object_is_nil {}",
        matches!(obj, Value::Nil)
    );
}

/// **THE LENGTH OF A NIL ARENA OF RECORDS IS ० ON BOTH SIDES — SINCE `W-355`.**
///
/// Until 2026-10-03 it was 1 on both sides, measured, and that refuted the
/// mechanism once proposed for the `BadAccess { addr: 0 }` in `स्थानविन्यासः`
/// (० interpreted against १ natively): the interpreter's zero run held one nil
/// element and `ir.t1`'s `खण्डदैर्घ्यरचना` added `(दैर्घ्यफलम् == ०)` to
/// reproduce it natively. The owner ruled LENGTH ० (`W-355`): `zero_at` gives
/// an empty arena and the native length is the header word, ० for the nil word.
/// `crates/yantra/tests/w355_fresh_run.rs` measures the native side; this pins
/// the interpreted one.
///
/// Pinned here because it is the shared premise of anything built on arena
/// lengths.
#[test]
fn a_nil_arena_of_records_reads_length_zero_on_both_sides() {
    const PROBE: &str = "\
मण्डलम् दैर्घ्यपरीक्षा ॥

सार्वजनिक संरचना बिन्दु आरभ्य
  क ॱॱ न६४
समाप्तम् ।

सार्वजनिक वृत्तिः रिक्तदैर्घ्यम् ददाति न६४ आदि
    चरः कोश ॱॱ अङ्कः अन्तः बिन्दु भवति ० ।
    प्रत्यागमनम् कोश ॱ दैर्घ्य ।
इति
";
    let mut it = Interpreter::load(&[("probe.t1", PROBE)], &spec_root()).expect("the probe loads");
    let n = it
        .call("दैर्घ्यपरीक्षाॱरिक्तदैर्घ्यम्", vec![], 5_000_000)
        .expect("रिक्तदैर्घ्यम् runs");
    println!("METRIC t1_nil_arena_length_interpreted {n:?}");
    assert_eq!(
        n,
        Value::Int(0),
        "a nil arena of records reads ० INTERPRETED since W-355 — a fresh run \
         is empty. १ is the old zero run of one nil; natively the same shape \
         reads ० too (w355_fresh_run.rs), and if the two ever come apart every \
         arena walk in the corpus is affected."
    );
}

/// **WHAT `प्रकारसंज्ञामूल्यानि` ACTUALLY HOLDS FOR AN ENUM VARIANT.**
///
/// A pre-check, not a finding. `artha.t1:250`'s margin says "SymbolId+१, the
/// same keying ... so ० stays none", which reads equally as a statement about
/// the KEY and about the VALUE. The fix for clause nine needs the variant's
/// POSITION, so which of those it is decides whether this table is the source
/// at all — and guessing between two readings of one margin is how five hours
/// went tonight.
#[test]
fn what_the_variant_value_table_holds() {
    const PROBE: &str = "\
मण्डलम् गणनपरीक्षा ॥

सार्वजनिक गणना दिशा आरभ्य
  पूर्वा ऽ
  पश्चिमा
समाप्तम् ।
";
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let decls = it
        .call("शृङ्खलाॱपठनम्", vec![octets(PROBE.as_bytes())], 40_000_000_000)
        .expect("पठनम्")
        .as_int()
        .unwrap_or(0);
    let ok = it
        .call("शृङ्खलाॱनिर्णयः", vec![Value::Int(decls)], 80_000_000_000)
        .expect("निर्णयः");
    println!("METRIC t1_enumprobe_declarations {decls}");
    println!("METRIC t1_enumprobe_resolved {ok:?}");

    // NAMES AS TEXT, not as byte arrays — the slots are what decide whether this
    // table keys TYPES or VARIANTS, and a byte dump does not say.
    if let Some(Value::Arena(a)) = it.global("प्रकारसंज्ञानामानि")
    {
        for (k, e) in a.borrow().iter().enumerate() {
            let t = e
                .octets()
                .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
                .unwrap_or_else(|| "-".into());
            println!("  slot {k}: {t}");
        }
    }
    for g in [
        "प्रकारसंज्ञानामानि",
        "प्रकारसंज्ञाभेदाः",
        "प्रकारसंज्ञामूल्यानि",
        "प्रकारसंज्ञासूचकाङ्क",
    ] {
        let v = it
            .global(g)
            .map(|v| format!("{v:?}"))
            .unwrap_or_else(|| "-".into());
        println!("  {g} = {}", &v[..v.len().min(300)]);
    }

    // **THE MEASURED ANSWER: every slot holds the TYPE name, not a variant.**
    // `प्रकारसंज्ञामूल्यानि` maps a type SPELLING to that type's symbol — twice,
    // bare and module-qualified, per `artha.t1:250`'s margin on bare spellings.
    // It does NOT hold a variant's position.
    //
    // That matters because this table was named as the source for the
    // clause-nine fix, which needs the POSITION. It is not the source. The
    // positions are gathered at `sanchaya.t1:291` (`भेदसङ्ग्रहः क्रमः` over the
    // declaration's parameter range); whether that index is reachable from a
    // variant's SYMBOL at `ir.t1:2458` is NOT established and is the next thing
    // to measure — before any edit, not after.
    let names: Vec<String> = match it.global("प्रकारसंज्ञानामानि")
    {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .filter_map(|e| {
                e.octets()
                    .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
            })
            .collect(),
        _ => panic!("`प्रकारसंज्ञानामानि` is an arena"),
    };
    // IS THE POSITION REACHABLE FROM THE VARIANT'S SYMBOL? That is the only
    // question the fix depends on, and it is the one nobody has measured.
    for g in ["संज्ञाघोषणाकोश", "संज्ञाभेदकोश", "संज्ञाप्रकारकोश"]
    {
        let v = it
            .global(g)
            .map(|v| format!("{v:?}"))
            .unwrap_or_else(|| "-".into());
        println!("  {g} = {}", &v[..v.len().min(200)]);
    }
    // and what the store holds per declaration entry
    for slot in 1..=4i128 {
        let name = it
            .call("घोषणासञ्चयॱप्रविष्टिनाम", vec![Value::Int(slot)], 8_000_000)
            .ok()
            .and_then(|v| {
                v.octets()
                    .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
            })
            .unwrap_or_default();
        let kind = it
            .call("घोषणासञ्चयॱप्रविष्टिभेदः", vec![Value::Int(slot)], 8_000_000)
            .ok()
            .and_then(|v| v.as_int())
            .unwrap_or(-1);
        let nparam = it
            .call(
                "घोषणासञ्चयॱप्रविष्टिप्राचलसंख्या",
                vec![Value::Int(slot)],
                8_000_000,
            )
            .ok()
            .and_then(|v| v.as_int())
            .unwrap_or(-1);
        if !name.is_empty() {
            println!("  store[{slot}] name={name} kind={kind} params={nparam}");
        }
    }

    assert!(
        !names.is_empty() && names.iter().all(|n| n == "दिशा"),
        "every slot must hold the TYPE name; a VARIANT name here would mean this \
         table does key variants and the fix can read it after all. got {names:?}"
    );
}

/// **THE NATIVE SIDE FAULTS, MEASURED 2026-09-18 — `BadAccess { addr: 0 }`.**
///
/// The reference below is interpreted. The same rung, compiled into a self-image
/// and run under `yantra`, does NOT reproduce it:
///
/// ```text
///   interpreted   status Int(0), 40 octets, 130101ff…67800000
///   native        halt: BadAccess { pc: 2147930364 (0x8006D0FC), addr: 0 }, 0 octets
///   build         t1_image --entry शृङ्खला स्वपरीक्षाशीतिः, 1,373,415 octets
///   native run    3,997 ms to the fault
/// ```
///
/// **AND THE BISECT CAME FREE, BECAUSE THE FIRST VERSION OF THE RUNG WAS WRONG.**
/// With an input carrying NO routine, the same image halted cleanly with status
/// ० and an empty stream. With one routine it faults. So the divergence is in
/// compiling a ROUTINE natively, not in the harness, the channel, or the entry.
///
/// A fault at address ० is the shape this corpus already knows: a refused target
/// lowers to a constant ० and the arm stores through it. It is NOT what
/// `वस्तुपाठ्यमुद्रणम्`'s margin predicted — that said १००३, a clean refusal —
/// so the compiled back half is not declining, it is running into a null.
///
/// This is the first native-vs-interpreted divergence reached through the
/// product path with a PAYLOAD rather than a scalar, which is what the rung was
/// built to find.
///
/// **RUNG 80 INTERPRETED — THE REFERENCE STREAM, REGISTERED BEFORE THE NATIVE RUN.**
///
/// Rung 80 is the first caller `वस्तुपाठ्यमुद्रणम्` has ever had: it compiles a
/// one-routine module and pushes the object's `.text` through `अष्टकॱमुद्रणम्`.
/// Every other rung answers a NUMBER, so native and interpreted are compared
/// through a scalar — equal numbers, unknown octets. This is the payload.
///
/// **THE STATUS ALONE CANNOT CARRY THE CLAIM, MEASURED.** The rung's first
/// version used an input with no routines, whose `.text` is legitimately empty.
/// It halted `Finisher { status: Some(0) }` having written ZERO octets, and ० is
/// exactly what a COMPLETE write answers. So this asserts the stream is NOT
/// EMPTY before anything else: a zero-length stream is the degenerate prefix and
/// it agrees with a zero-length native stream for the wrong reason.
#[test]
fn rung_80_interpreted_emits_the_objects_text_through_the_channel() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let status = it
        .call("शृङ्खलाॱस्वपरीक्षाशीतिः", vec![], 80_000_000_000)
        .expect("rung 80 runs");
    let octets = it.sink().to_vec();

    println!("METRIC t1_rung80_interpreted_status {status:?}");
    println!("METRIC t1_rung80_interpreted_octets {}", octets.len());
    let sum: u64 = octets.iter().map(|b| u64::from(*b)).sum();
    println!("METRIC t1_rung80_interpreted_octet_sum {sum}");
    // THE STREAM ITSELF, not a digest of it. A sum agrees under transposition
    // and a length agrees under substitution; the native side is compared
    // against THESE octets, so they are printed where a reader can take them.
    let hex: String = octets.iter().map(|b| format!("{b:02x}")).collect();
    println!("METRIC t1_rung80_interpreted_octets_hex {hex}");
    println!(
        "  as text: {}",
        String::from_utf8_lossy(&octets).escape_debug()
    );

    assert!(
        !octets.is_empty(),
        "the channel carried NOTHING. ० is what a complete write answers, so an \
         empty stream and a finished one are the same status — this is the \
         degenerate prefix the routine's own margin warns about"
    );
    assert_eq!(
        status,
        Value::Int(0),
        "the write did not complete: a non-zero answer is १००० plus the builder's \
         own exit code, and the stream above is then a PREFIX of an object"
    );
}

/// **THE BISECT'S INTERPRETED HALF.** Rung 80 faults natively with
/// `BadAccess { addr: 0 }` on an input carrying one routine, and halts cleanly
/// on the same input without one. Rungs 46 and 47 run only the FRONT half on
/// routine-carrying inputs — `मण्डलसङ्कलनम्`, source to Sassembly text, no
/// assembler and no channel. Their references are registered here so a native
/// run of either splits the compiler in two: front half or back half.
#[test]
fn rungs_46_and_47_interpreted_are_the_front_half_references() {
    for (name, rung) in [
        ("46", "शृङ्खलाॱस्वपरीक्षाषट्चत्वारिंशी"),
        ("47", "शृङ्खलाॱस्वपरीक्षासप्तचत्वारिंशी"),
    ] {
        let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
        let v = it
            .call(rung, vec![], 80_000_000_000)
            .expect("the rung runs");
        println!("METRIC t1_rung{name}_interpreted {v:?}");
        assert!(
            !matches!(v, Value::Int(0)),
            "rung {name} answered ०, so it emitted no text and cannot serve as a \
             front-half reference — the same empty-payload trap rung 80 hit"
        );
    }
}

/// **NATIVE RESULT, MEASURED 2026-09-18: rung 81 FAULTS TOO — the channel is
/// exonerated and the assembler is the site.**
///
/// ```text
///   rung 81 interpreted   Int(2040)
///   rung 81 native        BadAccess { pc: 2147930756, addr: 0 }
/// ```
///
/// Rung 81 makes no call to `अष्टकॱमुद्रणम्`, so the fault is not in the write.
/// `पाठवस्तुरचना` — the assembler — faults on a module that has instructions in
/// it, and is clean on one that does not (rung 73, २००० natively).
///
/// **AND THE TWO PCs SAY IT IS ONE SITE, NOT TWO.**
///
/// ```text
///   rung 80   pc 2147930364   image 1,373,415
///   rung 81   pc 2147930756   image 1,373,807
///             Δpc      392          Δimage 392
/// ```
///
/// The faulting address moved by exactly the amount the image grew, so the site
/// sits after the added rung in the layout and everything past it shifted
/// uniformly. Two runs, two different entries, one defect.
///
/// **RUNG 81 INTERPRETED — the back half on a routine, without the channel.**
///
/// The bisect so far: rung 47 answers 1694 NATIVELY and interpreted (front half
/// compiles a routine correctly); rung 73 answers २००० natively (back half is
/// clean on an input with no instructions); rung 80 faults natively with
/// `BadAccess { addr: 0 }` on front+back+channel with one routine. Rung 81 is
/// rung 79's shape with rung 47's input — `पाठवस्तुरचना` and the object's own
/// length, no `अष्टकॱमुद्रणम्` — so a native run separates the assembler from
/// the channel.
#[test]
fn rung_81_interpreted_is_the_back_half_reference() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let v = it
        .call("शृङ्खलाॱस्वपरीक्षैकाशीतिः", vec![], 80_000_000_000)
        .expect("rung 81 runs");
    println!("METRIC t1_rung81_interpreted {v:?}");
    // २००० + the object's `.text` length. Rung 80 measured that text as 40
    // octets interpreted, so २०४० is the expected shape — asserted as a RANGE
    // rather than a literal, because the number is the thing under test and a
    // literal here would pin the compiler's output to today's codegen.
    match v {
        Value::Int(n) => assert!(
            n > 2000,
            "rung 81 answered {n}: १००x is a DECLINE and २००० exactly is an \
             object with no text — neither is the back half encoding a routine"
        ),
        other => panic!("rung 81 answered {other:?}, not a number"),
    }
}

/// **RUNG 84 INTERPRETED — the encoder's own trace, as the reference prefix.**
///
/// Rung 83 established natively that parse, program construction and address
/// layout all succeed and `वस्तुसङ्केतनम्` faults. Rung 84 re-runs that with the
/// encoder's built-in dump on, so the stream carries the encoder's own tagged
/// records rather than markers of mine. The native run produces a PREFIX of what
/// this prints; the first record missing from it names where it died.
#[test]
fn rung_84_interpreted_is_the_encoder_trace_reference() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let status = it
        .call("शृङ्खलाॱस्वपरीक्षाचतुरशीतिः", vec![], 80_000_000_000)
        .expect("rung 84 runs");
    let octets = it.sink().to_vec();
    println!("METRIC t1_rung84_interpreted_status {status:?}");
    println!("METRIC t1_rung84_interpreted_octets {}", octets.len());
    let hex: String = octets.iter().map(|b| format!("{b:02x}")).collect();
    println!("METRIC t1_rung84_interpreted_hex {hex}");
    // The five stage letters lead the stream, then the encoder's records. The
    // letters are rung 79's control one level down: if turning the dump ON
    // changed them, the instrument moved what it measures.
    assert!(
        octets.starts_with(b"ABCDE"),
        "the stage markers are not the first five octets — the dump perturbed \
         the run, so the trace after them is not a reading of the same execution"
    );
    assert_eq!(
        status,
        Value::Int(2040),
        "rung 84 must answer what 81 and 83 answer; a different number means the \
         dump changed the encode, not just observed it"
    );
}

/// **RUNG 86 INTERPRETED — did the seed take?** Rung 85's pre-seed did not move
/// the native fault, which I first read as refuting the unconstructed-global
/// hypothesis. It does not: `सङ्केतनदोषरचना` is the very operation under
/// suspicion, so if construction into a global declared ० is the broken thing,
/// rung 85 measured the unseeded case twice. This seeds ४२४२, reads the field
/// straight back, and answers ३००० + what it read.
#[test]
fn rung_86_interpreted_seeds_and_reads_back() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let v = it
        .call("शृङ्खलाॱस्वपरीक्षाषडशीतिः", vec![], 80_000_000_000)
        .expect("rung 86 runs");
    println!("METRIC t1_rung86_interpreted {v:?}");
    assert_eq!(
        v,
        Value::Int(3000 + 4242),
        "interpreted, a seeded field must read back what was written — if this \
         fails the probe is wrong before the native run is even attempted"
    );
}

/// **RUNG 97: THE COMPILED COMPILER LOWERS A CALL — 2026-09-19, 20:57.**
///
/// ```text
///   halt:    Finisher { value: 204223283, status: Some(3116) }
///   (3116 << 16) | 0x3333 = 204223283
///   predict: interpreted -> 3116
///
///   marker-diff  interpreted 1352 records / native 1352 records
///                IDENTICAL — 1352 records agree
/// ```
///
/// The status is `3000 + len(object)`, so 3116 is 116 octets — 29 machine
/// instructions, the whole two-routine module. A CALL lowers natively and the
/// object matches the interpreter's octet for octet.
///
/// ```text
///                 routines  IR insts  calls   records identical   object
///   rung 84/105       1         1       0            586          40 octets
///   rung 97           2         7       1           1352         116 octets
/// ```
///
/// BOTH PREDICTIONS HELD, each recorded before its image existed. Rung 97's
/// named four falsifiers, including one that would have overturned a reading
/// relied on all day — a status BETWEEN 3000 and 3116 should be impossible,
/// since `उत्सर्जनक्रमः` returns `शून्यम्` on refusal and the object is all or
/// nothing. None fired.
///
/// AND THE STATUS ALONE WAS NOT ACCEPTED. The prediction said 3116 with
/// DIFFERING traces would mean the right length by coincidence, so the trace
/// was diffed too. 1352 records, identical.
///
/// **THE LADDER AFTER RUNG 97, SIZED — 2026-09-19.**
///
/// All six programs in `spec/demo/` pass through the RUST-hosted chain and
/// answer what their own source claims (120, 42, 5, 8, 55, 1 — read from a
/// `Halt::Finisher` in that process, not quoted). The compiled compiler has
/// been shown to lower NONE of them; rung 97 is the first attempt.
///
/// ```text
///   program          routines  blocks  IR insts  calls  machine   construct
///   fixture (84)         1        2        1       0      10     constant
///   द्विगुणम् (97)        2        4        7       1      29     a CALL
///   महत्तमम्              2        7       12       1      41     a BRANCH
///   योगपर्यन्तम्           1        5       16       0      29     a LOOP
///   क्रमगुणितम्            2        7       14       2      44     RECURSION
///   समविषमम्              2        8       11       1      39     branch-dense
/// ```
///
/// COMPLETED 2026-09-19 — the two rows left as dashes are filled, and they
/// change the ordering. `क्रमगुणितम्` is the only source with TWO calls, which
/// is what recursion looks like here. `समविषमम्` carries the MOST blocks of any
/// of the six (8) on the FEWEST instructions after the fixture (11), so it is
/// the most branch-dense and not, as its name suggested, a modulo test.
/// `योगपर्यन्तम्` remains the only LOOP and the only multi-block SINGLE routine.
///
/// The block count is what names the construct: `महत्तमम्` carries SEVEN blocks
/// across two routines (a `यदि` with both arms), `योगपर्यन्तम्` five in ONE
/// routine (a `यावत्`). Neither shape has been through the compiled emitter.
///
/// So the order is call, branch, loop, recursion — each one construct further,
/// each with a known answer to check against, and none of them larger than 16
/// IR instructions. `ashtaka.t1`, the smallest real module, is 388.
///
/// **RUNG 99 IS THE LOOP, AND IT IS WRITTEN — 2026-09-20.**
///
/// `शृङ्खलाॱस्वपरीक्षानवनवतिः` embeds `spec/demo/योगपर्यन्तम्.t1` with its names
/// shortened and answers `६००० + len(object)`. The prefix ६००० is free —
/// २०००, ३०००, ४००० and ५००० are taken — so an empty object reads ६००० and
/// cannot be confused with rung 97's ३११६ or rung 98's ४१६४.
///
/// ```text
///   predict: interpreted शृङ्खलाॱस्वपरीक्षानवनवतिः -> 6116
/// ```
///
/// 6116 − 6000 = 116 octets = 29 instructions × 4, which is exactly what
/// `tools/demo.sh` reports for the shortened source: 1 routine, 5 blocks, 16 IR
/// instructions, 0 calls, 29 machine instructions, and the machine halts with
/// status ५५ — १ + २ + … + १० — so the embedded module is the demo program and
/// not merely something the same size.
///
/// **WHAT THIS RUNG TESTS THAT 97 AND 98 DID NOT: A BACKWARD EDGE.** Rung 98's
/// `यदि` has both arms RETURN, so every block it makes is an exit; nothing
/// joins and nothing flows backwards. The five blocks here are entry, test,
/// body, join and exit, and the body jumps BACK to the test —
/// `लङ्घनम् शून्यःम् कघपर्व१य् ।` at the foot of पर्व२, a jump to a label the
/// object has already passed. It is also the only rung of this ladder with
/// ZERO calls, so it is control flow with no call underneath it.
///
/// NO NATIVE RUN YET. This records the rung and its interpreted reference; the
/// image's own answer is open, and the falsifiers are named in the `.t1`
/// margin — a status strictly between ६००० and ६११६ should be impossible,
/// since the encoder returns `शून्यम्` on refusal and the object is all or
/// nothing, and a status ABOVE ६११६ would mean a relaxation that did not
/// converge around the backward edge.
///
/// **RUNG 100 IS RECURSION, AND IT IS WRITTEN — 2026-09-20.**
///
/// **REFOUNDED AT THE 2026-09-21 TRUNK MERGE — READ THIS FIRST.** Two cycles
/// wrote this rung independently. The one that LANDED is
/// `शृङ्खलाॱस्वपरीक्षाशततमी` with the `८०००` prefix, answering **8176** and run
/// natively BYTE-IDENTICAL over 1990 records (`5935734a`). The routine this
/// paragraph describes, `शृङ्खलाॱस्वपरीक्षाशतम्` with `१००००`, came from a
/// salvaged cycle and was the DUPLICATE dropped at the merge — hand-splicing
/// both left `shrinkhala.t1` loading no routine at all. So the `10176` below
/// is this paragraph's own measurement of a routine that no longer exists.
///
/// **ITS ARGUMENT STILL STANDS AND IS NOT ANSWERED.** It chose `१००००` because
/// `८०००` is also rung 55's prefix, so a status in the 8000s cannot be read
/// alone as belonging to one rung. The landed rung carries exactly that
/// ambiguity. It is harmless while every rung is run by its entry name, and
/// it is the reason not to trust a bare status number here.
///
/// `शृङ्खलाॱस्वपरीक्षाशतम्` embeds `spec/demo/क्रमगुणितम्.t1` with its names
/// shortened and answers `१०००० + len(object)`. EVERY four-digit prefix on
/// this ladder is taken — २०००, ३०००, ४०००, ५००० and ६००० by the rungs above,
/// ७००० by rung 78, ८००० by rung 55, ९००० by rung 56 — so a five-digit prefix
/// is the first that cannot be read as another rung's answer, and `१००००`
/// appears nowhere else in `shrinkhala.t1` as a literal.
///
/// ```text
///   predict: interpreted शृङ्खलाॱस्वपरीक्षाशतम् -> 10176
/// ```
///
/// 10176 − 10000 = 176 octets = 44 instructions × 4. `tools/demo.sh` was run
/// over the SHORTENED source before it was embedded, not over the original, so
/// what the rung carries is provably this program: 46 tokens, 2 declarations
/// both routines, 4 numeral literals (4 non-zero), 2 routines / 7 blocks / 14
/// IR instructions / 2 calls (0 cross-module), 58 lines of T0, twins AGREE, 44
/// instructions in `.text`, and the machine halts with status १२० — which is
/// ५!. Every figure equals the original's.
///
/// **WHAT THIS RUNG TESTS THAT 97, 98 AND 99 DID NOT: A SELF-CALL.** Rung 97's
/// call is FORWARD and once — `घ` calls `ङ`, `ङ` returns, and no frame of `ङ`
/// is ever nested inside another of its own. Here `ङ` calls `ङ`, so the
/// compiled emitter must place a call whose target is the label it is
/// currently emitting, and size a frame that is re-entrant:
/// `लङ्घनम् पुनःस्थानम्म् कङय् ।` inside `कङ`'s own पर्व४. It is also the first
/// rung whose argument is live ACROSS a conditional and across a call — the
/// `यदि` returns १ on the base case and the recursive call sits in the join —
/// which is why `कङ` spills three saved registers where every rung before it
/// spilled at most one.
///
/// THE INTERPRETED TRACE IS TAKEN WITH THE PREDICTION, not left for the
/// native run to be compared against nothing:
///
/// ```text
///   --predict-sink  7966 octets, preamble 'ABCDE', 1990 records, 1 trailing
/// ```
///
/// 1990 records against rung 99's 1362 and rung 97's 1352. The sink costs 2.2 s
/// where the image costs ~34 minutes, so it is re-takeable on any tree.
///
/// NO NATIVE RUN YET. This records the rung and its interpreted reference; the
/// image's own answer is open, and the falsifiers are named in the `.t1`
/// margin — १०००० alone means an empty object; a status strictly between
/// १०००० and १०१७६ should be impossible, since the encoder returns `शून्यम्`
/// on refusal and the object is all or nothing; and a status ABOVE १०१७६ would
/// mean the compiled emitter placed more than the Rust chain did, which for a
/// self-call would most likely be a frame sized per call site rather than once.
///
/// **THE SCALE GAP, MEASURED RATHER THAN GESTURED AT — 2026-09-19.**
///
/// rung105 proves the compiled compiler emits a correct object for ONE
/// routine. `tools/demo.sh` over the smallest real module in the corpus says
/// what that is worth, Rust-side in about a second:
///
/// ```text
///                        routines   blocks   IR instructions
///   rung105's fixture         1        2            1
///   ashtaka.t1 (206 lines)   11       79          388
///   the corpus            21 modules, 38,014 lines
/// ```
///
/// The fixture's ONE IR instruction assembles to ten machine instructions —
/// prologue, epilogue and the return path dominate it — so "ten instructions"
/// overstates what was exercised. What the compiled back half has been shown to
/// handle is a single constant-returning routine.
///
/// `ashtaka.t1` is the SMALLEST module in the corpus and needs 388. Nothing
/// says the repairs scale; they were both measured on the fixture.
///
/// The demo's EMIT line reads REFUSED for that module and it is NOT a compiler
/// defect: the demo stub calls an entry with no arguments and `अष्टक`'s entry
/// takes two. A refusal from a harness is not a finding about the product.
///
/// **THE CORPUS STILL LINKS AFTER BOTH REPAIRS — 2026-09-19, 18:03.**
///
/// ```text
///   build:  1 source failed to compile (lib), 21 object(s) linked
///   write:  1,380,782 octets of ELF
///   halt:   Finisher { value: 21845, status: Some(0) }
///   ram:    high water 0
///
///   recorded 2026-09-18   1,373,231 octets
///   delta                    +7,551
/// ```
///
/// Predicted at 16:20 before the image existed — status 0, 21 objects, octets
/// MORE than 1,373,231 — and all three held. The growth is the repair's own
/// code: `मूल्याङ्कवाचकः` plus the two E23 value probes.
///
/// READ IT AS THE COMMIT BELOW SAYS. This status ० is the STARTUP STUB, and
/// `ram: high water 0` is the tell — nothing allocated, because nothing of the
/// compiler ran. What it establishes is that two repairs to `vishlesana.t1` and
/// `encode.t1` did not break compile, link or load of the whole corpus. That is
/// a regression check, and it is worth having; it is not the bootstrap.
///
/// **WHAT `status 0` ON THE 21-SOURCE IMAGE DOES AND DOES NOT MEAN.**
///
/// The recorded goal — 21 sources, 21 objects, 1,373,231 octets, halt
/// `Finisher { value: 21845, status: Some(0) }` — is built with NO `--entry`,
/// and `t1_image`'s own header says what that means: "With none named the
/// startup stub halts success, which is the image's RUN rung." So the ० comes
/// from the STUB. That build proves compile, link and load of 21 objects into
/// a loadable image, and nothing about the compiled compiler running.
///
/// `rung105`'s २०४० is the smaller-sounding number and the stronger result:
/// the entry ends `प्रत्यागमनम् २००० योगः अष्टकाः ॱ दैर्घ्य`, so the status IS
/// `2000 + len(object)`. It reports a compiled compiler that ran its own back
/// half and emitted forty octets. The two numbers are not on one ladder.
///
/// **THE COMPILED COMPILER EMITS THE WHOLE OBJECT — rung105, 2026-09-19.**
///
/// ```text
///   halt:    Finisher { value: 133706547, status: Some(2040) }
///   (2040 << 16) | 0x3333 = 133706547
///   predict: interpreted -> 2040
///
///   marker-diff  interpreted 586 records / native 586 records
///                IDENTICAL — 586 records agree
/// ```
///
/// The status is `2000 + len(object)`, so 2040 is FORTY OCTETS — the complete
/// ten-instruction program. `लेखः` and `विन्यस्तस्थानम्` now agree at every
/// step, 0 4 8 12 16 20 24 28 32 36; `९००३` is reached ten times where rung104
/// reached it three; and no `९००४` frame is emitted at all, so E23 never fires.
///
/// **AND THIS PROVES E02 WAS THE REFUSAL**, which `e713bc3a` could only infer.
/// E23 overwrites `अन्तिमसङ्केतनदोषः`, so the image's own last records read
/// `E23` and the earlier code was gone. Changing ONLY the E02 test — from
/// asking a one-word optional whether it is there to asking the text whether
/// the operand is a value — made all ten instructions emit. Proof by
/// intervention, where reading the sink could not reach it.
///
/// The progression across four images, every figure measured:
///
/// ```text
///   rung101/102   BadAccess 0x80009cbc   292 records agree    0 lowered
///   rung103/104   no fault, status 2000  374 records agree    3 lowered, 0 octets
///   rung105       status 2040            586 records IDENTICAL 10 lowered, 40 octets
/// ```
///
/// **INSTRUCTION २ IS THE FIRST WITH A ZERO OPERAND — named 2026-09-19.**
///
/// `tools/demo.sh` over the exact source the entry compiles, Rust-side, 0.8 s,
/// gives its ten instructions. The first three:
///
/// ```text
///   0  योगः स्तूपसूचकःम् स्तूपसूचकःन ऋण१६न    addi sp, sp, -16   imm -16   wrote 4
///   1  निधानम् स्तूपसूचकःय् ८न पुनःस्थानम्न    sd ra, 8(sp)       imm   8   wrote 4
///   2  निधानम् स्तूपसूचकःय् ०न स्थिर०न        sd s0, 0(sp)       imm   ०   wrote 0
/// ```
///
/// Instruction २ — the one rung104 measured as writing nothing — is the FIRST
/// whose immediate is ZERO. And `स्थानसङ्केतनम्` holds an operand's value in a
/// one-word optional and refuses it when it reads as absent:
///
/// ```text
///   encode.t1:4807
///     चरः मूल्याङ्कम् ॱॱ सम्भाव्य न६४ भवति मूल्याङ्कः मूलम् क्षेत्रयोग्यम् ।
///     ॰ E02
///     यदि मूल्याङ्कम् समम् शून्यम् आदि
///         सङ्केतनदोषरचना पङ्क्तिः २ पदार्थादिः १ ।
///         प्रत्यागमनम् शून्यम् ।
/// ```
///
/// `Some(०)` IS the nil word natively — measured at 7001 interpreted against
/// 7002 in the image — so an operand of ० reads as ABSENT and E02 refuses a
/// well-formed instruction. THE SAME DEFECT AS `न्यासरचना`, one layer over, and
/// `मूल्याङ्कः` is already one of the 25 producers in
/// `tools/optional-whether-census.py`.
///
/// NOT PROVEN: that instruction २'s refusal is E02 specifically rather than
/// another refusal in the same routine. E23 raises its own
/// `सङ्केतनदोषरचना ० २३ ० ०` afterwards, which overwrites
/// `अन्तिमसङ्केतनदोषः`, so the image's last records read `E23` and the earlier
/// code is gone. The falsifier is one emit of the code at the E02 site.
///
/// **2040 MINUS 2000 IS THE OBJECT'S LENGTH — decomposed 2026-09-19.**
///
/// `शृङ्खलाॱस्वपरीक्षाचतुरशीतिः` ends
///
/// ```text
///   प्रत्यागमनम् २००० योगः अष्टकाः ॱ दैर्घ्य ।
/// ```
///
/// so the status IS `2000 + len(object)`. Interpreted 2040 means FORTY octets —
/// ten instructions of four. Natively 2000 means **ZERO**: `वस्तुसङ्केतनम्`
/// hands back an empty run, because E23 refuses and `उत्सर्जनक्रमः` returns
/// `शून्यम्`. The difference was carried as "40, unexplained" for two commits;
/// it is the whole object.
///
/// The source it compiles is one small module, built from two literal pieces
/// (the split falls inside `इति`, which is why neither piece reads as a
/// terminator):
///
/// ```text
///   मण्डलम् क ॥ सार्वजनिक वृत्तिः घ ददाति न६४ आदि प्रत्यागमनम् ६ । इति
/// ```
///
/// **THE REFUSAL IS DATA-DEPENDENT, NOT BLANKET — rung104, 2026-09-19.**
///
/// `उत्सर्जनक्रमः`'s own margin asks the question this answers: "If EVERY
/// instruction trips this, the `दोषयुक्त` success wrapper does not survive
/// natively and the caller's nil test is always true — a blanket defect. If
/// only SOME do, it is data-dependent and the count says how many."
///
/// ```text
///   instruction   लेखः   विन्यस्तस्थानम्
///       0           0          0
///       1           4          4
///       2           8          8
///       3           8         12     <== E23
/// ```
///
/// ONLY SOME. Instruction ० advanced `लेखः` 0 -> 4 and instruction १ advanced
/// it 4 -> 8, so both wrote their four octets through the `अन्यथा` arm.
/// Instruction २ advanced it 8 -> 8: it wrote NOTHING, which only the
/// `यदि पदम् समम् शून्यम्` branch does. So the `दोषयुक्त` wrapper DOES survive
/// natively — it carried two instructions — and the defect is about instruction
/// २ in particular.
///
/// `विन्यस्तस्थानम्` is CORRECT throughout: 0, 4, 8, 12 is exactly the
/// interpreted series. The layout pass is not at fault; `लेखः` is four short.
///
/// **AND `९००३` MARKS ARRIVAL, NOT EMISSION.** It is printed immediately after
/// `चरः पदम् ... भवति स्थानसङ्केतनम् ...` and BEFORE the branch that writes, so
/// an instruction can print it and emit nothing. rung103's "three instructions
/// lowered" was read here as three instructions WRITTEN, and a prediction was
/// recorded on that reading; it was wrong and its own falsifier named this
/// exact outcome.
///
/// **THE FAULT IS GONE AND THE BACK HALF RUNS — rung103, 2026-09-19.**
///
/// ```text
///   halt:  Finisher { value: 131085107, status: Some(2000) }
///   (2000 << 16) | 0x3333 = 131085107
///   predict: interpreted -> 2040
/// ```
///
/// NOT `BadAccess`. The compiled compiler ran its back half to completion for
/// the first time; `0x80009cbc` is no longer reached. The repair was the pair
/// reader no longer asking a one-word optional whether it is there.
///
/// ```text
///   records agreeing        292 -> 374
///   9003 INSTRUCTION LOWERED  0 -> 3      (interpreted: 10)
///   9999 per-instruction      1 -> 4      (interpreted: 10)
/// ```
///
/// THE NEXT DEFECT IS NAMED BY WHAT WAS SKIPPED. At record 374 the interpreter
/// prints 9002 and the image does not. 9002 sits under
/// `यदि विन्यासमुद्रणार्हम् समम् सत्यम्`, which is on — every other marker in
/// this run proves it — so the only way past it without printing is the early
/// return between 9001 and 9002:
///
/// ```text
///   यदि लेखः असमम् विन्यस्तस्थानम् आदि
///       सङ्केतनदोषरचना ० २३ ० ० ।
///       प्रत्यागमनम् शून्यम् ।
///   इति
/// ```
///
/// E23 — the laid-out address disagrees with the written position, on the
/// FOURTH instruction. That block's own margin says the two passes "agree by
/// construction ... the E23 check above is what would say so if they did not".
/// Natively they do not.
///
/// **AND THE IMAGE NAMES THE CODE ITSELF — DECODED 2026-09-19, NOT INFERRED.**
/// The six records emitted in place of the interpreter's markers are
/// `1, 0, 3, 69, 50, 51`, and the last three are `सङ्केतनकूटः`'s output read
/// octet by octet. That routine builds its text by an exact formula:
///
/// ```text
///   कूटः[0] = ६९                       = 'E'
///   कूटः[1] = ४८ + (सङ्ख्या विभाजनम् १०)  = 48 + 2 = 50 = '2'
///   कूटः[2] = ४८ + (सङ्ख्या शेषः १०)      = 48 + 3 = 51 = '3'
/// ```
///
/// `सङ्ख्या = 23` is the only value that yields `69, 50, 51`. So the image
/// PRINTED "E23". The inference from the missing 9002 and the measurement now
/// agree, and `सङ्केतनदोषरचना` itself prints nothing — it records into
/// `अन्तिमसङ्केतनदोषः` and returns the code, so these octets come from whatever
/// reports the refusal, not from the raise site.
///
/// `1, 0, 3` still stand undecoded. They read like a run dump — start ० and
/// length ३ ahead of three octets — but the routine that emits them has not
/// been found, so that is a resemblance and not a reading.
///
/// **IS `Some(०)` DISTINGUISHABLE FROM ABSENT? — `शून्यविकल्पपरीक्षा`.**
///
/// `ir.t1` decides kind ५ (`सम्भाव्य`) is ONE WORD with ० meaning ABSENT and
/// names the cost in its own margin. `सङ्केतनॱशून्यविकल्पपरीक्षा` asks the
/// machine instead: it returns a `सम्भाव्य न६४` holding ० and tests it with
/// `असमम् शून्यम्`, answering ७००१ if the two are distinguishable and ७००२ if
/// they are one word.
///
/// ```text
///   predict: interpreted सङ्केतनॱशून्यविकल्पपरीक्षा -> 7001
///   halt:    Finisher { value: 458896179, status: Some(7002) }   2026-09-19
/// ```
///
/// **THE TWO HALVES DISAGREE, AND THAT IS THE ANSWER.** ७००१ interpreted,
/// ७००२ natively: `Some(०)` IS the nil word in compiled code, exactly as
/// `ir.t1`'s margin decided and unlike what the interpreter does. The value
/// checks out against the halt protocol — (7002 << 16) | 0x3333 = 458896179 —
/// so the number is the routine's and not a truncation.
///
/// This is the last inferred link in the `0x80009cbc` chain made measured:
/// `न्यासरचना` writes a pair only when `मूलम् असमम् शून्यम्`, 484 of 2801 pairs
/// in the encoding table carry `मूलांश == 0`, and every register slot begins
/// with one. Natively those pairs are read as ABSENT and skipped.
///
/// It is an ENTRY POINT, run as `t1_image --entry सङ्केतन शून्यविकल्पपरीक्षा`
/// over सङ्केतन's 8-module closure (9 files — ast.t1 and vastu.t1 are both
/// वास्तु), so nothing inside the corpus calls it, exactly as the rung ladder's
/// routines are called by a command line and not by `.t1`. It is named here
/// because that is what keeps it out of the "public and referenced nowhere"
/// set, and because this is where its two numbers belong.
///
/// WHY IT CARRIES NO TEXT LITERAL: no literal in this corpus holds ASCII
/// digits, and building an octet run of "0" reaches outside सङ्केतन's closure.
/// `दशाङ्कमूल्यम्`'s own two exits ARE this shape — `प्रत्यागमनम् शून्यम्` on
/// refusal against `प्रत्यागमनम् पदम्` on success, where पदम् is ० for "0".
///
/// **THE 0x80009cbc FAULT, NAMED FROM THE IMAGE — 2026-09-19. No build.**
///
/// An EMPTY RUN IS ITERATED ONCE, and its one element is null. Read from the
/// machine code at the fault, by `tools/native-window.py`:
///
/// ```text
///   ld s0, 0(sp) ; ld s0, 16(s0)   the run's data pointer
///   beq s0, zero, skip             data == 0, so the length stays 0
///   li s1, 8 ; sub s1, s0, s1
///   ld s1, 0(s1)                   length = *(data - 8)
///   skip:
///   li    s0, 0
///   sub   s0, s1, s0               s0 = length
///   sltiu s0, s0, 1                s0 = (length == 0)
///   add   s0, s1, s0               BOUND = length + (length == 0)   <<<<
///   ...
///   beq s1, s0, +0x24              arena == 0, so the element defaults to 0
///   ld s1, 104(sp) ; ld s1, 0(s1)  FAULT at address 0
/// ```
///
/// The chain is complete and every step is machine code, not inference:
/// data ० -> length ० -> bound १ -> the body runs once -> the element read is
/// guarded and answers ० -> the body dereferences that ० and faults.
///
/// THE BOUND IS NOT A ONE-OFF. The `sltiu rX, rY, 1 ; add rZ, rY, rX` idiom
/// stands at **134 sites** in this image, found with the known site at
/// 0x80009c44 as a positive control. It is the general lowering for a loop
/// bounded by a run's length.
///
/// AND IT IS NOT OBVIOUSLY WRONG. The interpreter's empty non-octet run IS
/// length १ carrying a NIL ELEMENT, so the lowering reproduces the interpreter's
/// length convention faithfully. What it does not reproduce is the element:
/// natively the run has no storage, the guard hands the body a ०, and the body
/// dereferences it. The two halves agree on the COUNT and disagree on whether
/// there is anything to read — which is the state divergence, not an output one.
///
/// WHAT NAMED THE SITE: marker constants are materialised `lui` + **`addi`**
/// (not `addiw`), so they can be located in the image. 9105 sits at 0x80012e68
/// and the fault at 0x80009cbc is 36 KB EARLIER — so the fault is in a CALLEE,
/// not in the remainder of स्थानसङ्केतनम्. Interpolating encode.t1 between two
/// marker anchors (line 3323 -> 0x8000f770, line 6552 -> 0x80018c44, 11.8
/// octets per line) puts the faulting routine's entry (0x80009bdc, frame 160,
/// 356 octets) near encode.t1:1336. THE EXACT ROUTINE IS NOT PINNED: the
/// candidate there, `कोष्ठसंख्यानम्`, matches on shape and local count but reads
/// its run at field offset 16 where `सङ्केत ॱ अवकाशाः` should be 40, so it is
/// named here as unresolved rather than asserted.
///
/// **AFTER THE FIX, 2026-09-19 — the compiled parser PARSES, and the next fault
/// is a different shape.**
///
/// ```text
///   bytes emitted        997 -> 1173
///   9303 LINE PARSED     0   -> 4     the line parser completes, four times
///   9201 record fetched  0   -> 4
///   9106 family check    0   -> 1
///   9103 register lookup 0   -> 3
///   9105 operand loop    0   -> 1     the operand classification loop COMPLETES
///   halt  BadAccess { pc: 0x80009cbc, addr: 0 }
/// ```
///
/// Qualifying eight bare cross-module record declarations took the compiled
/// compiler from dying inside the first instruction's family lookup to parsing
/// four opcode-table lines and finishing operand classification.
///
/// THE REMAINING FAULT IS A READ, NOT A WRITE:
///
/// ```text
///   ld s1, 104(sp)      ; a record reference
///   ld s1, 0(s1)        ; FAULT — field 0 read through null
/// ```
///
/// The last five fixes did not move it (1173 octets and the same pc before and
/// after), so those sites are in paths this run does not reach and are unverified
/// rather than wrong. This is a different defect.
///
/// **THE MACHINE CODE AT THE FAULT, 2026-09-18 — read from the image, no build.**
///
/// ```text
///   0x8006e558  ld   s0, 136(sp)     ; s0 = आज्ञापाठः  (the value)
///   0x8006e55c  ld   s2, 24(sp)      ; s2 = नव         (the record base)
///   0x8006e560  addi s2, s2, 0       ; + field offset 0
///   0x8006e564  sd   s0, 0(s2)       ; FAULT — s2 is 0
/// ```
///
/// **NOTHING IN A 160-OCTET WINDOW WRITES SLOT 24(sp).** The base is only ever
/// read from it, and `चरः नव ॱॱ सङ्केत भवति ०` is exactly what puts zero there.
/// There is no allocation anywhere near: the store goes through a null because
/// the declaration said `०`.
///
/// **SO THE QUESTION INVERTS.** This fault is what a `०`-declared record local
/// SHOULD produce. The anomaly is the five probes that pass — something about
/// their shape keeps a usable value where this one reloads a zero. Eleven
/// explanations were spent asking why this site fails; the answer may be that it
/// does not need one, and that the passing cases are the special ones.
///
/// **RUNG 98, 2026-09-18 — THE RECORD IS USABLE; ONLY THE RUN-TYPED WRITE DIES.**
///
/// At the fault itself, one line apart on one record:
///
/// ```text
///   नव ॱ पङ्क्ति भवति ० ।         scalar field   9701 LANDS
///   नव ॱ आज्ञा भवति आज्ञापाठः ।   run field      9402 ABSENT — BadAccess addr 0
/// ```
///
/// Three things follow, measured at the fault rather than inferred:
///
/// * The record is USABLE — a field write to `नव` completes, and it forces the
///   record into existence before the run write. "Never allocated" is no longer
///   available as an explanation.
/// * The source is SOUND — rung 97 put the slice's `दैर्घ्य` at ३ natively and ३
///   interpreted, same byte offset, native still an exact prefix.
/// * The difference is the FIELD'S TYPE.
///
/// And that is not a general rule, which is why it matters: a run-typed field
/// write into a ०-declared record succeeds in FIVE places measured the same day
/// — rung 86 (७२४२), `खण्डलेखनपरीक्षा` (६००३), `परमण्डलक्षेत्रलेखनपरीक्षा`
/// (६००३), `अन्तरालसहितलेखनपरीक्षा` (६००३), rung 96 (५०१२). Same operation, same
/// record type, same module, same shape of value; five passes and one fault.
///
/// **NATIVE RESULT, 2026-09-18: `ABC` + status 5012 — IT WORKS IN ISOLATION.**
///
/// `(5012 << 16) | 0x3333 = 328479539`, exact. A `सङ्केत` field write with an
/// octet run is fine natively, two statements in. The SAME operation on the SAME
/// record type faults at vishlesana.t1 once a compile has been running.
///
/// **So the fault is STATE-DEPENDENT, not statement-dependent**, and the
/// control-flow bisect that got us here has reached the end of what it can
/// answer. Sixteen probes narrowed a fault from "this compiler" to one
/// statement; the seventeenth says the statement is innocent. What breaks it is
/// what the compile LEAVES BEHIND after ~232 records of encoding.
///
/// The next question is a QUANTITY that goes wrong earlier and is only lethal
/// later — not a branch. Bisecting control flow will keep landing on innocent
/// statements.
///
/// **RUNG 96 INTERPRETED — record type, or context?**
///
/// Rung 95 measured that the slice returns and the FIELD WRITE faults natively.
/// Three explanations are dead, all killed by rung 86: `कोष्ठाङ्कः` Some(०); "a
/// ०-declared record local cannot be written"; "a RUN-typed field cannot be
/// written into one". `सङ्केतनदोषरचना` does both of the latter and answers ७२४२
/// natively. This writes the SAME field the faulting site writes, two statements
/// in, so the only difference from vishlesana.t1:885 is everything that did not
/// happen before it.
#[test]
fn rung_96_interpreted_writes_a_sanketa_field_in_isolation() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let v = it
        .call("शृङ्खलाॱस्वपरीक्षाषण्णवतिः", vec![], 80_000_000_000)
        .expect("rung 96 runs");
    println!("METRIC t1_rung96_interpreted {v:?}");
    // ५००० + the written run's length. ५००० exactly would mean the field holds an
    // empty run — written but carrying nothing, which reads as success and is
    // the empty-payload trap rung 80 already fell into once.
    match v {
        Value::Int(n) => assert!(
            n > 5000,
            "rung 96 answered {n}: ५००० exactly means the field holds an EMPTY \
             run, so the write proved nothing"
        ),
        other => panic!("rung 96 answered {other:?}, not a number"),
    }
}

// ══════════════════════════════════════════════════════════════════════════
// E23 — THE REFUSAL THAT NAMES ITS OWN TWO NUMBERS
//
// `becfca66` and `6303b0d3` established, natively, that `उत्सर्जनक्रमः`
// refuses with `E23` on the fourth instruction: `पङ्क्ति` came back ० where a
// seeded sentinel was ७७७७, and the image's last three records spell `E23`
// through `सङ्केतनकूटः`'s exact formula. Neither run said WHICH of `लेखः` and
// `विन्यस्तस्थानम्` drifted, and the two accounts want opposite repairs. The
// `९००४` frame added to that branch says both numbers plus the refusal count
// that separates them.
//
// # Why these tests are not "assert the fault is gone"
//
// Interpreted, the two passes AGREE — that is the whole reference this
// measurement is taken against, and it means the refusal branch is never
// entered on any exemplar this crate can build. An acceptance that only ran
// the agreeing path would assert nothing about the numbers: the frame would be
// absent, the test green, and the instrument unproven in exactly the branch it
// exists for.
//
// So the drift is FORCED, by mutating the emit loop to write one octet fewer
// than the layout counted, and the frame is then read for the two addresses.
// The control is the unmutated run on the SAME exemplar, which must stay
// silent — an instrument that speaks when nothing is wrong is worse than none.
// ══════════════════════════════════════════════════════════════════════════

/// The sink as a flat run of little-endian words.
///
/// **NOT `decode_items`.** That reader frames every item as a count followed by
/// that many words, which is right for `विन्यासमुद्रणम्`'s arrays and wrong for
/// the per-instruction markers `९९९९`, `९००१`, `९००२` and `९००३`, which
/// `उत्सर्जनक्रमः` writes BARE. On a module carrying instructions the framed
/// reader meets `९९९९` and reads it as a count of nine thousand nine hundred
/// and ninety-nine elements. A flat reader cannot make that mistake.
fn sink_words(sink: &[u8]) -> Vec<u32> {
    assert_eq!(
        sink.len() % 4,
        0,
        "the channel writes four octets at a time; a sink of {} is a partial \
         word and every reading below would be off by the remainder",
        sink.len()
    );
    sink.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// The `n`-element frame led by `tag`, found by NAME rather than by position.
///
/// Requires the COUNT to sit immediately before the tag, which is what makes
/// this a frame rather than a scan: a payload word that happens to equal the
/// tag is not preceded by its own element count.
fn framed_tag(sink: &[u8], tag: u32, n: usize) -> Option<Vec<u32>> {
    let w = sink_words(sink);
    (0..w.len()).find_map(|i| {
        (w[i] == n as u32 && i + n < w.len() && w[i + 1] == tag)
            .then(|| w[i + 1..i + 1 + n].to_vec())
    })
}

/// A module with a routine in it — the exemplar that REACHES the emit loop.
///
/// `TINY` cannot serve: it is data only, `आज्ञासंख्यान` is ०, and
/// `उत्सर्जनक्रमः`'s loop body never runs. That is the exemplar every rung from
/// 73 to 79 shared, and it is why "refused" and "produced nothing" could not be
/// told apart for a week.
const WITH_CODE_E23: &str = "मण्डलम् ङ ॥\nसार्वजनिक वृत्तिः च ददाति न६४ आदि\n    प्रत्यागमनम् ७ ।\nइति\n";

/// The chain with exactly one edit applied to `encode.t1`.
///
/// `matches` is asserted to be ONE. A mutation that matches twice changes two
/// places and stops being evidence about either; a mutation that matches none
/// is a test that measures the unmutated tree and passes for the wrong reason,
/// which is the shape three dead controls in this tree had.
fn chain_with_encode_edit(from: &str, to: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut hits = 0usize;
    for (name, src) in CHAIN {
        if *name == "encode.t1" {
            hits = src.matches(from).count();
            out.push(((*name).to_string(), src.replace(from, to)));
        } else {
            out.push(((*name).to_string(), (*src).to_string()));
        }
    }
    assert_eq!(
        hits, 1,
        "the anchor {from:?} matches {hits} places in encode.t1; a mutation is \
         only evidence when it changes exactly one"
    );
    out
}

/// Drive `सङ्केतनॱवस्तुसङ्केतनम्` over [`WITH_CODE_E23`] on a given chain.
///
/// The switch goes AFTER `सङ्कलनारम्भः` for the reason `build_tiny` records:
/// that call clears `सङ्केतन`'s module state, so a flag set ahead of it is
/// cleared with everything else and the sink comes back empty — which reads
/// exactly like a dump site that was never reached.
fn build_with_code(chain: &[(String, String)], dump: bool) -> (Interpreter, Value, i128) {
    let borrowed: Vec<(&str, &str)> = chain
        .iter()
        .map(|(n, s)| (n.as_str(), s.as_str()))
        .collect();
    let mut it = Interpreter::load(&borrowed, &spec_root()).expect("the chain loads");
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(WITH_CODE_E23.as_bytes()), octets("ङ".as_bytes())],
            80_000_000_000,
        )
        .expect("मण्डलसङ्कलनम् runs")
        .octets()
        .expect("octets")
        .as_slice()
        .to_vec();
    it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], 8_000_000_000)
        .expect("सङ्कलनारम्भः runs");
    it.call("वाक्यविभागॱसङ्कलनम्", vec![octets(&text)], 40_000_000_000)
        .expect("सङ्कलनम् runs");
    let program = it
        .call("वाक्यविभागॱकार्यक्रमरचना", vec![], 8_000_000_000)
        .expect("कार्यक्रमरचना runs");
    let instructions = match &program {
        Value::Record(r) => match r.borrow().get("आज्ञासंख्यान") {
            Some(Value::Int(n)) => *n,
            other => panic!("आज्ञासंख्यान is a number, not {other:?}"),
        },
        other => panic!("कार्यक्रम is a record, not {other:?}"),
    };
    if dump {
        it.call("सङ्केतनॱविन्यासमुद्रणारम्भः", vec![Value::Bool(true)], 1_000_000)
            .expect("the switch is a routine");
        assert_eq!(
            it.global("विन्यासमुद्रणार्हम्"),
            Some(&Value::Bool(true)),
            "the switch did not take — nothing below is about the dump site"
        );
    }
    let obj = it
        .call("सङ्केतनॱवस्तुसङ्केतनम्", vec![program], 80_000_000_000)
        .expect("वस्तुसङ्केतनम् must not fault interpreted");
    (it, obj, instructions)
}

/// **THE CONTROL: the E23 frame must NOT appear when the two passes agree.**
///
/// This is the case that must still be refused, and it is the whole reason the
/// frame carries a tag: an instrument that fires on a correct run turns every
/// native reading of the `९००४` payload into noise, and the native side has no
/// second opinion to check it against.
///
/// It also pins the premise the forced-drift test rests on — that this exemplar
/// REACHES the emit loop, with more than one instruction in it, so a drift
/// introduced at instruction ० can be caught at instruction १.
#[test]
fn the_e23_frame_is_absent_when_layout_and_emission_agree() {
    let chain: Vec<(String, String)> = CHAIN
        .iter()
        .map(|(n, s)| ((*n).to_string(), (*s).to_string()))
        .collect();
    let (loud, obj, instructions) = build_with_code(&chain, true);
    println!("METRIC t1_e23_exemplar_instructions {instructions}");
    assert!(
        instructions > 1,
        "the exemplar carries {instructions} instruction(s); a drift introduced \
         at instruction ० can only be CAUGHT at instruction १, so a one-\
         instruction exemplar would make the forced-drift test pass by never \
         entering the branch"
    );
    assert!(
        !matches!(obj, Value::Nil),
        "`वस्तुसङ्केतनम्` refused this exemplar INTERPRETED — then the emitter \
         never ran to completion and the absence asserted below is about the \
         wrong routine"
    );
    assert_eq!(
        framed_tag(loud.sink(), 9004, 5),
        None,
        "the E23 frame appeared on a run whose two passes AGREE. Interpreted \
         they do — `the_layout_dump_is_silent_until_asked_and_then_frames_two_\
         arrays` asserts it — so this instrument fires on correct input and no \
         native reading of it means anything. Stream: {:?}",
        sink_words(loud.sink())
    );
}

/// **THE ACCEPTANCE: a forced drift makes the refusal name both addresses.**
///
/// The emit loop writes `४` octets per uncompressed instruction, which is the
/// width `आज्ञाविस्तारः` counted in the layout. Cutting the write to `३` leaves
/// the LAYOUT untouched and puts emission exactly one octet behind per
/// instruction, so:
///
/// ```text
///   instruction ०   विन्यस्तस्थानम् ०   लेखः ०   agree, writes 3 octets
///   instruction १   विन्यस्तस्थानम् W   लेखः W-1  E23, drift 1
/// ```
///
/// The assertion is on the DRIFT and the INDEX rather than on `W` itself: the
/// laid-out width belongs to `आज्ञाविस्तारः` and the target, and pinning its
/// value here would red this test the day a compressed form is counted for this
/// exemplar — a number that encodes nothing about whether the refusal can name
/// its own two numbers.
///
/// `दोषगणना` is asserted ० because it is what separates the two accounts of the
/// native drift: a SHORT write leaves it ०, while an instruction refused
/// outright by `स्थानसङ्केतनम्` increments it. A frame that reported the drift
/// without that count would leave the native reading needing a second run.
#[test]
fn a_forced_drift_makes_the_e23_refusal_name_the_laid_out_and_the_emitted_address() {
    let chain = chain_with_encode_edit("यावत् अष्टकक्रमः न्यूनम् ४ आदि", "यावत् अष्टकक्रमः न्यूनम् ३ आदि");
    let (loud, obj, instructions) = build_with_code(&chain, true);
    assert!(
        instructions > 1,
        "the exemplar must carry more than one instruction for the drift to be \
         caught; it carries {instructions}"
    );
    assert!(
        matches!(obj, Value::Nil),
        "the mutated emitter produced an object — then the drift did not happen \
         and the frame read below is not about a refusal"
    );
    let frame = framed_tag(loud.sink(), 9004, 5).unwrap_or_else(|| {
        panic!(
            "no ९००४ frame in a drifting run. Either the refusal branch was not \
             taken — in which case the mutation did not drift anything — or the \
             frame is not framed as a count of ५ followed by the tag. Stream: \
             {:?}",
            sink_words(loud.sink())
        )
    });
    println!("METRIC t1_e23_frame {frame:?}");
    let (tag, index, laid_out, written, refused) =
        (frame[0], frame[1], frame[2], frame[3], frame[4]);
    assert_eq!(tag, 9004, "the tag leads its own frame");
    assert_eq!(
        index, 1,
        "the drift must be caught at instruction १ — instruction ० is laid out \
         at ० and emitted at ०, so it agrees no matter how few octets it writes. \
         A refusal reported at ० would mean the check reads a stale cursor"
    );
    assert_eq!(
        laid_out.checked_sub(written),
        Some(1),
        "the laid-out address {laid_out} and the written position {written} must \
         differ by exactly the one octet the mutation removed; any other drift \
         means the frame is not reporting the two cursors this branch compares"
    );
    assert_eq!(
        refused, 0,
        "`दोषगणना` must be ० on a SHORT-WRITE drift. If it counts a refusal \
         here, the number cannot separate the two accounts of the native E23 \
         and the instrument answers half the question it was added for"
    );
    // AND THE REFUSAL ITSELF, not only its report. The frame is a diagnostic;
    // the `E23` a caller sees is `सङ्केतनदोष`, and a frame printed beside a
    // diagnostic that says something else would be two instruments disagreeing.
    let code = loud.global("अन्तिमसङ्केतनदोषः").cloned();
    println!("METRIC t1_e23_recorded_diagnostic {code:?}");
}

/// **THE SAFETY CLAIM, on the branch that carries the new emit.**
///
/// `t1_debug_marker_guard` asserts the SHAPE — that the call sits under the
/// switch. This asserts the CONSEQUENCE on the one path that reaches it: with
/// the switch off, a run that refuses with E23 writes not one octet. Without
/// this the guard's claim is about lexical nesting and nothing has driven the
/// branch with the switch down.
#[test]
fn the_e23_refusal_writes_nothing_with_the_switch_off() {
    let chain = chain_with_encode_edit("यावत् अष्टकक्रमः न्यूनम् ४ आदि", "यावत् अष्टकक्रमः न्यूनम् ३ आदि");
    let (quiet, obj, _) = build_with_code(&chain, false);
    assert!(
        matches!(obj, Value::Nil),
        "the mutated emitter must still refuse with the switch off — otherwise \
         this run never reached the branch and the silence below is free"
    );
    assert!(
        quiet.sink().is_empty(),
        "the E23 branch wrote {} octets with the dump switch OFF. Every object \
         this crate assembles goes out on that same channel, so an unguarded \
         emit here corrupts the product's output on exactly the runs that fail. \
         Stream: {:?}",
        quiet.sink().len(),
        sink_words(quiet.sink())
    );
}
