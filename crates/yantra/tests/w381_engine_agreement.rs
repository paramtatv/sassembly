//! **`W-381` PART A: THE ENGINE-AGREEMENT RATCHET.** Every interpreter/native
//! mismatch the row has collected, as one table. This test makes no fix.
//!
//! Each row is one program (or, for `V-009` (i), one register name) with the
//! outcome RECORDED for each engine: the interpreter, the image the `.t1` chain
//! builds (what `t1_image` writes), and the image the Rust twin builds
//! (`riscv64.rs`). Each row also has a status, `Agrees` or
//! `Known("<row note>")`. The test passes only when EVERY row behaves exactly as
//! recorded:
//!
//! - every engine's outcome matches its recorded one; and
//! - the agreement this run COMPUTES ([`agree`]) equals the recorded status.
//!
//! So the table can only move by an edit to it. A divergence that disappears
//! (someone fixed it) changes a native outcome and flips the computed agreement,
//! and that is red until the row says `Agrees`. A divergence that appears (a
//! regression) is red until somebody records it. The table cannot land red,
//! because it records what main does, divergences included. The two
//! `ratchet_refuses_*` tests at the bottom show the checker refusing a flipped
//! status and a wrong expectation without running an engine.
//!
//! `W381_RECORD=1` prints each observed triple as a table line and asserts
//! nothing. That is how the table was written, and how it is rewritten when a
//! stage of `W-381` lands.
//!
//! WHICH STAGE FIXES WHICH ROW is the `stage` column, using the row's own staging:
//! `3` = one integer semantics (P1: 64-bit wrapping, the unsigned `न६४` compare,
//! checked operators); `4` = native bounds checks on every read and write (O4);
//! `other` = resolver, loader, scan or encoder. Stage 3a (2026-10-06: 64-bit
//! wrapping and the unsigned `न६४` compare) flipped p14, p15, p17, p21 and
//! `W-373` (f) to `Agrees` and moved p16 on both engines; its stage reads
//! `3 (done)`. Stage 3's owner rulings of 2026-10-06 flipped p19 (division by
//! zero refused on both) and, by owner ruling (a), p20 (an octet store of 300
//! keeps its low eight bits).
//! Stages 3 and 4 were HELD until
//! `V-009` landed (owner, 2026-10-05).
//!
//! STAGE 4 (O4, 2026-10-05: native bounds checks on every indexed read AND write,
//! on by default, fail closed, index −1 included) moved ten rows to `Agrees`
//! (p01–p05, p22–p24, `W-338` (a) twice; each says what it was natively) and
//! added the `s4-*` probes. Natively an out-of-bounds index halts with `W-355`'s
//! `0x355`, the interpreter's same refusal (`agreement.rs`'s `NATIVE_REFUSALS`).
//! The GAP READ (p06, `W-338` (b)) is NOT a bounds case — the slot is inside the
//! length — and the owner ruled (2026-10-06, option A) that it reads 0 on both
//! engines; an out-of-bounds STORE refuses with its own code, `0x35d`.

use sadhana::t1::agreement::{Run, agree, natives_agree};
use sadhana::t1::chain::{self, CHAIN};
use sadhana::t1::nirvahana::{self, Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::input::{Replayed, find_event_slot, replay};
use yantra::{Halt, Machine};

const FUEL: u64 = 80_000_000_000;
const STEPS: u64 = 400_000_000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

/// A decimal numeral for a `u64`, in the corpus's digits (`w373_wait.rs`'s).
fn dec(n: u64) -> String {
    const DIGITS: [&str; 10] = ["०", "१", "२", "३", "४", "५", "६", "७", "८", "९"];
    n.to_string()
        .chars()
        .map(|c| DIGITS[c.to_digit(10).unwrap() as usize])
        .collect()
}

/// The wait intrinsic's qualified name, from the interpreter's two constants.
fn wait() -> String {
    format!(
        "{}\u{971}{}",
        nirvahana::WAIT_BUILTIN_MODULE,
        nirvahana::WAIT_BUILTIN_MEMBER
    )
}

// ── what one engine came to ────────────────────────────────────────────────────

/// One engine's outcome: `sadhana::t1::agreement::Run`, the type `t1_image`'s
/// differential gate also reads (`W-381` Part B). Named `Got` here, as it was
/// when this file kept its own copy.
type Got = Run;

/// What a row records for one engine. A text field must be CONTAINED in the
/// observed text, so a reworded message still matches its key phrase.
#[derive(Debug, Clone, Copy)]
enum Want {
    Ran(i128, &'static [u8]),
    Leftover(i128, usize),
    /// No row records one since `W-381` stage 4's gap-read ruling flipped p06 (it
    /// answered Nil); kept so `W381_RECORD=1` can name a non-integer answer.
    #[allow(dead_code)]
    Answered(&'static str),
    Refused(&'static str),
    NotLoaded(&'static str),
    NoEvents(&'static str),
    Code(u64),
    Fault(&'static str),
    NotBuilt(&'static str),
    Absent,
}

impl Want {
    fn matches(&self, got: &Got) -> bool {
        match (self, got) {
            (Want::Ran(s, o), Got::Ran { status, out }) => s == status && *o == out.as_slice(),
            (
                Want::Leftover(s, d),
                Got::Leftover {
                    status, delivered, ..
                },
            ) => s == status && d == delivered,
            (Want::Answered(w), Got::Answered(g))
            | (Want::Refused(w), Got::Refused { why: g, .. })
            | (Want::NotLoaded(w), Got::NotLoaded(g))
            | (Want::NoEvents(w), Got::NoEvents(g))
            | (Want::Fault(w), Got::Fault(g))
            | (Want::NotBuilt(w), Got::NotBuilt(g)) => g.contains(w),
            (Want::Code(w), Got::Code(g)) => w == g,
            (Want::Absent, Got::Absent) => true,
            _ => false,
        }
    }
}

/// Whether a row's three outcomes agree: the interpreter with each native
/// image, and the two images with each other (two build refusals agree
/// whatever their texts: the two builders word their refusals differently).
///
/// THE RULE ITSELF IS `sadhana::t1::agreement::agree` — ONE copy, read by this
/// ratchet and by `t1_image`'s gate (owner ruling on `W-381` Part B): the gate
/// refuses exactly the pairs this table records as divergences.
fn agrees(g: &[Got; 3]) -> bool {
    agree(&g[0], &g[1]) && agree(&g[0], &g[2]) && natives_agree(&g[1], &g[2])
}

// ── the rows ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    Agrees,
    /// A divergence the row knows about, by its note in `BACKLOG.tsv`'s `W-381`.
    Known(&'static str),
}

/// What a row runs.
#[derive(Clone, Copy)]
enum Subject {
    /// A program: its modules (the FIRST holds `मुख्यम्`, the entry) and an
    /// event log, if the program waits.
    Program(fn() -> Vec<String>, Option<&'static [u64]>),
    /// A register name looked up by the `.t1` encoder (interpreted) and by the
    /// Rust encoder. Both answers are one-based, ० for "no such register".
    Register(fn() -> String),
}

struct Row {
    id: &'static str,
    subject: Subject,
    /// interpreter, `.t1` image, Rust twin. For a `Register` row: `.t1`
    /// encoder, Rust encoder, `Absent`.
    want: [Want; 3],
    status: Status,
    /// Which `W-381` stage removes the divergence: `3`, `4`, `other: ...`, or
    /// `-` for an agreeing control.
    stage: &'static str,
}

// ── the engines ────────────────────────────────────────────────────────────────

fn first_line(s: &str) -> String {
    s.lines().next().unwrap_or("").to_string()
}

fn module_of(src: &str) -> String {
    chain::module_name(src).expect("every probe declares its module")
}

/// THE INTERPRETER, as `t1_image --load` runs the predict: the chain loaded
/// beside the program's modules, then `<entry>ॱमुख्यम्`.
fn interpreted(srcs: &[String], log: Option<&[u64]>) -> Got {
    let files: Vec<(String, &str)> = srcs
        .iter()
        .map(|s| (format!("{}.t1", module_of(s)), s.as_str()))
        .collect();
    let mut all: Vec<(&str, &str)> = CHAIN.to_vec();
    for (f, s) in &files {
        all.push((f.as_str(), s));
    }
    let mut it = match Interpreter::load(&all, &spec_root()) {
        Ok(it) => it,
        Err(e) => return Got::NotLoaded(first_line(&e.reason)),
    };
    if let Some(l) = log
        && let Err(e) = it.set_events(l)
    {
        return Got::NoEvents(first_line(&e.reason));
    }
    let entry = format!("{}\u{971}मुख्यम्", module_of(&srcs[0]));
    match it.call(&entry, Vec::new(), FUEL) {
        Ok(Value::Int(status)) => {
            let out = it.sink().to_vec();
            match log {
                Some(l) if it.events_delivered() < l.len() => Got::Leftover {
                    status,
                    delivered: it.events_delivered(),
                    out,
                },
                _ => Got::Ran { status, out },
            }
        }
        Ok(v) => Got::Answered(format!("{v:?}")),
        Err(e) => Got::Refused {
            why: first_line(&e.reason),
            out: it.sink().to_vec(),
        },
    }
}

/// THE `.t1` CHAIN'S IMAGE, as `t1_image` builds it: the entry named, every
/// module handed to `मण्डलानिप्रतिबिम्बम्` in order, and a build that left a
/// failed source or an empty image counted as not built (`t1_image`'s nonzero
/// exit).
fn t1_image(srcs: &[String]) -> Result<Vec<u8>, String> {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![
            octets(module_of(&srcs[0]).as_bytes()),
            octets("मुख्यम्".as_bytes()),
        ],
        1_000_000_000,
    )
    .expect("the entry is named");
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(srcs.iter().map(|s| octets(s.as_bytes())).collect()),
                arena(
                    srcs.iter()
                        .map(|s| octets(module_of(s).as_bytes()))
                        .collect(),
                ),
                Value::Int(srcs.len() as i128),
            ],
            FUEL,
        )
        .map_err(|e| first_line(&e.reason))?
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    let failed = match it.global("सङ्कलनविफलसंख्या") {
        Some(Value::Int(k)) => *k,
        _ => -1,
    };
    if image.is_empty() || failed > 0 {
        return Err(format!(
            "failed {failed}, refusal {:?}, link {:?}",
            chain::refusal_site(&it),
            chain::link_refusals(&it)
        ));
    }
    Ok(image)
}

/// THE RUST TWIN: the `.t1` front end, then `riscv64.rs`, the Rust assembler
/// and linker (`v008_vector_lowering.rs`'s `rust_twin_image`, refusing with a
/// text instead of a panic).
fn rust_twin_image(srcs: &[String]) -> Result<Vec<u8>, String> {
    use sadhana::encode::Target;
    use sadhana::nidana::Language;
    use sadhana::t1::chain::Front;
    use sadhana::t1::riscv64;
    use sadhana::{assemble_object, vastu};
    const LOAD: u64 = 0x8000_0000;
    let to_object = |text: &str, name: &str| -> Result<vastu::Object, String> {
        let bytes = assemble_object(
            text,
            Some(name),
            Target::Uncompressed,
            false,
            Language::English,
        )
        .map_err(|ds| format!("assemble {name}: {ds:?}"))?;
        vastu::read(&bytes).ok_or_else(|| format!("{name} does not read back"))
    };
    let mut front = Front::load(&spec_root())?;
    for src in srcs {
        front.gather(src).map_err(|e| format!("gather: {e}"))?;
    }
    let entry_module = module_of(&srcs[0]);
    let mut objects = Vec::new();
    let mut allocates = false;
    for (i, src) in srcs.iter().enumerate() {
        let m = module_of(src);
        front.lex(src).map_err(|e| format!("lex: {e}"))?;
        front.parse().map_err(|e| format!("parse: {e}"))?;
        front.resolve().map_err(|e| format!("resolve: {e}"))?;
        front.typecheck().map_err(|e| format!("typecheck: {e}"))?;
        front.build_ir().map_err(|e| format!("build_ir: {e}"))?;
        // The FIRST source holds the entry: a module may span two sources.
        let entry = (i == 0 && m == entry_module).then_some("मुख्यम्");
        let module = front
            .module(&m, entry)
            .map_err(|e| format!("module: {e}"))?;
        allocates |= riscv64::module_allocates(&module);
        let text = riscv64::emit_module(&module).map_err(|e| format!("emit: {e:?}"))?;
        objects.push(to_object(&text, &m)?);
    }
    let startup = to_object(
        &riscv64::emit_startup_object_with_records(Some(&format!("{entry_module}मुख्यम्")), allocates),
        "यन्त्रारम्भ",
    )?;
    let mut all = vec![startup];
    all.extend(objects);
    let linked = sadhana::samyojana::link_at(&all, LOAD).map_err(|es| format!("link: {es:?}"))?;
    Ok(sadhana::kosha::write_debuggable_at(
        &linked.text,
        &linked.data,
        &[],
        linked.bss,
        &[],
        LOAD,
    ))
}

fn halt_got(h: Halt, out: Vec<u8>, delivered: Option<(usize, usize)>) -> Got {
    match h {
        Halt::Finisher {
            status: Some(s), ..
        } => match delivered {
            Some((d, len)) if d < len => Got::Leftover {
                status: i128::from(s),
                delivered: d,
                out,
            },
            _ => Got::Ran {
                status: i128::from(s),
                out,
            },
        },
        Halt::Finisher {
            status: None,
            value,
        } => Got::Code(value),
        Halt::BadAccess { addr, .. } => Got::Fault(format!("BadAccess addr {addr:#x}")),
        other => {
            let d = format!("{other:?}");
            Got::Fault(d.split([' ', '{']).next().unwrap_or("").to_string())
        }
    }
}

/// A NATIVE RUN, as `yantra-run` judges it: with a log, the tag found at load
/// and the log replayed; without one, a plain run.
fn native(image: Result<Vec<u8>, String>, log: Option<&[u64]>) -> Got {
    let image = match image {
        Ok(i) => i,
        Err(e) => return Got::NotBuilt(e),
    };
    let mut m = match Machine::load_elf(&image, yantra::ram_for(&image)) {
        Ok(m) => m,
        Err(e) => return Got::Fault(format!("load: {e:?}")),
    };
    let mut out = Vec::new();
    let Some(log) = log else {
        let h = m.run(STEPS, &mut out);
        return halt_got(h, out, None);
    };
    let tag = match find_event_slot(&m.mem) {
        Ok(t) => t,
        Err(e) => return Got::NoEvents(first_line(&e)),
    };
    match replay(&mut m, tag, log, STEPS, &mut out) {
        Replayed::Short { index, .. } => Got::Fault(format!("Short at wait {index}")),
        Replayed::Halted { halt, delivered } => halt_got(halt, out, Some((delivered, log.len()))),
    }
}

/// `.t1` encoder (interpreted) and Rust encoder on one register name, both
/// one-based.
fn register_pair(name: &str) -> [Got; 3] {
    let t1 = match Interpreter::load(CHAIN, &spec_root()) {
        Ok(mut it) => match it.call(
            "सङ्केतनॱकोष्ठाङ्कः",
            vec![octets(name.as_bytes())],
            1_000_000_000,
        ) {
            Ok(Value::Int(n)) => Got::Ran {
                status: n,
                out: Vec::new(),
            },
            Ok(v) => Got::Answered(format!("{v:?}")),
            Err(e) => Got::Refused {
                why: first_line(&e.reason),
                out: Vec::new(),
            },
        },
        Err(e) => Got::NotLoaded(first_line(&e.reason)),
    };
    let rust = Got::Ran {
        status: sadhana::encode::register(name).map_or(0, |(n, _)| i128::from(n) + 1),
        out: Vec::new(),
    };
    [t1, rust, Got::Absent]
}

fn observe(s: Subject) -> [Got; 3] {
    match s {
        Subject::Program(src, log) => {
            let srcs = src();
            [
                interpreted(&srcs, log),
                native(t1_image(&srcs), log),
                native(rust_twin_image(&srcs), log),
            ]
        }
        Subject::Register(name) => register_pair(&name()),
    }
}

/// Every way a row can fail the ratchet, said in words; empty means it holds.
fn check(row: &Row, got: &[Got; 3]) -> Vec<String> {
    const ENGINES: [&str; 3] = ["interpreter", ".t1 image", "rust twin"];
    let mut bad = Vec::new();
    for k in 0..3 {
        if !row.want[k].matches(&got[k]) {
            bad.push(format!(
                "{}: the {} gave {:?}, the table records {:?}",
                row.id, ENGINES[k], got[k], row.want[k]
            ));
        }
    }
    let computed = agrees(got);
    match (row.status, computed) {
        (Status::Agrees, false) => bad.push(format!(
            "{}: recorded Agrees, but the engines DIVERGE ({:?}): a regression, or a row \
             that needs Known(<note>)",
            row.id, got
        )),
        (Status::Known(n), true) => bad.push(format!(
            "{}: recorded Known({n}), but the engines now AGREE: the divergence was fixed; \
             record the new outcomes and set Agrees",
            row.id
        )),
        _ => {}
    }
    bad
}

/// The table line for an observed triple, for `W381_RECORD=1`.
fn record_line(row: &Row, got: &[Got; 3]) -> String {
    let want = |g: &Got| match g {
        Got::Ran { status, out } => {
            format!("Want::Ran({status}, b{:?})", String::from_utf8_lossy(out))
        }
        Got::Leftover {
            status, delivered, ..
        } => format!("Want::Leftover({status}, {delivered})"),
        Got::Answered(s) => format!("Want::Answered({s:?})"),
        Got::Refused { why, .. } => format!("Want::Refused({why:?})"),
        Got::NotLoaded(s) => format!("Want::NotLoaded({s:?})"),
        Got::NoEvents(s) => format!("Want::NoEvents({s:?})"),
        Got::Code(v) => format!("Want::Code({v:#x})"),
        Got::Fault(s) => format!("Want::Fault({s:?})"),
        Got::NotBuilt(s) => format!("Want::NotBuilt({s:?})"),
        Got::Absent => "Want::Absent".to_string(),
        Got::Paused { .. } => "Want::Paused".to_string(),
        Got::Inconclusive(s) => format!("Want::Inconclusive({s:?})"),
    };
    format!(
        "RECORD {} agrees={} [{}, {}, {}]",
        row.id,
        agrees(got),
        want(&got[0]),
        want(&got[1]),
        want(&got[2])
    )
}

// ── the programs this file writes (the rest are copied verbatim, below) ────────

/// `W-338` review (a): a WRITE at index −1 INSIDE A LOOP, into a run of two.
/// Answers the run's length. Natively the store lands on the length word.
const W338_NEG_WRITE_LOOP: &str = "मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    क अङ्कः १ अन्तः भवति ८ ।
    चरः ज ॱॱ अ६४ भवति ऋण१ ।
    चरः म ॱॱ न६४ भवति ० ।
    यावत् म न्यूनम् १ आदि
        क अङ्कः ज अन्तः भवति ५ ।
        म भवति म योगः १ ।
    इति
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति
";

/// The same write at −1 in straight-line code.
const W338_NEG_WRITE: &str = "मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    क अङ्कः १ अन्तः भवति ८ ।
    क अङ्कः ऋण१ अन्तः भवति ५ ।
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति
";

/// `W-381` stage 4, derived by script from the probes above (`S4_OCTET_WRITE_MINUS1`).
const S4_OCTET_WRITE_MINUS1: &str = "मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    क अङ्कः १ अन्तः भवति ८ ।
    क अङ्कः ऋण१ अन्तः भवति ५ ।
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति
";

/// `W-381` stage 4, derived by script from the probes above (`S4_WRITE_PAST_END`).
const S4_WRITE_PAST_END: &str = "मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    क अङ्कः १ अन्तः भवति ८ ।
    क अङ्कः ५ अन्तः भवति ५ ।
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति
";

/// `W-381` stage 4, derived by script from the probes above (`S4_GLOBAL_WRITE_MINUS1`).
const S4_GLOBAL_WRITE_MINUS1: &str = "मण्डलम् परकर्तृ ॥

चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    क अङ्कः ० अन्तः भवति ७ ।
    क अङ्कः १ अन्तः भवति ८ ।
    क अङ्कः ऋण१ अन्तः भवति ५ ।
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति
";

/// `W-381` stage 4, derived by script from the probes above (`S4_GLOBAL_WRITE_MINUS1_LOOP`).
const S4_GLOBAL_WRITE_MINUS1_LOOP: &str = "मण्डलम् परकर्तृ ॥

चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    क अङ्कः ० अन्तः भवति ७ ।
    क अङ्कः १ अन्तः भवति ८ ।
    चरः ज ॱॱ अ६४ भवति ऋण१ ।
    चरः म ॱॱ न६४ भवति ० ।
    यावत् म न्यूनम् १ आदि
        क अङ्कः ज अन्तः भवति ५ ।
        म भवति म योगः १ ।
    इति
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति
";

/// `W-381` stage 4, derived by script from the probes above (`S4_OCTET_READ_MINUS1`).
const S4_OCTET_READ_MINUS1: &str = "मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः ख ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    ख अङ्कः ० अन्तः भवति ७ ।
    ख अङ्कः १ अन्तः भवति ८ ।
    प्रत्यागमनम् ख अङ्कः ऋण१ अन्तः ।
इति
";

/// `W-381` stage 4, derived by script from the probes above (`S4_FAR_NEGATIVE_WRITE`).
const S4_FAR_NEGATIVE_WRITE: &str = "मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    क अङ्कः १ अन्तः भवति ८ ।
    क अङ्कः ऋण१००००००००० अन्तः भवति ५ ।
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति
";

/// `W-381` stage 4, derived by script from the probes above (`S4_PARAM_WRITE_MINUS1`).
const S4_PARAM_WRITE_MINUS1: &str = "मण्डलम् परकर्तृ ॥

वृत्तिः ख आदाय क ॱॱ अङ्कः अन्तः न६४ ददाति न६४ आदि
    क अङ्कः ऋण१ अन्तः भवति ९९ ।
    प्रत्यागमनम् ० ।
इति

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    चरः उ ॱॱ न६४ भवति ख क ।
    प्रत्यागमनम् क अङ्कः ५ अन्तः ।
इति
";

/// `W-381` stage 4, derived by script from the probes above (`S4_NESTED_WRITE_MINUS1`).
const S4_NESTED_WRITE_MINUS1: &str = "मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः अङ्कः अन्तः न६४ भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    ख अङ्कः ० अन्तः भवति ७ ।
    ख अङ्कः १ अन्तः भवति ८ ।
    क अङ्कः ० अन्तः भवति ख ।
    क अङ्कः ० अन्तः अङ्कः ऋण१ अन्तः भवति ५ ।
    प्रत्यागमनम् ख ॱ दैर्घ्य ।
इति
";

// ── `W-381` stage 4, the review's finding 1: a store FAR past the end ──────────
// `W338_NEG_WRITE`'s run of two (or its loop, or as a global) with the index
// replaced by `i`, answering the run's length. Below the largest run
// (`MAX_RUN_ENTRIES`, 2^25) such a store grows the run; at or past it both engines
// refuse 0x35d. Before the fix a word run's need (index + 1) × 8 WRAPPED at
// 2^61 natively (element 0 overwritten, length 2^61 + 1), a store at 10^12 faulted
// `BeyondRam`, and the interpreter aborted its HOST asking for terabytes.
fn s4_store_at(i: &str) -> String {
    format!(
        "मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    क अङ्कः १ अन्तः भवति ८ ।
    क अङ्कः {i} अन्तः भवति ५ ।
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति
"
    )
}
fn s4_store_at_loop(i: &str) -> String {
    format!(
        "मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    क अङ्कः १ अन्तः भवति ८ ।
    चरः ज ॱॱ अ६४ भवति {i} ।
    चरः म ॱॱ न६४ भवति ० ।
    यावत् म न्यूनम् १ आदि
        क अङ्कः ज अन्तः भवति ५ ।
        म भवति म योगः १ ।
    इति
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति
"
    )
}
fn s4_store_at_global(i: &str) -> String {
    format!(
        "मण्डलम् परकर्तृ ॥

चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    क अङ्कः ० अन्तः भवति ७ ।
    क अङ्कः १ अन्तः भवति ८ ।
    क अङ्कः {i} अन्तः भवति ५ ।
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति
"
    )
}
fn s4_far_2_61() -> Vec<String> {
    vec![s4_store_at(&dec(1 << 61))]
}
fn s4_far_2_62() -> Vec<String> {
    vec![s4_store_at(&dec(1 << 62))]
}
fn s4_far_2_63_minus_1() -> Vec<String> {
    vec![s4_store_at(&dec(i64::MAX as u64))]
}
fn s4_far_10_12() -> Vec<String> {
    vec![s4_store_at(&dec(1_000_000_000_000))]
}
fn s4_far_2_61_loop() -> Vec<String> {
    vec![s4_store_at_loop(&dec(1 << 61))]
}
fn s4_far_2_61_global() -> Vec<String> {
    vec![s4_store_at_global(&dec(1 << 61))]
}
/// THE BOUNDARY: a store at exactly 2^25 (`MAX_RUN_ENTRIES`) is the first index
/// refused. (A control at 2^25 − 1 is not run: it would grow a run of 2^25 words,
/// 256 MiB natively — past this test machine's RAM, a resource fault — and a
/// ~1 GiB arena in the interpreter. `s4-write-past-end` is the growing control.)
fn s4_at_the_bound() -> Vec<String> {
    vec![s4_store_at(&dec(1 << 25))]
}

/// `W-381` stage 4, the review's finding 3: a store THROUGH an element of a run
/// of records at index −1 (`क अङ्कः −1 अन्तः ॱ प्लवयोगः भवति ७`). The access out
/// of bounds is the READ of the element (the record's address); the store writes
/// a field of whatever record that is. So both engines refuse it as a READ,
/// 0x355 — not a store into the run, which is what 0x35d names.
fn s4_record_element_minus1() -> Vec<String> {
    vec![format!(
        "मण्डलम् परकर्तृ ॥

{RECORD}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः अभिलेखः भवति ० ।
    चरः र ॱॱ अभिलेखः भवति ० ।
    क अङ्कः ० अन्तः भवति र ।
    क अङ्कः ऋण१ अन्तः ॱ प्लवयोगः भवति ७ ।
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति
"
    )]
}

/// `V-008`'s null record-typed MODULE GLOBAL: read a field, write ७, answer the
/// second read plus the first (interpreter: ० + ७).
fn v008_null_global() -> Vec<String> {
    vec![format!(
        "मण्डलम् परकर्तृ ॥

{RECORD}
चरः र ॱॱ अभिलेखः भवति ० ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः य ॱॱ न६४ भवति र ॱ प्लवयोगः ।
    र ॱ प्लवयोगः भवति ७ ।
    प्रत्यागमनम् र ॱ प्लवयोगः योगः य ।
इति
"
    )]
}

/// The same record as a LOCAL: the control, which agrees.
fn v008_local_control() -> Vec<String> {
    vec![format!(
        "मण्डलम् परकर्तृ ॥

{RECORD}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः र ॱॱ अभिलेखः भवति ० ।
    चरः य ॱॱ न६४ भवति र ॱ प्लवयोगः ।
    र ॱ प्लवयोगः भवति ७ ।
    प्रत्यागमनम् र ॱ प्लवयोगः योगः य ।
इति
"
    )]
}

/// `V-009` (ii): 2×2 runs `फल`, `क`, `ख` (`फल` of `n` elements) and the
/// product `व्यूहॱआव्यूहगुणनम् ( args , २ , २ , २ , व्यूहॱपङ्क्तिप्रधानम् )`.
fn v009ii_product(args: &str, n: u64) -> Vec<String> {
    let mut fill = String::new();
    for (run, len) in [("फल", n), ("क", 4), ("ख", 4)] {
        fill.push_str(&format!("    चरः {run} ॱॱ अङ्कः अन्तः प६४ भवति ० ।\n"));
        for i in 0..len {
            fill.push_str(&format!("    {run} अङ्कः {} अन्तः भवति घ ।\n", dec(i)));
        }
    }
    ashtaka(
        "",
        &format!(
            "    चरः घ ॱॱ प६४ भवति अष्टकॱप्लवसंचारः ० ।\n{fill}    चरः परिमाणम् ॱॱ अ६४ भवति व्यूहॱआव्यूहगुणनम् आरभ्य {args} ऽ २ ऽ २ ऽ २ ऽ व्यूहॱपङ्क्तिप्रधानम् समाप्तम् ।"
        ),
    )
}

/// `V-009` (ii), the review's follow-up 1: a user routine named `member`
/// (its argument plus 7), called from the entry with 58 and its answer printed
/// (`A`), in a module that also makes `v009ii_product`'s 2×2 product call when
/// `matrix`. A module that makes a matrix call is given both kernel routines,
/// named by the product's and the transpose's members.
fn v009ii_reserved(member: &str, matrix: bool) -> Vec<String> {
    let routine = format!(
        "वृत्तिः {member} आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि\n    प्रत्यागमनम् मूल्यम् योगः ७ ।\nइति\n\n"
    );
    let call = format!(
        "    अवगणना भवति अष्टकॱमुद्रणम् आरभ्य {member} {} समाप्तम् ।\n",
        dec(58)
    );
    let mut srcs = if matrix {
        v009ii_product("फल ऽ क ऽ ख", 4)
    } else {
        ashtaka("", "")
    };
    let s = &mut srcs[0];
    let at = s.find("प्रत्यागमनम् ३ ।").expect("the entry's return");
    s.insert_str(at - 4, &call);
    let at = s.find("सार्वजनिक वृत्तिः मुख्यम्").expect("the entry");
    s.insert_str(at, &routine);
    srcs
}

/// Follow-ups 2: a module GLOBAL named `member` holding 65 (`A`), printed from
/// the entry, beside `v009ii_product`'s product call when `matrix`.
fn v009ii_reserved_global(member: &str, matrix: bool) -> Vec<String> {
    let global = format!("चरः {member} ॱॱ न६४ भवति {} ।\n\n", dec(65));
    let call = format!("    अवगणना भवति अष्टकॱमुद्रणम् {member} ।\n");
    let mut srcs = if matrix {
        v009ii_product("फल ऽ क ऽ ख", 4)
    } else {
        ashtaka("", "")
    };
    let s = &mut srcs[0];
    let at = s.find("प्रत्यागमनम् ३ ।").expect("the entry's return");
    s.insert_str(at - 4, &call);
    let at = s.find("सार्वजनिक वृत्तिः मुख्यम्").expect("the entry");
    s.insert_str(at, &global);
    srcs
}

/// The follow-ups 2 review's two OLDER divergences, pinned (no fix): (a) the
/// entry module with a global `क्षण<product member>` beside a module
/// `<entry>क्षण` with a global named by the member — both labels are
/// `<entry>क्षण<member>`; (b) one routine name in both sources of one module.
/// Neither makes a matrix call.
fn v009ii_concatenated_label() -> Vec<String> {
    let member = nirvahana::MATRIX_PRODUCT_MEMBER;
    let decl = |name: &str| format!("चरः {name} ॱॱ न६४ भवति {} ।\n\n", dec(65));
    let mut srcs = ashtaka(&decl(&format!("क्षण{member}")), "");
    srcs.push(format!(
        "{}\n\n{}",
        "मण्डलम् परकर्तृ ॥".replacen("परकर्तृ", "परकर्तृक्षण", 1),
        decl(member)
    ));
    srcs
}

fn v009ii_routine_in_two_sources() -> Vec<String> {
    let member = nirvahana::MATRIX_PRODUCT_MEMBER;
    let routine = format!(
        "वृत्तिः {member} आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि\n    प्रत्यागमनम् मूल्यम् योगः ७ ।\nइति\n\n"
    );
    let mut srcs = ashtaka(&routine, "");
    srcs.push(format!("मण्डलम् परकर्तृ ॥\n\n{routine}"));
    srcs
}

/// A one-module program importing `अष्टक`, running `body` and answering ३.
fn ashtaka(globals: &str, body: &str) -> Vec<String> {
    vec![format!(
        "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

{globals}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
{body}
    प्रत्यागमनम् ३ ।
इति
"
    )]
}

/// The event interface: the tag, then the slot (`w373_wait.rs`'s two names).
fn event_globals() -> String {
    format!(
        "सार्वजनिक चरः घटनासङ्केतक ॱॱ न६४ भवति {} ।
सार्वजनिक चरः घटनामूल्यक ॱॱ न६४ भवति ० ।
",
        dec(nirvahana::EVENT_TAG)
    )
}

/// `W-373` (a), the print: `अष्टकॱमुद्रणम्` handed TWO arguments.
fn w373_a_print_two() -> Vec<String> {
    ashtaka("", "    अवगणना भवति अष्टकॱमुद्रणम् ६५ ६६ ।")
}

/// `W-373` (a), the wait: handed TWO arguments, one record `५`; answers the slot.
fn w373_a_wait_two() -> Vec<String> {
    vec![format!(
        "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

{g}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    अवगणना भवति {w} ० ० ।
    प्रत्यागमनम् घटनामूल्यक ।
इति
",
        g = event_globals(),
        w = wait()
    )]
}

/// `W-373` (a), a program's own one-parameter routine handed TWO arguments.
const W373_A_USER_TWO: &str = "मण्डलम् परकर्तृ ॥

वृत्तिः ख आदाय क ॱॱ न६४ ददाति न६४ आदि
    प्रत्यागमनम् क योगः १ ।
इति

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः उ ॱॱ न६४ भवति ख ५ ६ ।
    प्रत्यागमनम् उ ।
इति
";

/// `W-373` (b): an UNDECLARED zero-argument qualified name, plus ७.
const W373_B_UNDECLARED: &str = "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    प्रत्यागमनम् अष्टकॱअसत्सदस्यः योगः ७ ।
इति
";

/// `W-373` (b): `अष्टकॱमुद्रणम्` with NO argument.
fn w373_b_print_none() -> Vec<String> {
    ashtaka("", "    अवगणना भवति अष्टकॱमुद्रणम् ।")
}

/// `W-373` (b): the wait with NO argument, one record `५`; answers the slot.
fn w373_b_wait_none() -> Vec<String> {
    vec![format!(
        "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

{g}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    अवगणना भवति {w} ।
    प्रत्यागमनम् घटनामूल्यक ।
इति
",
        g = event_globals(),
        w = wait()
    )]
}

/// `W-373` (c): the tag is its module's LAST global; the next module's first
/// global is `परः` (९). One record `४४`; answers `परः` through its module.
fn w373_c_tag_last() -> Vec<String> {
    vec![
        format!(
            "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।
आयातः परद्वितीय ।

सार्वजनिक चरः घटनासङ्केतक ॱॱ न६४ भवति {} ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    अवगणना भवति {} ० ।
    प्रत्यागमनम् परद्वितीयॱदर्शय ।
इति
",
            dec(nirvahana::EVENT_TAG),
            wait()
        ),
        "मण्डलम् परद्वितीय ॥

सार्वजनिक चरः परः ॱॱ न६४ भवति ९ ।

सार्वजनिक वृत्तिः दर्शय ददाति न६४ आदि
    प्रत्यागमनम् परः ।
इति
"
        .to_string(),
    ]
}

/// `W-373` (d): the tag's numeral ALSO used in code, as a local; one record
/// `४४`; answers the slot.
fn w373_d_tag_in_code() -> Vec<String> {
    vec![format!(
        "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

{g}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    चरः ट ॱॱ न६४ भवति {t} ।
    अवगणना भवति {w} ० ।
    यदि ट समम् घटनासङ्केतक आदि
        प्रत्यागमनम् घटनामूल्यक ।
    इति
    प्रत्यागमनम् १ ।
इति
",
        g = event_globals(),
        t = dec(nirvahana::EVENT_TAG),
        w = wait()
    )]
}

/// `W-373` (e): TWO modules each declaring a private global `ग` (९). The entry
/// writes ४४ to its own and answers its own × १०० + the other module's.
fn w373_e_same_global() -> Vec<String> {
    vec![
        "मण्डलम् परकर्तृ ॥
आयातः परद्वितीय ।

चरः ग ॱॱ न६४ भवति ९ ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    ग भवति ४४ ।
    चरः ब ॱॱ न६४ भवति परद्वितीयॱदर्शय ।
    प्रत्यागमनम् ग गुणनम् १०० योगः ब ।
इति
"
        .to_string(),
        "मण्डलम् परद्वितीय ॥

चरः ग ॱॱ न६४ भवति ९ ।

सार्वजनिक वृत्तिः दर्शय ददाति न६४ आदि
    प्रत्यागमनम् ग ।
इति
"
        .to_string(),
    ]
}

/// `W-373` (f): `न६४` MAX `अधिकम्` १०० — १ if greater, २ if not.
fn w373_f_max_greater() -> Vec<String> {
    vec![format!(
        "मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति {} ।
    यदि क अधिकम् १०० आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् २ ।
इति
",
        dec(u64::MAX)
    )]
}

/// `W-373` (g): `अष्टकॱमुद्रणम्` with NO `आयातः अष्टक`.
const W373_G_NO_IMPORT: &str = "मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    अवगणना भवति अष्टकॱमुद्रणम् ६५ ।
    प्रत्यागमनम् ३ ।
इति
";

/// `V-009` (i): the register table's header row's field ०, read from the file.
fn header_name() -> String {
    let t = std::fs::read_to_string(spec_root().join("registers-riscv64.tsv"))
        .expect("the register table");
    let row = t
        .lines()
        .find(|l| !l.starts_with('#') && !l.trim().is_empty())
        .expect("a header row");
    row.split('\t').next().unwrap().to_string()
}

/// `V-009` (i)'s control: the first DATA row's name, read from the file.
fn first_register() -> String {
    let t = std::fs::read_to_string(spec_root().join("registers-riscv64.tsv"))
        .expect("the register table");
    let row = t
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .nth(1)
        .expect("a data row");
    row.split('\t').next().unwrap().to_string()
}

/// ADR-0044 D5: a `वर्णाष्टकम्` literal of the given code points, printed one
/// octet at a time; the program answers the literal's length. The opener is the
/// crate's constant, never a retyped spelling.
fn devanagari8_print(cps: &[u32]) -> Vec<String> {
    let letters: String = cps.iter().map(|c| char::from_u32(*c).unwrap()).collect();
    ashtaka_answering(&format!(
        "    चरः वर्णाः ॱॱ अङ्कः अन्तः अ८ भवति {} {letters} इति ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् वर्णाः ॱ दैर्घ्य आदि
        चरः एकम् ॱॱ न६४ भवति वर्णाः अङ्कः क्रमः अन्तः ।
        अवगणना भवति अष्टकॱमुद्रणम् एकम् ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    प्रत्यागमनम् वर्णाः ॱ दैर्घ्य ।",
        sadhana::devanagari8::OPEN
    ))
}

/// `ashtaka`, with the body's own `प्रत्यागमनम्` in place of the fixed ३.
fn ashtaka_answering(body: &str) -> Vec<String> {
    vec![format!(
        "मण्डलम् परकर्तृ ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
{body}
इति
"
    )]
}

/// Letters from both halves of the block, none a mark: the pool writes them as
/// `वर्णाष्टकम् … इति` text and both assemblers pack it.
fn d8_text_form() -> Vec<String> {
    devanagari8_print(&[0x0915, 0x0960, 0x097F, 0x0905, 0x0966])
}

/// The edges U+0900, ऽ, ।, ॱ, U+097F: the marks keep the pool on numerals.
fn d8_edges() -> Vec<String> {
    devanagari8_print(&[0x0900, 0x093D, 0x0964, 0x0971, 0x097F])
}

/// `इति इति` inside: the literal `इति`, three octets.
fn d8_pair() -> Vec<String> {
    let mut v = devanagari8_print(&[]);
    v[0] = v[0].replacen(" इति ।\n    चरः क्रमः", " इति इति इति ।\n    चरः क्रमः", 1);
    v
}

/// `॰` inside a literal. THE DIVERGENCE, stated here because no live row holds
/// it: the Rust lexer takes the literal WHOLE before it looks for the comment
/// mark (ADR-0017), so the interpreter reads `क॰ख`; `lex.t1` cuts the line at
/// the FIRST `॰` before any literal is seen, so both chains never see the close
/// on that line and refuse to build (for `वर्णाष्टकम्` with the owner's refusal,
/// because the scan then reaches a later line's whitespace). The same split for
/// `उक्तम्` was filed as `W-357`, which is DROPPED; this row pins it for the new
/// opener.
fn d8_comment_mark() -> Vec<String> {
    devanagari8_print(&[0x0915, 0x0970, 0x0916])
}

/// `F5` (ADR-0044 review): a literal SPANNING TWO LINES — the body on one, the
/// close on the next. The Rust lexer refuses it as unclosed (the one-line rule),
/// so the interpreter does not load it; the `.t1` chain's parser scans tokens
/// across the newline and compiles it. `उक्तम्` does the same. Pinned, not fixed.
fn d8_cross_line() -> Vec<String> {
    vec![format!(
        "मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः वर्णाः ॱॱ अङ्कः अन्तः अ८ भवति {} {}
    इति ।
    प्रत्यागमनम् वर्णाः ॱ दैर्घ्य ।
इति
",
        sadhana::devanagari8::OPEN,
        char::from_u32(0x0916).unwrap()
    )]
}

/// The coordinator's reviewer's packing rows (`zz_review_adr0044.rs`, `patch_w381.py`):
/// lengths around every packing boundary over both halves of the block, the five
/// marks, a UTF-8-shaped run and a trailing lead. `itiletters` is left out: its
/// letters spell the doubled close and it is `adr0044-d8-pair`, written correctly.
fn adr0044_rev_len1() -> Vec<String> {
    devanagari8_print(&[2325])
}
fn adr0044_rev_len7() -> Vec<String> {
    devanagari8_print(&[2325, 2400, 2309, 2431, 2304, 2368, 2367])
}
fn adr0044_rev_len8() -> Vec<String> {
    devanagari8_print(&[2325, 2400, 2309, 2431, 2304, 2368, 2367, 2406])
}
fn adr0044_rev_len9() -> Vec<String> {
    devanagari8_print(&[2325, 2400, 2309, 2431, 2304, 2368, 2367, 2406, 2364])
}
fn adr0044_rev_len15() -> Vec<String> {
    devanagari8_print(&[
        2325, 2400, 2309, 2431, 2304, 2368, 2367, 2406, 2364, 2325, 2400, 2309, 2431, 2304, 2368,
    ])
}
fn adr0044_rev_len16() -> Vec<String> {
    devanagari8_print(&[
        2325, 2400, 2309, 2431, 2304, 2368, 2367, 2406, 2364, 2325, 2400, 2309, 2431, 2304, 2368,
        2367,
    ])
}
fn adr0044_rev_len17() -> Vec<String> {
    devanagari8_print(&[
        2325, 2400, 2309, 2431, 2304, 2368, 2367, 2406, 2364, 2325, 2400, 2309, 2431, 2304, 2368,
        2367, 2406,
    ])
}
fn adr0044_rev_len24() -> Vec<String> {
    devanagari8_print(&[
        2325, 2400, 2309, 2431, 2304, 2368, 2367, 2406, 2364, 2325, 2400, 2309, 2431, 2304, 2368,
        2367, 2406, 2364, 2325, 2400, 2309, 2431, 2304, 2368,
    ])
}
fn adr0044_rev_len25() -> Vec<String> {
    devanagari8_print(&[
        2325, 2400, 2309, 2431, 2304, 2368, 2367, 2406, 2364, 2325, 2400, 2309, 2431, 2304, 2368,
        2367, 2406, 2364, 2325, 2400, 2309, 2431, 2304, 2368, 2367,
    ])
}
fn adr0044_rev_marks9() -> Vec<String> {
    devanagari8_print(&[2325, 2365, 2404, 2405, 2417, 2326, 2304, 2431, 2368])
}
fn adr0044_rev_utf8shaped() -> Vec<String> {
    devanagari8_print(&[2400, 2340, 2325])
}
fn adr0044_rev_trailinglead() -> Vec<String> {
    devanagari8_print(&[2400, 2340, 2325, 2400])
}
fn one(s: &str) -> Vec<String> {
    vec![s.to_string()]
}

/// `W-381` stage 3: `p20`'s run declared with a narrow element, written a value
/// wider than the element (२^३२ + २^१५ + ४४, or + २^३१ + ४४ for ३२ bits) and read
/// back into an `अ६४` name compared with the native result; १ when equal.
/// Words are taken from `P14`/`P20`/`P21`, never retyped (the twin of
/// `w381_stage3_integer_semantics.rs`'s `narrow_round_trip`).
fn narrow_trip(bits: u32, signed: bool) -> String {
    let tok = |src: &str, line: usize, n: usize| -> String {
        src.lines()
            .nth(line)
            .and_then(|l| l.split_whitespace().nth(n))
            .expect("a token")
            .to_string()
    };
    let neg = |n: u64| {
        format!(
            "{}{}",
            tok(P18, 3, 5).strip_suffix(dec(1).as_str()).expect("ऋण"),
            dec(n)
        )
    };
    let u = sadhana::t1::nirvahana::UNSIGNED_INTEGER_TYPES
        .iter()
        .find(|t| t.ends_with(&dec(u64::from(bits))))
        .expect("an unsigned width");
    let ty = if signed {
        let a = tok(P14, 3, 3).chars().next().expect("अ");
        format!("{a}{}", u.chars().skip(1).collect::<String>())
    } else {
        (*u).to_string()
    };
    let top: u64 = if bits == 32 { 1 << 31 } else { 1 << 15 };
    let stored = (1u64 << 32) + top + 44;
    let low = (top + 44) as i128;
    let want = if signed { low - (1i128 << bits) } else { low };
    let want = if want < 0 {
        neg(want.unsigned_abs() as u64)
    } else {
        dec(want as u64)
    };
    let read = P20.lines().nth(5).expect("p20's return line");
    let index = read.trim_start().split_once(' ').expect("a return").1;
    let l = tok(P21, 4, 1);
    let let_line = format!(
        "    {} {l} {} {} {} {index}",
        tok(P14, 3, 0),
        tok(P14, 3, 2),
        tok(P14, 3, 3),
        tok(P14, 3, 4)
    );
    let eq = tok(P21, 5, 2);
    let tail = P21
        .lines()
        .skip(5)
        .collect::<Vec<_>>()
        .join("\n")
        .replace(&format!("{eq} {}", dec(0)), &format!("{eq} {want}"));
    let head = P20
        .replacen(&tok(P20, 3, 5), &ty, 1)
        .replacen(&dec(300), &dec(stored), 1)
        .replace(read, &let_line);
    let end = head.rfind(&tok(P20, 6, 0)).expect("p20's closing word");
    format!("{}{tail}\n", &head[..end])
}

macro_rules! src {
    ($c:ident) => {
        Subject::Program(|| one($c), None)
    };
}

const LOG_5: &[u64] = &[5];
const LOG_44: &[u64] = &[44];

fn rows() -> Vec<Row> {
    use Status::{Agrees, Known};
    use Want::{Fault, Leftover, NoEvents, NotBuilt, NotLoaded, Ran, Refused};
    let p = |s| Subject::Program(s, None);
    vec![
        // ── the 26 b1 safety probes (`~/b1-safety-probes`, origin/main 6bfd55b3) ──
        // W-381 STAGE 4 (O4) flipped this row: natively it was Ran(0, b"") on both images.
        Row {
            id: "p01-local-read-past",
            subject: src!(P01),
            want: [
                Refused("entry 5 is outside an arena of 2"),
                Ran(853, b""),
                Ran(853, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // W-381 STAGE 4 (O4) flipped this row: natively it was Ran(0, b"") on both images.
        Row {
            id: "p02-octet-read-past",
            subject: src!(P02),
            want: [
                Refused("octet 5 is outside a run of 2 octets"),
                Ran(853, b""),
                Ran(853, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // W-381 STAGE 4 (O4) flipped this row: natively it was Ran(0, b"") on both images.
        Row {
            id: "p03-global-read-past",
            subject: src!(P03),
            want: [
                Refused("entry 5 is outside an arena of 1"),
                Ran(853, b""),
                Ran(853, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // W-381 STAGE 4 (O4) flipped this row: natively it was Ran(0, b"") on both images.
        Row {
            id: "p04-global-fresh-read",
            subject: src!(P04),
            want: [
                Refused("entry 0 is outside an arena of 0"),
                Ran(853, b""),
                Ran(853, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // W-381 STAGE 4 (O4) flipped this row: natively it was Ran(0, b"") on both images.
        Row {
            id: "p05-param-read-past",
            subject: src!(P05),
            want: [
                Refused("entry 5 is outside an arena of 1"),
                Ran(853, b""),
                Ran(853, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // OWNER RULING 2026-10-06 (gap reads, option A): "Interpreter reads
        // unwritten slots as 0. Matches native execution behavior and avoids
        // requiring breaking source modifications across compiler tables." A gap
        // slot is INSIDE the length, so its read is NOT refused: both engines
        // read 0 (the interpreter answered Nil, or refused adding it, before).
        Row {
            id: "p06-gap-read",
            subject: src!(P06),
            want: [Ran(0, b""), Ran(0, b""), Ran(0, b"")],
            status: Agrees,
            stage: "- (4 fixed it: gap reads, option A)",
        },
        Row {
            id: "p07-local-grow-len",
            subject: src!(P07),
            want: [Ran(105, b""), Ran(105, b""), Ran(105, b"")],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "p08-fresh-read",
            subject: src!(P08),
            want: [
                Refused("entry 0 is outside an arena of 0"),
                // W-381 moved this from Code(0x355): the refusal is now the
                // FAIL-form word 0x0355_3333, status 0x355 (853).
                Ran(0x355, b""),
                Ran(0x355, b""),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "p09-callee-store-past",
            subject: src!(P09),
            // W-381 moved this from Code(0x359): FAIL form, status 0x359 (857).
            want: [Refused("(W-359)"), Ran(0x359, b""), Ran(0x359, b"")],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "p10-grow-and-return",
            subject: src!(P10),
            want: [Ran(99, b""), Ran(99, b""), Ran(99, b"")],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "p11-grow-dropped",
            subject: src!(P11),
            want: [
                NotLoaded("(W-359)"),
                NotBuilt("failed 1"),
                NotBuilt("typecheck:"),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "p12-octet-param-cow",
            subject: src!(P12),
            want: [Ran(1, b""), Ran(9, b""), Ran(9, b"")],
            status: Known(
                "W-381 live p12, an octet-run parameter is copy-on-write only in the interpreter",
            ),
            stage: "other: parameter passing",
        },
        Row {
            id: "p13-arena-param-inplace",
            subject: src!(P13),
            want: [Ran(9, b""), Ran(9, b""), Ran(9, b"")],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "p14-add-overflow",
            subject: src!(P14),
            // AGREES since stage 3 (P1): the interpreter wraps to 64 bits.
            want: [Ran(1, b""), Ran(1, b""), Ran(1, b"")],
            status: Agrees,
            stage: "3 (done)",
        },
        Row {
            id: "p15-unsigned-cmp",
            subject: src!(P15),
            // AGREES since stage 3 (P1): a name declared न६४ compares unsigned.
            want: [Ran(2, b""), Ran(2, b""), Ran(2, b"")],
            status: Agrees,
            stage: "3 (done)",
        },
        // Agreed BEFORE stage 3 on the wrong answer (१ on all three): 0 - 1
        // wraps to 2^64 - 1, which is not below 0. Stage 3 moved BOTH engines to 2.
        Row {
            id: "p16-underflow-cmp",
            subject: src!(P16),
            want: [Ran(2, b""), Ran(2, b""), Ran(2, b"")],
            status: Agrees,
            stage: "3 (done, moved both)",
        },
        Row {
            id: "p17-shl64",
            subject: src!(P17),
            // AGREES since stage 3 (P1): a shift count is taken modulo 64, as
            // `sll` reads it, so 1 shifted left by 64 is 1 (it was 2^64 here).
            want: [Ran(1, b""), Ran(1, b""), Ran(1, b"")],
            status: Agrees,
            stage: "3 (done)",
        },
        Row {
            id: "p18-srl-sra",
            subject: src!(P18),
            want: [Ran(11, b""), Ran(11, b""), Ran(11, b"")],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "p19-div0",
            subject: src!(P19),
            // AGREES since stage 3 (owner ruling (b), 2026-10-06): a division
            // by zero is refused on both engines, natively by a zero test that
            // halts on 0x35e's FAIL form (it answered RV64's all-ones + 1 = 0).
            want: [Refused("division by"), Ran(0x35e, b""), Ran(0x35e, b"")],
            status: Agrees,
            stage: "3 (done)",
        },
        Row {
            id: "p20-octet-store-300",
            subject: src!(P20),
            // AGREES since owner ruling (a), 2026-10-06: the interpreter keeps
            // the low eight bits, as the native `sb` does (it refused).
            want: [Ran(44, b""), Ran(44, b""), Ran(44, b"")],
            status: Agrees,
            stage: "3 (done)",
        },
        Row {
            id: "p21-mul-wrap",
            subject: src!(P21),
            // AGREES since stage 3 (P1): the product keeps its low 64 bits.
            want: [Ran(1, b""), Ran(1, b""), Ran(1, b"")],
            status: Agrees,
            stage: "3 (done)",
        },
        // `W-381` stage 3, owner rulings 2026-10-06: a narrow run stored past
        // its width and read back. Before 4d56a917 the interpreter kept the
        // whole value (4295000108) and natively `न१६` sign-extended (-32724);
        // now both keep the low bits, zero-extended for `न१६` (`lhu`) and
        // sign-extended for `अ३२` (`lw`). Entries 1 = the round trip held.
        Row {
            id: "w381-narrow-n16-wide",
            subject: Subject::Program(|| one(&narrow_trip(16, false)), None),
            want: [Ran(1, b""), Ran(1, b""), Ran(1, b"")],
            status: Agrees,
            stage: "3 (done)",
        },
        Row {
            id: "w381-narrow-a32-wide",
            subject: Subject::Program(|| one(&narrow_trip(32, true)), None),
            want: [Ran(1, b""), Ran(1, b""), Ran(1, b"")],
            status: Agrees,
            stage: "3 (done)",
        },
        // W-381 STAGE 4 (O4) flipped this row: natively it was Ran(2, b"") on both images.
        Row {
            id: "p22-negative-read",
            subject: src!(P22),
            want: [
                Refused("entry -1 is outside an arena of 2"),
                Ran(853, b""),
                Ran(853, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // W-381 STAGE 4 (O4) flipped this row: natively it was Fault("BeyondRam") on both images.
        Row {
            id: "p23-far-read",
            subject: src!(P23),
            want: [
                Refused("entry 1000000000 is outside an arena of 1"),
                Ran(853, b""),
                Ran(853, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // W-381 STAGE 4 (O4) flipped this row: natively it was Ran(129_024, b"") on both images.
        Row {
            id: "p24-read-into-neighbour",
            subject: src!(P24),
            want: [
                Refused("entry 1 is outside an arena of 1"),
                Ran(853, b""),
                Ran(853, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        Row {
            id: "p25-record-unwritten",
            subject: src!(P25),
            want: [Ran(1, b""), Ran(1, b""), Ran(1, b"")],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "p26-global-grow",
            subject: src!(P26),
            want: [Ran(50_193, b""), Ran(50_193, b""), Ran(50_193, b"")],
            status: Agrees,
            stage: "-",
        },
        // ── W-338 review (a) and (b) ──
        // W-381 STAGE 4 (O4) flipped this row: natively it was Ran(5, b"") on both images.
        // 0x35d: an out-of-bounds STORE's own refusal (owner ruling 2026-10-06).
        Row {
            id: "w338-a-write-minus1-loop",
            subject: src!(W338_NEG_WRITE_LOOP),
            want: [
                Refused("arena index -1 is not an index"),
                Ran(0x35d, b""),
                Ran(0x35d, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // W-381 STAGE 4 (O4) flipped this row: natively it was Ran(5, b"") on both images.
        // 0x35d: an out-of-bounds STORE's own refusal (owner ruling 2026-10-06).
        Row {
            id: "w338-a-write-minus1-line",
            subject: src!(W338_NEG_WRITE),
            want: [
                Refused("arena index -1 is not an index"),
                Ran(0x35d, b""),
                Ran(0x35d, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // OWNER RULING 2026-10-06 (gap reads, option A): "Interpreter reads
        // unwritten slots as 0. Matches native execution behavior and avoids
        // requiring breaking source modifications across compiler tables." A gap
        // slot is INSIDE the length, so its read is NOT refused: both engines
        // read 0 (the interpreter answered Nil, or refused adding it, before).
        Row {
            id: "w338-b-gap-loop",
            subject: src!(W338_GAP),
            want: [Ran(11_003, b""), Ran(11_003, b""), Ran(11_003, b"")],
            status: Agrees,
            stage: "- (4 fixed it: gap reads, option A)",
        },
        // ── W-381 stage 4's own probes (reads and writes, -1 included) ──
        // W-381 STAGE 4 (O4) flipped this row: natively it was Ran(2, b"") on both images.
        // 0x35d: an out-of-bounds STORE's own refusal (owner ruling 2026-10-06).
        Row {
            id: "s4-octet-write-minus1",
            subject: src!(S4_OCTET_WRITE_MINUS1),
            want: [
                Refused("octet index -1 is not an index"),
                Ran(0x35d, b""),
                Ran(0x35d, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // A control: agrees before and after stage 4.
        Row {
            id: "s4-write-past-end",
            subject: src!(S4_WRITE_PAST_END),
            want: [Ran(6, b""), Ran(6, b""), Ran(6, b"")],
            status: Agrees,
            stage: "-",
        },
        // W-381 STAGE 4 (O4) flipped this row: natively it was Ran(5, b"") on both images.
        // 0x35d: an out-of-bounds STORE's own refusal (owner ruling 2026-10-06).
        Row {
            id: "s4-global-write-minus1",
            subject: src!(S4_GLOBAL_WRITE_MINUS1),
            want: [
                Refused("arena index -1 is not an index"),
                Ran(0x35d, b""),
                Ran(0x35d, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // W-381 STAGE 4 (O4) flipped this row: natively it was Ran(5, b"") on both images.
        // 0x35d: an out-of-bounds STORE's own refusal (owner ruling 2026-10-06).
        Row {
            id: "s4-global-write-minus1-loop",
            subject: src!(S4_GLOBAL_WRITE_MINUS1_LOOP),
            want: [
                Refused("arena index -1 is not an index"),
                Ran(0x35d, b""),
                Ran(0x35d, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // W-381 STAGE 4 (O4) flipped this row: natively it was Ran(0, b"") on both images.
        Row {
            id: "s4-octet-read-minus1",
            subject: src!(S4_OCTET_READ_MINUS1),
            want: [
                Refused("octet -1 is outside a run of 2 octets"),
                Ran(853, b""),
                Ran(853, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // W-381 STAGE 4 (O4) flipped this row: natively it was Fault("BeyondRam") on both images.
        // 0x35d: an out-of-bounds STORE's own refusal (owner ruling 2026-10-06).
        Row {
            id: "s4-far-negative-write",
            subject: src!(S4_FAR_NEGATIVE_WRITE),
            want: [
                Refused("arena index -1000000000 is not an index"),
                Ran(0x35d, b""),
                Ran(0x35d, b""),
            ],
            status: Agrees,
            stage: "- (4 fixed it)",
        },
        // A control: agrees before and after stage 4.
        Row {
            id: "s4-param-write-minus1",
            subject: src!(S4_PARAM_WRITE_MINUS1),
            want: [Refused("(W-359)"), Ran(857, b""), Ran(857, b"")],
            status: Agrees,
            stage: "-",
        },
        // FOUND by W-381 stage 4's probes, NOT a bounds divergence: the
        // interpreter's parser refuses an index of an index (`क अङ्कः ० अन्तः
        // अङ्कः −१ अन्तः`), which the `.t1` front end accepts; natively the store
        // misses the inner run (its length stays 2). The same chained READ
        // answers an address natively (not recorded: it moves with the code).
        Row {
            id: "s4-nested-write-minus1",
            subject: src!(S4_NESTED_WRITE_MINUS1),
            want: [
                Refused("`अङ्कः` is not a name in scope"),
                Ran(2, b""),
                Ran(2, b""),
            ],
            status: Known("W-381 stage 4 FOUND, an index of an index parses only natively"),
            stage: "other: parser",
        },
        // ── W-381 stage 4, the review's findings 1 and 3 ──
        Row {
            id: "s4-far-store-2^61",
            subject: p(s4_far_2_61),
            want: [
                Refused("entry 2305843009213693952 is past the largest run"),
                Ran(0x35d, b""),
                Ran(0x35d, b""),
            ],
            status: Agrees,
            stage: "- (4 refuses it)",
        },
        Row {
            id: "s4-far-store-2^62",
            subject: p(s4_far_2_62),
            want: [
                Refused("entry 4611686018427387904 is past the largest run"),
                Ran(0x35d, b""),
                Ran(0x35d, b""),
            ],
            status: Agrees,
            stage: "- (4 refuses it)",
        },
        Row {
            id: "s4-far-store-2^63-1",
            subject: p(s4_far_2_63_minus_1),
            want: [
                Refused("entry 9223372036854775807 is past the largest run"),
                Ran(0x35d, b""),
                Ran(0x35d, b""),
            ],
            status: Agrees,
            stage: "- (4 refuses it)",
        },
        Row {
            id: "s4-far-store-10^12",
            subject: p(s4_far_10_12),
            want: [
                Refused("entry 1000000000000 is past the largest run"),
                Ran(0x35d, b""),
                Ran(0x35d, b""),
            ],
            status: Agrees,
            stage: "- (4 refuses it)",
        },
        Row {
            id: "s4-far-store-2^61-loop",
            subject: p(s4_far_2_61_loop),
            want: [
                Refused("entry 2305843009213693952 is past the largest run"),
                Ran(0x35d, b""),
                Ran(0x35d, b""),
            ],
            status: Agrees,
            stage: "- (4 refuses it)",
        },
        Row {
            id: "s4-far-store-2^61-global",
            subject: p(s4_far_2_61_global),
            want: [
                Refused("entry 2305843009213693952 is past the largest run"),
                Ran(0x35d, b""),
                Ran(0x35d, b""),
            ],
            status: Agrees,
            stage: "- (4 refuses it)",
        },
        Row {
            id: "s4-store-at-2^25",
            subject: p(s4_at_the_bound),
            want: [
                Refused("entry 33554432 is past the largest run"),
                Ran(0x35d, b""),
                Ran(0x35d, b""),
            ],
            status: Agrees,
            stage: "- (4 refuses it)",
        },
        Row {
            id: "s4-record-element-minus1",
            subject: p(s4_record_element_minus1),
            want: [
                Refused("entry -1 is outside an arena of 1"),
                Ran(0x355, b""),
                Ran(0x355, b""),
            ],
            status: Agrees,
            stage: "- (4: the element READ refuses)",
        },
        // ── V-008 part 2's null record global ──
        Row {
            id: "v008-null-record-global",
            subject: p(v008_null_global),
            want: [
                Ran(7, b""),
                Fault("BadAccess addr 0x0"),
                Fault("BadAccess addr 0x0"),
            ],
            status: Known("V-008 FOUND, a record-typed module global is a null pointer natively"),
            stage: "other: global record initialisation",
        },
        Row {
            id: "v008-record-local-control",
            subject: p(v008_local_control),
            want: [Ran(7, b""), Ran(7, b""), Ran(7, b"")],
            status: Agrees,
            stage: "-",
        },
        // ── W-373 review (a) to (g) ──
        Row {
            id: "w373-a-print-two-args",
            subject: p(w373_a_print_two),
            want: [Ran(3, b"A"), Ran(3, b"A"), Ran(3, b"A")],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "w373-a-wait-two-args",
            subject: Subject::Program(w373_a_wait_two, Some(LOG_5)),
            want: [
                Ran(5, b""),
                NotBuilt("failed 0"),
                NotBuilt("is not defined by any object"),
            ],
            status: Known("W-373 review (a), a two-argument wait waits only in the interpreter"),
            stage: "other: interpreter parser (arity)",
        },
        Row {
            id: "w373-a-user-two-args",
            subject: src!(W373_A_USER_TWO),
            want: [Ran(6, b""), Ran(6, b""), Ran(6, b"")],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "w373-b-undeclared-member",
            subject: src!(W373_B_UNDECLARED),
            want: [Refused("is not a name in scope"), Ran(7, b""), Ran(7, b"")],
            status: Known(
                "W-373 review (b), an undeclared zero-argument qualified name is 0 natively",
            ),
            stage: "other: native stub (ir.t1 cause-1 identifier stub)",
        },
        Row {
            id: "w373-b-print-no-arg",
            subject: p(w373_b_print_none),
            want: [
                Refused("is not the start of an expression"),
                Ran(3, b""),
                Ran(3, b""),
            ],
            status: Known("W-373 review (b), a print with no argument prints nothing natively"),
            stage: "other: native stub (ir.t1 cause-1 identifier stub)",
        },
        Row {
            id: "w373-b-wait-no-arg",
            subject: Subject::Program(w373_b_wait_none, Some(LOG_5)),
            want: [
                Refused("is not the start of an expression"),
                Leftover(0, 0),
                Leftover(0, 0),
            ],
            status: Known("W-373 review (b), a zero-argument wait never waits natively"),
            stage: "other: native stub (ir.t1 cause-1 identifier stub)",
        },
        Row {
            id: "w373-c-tag-last-global",
            subject: Subject::Program(w373_c_tag_last, Some(LOG_44)),
            want: [
                NoEvents("has no global after it"),
                Ran(44, b""),
                Ran(44, b""),
            ],
            status: Known(
                "W-373 review (c), the slot after a module's last global is the next module's",
            ),
            stage: "other: event slot scan (loader)",
        },
        Row {
            id: "w373-d-tag-numeral-in-code",
            subject: Subject::Program(w373_d_tag_in_code, Some(LOG_44)),
            want: [
                Ran(44, b""),
                NoEvents("the SASEVENT tag appears at 2 words"),
                NoEvents("the SASEVENT tag appears at 2 words"),
            ],
            status: Known("W-373 review (d), the native scan matches the tag at any RAM word"),
            stage: "other: event slot scan (loader)",
        },
        Row {
            id: "w373-e-same-global-name",
            subject: p(w373_e_same_global),
            want: [Ran(4444, b""), Ran(4409, b""), Ran(4409, b"")],
            status: Known("W-373 review (e), the interpreter keys globals by bare name"),
            stage: "other: interpreter resolver (global keying)",
        },
        Row {
            id: "w373-f-n64-max-greater",
            subject: p(w373_f_max_greater),
            // AGREES since stage 3 (P1): the native compare is unsigned for a
            // name declared न६४ (it answered 2, the signed reading of MAX).
            want: [Ran(1, b""), Ran(1, b""), Ran(1, b"")],
            status: Agrees,
            stage: "3 (done)",
        },
        Row {
            id: "w373-g-missing-import",
            subject: src!(W373_G_NO_IMPORT),
            want: [
                Ran(3, b"A"),
                NotBuilt("has no declaration"),
                NotBuilt("has no declaration"),
            ],
            status: Known("W-373 review (g), a missing import is accepted by the interpreter"),
            stage: "other: interpreter resolver (imports)",
        },
        // ── V-009 (ii): a matrix product whose shape disagrees, and one in place ──
        Row {
            id: "v009ii-shape-refusal",
            subject: p(|| v009ii_product("फल ऽ क ऽ ख", 5)),
            want: [Refused("0x35a"), Ran(0x35a, b""), Ran(0x35a, b"")],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "v009ii-alias-refusal",
            subject: p(|| v009ii_product("फल ऽ फल ऽ ख", 4)),
            want: [Refused("0x35b"), Ran(0x35b, b""), Ran(0x35b, b"")],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "v009ii-control",
            subject: p(|| v009ii_product("फल ऽ क ऽ ख", 4)),
            want: [Ran(3, b""), Ran(3, b""), Ran(3, b"")],
            status: Agrees,
            stage: "-",
        },
        // ── V-009 (ii) follow-up 1: a routine named like the module's matrix kernel ──
        // The interpreter mirrors the native rule: refused at load iff the module
        // declares the routine AND makes a matrix call (the kernel is then synthesised).
        Row {
            id: "v009ii-kernel-name-product",
            subject: p(|| v009ii_reserved(nirvahana::MATRIX_PRODUCT_MEMBER, true)),
            want: [
                NotLoaded("is reserved for the matrix built-in in a module that uses it"),
                NotBuilt("is reserved for the matrix built-in in a module that uses it"),
                NotBuilt("is reserved for the matrix built-in in a module that uses it"),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "v009ii-kernel-name-transpose",
            subject: p(|| v009ii_reserved(nirvahana::MATRIX_TRANSPOSE_MEMBER, true)),
            want: [
                NotLoaded("is reserved for the matrix built-in in a module that uses it"),
                NotBuilt("is reserved for the matrix built-in in a module that uses it"),
                NotBuilt("is reserved for the matrix built-in in a module that uses it"),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "v009ii-kernel-name-control",
            subject: p(|| v009ii_reserved(nirvahana::MATRIX_PRODUCT_MEMBER, false)),
            want: [Ran(3, b"A"), Ran(3, b"A"), Ran(3, b"A")],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "v009ii-tensor-name-control",
            subject: p(|| v009ii_reserved(nirvahana::TENSOR_MEMBER, true)),
            want: [Ran(3, b"A"), Ran(3, b"A"), Ran(3, b"A")],
            status: Agrees,
            stage: "-",
        },
        // ── follow-ups 2: a GLOBAL named like the module's matrix kernel ──
        Row {
            id: "v009ii-kernel-global-product",
            subject: p(|| v009ii_reserved_global(nirvahana::MATRIX_PRODUCT_MEMBER, true)),
            want: [
                NotLoaded("is reserved for the matrix built-in in a module that uses it"),
                NotBuilt("is reserved for the matrix built-in in a module that uses it"),
                NotBuilt("is reserved for the matrix built-in in a module that uses it"),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "v009ii-kernel-global-transpose",
            subject: p(|| v009ii_reserved_global(nirvahana::MATRIX_TRANSPOSE_MEMBER, true)),
            want: [
                NotLoaded("is reserved for the matrix built-in in a module that uses it"),
                NotBuilt("is reserved for the matrix built-in in a module that uses it"),
                NotBuilt("is reserved for the matrix built-in in a module that uses it"),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "v009ii-kernel-global-control",
            subject: p(|| v009ii_reserved_global(nirvahana::MATRIX_PRODUCT_MEMBER, false)),
            want: [Ran(3, b"A"), Ran(3, b"A"), Ran(3, b"A")],
            status: Agrees,
            stage: "-",
        },
        // ── follow-ups 2 review: two OLDER divergences, pinned (no fix) ──
        Row {
            id: "v009ii-concatenated-label",
            subject: p(v009ii_concatenated_label),
            want: [
                Ran(3, b""),
                NotBuilt("code 2"),
                NotBuilt("more than one object"),
            ],
            status: Known(
                "V-009 (ii) follow-ups 2 review (a): two modules' labels concatenate to one word",
            ),
            stage: "other: interpreter loader (labels)",
        },
        Row {
            id: "v009ii-routine-in-two-sources",
            subject: p(v009ii_routine_in_two_sources),
            want: [
                Ran(3, b""),
                NotBuilt("code 2"),
                NotBuilt("more than one object"),
            ],
            status: Known(
                "V-009 (ii) follow-ups 2 review (b): one routine name in both sources of a module",
            ),
            stage: "other: interpreter loader (two-source merge)",
        },
        // ── V-009 (i)'s header row in the .t1 register index ──
        Row {
            id: "v009i-register-header-row",
            subject: Subject::Register(header_name),
            want: [
                Refused("expected a number, found Nil"),
                Ran(0, b""),
                Want::Absent,
            ],
            status: Known("V-009 (i) FOUND, encode.t1's register index takes the header row"),
            stage: "other: .t1 encoder",
        },
        // ── ADR-0044 D5: a `वर्णाष्टकम्` literal reads the same octets on all three ──
        Row {
            id: "adr0044-d8-text-form",
            subject: p(d8_text_form),
            want: [
                Ran(5, &[0x95, 0xE0, 0xFF, 0x85, 0xE6]),
                Ran(5, &[0x95, 0xE0, 0xFF, 0x85, 0xE6]),
                Ran(5, &[0x95, 0xE0, 0xFF, 0x85, 0xE6]),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-d8-edges",
            subject: p(d8_edges),
            want: [
                Ran(5, &[0x80, 0xBD, 0xE4, 0xF1, 0xFF]),
                Ran(5, &[0x80, 0xBD, 0xE4, 0xF1, 0xFF]),
                Ran(5, &[0x80, 0xBD, 0xE4, 0xF1, 0xFF]),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-d8-pair",
            subject: p(d8_pair),
            want: [
                Ran(3, &[0x87, 0xA4, 0xBF]),
                Ran(3, &[0x87, 0xA4, 0xBF]),
                Ran(3, &[0x87, 0xA4, 0xBF]),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-d8-comment-mark",
            subject: p(d8_comment_mark),
            want: [Ran(3, &[0x95, 0xF0, 0x96]), NotBuilt(""), NotBuilt("")],
            status: Known(
                "W-357 (DROPPED), the same split for उक्तम्: lex.t1 cuts a line at ॰ before \
                 any literal is seen, so the chains refuse what the interpreter reads; the \
                 divergence is stated on `d8_comment_mark`",
            ),
            stage: "other: lex.t1 string-before-comment (ADR-0017's .t1 half)",
        },
        Row {
            id: "adr0044-d8-cross-line",
            subject: p(d8_cross_line),
            want: [NotLoaded("opens a string"), Ran(1, b""), Ran(1, b"")],
            status: Known(
                "ADR-0044 review F5, PRE-EXISTING for उक्तम्: the .t1 chain compiles a literal \
                 spanning two lines that the Rust lexer refuses as unclosed",
            ),
            stage: "other: the one-line rule in the .t1 parser",
        },
        // ── ADR-0044 review: the reviewer's packing rows, all three engines agree ──
        Row {
            id: "adr0044-rev-len1",
            subject: p(adr0044_rev_len1),
            want: [Ran(1, &[0x95]), Ran(1, &[0x95]), Ran(1, &[0x95])],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-rev-len7",
            subject: p(adr0044_rev_len7),
            want: [
                Ran(7, &[0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF]),
                Ran(7, &[0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF]),
                Ran(7, &[0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF]),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-rev-len8",
            subject: p(adr0044_rev_len8),
            want: [
                Ran(8, &[0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6]),
                Ran(8, &[0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6]),
                Ran(8, &[0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6]),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-rev-len9",
            subject: p(adr0044_rev_len9),
            want: [
                Ran(9, &[0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC]),
                Ran(9, &[0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC]),
                Ran(9, &[0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC]),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-rev-len15",
            subject: p(adr0044_rev_len15),
            want: [
                Ran(
                    15,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0,
                    ],
                ),
                Ran(
                    15,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0,
                    ],
                ),
                Ran(
                    15,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0,
                    ],
                ),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-rev-len16",
            subject: p(adr0044_rev_len16),
            want: [
                Ran(
                    16,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0, 0xBF,
                    ],
                ),
                Ran(
                    16,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0, 0xBF,
                    ],
                ),
                Ran(
                    16,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0, 0xBF,
                    ],
                ),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-rev-len17",
            subject: p(adr0044_rev_len17),
            want: [
                Ran(
                    17,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0, 0xBF, 0xE6,
                    ],
                ),
                Ran(
                    17,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0, 0xBF, 0xE6,
                    ],
                ),
                Ran(
                    17,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0, 0xBF, 0xE6,
                    ],
                ),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-rev-len24",
            subject: p(adr0044_rev_len24),
            want: [
                Ran(
                    24,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0,
                    ],
                ),
                Ran(
                    24,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0,
                    ],
                ),
                Ran(
                    24,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0,
                    ],
                ),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-rev-len25",
            subject: p(adr0044_rev_len25),
            want: [
                Ran(
                    25,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0,
                        0xBF,
                    ],
                ),
                Ran(
                    25,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0,
                        0xBF,
                    ],
                ),
                Ran(
                    25,
                    &[
                        0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85,
                        0xFF, 0x80, 0xC0, 0xBF, 0xE6, 0xBC, 0x95, 0xE0, 0x85, 0xFF, 0x80, 0xC0,
                        0xBF,
                    ],
                ),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-rev-marks9",
            subject: p(adr0044_rev_marks9),
            want: [
                Ran(9, &[0x95, 0xBD, 0xE4, 0xE5, 0xF1, 0x96, 0x80, 0xFF, 0xC0]),
                Ran(9, &[0x95, 0xBD, 0xE4, 0xE5, 0xF1, 0x96, 0x80, 0xFF, 0xC0]),
                Ran(9, &[0x95, 0xBD, 0xE4, 0xE5, 0xF1, 0x96, 0x80, 0xFF, 0xC0]),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-rev-utf8shaped",
            subject: p(adr0044_rev_utf8shaped),
            want: [
                Ran(3, &[0xE0, 0xA4, 0x95]),
                Ran(3, &[0xE0, 0xA4, 0x95]),
                Ran(3, &[0xE0, 0xA4, 0x95]),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "adr0044-rev-trailinglead",
            subject: p(adr0044_rev_trailinglead),
            want: [
                Ran(4, &[0xE0, 0xA4, 0x95, 0xE0]),
                Ran(4, &[0xE0, 0xA4, 0x95, 0xE0]),
                Ran(4, &[0xE0, 0xA4, 0x95, 0xE0]),
            ],
            status: Agrees,
            stage: "-",
        },
        Row {
            id: "v009i-register-control",
            subject: Subject::Register(first_register),
            want: [Ran(1, b""), Ran(1, b""), Want::Absent],
            status: Agrees,
            stage: "-",
        },
    ]
}

/// Each row observed on its own thread (an interpreter is not `Send`, so each
/// thread builds its own), at most `W381_JOBS` (default 8) at a time.
fn observe_all(rows: &[Row]) -> Vec<[Got; 3]> {
    let jobs: usize = std::env::var("W381_JOBS")
        .ok()
        .and_then(|j| j.parse().ok())
        .unwrap_or(8)
        .max(1);
    let mut all: Vec<Option<[Got; 3]>> = vec![None; rows.len()];
    for (c, chunk) in rows.chunks(jobs).enumerate() {
        let handles: Vec<_> = chunk
            .iter()
            .map(|r| {
                let s = r.subject;
                std::thread::Builder::new()
                    .stack_size(256 << 20)
                    .spawn(move || observe(s))
                    .expect("a thread")
            })
            .collect();
        for (k, h) in handles.into_iter().enumerate() {
            all[c * jobs + k] = Some(h.join().expect("a probe thread panicked"));
        }
    }
    all.into_iter().map(Option::unwrap).collect()
}

/// THE RATCHET: every row as recorded, outcomes and agreement both.
#[test]
fn w381_engine_agreement_is_as_recorded() {
    let rows = rows();
    let ids: std::collections::BTreeSet<_> = rows.iter().map(|r| r.id).collect();
    assert_eq!(ids.len(), rows.len(), "row ids are unique");
    let got = observe_all(&rows);
    if std::env::var_os("W381_RECORD").is_some() {
        for (r, g) in rows.iter().zip(&got) {
            println!("{}", record_line(r, g));
        }
        return;
    }
    let mut bad = Vec::new();
    for (r, g) in rows.iter().zip(&got) {
        println!(
            "{:<22} {:<28} stage {:<10} {:?}",
            r.id,
            format!("{:?}", r.status),
            r.stage,
            g
        );
        bad.extend(check(r, g));
    }
    let known = rows.iter().filter(|r| r.status != Status::Agrees).count();
    println!(
        "W-381 ratchet: {} rows, {known} known divergences, {} agreeing",
        rows.len(),
        rows.len() - known
    );
    assert!(bad.is_empty(), "the ratchet moved:\n{}", bad.join("\n"));
}

// ── the checker refuses, shown without an engine ──────────────────────────────

fn demo_row(want: [Want; 3], status: Status) -> Row {
    Row {
        id: "demo",
        subject: src!(P12),
        want,
        status,
        stage: "-",
    }
}

/// p12 as main records it: interpreter १, both images ९.
fn p12_today() -> [Got; 3] {
    let ran = |s| Got::Ran {
        status: s,
        out: Vec::new(),
    };
    [ran(1), ran(9), ran(9)]
}

#[test]
fn ratchet_refuses_a_divergence_recorded_as_agreeing() {
    let w = [Want::Ran(1, b""), Want::Ran(9, b""), Want::Ran(9, b"")];
    assert!(check(&demo_row(w, Status::Known("p12")), &p12_today()).is_empty());
    let bad = check(&demo_row(w, Status::Agrees), &p12_today());
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].contains("DIVERGE"), "{bad:?}");
}

#[test]
fn ratchet_refuses_a_fixed_divergence_still_recorded_known() {
    let fixed = [Want::Ran(1, b""), Want::Ran(1, b""), Want::Ran(1, b"")];
    let ran1 = || Got::Ran {
        status: 1,
        out: Vec::new(),
    };
    let bad = check(
        &demo_row(fixed, Status::Known("p12")),
        &[ran1(), ran1(), ran1()],
    );
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].contains("now AGREE"), "{bad:?}");
}

#[test]
fn ratchet_refuses_a_wrong_expectation() {
    let w = [Want::Ran(1, b""), Want::Ran(8, b""), Want::Ran(9, b"")];
    let bad = check(&demo_row(w, Status::Known("p12")), &p12_today());
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].contains(".t1 image gave"), "{bad:?}");
}

fn refused(why: &str, out: &[u8]) -> Got {
    Got::Refused {
        why: why.to_string(),
        out: out.to_vec(),
    }
}

/// THE MUTANT ROW (review of 74e9b26e): the interpreter refuses a fresh-run
/// read having printed nothing, while the native image prints `X` and then
/// RETURNS ८५३ — the W-355 refusal's own status. Recorded as agreeing, the
/// ratchet must go RED; only the same octets make a status a refusal.
#[test]
fn ratchet_refuses_a_native_return_of_853_that_printed() {
    let ran = |out: &[u8]| Got::Ran {
        status: 0x355,
        out: out.to_vec(),
    };
    let interp = refused("entry 0 is outside an arena of 0", b"");
    let mutant = [interp.clone(), ran(b"X"), ran(b"X")];
    let w = [
        Want::Refused("entry 0 is outside an arena of 0"),
        Want::Ran(0x355, b"X"),
        Want::Ran(0x355, b"X"),
    ];
    let bad = check(&demo_row(w, Status::Agrees), &mutant);
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].contains("DIVERGE"), "{bad:?}");
    // The control: the real refusal, nothing printed on any engine, agrees.
    let real = [interp.clone(), ran(b""), ran(b"")];
    let w = [
        Want::Refused("entry 0 is outside an arena of 0"),
        Want::Ran(0x355, b""),
        Want::Ran(0x355, b""),
    ];
    assert!(check(&demo_row(w, Status::Agrees), &real).is_empty());
    // And what the interpreter printed BEFORE refusing must be what the image
    // printed before its refusal store: the same octets agree, others do not.
    assert!(agree(
        &refused("entry 0 is outside an arena of 0", b"A"),
        &ran(b"A")
    ));
    assert!(!agree(
        &refused("entry 0 is outside an arena of 0", b"A"),
        &ran(b"")
    ));
}

#[test]
fn agreement_cuts_the_interpreters_answer_to_the_finishers_48_bits() {
    let ran = |s| Got::Ran {
        status: s,
        out: Vec::new(),
    };
    assert!(agree(&ran(-1), &ran((1 << 48) - 1)));
    assert!(!agree(&ran(1 << 64), &ran(1)));
    assert!(agree(
        &refused("entry 0 is outside an arena of 0", b""),
        &ran(0x355)
    ));
    assert!(!agree(
        &refused("entry 0 is outside an arena of 0", b""),
        &Got::Code(0x355)
    ));
    // A raw (pre-W-381) refusal word is a Code, never a status: the FAIL form's
    // Ran(0x355) does not match a recorded Code(0x355), so a regression to the
    // raw word turns p08 red.
    assert!(!Want::Code(0x355).matches(&ran(0x355)));
    assert!(Want::Code(0x355).matches(&Got::Code(0x355)));
    assert!(!agree(
        &refused("entry 5 is outside an arena of 2", b""),
        &ran(0)
    ));
}

// ── the probes, copied verbatim ────────────────────────────────────────────────

/// `~/b1-safety-probes/p01_local_read_past.t1`, verbatim.
const P01: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    क अङ्कः १ अन्तः भवति ८ ।
    प्रत्यागमनम् क अङ्कः ५ अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p02_octet_read_past.t1`, verbatim.
const P02: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः ख ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    ख अङ्कः ० अन्तः भवति ७ ।
    ख अङ्कः १ अन्तः भवति ८ ।
    प्रत्यागमनम् ख अङ्कः ५ अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p03_global_read_past.t1`, verbatim.
const P03: &str = r#"मण्डलम् परकर्तृ ॥

चरः ग ॱॱ अङ्कः अन्तः न६४ भवति ० ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    ग अङ्कः ० अन्तः भवति ७ ।
    प्रत्यागमनम् ग अङ्कः ५ अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p04_global_fresh_read0.t1`, verbatim.
const P04: &str = r#"मण्डलम् परकर्तृ ॥

चरः ग ॱॱ अङ्कः अन्तः न६४ भवति ० ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    प्रत्यागमनम् ग अङ्कः ० अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p05_param_read_past.t1`, verbatim.
const P05: &str = r#"मण्डलम् परकर्तृ ॥

वृत्तिः ख आदाय क ॱॱ अङ्कः अन्तः न६४ ददाति न६४ आदि
    प्रत्यागमनम् क अङ्कः ५ अन्तः ।
इति

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    प्रत्यागमनम् ख क ।
इति
"#;

/// `~/b1-safety-probes/p06_local_skip_slot.t1`, verbatim.
const P06: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ५ अन्तः भवति ९९ ।
    प्रत्यागमनम् क अङ्कः ३ अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p07_local_grow_len.t1`, verbatim.
const P07: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ५ अन्तः भवति ९९ ।
    प्रत्यागमनम् क ॱ दैर्घ्य योगः क अङ्कः ५ अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p08_fresh_read0.t1`, verbatim.
const P08: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    प्रत्यागमनम् क अङ्कः ० अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p09_callee_store_past.t1`, verbatim.
const P09: &str = r#"मण्डलम् परकर्तृ ॥

वृत्तिः ख आदाय क ॱॱ अङ्कः अन्तः न६४ ददाति न६४ आदि
    क अङ्कः ५ अन्तः भवति ९९ ।
    प्रत्यागमनम् ० ।
इति

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    चरः उ ॱॱ न६४ भवति ख क ।
    प्रत्यागमनम् क अङ्कः ५ अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p10_grow_and_return.t1`, verbatim.
const P10: &str = r#"मण्डलम् परकर्तृ ॥

वृत्तिः ख आदाय क ॱॱ अङ्कः अन्तः न६४ ददाति अङ्कः अन्तः न६४ आदि
    क अङ्कः ५ अन्तः भवति ९९ ।
    प्रत्यागमनम् क ।
इति

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    क भवति ख क ।
    प्रत्यागमनम् क अङ्कः ५ अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p11_grow_dropped.t1`, verbatim.
const P11: &str = r#"मण्डलम् परकर्तृ ॥

वृत्तिः ख आदाय क ॱॱ अङ्कः अन्तः न६४ ददाति अङ्कः अन्तः न६४ आदि
    क अङ्कः ५ अन्तः भवति ९९ ।
    प्रत्यागमनम् क ।
इति

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    चरः ज ॱॱ अङ्कः अन्तः न६४ भवति ख क ।
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति
"#;

/// `~/b1-safety-probes/p12_octet_param_cow.t1`, verbatim.
const P12: &str = r#"मण्डलम् परकर्तृ ॥

वृत्तिः ख आदाय क ॱॱ अङ्कः अन्तः अ८ ददाति न६४ आदि
    क अङ्कः ० अन्तः भवति ९ ।
    प्रत्यागमनम् ० ।
इति

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    क अङ्कः ० अन्तः भवति १ ।
    क अङ्कः १ अन्तः भवति २ ।
    चरः उ ॱॱ न६४ भवति ख क ।
    प्रत्यागमनम् क अङ्कः ० अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p13_arena_param_inplace.t1`, verbatim.
const P13: &str = r#"मण्डलम् परकर्तृ ॥

वृत्तिः ख आदाय क ॱॱ अङ्कः अन्तः न६४ ददाति न६४ आदि
    क अङ्कः ० अन्तः भवति ९ ।
    प्रत्यागमनम् ० ।
इति

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति १ ।
    क अङ्कः १ अन्तः भवति २ ।
    चरः उ ॱॱ न६४ भवति ख क ।
    प्रत्यागमनम् क अङ्कः ० अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p14_add_overflow.t1`, verbatim.
const P14: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अ६४ भवति ९२२३३७२०३६८५४७७५८०७ ।
    क भवति क योगः १ ।
    यदि क न्यूनम् ० आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् २ ।
इति
"#;

/// `~/b1-safety-probes/p15_unsigned_cmp.t1`, verbatim.
const P15: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति १ वामसृ ६३ ।
    यदि क न्यूनम् ० आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् २ ।
इति
"#;

/// `~/b1-safety-probes/p16_underflow_cmp.t1`, verbatim.
const P16: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ० ।
    क भवति क वियोगः १ ।
    यदि क न्यूनम् ० आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् २ ।
इति
"#;

/// `~/b1-safety-probes/p17_shl64.t1`, verbatim.
const P17: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति १ ।
    चरः श ॱॱ न६४ भवति ६४ ।
    चरः ल ॱॱ न६४ भवति क वामसृ श ।
    प्रत्यागमनम् ल ।
इति
"#;

/// `~/b1-safety-probes/p18_srl_srA.t1`, verbatim.
const P18: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः उ ॱॱ न६४ भवति ऋण१ ।
    चरः स ॱॱ अ६४ भवति ऋण१ ।
    चरः अ ॱॱ न६४ भवति उ दक्षिणसृ ६३ ।
    चरः ब ॱॱ अ६४ भवति स दक्षिणसृ ६३ ।
    यदि ब समम् ऋण१ आदि
        प्रत्यागमनम् अ योगः १० ।
    इति
    प्रत्यागमनम् अ ।
इति
"#;

/// `~/b1-safety-probes/p19_div0.t1`, verbatim.
const P19: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति ७ ।
    चरः श ॱॱ न६४ भवति ० ।
    चरः ल ॱॱ न६४ भवति क विभाजनम् श ।
    प्रत्यागमनम् ल योगः १ ।
इति
"#;

/// `~/b1-safety-probes/p20_octet_store_300.t1`, verbatim.
const P20: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः ख ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    ख अङ्कः ० अन्तः भवति ३०० ।
    प्रत्यागमनम् ख अङ्कः ० अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p21_mul_wrap.t1`, verbatim.
const P21: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ न६४ भवति १ वामसृ ६२ ।
    चरः ल ॱॱ न६४ भवति क गुणनम् ८ ।
    यदि ल समम् ० आदि
        प्रत्यागमनम् १ ।
    इति
    प्रत्यागमनम् २ ।
इति
"#;

/// `~/b1-safety-probes/p22_negative_index.t1`, verbatim.
const P22: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    क अङ्कः १ अन्तः भवति ८ ।
    प्रत्यागमनम् क अङ्कः ऋण१ अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p23_far_read.t1`, verbatim.
const P23: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    प्रत्यागमनम् क अङ्कः १००००००००० अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p24_read_into_neighbour.t1`, verbatim.
const P24: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    क अङ्कः ० अन्तः भवति ७ ।
    चरः ख ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    ख अङ्कः ० अन्तः भवति ५५ ।
    चरः इ ॱॱ न६४ भवति १ ।
    यावत् इ न्यूनम् ४००० आदि
        यदि क अङ्कः इ अन्तः असमम् ० आदि
            प्रत्यागमनम् इ गुणनम् १००० योगः क अङ्कः इ अन्तः ।
        इति
        इ भवति इ योगः १ ।
    इति
    प्रत्यागमनम् ७ ।
इति
"#;

/// `~/b1-safety-probes/p25_record_unwritten.t1`, verbatim.
const P25: &str = r#"मण्डलम् परकर्तृ ॥

संरचना वस्तु आरभ्य
  अ ॱॱ न६४ ऽ
  ब ॱॱ न६४
समाप्तम् ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः र ॱॱ वस्तु भवति ० ।
    र ॱ अ भवति ५ ।
    प्रत्यागमनम् र ॱ ब योगः १ ।
इति
"#;

/// `~/b1-safety-probes/p26_global_grow.t1`, verbatim.
const P26: &str = r#"मण्डलम् परकर्तृ ॥

चरः ग ॱॱ अङ्कः अन्तः न६४ भवति ० ।
चरः घ ॱॱ अङ्कः अन्तः न६४ भवति ० ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    घ अङ्कः ० अन्तः भवति ३ ।
    ग अङ्कः ५०० अन्तः भवति ९ ।
    प्रत्यागमनम् ग अङ्कः ५०० अन्तः गुणनम् १० योगः घ अङ्कः ० अन्तः योगः ग ॱ दैर्घ्य गुणनम् १०० ।
इति
"#;

/// `w338_write_cost.rs`'s `GAP`, verbatim (the W-338 review's gap-slot case, in a loop).
const W338_GAP: &str = "मण्डलम् लेखव्यय ॥
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

/// `v008_vector_lowering.rs`'s `RECORD`, verbatim.
const RECORD: &str = "संरचना अभिलेखः आरभ्य
    प्लवयोगः ॱॱ न६४
समाप्तम् ।
";

/// `t1_image`'s provenance digests with `sadhana::t1::agreement::sha256`, a copy
/// of `yantra::smp::sha256` (sadhana does not depend on yantra). The two must be
/// the same function: every length across the padding boundary, and a corpus file.
#[test]
fn the_provenance_digest_is_yantras_sha256() {
    let data: Vec<u8> = (0..300u32).map(|i| (i * 131 + 7) as u8).collect();
    for n in 0..data.len() {
        assert_eq!(
            sadhana::t1::agreement::sha256(&data[..n]),
            yantra::smp::sha256(&data[..n]),
            "length {n}"
        );
    }
    let ir = std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src/ir.t1"))
        .expect("ir.t1");
    assert_eq!(
        sadhana::t1::agreement::sha256(&ir),
        yantra::smp::sha256(&ir)
    );
}
