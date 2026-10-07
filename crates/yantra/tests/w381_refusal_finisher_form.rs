//! **`W-381`: A NATIVE REFUSAL MUST STOP THE MACHINE ON QEMU TOO.**
//!
//! The lowering's refusals (`ir.t1`'s `रिक्तखण्डपठननिषेधः`, `प्राचलसीमानिषेधः`,
//! `व्यूहदैर्घ्यनिषेधः`) store a word to the finisher. yantra halts on ANY store
//! there; the sifive-test device QEMU models (and hardware) acts only on its
//! PASS (`0x5555`) and FAIL (`0x3333 | n << 16`) forms and IGNORES any other
//! word, so a raw `0x355` lets the program RUN ON: the probe below then prints
//! `X` and returns ७ on QEMU, while yantra halts at the refusal with nothing
//! printed. Each probe is built by both emitters (the `.t1` chain and the Rust
//! twin) and run on yantra and on QEMU; the two machines must agree, and the
//! interpreter must refuse.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::{Halt, Machine};

mod qemu_leg;

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

const MODULE: &str = "परकर्तृ";

/// `W-355`: a read of a fresh run (length ०), then `X` printed and ७ returned.
const FRESH_READ: &str = "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    चरः ज ॱॱ न६४ भवति क अङ्कः ० अन्तः ।
    चरः अवगणना ॱॱ न६४ भवति अष्टकॱमुद्रणम् ८८ ।
    प्रत्यागमनम् ७ ।
इति
";

/// `W-381` stage 4 (O4), derived by script from `FRESH_READ` and the ratchet's probes:
/// an out-of-bounds access, then `X` printed and ७ returned.
const BOUND_READ: &str = "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    चरः ज ॱॱ न६४ भवति क अङ्कः ५ अन्तः ।
    चरः अवगणना ॱॱ न६४ भवति अष्टकॱमुद्रणम् ८८ ।
    प्रत्यागमनम् ७ ।
इति
";

/// `W-381` stage 4 (O4), derived by script from `FRESH_READ` and the ratchet's probes:
/// an out-of-bounds access, then `X` printed and ७ returned.
const NEGATIVE_READ: &str = "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    चरः ज ॱॱ न६४ भवति क अङ्कः ऋण१ अन्तः ।
    चरः अवगणना ॱॱ न६४ भवति अष्टकॱमुद्रणम् ८८ ।
    प्रत्यागमनम् ७ ।
इति
";

/// `W-381` stage 4 (O4), derived by script from `FRESH_READ` and the ratchet's probes:
/// an out-of-bounds access, then `X` printed and ७ returned.
const GLOBAL_BOUND_READ: &str = "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    क अङ्कः ० अन्तः भवति ७ ।
    चरः ज ॱॱ न६४ भवति क अङ्कः ५ अन्तः ।
    चरः अवगणना ॱॱ न६४ भवति अष्टकॱमुद्रणम् ८८ ।
    प्रत्यागमनम् ७ ।
इति
";

/// `W-381` stage 4 (O4), derived by script from `FRESH_READ` and the ratchet's probes:
/// an out-of-bounds access, then `X` printed and ७ returned.
const NEGATIVE_WRITE: &str = "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    क अङ्कः ऋण१ अन्तः भवति ५ ।
    चरः अवगणना ॱॱ न६४ भवति अष्टकॱमुद्रणम् ८८ ।
    प्रत्यागमनम् ७ ।
इति
";

/// `W-381` stage 4 (O4), derived by script from `FRESH_READ` and the ratchet's probes:
/// an out-of-bounds access, then `X` printed and ७ returned.
const NEGATIVE_WRITE_LOOP: &str = "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    चरः ज ॱॱ अ६४ भवति ऋण१ ।
    चरः म ॱॱ न६४ भवति ० ।
    यावत् म न्यूनम् १ आदि
        क अङ्कः ज अन्तः भवति ५ ।
        म भवति म योगः १ ।
    इति
    चरः अवगणना ॱॱ न६४ भवति अष्टकॱमुद्रणम् ८८ ।
    प्रत्यागमनम् ७ ।
इति
";

/// `W-359`: a store past the end of a guarded run parameter (the b1 probe
/// p09's shape), then `X` printed and ७ returned.
const PARAM_STORE_PAST: &str = "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

वृत्तिः ख आदाय क ॱॱ अङ्कः अन्तः न६४ ददाति न६४ आदि
    क अङ्कः ५ अन्तः भवति ९९ ।
    प्रत्यागमनम् ० ।
इति

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    चरः उ ॱॱ न६४ भवति ख क ।
    चरः अवगणना ॱॱ न६४ भवति अष्टकॱमुद्रणम् ८८ ।
    प्रत्यागमनम् ७ ।
इति
";

/// `V-008`'s unequal vector lengths (a fresh result run, length ०, against two
/// of length १), then `X` printed and ७ returned. The refusal is followed by a
/// spin, so with the raw word QEMU spins instead of running on.
fn vector_mismatch() -> String {
    let add = sadhana::t1::nirvahana::FLOAT_BUILTINS
        .iter()
        .find(|(_, _, _, m)| *m == "fadd.d")
        .map(|(n, ..)| *n)
        .expect("the float table names fadd.d");
    format!(
        "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः घ ॱॱ प६४ भवति ० ।
    चरः फल ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः क ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    क अङ्कः ० अन्तः भवति घ ।
    ख अङ्कः ० अन्तः भवति घ ।
    चरः परिमाणम् ॱॱ अ६४ भवति {m}ॱ{a} आरभ्य फल ऽ क ऽ ख समाप्तम् ।
    चरः अवगणना ॱॱ न६४ भवति अष्टकॱमुद्रणम् ८८ ।
    प्रत्यागमनम् ७ ।
इति
",
        m = sadhana::t1::nirvahana::VECTOR_BUILTIN_MODULE,
        a = add
    )
}

fn interpreted(src: &str) -> Result<i128, String> {
    let mut all: Vec<(&str, &str)> = CHAIN.to_vec();
    all.push(("परकर्तृ.t1", src));
    let mut it = Interpreter::load(&all, &spec_root()).map_err(|e| e.reason)?;
    it.call(&format!("{MODULE}\u{971}मुख्यम्"), Vec::new(), FUEL)
        .map(|v| v.as_int().unwrap_or(-1))
        .map_err(|e| e.reason)
}

fn t1_image(src: &str) -> Vec<u8> {
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

fn rust_twin(src: &str) -> Vec<u8> {
    use sadhana::encode::Target;
    use sadhana::nidana::Language;
    use sadhana::t1::chain::Front;
    use sadhana::t1::riscv64;
    use sadhana::{assemble_object, vastu};
    const LOAD: u64 = 0x8000_0000;
    let mut front = Front::load(&spec_root()).expect("Front loads");
    front.lex(src).expect("lex");
    front.parse().expect("parse");
    front.resolve().expect("resolve");
    front.typecheck().expect("typecheck");
    front.build_ir().expect("build_ir");
    let module = front
        .module(MODULE, Some("मुख्यम्"))
        .expect("the module builds");
    let text = riscv64::emit_module(&module).expect("the Rust emitter emits");
    let startup = riscv64::emit_startup_object_with_records(
        Some(&format!("{MODULE}मुख्यम्")),
        riscv64::module_allocates(&module),
    );
    let obj = |t: &str, n: &str| {
        let b = assemble_object(t, Some(n), Target::Uncompressed, false, Language::English)
            .unwrap_or_else(|d| panic!("{n} does not assemble: {d:?}"));
        vastu::read(&b).expect("reads back")
    };
    let linked =
        sadhana::samyojana::link_at(&[obj(&startup, "यन्त्रारम्भ"), obj(&text, MODULE)], LOAD)
            .expect("links");
    sadhana::kosha::write_debuggable_at(&linked.text, &linked.data, &[], linked.bss, &[], LOAD)
}

/// The refusal's FAIL-form finisher word, its yantra status and QEMU's process
/// status (the low octet of the status).
fn assert_refuses_alike(what: &str, src: &str, code: u64) {
    let why = interpreted(src).expect_err("the interpreter refuses");
    println!(
        "{what}: interpreter refuses: {}",
        why.lines().next().unwrap_or("")
    );
    for (engine, img) in [("t1", t1_image(src)), ("rust", rust_twin(src))] {
        if let Some(d) = std::env::var_os("W381_DUMP") {
            let _ = std::fs::write(
                std::path::Path::new(&d).join(format!("{code:x}-{engine}.elf")),
                &img,
            );
        }
        let mut m = Machine::load_elf(&img, yantra::ram_for(&img)).expect("the image loads");
        let mut out = Vec::new();
        let h = m.run(200_000_000, &mut out);
        println!("{what}, {engine}, yantra: {h:?}, printed {out:?}");
        let q = qemu_leg::run(&img);
        println!("{what}, {engine}, qemu: {q:?}");
        assert_eq!(
            h,
            Halt::Finisher {
                value: (code << 16) | 0x3333,
                status: Some(code)
            },
            "{what}, {engine}: yantra halts on the refusal's FAIL form"
        );
        assert!(out.is_empty(), "{what}, {engine}: yantra printed {out:?}");
        let want = format!("exit status: {}", code & 0xff);
        match q {
            Err(e) if e.contains(&want) => {}
            other => panic!(
                "{what}, {engine}: QEMU must stop at the refusal ({want}), not run on to the program's own return, got {other:?}"
            ),
        }
    }
}

#[test]
fn w381_a_w355_refusal_stops_qemu_as_it_stops_yantra() {
    assert_refuses_alike("W-355 fresh read", FRESH_READ, 0x355);
}

#[test]
fn w381_a_w359_refusal_stops_qemu_as_it_stops_yantra() {
    assert_refuses_alike("W-359 parameter store", PARAM_STORE_PAST, 0x359);
}

#[test]
fn w381_a_vector_length_refusal_stops_qemu_as_it_stops_yantra() {
    assert_refuses_alike(
        "V-008 unequal lengths",
        &vector_mismatch(),
        sadhana::t1::nirvahana::VECTOR_LENGTH_REFUSAL,
    );
}

#[test]
fn w381_stage4_bound_read_stops_qemu_as_it_stops_yantra() {
    assert_refuses_alike("W-381 stage 4, a read past the length", BOUND_READ, 0x355);
}

#[test]
fn w381_stage4_negative_read_stops_qemu_as_it_stops_yantra() {
    assert_refuses_alike("W-381 stage 4, a read at -1", NEGATIVE_READ, 0x355);
}

#[test]
fn w381_stage4_global_bound_read_stops_qemu_as_it_stops_yantra() {
    assert_refuses_alike(
        "W-381 stage 4, a global read past the length",
        GLOBAL_BOUND_READ,
        0x355,
    );
}

// `W-381` stage 4, owner ruling 2026-10-06: an out-of-bounds STORE has its own
// refusal, 0x35d (QEMU exit 93), split from the read's 0x355.
#[test]
fn w381_stage4_negative_write_stops_qemu_as_it_stops_yantra() {
    assert_refuses_alike("W-381 stage 4, a store at -1", NEGATIVE_WRITE, 0x35d);
}

#[test]
fn w381_stage4_negative_write_loop_stops_qemu_as_it_stops_yantra() {
    assert_refuses_alike(
        "W-381 stage 4, a store at -1 in a loop",
        NEGATIVE_WRITE_LOOP,
        0x35d,
    );
}

/// `W-381` stage 4 (the review's note): ONE routine with many read sites, so the
/// routine is far larger than a conditional branch's ±4 KiB reach and the shared
/// refusal blocks are renumbered every `निषेधपर्वसमीपता` IR instructions; W-306's
/// relaxation must keep every branch correct. Index ० of a run of one is read
/// `SITES` times, then `last` is read: in bounds the program answers
/// `SITES × 7 + 7`; at ५ every engine must refuse with 0x355 (QEMU too).
const SITES: usize = 400;
fn many_sites(last: &str) -> String {
    let base = BOUND_READ;
    let read = "    चरः ज ॱॱ न६४ भवति क अङ्कः ५ अन्तः ।\n";
    assert_eq!(base.matches(read).count(), 1, "BOUND_READ's read line");
    let mut body = String::from("    चरः ज ॱॱ न६४ भवति ० ।\n");
    for _ in 0..SITES {
        body.push_str("    ज भवति ज योगः क अङ्कः ० अन्तः ।\n");
    }
    body.push_str(&format!("    ज भवति ज योगः क अङ्कः {last} अन्तः ।\n"));
    base.replacen(read, &body, 1)
        .replacen("प्रत्यागमनम् ७ ।", "प्रत्यागमनम् ज ।", 1)
}

#[test]
fn w381_stage4_a_routine_past_the_branch_reach_stays_correct() {
    let ok = many_sites("०");
    let want = (SITES as i128 + 1) * 7;
    assert_eq!(interpreted(&ok), Ok(want), "the interpreter's answer");
    for (engine, img) in [("t1", t1_image(&ok)), ("rust", rust_twin(&ok))] {
        assert!(
            img.len() > 3 * 4096,
            "{engine}: the image must outgrow a branch's reach ({} octets)",
            img.len()
        );
        let mut m = Machine::load_elf(&img, yantra::ram_for(&img)).expect("loads");
        let mut out = Vec::new();
        let h = m.run(200_000_000, &mut out);
        assert!(
            matches!(h, Halt::Finisher { status: Some(st), .. } if i128::from(st) == want),
            "{engine}: in bounds, every one of {SITES} sites reads its element; halted {h:?}"
        );
    }
    assert_refuses_alike(
        "W-381 stage 4, an out-of-bounds read after 400 sites",
        &many_sites("५"),
        0x355,
    );
}
