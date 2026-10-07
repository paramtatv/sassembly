//! **THE `.t1` SIDE ASSEMBLES A SOURCE AND WRITES AN IMAGE `yantra` RUNS.**
//!
//! No Rust compiler stage is in this path. `पदविभाग` lexes, `वाक्यविभाग` reads
//! the sentences, `सङ्केतन` encodes them to octets, and `कोश` writes the ELF —
//! four `.t1` modules under the interpreter. Rust loads the result and runs it,
//! which is the machine's job and not the compiler's.
//!
//! # THE LOOP IN THIS FILE WAS A MISSING COMPILER STAGE. IT IS NOT ANY MORE.
//!
//! It said: "Nothing in `.t1` loops the tokens — the Sassembly assembler has no
//! top level." **That was wrong, and the tree said so under a name I did not
//! search for.** `वाक्यविभागॱसङ्कलनम्` takes source octets, lexes them and calls
//! the statement splitter `वाक्यविभाजनम्`; its margin reads "Rust's
//! `assemble_source` — the same". I grepped for readers named `…पठनम्` and it is
//! named `सङ्कलनम्`, so I concluded absence from a naming convention — the third
//! time in one day that a name mismatch read as a missing thing.
//!
//! What WAS missing is smaller and was real: the splitter recorded statements
//! and nothing turned them into instructions, so `सङ्कलनम्` answered four
//! statements and built zero आज्ञा. Four lines in `वाक्यविभाजनम्` join the two,
//! and `the_assemblers_top_level_takes_a_source_and_the_image_runs` below is the
//! one-call path.
//!
//! `sentences()` here stays as the LOWER-LEVEL check — it drives `वाक्यपठनम्`
//! directly, so a defect in the splitter and a defect in the reader fail in
//! different tests instead of one.
//!
//! Naming it matters because the alternative hides the gap behind a green test.
//! Every other test that drives `सङ्केतनॱवस्तुसङ्केतनम्` today does so over
//! arenas the RUST parser filled — `t1_exec_encode.rs`'s `mirror` calls
//! `sadhana::parse::assemble_program` — which is legitimate for testing the
//! encoder and is NOT a `.t1` path. This one is, apart from the loop, and the
//! loop is named.
//!
//! # What is deliberately absent
//!
//! No symbols, no relocations, no linker. A self-contained program whose entry
//! is its first instruction needs none: `प्रतिबिम्बलेखनम्` sets `e_entry` to the
//! load address, so there is nothing to resolve. That is what makes this
//! reachable before the object constructor exists — the mapping from the
//! encoder's shapes to `वास्तु`'s WIDENS this to multi-object programs; it does
//! not unblock it.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::{Halt, Machine};

const LOAD: u64 = 0x8000_0000;
/// RAM FOR A `.t1` IMAGE IS SIZED FROM THE IMAGE, NOT FROM `DEFAULT_RAM`.
///
/// `fa8fbc05` (2026-09-13) raised the record region to 320 MiB for a full native
/// self-compile; `DEFAULT_RAM` is 20 MiB. Every call below that loads a `.t1`
/// image at this constant is refused AT LOAD with "segment needs 335,616,120
/// bytes and RAM is 20,971,520" — which is how the hop ladder lost five sources
/// for two days (`b02f62ea`), reading a loader refusal as a compile failure.
///
/// `yantra::ram_for` is the one statement of that sizing and `yantra-run` uses
/// it. Kept as a named constant for the small hand-built images in this file,
/// which are nowhere near either bound.
const RAM: usize = yantra::DEFAULT_RAM;
const BUDGET: u64 = yantra::DEFAULT_STEPS;
/// `पदविभाग`'s `दण्डभेद` — the `।` that ends a sentence.
const DANDA: i128 = 4;

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

/// `पदविभाग` · `वाक्यविभाग` · `सङ्केतन` · `कोश`, and what they reach.
fn assembler() -> Interpreter {
    let names = [
        "lex.t1",
        "vakyavibhaga.t1",
        "encode.t1",
        "vishlesana.t1",
        "sanskrit_text.t1",
        "ashtaka.t1",
        "kosha.t1",
    ];
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("{names:?} load: {e:?}"))
}

fn call(it: &mut Interpreter, name: &str, args: Vec<Value>, fuel: u64) -> Value {
    it.call(name, args, fuel)
        .unwrap_or_else(|e| panic!("`{name}` runs: {e}"))
}

/// **THE MISSING DRIVER.** Every sentence's `[head, danda)` span, over the
/// tokens `पदविभाग` just wrote. One `.t1` routine reads one sentence; nothing
/// in `.t1` decides where they begin and end, so this does.
fn sentences(it: &Interpreter, count: usize) -> Vec<(usize, usize)> {
    let arena = match it.global("चिह्नककोश") {
        Some(Value::Arena(a)) => a.clone(),
        other => panic!("`चिह्नककोश` is an arena, not {other:?}"),
    };
    // A record's field, by name — `nirvahana` exports no accessor, so every
    // test that reads one does this.
    let kind_at = |i: usize| -> i128 {
        match arena.borrow().get(i) {
            Some(Value::Record(r)) => r.borrow().get("भेद").and_then(Value::as_int).unwrap_or(0),
            _ => 0,
        }
    };
    let mut out = Vec::new();
    let mut head = 1usize; // the arena is 1-based
    for i in 1..=count {
        if kind_at(i) == DANDA {
            if i > head {
                out.push((head, i));
            }
            head = i + 1;
        }
    }
    out
}

/// **THE ACCEPTANCE: a Sassembly source becomes an image that RUNS, through
/// `.t1` alone.** The program stores `0x00073333` to the finisher, so the
/// machine halts with status 7 — a number chosen because it is not 0, which is
/// what a finisher reports for `0x5555` and what a program that never ran would
/// leave behind.
///
/// # IT PASSES NOW, AND WHAT IT FOUND ON THE WAY IS THE POINT
///
/// This was `#[ignore]`d for one commit, because `सङ्केतन` refused the program
/// at every length and the writer was never reached. The probe below bisected
/// it — **E02 at one instruction, E23 only from two up** — and E02 is
/// "`{0}` is not a register or a numeral": an OPERAND the encoder could not
/// read. E23 was its shadow, because `अन्तिमसङ्केतनदोषः` holds the LAST refusal
/// and a mis-sized instruction makes every later layout disagree.
///
/// THE DEFECT WAS ONE CONSTANT. `वाक्यविभागॱकारकपदपठनम्` passed `असत्यम्` —
/// hardcoded false — as `सङ्ख्यात्व` for every operand it read, so a numeral was
/// recorded as not-a-numeral and `encode.t1:3818` refused it. The routine's own
/// margin said why nobody had seen it: a numeral "is refused one arm down as
/// P10" because `अक्षरकोशॱसङ्ख्या` was a stub. `W-257` and 47bad575 ended that,
/// numerals began reaching the last line, and the constant answered a question
/// nobody was asking any more.
///
/// A STALE PREMISE, A CONSTANT STANDING IN FOR A QUESTION, AND NO CALLER. Any
/// two of those three are survivable; together they are invisible. Every test
/// over `सङ्केतन` builds operands with `कारकपदयोजनम्` DIRECTLY from a
/// `sadhana::parse` result — `t1_exec_encode.rs`'s `mirror` — so the reader that
/// fills the field had never had its output encoded by anything.
///
/// # THE LOOP IN THIS FILE IS STILL A MISSING COMPILER STAGE
///
/// `sentences()` below is what a ported driver would do, and naming it matters:
/// nothing in `.t1` loops the tokens, so the assembler still has no top level.
/// What this file now shows is that the modules BENEATH that driver work — a
/// source in, an image out, running on the machine — which is a different claim
/// from "the chain is self-hosted" and a smaller one.
///
#[test]
fn t1_assembles_a_source_and_writes_an_image_that_runs() {
    // Self-contained: no labels, no directives, no data, no external names.
    // lui t4, 0x100      → 0x0010_0000, the finisher
    // lui t5, 0x73 ; addi t5, t5, 0x333 → 0x0007_3333
    // sw  t5, 0(t4)      → halt, status 7
    let src = concat!(
        "उपरिभारः क्षणिक४म् ०षोड्१००न ।\n",
        "उपरिभारः क्षणिक५म् ०षोड्७३न ।\n",
        "योगः क्षणिक५म् क्षणिक५न ०षोड्३३३न ।\n",
        "निधानम्ॱअ३२ क्षणिक४य् ०न क्षणिक५न ।\n",
    );

    let mut it = assembler();
    // RESET, THEN THE SPEC TABLES, THEN THE SOURCE — and the order is not a
    // detail. `आरम्भः` clears `संज्ञाकुलसूचकाङ्क` and `निर्देशसूचकाङ्क`, the
    // TABLE cursors, so reading the tables first and resetting after empties
    // them again: `वाक्यपठनम्` then refuses every mnemonic with
    // `अज्ञातसंज्ञादोषः`, which is what an unread table looks like from inside.
    //
    // Getting it backwards does not fail there, either. It fails much later at
    // E23 — "laid out at X but written at Y" — because the arenas still held
    // the previous state and `कार्यक्रमरचना` sealed ranges that included it.
    // A stale arena is a wrong LAYOUT, and a wrong layout assembles cleanly and
    // branches to the wrong place. That is this driver's whole content, and it
    // is why the missing stage is a stage rather than a loop.
    call(&mut it, "वाक्यविभागॱआरम्भः", vec![], 200_000_000);
    call(&mut it, "वाक्यविभागॱसंज्ञाकुलपठनम्", vec![], 2_000_000_000);
    call(&mut it, "वाक्यविभागॱनिर्देशकोशपठनम्", vec![], 2_000_000_000);
    let tokens = call(
        &mut it,
        "पदविभागॱपदविभाग",
        vec![octets(src.as_bytes())],
        2_000_000_000,
    )
    .as_int()
    .unwrap_or(0);
    assert!(tokens > 0, "the source lexes to tokens");
    let spans = sentences(&it, tokens as usize);
    assert_eq!(spans.len(), 4, "four sentences: {spans:?}");

    for (head, end) in spans {
        let tok = match it.global("चिह्नककोश") {
            Some(Value::Arena(a)) => a.borrow()[head].clone(),
            other => panic!("`चिह्नककोश` is an arena, not {other:?}"),
        };
        let read = call(
            &mut it,
            "वाक्यविभागॱवाक्यपठनम्",
            vec![tok, Value::Int(head as i128), Value::Int(end as i128)],
            200_000_000,
        );
        assert_ne!(
            read.as_int(),
            Some(0),
            "वाक्यपठनम् refused the sentence at tokens {head}..{end}"
        );
    }

    // WHAT THE DRIVER ACTUALLY BUILT, before asking the encoder to trust it.
    // The Rust assembler reads this same source as FOUR instructions
    // (`sadhana::parse::assemble_program`), so a different count here is the
    // driver's fault and not the source's.
    let built = it
        // **RENAMED BY `W-249`, AND THE LOAD SET IS WHY.** `assembler()` at `:71`
        // loads lex · vakyavibhaga · encode · vishlesana · sanskrit_text ·
        // ashtaka · kosha — **`ir.t1` IS NOT IN IT.** Both modules once declared
        // a bare `आज्ञासूचकाङ्क` into one flat cell; the rename moved
        // `वाक्यविभाग`'s, and no module in THIS set declares the bare name any
        // more, so the lookup went absent and `.unwrap_or(-1)` reported `-1`.
        // The 4 still means what the comment above says: `वाक्यविभाग` is the
        // statement splitter and the source is four lines of assembly.
        .global("वाक्यविभागआज्ञासूचकाङ्क")
        .and_then(Value::as_int)
        .unwrap_or(-1);
    assert_eq!(
        built, 4,
        "the driver read {built} instructions where Rust reads 4 from the same source"
    );
    let program = call(&mut it, "वाक्यविभागॱकार्यक्रमरचना", vec![], 20_000_000);
    let obj = call(&mut it, "सङ्केतनॱवस्तुसङ्केतनम्", vec![program], 2_000_000_000);
    let text = match obj.octets() {
        Some(o) => o.as_slice().to_vec(),
        // NAME THE DIAGNOSTIC, DO NOT SAY `Nil`. The encoder records its
        // refusal in `अन्तिमसङ्केतनदोषः` and returns nothing; a test that
        // reports only the nothing sends the next reader to the wrong file.
        None => {
            let why = it.global("अन्तिमसङ्केतनदोषः").cloned();
            panic!("वस्तुसङ्केतनम् refused the program.\n  diagnostic: {why:?}");
        }
    };
    assert_eq!(text.len(), 16, "four instructions, four octets each");

    let image = call(
        &mut it,
        "कोशॱप्रतिबिम्बलेखनम्",
        vec![
            octets(&text),
            octets(&[]),
            Value::Int(0),
            Value::Int(i128::from(LOAD)),
        ],
        200_000_000,
    );
    let image = match image.octets() {
        Some(o) => o.as_slice().to_vec(),
        None => panic!("प्रतिबिम्बलेखनम् returned {image:?}"),
    };

    let mut m = Machine::load_elf(&image, RAM).expect("yantra loads an image kosha.t1 wrote");
    let mut out = Vec::new();
    let halt = m.run(BUDGET, &mut out);
    match halt {
        Halt::Finisher {
            status: Some(7), ..
        } => {}
        other => panic!("expected the finisher at status 7, got {other:?}\ntext {text:02x?}"),
    }
    println!("METRIC t1_end_to_end_image_octets {}", image.len());
}

/// A PROBE, not an acceptance: how many instructions can `सङ्केतन` encode from
/// a driver-built program before E23 says the layout and the emission disagree?
#[test]
fn how_many_instructions_encode_before_the_layout_disagrees() {
    let lines = [
        "उपरिभारः क्षणिक४म् ०षोड्१००न ।\n",
        "उपरिभारः क्षणिक५म् ०षोड्७३न ।\n",
        "योगः क्षणिक५म् क्षणिक५न ०षोड्३३३न ।\n",
        "निधानम्ॱअ३२ क्षणिक४य् ०न क्षणिक५न ।\n",
    ];
    for n in 1..=lines.len() {
        let src: String = lines[..n].concat();
        let mut it = assembler();
        call(&mut it, "वाक्यविभागॱआरम्भः", vec![], 200_000_000);
        call(&mut it, "वाक्यविभागॱसंज्ञाकुलपठनम्", vec![], 2_000_000_000);
        call(&mut it, "वाक्यविभागॱनिर्देशकोशपठनम्", vec![], 2_000_000_000);
        let tokens = call(
            &mut it,
            "पदविभागॱपदविभाग",
            vec![octets(src.as_bytes())],
            2_000_000_000,
        )
        .as_int()
        .unwrap_or(0);
        for (head, end) in sentences(&it, tokens as usize) {
            let tok = match it.global("चिह्नककोश") {
                Some(Value::Arena(a)) => a.borrow()[head].clone(),
                other => panic!("arena, not {other:?}"),
            };
            call(
                &mut it,
                "वाक्यविभागॱवाक्यपठनम्",
                vec![tok, Value::Int(head as i128), Value::Int(end as i128)],
                200_000_000,
            );
        }
        let program = call(&mut it, "वाक्यविभागॱकार्यक्रमरचना", vec![], 20_000_000);
        let obj = call(&mut it, "सङ्केतनॱवस्तुसङ्केतनम्", vec![program], 2_000_000_000);
        // NAME THE CODE PER LENGTH. The first version printed only "REFUSED"
        // and I read the four-instruction case's E23 onto all four — one cause
        // assumed because one cause explained the one case I had looked at.
        let got = match obj.octets() {
            Some(o) => format!("{} octets", o.as_slice().len()),
            None => {
                let code = it
                    .global("\u{905}\u{928}\u{94d}\u{924}\u{93f}\u{92e}\u{938}\u{919}\u{94d}\u{915}\u{947}\u{924}\u{928}\u{926}\u{94b}\u{937}\u{903}")
                    .and_then(|v| match v {
                        Value::Record(r) => r
                            .borrow()
                            .get("\u{938}\u{919}\u{94d}\u{915}\u{947}\u{924}\u{93e}\u{919}\u{94d}\u{915}")
                            .cloned(),
                        _ => None,
                    })
                    .and_then(|c| {
                        c.octets()
                            .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
                    })
                    .unwrap_or_else(|| "?".into());
                format!("REFUSED {code}")
            }
        };
        println!("PROBE {n} instruction(s): {got}");
    }
}

/// **THE DIAGNOSIS PROBE.** One instruction, built two ways, records compared.
///
/// `mirror` in `t1_exec_encode.rs` builds a program by calling
/// `कारकपदयोजनम्`/`आज्ञायोजनम्` from a `sadhana::parse` result and the encoder
/// accepts it. `वाक्यपठनम्` builds one from tokens and the encoder refuses it
/// with E23. The two must differ in the `आज्ञा` record, and this prints both.
#[test]
fn what_vaakyapathanam_builds_differs_from_what_the_encoder_accepts() {
    let src = "उपरिभारः क्षणिक४म् ०षोड्१००न ।\n";
    let mut it = assembler();
    call(&mut it, "वाक्यविभागॱआरम्भः", vec![], 200_000_000);
    call(&mut it, "वाक्यविभागॱसंज्ञाकुलपठनम्", vec![], 2_000_000_000);
    call(&mut it, "वाक्यविभागॱनिर्देशकोशपठनम्", vec![], 2_000_000_000);
    let tokens = call(
        &mut it,
        "पदविभागॱपदविभाग",
        vec![octets(src.as_bytes())],
        2_000_000_000,
    )
    .as_int()
    .unwrap_or(0);
    for (head, end) in sentences(&it, tokens as usize) {
        let tok = match it.global("चिह्नककोश") {
            Some(Value::Arena(a)) => a.borrow()[head].clone(),
            other => panic!("arena, not {other:?}"),
        };
        call(
            &mut it,
            "वाक्यविभागॱवाक्यपठनम्",
            vec![tok, Value::Int(head as i128), Value::Int(end as i128)],
            200_000_000,
        );
    }
    for name in ["आज्ञासूचकाङ्क", "कारकपदसूचकाङ्क", "संरेखसूचकाङ्क", "दोषसूचकाङ्क"]
    {
        println!(
            "CURSOR {name} = {:?}",
            it.global(name).and_then(Value::as_int)
        );
    }
    // THE OPERAND RECORDS THEMSELVES. `कारकपद` is {मूलपाठ, कारक, सङ्ख्यात्व} —
    // the text with its sigil removed, the role number `पदविभाग` gave it, and
    // whether it is a numeral. E02 says the encoder could not read one of these
    // as a register or a numeral, so the answer is a FIELD and this prints them
    // rather than reasoning from the two writers' code.
    if let Some(Value::Arena(a)) = it.global("कारकपदकोश") {
        for (i, v) in a.borrow().iter().enumerate().take(4) {
            if let Value::Record(r) = v {
                let f = r.borrow();
                let base = f
                    .get("मूलपाठ")
                    .and_then(Value::octets)
                    .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
                    .unwrap_or_else(|| "<none>".into());
                println!(
                    "OPERAND[{i}] मूलपाठ={base:?} कारक={:?} सङ्ख्यात्व={:?}",
                    f.get("कारक").and_then(Value::as_int),
                    f.get("सङ्ख्यात्व"),
                );
            }
        }
    }
    if let Some(Value::Arena(a)) = it.global("आज्ञाकोश") {
        for (i, v) in a.borrow().iter().enumerate().take(3) {
            if let Value::Record(r) = v {
                let f = r.borrow();
                println!(
                    "INST[{i}] व्याप्ति={:?} प्रकारसंख्यान={:?} कारकारम्भ={:?} कारकसंख्यान={:?} पङ्क्ति={:?}",
                    f.get("व\u{94d}याप\u{94d}ति").and_then(Value::as_int),
                    f.get("प\u{94d}रकारस\u{902}ख\u{94d}यान")
                        .and_then(Value::as_int),
                    f.get("कारकारम\u{94d}भ").and_then(Value::as_int),
                    f.get("कारकस\u{902}ख\u{94d}यान").and_then(Value::as_int),
                    f.get("पङ\u{94d}क\u{94d}ति").and_then(Value::as_int),
                );
                // `कुल` is the registry row the mnemonic resolved to, and it is
                // what the LAYOUT sizes an instruction from. A record whose
                // family is empty sizes to nothing, which is a layout of zeroes
                // and an E23 the moment emission passes the first instruction.
                match f.get("क\u{941}ल") {
                    Some(Value::Record(k)) => {
                        let kb = k.borrow();
                        let mut keys: Vec<&String> = kb.keys().collect();
                        keys.sort();
                        let shown: Vec<String> = keys
                            .iter()
                            .map(|n| format!("{n}={:?}", kb.get(*n).and_then(Value::as_int)))
                            .collect();
                        println!("  कुल: {}", shown.join(" "));
                    }
                    other => println!("  कुल IS NOT A RECORD: {other:?}"),
                }
            }
        }
    }
}

/// **THE ASSEMBLER'S TOP LEVEL, IN ONE CALL, WITH NO RUST IN THE PATH.**
///
/// `वाक्यविभागॱसङ्कलनम्` takes SOURCE OCTETS — it lexes with `पदविभागॱपदविभाग`,
/// splits statements with `वाक्यविभाजनम्`, and now reads each instruction
/// statement with `वाक्यपठनम्`. Its own margin calls it "Rust's
/// `assemble_source` — the same".
///
/// IT WAS HALF PRESENT AND THAT READ LIKE ABSENT. Before this row `सङ्कलनम्`
/// answered FOUR statements for this source and left `आज्ञासूचकाङ्क` at ZERO:
/// the splitter recorded statements into `वाक्यकोश` and nothing turned them into
/// instructions, so `सङ्केतन` encoded 0 octets and the image halted
/// `Unimplemented` at its first word. A splitter with no reader behind it is
/// indistinguishable, from outside, from having no top level at all — which is
/// what I reported before measuring it.
///
/// The join is four lines in `वाक्यविभाजनम्`, guarded so a statement the walk
/// REFUSED is not read as an instruction anyway.
#[test]
fn the_assemblers_top_level_takes_a_source_and_the_image_runs() {
    let src = concat!(
        "उपरिभारः क्षणिक४म् ०षोड्१००न ।\n",
        "उपरिभारः क्षणिक५म् ०षोड्७३न ।\n",
        "योगः क्षणिक५म् क्षणिक५न ०षोड्३३३न ।\n",
        "निधानम्ॱअ३२ क्षणिक४य् ०न क्षणिक५न ।\n",
    );
    assert_eq!(
        assemble_and_run(src),
        Some(7),
        "four instructions through सङ्कलनम् must halt the finisher at 7"
    );

    // THE CONTROL. A source whose mnemonic does not exist must be REFUSED, so
    // that the pass above is the mechanism agreeing rather than this test being
    // lenient about what it accepts.
    assert_eq!(
        assemble_and_run("अविद्यमानाज्ञा क्षणिक४म् २५६न ।\n"),
        None,
        "an unknown mnemonic must refuse; a pass here would mean सङ्कलनम् \
         accepts anything and the assertion above proves nothing"
    );
}

/// Source octets to the finisher's status, through `.t1` alone — or `None` when
/// any stage refuses. `nirvahana` interpreting the modules is the bootstrap and
/// is not a Rust COMPILER stage: no Rust lexes, parses, encodes or links here.
fn assemble_and_run(src: &str) -> Option<u64> {
    let mut it = assembler();
    call(&mut it, "वाक्यविभागॱआरम्भः", vec![], 200_000_000);
    call(&mut it, "वाक्यविभागॱसंज्ञाकुलपठनम्", vec![], 2_000_000_000);
    call(&mut it, "वाक्यविभागॱनिर्देशकोशपठनम्", vec![], 2_000_000_000);
    call(
        &mut it,
        "वाक्यविभागॱसङ्कलनम्",
        vec![octets(src.as_bytes())],
        4_000_000_000,
    );
    let program = call(&mut it, "वाक्यविभागॱकार्यक्रमरचना", vec![], 20_000_000);
    let text = call(&mut it, "सङ्केतनॱवस्तुसङ्केतनम्", vec![program], 2_000_000_000)
        .octets()?
        .as_slice()
        .to_vec();
    if text.is_empty() {
        return None;
    }
    let image = call(
        &mut it,
        "कोशॱप्रतिबिम्बलेखनम्",
        vec![
            octets(&text),
            octets(&[]),
            Value::Int(0),
            Value::Int(i128::from(LOAD)),
        ],
        200_000_000,
    )
    .octets()?
    .as_slice()
    .to_vec();
    let mut m = Machine::load_elf(&image, RAM).ok()?;
    let mut out = Vec::new();
    match m.run(BUDGET, &mut out) {
        Halt::Finisher { status, .. } => status,
        _ => None,
    }
}

/// **THE WHOLE CHAIN, DRIVEN FROM `.t1`: ONE CALL FROM A `.t1` SOURCE TO AN
/// ELF IMAGE `yantra` LOADS.**
///
/// `assemble_and_run` above is the shape this replaces — Rust calling
/// `सङ्कलनम्`, then `कार्यक्रमरचना`, then `वस्तुसङ्केतनम्`, then
/// `प्रतिबिम्बलेखनम्`, in order, holding each intermediate. **Every stage was
/// already `.t1`; the ORDERING was Rust.** So a green there proved the stages
/// work and said nothing about whether anything in `.t1` could run them.
///
/// **THIS CALLS ONE ROUTINE.** Rust supplies the source octets and loads the
/// resulting image; between those two points nothing Rust does is a compiler
/// stage — `nirvahana` interpreting the modules is the bootstrap, and the
/// module list is `CHAIN`, the shipped manifest, not a hand-written set.
///
/// The controls are both halves of the claim: a source that assembles must
/// produce a RUNNING image at a chosen status, and a source with an unknown
/// mnemonic must produce NOTHING. A driver that answered octets for everything
/// would pass the first alone.
#[test]
fn the_driver_takes_a_source_to_an_image_that_loads() {
    let texts: Vec<(String, String)> = CHAIN
        .iter()
        .map(|(n, t)| ((*n).to_string(), (*t).to_string()))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let mut it = Interpreter::load(&refs, &spec_root()).expect("CHAIN loads");

    // NO SETUP CALL. The driver resets the assembler's tables itself, between
    // the two halves — Rust supplies a source and reads an image, and does not
    // sequence anything in between. A setup call here would be Rust holding
    // part of the ordering, which is the thing this row removes.

    // A `.t1` SOURCE, NOT SASSEMBLY. `मण्डलप्रतिबिम्बम्` begins with
    // `मण्डलसङ्कलनम्`, the COMPILER's front half — lex, parse, resolve,
    // typecheck, IR, emit — whose input is a T1 module. The first version of
    // this test fed it Sassembly and got an empty image with
    // `आज्ञासूचकाङ्क` at 0 and no encoder diagnostic: the front half had
    // refused a source in the wrong language and the back half never ran.
    let src = concat!(
        "मण्डलम् क ॥\n",
        "सार्वजनिक वृत्तिः ग ददाति न६४ आदि\n",
        "    प्रत्यागमनम् ३ योगः ४ ।\n",
        "इति\n",
    );
    let image = call(
        &mut it,
        "शृङ्खलाॱमण्डलप्रतिबिम्बम्",
        vec![octets(src.as_bytes()), octets("क".as_bytes())],
        60_000_000_000,
    )
    .octets()
    .map(|o| o.as_slice().to_vec())
    .unwrap_or_default();

    assert!(
        !image.is_empty(),
        "the driver produced no image.\n  अन्तिमवस्तुदोषः: {:?}\n  अन्तिमसङ्केतनदोषः: {:?}\n  आज्ञासूचकाङ्क: {:?}",
        it.global("अन्तिमवस्तुदोषः"),
        it.global("अन्तिमसङ्केतनदोषः"),
        it.global("आज्ञासूचकाङ्क"),
    );
    assert_eq!(&image[..4], b"\x7fELF", "and it is an ELF image");

    // **IT LOADS. IT DOES NOT YET RUN, AND THAT IS NOT A DEFECT IN THE DRIVER.**
    // `मण्डलसङ्कलनम्` emits a MODULE — the probe reads
    // `॥ वैश्विकम् कग ॥ / कगॱॱ / …` — and a module is not a program. There is
    // no `यन्त्रारम्भ` in the emitted text, so `e_entry` points at a routine
    // body: it executes, finds no stack pointer, and halts `BeyondRam` at the
    // second instruction with `addr` = -8.
    //
    // The startup IS emittable — `यन्त्रोत्सर्जनॱयन्त्रारम्भोत्सर्जनम्` writes
    // it — but `यन्त्रमण्डलोत्सर्जनम्` does not include it, and linking it in
    // means assembling TWO objects and linking them together. **Asserting a
    // halt status here would either fail honestly or be made to pass by
    // testing something other than the driver**, so this asserts what is true:
    // the driver's output is an ELF image `yantra` accepts.
    let loaded = Machine::load_elf(&image, RAM);
    assert!(
        loaded.is_ok(),
        "yantra must LOAD what शृङ्खला wrote: {:?}",
        loaded.err()
    );
    println!("METRIC t1_driver_image_octets {}", image.len());

    // THE CONTROL: an unknown mnemonic must yield NOTHING, or the test above
    // passes for a driver that answers octets regardless of its input.
    let refused = call(
        &mut it,
        "शृङ्खलाॱमण्डलप्रतिबिम्बम्",
        vec![
            octets(
                "मण्डलम् ख ॥\nसार्वजनिक वृत्तिः घ ददाति न६४ आदि\n    प्रत्यागमनम् अपरिभाषितम् ।\nइति\n"
                    .as_bytes(),
            ),
            octets("ख".as_bytes()),
        ],
        60_000_000_000,
    )
    .octets()
    .map(|o| o.as_slice().to_vec())
    .unwrap_or_default();
    assert!(
        refused.is_empty(),
        "a source naming something it does not define must produce no image; got {} octets",
        refused.len()
    );
}

/// **`W-279`'s REMAINING ACCEPTANCE HALF — the driver walks the WHOLE CORPUS.**
///
/// The row's `WHAT REMAINS` names exactly this: *"the chain drives ONE source at
/// a time; nothing yet walks all 19 and feeds each module's output to the
/// next."* The seventeen sources that run today run under an orchestration that
/// is RUST — `paradigm_encode.rs` loops and links. This asserts the same walk
/// with the loop and the link BOTH on the `.t1` side.
///
/// **ENUMERATED AT RUNTIME AND NEVER PINNED.** The acceptance said "the 19 `.t1`
/// sources" while the corpus held 20 — the twentieth being `shrinkhala.t1`, the
/// driver itself, added the day after the row was written. Nothing edited the
/// number, nothing went red, and it drifted stale with no commit as the culprit:
/// a COUNT standing where a PROPERTY was meant. So this globs the directory and
/// asserts against what it FOUND. A twenty-first source extends this test on the
/// day it lands instead of silently escaping it.
///
/// Enumeration lives here because `cc *.c` is the SHELL globbing and handing
/// over a list — reading a directory is not a compiler's job, and a driver whose
/// signature takes the list is not a compiler that fell short.
#[test]
#[ignore = "MEASUREMENT, 1237s — and it PASSES. The old reason said BLOCKED at \
           encode.t1:5820, refusing R_RISCV_PCREL_LO12_I unconditionally; that site \
           now handles it in an अन्यथा branch and the claim is stale. Measured \
           2026-09-18: 21 sources, 21 objects linked, 1,373,231 octets, halt \
           Finisher { value: 21845, status: Some(0) }, 1763s. RE-TAKEN \
           2026-10-01: 21 sources, 21 objects linked, 1,457,194 octets, halt \
           Finisher { value: 21845, status: Some(0) }. THE PASS HELD AND THE \
           SIZE MOVED +83,963 \
           over thirteen days, measured on an x86_64 Linux host. THE FORM OF \
           THAT SENTENCE IS LOAD-BEARING AND WAS WRONG ONCE: `loop-context.sh` \
           greps `DATE: N sources, N objects linked` out of this very string, so \
           a reading written as prose (`2026-10-01 at <sha> on a Linux host: 21 \
           sources, Some(21) objects linked`) PARSES AS NOTHING and leaves the \
           banner quoting the older figure while looking updated. Write the \
           date, a colon, and bare integers. ADDED rather than substituted, \
           because this \
           reason's own instruction is to quote the size WITH its date or not \
           at all, and a series of dated readings is what makes the drift \
           legible. STAMPED WITH ITS COMMIT AND NOT ONLY ITS DATE: the trunk \
           moved to a6fdd068 hours later and the image SHRANK to 1,441,546 \
           there, because W-306c slice 3 made the narrow indexed store emit ONE \
           width-bearing instruction where it had emitted a sequence. So \
           1,457,194 is a reading of 68cdd66b, not of 2026-10-01. NO NEW \
           RUNTIME IS CLAIMED: the 23m47s window that produced this figure \
           covered the WHOLE `-p yantra -- --ignored` suite, not this test \
           alone, so the 1763s above stands unchallenged. THE OCTET COUNT \
           TRACKS THE LIVE CORPUS AND IS STALE THE MOMENT A .t1 LANDS: it read \
           1,371,623 on 2026-09-16 and 1,372,743 earlier on 09-18, and the \
           differences are real work, not noise — 1cc531bd alone took 6 \
           instructions out of ir.t1 and 24 octets out of the image, four per \
           instruction, reconciled across two instruments. Quote it WITH its \
           date or not at all. The PASS is the durable claim; the size is a \
           reading. Ignored for COST, not for breakage — 20 minutes on \
           every workspace run is the pin tax ruling item 1 struck. Run it \
           deliberately: cargo test -p yantra --release --test kosha_end_to_end -- \
           --include-ignored --exact the_driver_compiles_every_corpus_source_into_one_image_that_runs"]
fn the_driver_compiles_every_corpus_source_into_one_image_that_runs() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    files.sort();

    // NON-VACUITY. An empty glob compiles nothing, links the startup alone, and
    // produces a perfectly good image — passing this test while proving nothing.
    assert!(
        files.len() >= 15,
        "the corpus glob found only {} sources; a thin glob passes this test \
         vacuously by linking the startup alone",
        files.len()
    );
    println!("METRIC t1_driver_corpus_sources {}", files.len());

    // The driver takes the MODULE name, not the file name — `मण्डलसङ्कलनम्`
    // uses it as half of every label the emitter writes. A source declaring no
    // module is passed an empty name and refuses in the front half, which is
    // the honest stop: `lib.t1` declares nothing and must not be special-cased
    // into looking like a success.
    let mut sources: Vec<Value> = Vec::new();
    let mut modules: Vec<Value> = Vec::new();
    for p in &files {
        let text = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{p:?}: {e}"));
        let module = text
            .lines()
            .find_map(|l| l.strip_prefix("मण्डलम् "))
            .and_then(|r| r.split_whitespace().next())
            .unwrap_or("")
            .to_string();
        sources.push(octets(text.as_bytes()));
        modules.push(octets(module.as_bytes()));
    }
    let n = sources.len();
    let arena = |v: Vec<Value>| Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(v)));

    let texts: Vec<(String, String)> = CHAIN
        .iter()
        .map(|(nm, t)| ((*nm).to_string(), (*t).to_string()))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(nm, t)| (nm.as_str(), t.as_str()))
        .collect();
    let mut it = Interpreter::load(&refs, &spec_root()).expect("CHAIN loads");

    let image = call(
        &mut it,
        "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
        vec![arena(sources), arena(modules), Value::Int(n as i128)],
        400_000_000_000,
    )
    .octets()
    .map(|o| o.as_slice().to_vec())
    .unwrap_or_default();

    // HOW MANY OBJECTS WENT IN, asked of the driver rather than inferred from
    // the image. Twenty sources reaching a driver that silently linked one would
    // still produce a valid ELF, and every assertion below would pass.
    let linked = it
        .global("शृङ्खलाॱसंयोजितवस्तुसंख्या")
        .or_else(|| it.global("संयोजितवस्तुसंख्या"))
        .and_then(|v| v.as_int());
    println!("METRIC t1_driver_linked_objects {linked:?}");

    assert!(
        !image.is_empty(),
        "the driver produced no image from {n} sources.\n  linked objects: {linked:?}\
         \n  अन्तिमवस्तुदोषः: {:?}\n  अन्तिमसङ्केतनदोषः: {:?}",
        it.global("अन्तिमवस्तुदोषः"),
        it.global("अन्तिमसङ्केतनदोषः"),
    );
    assert_eq!(&image[..4], b"\x7fELF", "and it is an ELF image");
    println!("METRIC t1_driver_corpus_image_octets {}", image.len());

    // MORE THAN THE STARTUP. One object is the startup alone — an image that
    // loads and halts success while having compiled nothing.
    assert!(
        linked.unwrap_or(0) > 1,
        "the driver linked {linked:?} object(s): the startup alone, or none. \
         The image is real and proves nothing about the corpus."
    );

    // **LOADS.**
    let mut m = Machine::load_elf(&image, yantra::ram_for(&image))
        .unwrap_or_else(|e| panic!("yantra must LOAD the corpus image: {e:?}"));

    // **AND RUNS** — the clause `मण्डलप्रतिबिम्बम्` could not reach, because it
    // linked the module alone and a module has no `यन्त्रारम्भ`, so `e_entry`
    // landed in a routine body and halted `BeyondRam`. The startup object is
    // linked FIRST here, so `e_entry` is the stub's first instruction. No entry
    // routine is named, and the stub's own margin says `०` gives an image
    // "whose stub halts success" — so a clean finisher halt IS the RUN clause.
    let mut out = Vec::new();
    let halt = m.run(BUDGET, &mut out);
    println!("METRIC t1_driver_corpus_halt {halt:?}");
    assert!(
        matches!(halt, Halt::Finisher { .. }),
        "the corpus image must RUN to a finisher halt, not {halt:?} — \
         BeyondRam here means e_entry is not the startup stub"
    );
}

/// **THE STARTUP EMITTER, ALONE, IN A FRESH IMAGE.** A probe, because
/// `मण्डलानिप्रतिबिम्बम्` has two early returns before its counter is set and
/// they are indistinguishable from outside: an empty startup text and a startup
/// object that would not build both leave `संयोजितवस्तुसंख्या` at 0.
///
/// Bisecting by planting the question rather than reasoning about it — five
/// readings of the emitter could each be charitable; one call cannot.
#[test]
#[ignore = "BLOCKED at encode.t1:5820 — claim UNVERIFIED as of 2026-09-16. The \
           sibling test carrying this same reason was run and PASSED, and the site \
           it names now handles R_RISCV_PCREL_LO12_I. This one has NOT been re-run; \
           the reason is kept only because nobody has measured it"]
fn the_startup_object_emits_from_a_fresh_image() {
    let texts: Vec<(String, String)> = CHAIN
        .iter()
        .map(|(nm, t)| ((*nm).to_string(), (*t).to_string()))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(nm, t)| (nm.as_str(), t.as_str()))
        .collect();
    let mut it = Interpreter::load(&refs, &spec_root()).expect("CHAIN loads");

    let text = call(
        &mut it,
        "यन्त्रोत्सर्जनॱयन्त्रारम्भमण्डलोत्सर्जनम्",
        vec![Value::Bool(false)],
        4_000_000_000,
    )
    .octets()
    .map(|o| o.as_slice().to_vec())
    .unwrap_or_default();
    println!("METRIC t1_startup_text_octets {}", text.len());
    println!(
        "  first 120: {}",
        String::from_utf8_lossy(&text[..text.len().min(120)])
    );
    assert!(
        !text.is_empty(),
        "the startup emitter answered nothing in a fresh image — \
         यन्त्रप्रवेशसंज्ञा {:?}",
        it.global("यन्त्रोत्सर्जनॱयन्त्रप्रवेशसंज्ञा")
    );

    // ONE STAGE FURTHER: the text is fine, so the question is whether the
    // ASSEMBLER can read it back. `पाठवस्तुरचना` refuses at three points and
    // answers शून्यम् at all three, so the globals are the only way to tell
    // which — the diagnostic lives in the module that refused, by design.
    let _ = call(&mut it, "शृङ्खलाॱसङ्कलनारम्भः", vec![], 4_000_000_000);
    let obj = call(
        &mut it,
        "शृङ्खलाॱपाठवस्तुरचना",
        vec![octets(&text)],
        40_000_000_000,
    );
    println!(
        "METRIC t1_startup_object_is_nil {}",
        obj.as_int().is_none() && !matches!(obj, Value::Record(_))
    );
    println!(
        "  वाक्यानि(sentences)  {:?}",
        it.global("वाक्यविभागॱवाक्यसूचकाङ्क")
    );
    println!("  आज्ञासूचकाङ्क(insts) {:?}", it.global("वाक्यविभागआज्ञासूचकाङ्क"));
    println!("  अन्तिमसङ्केतनदोषः    {:?}", it.global("अन्तिमसङ्केतनदोषः"));
    println!("  अन्तिमवस्तुदोषः      {:?}", it.global("अन्तिमवस्तुदोषः"));
    assert!(
        matches!(obj, Value::Record(_)),
        "the startup TEXT assembles to nothing. It is 1115 octets of valid \
         Sassembly the emitter just wrote, so a refusal here is the assembler \
         declining its own emitter's output — which is the join W-279 asserts."
    );
}

/// **WHICH STAGE OF THE FRONT HALF IS THE WALL?** `मण्डलानिप्रतिबिम्बम्` linked
/// the startup and nothing else, and `मण्डलसङ्कलनम्` answers ० when ANY stage
/// refuses — so the whole-call empty answer names nothing. Each stage answers
/// its own count here, and the first zero is the wall.
///
/// The driver's existing single-source test feeds a four-line toy. `lex.t1` is
/// 51,001 octets. The census reaches `run` for 17 sources under RUST
/// orchestration — `paradigm_encode`'s own `chain()` — while this is the `.t1`
/// driver's sequencing of the same stages, which has only ever seen toys. Two
/// callers of one set of modules, and only one of them exercised.
#[test]
#[ignore = "probe: bisecting why the corpus walk links only the startup"]
fn the_front_half_compiles_one_real_corpus_source() {
    let texts: Vec<(String, String)> = CHAIN
        .iter()
        .map(|(nm, t)| ((*nm).to_string(), (*t).to_string()))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(nm, t)| (nm.as_str(), t.as_str()))
        .collect();
    let mut it = Interpreter::load(&refs, &spec_root()).expect("CHAIN loads");

    let src = source("lex.t1");
    println!("METRIC t1_probe_source_octets {}", src.len());

    let decls = call(
        &mut it,
        "शृङ्खलाॱपठनम्",
        vec![octets(src.as_bytes())],
        200_000_000_000,
    )
    .as_int()
    .unwrap_or(-1);
    println!("METRIC t1_probe_declarations {decls}");
    assert!(decls > 0, "पठनम् (lex+parse) answered {decls} for lex.t1");

    let typed = call(
        &mut it,
        "शृङ्खलाॱनिर्णयः",
        vec![Value::Int(decls)],
        200_000_000_000,
    );
    println!("METRIC t1_probe_typechecked {typed:?}");
    assert!(
        matches!(typed, Value::Bool(true)),
        "निर्णयः (resolve+typecheck) answered {typed:?} for lex.t1"
    );

    let routines = call(
        &mut it,
        "शृङ्खलाॱरचना",
        vec![Value::Int(decls)],
        200_000_000_000,
    )
    .as_int()
    .unwrap_or(-1);
    println!("METRIC t1_probe_routines {routines}");
    assert!(routines > 0, "रचना (IR) answered {routines} for lex.t1");

    // The three above are lex, parse, resolve, typecheck and IR — all green on
    // a real source. `मण्डलसङ्कलनम्` calls two more after them, and one of
    // those is the wall.
    let named = call(
        &mut it,
        "शृङ्खलाॱनामसञ्चयः",
        vec![octets("पदविभाग".as_bytes())],
        200_000_000_000,
    )
    .as_int()
    .unwrap_or(-1);
    println!("METRIC t1_probe_named {named}");

    let emitted = call(&mut it, "शृङ्खलाॱउत्सर्जनम्", vec![], 200_000_000_000)
        .octets()
        .map(|o| o.as_slice().len())
        .unwrap_or(0);
    println!("METRIC t1_probe_emitted_octets {emitted}");

    assert!(
        named > 0,
        "नामसञ्चयः (the emitter's name table) answered {named} for lex.t1 — \
         the first four stages were green, so this is the wall"
    );
    // THE EMITTER RECORDS ITS OWN REFUSAL. `यन्त्रमण्डलोत्सर्जनम्` returns an
    // empty run from four different arms, so the empty answer names none of
    // them; `यन्त्रनिषेधभेद` says which. ४ is यन्त्रानामसंज्ञानिषेधभेद —
    // UnnamedSymbol, the same family the t1_build sweep found.
    // BARE NAMES, NOT QUALIFIED. `Interpreter::globals` is keyed by the bare
    // name; a qualified `मॉड्यूलॱनाम` key does not resolve and answers None —
    // which reads exactly like "the value is 0" and is not. Three reads in this
    // session returned None from a guessed qualification before this was fixed.
    for g in [
        "यन्त्रनिषेधमस्ति",
        "यन्त्रनिषेधभेद",
        "यन्त्रनिषेधसंख्या",
        "यन्त्रनामसूचकाङ्क",
        "वृत्तिसूचकाङ्क",
        "यन्त्रनिषेधचिह्न",
        "यन्त्रनिषेधवृत्ति",
        "यन्त्रनिषेधलक्ष्य",
    ] {
        println!("  {g}: {:?}", it.global(g));
    }
    // WHICH SymbolId DOES EACH ROUTINE CARRY? Every routine resolved to
    // declaration १ and got the label `पदविभागपदभेद`. `संज्ञाघोषणाकोश` is keyed
    // by `symbol + १` BY DESIGN — shrinkhala.t1:105 records that guarding
    // against symbol ० rejected the only routine of a one-routine module — so
    // the arithmetic is not the suspect. Two candidates remain and the symbols
    // themselves split them:
    //   all ०        → the routine records carry no distinct SymbolIds
    //   distinct     → the arena maps them all to १, and शृङ्खला reads honestly
    if let Some(Value::Arena(a)) = it.global("वृत्तिकोश") {
        let syms: Vec<String> = a
            .borrow()
            .iter()
            .skip(1)
            .map(|v| match v {
                Value::Record(r) => format!("{:?}", r.borrow().get("नाम").and_then(Value::as_int)),
                other => format!("{other:?}"),
            })
            .collect();
        println!("  routine symbols ({}): {}", syms.len(), syms.join(" "));
    } else {
        println!("  वृत्तिकोश is not an arena: {:?}", it.global("वृत्तिकोश"));
    }

    assert!(
        emitted > 0,
        "उत्सर्जनम् answered nothing for lex.t1 with {named} names — \
         the wall is the emitter, not the name table"
    );
}
