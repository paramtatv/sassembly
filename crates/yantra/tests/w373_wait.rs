//! **`W-373`: THE `.t1` WAIT INTRINSIC, IN BOTH ENGINES, READING EVENTS FROM THE SAME LOG.**
//!
//! ADR-0040 Option C, R4, as the owner ruled it on 2026-10-05 (BACKLOG row `W-373`):
//! a ONE-TOKEN qualified call into `अष्टक` with ONE argument, recognised by its exact
//! qualified name on both sides and declared nowhere. Natively it is one 8-octet store
//! to [`WAIT`] (`ir.t1`, a copy of the print arm's store); interpreted it is LOG REPLAY:
//! at wait `k` the interpreter writes `log[k]` into the global the program declared
//! right after its `SASEVENT` tag ([`EVENT_TAG`]) and continues — what
//! [`yantra::input::replay`] does at each [`Halt::Wait`].
//!
//! The log is parsed ONCE, by [`parse_event_log`], and the same `Vec<u64>` goes to the
//! native replay and to [`Interpreter::set_events`].
//!
//! THE FALSIFIERS, each on THREE engines where it applies — the interpreter, the image
//! the `.t1` chain builds, and the image `riscv64.rs` (the Rust twin) builds:
//!
//! - (a) three waits on the log `[3, 5, 7]`, the running sum printed after each:
//!   byte-identical output and the same status everywhere;
//! - (b) a log with `t=` clock records replays identically;
//! - (c) a short log, a long log, a missing tag, a duplicated tag and no log at all
//!   refuse in the same CLASS on every engine — the classes are `yantra-run`'s, and
//!   the real binary is run on the `.t1` image to anchor them;
//! - (d) two mutants of the native lowering, in a COPY of `ir.t1` — the store aimed at
//!   the UART, and the store dropped — each go red against the interpreter;
//! - (e) an interpreter that delivers `log[k + 1]` goes red against the native run;
//! - (f) the interpreter's `EVENT_TAG` is `yantra`'s, and its member name is the
//!   ruling's, byte for byte.
//!
//! And the TOKEN RATCHET moved here from `W-372`: the member token appears in NO corpus
//! code, the recogniser's literal exactly once in `ir.t1`, and the tag's numeral nowhere
//! — with a mutation control that inserts a call into a copy of a corpus file.
//!
//! EVERY NUMERAL IS GENERATED from a Rust constant ([`dec`]), and the member name is
//! read from `nirvahana.rs`'s constant, never typed here.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{self, Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use yantra::input::{EVENT_TAG, Replayed, find_event_slot, parse_event_log, replay};
use yantra::{Halt, Machine, UART, WAIT};

const FUEL: u64 = 80_000_000_000;
const STEPS: u64 = 200_000_000;
const MODULE: &str = "घटनापरीक्षण";
/// What the event word holds before any delivery: NOT zero, so a run that resumed
/// without delivering prints sums of this rather than something that passes for empty.
const UNDELIVERED: u64 = 1000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src")
}

/// The qualified call, built from the interpreter's two constants.
fn qualified() -> String {
    format!(
        "{}\u{971}{}",
        nirvahana::WAIT_BUILTIN_MODULE,
        nirvahana::WAIT_BUILTIN_MEMBER
    )
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

/// A decimal numeral for a `u64`, in the corpus's digits.
fn dec(n: u64) -> String {
    const DIGITS: [&str; 10] = ["०", "१", "२", "३", "४", "५", "६", "७", "८", "९"];
    n.to_string()
        .chars()
        .map(|c| DIGITS[c.to_digit(10).unwrap() as usize])
        .collect()
}

/// A pattern as a T1 hexadecimal numeral (`grammar-t1.ebnf` `hex_digit`), for the
/// ratchet only: the tag must not appear in the corpus in EITHER spelling.
fn hex(v: u64) -> String {
    const DIGITS: [&str; 16] = [
        "०", "१", "२", "३", "४", "५", "६", "७", "८", "९", "अ", "आ", "इ", "ई", "उ", "ऊ",
    ];
    let s: String = format!("{v:x}")
        .chars()
        .map(|c| DIGITS[c.to_digit(16).unwrap() as usize])
        .collect();
    format!("०षोड्{s}")
}

/// Eight little-endian octets per word through the output channel, as
/// `v005_floats.rs`'s printer (the finisher keeps only 48 bits of a status).
const PRINTER: &str = "वृत्तिः शब्दमुद्रणम् आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि
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

/// How the program declares the event interface.
#[derive(Clone, Copy, Debug)]
enum Tags {
    /// No tag: the slot is preceded by a zero word, the image built without the interface.
    Missing,
    /// One tag, the slot right after it.
    One,
    /// Two tags, each with a slot after it: the scan cannot tell which one is read.
    Duplicated,
}

/// THE PROGRAM: waits three times, adding the event word into a running sum after each
/// wait and printing the sum, then answers ०. The wait's argument is the loop counter —
/// a word the native store carries and both engines ignore.
fn program(tags: Tags) -> String {
    let tag = match tags {
        Tags::Missing => dec(0),
        Tags::One | Tags::Duplicated => dec(EVENT_TAG),
    };
    let mut globals = format!(
        "सार्वजनिक चरः घटनासङ्केतक ॱॱ न६४ भवति {tag} ।\n\
         सार्वजनिक चरः घटनामूल्यक ॱॱ न६४ भवति {u} ।\n",
        u = dec(UNDELIVERED)
    );
    if matches!(tags, Tags::Duplicated) {
        globals.push_str(&format!(
            "सार्वजनिक चरः घटनासङ्केतख ॱॱ न६४ भवति {tag} ।\n\
             सार्वजनिक चरः घटनामूल्यख ॱॱ न६४ भवति {u} ।\n",
            u = dec(UNDELIVERED)
        ));
    }
    format!(
        "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

{globals}
{PRINTER}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः योगफलम् ॱॱ न६४ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    चरः अवगणना ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् ३ आदि
        अवगणना भवति {wait} क्रमः ।
        योगफलम् भवति योगफलम् योगः घटनामूल्यक ।
        अवगणना भवति शब्दमुद्रणम् योगफलम् ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    प्रत्यागमनम् ० ।
इति
",
        wait = qualified()
    )
}

/// The octets the program prints for a log it consumed: each running sum, eight
/// little-endian octets — computed here, from the log, independently of both engines.
fn sums(log: &[u64]) -> Vec<u8> {
    let mut sum = 0u64;
    let mut out = Vec::new();
    for r in log {
        sum = sum.wrapping_add(*r);
        out.extend_from_slice(&sum.to_le_bytes());
    }
    out
}

// ── what a run came to ───────────────────────────────────────────────────────

/// ONE VOCABULARY FOR BOTH ENGINES: a run ends in one of `yantra-run`'s classes. The
/// octets printed before the end are carried too, so a refusal is compared on what the
/// program had already written, not only on its name.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    /// Finished, every record consumed: the status and the octets.
    Finished { status: u64, out: Vec<u8> },
    /// `Replayed::Short`: wait `index` had no record; the program was not resumed.
    Short { index: usize, out: Vec<u8> },
    /// Finished with records left over: `yantra-run`'s "LONGER than the run".
    Long {
        delivered: usize,
        len: usize,
        out: Vec<u8>,
    },
    /// A wait with no event source: `yantra-run`'s exit 75.
    NoSource { out: Vec<u8> },
    /// Refused before running: no `SASEVENT` tag.
    NoTag,
    /// Refused before running: the tag twice.
    DuplicatedTag,
    /// Anything else, said in full.
    Other(String),
}

/// A load-time refusal, by `find_event_slot`'s words (the interpreter's are written to
/// fall in the same classes).
fn refusal_class(why: &str) -> Outcome {
    if why.starts_with("no event interface") {
        Outcome::NoTag
    } else if why.starts_with("the SASEVENT tag appears at") {
        Outcome::DuplicatedTag
    } else {
        Outcome::Other(why.to_string())
    }
}

/// THE NATIVE SIDE, as `yantra-run` judges it: the tag found at load when a log is
/// given, then [`replay`], a long log refused after a finish, a wait without a log the
/// exit-75 pause.
fn native_outcome(image: &[u8], log: Option<&[u64]>) -> Outcome {
    let mut m = Machine::load_elf(image, yantra::ram_for(image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let Some(log) = log else {
        return match m.run(STEPS, &mut out) {
            Halt::Wait { .. } => Outcome::NoSource { out },
            Halt::Finisher {
                status: Some(status),
                ..
            } => Outcome::Finished { status, out },
            h => Outcome::Other(format!("halted {h:?}")),
        };
    };
    let tag = match find_event_slot(&m.mem) {
        Ok(t) => t,
        Err(e) => return refusal_class(&e),
    };
    match replay(&mut m, tag, log, STEPS, &mut out) {
        Replayed::Short { index, .. } => Outcome::Short { index, out },
        Replayed::Halted {
            halt:
                Halt::Finisher {
                    status: Some(status),
                    ..
                },
            delivered,
        } => {
            if delivered < log.len() {
                Outcome::Long {
                    delivered,
                    len: log.len(),
                    out,
                }
            } else {
                Outcome::Finished { status, out }
            }
        }
        Replayed::Halted { halt, delivered } => {
            Outcome::Other(format!("halted {halt:?} after {delivered} records"))
        }
    }
}

/// THE INTERPRETED SIDE: the chain loaded beside the probe, as `t1_image --load` does;
/// the log handed to [`Interpreter::set_events`] before the entry runs. `deliver` is
/// what the interpreter is GIVEN — the log itself, except in the (e) mutant.
fn interpreted_with(src: &str, log: Option<&[u64]>, deliver: Option<&[u64]>) -> Outcome {
    let file = format!("{MODULE}.t1");
    let mut srcs: Vec<(&str, &str)> = CHAIN.to_vec();
    srcs.push((file.as_str(), src));
    let mut it = Interpreter::load(&srcs, &spec_root())
        .unwrap_or_else(|e| panic!("the probe must load: {}", e.reason));
    if let Some(d) = deliver
        && let Err(e) = it.set_events(d)
    {
        return refusal_class(&e.reason);
    }
    match it.call(&format!("{MODULE}\u{971}मुख्यम्"), Vec::new(), FUEL) {
        Ok(v) => {
            let out = it.sink().to_vec();
            let status = v.as_int().map_or(u64::MAX, |s| s as u64);
            match log {
                Some(l) if it.events_delivered() < l.len() => Outcome::Long {
                    delivered: it.events_delivered(),
                    len: l.len(),
                    out,
                },
                _ => Outcome::Finished { status, out },
            }
        }
        Err(e) => {
            let out = it.sink().to_vec();
            let r = &e.reason;
            if let Some(rest) = r.split("wait index ").nth(1)
                && r.contains("SHORTER")
            {
                let index = rest
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(usize::MAX);
                Outcome::Short { index, out }
            } else if r.starts_with("WAIT") && r.contains("no event source") {
                Outcome::NoSource { out }
            } else {
                Outcome::Other(r.clone())
            }
        }
    }
}

fn interpreted(src: &str, log: Option<&[u64]>) -> Outcome {
    interpreted_with(src, log, log)
}

/// THE `.t1` CHAIN'S IMAGE: `src` compiled by the given compiler sources, running in
/// the interpreter (`v005_floats.rs`'s `native_with`, answering the image).
fn t1_image_with(chain: &[(&str, &str)], src: &str) -> Vec<u8> {
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
    assert!(
        !image.is_empty(),
        "the probe built no image: refusal {:?}, link {:?}",
        sadhana::t1::chain::refusal_site(&it),
        sadhana::t1::chain::link_refusals(&it)
    );
    image
}

/// THE RUST TWIN: the same `.t1` front end, then `riscv64.rs`, the Rust assembler and
/// linker (`v005_floats.rs`'s `rust_twin_image`).
fn rust_twin_image(src: &str) -> Vec<u8> {
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
    let module_text = riscv64::emit_module(&module).expect("the Rust emitter emits");
    let startup_text = riscv64::emit_startup_object_with_records(
        Some(&format!("{MODULE}मुख्यम्")),
        riscv64::module_allocates(&module),
    );
    let to_object = |text: &str, name: Option<&str>| -> vastu::Object {
        let bytes = assemble_object(text, name, Target::Uncompressed, false, Language::English)
            .unwrap_or_else(|ds| {
                panic!("{name:?} does not assemble: {ds:?}\n--- text ---\n{text}")
            });
        vastu::read(&bytes).unwrap_or_else(|| panic!("{name:?} does not read back"))
    };
    let startup = to_object(&startup_text, Some("यन्त्रारम्भ"));
    let module_obj = to_object(&module_text, Some(MODULE));
    let linked = sadhana::samyojana::link_at(&[startup, module_obj], LOAD)
        .unwrap_or_else(|es| panic!("the Rust path does not link: {es:?}"));
    sadhana::kosha::write_debuggable_at(&linked.text, &linked.data, &[], linked.bss, &[], LOAD)
}

/// Each image is built ONCE per test binary: a `.t1` compile through the interpreter is
/// the slow part of every test here.
fn image(which: Tags, twin: bool) -> &'static [u8] {
    static T1: [OnceLock<Vec<u8>>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    static TWIN: [OnceLock<Vec<u8>>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let k = which as usize;
    if twin {
        TWIN[k].get_or_init(|| rust_twin_image(&program(which)))
    } else {
        T1[k].get_or_init(|| t1_image_with(CHAIN, &program(which)))
    }
}

/// The three engines over one program and one log: each outcome, labelled.
fn three(which: Tags, log: Option<&[u64]>) -> [(&'static str, Outcome); 3] {
    [
        ("interpreter", interpreted(&program(which), log)),
        (".t1 image", native_outcome(image(which, false), log)),
        ("rust twin", native_outcome(image(which, true), log)),
    ]
}

fn assert_all(what: &str, got: &[(&str, Outcome)], want: &Outcome) {
    for (engine, o) in got {
        println!("{what}, {engine}: {o:?}");
    }
    for (engine, o) in got {
        assert_eq!(o, want, "{what}: the {engine} differs");
    }
}

// ── (a) three waits on [3, 5, 7] ─────────────────────────────────────────────

#[test]
fn w373_a_three_waits_on_3_5_7_agree_on_every_engine() {
    let log = parse_event_log("# W-373 (a)\n3\n5\n7\n").expect("the log parses");
    let want = Outcome::Finished {
        status: 0,
        out: sums(&log),
    };
    assert_all("(a) [3,5,7]", &three(Tags::One, Some(&log)), &want);
}

// ── (b) a log with clock records ─────────────────────────────────────────────

#[test]
fn w373_b_a_clock_log_replays_identically() {
    let text = format!(
        "{}\nt=1791000000123456789\n7\nt=0x10\n",
        yantra::input::RECORDED_LOG_HEADER
    );
    let log = parse_event_log(&text).expect("the log parses");
    assert_eq!(log.len(), 3, "three records, one a plain word");
    let want = Outcome::Finished {
        status: 0,
        out: sums(&log),
    };
    assert_all("(b) t= log", &three(Tags::One, Some(&log)), &want);
}

// ── (c) the refusals, alike ──────────────────────────────────────────────────

#[test]
fn w373_c_a_short_log_refuses_alike_at_wait_two() {
    let log = parse_event_log("3\n5\n").unwrap();
    let want = Outcome::Short {
        index: 2,
        out: sums(&log),
    };
    assert_all("(c) short", &three(Tags::One, Some(&log)), &want);
}

#[test]
fn w373_c_a_long_log_refuses_alike() {
    let log = parse_event_log("3\n5\n7\n9\n").unwrap();
    let want = Outcome::Long {
        delivered: 3,
        len: 4,
        out: sums(&log[..3]),
    };
    assert_all("(c) long", &three(Tags::One, Some(&log)), &want);
}

#[test]
fn w373_c_a_missing_tag_refuses_alike_at_load() {
    let log = parse_event_log("3\n5\n7\n").unwrap();
    assert_all(
        "(c) no tag",
        &three(Tags::Missing, Some(&log)),
        &Outcome::NoTag,
    );
}

#[test]
fn w373_c_a_duplicated_tag_refuses_alike_at_load() {
    let log = parse_event_log("3\n5\n7\n").unwrap();
    assert_all(
        "(c) two tags",
        &three(Tags::Duplicated, Some(&log)),
        &Outcome::DuplicatedTag,
    );
}

#[test]
fn w373_c_a_wait_with_no_log_pauses_alike() {
    assert_all(
        "(c) no log",
        &three(Tags::One, None),
        &Outcome::NoSource { out: Vec::new() },
    );
}

/// THE CLASSES ARE `yantra-run`'S: the real binary on the `.t1` image, for each case
/// above — exit 0 and the octets, exit 1 naming SHORTER / LONGER / the tag, exit 75.
#[test]
fn w373_c_the_classes_are_the_runners_own() {
    let dir = std::env::temp_dir().join(format!(
        "w373-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let run = |which: Tags, log: Option<&str>| {
        let elf = dir.join(format!("{which:?}.elf"));
        std::fs::write(&elf, image(which, false)).expect("write the image");
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_yantra-run"));
        if let Some(log) = log {
            let path = dir.join("events.log");
            std::fs::write(&path, log).expect("write the log");
            cmd.arg("--events").arg(&path);
        }
        let o = cmd.arg(&elf).output().expect("yantra-run runs");
        (
            o.status.code(),
            o.stdout,
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    };
    let refused = |err: &str, what: &str| {
        err.lines()
            .any(|l| l.to_lowercase().starts_with("events: refused") && l.contains(what))
    };
    let (code, out, err) = run(Tags::One, Some("3\n5\n7\n"));
    assert_eq!((code, out), (Some(0), sums(&[3, 5, 7])), "finished:\n{err}");
    let (code, _, err) = run(Tags::One, Some("3\n5\n"));
    assert!(
        code == Some(1) && refused(&err, "SHORTER") && err.contains("wait index 2"),
        "short:\n{err}"
    );
    let (code, _, err) = run(Tags::One, Some("3\n5\n7\n9\n"));
    assert!(code == Some(1) && refused(&err, "LONGER"), "long:\n{err}");
    let (code, _, err) = run(Tags::Missing, Some("3\n5\n7\n"));
    assert!(
        code == Some(1) && refused(&err, "no event interface"),
        "no tag:\n{err}"
    );
    let (code, _, err) = run(Tags::Duplicated, Some("3\n5\n7\n"));
    assert!(
        code == Some(1) && refused(&err, "the SASEVENT tag appears at 2"),
        "two tags:\n{err}"
    );
    let (code, _, err) = run(Tags::One, None);
    assert!(
        code == Some(75) && err.lines().any(|l| l.starts_with("yantra-run: WAIT")),
        "no log:\n{err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ── (d) the lowering's mutants ───────────────────────────────────────────────

/// `CHAIN` with `ir.t1`'s ONE occurrence of `live` replaced by `mutant`.
fn mutated_ir(live: &str, mutant: &str) -> Vec<(&'static str, String)> {
    CHAIN
        .iter()
        .map(|(n, s)| {
            if *n == "ir.t1" {
                assert_eq!(
                    s.matches(live).count(),
                    1,
                    "the mutation must match exactly one site of ir.t1: {live}"
                );
                (*n, s.replacen(live, mutant, 1))
            } else {
                (*n, (*s).to_string())
            }
        })
        .collect()
}

/// The `.t1` image a mutated chain builds, run on `log` — or the reason none was built.
fn mutant_outcome(chain: &[(&'static str, String)], log: &[u64]) -> Outcome {
    let refs: Vec<(&str, &str)> = chain.iter().map(|(n, s)| (*n, s.as_str())).collect();
    let src = program(Tags::One);
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t1_image_with(&refs, &src))) {
        Ok(img) => native_outcome(&img, Some(log)),
        Err(p) => Outcome::Other(
            p.downcast_ref::<String>()
                .cloned()
                .unwrap_or_else(|| "a panic with no message".to_string()),
        ),
    }
}

/// The store's address in `ir.t1`, as the lowering spells it.
fn wait_site() -> String {
    format!("ध्रुवरचना {} ।", dec(WAIT))
}

fn assert_mutant_red(what: &str, chain: &[(&'static str, String)]) {
    let log = [3, 5, 7];
    let got = mutant_outcome(chain, &log);
    let want = interpreted(&program(Tags::One), Some(&log));
    println!("{what}: mutant {got:?}\n{what}: interpreter {want:?}");
    assert_eq!(
        want,
        Outcome::Finished {
            status: 0,
            out: sums(&log)
        },
        "the control: the interpreter itself is right"
    );
    assert_ne!(got, want, "{what}: the mutant agreed with the interpreter");
}

/// MUTANT 1: the wait's store aimed at the UART. Nothing waits, so the log is never
/// consumed, and the stored word is printed as an octet.
#[test]
fn w373_d_mutant_store_to_the_uart_is_red() {
    let chain = mutated_ir(&wait_site(), &format!("ध्रुवरचना {} ।", dec(UART)));
    assert_mutant_red("store to the UART", &chain);
}

/// MUTANT 2: the store dropped. The call answers ० and nothing waits.
#[test]
fn w373_d_mutant_store_dropped_is_red() {
    let chain = mutated_ir(DROP_LIVE, "");
    assert_mutant_red("store dropped", &chain);
}

/// The store, quoted from `ir.t1`'s out-of-line lowering (it must match once).
const DROP_LIVE: &str =
    "चरः अवगणनप्रतीक्षणनिधान ॱॱ न६४ भवति स्थाननिधानरचना आरभ्य प्रतीक्षणस्थानम् ऽ प्रतीक्षणमूल्यम् समाप्तम् ।";

// ── (e) the interpreter's mutant ─────────────────────────────────────────────

/// AN INTERPRETER THAT DELIVERS `log[k + 1]` AT WAIT `k`, observed from outside: it is
/// handed the log less its first record, which is exactly what such an interpreter
/// reads — `log[1]`, `log[2]`, then nothing at wait 2. It must go red against the
/// native run of the same log. (`nirvahana.rs` cannot be mutated per test; what is
/// mutated is the record each wait sees, which is the whole of the claim.)
#[test]
fn w373_e_an_interpreter_delivering_the_next_record_is_red() {
    let log = [3, 5, 7];
    let native = native_outcome(image(Tags::One, false), Some(&log));
    let mutant = interpreted_with(&program(Tags::One), Some(&log), Some(&log[1..]));
    println!("native {native:?}\nmutant {mutant:?}");
    assert_eq!(
        native,
        Outcome::Finished {
            status: 0,
            out: sums(&log)
        },
        "the control: the native run itself is right"
    );
    assert_ne!(
        mutant, native,
        "a k+1 interpreter agreed with the native run"
    );
}

// ── (g) the record is 64 BITS, read through the slot's declared type ─────────

/// One wait into a slot declared `slot_ty`, then `body` — which reads the slot as
/// `घटनामूल्यक` and prints through `शब्दमुद्रणम्` / `अष्टकॱमुद्रणम्`.
///
/// NATIVELY THE HOST WRITES 64 BITS AND THE PROGRAM'S TYPE READS THEM: a `प६४` slot is
/// a double's bit pattern, an `अ६४` slot a two's-complement word. The interpreter must
/// reinterpret the same 64 bits the same way (review finding 1 at 1540744e).
fn typed_program(slot_ty: &str, body: &str) -> String {
    format!(
        "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

सार्वजनिक चरः घटनासङ्केतक ॱॱ न६४ भवति {tag} ।
सार्वजनिक चरः घटनामूल्यक ॱॱ {slot_ty} भवति ० ।

{PRINTER}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    अवगणना भवति {wait} ० ।
{body}
    प्रत्यागमनम् ० ।
इति
",
        tag = dec(EVENT_TAG),
        wait = qualified()
    )
}

/// The three engines over a source built here (no image cache).
fn three_src(src: &str, log: &[u64]) -> [(&'static str, Outcome); 3] {
    [
        ("interpreter", interpreted(src, Some(log))),
        (
            ".t1 image",
            native_outcome(&t1_image_with(CHAIN, src), Some(log)),
        ),
        (
            "rust twin",
            native_outcome(&rust_twin_image(src), Some(log)),
        ),
    ]
}

/// A `प६४` SLOT: the record `0x4000000000000000` is the double 2.0, and moving its bits
/// back out (`अष्टकॱप्लवसंचारः`) prints the record itself.
#[test]
fn w373_g_a_float_slot_reads_the_records_bits() {
    const RECORD: u64 = 0x4000_0000_0000_0000;
    let src = typed_program(
        "प६४",
        "    चरः ब ॱॱ न६४ भवति अष्टकॱप्लवसंचारः घटनामूल्यक ।
    अवगणना भवति शब्दमुद्रणम् ब ।",
    );
    let want = Outcome::Finished {
        status: 0,
        out: RECORD.to_le_bytes().to_vec(),
    };
    assert_all("(g) प६४ slot", &three_src(&src, &[RECORD]), &want);
}

/// An `अ६४` SLOT: the record `2^63 + 6` is NEGATIVE to a signed word. Halved it is
/// `-4611686018427387901` (printed as its 64 bits, `13835058055282163715`), and it is
/// below ०, so the program prints `N` and then `P`.
#[test]
fn w373_g_a_signed_slot_reads_the_record_as_twos_complement() {
    const RECORD: u64 = (1 << 63) + 6;
    let src = typed_program(
        "अ६४",
        &format!(
            "    चरः अर्धम् ॱॱ अ६४ भवति घटनामूल्यक विभाजनम् २ ।
    अवगणना भवति शब्दमुद्रणम् अर्धम् ।
    यदि घटनामूल्यक न्यूनम् ० आदि
        अवगणना भवति अष्टकॱमुद्रणम् {n} ।
    इति
    अवगणना भवति अष्टकॱमुद्रणम् {p} ।",
            n = dec(u64::from(b'N')),
            p = dec(u64::from(b'P'))
        ),
    );
    let half = ((RECORD as i64) / 2) as u64;
    assert_eq!(
        half, 13_835_058_055_282_163_715,
        "the arithmetic, checked here"
    );
    let mut out = half.to_le_bytes().to_vec();
    out.extend_from_slice(b"NP");
    let want = Outcome::Finished { status: 0, out };
    assert_all("(g) अ६४ slot", &three_src(&src, &[RECORD]), &want);
}

// ── (f) one tag, one name ────────────────────────────────────────────────────

#[test]
fn w373_f_the_interpreters_event_tag_is_yantras() {
    assert_eq!(nirvahana::EVENT_TAG, EVENT_TAG);
    assert_eq!(&nirvahana::EVENT_TAG.to_le_bytes(), b"SASEVENT");
}

/// The member name in `nirvahana.rs` is the OWNER'S RULING, byte for byte: read out of
/// BACKLOG row `W-373` ("(1) NAME <qualified>"), never retyped.
#[test]
#[ignore = "blocked: needs BACKLOG.tsv not in the public repository"]
fn w373_f_the_name_is_the_rulings() {
    let backlog =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../BACKLOG.tsv"))
            .expect("BACKLOG.tsv");
    let row = backlog
        .lines()
        .find(|l| l.starts_with("W-373\t"))
        .expect("the W-373 row");
    let ruled = row
        .split("(1) NAME ")
        .nth(1)
        .and_then(|r| r.split_whitespace().next())
        .expect("the row names the intrinsic");
    assert_eq!(qualified(), ruled);
}

// ── THE TOKEN RATCHET (moved here from `W-372`) ──────────────────────────────

/// A corpus file's CODE, line by line: `॰` comments cut, `उक्तम् … इति` string literals
/// dropped (a literal may span lines). Answers `(line, token)` for every code token.
fn code_tokens(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_string = false;
    for (n, line) in text.lines().enumerate() {
        let code = line.split('॰').next().unwrap_or("");
        for tok in code.split_whitespace() {
            if in_string {
                if tok == "इति" {
                    in_string = false;
                }
                continue;
            }
            if tok == "उक्तम्" {
                in_string = true;
                continue;
            }
            out.push((n + 1, tok.to_string()));
        }
    }
    out
}

/// Every place the member token appears in CODE, as `file:line: token` — a WHOLE token
/// equal to it or any token CONTAINING it (sandhi and the qualified spelling both hide
/// it inside a longer token).
fn member_in_code(files: &[(String, String)]) -> Vec<String> {
    let member = nirvahana::WAIT_BUILTIN_MEMBER;
    let mut hits = Vec::new();
    for (name, text) in files {
        for (line, tok) in code_tokens(text) {
            if tok.contains(member) {
                let how = if tok == member { "whole" } else { "substring" };
                hits.push(format!("{name}:{line}: {tok} ({how})"));
            }
        }
    }
    hits
}

fn corpus() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = std::fs::read_dir(corpus_root())
        .expect("crates/sadhana-t1/src")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).expect("a corpus file"),
            )
        })
        .collect();
    files.sort();
    assert!(files.len() >= 20, "the corpus is {} files", files.len());
    files
}

/// The corpus never CALLS the wait (W-372's image half refuses a Stage 2 that waits; this
/// is the source half): 0 whole tokens and 0 substrings in code.
#[test]
fn w373_ratchet_the_member_appears_in_no_corpus_code() {
    let hits = member_in_code(&corpus());
    assert!(
        hits.is_empty(),
        "the wait in corpus code:\n{}",
        hits.join("\n")
    );
}

/// The recogniser's literal — the member alone, split from its qualifier — exactly once
/// in `ir.t1`, and the qualified token nowhere in it.
#[test]
fn w373_ratchet_the_recogniser_literal_is_once_in_ir() {
    let ir = std::fs::read_to_string(corpus_root().join("ir.t1")).expect("ir.t1");
    let literal = format!("उक्तम् {} इति", nirvahana::WAIT_BUILTIN_MEMBER);
    assert_eq!(ir.matches(&literal).count(), 1, "`{literal}` in ir.t1");
    assert_eq!(
        ir.matches(&qualified()).count(),
        0,
        "the qualified token in ir.t1"
    );
}

/// The tag is the PROGRAM's to declare: its numeral, in either spelling, is in no corpus
/// file.
#[test]
fn w373_ratchet_the_event_tag_numeral_is_in_no_corpus_file() {
    for (name, text) in corpus() {
        for n in [dec(EVENT_TAG), hex(EVENT_TAG)] {
            assert_eq!(text.matches(&n).count(), 0, "{name} spells the tag {n}");
        }
    }
}

/// THE MUTATION CONTROL: a call inserted into a COPY of a corpus file goes red naming
/// its file and line — and the same text in a comment or a string literal does not.
#[test]
fn w373_ratchet_mutation_control() {
    let files = corpus();
    let (name, text) = files
        .iter()
        .find(|(n, _)| n == "lex.t1")
        .expect("lex.t1")
        .clone();
    let insert_at = 10; // after line 10, so the call is line 11
    let mutate = |inserted: &str| -> Vec<(String, String)> {
        let mut lines: Vec<&str> = text.lines().collect();
        lines.insert(insert_at, inserted);
        let mut copy = files.clone();
        for f in &mut copy {
            if f.0 == name {
                f.1 = lines.join("\n");
            }
        }
        copy
    };
    let call = format!("    अवगणना भवति {} ० ।", qualified());
    let hits = member_in_code(&mutate(&call));
    println!("{hits:?}");
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(
        hits[0].starts_with(&format!("{name}:{}:", insert_at + 1)),
        "names file:line: {hits:?}"
    );
    let bare = format!("    अवगणना भवति {} ० ।", nirvahana::WAIT_BUILTIN_MEMBER);
    let hits = member_in_code(&mutate(&bare));
    assert!(
        hits.len() == 1 && hits[0].ends_with("(whole)"),
        "the bare member is a whole-token hit: {hits:?}"
    );
    let comment = format!("॰ {}", qualified());
    assert!(
        member_in_code(&mutate(&comment)).is_empty(),
        "a comment is not code"
    );
    let string = format!("    चरः क ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् {} इति ।", qualified());
    assert!(
        member_in_code(&mutate(&string)).is_empty(),
        "a string is not code"
    );
}
