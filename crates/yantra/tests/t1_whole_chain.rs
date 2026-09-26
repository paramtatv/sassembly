//! **ONE SOURCE THROUGH THE WHOLE CHAIN, DRIVEN FROM `.t1`.**
//!
//! Everything measured so far stops at EMIT. `ashtaka.t1` produces 44,785
//! octets of Sassembly text with a name table byte-equal to Rust's, and `lex.t1`
//! produces 128,356 — but **object build, link, load and run have never been
//! attempted even once from the `.t1` side.** This asks whether they carry.
//!
//! `शृङ्खलाॱमण्डलप्रतिबिम्बम्` (`shrinkhala.t1:439`) already exists and its margin
//! claims exactly this: *the whole chain, from a `.t1` source to an image that
//! runs, driven from `.t1`*. **Nothing exercised it.** The only references in
//! the tree are `shrinkhala.t1` calling itself and two censuses that COUNT
//! mentions rather than call. So this is not new code; it is the first
//! measurement of code that has been sitting there.
//!
//! **`ashtaka.t1` is the subject** for the same reason it was the first rung:
//! nine routines, zero imports, front half already measured through emit. The
//! front half is not what is being tested here.
//!
//! # The outcomes, registered before running
//!
//! `मण्डलप्रतिबिम्बम्` answers **empty octets on any refusal** and names no
//! stage, so an empty answer alone is not a result. This test probes each stage
//! so a red says WHICH:
//!
//! - **front half empty** — `मण्डलसङ्कलनम्` refused, which would contradict the
//!   measured 44,785 octets and mean something else changed.
//! - **`वस्तुरचनानिषेधः` true** — `पाठवस्तुरचना` refused; `अन्तिमवस्तुदोषः`
//!   carries the reason. The object builder is the stage.
//! - **object built, image empty** — `वस्तुप्रतिबिम्बम्` refused. The image
//!   writer is the stage.
//! - **image octets, `load_elf` refuses** — the writer produced something the
//!   loader will not take, and the loader says why.
//! - **loads and runs** — the architecture is proven for one source.
//!
//! **THE HONEST PRIOR IS A REFUSAL.** `kosha` has already shown that a stage can
//! accept its input and still leave a placeholder behind, and no part of this
//! path has executed on a real source. **A red naming its stage is worth as much
//! as a green**, because the open question is not whether it works today but
//! how many unknowns sit between here and an image.
//!
//! **AND A GREEN IS NOT SELF-HOSTING.** It is ONE SOURCE through the whole
//! chain — the architecture proven — after which twenty is repetition against a
//! known path rather than an open question. The tracked figure is a different
//! claim and this test does not move it.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::Machine;

/// **RAM SIZED FROM THE IMAGE, NOT A CONSTANT.** `W-295`.
///
/// This read `yantra::DEFAULT_RAM` and every fixture that ALLOCATES refused at
/// load, before an instruction ran: `"segment at 0x80000000 needs 536938240
/// bytes and RAM is 20971520 — raise it"`. That is the 512 MiB record region
/// (`e9bb8e77`, sized from a MEASURED whole-corpus high water) plus the image;
/// the region is `॥ स्थानम् ॥` in `ॱरिक्त`, address space with no file bytes, so
/// the only thing that has to move is the loader's RAM argument.
///
/// `yantra::ram_for` is the ONE statement of that sizing — `yantra-run` calls
/// it, `paradigm_encode.rs:1971` calls it — and its margin already records this
/// failure once, against the census's runner: *"two loaders, two answers"*.
/// **Measured 2026-09-17, there were FOUR loaders and three of them were wrong**:
/// this file, `t1_whole_chain.rs` and `t1_storage_witness.rs` each carried the
/// stale constant, which is why a region raise landed green and left reds in
/// three targets that no one attributed to it.
fn ram_for(elf: &[u8]) -> usize {
    yantra::ram_for(elf)
}
const BUDGET: u64 = 200_000_000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sadhana-t1/src")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn text_of(it: &mut Interpreter, g: &str) -> String {
    it.global(g)
        .and_then(|v| v.octets())
        .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
        .unwrap_or_default()
}

#[test]
// **UN-IGNORED 2026-09-10, W-282. THE OLD REASON IS KEPT BECAUSE IT EXPIRED
// RATHER THAN BEING WRONG**, and a reader should see which:
//     "THE BACK HALF HAS NEVER RUN. Expected RED until object build, link and
//      load carry a real source ... Un-ignore when it answers an image that
//      Machine::load_elf accepts."
// It answers exactly that, and has since the back-half unit landed. The premise
// went stale and the switch stayed off, so the capability was proven in a manual
// `--ignored` run and NOTHING WOULD HAVE NOTICED A REGRESSION. That is the same
// defect W-282 was opened for — a claim with no mechanism — one layer lower and
// worse, because here the mechanism existed and was disabled.
// Measured on un-ignoring: halt Finisher 21845 (0x5555) status 0, image 67330
// octets, loadable segment 67210 = Rust's 67210.
//
// **AND IT IS THE PATTERN, NOT THE INCIDENT — THE THIRD PREMISE-EXPIRY IN TWO
// UNITS, NONE OF WHICH ANNOUNCED ITSELF:**
//   1. `सङ्कलनावृत्तिभेद` — an exit constant left declared at in-degree 0 when
//      the guard that assigned it was removed, while a DIFFERENT exit silently
//      inherited its case.
//   2. a sign guard specified against a defect that the fix in its own commit
//      had already made unreachable — written, then withdrawn before landing.
//   3. this switch: a reason that was TRUE when written, expired, and stayed off.
// Each was locally well-reasoned and each failed against something outside its
// own frame. A guard, a margin, and a test switch fail the same way, and none of
// them reports that its premise has gone — which is why the expiry has to be
// checked by something other than re-reading the thing itself.
fn the_t1_chain_carries_ashtaka_to_an_image_that_runs() {
    // THE SHIPPED MANIFEST, not a hand-written list. This file's siblings have
    // been bitten by a list that passed while `CHAIN` was missing a module, and
    // the back half needs सङ्केतन, संयोजन and कोश as well as the front half's
    // modules — a hand list is exactly where one goes missing.
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let src = source("ashtaka.t1");
    println!("METRIC t1_chain_source_octets {}", src.len());

    // **THE MANY-SOURCE DRIVER, WITH A LIST OF ONE — NOT `मण्डलप्रतिबिम्बम्`.**
    // The single-source path routes through `वस्तुप्रतिबिम्बम्` (`:424`), which
    // links exactly one object — `वस्तूनि अङ्कः ० अन्तः भवति वस्तु` — and NO
    // STARTUP. `मण्डलानिप्रतिबिम्बम्` (`:511`) says so in its own margin: *THE
    // STARTUP FIRST — object zero, so `e_entry` is `यन्त्रारम्भ`*.
    //
    // A first version called the single path. It produced a 1770-octet image
    // that `Machine::load_elf` ACCEPTED and that faulted on its second
    // instruction with `BeyondRam { addr: 0xFFFFFFFFFFFFFFF8 }` — `sp − 8` off a
    // stack pointer nothing had set, which is exactly what a startup-less image
    // does. The traversal that reading established is real; the fault was a
    // wrong entry point on the TEST side.
    //
    // The arenas are ०-BASED: `क्रमः` starts at ० and the loop runs
    // `यावत् क्रमः न्यूनम् संख्या`, so one source is one element at index ०.
    let arena = |vs: Vec<Value>| Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)));
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(src.as_bytes())]),
                arena(vec![octets("अष्टक".as_bytes())]),
                Value::Int(1),
            ],
            80_000_000_000,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    println!("METRIC t1_chain_image_octets {}", image.len());

    if image.is_empty() {
        // WHICH STAGE. The pipeline returns empty from three different places
        // and the answer is identical from all of them, so the stage has to be
        // named from the flags each one sets rather than from the return value.
        let object_reason = text_of(&mut it, "अन्तिमवस्तुदोषः");
        let refused_object = format!("{:?}", it.global("वस्तुरचनानिषेधः"));
        let emit_kind = format!("{:?}", it.global("यन्त्रनिषेधभेद"));
        let emit_symbol = format!("{:?}", it.global("यन्त्रनिषेधसंख्या"));
        panic!(
            "मण्डलप्रतिबिम्बम् answered no image.\n  \
             वस्तुरचनानिषेधः = {refused_object}\n  \
             अन्तिमवस्तुदोषः = {object_reason:?}\n  \
             यन्त्रनिषेधभेद = {emit_kind}  संख्या = {emit_symbol}\n  \
             The front half is measured at 44785 octets, so an emit refusal here \
             would be a change elsewhere; a वस्तुरचनानिषेधः names the OBJECT \
             BUILDER; neither set names the IMAGE WRITER."
        );
    }

    // THE LOADER IS THE FIRST READER THAT IS NOT OURS. Everything up to here is
    // `.t1` checking its own work; `Machine::load_elf` is the shipped loader and
    // it accepts or refuses on its own terms.
    assert!(
        image.len() >= 64,
        "an image is at least a 64-octet header; got {}",
        image.len()
    );
    assert_eq!(&image[0..4], b"\x7fELF", "the magic");
    assert_eq!(image[4], 2, "ELFCLASS64");
    assert_eq!(image[5], 1, "ELFDATA2LSB");

    let loaded = Machine::load_elf(&image, ram_for(&image));
    let mut m = match loaded {
        Ok(m) => m,
        Err(e) => panic!(
            "the .t1 image writer produced {} octets that yantra will not load: {e:?}",
            image.len()
        ),
    };

    // AND IT RUNS — BUT A STATUS OF ० IS NOT EVIDENCE, AND THIS SAYS SO RATHER
    // THAN BANKING IT. `shrinkhala.t1:486` names no entry DELIBERATELY: the
    // startup stub's margin says entry ० gives an image "whose stub halts
    // success", so ० is what a correct image returns here AND what an image
    // that does nothing returns. On 2d's census 13 of the 14 running sources
    // return `Some(0)` and the fourteenth is a record reference, so no corpus
    // source has a computed observable.
    //
    // SETTING AN ENTRY IS NOT A FREE FIX. The same margin: naming one would be
    // the second thing in the tree to reference a startup-DEFINED symbol, and
    // the pre-link closure in `paradigm_encode.rs` is BLIND to the startup
    // object — dormant on main only because nothing references such a symbol.
    // ONE NEW CAPABILITY PER UNIT; that is a different unit.
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(BUDGET, &mut out);
    println!("METRIC t1_chain_halt {halt:?}");
    println!(
        "  stdout {} octets — status ० is EXPECTED and proves nothing",
        out.len()
    );

    // **ASSERTED, NOT PRINTED.** The first version of this test PRINTED the halt
    // and reported `ok` while the program faulted out of RAM — a check that
    // cannot fail, in the test written to avoid exactly that. The program told
    // the truth loudly and the harness swallowed it.
    //
    // AND THE ASSERTION IS "NOT A FAULT" RATHER THAN A STATUS. `ashtaka` names
    // no entry, so the startup stub halts success and the status is ० — which is
    // also what an image that does nothing returns, so a status check cannot
    // discriminate between a working back half and a degenerate one. `BeyondRam`
    // CAN: it is what the startup-less image actually produced, and no correct
    // image reaches it.
    assert!(
        !matches!(halt, yantra::Halt::BeyondRam { .. }),
        "the image faulted out of RAM — the signature of a startup that never set \
         `sp`, and what the single-object path produced: {halt:?}"
    );
}

// **THE BYTE-EQUALITY TWIN IS NOT HERE, AND THE REASON IS A MISSING API RATHER
// THAN A CHOICE.** 6e's acceptance asks for the `.t1` image compared octet for
// octet against the Rust one. `sadhana` exposes `assemble_object(source, ..)`,
// which takes Sassembly TEXT and answers an object — but the step from object
// to image is `link_images`, and that is **test-local in `paradigm_encode.rs`
// (`:1430`), not public API.** There is no callable Rust path from a source to
// an image to compare against.
//
// WHAT EXISTS INSTEAD, so this is not read as uncovered: `kosha_image.rs`
// already asserts the `.t1` image writer produces octets `yantra` loads — but
// from a HAND-BUILT text, and against the loader rather than against Rust. So
// the writer stage has a load check and no twin, and what is new here is the
// front half FEEDING it: a real source carried the whole way.
//
// **SO THIS UNIT DEMONSTRATES TRAVERSAL AND DOES NOT VERIFY CONTENT**, and that
// is the honest boundary. Making the twin possible means exporting a link step;
// that is its own unit and it is not blocked on anything.

/// **THE ONLY CONTENT CHECK AVAILABLE, AND THE ONE THIS PATH HAS NEVER HAD.**
///
/// The traversal test above shows the `.t1` back half produces an image that
/// loads and runs. It cannot show the image is RIGHT: `ashtaka` names no entry,
/// so the finisher status is `०`, and `value 0x5555` is the RISC-V test device's
/// success code that all seventeen sources produce. **Nothing distinguishes
/// 67,330 correct octets from 67,330 wrong ones that reach a finisher.**
///
/// So the check is byte equality against the Rust path, which needs no entry.
/// It is the house standard one stage earlier — `measure_corpus_twin_emit`
/// compares the two EMITTERS octet for octet — and it subsumes section sizes,
/// symbol counts and entry points, each of which is a place to be right about
/// the wrong thing.
///
/// **THE REFERENCE IS FIVE CALLS, NOT FOUR**, and the fifth is the one that is
/// easy to miss: `assemble_object` answers object BYTES and `link_objects` wants
/// a `vastu::Object`, so `vastu::read` sits between them. `paradigm_encode`
/// does exactly this at `:1182`.
///
/// **ITS WEAKNESS, NAMED RATHER THAN PAPERED OVER:** a shared error — the port
/// and its reference wrong the same way — agrees perfectly. The decode round
/// trip in `paradigm_encode` is what covers that, and it is not in this unit.
#[test]
// UN-IGNORED WITH THE TRAVERSAL, as its reason said: "pairs with the traversal
// test; un-ignore together". This is the SECOND witness behind hop 5 — a success
// halt alone could be an uninitialised register agreeing by accident, and an
// octet-identical loadable segment says the image is the one Rust builds.
fn the_t1_image_is_octet_for_octet_the_rust_one() {
    use sadhana::encode::Target;
    use sadhana::nidana::Language;
    use sadhana::t1::chain::Front;
    use sadhana::t1::riscv64;
    use sadhana::{assemble_object, vastu};

    const LOAD: u64 = 0x8000_0000; // भारस्थानम् = २१४७४८३६४८ on the .t1 side

    let src = source("ashtaka.t1");

    // THE `.t1` PATH — the same call the traversal test makes.
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let arena = |vs: Vec<Value>| Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)));
    let mine = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(src.as_bytes())]),
                arena(vec![octets("अष्टक".as_bytes())]),
                Value::Int(1),
            ],
            80_000_000_000,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(!mine.is_empty(), "no .t1 image; see the traversal test");

    // THE RUST PATH ON THE SAME SOURCE, startup FIRST so `e_entry` matches.
    let mut front = Front::load(&spec_root()).expect("Front loads");
    front.lex(&src).expect("lex");
    front.parse().expect("parse");
    front.resolve().expect("resolve");
    front.typecheck().expect("typecheck");
    front.build_ir().expect("build_ir");
    let module = front.module("अष्टक", None).expect("module builds");
    let module_text = riscv64::emit_module(&module).expect("the Rust emitter emits");
    let startup_text =
        riscv64::emit_startup_object_with_records(None, riscv64::module_allocates(&module));

    let to_object = |text: &str, name: Option<&str>| -> vastu::Object {
        let bytes = assemble_object(text, name, Target::Uncompressed, false, Language::English)
            .unwrap_or_else(|ds| panic!("{name:?} does not assemble: {} diagnostics", ds.len()));
        vastu::read(&bytes).unwrap_or_else(|| panic!("{name:?} does not read back"))
    };
    let startup = to_object(&startup_text, Some("यन्त्रारम्भ"));
    let module_obj = to_object(&module_text, Some("अष्टक"));
    // **NOT `link_objects` — IT WRITES A DIFFERENT KIND OF IMAGE.** A first
    // version used it and the twin fired at OFFSET 40 with mine ० and rust १६०:
    // offset 40 is `e_shoff`, the section header table offset, and the images
    // differed by 7710 octets. `link_objects` ends in `kosha::write_debuggable_at`
    // with `&debug` populated, so it emits section headers; the `.t1` writer
    // takes `(पाठ्यम्, दत्तम्, शून्यक्षेत्रम्, भारः)` — text, data, bss, load —
    // with NO symbols and NO debug, and `kosha.t1`'s own margin says why: the
    // loader "READS NO SECTION HEADERS AT ALL".
    //
    // **That red was a wrong REFERENCE, not a wrong image**, and the offset is
    // what said so — an early offset is a header field, and `e_shoff` names the
    // one difference that is a choice rather than a defect.
    let linked = sadhana::samyojana::link_at(&[startup, module_obj], LOAD)
        .unwrap_or_else(|es| panic!("the Rust path does not link: {es:?}"));
    let theirs =
        sadhana::kosha::write_debuggable_at(&linked.text, &linked.data, &[], linked.bss, &[], LOAD);

    println!("METRIC t1_twin_image_mine {}", mine.len());
    println!("METRIC t1_twin_image_rust {}", theirs.len());

    // **THE LOADABLE SEGMENT, NOT THE FILE — the file comparison is a CATEGORY
    // ERROR.** A first version compared whole images and fired at offset 40,
    // `e_shoff`: mine ०, rust १६० and then ३२ once symbols and debug were
    // emptied. Every Rust writer emits section headers — `kosha.rs:564` sets
    // `shnum`, `:616` and `:1035` set `shoff`, unconditionally — and NOTHING in
    // `kosha.rs` writes `e_shoff = ०`. The `.t1` writer emits none, deliberately:
    // `kosha.t1`'s margin says yantra's loader "READS NO SECTION HEADERS AT
    // ALL". **The two sides produce different ARTEFACTS by design, so file
    // equality is not a check that can pass.**
    //
    // What the loader reads, and what runs, is the PT_LOAD segment. Comparing
    // that is both well-defined and STRONGER than a file comparison: it is
    // insensitive to padding and header layout no loader looks at, and
    // sensitive to every octet that executes.
    let seg = |img: &[u8], who: &str| -> (u64, Vec<u8>) {
        let ph_off = u64::from_le_bytes(img[32..40].try_into().unwrap()) as usize;
        let ph_num = u16::from_le_bytes(img[56..58].try_into().unwrap());
        assert_eq!(
            ph_num, 1,
            "{who}: one PT_LOAD is the shape both sides write"
        );
        let p_offset =
            u64::from_le_bytes(img[ph_off + 8..ph_off + 16].try_into().unwrap()) as usize;
        let p_filesz =
            u64::from_le_bytes(img[ph_off + 32..ph_off + 40].try_into().unwrap()) as usize;
        let entry = u64::from_le_bytes(img[24..32].try_into().unwrap());
        (entry, img[p_offset..p_offset + p_filesz].to_vec())
    };
    let (my_entry, my_seg) = seg(&mine, "the .t1 image");
    let (rust_entry, rust_seg) = seg(&theirs, "the Rust image");
    println!("METRIC t1_twin_segment_mine {}", my_seg.len());
    println!("METRIC t1_twin_segment_rust {}", rust_seg.len());
    println!("  e_entry mine {my_entry:#x} rust {rust_entry:#x}");

    assert_eq!(
        my_entry, rust_entry,
        "the two images enter at different addresses"
    );
    if my_seg != rust_seg {
        let at = my_seg
            .iter()
            .zip(rust_seg.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| my_seg.len().min(rust_seg.len()));
        panic!(
            "THE LOADABLE SEGMENTS DIFFER — this is code or data, not a format \
             choice. mine {} octets, rust {} octets, FIRST DIFFERING OFFSET {at} \
             within the segment (mine {:?}, rust {:?})",
            my_seg.len(),
            rust_seg.len(),
            my_seg.get(at),
            rust_seg.get(at),
        );
    }
    println!("  the loadable segments are octet-identical");

    // THE FIRST DIFFERING OFFSET, not merely "they differ". A length mismatch
    // and a content mismatch want different fixes, and an offset says which —
    // an early offset is a header field, a late one is code or data.
    // **THE FILE-LEVEL COMPARISON IS GONE, DELIBERATELY.** It fired twice at
    // offset 40 — `e_shoff`, mine ० and rust १६० then ३२ — and both firings were
    // the same category error: every Rust writer emits section headers
    // unconditionally (`kosha.rs:564`, `:616`, `:1035`) and the `.t1` writer
    // emits none by design. **The two sides produce different ARTEFACTS, so file
    // equality is not a check that can pass**, and keeping it would be an
    // assertion that must always be red.
    //
    // What replaced it is stronger, not weaker: the segment comparison above
    // covers every octet the machine fetches plus `e_entry`, and is insensitive
    // only to bytes no loader reads.
}

/// **THE BUFFER PROBE — the fold's ordering, measured rather than argued.**
///
/// The fold moves the startup emission from before the loop to after it.
/// `यन्त्रारम्भमण्डलोत्सर्जनम्` writes through the SHARED output buffer that every
/// module's `मण्डलसङ्कलनम्` also uses, and ends `निर्गमांशः ०` — a slice from
/// ZERO, and `निर्गमांशः` does not clear. **Slicing from ० after a loop has
/// filled the buffer would return every module's text concatenated with the
/// startup's**, which is what I expected to find.
///
/// It does not, because `निर्गमारम्भः` (`utsarjana.t1:849`) sets
/// `निर्गमसूचकाङ्क भवति ०` — its margin says `self.output.clear()` — and the
/// routine opens with that call. **But that is a READING, and three readings
/// were wrong tonight.** This runs it.
///
/// **ONE ASSERTION, TWO FAILURE MODES, AND THE LENGTH SEPARATES THEM.** Buffer
/// leakage makes the second emission LONGER by the module's text. An entry
/// symbol set as a side effect of compiling makes it DIFFERENT at a similar
/// length, because `यन्त्रारम्भमण्डलोत्सर्जनम्` reads `यन्त्रप्रवेशसंज्ञा` at
/// emission time. So this also answers whether anything in a module's
/// compilation touches that global — which the reading cannot.
#[test]
// UN-IGNORED WITH THE OTHER TWO, as its reason said: "pairs with the traversal
// and twin; un-ignore together".
fn the_startup_emits_the_same_octets_before_and_after_a_module_compiles() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");

    let emit_startup = |it: &mut Interpreter| -> Vec<u8> {
        it.call(
            "यन्त्रोत्सर्जनॱयन्त्रारम्भमण्डलोत्सर्जनम्",
            vec![Value::Bool(false)],
            8_000_000_000,
        )
        .expect("यन्त्रारम्भमण्डलोत्सर्जनम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default()
    };

    let before = emit_startup(&mut it);
    assert!(!before.is_empty(), "the startup emits something at all");
    println!("METRIC t1_startup_octets_before {}", before.len());

    // A REAL MODULE THROUGH THE FRONT HALF — the thing that fills the buffer.
    let src = source("ashtaka.t1");
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(src.as_bytes()), octets("अष्टक".as_bytes())],
            80_000_000_000,
        )
        .expect("मण्डलसङ्कलनम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(
        !text.is_empty(),
        "the front half must produce text for this probe to mean anything"
    );
    println!("METRIC t1_startup_probe_module_octets {}", text.len());

    let after = emit_startup(&mut it);
    println!("METRIC t1_startup_octets_after {}", after.len());
    println!("  यन्त्रप्रवेशसंज्ञा = {:?}", it.global("यन्त्रप्रवेशसंज्ञा"));

    assert_eq!(
        after.len(),
        before.len(),
        "the startup emission LENGTH changed after a module compiled — longer by \
         about the module's {} octets means the shared buffer leaked; a small \
         change means something else",
        text.len()
    );
    assert_eq!(
        after, before,
        "the startup emission CONTENT changed after a module compiled at the same \
         length — the buffer did not leak, so something set यन्त्रप्रवेशसंज्ञा as a \
         side effect and the entry stub differs"
    );
}

// ══════════════════════════════════════════════════════════════════════════
// W-282 — THE LENIENT HALF OF THE TRACKED NUMBER.
//
// `SELF-HOSTING: 0 of 20` had NO TEST BEHIND IT. STATE.md's own ruling said so:
// "No test asserts this quantity yet; the first is `ashtaka.t1` and it is
// expected to fail at hop one." A figure that cannot go wrong cannot go right —
// it would have read 0 on the day a source genuinely self-hosted, until someone
// edited a markdown file.
//
// **THE HOP IS ASSERTED, NOT PASS OR FAIL.** A boolean against a prediction of
// failure says only what was already believed and can never move; a hop number
// moves the day a hop is cleared, which is the whole reason to build it.
//
// **LENIENT means no Rust STAGE runs while the corpus text may come from the
// Rust driver's constant.** That constant is `(name, include_str!(source))`
// pairs, so this arm takes THE ENTIRE CORPUS out of the Rust binary — a larger
// borrow than "a list of names", which is what it was first reported to be. The
// STRICT figure is measured in `t1_selfhost_strict.rs`, which reads the sources
// off disk and asserts it names no symbol from the Rust driver.
//
// THE LADDER IS IN `hopladder/mod.rs`, ONE IMPLEMENTATION FOR BOTH ARMS. A first
// version of this unit wrote it twice; two counters that must agree and can
// drift are how the Rust-driven ladder came to be read as the self-hosting one.

mod hopladder;

use hopladder::{Reach, reach_with};

#[test]
fn the_self_hosting_ladder_names_the_hop_ashtaka_reaches() {
    let ashtaka_src = source("ashtaka.t1");
    let build =
        || Interpreter::load(CHAIN, &spec_root()).map_err(|e| format!("lenient load: {e:?}"));
    let ashtaka = reach_with(build, "अष्टक", &ashtaka_src);
    println!(
        "METRIC t1_selfhost_hops_lenient_ashtaka {}",
        ashtaka.number()
    );
    println!("  lenient ashtaka.t1 → {ashtaka:?}");

    // THE POSITIVE CONTROL MUST LAND **LOWER**. A ladder answering the same for a
    // real module and a refused source measures nothing. `अज्ञातनाम` is declared
    // nowhere, so `अर्थ` refuses at resolve and no text is emitted — hop 0, by a
    // different route than any harness failure, which reports −1.
    let control_src = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् अज्ञातनाम ।\nइति\n";
    let build2 =
        || Interpreter::load(CHAIN, &spec_root()).map_err(|e| format!("lenient load: {e:?}"));
    let control = reach_with(build2, "क", control_src);
    println!(
        "METRIC t1_selfhost_hops_lenient_control {}",
        control.number()
    );
    println!("  lenient control → {control:?}");

    assert_eq!(
        ashtaka,
        Reach::Hops(5),
        "ashtaka.t1 reached {ashtaka:?} on the LENIENT ladder, not RUN. Two \
         independent witnesses stand behind hop 5 and a red means one moved: the \
         halt must not fault (measured Finisher 21845 = 0x5555, status 0) and the \
         loadable segment must equal Rust's octet for octet (measured 67210 = \
         67210, in `the_t1_image_is_octet_for_octet_the_rust_one`). Check which."
    );
    assert!(
        control.number() < ashtaka.number(),
        "control {control:?} did not land below ashtaka {ashtaka:?}"
    );
    assert_eq!(
        control,
        Reach::Hops(0),
        "the control must be refused at EMIT, not fail in the harness: {control:?}"
    );
}
