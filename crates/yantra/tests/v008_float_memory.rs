//! **`V-008` PART 1: A `प६४` STORED IN MEMORY, ON ALL THREE ENGINES.**
//!
//! Owner ruling 2026-10-05 (O1): V-008's scope includes storing `प६४` values
//! into RUN ELEMENTS, RECORD FIELDS and MODULE GLOBALS, built before any vector
//! lowering. V-005 refused all three natively by `FileMismatch` while the
//! interpreter stored them; this file is that divergence closed.
//!
//! THE RULE IS V-005's, CARRIED INTO MEMORY: a slot DECLARED `प६४` takes only a
//! float and gives back a float; any other slot never takes one. A value in the
//! other file is refused by the ONE named cause, `FileMismatch`, on every engine
//! — the interpreter, the `.t1` chain and the Rust twin. The explicit bit move
//! `अष्टकॱप्लवसंचारः` stays the only way across.
//!
//! NATIVELY the slot is the same 64-bit word the integer path uses — `fsd`/`fld`
//! (`प्लवनिधानम्`/`प्लवाहारः`) at the address an integer store would use — so no
//! layout moves. Every agreement test below also compares the two emitters'
//! loadable segments octet for octet.
//!
//! EVERY BIT PATTERN IS GENERATED FROM A `u64` BY [`hex`], as `v005_floats.rs`
//! does, so a probe cannot carry a transcription slip its assertion agrees with.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::{Halt, Machine};

const FUEL: u64 = 80_000_000_000;
const MODULE: &str = "प्लवपरीक्षण";

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

/// A 64-bit pattern as a T1 numeral (`v005_floats.rs`'s `hex`, copied): a
/// pattern with the top bit set is written as the NEGATIVE of its two's
/// complement, because a numeral's magnitude is read as a signed 64-bit value.
fn hex(v: u64) -> String {
    const DIGITS: [&str; 16] = [
        "०", "१", "२", "३", "४", "५", "६", "७", "८", "९", "अ", "आ", "इ", "ई", "उ", "ऊ",
    ];
    let (neg, mag) = if v >> 63 == 1 {
        (true, v.wrapping_neg())
    } else {
        (false, v)
    };
    let mut s = String::new();
    for c in format!("{mag:x}").chars() {
        s.push_str(DIGITS[c.to_digit(16).unwrap() as usize]);
    }
    format!("{}०षोड्{s}", if neg { "ऋण" } else { "" })
}

/// A decimal numeral for a small integer.
fn dec(n: u64) -> String {
    const DIGITS: [&str; 10] = ["०", "१", "२", "३", "४", "५", "६", "७", "८", "९"];
    n.to_string()
        .chars()
        .map(|c| DIGITS[c.to_digit(10).unwrap() as usize])
        .collect()
}

const TENTH: u64 = 0x3FB9_9999_9999_999A;
const FIFTH: u64 = 0x3FC9_9999_9999_999A;
/// `0.1 + 0.2`, the V-005 falsifier's answer.
const SUM: u64 = 0x3FD3_3333_3333_3334;
/// A SIGNALLING NaN: an arithmetic op would quiet it, a store and a load must not.
const SNAN: u64 = 0x7FF0_0000_0000_0001;
/// A quiet NaN with a payload an arithmetic op would replace by the canonical one.
const QNAN_PAYLOAD: u64 = 0x7FF8_DEAD_BEEF_0001;
const NEG_ZERO: u64 = 0x8000_0000_0000_0000;

/// The routine every probe prints a 64-bit answer with (`v005_floats.rs`'s).
const PRINTER: &str = "वृत्तिः प्लवमुद्रणम् आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि
    चरः क्रमः ॱॱ न६४ भवति ० ।
    चरः अवगणना ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् ८ आदि
        चरः सरणम् ॱॱ न६४ भवति क्रमः गुणनम् ८ ।
        चरः सृतम् ॱॱ न६४ भवति मूल्यम् दक्षिणसृ सरणम् ।
        चरः अष्टकम् ॱॱ न६४ भवति सृतम् युक् २५५ ।
        अवगणना भवति अष्टकॱमुद्रणम् अष्टकम् ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    प्रत्यागमनम् ० ।
इति
";

/// One probe module: `अष्टक` imported, the declarations, the printer, and a
/// `मुख्यम्` whose body is `body` (which answers nothing; `मुख्यम्` answers ०).
fn module(decls: &str, body: &str) -> Vec<(&'static str, String)> {
    vec![(
        MODULE,
        format!(
            "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

{decls}
{PRINTER}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
{body}    प्रत्यागमनम् ० ।
इति
"
        ),
    )]
}

/// The line that prints a `प६४` NAME's bits.
fn print_float(name: &str) -> String {
    format!("    अवगणना भवति प्लवमुद्रणम् आरभ्य अष्टकॱप्लवसंचारः {name} समाप्तम् ।\n")
}

// ── the engines (`v005_floats.rs`'s, copied) ────────────────────────────────

fn encode_site(it: &Interpreter) -> Option<String> {
    let object = match it.global("अन्तिमवस्तुदोषः") {
        Some(Value::Octets(o)) if !o.as_slice().is_empty() => {
            Some(String::from_utf8_lossy(o.as_slice()).into_owned())
        }
        _ => None,
    };
    let Some(rec) = it.global("अन्तिमसङ्केतनदोषः") else {
        return object;
    };
    let field = |name: &str| -> Option<Value> {
        match &rec {
            Value::Record(r) => r.borrow().get(name).cloned(),
            _ => None,
        }
    };
    let text = |v: Option<Value>| {
        v.and_then(|v| {
            v.octets()
                .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
        })
        .unwrap_or_default()
    };
    let code = text(field("सङ्केताङ्क"));
    if code.is_empty() {
        return object;
    }
    let line = field("पङ्क्ति").and_then(|v| v.as_int()).unwrap_or(0);
    Some(format!("{code} at line {line}: {}", text(field("कारण"))))
}

fn words(out: &[u8]) -> Vec<u64> {
    out.chunks(8)
        .map(|c| {
            let mut w = [0u8; 8];
            w[..c.len()].copy_from_slice(c);
            u64::from_le_bytes(w)
        })
        .collect()
}

fn status(h: &Halt) -> u64 {
    match h {
        Halt::Finisher {
            status: Some(s), ..
        } => *s,
        other => panic!("the image did not finish with a status: {other:?}"),
    }
}

fn all_sources<'a>(files: &'a [(String, &'a str)]) -> Vec<(&'a str, &'a str)> {
    let mut all: Vec<(&str, &str)> = CHAIN.to_vec();
    for (f, s) in files {
        all.push((f.as_str(), s));
    }
    all
}

/// The interpreter's run of a probe: `Ok` with its words, or the refusal text.
fn interpret_result(srcs: &[(&str, String)]) -> Result<Vec<u64>, String> {
    let files: Vec<(String, &str)> = srcs
        .iter()
        .map(|(m, s)| (format!("{m}.t1"), s.as_str()))
        .collect();
    let mut it = Interpreter::load(&all_sources(&files), &spec_root()).map_err(|e| e.reason)?;
    let v = it
        .call(&format!("{MODULE}\u{971}मुख्यम्"), Vec::new(), FUEL)
        .map_err(|e| e.reason)?;
    assert_eq!(v.as_int(), Some(0), "the interpreted entry answers ०");
    Ok(words(it.sink()))
}

/// The `.t1` chain's image of the probe (compiled HERE, by the current `.t1`
/// compiler running in the interpreter), or a panic naming the refusal.
fn t1_image(srcs: &[(&str, String)]) -> Vec<u8> {
    t1_image_with(CHAIN, srcs)
}

fn t1_image_with(chain: &[(&str, &str)], srcs: &[(&str, String)]) -> Vec<u8> {
    let mut it = Interpreter::load(chain, &spec_root()).expect("the chain loads");
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
                arena(srcs.iter().map(|(_, s)| octets(s.as_bytes())).collect()),
                arena(srcs.iter().map(|(m, _)| octets(m.as_bytes())).collect()),
                Value::Int(srcs.len() as i128),
            ],
            FUEL,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(
        !image.is_empty(),
        "the probe built no image: refusal {:?}, encoder {:?}, link {:?}",
        sadhana::t1::chain::refusal_site(&it),
        encode_site(&it),
        sadhana::t1::chain::link_refusals(&it)
    );
    image
}

fn run_image(image: &[u8]) -> (Halt, Vec<u8>) {
    let mut m = Machine::load_elf(image, yantra::ram_for(image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let h = m.run(200_000_000, &mut out);
    (h, out)
}

/// THE RUST EMITTER'S TEXT for the probe's module — what the lowering wrote,
/// for the tests that pin WHICH instructions carry a float through memory.
fn rust_module_text(srcs: &[(&str, String)]) -> String {
    use sadhana::t1::chain::Front;
    use sadhana::t1::riscv64;
    let mut front = Front::load(&spec_root()).expect("Front loads");
    for (_, src) in srcs {
        front.gather(src).expect("gather");
    }
    let (m, src) = &srcs[0];
    front.lex(src).expect("lex");
    front.parse().expect("parse");
    front.resolve().expect("resolve");
    front.typecheck().expect("typecheck");
    front.build_ir().expect("build_ir");
    let module = front.module(m, Some("मुख्यम्")).expect("the module builds");
    riscv64::emit_module(&module).expect("the Rust emitter emits")
}

/// THE RUST TWIN: the same `.t1` front end (`Front`, which drives `ir.t1`), then
/// `riscv64.rs`, the Rust assembler and the Rust linker.
fn rust_twin_image(srcs: &[(&str, String)]) -> Vec<u8> {
    use sadhana::encode::Target;
    use sadhana::nidana::Language;
    use sadhana::t1::chain::Front;
    use sadhana::t1::riscv64;
    use sadhana::{assemble_object, vastu};
    const LOAD: u64 = 0x8000_0000;
    let to_object = |text: &str, name: Option<&str>| -> vastu::Object {
        let bytes = assemble_object(text, name, Target::Uncompressed, false, Language::English)
            .unwrap_or_else(|ds| panic!("{name:?} does not assemble: {ds:?}\n{text}"));
        vastu::read(&bytes).unwrap_or_else(|| panic!("{name:?} does not read back"))
    };
    let mut front = Front::load(&spec_root()).expect("Front loads");
    for (_, src) in srcs {
        front.gather(src).expect("gather");
    }
    let mut objects = Vec::new();
    let mut allocates = false;
    for (m, src) in srcs {
        front.lex(src).expect("lex");
        front.parse().expect("parse");
        front.resolve().expect("resolve");
        front.typecheck().expect("typecheck");
        front.build_ir().expect("build_ir");
        let entry = (*m == MODULE).then_some("मुख्यम्");
        let module = front.module(m, entry).expect("the module builds");
        allocates |= riscv64::module_allocates(&module);
        let text = riscv64::emit_module(&module).expect("the Rust emitter emits");
        objects.push(to_object(&text, Some(m)));
    }
    let startup = to_object(
        &riscv64::emit_startup_object_with_records(Some(&format!("{MODULE}मुख्यम्")), allocates),
        Some("यन्त्रारम्भ"),
    );
    let mut all = vec![startup];
    all.extend(objects);
    let linked = sadhana::samyojana::link_at(&all, LOAD)
        .unwrap_or_else(|es| panic!("the Rust path does not link: {es:?}"));
    sadhana::kosha::write_debuggable_at(&linked.text, &linked.data, &[], linked.bss, &[], LOAD)
}

fn segment(img: &[u8]) -> (u64, Vec<u8>) {
    let ph_off = u64::from_le_bytes(img[32..40].try_into().unwrap()) as usize;
    let p_offset = u64::from_le_bytes(img[ph_off + 8..ph_off + 16].try_into().unwrap()) as usize;
    let p_filesz = u64::from_le_bytes(img[ph_off + 32..ph_off + 40].try_into().unwrap()) as usize;
    let entry = u64::from_le_bytes(img[24..32].try_into().unwrap());
    (entry, img[p_offset..p_offset + p_filesz].to_vec())
}

/// A probe every engine must RUN and print `want` from, and whose two native
/// images must be the same loadable segment octet for octet.
fn assert_agree(what: &str, srcs: &[(&str, String)], want: &[u64]) {
    let show = |w: &[u64]| -> Vec<String> { w.iter().map(|x| format!("{x:#018x}")).collect() };
    let interp =
        interpret_result(srcs).unwrap_or_else(|e| panic!("{what}: the interpreter refused: {e}"));
    assert_eq!(show(&interp), show(want), "{what}: interpreted");
    let t1 = t1_image(srcs);
    let (h, out) = run_image(&t1);
    assert_eq!(status(&h), 0, "{what}: native (.t1) finish {h:?}");
    assert_eq!(show(&words(&out)), show(want), "{what}: native (.t1)");
    let rust = rust_twin_image(srcs);
    let (h, out) = run_image(&rust);
    assert_eq!(status(&h), 0, "{what}: rust twin finish {h:?}");
    assert_eq!(show(&words(&out)), show(want), "{what}: rust twin");
    let (me, ms) = segment(&t1);
    let (re, rs) = segment(&rust);
    assert_eq!(
        me, re,
        "{what}: the two images enter at different addresses"
    );
    if ms != rs {
        let at = ms
            .iter()
            .zip(rs.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| ms.len().min(rs.len()));
        panic!(
            "{what}: segments differ — .t1 {} octets, Rust {}, first difference at {at}",
            ms.len(),
            rs.len()
        );
    }
}

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<String>()
        .cloned()
        .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_default()
}

/// A probe every engine must REFUSE with `FileMismatch`: the interpreter by its
/// run (or load) error, the `.t1` chain by `refusal_site`, and the Rust twin by
/// `Front::build_ir`'s error.
fn assert_refused_everywhere(what: &str, srcs: &[(&str, String)]) {
    let interp = interpret_result(srcs);
    println!("{what}, interpreter: {interp:?}");
    match &interp {
        Err(e) => assert!(
            e.contains("FileMismatch"),
            "{what}: the interpreter refused otherwise: {e}"
        ),
        Ok(w) => panic!("{what}: the interpreter ran it, printing {w:x?}"),
    }
    let t1 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t1_image(srcs)));
    let t1 = t1.map(|_| ()).map_err(panic_text);
    println!("{what}, .t1: {t1:?}");
    match &t1 {
        Err(e) => assert!(
            e.contains("FileMismatch"),
            "{what}: the .t1 chain refused otherwise: {e}"
        ),
        Ok(()) => panic!("{what}: the .t1 chain built it"),
    }
    let rust = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rust_twin_image(srcs)));
    let rust = rust.map(|_| ()).map_err(panic_text);
    println!("{what}, rust: {rust:?}");
    match &rust {
        Err(e) => assert!(
            e.contains("FileMismatch"),
            "{what}: the Rust twin refused otherwise: {e}"
        ),
        Ok(()) => panic!("{what}: the Rust twin built it"),
    }
}

// ── the declarations every probe shares ─────────────────────────────────────

/// A record with a float field BETWEEN two integer ones, so a field offset that
/// ignored the float would land on a neighbour; a float run as a field; and a
/// float global, a float run global and an integer global for the controls.
const DECLS: &str = "संरचना धारकः आरभ्य
    पूर्वम् ॱॱ न६४ ऽ
    मानम् ॱॱ प६४ ऽ
    अन्त्यम् ॱॱ न६४ ऽ
    सूची ॱॱ अङ्कः अन्तः प६४
समाप्तम् ।

संरचना पूर्णधारकः आरभ्य
    मानम् ॱॱ न६४
समाप्तम् ।

चरः सञ्चितम् ॱॱ प६४ भवति ० ।
चरः वैश्विकसूची ॱॱ अङ्कः अन्तः प६४ भवति ० ।
चरः पूर्णसञ्चितम् ॱॱ न६४ भवति ० ।
";

// ── (1) a round trip through each shape ─────────────────────────────────────

/// `0.1 + 0.2` (computed, not moved in), a signalling NaN, a quiet NaN with a
/// payload and `-0.0`, each stored into a RUN ELEMENT, a RECORD FIELD and a
/// MODULE GLOBAL, read back as a `प६४` and bit-moved out. First, the fresh field
/// and the fresh global, which must read `+0.0` — all sixty-four bits clear —
/// on every engine. The integer neighbours of the float field are written
/// before and read after, so a store at the wrong offset shows.
fn probe_round_trip() -> Vec<(&'static str, String)> {
    let mut body = format!(
        "    चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {tenth} ।
    चरः ख ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {fifth} ।
    चरः ग ॱॱ प६४ भवति अष्टकॱप्लवयोगः आरभ्य क ऽ ख समाप्तम् ।
    चरः र ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः ध ॱॱ धारकः भवति ० ।
    ध ॱ पूर्वम् भवति ७ ।
    ध ॱ अन्त्यम् भवति ९ ।
    चरः घ ॱॱ प६४ भवति ध ॱ मानम् ।
{p_fresh_field}    घ भवति सञ्चितम् ।
{p_fresh_global}",
        tenth = hex(TENTH),
        fifth = hex(FIFTH),
        p_fresh_field = print_float("घ"),
        p_fresh_global = print_float("घ"),
    );
    for (k, v) in [None, Some(SNAN), Some(QNAN_PAYLOAD), Some(NEG_ZERO)]
        .into_iter()
        .enumerate()
    {
        if let Some(v) = v {
            body.push_str(&format!("    ग भवति अष्टकॱप्लवसंचारः {} ।\n", hex(v)));
        }
        body.push_str(&format!(
            "    र अङ्कः {i} अन्तः भवति ग ।
    घ भवति र अङ्कः {i} अन्तः ।
{p1}    ध ॱ मानम् भवति ग ।
    घ भवति ध ॱ मानम् ।
{p2}    सञ्चितम् भवति ग ।
    घ भवति सञ्चितम् ।
{p3}",
            i = dec(k as u64),
            p1 = print_float("घ"),
            p2 = print_float("घ"),
            p3 = print_float("घ"),
        ));
    }
    body.push_str(
        "    अवगणना भवति प्लवमुद्रणम् ध ॱ पूर्वम् ।
    अवगणना भवति प्लवमुद्रणम् ध ॱ अन्त्यम् ।
    अवगणना भवति प्लवमुद्रणम् र ॱ दैर्घ्य ।
",
    );
    module(DECLS, &body)
}

fn round_trip_want() -> Vec<u64> {
    let mut want = vec![0, 0];
    for v in [SUM, SNAN, QNAN_PAYLOAD, NEG_ZERO] {
        want.extend([v, v, v]);
    }
    want.extend([7, 9, 4]);
    want
}

#[test]
fn v008_a_float_round_trips_through_an_element_a_field_and_a_global() {
    assert_agree("the round trip", &probe_round_trip(), &round_trip_want());
}

// ── (2) a float run summed in a loop ────────────────────────────────────────

/// A THOUSAND ELEMENTS appended inside a `यावत्` — W-356's inline capacity test,
/// and past the first block's १२८, so the run is REALLOCATED and copied while it
/// holds floats — each `i / 3`, then summed in a second loop that reads every
/// element as a float. The sum is rounding-sensitive, so it is the bits of the
/// same sequential sum in Rust, not a value an approximation could reach.
fn probe_sum() -> Vec<(&'static str, String)> {
    let body = format!(
        "    चरः त्रयम् ॱॱ प६४ भवति अष्टकॱप्लवरूपान्तरम् ३ ।
    चरः र ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् १००० आदि
        चरः अंशः ॱॱ प६४ भवति अष्टकॱप्लवरूपान्तरम् क्रमः ।
        र अङ्कः क्रमः अन्तः भवति अष्टकॱप्लवभागः आरभ्य अंशः ऽ त्रयम् समाप्तम् ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    चरः योगफलम् ॱॱ प६४ भवति ० ।
    चरः सूचकः ॱॱ न६४ भवति ० ।
    यावत् सूचकः न्यूनम् र ॱ दैर्घ्य आदि
        चरः तत्त्वम् ॱॱ प६४ भवति र अङ्कः सूचकः अन्तः ।
        योगफलम् भवति अष्टकॱप्लवयोगः आरभ्य योगफलम् ऽ तत्त्वम् समाप्तम् ।
        सूचकः भवति सूचकः योगः १ ।
    इति
    अवगणना भवति प्लवमुद्रणम् र ॱ दैर्घ्य ।
{p_sum}",
        p_sum = print_float("योगफलम्"),
    );
    module(DECLS, &body)
}

fn sum_want() -> Vec<u64> {
    let mut s = 0.0f64;
    for i in 0..1000u32 {
        s += f64::from(i) / 3.0;
    }
    vec![1000, s.to_bits()]
}

#[test]
fn v008_a_thousand_float_elements_summed_in_a_loop() {
    assert_agree("the sum", &probe_sum(), &sum_want());
}

// ── (3) growth of a float run, and a store within the length ────────────────

/// THE GROWTH PATHS, each over a `प६४` run: appends OUTSIDE a loop (W-356's
/// out-of-line call) into a LOCAL, a FIELD run and a GLOBAL run, and a routine
/// that grows a run PARAMETER and returns it (W-359's grow-and-return), its
/// result assigned back. THEN ONE STORE THAT IS NOT GROWTH: a routine that does
/// NOT return its run parameter stores element ० of it — a float run stored
/// within the length on a non-returning parameter (W-359's guarded store; past
/// the length it is refused there, never grown). Each run is read back element
/// by element as floats.
fn probe_growth() -> Vec<(&'static str, String)> {
    let decls = format!(
        "{DECLS}
वृत्तिः योजय आदाय सूची ॱॱ अङ्कः अन्तः प६४ ऽ नवम् ॱॱ प६४ ददाति अङ्कः अन्तः प६४ आदि
    सूची अङ्कः सूची ॱ दैर्घ्य अन्तः भवति नवम् ।
    प्रत्यागमनम् सूची ।
इति
वृत्तिः प्रथमलेखः आदाय सूची ॱॱ अङ्कः अन्तः प६४ ऽ नवम् ॱॱ प६४ ददाति न६४ आदि
    सूची अङ्कः ० अन्तः भवति नवम् ।
    प्रत्यागमनम् ० ।
इति
"
    );
    let mut body = String::from(
        "    चरः ध ॱॱ धारकः भवति ० ।
    चरः र ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः घ ॱॱ प६४ भवति ० ।
",
    );
    let vals = [SUM, SNAN, NEG_ZERO];
    for (k, v) in vals.iter().enumerate() {
        let i = dec(k as u64);
        body.push_str(&format!(
            "    घ भवति अष्टकॱप्लवसंचारः {h} ।
    र अङ्कः {i} अन्तः भवति घ ।
    ध ॱ सूची अङ्कः {i} अन्तः भवति घ ।
    वैश्विकसूची अङ्कः {i} अन्तः भवति घ ।
",
            h = hex(*v)
        ));
    }
    body.push_str(&format!(
        "    घ भवति अष्टकॱप्लवसंचारः {h} ।
    र भवति योजय आरभ्य र ऽ घ समाप्तम् ।
    घ भवति अष्टकॱप्लवसंचारः {one} ।
    अवगणना भवति प्रथमलेखः आरभ्य र ऽ घ समाप्तम् ।
",
        h = hex(QNAN_PAYLOAD),
        one = hex(1.0f64.to_bits()),
    ));
    for k in 0..4u64 {
        body.push_str(&format!(
            "    घ भवति र अङ्कः {i} अन्तः ।\n{p}",
            i = dec(k),
            p = print_float("घ")
        ));
    }
    for k in 0..3u64 {
        body.push_str(&format!(
            "    घ भवति ध ॱ सूची अङ्कः {i} अन्तः ।\n{p}    घ भवति वैश्विकसूची अङ्कः {i} अन्तः ।\n{q}",
            i = dec(k),
            p = print_float("घ"),
            q = print_float("घ"),
        ));
    }
    body.push_str(
        "    अवगणना भवति प्लवमुद्रणम् र ॱ दैर्घ्य ।
    अवगणना भवति प्लवमुद्रणम् ध ॱ सूची ॱ दैर्घ्य ।
    अवगणना भवति प्लवमुद्रणम् वैश्विकसूची ॱ दैर्घ्य ।
",
    );
    module(&decls, &body)
}

fn growth_want() -> Vec<u64> {
    let mut want = vec![1.0f64.to_bits(), SNAN, NEG_ZERO, QNAN_PAYLOAD];
    for v in [SUM, SNAN, NEG_ZERO] {
        want.extend([v, v]);
    }
    want.extend([4, 3, 3]);
    want
}

#[test]
fn v008_a_float_run_grows_on_every_growth_path_and_is_stored_within_the_length_on_a_non_returning_parameter()
 {
    assert_agree("growth", &probe_growth(), &growth_want());
}

// ── (4) the refusals, both directions, every shape ──────────────────────────

const FLOAT_THREE: &str = "    चरः क ॱॱ प६४ भवति अष्टकॱप्लवरूपान्तरम् ३ ।\n";

#[test]
fn v008_a_float_into_an_integer_element_is_refused_everywhere() {
    assert_refused_everywhere(
        "a float stored into an integer run's element",
        &module(
            DECLS,
            &format!(
                "{FLOAT_THREE}    चरः र ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    र अङ्कः ० अन्तः भवति क ।
"
            ),
        ),
    );
}

#[test]
fn v008_an_integer_into_a_float_element_is_refused_everywhere() {
    assert_refused_everywhere(
        "an integer stored into a प६४ run's element",
        &module(
            DECLS,
            "    चरः र ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    र अङ्कः ० अन्तः भवति ५ ।
",
        ),
    );
}

#[test]
fn v008_a_float_into_an_integer_field_is_refused_everywhere() {
    assert_refused_everywhere(
        "a float stored into an integer field",
        &module(
            DECLS,
            &format!(
                "{FLOAT_THREE}    चरः ध ॱॱ पूर्णधारकः भवति ० ।
    ध ॱ मानम् भवति क ।
"
            ),
        ),
    );
}

#[test]
fn v008_an_integer_into_a_float_field_is_refused_everywhere() {
    assert_refused_everywhere(
        "an integer stored into a प६४ field",
        &module(
            DECLS,
            "    चरः ध ॱॱ धारकः भवति ० ।
    ध ॱ मानम् भवति ५ ।
",
        ),
    );
}

#[test]
fn v008_a_float_into_an_integer_global_is_refused_everywhere() {
    assert_refused_everywhere(
        "a float stored into an integer global",
        &module(DECLS, &format!("{FLOAT_THREE}    पूर्णसञ्चितम् भवति क ।\n")),
    );
}

#[test]
fn v008_an_integer_into_a_float_global_is_refused_everywhere() {
    assert_refused_everywhere(
        "an integer stored into a प६४ global",
        &module(DECLS, "    सञ्चितम् भवति ५ ।\n"),
    );
}

/// THE INITIALISER IS A STORE TOO: a `प६४` global initialised with an integer
/// numeral other than ० — which would lay the integer's bits as the float — is
/// refused like the assignment, even when nothing reads it. `भवति ०` stays the
/// typed zero, `+0.0`.
#[test]
fn v008_an_integer_initialising_a_float_global_is_refused_everywhere() {
    assert_refused_everywhere(
        "an integer initialising a प६४ global",
        &module(&format!("{DECLS}चरः पञ्चकम् ॱॱ प६४ भवति ५ ।\n"), ""),
    );
}

/// READING A `प६४` SLOT YIELDS A FLOAT, so binding one to an integer local is the
/// local refusal V-005 already has — from memory now, on every engine.
#[test]
fn v008_a_float_element_read_into_an_integer_local_is_refused_everywhere() {
    assert_refused_everywhere(
        "a प६४ element read into an integer local",
        &module(
            DECLS,
            &format!(
                "{FLOAT_THREE}    चरः र ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    र अङ्कः ० अन्तः भवति क ।
    चरः पूर्णम् ॱॱ न६४ भवति र अङ्कः ० अन्तः ।
"
            ),
        ),
    );
}

// ── (5) the instructions, and the mutants ───────────────────────────────────

/// THE FLOAT TRAVELS THROUGH MEMORY IN THE FLOAT FILE: the round trip's text
/// holds `प्लवनिधानम्` and `प्लवाहारः` AT an address in a register (`य् ०न` /
/// `त् ०न`, not the stack pointer's frame slots), so the bits did not cross into
/// an `x` register on the way. The integer neighbours keep `निधानम्`.
#[test]
fn v008_the_lowering_is_fsd_and_fld_at_the_slot_address() {
    let text = rust_module_text(&probe_round_trip());
    let at_address = |verb: &str, role: &str| {
        text.lines()
            .filter(|l| {
                let l = l.trim();
                l.starts_with(&format!("{verb} "))
                    && l.contains(&format!("{role} ०न"))
                    && !l.contains("स्तूपसूचकः")
            })
            .count()
    };
    let fsd = at_address("प्लवनिधानम्", "य्");
    let fld = at_address("प्लवाहारः", "त्");
    println!("fsd at an address: {fsd}, fld at an address: {fld}");
    // Four values into three shapes: twelve stores. Twelve reads back, the two
    // fresh reads (field, global) — and every element read's reload is a frame
    // slot, which the filter leaves out.
    assert!(
        fsd >= 12,
        "twelve float stores at an address, found {fsd}:\n{text}"
    );
    assert!(
        fld >= 14,
        "fourteen float loads at an address, found {fld}:\n{text}"
    );
}

/// `CHAIN` with `file`'s one occurrence of `live` replaced by `mutant`.
fn mutated_chain(file: &str, live: &str, mutant: &str) -> Vec<(&'static str, String)> {
    CHAIN
        .iter()
        .map(|(n, s)| {
            if *n == file {
                assert_eq!(
                    s.matches(live).count(),
                    1,
                    "the mutation must match exactly one site of {file}"
                );
                (*n, s.replacen(live, mutant, 1))
            } else {
                (*n, (*s).to_string())
            }
        })
        .collect()
}

/// What a mutated chain did with a probe: the words a clean run printed, or why
/// it did not finish cleanly (a refused build, or a halt other than status ०).
fn native_mutated(
    chain: &[(&'static str, String)],
    srcs: &[(&str, String)],
) -> Result<Vec<u64>, String> {
    let refs: Vec<(&str, &str)> = chain.iter().map(|(n, s)| (*n, s.as_str())).collect();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_image(&t1_image_with(&refs, srcs))
    }));
    match r {
        Ok((
            Halt::Finisher {
                status: Some(0), ..
            },
            out,
        )) => Ok(words(&out)),
        Ok((h, _)) => Err(format!("halted {h:?}")),
        Err(p) => Err(panic_text(p)),
    }
}

// MUTANT 1 — the whole-word StoreAt written with the INTEGER store whatever the
// value's file, in `yantrotsarjana.t1` (anchored on its margin's last line: the
// same text stands elsewhere in the file).
const MUTANT_FSD_LIVE: &str = "verifier refused a float at any narrower width.
                उत्सर्जनॱपाठयोजनम् आरभ्य यन्त्रनिधानपदम् आरभ्य यन्त्रसङ्केतवर्गः स्रोतः समाप्तम् समाप्तम् ।";
const MUTANT_FSD_DEAD: &str = "verifier refused a float at any narrower width.
                उत्सर्जनॱपाठयोजनम् उक्तम् निधानम् इति ।";
// MUTANT 2 — the element store's file check made vacuous in `ir.t1`: the slot's
// declared file replaced by the value's own, so nothing can mismatch.
const MUTANT_CHECK_LIVE: &str = "वर्गविरोधः आरभ्य सममूल्यम् ऽ सूचीलक्ष्यप्लवम् ऽ";
const MUTANT_CHECK_DEAD: &str = "वर्गविरोधः आरभ्य सममूल्यम् ऽ मूल्यप्लवकोश अङ्कः सममूल्यम् ॱ क्रमाङ्क अन्तः ऽ";

#[test]
fn v008_mutant_float_store_with_the_integer_store_is_red() {
    let chain = mutated_chain("yantrotsarjana.t1", MUTANT_FSD_LIVE, MUTANT_FSD_DEAD);
    let got = native_mutated(&chain, &probe_round_trip());
    println!("mutant 1: {got:x?}");
    assert!(
        got.as_ref().map_or(true, |w| *w != round_trip_want()),
        "mutant 1 printed the round trip's answer: {got:x?}"
    );
}

#[test]
fn v008_mutant_without_the_element_check_is_red() {
    let chain = mutated_chain("ir.t1", MUTANT_CHECK_LIVE, MUTANT_CHECK_DEAD);
    let srcs = module(
        DECLS,
        &format!(
            "{FLOAT_THREE}    चरः र ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    र अङ्कः ० अन्तः भवति क ।
"
        ),
    );
    let got = native_mutated(&chain, &srcs);
    println!("mutant 2: {got:x?}");
    assert!(
        got.as_ref()
            .map_or_else(|e| !e.contains("FileMismatch"), |_| true),
        "mutant 2 still refused by FileMismatch — the element check is not load-bearing: {got:x?}"
    );
}

// ── (6) the interpreter's float-run table stays bounded ─────────────────────

/// REVIEW NOTE F1: the interpreter tags each `अङ्कः अन्तः प६४` it makes in a
/// side table, and an unpruned table kept every run ever made alive — the
/// compiler runs in this interpreter in Stage 1. Twenty thousand short-lived
/// float runs, one per loop iteration and dropped at its end, must leave the
/// table at most its prune floor (१०२४) plus what is live; and a run made AFTER
/// the churn must still be held to its declared file.
#[test]
fn v008_short_lived_float_runs_do_not_accumulate_in_the_interpreter() {
    let src = format!(
        "मण्डलम् {MODULE} ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् २०००० आदि
        चरः अल्पायुः ॱॱ अङ्कः अन्तः प६४ भवति ० ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    प्रत्यागमनम् ० ।
इति

सार्वजनिक वृत्तिः अन्ते ददाति न६४ आदि
    चरः र ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    र अङ्कः ० अन्तः भवति ५ ।
    प्रत्यागमनम् ० ।
इति
"
    );
    let file = format!("{MODULE}.t1");
    let mut it = Interpreter::load(&[(file.as_str(), src.as_str())], &spec_root())
        .unwrap_or_else(|e| panic!("the probe loads: {}", e.reason));
    it.call(&format!("{MODULE}\u{971}मुख्यम्"), Vec::new(), FUEL)
        .unwrap_or_else(|e| panic!("the churn runs: {}", e.reason));
    let entries = it.float_run_entries();
    println!("float-run table after 20000 short-lived runs: {entries} entries");
    assert!(
        entries <= 1024,
        "the float-run table kept {entries} entries for 20000 dead runs"
    );
    let e = it
        .call(&format!("{MODULE}\u{971}अन्ते"), Vec::new(), FUEL)
        .expect_err("an integer into a प६४ run made after the churn is refused");
    assert!(
        e.reason.contains("FileMismatch"),
        "refused otherwise: {}",
        e.reason
    );
}
