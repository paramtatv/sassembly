//! **`W-376`: COOPERATIVE THREADS, SWITCHED BY THE HOST ONLY AT WAITS, THE SCHEDULE IN THE
//! EVENT LOG** — the falsifiers of `docs/adr/0040-addendum-w376-threads.md` §7, (a) to (i).
//!
//! Programs are compiled through the `.t1` path and through the Rust twin (`riscv64.rs`),
//! with `w373_wait.rs`'s harness; the interpreter runs the serial subset (option (b)).
//!
//! - (a) "A prints, waits, prints; B prints" under [A,B,A] prints `ABa` and under [B,A,A]
//!   prints `BAa`, and two replays of each agree on the retired count and the output;
//! - (b) NO PREEMPTION: a thread that never waits runs to its finisher while thread 1 has
//!   retired 0 and its context is untouched, and its count is the single-thread count; a
//!   host that switches every K instructions goes red;
//! - (c) each event goes to the thread that RESUMES; a host delivering at the wait goes red;
//! - (d) stack isolation: a local survives an interleaved wait; a host giving thread 1 the
//!   startup's `sp` goes red;
//! - (e) the thread-id slot is rewritten at every switch-in; a host that writes it only at
//!   a thread's first switch-in goes red;
//! - (f) every load and replay refusal of §6, and the runner's own;
//! - (g) a live record, then its replay, gives the same count;
//! - (h) the interpreter: [B,A,A] and the no-preemption case byte-identical on three
//!   engines, [A,B,A] refused by name and record index;
//! - (i) the W-372 extension: the tag numerals in no corpus code, with a mutation control
//!   naming file:line (the `threads:` refusal is in `w372_fixpoint_ratchet.rs`).
//!
//! EVERY NUMERAL IS GENERATED from a Rust constant ([`dec`]); none is typed.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{self, Interpreter, Octets, ThreadStep, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use yantra::Machine;
use yantra::input::{EVENT_TAG, ThreadRecord, parse_event_log, parse_thread_log};
use yantra::threads::{
    self, Context, Mutant, THREAD_ID_TAG, THREADS_TAG, ThreadState, Threads, ThreadsEnd,
    replay_threads,
};

const FUEL: u64 = 80_000_000_000;
const STEPS: u64 = 200_000_000;
const MODULE: &str = "सूत्रपरीक्षण";
/// What the event word holds before any delivery: not zero, and not a letter.
const UNDELIVERED: u64 = 1000;
/// What the thread-id slot holds before the host writes it: not a thread number.
const UNWRITTEN: u64 = 99;
/// Iterations of the no-preemption program's loop — long enough that a host cutting
/// every [`SLICE`] instructions cuts it several times.
const LOOP: u64 = 5000;
const SLICE: u64 = 1000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src")
}

/// The wait, built from the interpreter's two constants.
fn wait() -> String {
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

/// A pattern as a T1 hexadecimal numeral, for the ratchet only.
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

/// An octet's numeral.
fn ch(c: u8) -> String {
    dec(u64::from(c))
}

/// THE PROGRAM: the three interfaces, then `routines`, then the entry, which reads its
/// thread number from the slot into `स्वाङ्कः` and runs `body` (which returns).
fn program(count: u64, routines: &str, body: &str) -> String {
    format!(
        "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

सार्वजनिक चरः घटनासङ्केतक ॱॱ न६४ भवति {event} ।
सार्वजनिक चरः घटनामूल्यक ॱॱ न६४ भवति {undelivered} ।
सार्वजनिक चरः सूत्रगणसङ्केतक ॱॱ न६४ भवति {threads} ।
सार्वजनिक चरः सूत्रगणक ॱॱ न६४ भवति {count} ।
सार्वजनिक चरः सूत्राङ्कसङ्केतक ॱॱ न६४ भवति {id} ।
सार्वजनिक चरः सूत्राङ्कक ॱॱ न६४ भवति {unwritten} ।

{routines}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    चरः स्वाङ्कः ॱॱ न६४ भवति सूत्राङ्कक ।
{body}
इति
",
        event = dec(EVENT_TAG),
        undelivered = dec(UNDELIVERED),
        threads = dec(THREADS_TAG),
        count = dec(count),
        id = dec(THREAD_ID_TAG),
        unwritten = dec(UNWRITTEN),
    )
}

/// (a) "A prints, waits, prints; B prints": thread 0 is A, thread 1 is B. `ret` is what
/// thread 1 returns (thread 0 returns ०).
fn aba_with(ret: u64) -> String {
    program(
        2,
        "",
        &format!(
            "    यदि स्वाङ्कः न्यूनम् १ आदि
        अवगणना भवति अष्टकॱमुद्रणम् {a_up} ।
        अवगणना भवति {w} ० ।
        अवगणना भवति अष्टकॱमुद्रणम् {a_lo} ।
    इति
    चरः योगफलम् ॱॱ न६४ भवति ० ।
    यदि स्वाङ्कः अधिकम् ० आदि
        अवगणना भवति अष्टकॱमुद्रणम् {b_up} ।
        योगफलम् भवति {ret} ।
    इति
    प्रत्यागमनम् योगफलम् ।",
            a_up = ch(b'A'),
            a_lo = ch(b'a'),
            b_up = ch(b'B'),
            w = wait(),
            ret = dec(ret),
        ),
    )
}

fn aba() -> String {
    aba_with(0)
}

/// (b) thread 0 loops [`LOOP`] times and prints `A`, never waiting; thread 1 prints `B`.
fn no_wait(count: u64) -> String {
    program(
        count,
        "",
        &format!(
            "    चरः क्रमः ॱॱ न६४ भवति ० ।
    यदि स्वाङ्कः न्यूनम् १ आदि
        यावत् क्रमः न्यूनम् {lp} आदि
            क्रमः भवति क्रमः योगः १ ।
        इति
        अवगणना भवति अष्टकॱमुद्रणम् {a_up} ।
    इति
    यदि स्वाङ्कः अधिकम् ० आदि
        अवगणना भवति अष्टकॱमुद्रणम् {b_up} ।
    इति
    प्रत्यागमनम् ० ।",
            lp = dec(LOOP),
            a_up = ch(b'A'),
            b_up = ch(b'B'),
        ),
    )
}

/// (c) every thread waits, then prints the low octet of the event word.
fn both_wait() -> String {
    program(
        2,
        "",
        &format!(
            "    अवगणना भवति {w} ० ।
    अवगणना भवति अष्टकॱमुद्रणम् घटनामूल्यक ।
    प्रत्यागमनम् ० ।",
            w = wait()
        ),
    )
}

/// (d) every thread computes a local from its number, waits INSIDE a callee, and prints
/// the local: the value lives across a call, so it lives in a stack frame.
fn stack_local() -> String {
    program(
        2,
        &format!(
            "वृत्तिः प्रतीक्षणम् आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    अवगणना भवति {w} मूल्यम् ।
    प्रत्यागमनम् ० ।
इति

वृत्तिः कार्यम् आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    चरः स्थानीयम् ॱॱ न६४ भवति मूल्यम् योगः {p_up} ।
    अवगणना भवति प्रतीक्षणम् मूल्यम् ।
    अवगणना भवति अष्टकॱमुद्रणम् स्थानीयम् ।
    प्रत्यागमनम् ० ।
इति
",
            w = wait(),
            p_up = ch(b'P'),
        ),
        "    अवगणना भवति कार्यम् स्वाङ्कः ।
    प्रत्यागमनम् ० ।",
    )
}

/// (e) every thread waits, then reads the thread-id slot AGAIN and prints `A` + it.
fn id_after_wait() -> String {
    program(
        2,
        "",
        &format!(
            "    अवगणना भवति {w} ० ।
    चरः सृतम् ॱॱ न६४ भवति सूत्राङ्कक योगः {a_up} ।
    अवगणना भवति अष्टकॱमुद्रणम् सृतम् ।
    प्रत्यागमनम् ० ।",
            w = wait(),
            a_up = ch(b'A'),
        ),
    )
}

/// A log, parsed by the real reader.
fn log(text: &str) -> Vec<ThreadRecord> {
    parse_thread_log(text).unwrap_or_else(|e| panic!("{text:?}: {e}"))
}

// ── the engines ──────────────────────────────────────────────────────────────

/// THE `.t1` CHAIN'S IMAGE (`w373_wait.rs`'s `t1_image_with`).
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
    assert!(
        !image.is_empty(),
        "the probe built no image: refusal {:?}, link {:?}",
        sadhana::t1::chain::refusal_site(&it),
        sadhana::t1::chain::link_refusals(&it)
    );
    image
}

/// THE RUST TWIN (`w373_wait.rs`'s `rust_twin_image`).
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

/// Built images, keyed by `(source, twin)`.
type Images = HashMap<(String, bool), Vec<u8>>;

/// Each image is built ONCE per test binary, keyed by its source.
fn image(src: &str, twin: bool) -> Vec<u8> {
    static CACHE: OnceLock<Mutex<Images>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(img) = cache.lock().unwrap().get(&(src.to_string(), twin)) {
        return img.clone();
    }
    let img = if twin {
        rust_twin_image(src)
    } else {
        t1_image(src)
    };
    cache
        .lock()
        .unwrap()
        .insert((src.to_string(), twin), img.clone());
    img
}

/// The two native engines.
const NATIVE: [(&str, bool); 2] = [(".t1 image", false), ("rust twin", true)];

/// ONE VOCABULARY FOR EVERY ENGINE.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    /// Every thread ended, every record consumed: `(thread, status)` in the order the
    /// threads ended (a status the finisher does not define is `u64::MAX`), and the octets.
    Ended {
        statuses: Vec<(u32, u64)>,
        out: Vec<u8>,
    },
    Short {
        index: usize,
        thread: Option<u32>,
        out: Vec<u8>,
    },
    Long {
        consumed: usize,
        len: usize,
        out: Vec<u8>,
    },
    Refused {
        index: usize,
        why: String,
    },
    Halted(String),
    /// Refused at load, in the host's words.
    Load(String),
    /// The interpreter refused, in its words.
    Interp(String),
}

/// A native run, with the machine's count and the thread host left for inspection.
struct Native {
    outcome: Outcome,
    time: u64,
    threads: Option<Threads>,
    fresh: Vec<Context>,
}

/// Load, discover, make the stacks, then [`replay_threads`] — what `yantra-run --events`
/// does, in process.
fn native_with(img: &[u8], log: &[ThreadRecord], mutant: Option<Mutant>, budget: u64) -> Native {
    let mut m = Machine::load_elf(img, yantra::ram_for(img)).expect("the image loads");
    let d = match threads::discover(&m, img) {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Native {
                outcome: Outcome::Load("no thread interface".into()),
                time: 0,
                threads: None,
                fresh: Vec::new(),
            };
        }
        Err(e) => {
            return Native {
                outcome: Outcome::Load(e),
                time: 0,
                threads: None,
                fresh: Vec::new(),
            };
        }
    };
    let mut t = Threads::new(&mut m, d).expect("the stacks are made");
    if let Some(mu) = mutant {
        t.set_mutant(mu);
    }
    let fresh = t.contexts.clone();
    let mut out: Vec<u8> = Vec::new();
    let end = replay_threads(&mut m, &mut t, log, budget, &mut out);
    let outcome = match end {
        ThreadsEnd::Ended => Outcome::Ended {
            statuses: t
                .ends
                .iter()
                .map(|e| (e.thread, e.status.unwrap_or(u64::MAX)))
                .collect(),
            out,
        },
        ThreadsEnd::Short { index, thread } => Outcome::Short { index, thread, out },
        ThreadsEnd::Long { consumed, len } => Outcome::Long { consumed, len, out },
        ThreadsEnd::Refused { index, why } => Outcome::Refused { index, why },
        ThreadsEnd::Halted { thread, halt } => {
            Outcome::Halted(format!("thread {thread} halted {halt:?}"))
        }
        beyond @ ThreadsEnd::BeyondOwnStack { .. } => Outcome::Halted(format!("{beyond:?}")),
    };
    Native {
        outcome,
        time: m.time,
        threads: Some(t),
        fresh,
    }
}

fn native(img: &[u8], log: &[ThreadRecord]) -> Native {
    native_with(img, log, None, STEPS)
}

/// THE INTERPRETER'S SERIAL SUBSET: the chain loaded beside the probe, the schedule handed
/// to [`Interpreter::run_threads`].
fn interpreted(src: &str, log: &[ThreadRecord]) -> Outcome {
    let file = format!("{MODULE}.t1");
    let mut srcs: Vec<(&str, &str)> = CHAIN.to_vec();
    srcs.push((file.as_str(), src));
    let mut it = Interpreter::load(&srcs, &spec_root())
        .unwrap_or_else(|e| panic!("the probe must load: {}", e.reason));
    let steps: Vec<ThreadStep> = log
        .iter()
        .map(|r| match r {
            ThreadRecord::Value(v) => ThreadStep::Value(*v),
            ThreadRecord::Run(k) => ThreadStep::Run(*k),
        })
        .collect();
    match it.run_threads(&format!("{MODULE}\u{971}मुख्यम्"), &steps, FUEL) {
        Ok(ends) => Outcome::Ended {
            statuses: ends
                .iter()
                .map(|(k, v)| (*k, v.as_int().map_or(u64::MAX, |s| s as u64)))
                .collect(),
            out: it.sink().to_vec(),
        },
        Err(e) => Outcome::Interp(e.reason),
    }
}

fn ended(statuses: &[(u32, u64)], out: &[u8]) -> Outcome {
    Outcome::Ended {
        statuses: statuses.to_vec(),
        out: out.to_vec(),
    }
}

// ── (a) the two schedules ────────────────────────────────────────────────────

#[test]
fn w376_a_the_schedule_is_the_logs_order_and_each_replays_identically() {
    let src = aba();
    for (engine, twin) in NATIVE {
        let img = image(&src, twin);
        for (name, text, want) in [
            (
                "[A,B,A]",
                "@0\n@1\n@0\n7\n",
                ended(&[(1, 0), (0, 0)], b"ABa"),
            ),
            (
                "[B,A,A]",
                "@1\n@0\n@0\n7\n",
                ended(&[(1, 0), (0, 0)], b"BAa"),
            ),
        ] {
            let first = native(&img, &log(text));
            let second = native(&img, &log(text));
            println!(
                "(a) {engine} {name}: {:?}, {} steps",
                first.outcome, first.time
            );
            assert_eq!(first.outcome, want, "(a) {engine} {name}");
            assert_eq!(
                (second.outcome, second.time),
                (first.outcome, first.time),
                "(a) {engine} {name}: the replay differs"
            );
        }
    }
}

// ── (b) no preemption ────────────────────────────────────────────────────────

/// The property: on the log `@0`, thread 0 — which never waits — reaches its finisher
/// while thread 1 is untouched (retired 0, context as made, never switched in); and its
/// count is the count of the same program declared single-threaded. `Err` says which
/// part failed.
fn no_preemption(twin: bool, mutant: Option<Mutant>) -> Result<(), String> {
    let two = image(&no_wait(2), twin);
    let run = native_with(&two, &log("@0\n"), mutant, STEPS);
    let t = run.threads.as_ref().expect("threaded");
    let ended0 = matches!(
        t.states[0],
        ThreadState::Ended {
            status: Some(0),
            ..
        }
    );
    if !ended0 {
        return Err(format!(
            "thread 0 did not reach its finisher on `@0`: {:?}, states {:?}",
            run.outcome, t.states
        ));
    }
    if t.retired[1] != 0 || t.contexts[1] != run.fresh[1] || t.states[1] != ThreadState::Fresh {
        return Err(format!(
            "thread 1 was touched before thread 0 ended: retired {}, state {:?}",
            t.retired[1], t.states[1]
        ));
    }
    let one = image(&no_wait(1), twin);
    let single = native_with(&one, &log("@0\n"), None, STEPS);
    if single.outcome != ended(&[(0, 0)], b"A") {
        return Err(format!("the single-thread control: {:?}", single.outcome));
    }
    if t.retired[0] != single.time {
        return Err(format!(
            "thread 0 retired {} and the same work single-threaded {}",
            t.retired[0], single.time
        ));
    }
    Ok(())
}

#[test]
fn w376_b_a_thread_that_never_waits_runs_to_its_finisher_untouched() {
    for (engine, twin) in NATIVE {
        no_preemption(twin, None).unwrap_or_else(|e| panic!("(b) {engine}: {e}"));
        let full = native(&image(&no_wait(2), twin), &log("@0\n@1\n"));
        assert_eq!(
            full.outcome,
            ended(&[(0, 0), (1, 0)], b"AB"),
            "(b) {engine}"
        );
    }
}

#[test]
fn w376_b_mutant_a_host_that_switches_every_k_instructions_is_red() {
    for (engine, twin) in NATIVE {
        let e = no_preemption(twin, Some(Mutant::SwitchEvery(SLICE)))
            .expect_err("a preempting host passed the no-preemption property");
        println!("(b) mutant, {engine}: {e}");
    }
}

// ── (c) the event goes to the thread that resumes ────────────────────────────

const BOTH_WAIT: &str = "@0\n@1\n@0\n120\n@1\n121\n";

#[test]
fn w376_c_each_event_goes_to_the_thread_that_resumes() {
    for (engine, twin) in NATIVE {
        let run = native(&image(&both_wait(), twin), &log(BOTH_WAIT));
        assert_eq!(run.outcome, ended(&[(0, 0), (1, 0)], b"xy"), "(c) {engine}");
    }
}

#[test]
fn w376_c_mutant_delivering_at_the_wait_is_red() {
    for (engine, twin) in NATIVE {
        let good = native(&image(&both_wait(), twin), &log(BOTH_WAIT));
        let bad = native_with(
            &image(&both_wait(), twin),
            &log(BOTH_WAIT),
            Some(Mutant::DeliverAtWait),
            STEPS,
        );
        println!(
            "(c) {engine}: host {:?}, mutant {:?}",
            good.outcome, bad.outcome
        );
        assert_eq!(good.outcome, ended(&[(0, 0), (1, 0)], b"xy"), "the control");
        assert_ne!(bad.outcome, good.outcome, "(c) {engine}: the mutant agreed");
    }
}

// ── (d) stack isolation ──────────────────────────────────────────────────────

const INTERLEAVED: &str = "@0\n@1\n@0\n7\n@1\n7\n";

#[test]
fn w376_d_a_local_survives_an_interleaved_wait() {
    for (engine, twin) in NATIVE {
        let run = native(&image(&stack_local(), twin), &log(INTERLEAVED));
        assert_eq!(run.outcome, ended(&[(0, 0), (1, 0)], b"PQ"), "(d) {engine}");
        let t = run.threads.expect("threaded");
        let sp1 = run.fresh[1].x[2];
        assert_eq!(
            sp1,
            0x8000_0000 + (t.stacks_at + threads::STACK_OCTETS) as u64,
            "(d) {engine}: thread 1's sp is the top of its own host stack"
        );
        assert_ne!(sp1, t.declared.startup_sp, "and not the startup's");
    }
}

#[test]
fn w376_d_mutant_a_shared_sp_is_red() {
    for (engine, twin) in NATIVE {
        let good = native(&image(&stack_local(), twin), &log(INTERLEAVED));
        assert_eq!(good.outcome, ended(&[(0, 0), (1, 0)], b"PQ"), "the control");
        let bad = native_with(
            &image(&stack_local(), twin),
            &log(INTERLEAVED),
            Some(Mutant::SharedSp),
            STEPS,
        );
        println!("(d) mutant, {engine}: {:?}", bad.outcome);
        assert_ne!(
            bad.outcome,
            ended(&[(0, 0), (1, 0)], b"PQ"),
            "(d) {engine}: a shared stack agreed"
        );
    }
}

// ── (e) the thread id at every switch-in ─────────────────────────────────────

#[test]
fn w376_e_the_thread_id_is_rewritten_at_every_switch_in() {
    for (engine, twin) in NATIVE {
        let run = native(&image(&id_after_wait(), twin), &log(INTERLEAVED));
        assert_eq!(run.outcome, ended(&[(0, 0), (1, 0)], b"AB"), "(e) {engine}");
        let bad = native_with(
            &image(&id_after_wait(), twin),
            &log(INTERLEAVED),
            Some(Mutant::NoIdRewrite),
            STEPS,
        );
        println!("(e) mutant, {engine}: {:?}", bad.outcome);
        assert_ne!(
            bad.outcome, run.outcome,
            "(e) {engine}: a no-rewrite host agreed"
        );
    }
}

/// The switch retires no instruction: a thread's retired counts sum to the machine's.
#[test]
fn w376_e_a_switch_retires_no_instruction() {
    for (engine, twin) in NATIVE {
        let run = native(&image(&id_after_wait(), twin), &log(INTERLEAVED));
        let t = run.threads.expect("threaded");
        assert_eq!(t.retired.iter().sum::<u64>(), run.time, "(e) {engine}");
        assert_eq!(
            t.switches.iter().map(|s| s.0).collect::<Vec<_>>(),
            vec![0, 1, 0, 1],
            "(e) {engine}: one switch-in per @"
        );
    }
}

// ── (f) the refusals ─────────────────────────────────────────────────────────

/// A fresh machine from the [A,B,A] image (a `Machine` is not `Clone`), and the
/// interface's offsets.
fn loaded(twin: bool) -> (impl Fn() -> Machine, threads::Declared, Vec<u8>) {
    let img = image(&aba(), twin);
    let kept = img.clone();
    let load = move || Machine::load_elf(&img, yantra::ram_for(&img)).expect("loads");
    let d = threads::discover(&load(), &kept)
        .expect("the image is accepted")
        .expect("and threaded");
    (load, d, kept)
}

fn refused_at_load(m: &Machine, img: &[u8]) -> String {
    match threads::discover(m, img) {
        Err(e) => e,
        Ok(d) => panic!("accepted: {d:?}"),
    }
}

#[test]
fn w376_f_every_load_refusal() {
    for (engine, twin) in NATIVE {
        let (fresh, d, img) = loaded(twin);
        assert_eq!(d.count, 2, "{engine}");
        // The decoded startup computes the sp the startup itself sets.
        let mut x = fresh();
        let _ = x.run(2, &mut Vec::<u8>::new());
        assert_eq!(
            (x.pc, x.x[2]),
            (d.entry + 8, d.startup_sp),
            "{engine}: the two words decoded are the two words run"
        );
        let zero = |m: &mut Machine, at: usize| m.mem[at..at + 8].fill(0);
        let put = |m: &mut Machine, at: usize, w: u64| {
            m.mem[at..at + 8].copy_from_slice(&w.to_le_bytes())
        };
        // A duplicate must sit in the FILE-BACKED octets to be scanned: the event word.
        let spare = d.event_tag + 8;

        let mut x = fresh();
        zero(&mut x, d.id_tag);
        let e = refused_at_load(&x, &img);
        assert!(e.contains("without SASTHRID"), "{engine}: {e}");

        let mut x = fresh();
        zero(&mut x, d.threads_tag);
        let e = refused_at_load(&x, &img);
        assert!(e.contains("without SASTHRDS"), "{engine}: {e}");

        let mut x = fresh();
        put(&mut x, spare, THREADS_TAG);
        let e = refused_at_load(&x, &img);
        assert!(
            e.contains("the SASTHRDS tag appears at 2 words"),
            "{engine}: {e}"
        );

        let mut x = fresh();
        put(&mut x, spare, THREAD_ID_TAG);
        let e = refused_at_load(&x, &img);
        assert!(
            e.contains("the SASTHRID tag appears at 2 words"),
            "{engine}: {e}"
        );

        for bad in [0, threads::MAX_THREADS + 1] {
            let mut x = fresh();
            put(&mut x, d.threads_tag + 8, bad);
            let e = refused_at_load(&x, &img);
            assert!(e.contains(&format!("count is {bad}")), "{engine}: {e}");
        }
        let mut x = fresh();
        put(&mut x, d.threads_tag + 8, threads::MAX_THREADS);
        assert_eq!(
            threads::discover(&x, &img).unwrap().map(|d| d.count),
            Some(64),
            "{engine}: 64 is the ceiling, not past it"
        );

        let mut x = fresh();
        zero(&mut x, d.event_tag);
        let e = refused_at_load(&x, &img);
        assert!(
            e.contains("needs the event interface") && e.contains("SASEVENT"),
            "{engine}: {e}"
        );

        let mut x = fresh();
        let at = (d.entry - x.base) as usize;
        let w1 = u32::from_le_bytes(x.mem[at + 4..at + 8].try_into().unwrap());
        x.mem[at..at + 4].copy_from_slice(&0x13u32.to_le_bytes()); // nop
        let e = refused_at_load(&x, &img);
        assert!(
            e.contains("0x00000013") && e.contains(&format!("{w1:#010x}")),
            "{engine}: both words named: {e}"
        );

        // The stacks are made BEFORE the injection: a RAM already past the store bound is
        // what `input::inject` leaves, and it is refused.
        let mut x = fresh();
        x.mem.resize(x.mem.len() + 4096, 0);
        let e = Threads::new(&mut x, d.clone()).expect_err("stacks after an injection");
        assert!(e.contains("BEFORE the input is injected"), "{engine}: {e}");

        // And made where the ruling says: above `ram`, the store bound raised over them.
        let mut x = fresh();
        let ram = x.mem.len();
        let t = Threads::new(&mut x, d.clone()).expect("stacks");
        assert!(t.stacks_at >= ram, "{engine}");
        assert_eq!(
            x.mem.len(),
            t.stacks_at + threads::STACK_OCTETS,
            "{engine}: one stack"
        );
        assert_eq!(
            x.store_limit,
            x.mem.len(),
            "{engine}: stores reach the stacks"
        );
    }
}

#[test]
fn w376_f_every_replay_refusal() {
    for (engine, twin) in NATIVE {
        let img = image(&aba(), twin);
        let refused = |text: &str, index: usize, what: &str| {
            let o = native(&img, &log(text)).outcome;
            match &o {
                Outcome::Refused { index: i, why } if *i == index && why.contains(what) => {}
                _ => panic!(
                    "{engine} {text:?}: want a refusal at {index} naming {what:?}, got {o:?}"
                ),
            }
        };
        refused("@2\n", 0, "declares 2 threads");
        refused("@1\n@1\n", 1, "thread 1 has ended");
        refused("5\n", 0, "must be `@N`");
        refused("@0\n@1\n@0\n@1\n", 3, "its value record is due");
        assert_eq!(
            native(&img, &log("@0\n@1\n")).outcome,
            Outcome::Short {
                index: 2,
                thread: Some(1),
                out: b"AB".to_vec()
            },
            "{engine}: short at a decision"
        );
        assert_eq!(
            native(&img, &log("@0\n@1\n@0\n")).outcome,
            Outcome::Short {
                index: 3,
                thread: Some(0),
                out: b"AB".to_vec()
            },
            "{engine}: short at a value"
        );
        assert_eq!(
            native(&img, &log("@0\n@1\n@0\n7\n@0\n")).outcome,
            Outcome::Long {
                consumed: 4,
                len: 5,
                out: b"ABa".to_vec()
            },
            "{engine}: long"
        );
        // ANY NON-FINISHER HALT ENDS THE WHOLE RUN: here the step limit, inside thread 0.
        let o = native_with(&img, &log("@0\n@1\n@0\n7\n"), None, 3).outcome;
        assert!(
            matches!(&o, Outcome::Halted(h) if h.starts_with("thread 0 halted StepLimit")),
            "{engine}: {o:?}"
        );
    }
}

#[test]
fn w376_f_a_thread_status_is_judged_after_every_thread_ends() {
    for (engine, twin) in NATIVE {
        let run = native(&image(&aba_with(3), twin), &log("@0\n@1\n@0\n7\n"));
        assert_eq!(
            run.outcome,
            ended(&[(1, 3), (0, 0)], b"ABa"),
            "{engine}: thread 0 went on"
        );
        let t = run.threads.expect("threaded");
        let first = threads::first_failure(&t).expect("a failure");
        assert_eq!((first.thread, first.status), (1, Some(3)), "{engine}");
    }
}

/// THE RUNNER, on the `.t1` image: `threads: 2` whenever SASTHRDS is found; no log is
/// exit 1 AT LOAD, never 75; a replay finishes; a non-zero thread fails the run naming
/// it; a threaded log handed to an UNTHREADED image is refused by the single-thread
/// reader; a live record replays to the same count.
#[test]
fn w376_f_the_runner() {
    let dir = std::env::temp_dir().join(format!("w376-{}-{}", std::process::id(), line!()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let run = |img: &[u8], flag: Option<(&str, &str)>| {
        let elf = dir.join("p.elf");
        std::fs::write(&elf, img).expect("write the image");
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_yantra-run"));
        let path = dir.join("events.log");
        if let Some((flag, text)) = flag {
            if flag == "--events" {
                std::fs::write(&path, text).expect("write the log");
            }
            cmd.arg(flag).arg(&path);
        }
        let o = cmd.arg(&elf).output().expect("yantra-run runs");
        (
            o.status.code(),
            o.stdout,
            String::from_utf8_lossy(&o.stderr).into_owned(),
        )
    };
    let img = image(&aba(), false);
    let threads_line = |err: &str| err.lines().any(|l| l == "threads: 2");

    let (code, out, err) = run(&img, None);
    assert!(
        code == Some(1) && out.is_empty() && threads_line(&err),
        "no log:\n{err}"
    );
    assert!(
        err.contains("--events or --record-events"),
        "no log, named:\n{err}"
    );
    assert!(
        !err.contains("halt:"),
        "refused at load, nothing ran:\n{err}"
    );

    let (code, out, err) = run(&img, Some(("--events", "@1\n@0\n@0\n7\n")));
    assert_eq!((code, out), (Some(0), b"BAa".to_vec()), "[B,A,A]:\n{err}");
    assert!(threads_line(&err), "{err}");

    let (code, out, err) = run(
        &image(&aba_with(3), false),
        Some(("--events", "@0\n@1\n@0\n7\n")),
    );
    assert_eq!((code, out), (Some(1), b"ABa".to_vec()), "status 3:\n{err}");
    assert!(
        err.contains("thread 1 ended with status 3"),
        "named:\n{err}"
    );

    let (code, _, err) = run(&img, Some(("--events", "@0\n@1\n")));
    assert!(
        code == Some(1) && err.contains("SHORTER") && err.contains("record 2"),
        "short:\n{err}"
    );

    // UNTHREADED: the same image with both thread tags zeroed in the file's octets.
    let mut plain = img.clone();
    for tag in [THREADS_TAG, THREAD_ID_TAG] {
        let w = tag.to_le_bytes();
        let at = (0..plain.len() - 7)
            .find(|&o| plain[o..o + 8] == w)
            .expect("the tag is file-backed");
        plain[at..at + 8].fill(0);
    }
    let (code, _, err) = run(&plain, Some(("--events", "@0\n@1\n@0\n7\n")));
    assert!(
        code == Some(1) && err.contains("THREAD record"),
        "unthreaded image:\n{err}"
    );
    assert!(!threads_line(&err), "{err}");

    // LIVE, then REPLAY: the same output and the same count.
    let steps = |err: &str| {
        err.lines()
            .find(|l| l.starts_with("steps:"))
            .map(str::to_string)
            .expect("a steps line")
    };
    let (code, live_out, live_err) = run(&img, Some(("--record-events", "")));
    assert_eq!(code, Some(0), "live:\n{live_err}");
    let recorded = std::fs::read_to_string(dir.join("events.log")).expect("the live log");
    let (code, out, err) = run(&img, Some(("--events", &recorded)));
    assert_eq!(
        (code, &out),
        (Some(0), &live_out),
        "replay of the live log:\n{err}"
    );
    assert_eq!(steps(&err), steps(&live_err), "the count");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The single-thread reader refuses `@` by name; the threaded reader reads it.
#[test]
fn w376_f_the_log_readers() {
    let e = parse_event_log("@0\n").unwrap_err();
    assert!(e.contains("THREAD record"), "{e}");
    assert_eq!(
        parse_thread_log("@1\n@0\n@0\nt=9\n").unwrap(),
        vec![
            ThreadRecord::Run(1),
            ThreadRecord::Run(0),
            ThreadRecord::Run(0),
            ThreadRecord::Value(9)
        ]
    );
}

// ── (g) live, then replay ────────────────────────────────────────────────────

#[test]
fn w376_g_a_live_record_replays_to_the_same_count() {
    for (engine, twin) in NATIVE {
        let img = image(&aba(), twin);
        let mut m = Machine::load_elf(&img, yantra::ram_for(&img)).unwrap();
        let d = threads::discover(&m, &img).unwrap().unwrap();
        let mut t = Threads::new(&mut m, d).unwrap();
        let mut out: Vec<u8> = Vec::new();
        let mut text: Vec<u8> = Vec::new();
        let end = threads::record_live_threads(&mut m, &mut t, STEPS, &mut out, &mut text)
            .expect("the log is written");
        assert_eq!(end, ThreadsEnd::Ended, "{engine}");
        let text = String::from_utf8(text).unwrap();
        let recorded = log(&text);
        println!("(g) {engine}: {text}");
        assert!(
            matches!(
                recorded.as_slice(),
                [
                    ThreadRecord::Run(0),
                    ThreadRecord::Run(1),
                    ThreadRecord::Run(0),
                    ThreadRecord::Value(_)
                ]
            ),
            "{engine}: round-robin from thread 0, a t= record at the resume: {recorded:?}"
        );
        let replayed = native(&img, &recorded);
        assert_eq!(replayed.outcome, ended(&[(1, 0), (0, 0)], &out), "{engine}");
        assert_eq!(out, b"ABa", "{engine}");
        assert_eq!(replayed.time, m.time, "{engine}: the count");
    }
}

// ── (h) the interpreter's serial subset ──────────────────────────────────────

fn three(src: &str, text: &str) -> [(&'static str, Outcome); 3] {
    let l = log(text);
    [
        ("interpreter", interpreted(src, &l)),
        (".t1 image", native(&image(src, false), &l).outcome),
        ("rust twin", native(&image(src, true), &l).outcome),
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

#[test]
fn w376_h_baa_is_byte_identical_on_three_engines() {
    assert_all(
        "(h) [B,A,A]",
        &three(&aba(), "@1\n@0\n@0\n7\n"),
        &ended(&[(1, 0), (0, 0)], b"BAa"),
    );
}

#[test]
fn w376_h_no_preemption_is_byte_identical_on_three_engines() {
    assert_all(
        "(h) no preemption",
        &three(&no_wait(2), "@0\n@1\n"),
        &ended(&[(0, 0), (1, 0)], b"AB"),
    );
}

#[test]
fn w376_h_a_status_is_the_same_on_three_engines() {
    assert_all(
        "(h) status 3",
        &three(&aba_with(3), "@1\n@0\n@0\n7\n"),
        &ended(&[(1, 3), (0, 0)], b"BAa"),
    );
}

#[test]
fn w376_h_the_interpreter_refuses_an_interleaving_by_name() {
    let o = interpreted(&aba(), &log("@0\n@1\n@0\n7\n"));
    match &o {
        Outcome::Interp(r) if r.contains("INTERLEAVING") && r.contains("record index 1") => {}
        _ => panic!("[A,B,A] must be refused by name at record index 1: {o:?}"),
    }
    let o = interpreted(&aba(), &log("@1\n@0\n@0\n"));
    assert!(
        matches!(&o, Outcome::Interp(r) if r.contains("SHORTER") && r.contains("record index 3")),
        "short: {o:?}"
    );
    let o = interpreted(&aba(), &log("@1\n@0\n@0\n7\n@0\n"));
    assert!(
        matches!(&o, Outcome::Interp(r) if r.contains("LONGER")),
        "long: {o:?}"
    );
    let o = interpreted(&aba(), &log("@2\n"));
    assert!(
        matches!(&o, Outcome::Interp(r) if r.contains("record index 0")),
        "range: {o:?}"
    );
}

#[test]
fn w376_h_the_interpreters_tags_are_yantras() {
    assert_eq!(nirvahana::THREADS_TAG, THREADS_TAG);
    assert_eq!(nirvahana::THREAD_ID_TAG, THREAD_ID_TAG);
    assert_eq!(&THREADS_TAG.to_le_bytes(), b"SASTHRDS");
    assert_eq!(&THREAD_ID_TAG.to_le_bytes(), b"SASTHRID");
}

// ── (i) the W-372 extension: the tag numerals in no corpus code ──────────────

/// A corpus file's CODE tokens (`w373_wait.rs`'s `code_tokens`): `॰` comments cut,
/// `उक्तम् … इति` string literals dropped.
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

/// Every WHOLE code token that is one of the two tags' numerals, in either spelling, as
/// `file:line: token`.
fn tag_numerals_in_code(files: &[(String, String)]) -> Vec<String> {
    let numerals: Vec<String> = [THREADS_TAG, THREAD_ID_TAG]
        .into_iter()
        .flat_map(|t| [dec(t), hex(t)])
        .collect();
    let mut hits = Vec::new();
    for (name, text) in files {
        for (line, tok) in code_tokens(text) {
            if numerals.contains(&tok) {
                hits.push(format!("{name}:{line}: {tok}"));
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

#[test]
fn w376_i_the_tag_numerals_are_in_no_corpus_code() {
    let hits = tag_numerals_in_code(&corpus());
    assert!(
        hits.is_empty(),
        "a thread tag in corpus code:\n{}",
        hits.join("\n")
    );
}

/// THE MUTATION CONTROL: a tag numeral inserted into a COPY of a corpus file goes red
/// naming its file and line; the same numeral in a comment or a string does not.
#[test]
fn w376_i_mutation_control() {
    let files = corpus();
    let (name, text) = files
        .iter()
        .find(|(n, _)| n == "lex.t1")
        .expect("lex.t1")
        .clone();
    let insert_at = 10;
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
    for tag in [THREADS_TAG, THREAD_ID_TAG] {
        for numeral in [dec(tag), hex(tag)] {
            let hits = tag_numerals_in_code(&mutate(&format!("    चरः क ॱॱ न६४ भवति {numeral} ।")));
            println!("{hits:?}");
            assert_eq!(hits, vec![format!("{name}:{}: {numeral}", insert_at + 1)]);
            assert!(
                tag_numerals_in_code(&mutate(&format!("॰ {numeral}"))).is_empty(),
                "a comment is not code"
            );
            assert!(
                tag_numerals_in_code(&mutate(&format!(
                    "    चरः क ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् {numeral} इति ।"
                )))
                .is_empty(),
                "a string is not code"
            );
        }
    }
}

// ── review fixes (W-376 review bound to ad6f0800) ────────────────────────────

/// How deep thread 1 recurses in the heap case: enough to write down to the bottom of
/// its 64 KiB stack (the reviewer's figure).
const DEPTH: u64 = 2000;
/// How deep thread 2 recurses in the overflow case: far past its own 64 KiB.
const DEEP: u64 = 20000;

/// A routine that makes ONE small allocation (a fresh one-element run), and a routine
/// that recurses `मूल्यम्` deep.
fn alloc_and_recurse() -> String {
    "वृत्तिः लघुरचना आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि
    चरः लघु ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    लघु अङ्कः ० अन्तः भवति मूल्यम् ।
    प्रत्यागमनम् ० ।
इति

वृत्तिः गहनम् आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि
    यदि मूल्यम् न्यूनम् १ आदि
        प्रत्यागमनम् ० ।
    इति
    चरः अवरम् ॱॱ न६४ भवति मूल्यम् वियोगः १ ।
    चरः फलम् ॱॱ न६४ भवति गहनम् अवरम् ।
    प्रत्यागमनम् फलम् ।
इति
"
    .to_string()
}

/// FIX 1, THE REVIEWER'S CASE: thread 0 waits for a count K, makes K small allocations,
/// builds a run holding 1..8 and waits; thread 1 recurses [`DEPTH`] deep and ends; thread
/// 0 resumes, sums its run and prints the sum (36).
fn heap_past_ram() -> String {
    program(
        2,
        &alloc_and_recurse(),
        &format!(
            "    चरः परिमाणम् ॱॱ न६४ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    चरः सञ्चयः ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    चरः ज ॱॱ न६४ भवति ० ।
    चरः पूर्णम् ॱॱ न६४ भवति १ ।
    चरः योगफलम् ॱॱ न६४ भवति ० ।
    यदि स्वाङ्कः न्यूनम् १ आदि
        अवगणना भवति {w} ० ।
        परिमाणम् भवति घटनामूल्यक ।
        यावत् क्रमः न्यूनम् परिमाणम् आदि
            अवगणना भवति लघुरचना क्रमः ।
            क्रमः भवति क्रमः योगः १ ।
        इति
        यावत् ज न्यूनम् ८ आदि
            सञ्चयः अङ्कः ज अन्तः भवति पूर्णम् ।
            ज भवति ज योगः १ ।
            पूर्णम् भवति पूर्णम् योगः १ ।
        इति
        अवगणना भवति {w} ० ।
        ज भवति ० ।
        यावत् ज न्यूनम् सञ्चयः ॱ दैर्घ्य आदि
            योगफलम् भवति योगफलम् योगः सञ्चयः अङ्कः ज अन्तः ।
            ज भवति ज योगः १ ।
        इति
        अवगणना भवति अष्टकॱमुद्रणम् योगफलम् ।
    इति
    यदि स्वाङ्कः अधिकम् ० आदि
        अवगणना भवति गहनम् {depth} ।
    इति
    प्रत्यागमनम् ० ।",
            w = wait(),
            depth = dec(DEPTH),
        ),
    )
}

/// FIX 1, CROSS-THREAD: thread 1 waits inside a call (its frames on its stack); thread 2
/// recurses [`DEEP`] deep, past the bottom of its own stack and into thread 1's; thread 0
/// does nothing.
fn stack_overflow() -> String {
    program(
        3,
        &format!(
            "{}
वृत्तिः प्रतीक्षणम् आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    अवगणना भवति {w} मूल्यम् ।
    प्रत्यागमनम् ० ।
इति

वृत्तिः कार्यम् आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    चरः स्थानीयम् ॱॱ न६४ भवति मूल्यम् योगः {p_up} ।
    अवगणना भवति प्रतीक्षणम् मूल्यम् ।
    अवगणना भवति अष्टकॱमुद्रणम् स्थानीयम् ।
    प्रत्यागमनम् ० ।
इति
",
            alloc_and_recurse(),
            w = wait(),
            p_up = ch(b'P'),
        ),
        &format!(
            "    यदि स्वाङ्कः अधिकम् ० आदि
        यदि स्वाङ्कः न्यूनम् २ आदि
            अवगणना भवति कार्यम् स्वाङ्कः ।
        इति
    इति
    यदि स्वाङ्कः अधिकम् १ आदि
        अवगणना भवति गहनम् {deep} ।
    इति
    प्रत्यागमनम् ० ।",
            deep = dec(DEEP),
        ),
    )
}

/// A machine for `img` at 20 MiB under `Span::FileBacked` — the reviewer's second venue,
/// where the heap reaches past RAM after a few thousand allocations.
fn small_machine(img: &[u8]) -> Machine {
    Machine::load_elf_spanning(img, yantra::DEFAULT_RAM, yantra::Span::FileBacked)
        .expect("the file-backed octets fit")
}

/// The heap case run SINGLE-THREADED on K: thread 0's work alone, the id slot written 0
/// by hand, the two values replayed. `true`: it halted `BeyondRam`.
fn single_beyond_ram(img: &[u8], k: u64) -> bool {
    let mut m = small_machine(img);
    let d = discover_any(&m, img);
    m.mem[d.id_tag + 8..d.id_tag + 16].copy_from_slice(&0u64.to_le_bytes());
    let mut out: Vec<u8> = Vec::new();
    match yantra::input::replay(&mut m, d.event_tag, &[k, 7], STEPS, &mut out) {
        yantra::input::Replayed::Halted {
            halt: yantra::Halt::BeyondRam { .. },
            ..
        } => true,
        yantra::input::Replayed::Halted {
            halt: yantra::Halt::Finisher {
                status: Some(0), ..
            },
            ..
        } => {
            assert_eq!(out, vec![36], "K = {k}: the single-thread run sums 1..8");
            false
        }
        other => panic!("K = {k}: {other:?}"),
    }
}

/// The smallest K at which the single-thread run halts `BeyondRam` (binary search).
fn k0(img: &[u8]) -> u64 {
    let (mut lo, mut hi) = (0u64, 40_000u64);
    assert!(!single_beyond_ram(img, lo) && single_beyond_ram(img, hi));
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if single_beyond_ram(img, mid) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    hi
}

/// A threaded run of `img` on `machine`, answered as `(the end, Debug-printed, the octets)`.
fn run_on(mut m: Machine, img: &[u8], text: &str) -> (String, Vec<u8>) {
    let d = discover_any(&m, img);
    let mut t = Threads::new(&mut m, d).expect("stacks");
    let mut out: Vec<u8> = Vec::new();
    let end = replay_threads(&mut m, &mut t, &log(text), STEPS, &mut out);
    (format!("{end:?}"), out)
}

#[test]
fn w376_fix1_a_heap_past_ram_is_refused_naming_the_thread() {
    for (engine, twin) in NATIVE {
        let img = image(&heap_past_ram(), twin);
        let k = k0(&img);
        let text = format!("@0\n@0\n{k}\n@1\n@0\n7\n");
        let (end, out) = run_on(small_machine(&img), &img, &text);
        println!("fix 1 heap, {engine}: K0 = {k}, {end}, out {out:?}");
        assert!(
            end.starts_with("BeyondOwnStack { thread: 0,"),
            "{engine}: K0 = {k}: the heap went past RAM in thread 0 and the run must end \
             naming it, not {end} with {out:?}"
        );
        // THE CONTROL, one below K0: no store reaches past RAM, so the guard is silent; with
        // thread 1 run FIRST, thread 0 builds its run afterwards and sums 1..8.
        let text = format!("@1\n@0\n@0\n{}\n@0\n7\n", k - 1);
        let (end, out) = run_on(small_machine(&img), &img, &text);
        assert_eq!((end.as_str(), out), ("Ended", vec![36]), "{engine}: K0 - 1");
        // THE GAP LEFT ON PURPOSE (threads.rs, "NOT GUARDED"): at depth 2000 thread 1's
        // stack overflows DOWN past its own bottom into the top of RAM, below the line the
        // guard draws — the class thread 0's own stack overflow already is. Interleaved at
        // K0 - 1, thread 0's run sits just below that line, is overwritten, and the guard
        // does not fire. Pinned, so a change to it is seen.
        let text = format!("@0\n@0\n{}\n@1\n@0\n7\n", k - 1);
        let (end, out) = run_on(small_machine(&img), &img, &text);
        println!("fix 1 heap, {engine}: K0 - 1 interleaved: {end}, out {out:?}");
        assert_eq!(end, "Ended", "{engine}: below the line, no guard");
        assert_ne!(
            out,
            vec![36],
            "{engine}: thread 1's overflow reached thread 0's run"
        );
    }
}

#[test]
fn w376_fix1_a_stack_overflowing_into_another_threads_is_refused() {
    for (engine, twin) in NATIVE {
        let img = image(&stack_overflow(), twin);
        let m = Machine::load_elf(&img, yantra::ram_for(&img)).expect("loads");
        let (end, out) = run_on(m, &img, "@1\n@2\n@0\n@1\n7\n");
        println!("fix 1 overflow, {engine}: {end}, out {out:?}");
        assert!(
            end.starts_with("BeyondOwnStack { thread: 2,"),
            "{engine}: thread 2 overflowed into thread 1's stack and the run must end \
             naming it, not {end} with {out:?}"
        );
    }
}

/// FIX 2: only the FILE-BACKED octets are scanned — a tag word that appears in RAM only
/// beyond them (zeroed `.bss` at load, here written by hand) is not an interface.
#[test]
fn w376_fix2_a_tag_beyond_the_file_backed_octets_is_not_an_interface() {
    let img = image(&no_wait(1), false);
    let mut m = Machine::load_elf(&img, yantra::ram_for(&img)).expect("loads");
    let d = discover_any(&m, &img);
    let past = m.mem.len() - 64;
    m.mem[past..past + 8].copy_from_slice(&THREADS_TAG.to_le_bytes());
    assert_eq!(
        discover_result(&m, &img).map(|o| o.map(|x| x.threads_tag)),
        Ok(Some(d.threads_tag)),
        "the tag in .bss is not a second SASTHRDS"
    );
}

/// FIX 3: an UNTHREADED program whose initialised data holds the SASTHRDS word — one
/// global equal to it, no SASTHRID — is refused, by this message, and `yantra-run` exits
/// 1 at load: a half-declared thread interface does not run silently single-threaded.
#[test]
fn w376_fix3_a_lone_sasthrds_word_is_refused_by_name() {
    let src = format!(
        "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

सार्वजनिक चरः सूत्रगणसङ्केतक ॱॱ न६४ भवति {threads} ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    प्रत्यागमनम् ० ।
इति
",
        threads = dec(THREADS_TAG)
    );
    let want = format!(
        "the image declares SASTHRDS ({THREADS_TAG:#x}) without SASTHRID \
         ({THREAD_ID_TAG:#x}): a threaded image must declare both — the thread count and \
         the slot the host writes each thread's number into"
    );
    for (engine, twin) in NATIVE {
        let img = image(&src, twin);
        let m = Machine::load_elf(&img, yantra::ram_for(&img)).expect("loads");
        assert_eq!(discover_result(&m, &img), Err(want.clone()), "{engine}");
    }
    let dir = std::env::temp_dir().join(format!("w376-fix3-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let elf = dir.join("p.elf");
    std::fs::write(&elf, image(&src, false)).unwrap();
    let o = std::process::Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .arg(&elf)
        .output()
        .expect("yantra-run runs");
    let err = String::from_utf8_lossy(&o.stderr);
    assert_eq!(o.status.code(), Some(1), "{err}");
    assert!(
        err.lines()
            .any(|l| l == format!("threads: refused — {want}")),
        "{err}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// FIX 7: `run_threads` RESTORES the interpreter's event state on return — a wait in a
/// later call with no log set is "no event source", not SHORTER; and a log set before is
/// still the one a later wait reads.
#[test]
fn w376_fix7_run_threads_restores_the_event_state() {
    let file = format!("{MODULE}.t1");
    let src = aba();
    let mut srcs: Vec<(&str, &str)> = CHAIN.to_vec();
    srcs.push((file.as_str(), src.as_str()));
    let entry = format!("{MODULE}\u{971}मुख्यम्");
    let steps = [
        ThreadStep::Run(1),
        ThreadStep::Run(0),
        ThreadStep::Run(0),
        ThreadStep::Value(7),
    ];

    let mut it = Interpreter::load(&srcs, &spec_root()).expect("loads");
    it.run_threads(&entry, &steps, FUEL).expect("[B,A,A]");
    // The id slot holds 0 (the last thread), so the entry takes A's path and waits.
    let e = it
        .call(&entry, Vec::new(), FUEL)
        .expect_err("a wait with no log");
    assert!(
        e.reason.starts_with("WAIT") && e.reason.contains("no event source"),
        "{}",
        e.reason
    );

    let mut it = Interpreter::load(&srcs, &spec_root()).expect("loads");
    it.set_events(&[5]).expect("the event interface");
    it.run_threads(&entry, &steps, FUEL).expect("[B,A,A]");
    it.call(&entry, Vec::new(), FUEL)
        .expect("the log set before is read");
    assert_eq!(it.events_delivered(), 1, "the earlier log took the wait");
}

/// `threads::discover` on a machine loaded from `img` — the call this file makes for every
/// image, in one place so its signature is stated once.
fn discover_result(m: &Machine, img: &[u8]) -> Result<Option<threads::Declared>, String> {
    threads::discover(m, img)
}

fn discover_any(m: &Machine, img: &[u8]) -> threads::Declared {
    discover_result(m, img)
        .expect("the image is accepted")
        .expect("and threaded")
}

// ── V-009 (i-b2) round 2: the canary survives a later thread's entry ────────

/// Thread 0 recurses 1,400 deep — 1,760 octets past the stack's bottom, into the
/// 4 KiB guard, smashing the canary — returns, and waits; thread 1 then STARTS
/// (entering at `e_entry + 8`, past the canary's arming) and ends; thread 0
/// resumes and ends. Both endings check the canary and both must report the
/// overflow, `0x353B`. Before the store-if-zero arming, thread 1's entry wrote
/// the canary afresh and erased it: both ended `0`.
fn overflow_then_wait() -> String {
    program(
        2,
        "वृत्तिः अवरोहणम् आदाय क ॱॱ न६४ ददाति न६४ आदि
    यदि क समम् ० आदि
        प्रत्यागमनम् ० ।
    इति
    चरः पूर्वः ॱॱ न६४ भवति क वियोगः १ ।
    चरः अनुफलम् ॱॱ न६४ भवति अवरोहणम् पूर्वः ।
    प्रत्यागमनम् क योगः अनुफलम् ।
इति
",
        &format!(
            "    यदि स्वाङ्कः न्यूनम् १ आदि
        चरः गभीरम् ॱॱ न६४ भवति अवरोहणम् {depth} ।
        अवगणना भवति {w} ० ।
    इति
    प्रत्यागमनम् ० ।",
            depth = dec(1_400),
            w = wait(),
        ),
    )
}

#[test]
fn v009b2_an_overflow_by_thread_0_survives_thread_1_starting_and_ending() {
    let src = overflow_then_wait();
    for (engine, twin) in NATIVE {
        let run = native(&image(&src, twin), &log("@0\n@1\n@0\n7\n"));
        println!("v009b2 {engine}: {:?}", run.outcome);
        assert_eq!(
            run.outcome,
            ended(&[(1, 0x353B), (0, 0x353B)], b""),
            "{engine}: thread 0's overflow must be reported at both endings"
        );
    }
}
