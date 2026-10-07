//! `W-302` — three defects the census of the `.t1` chain over the 48
//! `spec/*.sas` programs found in the assembler half, each graded against
//! what Rust `sadhana` does with the SAME text, in process.
//!
//! 1. **An instruction with no operand crashed the encoder.** `आज्ञापनम् ।`
//!    alone faulted with a Nil read in `सङ्केतनॱकोष्ठसंख्यानम्` via
//!    `सङ्केतयोग्यम्`. An encoding row with `(none)` in its slot field is an
//!    arena that was never written, and an arena is BORN holding one
//!    `शून्यम्`, so a walk bounded by `ॱ दैर्घ्य` read that `शून्यम्` as a slot.
//!    `अवकाशसंख्या` already answered the true count; five sites did not ask
//!    it — four walks and `सङ्कोचनिषेधः`'s comparison (`encode.t1`).
//! 2. **`e_flags` was hard-coded ० in `kosha.t1`.** Rust writes
//!    `abi_flags(text_has_compressed(text))` — `spec/elf-abi-riscv64.tsv`'s
//!    `norvc` or `rvc` row, chosen by whether the text holds a 16-bit
//!    instruction.
//! 3. **A data directive in `ॱपाठ` was dropped without a word.** The splitter
//!    recorded a diagnostic — with an EMPTY code, P28 never having been built
//!    — and `पाठवस्तुरचना` built the object anyway, minus the directive's
//!    octets. Rust refuses the file with P28.
//!
//! Every expectation is read from `spec/` or from Rust's own output by this
//! file; nothing about the encodings is transcribed here.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

const LOAD: u64 = 0x8000_0000;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana-t1 has a grandparent")
        .to_path_buf()
}

fn spec(name: &str) -> String {
    let p = repo_root().join("spec").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn load_chain() -> Interpreter {
    Interpreter::load(CHAIN, &repo_root().join("spec")).expect("the chain loads")
}

fn int_global(it: &Interpreter, name: &str) -> i128 {
    match it.global(name) {
        Some(Value::Int(n)) => *n,
        other => panic!("`{name}` is an int, not {other:?}"),
    }
}

/// What `पाठवस्तुरचना` answered: its exit, and the object's `.text` if it built.
struct Built {
    exit: i128,
    text: Option<Vec<u8>>,
    object: Value,
}

/// Reset, then build an object from Sassembly text — the driver's own order.
fn build(it: &mut Interpreter, src: &str) -> Built {
    it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], 8_000_000_000)
        .expect("सङ्कलनारम्भः runs");
    let object = it
        .call(
            "शृङ्खलाॱपाठवस्तुरचना",
            vec![octets(src.as_bytes())],
            40_000_000_000,
        )
        .unwrap_or_else(|e| panic!("पाठवस्तुरचना runs on {src:?}: {e:?}"));
    let exit = int_global(it, "वस्तुरचनाविरामभेद");
    let text = match &object {
        Value::Nil => None,
        Value::Record(r) => Some(
            r.borrow()
                .get("पाठ्यम्")
                .and_then(|v| v.octets().map(|o| o.as_slice().to_vec()))
                .expect("a वस्तु carries `पाठ्यम्`"),
        ),
        other => panic!("`पाठवस्तुरचना` answers a वस्तु or शून्यम्, not {other:?}"),
    };
    Built { exit, text, object }
}

/// The `.t1` chain's image of one object, linked and written at `LOAD`.
fn image(it: &mut Interpreter, object: Value) -> Vec<u8> {
    for g in ["भारणस्थानम्", "भारस्थानम्"] {
        assert!(
            it.set_global(g, Value::Int(i128::from(LOAD))),
            "`{g}` is a global of the chain"
        );
    }
    it.call("शृङ्खलाॱवस्तुप्रतिबिम्बम्", vec![object], 40_000_000_000)
        .expect("वस्तुप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default()
}

fn rust_image(src: &str) -> Result<Vec<u8>, Vec<String>> {
    sadhana::assemble(
        src,
        sadhana::encode::Target::Uncompressed,
        LOAD,
        sadhana::nidana::Language::Sanskrit,
    )
    .map_err(|ds| ds.into_iter().map(|d| d.reason).collect())
}

fn u16_at(b: &[u8], at: usize) -> u64 {
    u64::from(u16::from_le_bytes([b[at], b[at + 1]]))
}
fn u32_at(b: &[u8], at: usize) -> u64 {
    u64::from(u32::from_le_bytes(
        b[at..at + 4].try_into().expect("four octets"),
    ))
}
fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().expect("eight octets"))
}

/// The first `PT_LOAD`'s file bytes.
fn first_load(img: &[u8]) -> &[u8] {
    let phoff = usize::try_from(u64_at(img, 32)).expect("phoff fits");
    let phnum = u16_at(img, 56);
    for i in 0..usize::try_from(phnum).expect("phnum fits") {
        let ph = phoff + 56 * i;
        if u32_at(img, ph) == 1 {
            let off = usize::try_from(u64_at(img, ph + 8)).expect("offset fits");
            let size = usize::try_from(u64_at(img, ph + 32)).expect("size fits");
            return &img[off..off + size];
        }
    }
    panic!("the image has no PT_LOAD");
}

fn e_flags(img: &[u8]) -> u64 {
    u32_at(img, 48)
}

/// `spec/elf-abi-riscv64.tsv`'s flags for one target, read from the file.
fn abi_row(target: &str) -> u64 {
    spec("elf-abi-riscv64.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once('\t'))
        .find(|(t, _)| *t == target)
        .and_then(|(_, v)| u64::from_str_radix(v.trim().trim_start_matches("0x"), 16).ok())
        .unwrap_or_else(|| panic!("spec/elf-abi-riscv64.tsv has a `{target}` row"))
}

/// Every 32-bit encoding row whose slot field is `(none)`: (Sanskrit name, pattern).
fn operandless_rows() -> Vec<(String, u32)> {
    operandless_rows_of_any_width()
        .into_iter()
        .filter(|r| r.width == 32)
        .map(|r| (r.sanskrit, r.pattern))
        .collect()
}

/// One encoding row whose slot field is `(none)`, of either width.
struct OperandlessRow {
    mnemonic: String,
    sanskrit: String,
    pattern: u32,
    width: u64,
}

/// Every encoding row whose slot field is `(none)` — the 32-bit rows AND the
/// 16-bit ones (`c.ebreak`, `c.nop` in the table today), read from the table.
fn operandless_rows_of_any_width() -> Vec<OperandlessRow> {
    spec("encodings-riscv64.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .filter(|f| f.len() > 6 && f[6] == "(none)")
        .map(|f| OperandlessRow {
            mnemonic: f[0].to_string(),
            sanskrit: f[2].to_string(),
            pattern: u32::from_str_radix(f[3].trim_start_matches("0x"), 16)
                .expect("the pattern is hexadecimal"),
            width: f[5].parse().expect("the width is a number"),
        })
        .collect()
}

/// **DEFECT 1.** Each operand-less instruction, alone in a file, encodes to the
/// four octets Rust emits — and to the row's own pattern, which is the third
/// witness that neither side is agreeing with a mistake of the other.
#[test]
fn an_instruction_with_no_operand_encodes_as_rust_encodes_it() {
    let rows = operandless_rows();
    let names: Vec<&str> = rows.iter().map(|(n, _)| n.as_str()).collect();
    println!("METRIC w302_operandless_rows {names:?}");
    assert!(
        rows.len() >= 4,
        "the table should hold at least ecall, ebreak, fence.i and sret as \
         32-bit rows with no operand; found {names:?}"
    );

    let mut it = load_chain();
    for (name, pattern) in &rows {
        let src = format!("{name} ।\n");
        let rust = rust_image(&src).unwrap_or_else(|e| panic!("Rust assembles {src:?}: {e:?}"));
        let rust_text = &first_load(&rust)[..4];
        assert_eq!(
            rust_text,
            pattern.to_le_bytes(),
            "Rust's `{name}` is not the table's pattern — the oracle is then wrong"
        );

        let b = build(&mut it, &src);
        assert_eq!(
            (b.exit, b.text.as_deref()),
            (0, Some(rust_text)),
            "`{name}` alone must build (exit ०) to Rust's four octets"
        );
    }

    // AND AMONG OTHERS — a walk that reads a शून्यम् as a slot fails on the
    // first row of its kind, so the operand-less word sits between two that
    // have operands, in a file Rust also accepts.
    let (ecall, _) = rows
        .iter()
        .find(|(_, p)| *p == 0x73)
        .expect("ecall's row, pattern 0x00000073");
    let src = format!("योगः स्थिर०म् शून्यःन ७न ।\n{ecall} ।\nयोगः स्थिर०म् शून्यःन ७न ।\n");
    let rust = rust_image(&src).expect("Rust assembles the three lines");
    let b = build(&mut it, &src);
    assert_eq!(
        b.text.as_deref(),
        Some(first_load(&rust)),
        "the three-line text must encode exactly as Rust's"
    );
}

/// **DEFECT 1, THE 16-BIT ROWS.** `(none)` is a slot field of two 16-bit rows
/// as well (`c.ebreak`, `c.nop`), and the same five sites walk them: the
/// compressor asks a 16-bit row's slots. The chain lays out uncompressed, so
/// no source reaches them through `पाठवस्तुरचना`; each row is fetched from the
/// encoder's own index by name and width, and every walk is asked. Each must
/// answer the TRUE slot count, ० — which is also each walk's not-found answer —
/// where a walk bounded by `ॱ दैर्घ्य` reads the arena's birth `शून्यम्` as a
/// slot. Rust's compressed target is the oracle that the 16-bit rows are real
/// encodings it emits.
#[test]
fn every_operandless_row_of_either_width_has_no_slot_on_every_walk() {
    let rows = operandless_rows_of_any_width();
    let names: Vec<(&str, u64)> = rows
        .iter()
        .map(|r| (r.mnemonic.as_str(), r.width))
        .collect();
    println!("METRIC w302_operandless_rows_any_width {names:?}");
    let narrow: Vec<&OperandlessRow> = rows.iter().filter(|r| r.width == 16).collect();
    assert!(
        narrow.len() >= 2,
        "the table should hold c.ebreak and c.nop as 16-bit rows with no operand; found {names:?}"
    );

    // THE ORACLE for one narrow row: Rust's compressed target emits c.ebreak's
    // two octets for the operand-less ebreak — the 16-bit row is live.
    let c_ebreak = narrow
        .iter()
        .find(|r| r.mnemonic == "c.ebreak")
        .expect("the c.ebreak row");
    let src = format!("{} ।\n", c_ebreak.sanskrit);
    let rust = sadhana::assemble(
        &src,
        sadhana::encode::Target::Compressed,
        LOAD,
        sadhana::nidana::Language::Sanskrit,
    )
    .unwrap_or_else(|ds| {
        panic!(
            "Rust assembles {src:?} compressed: {:?}",
            ds.iter().map(|d| &d.reason).collect::<Vec<_>>()
        )
    });
    let pattern16 = u16::try_from(c_ebreak.pattern).expect("a 16-bit pattern");
    assert_eq!(
        first_load(&rust),
        pattern16.to_le_bytes(),
        "Rust's compressed `{}` is c.ebreak's pattern",
        c_ebreak.sanskrit
    );

    let mut it = load_chain();
    // An arena as T1 makes one — EMPTY since `W-355` (it was born holding one
    // शून्यम् before). A slotless row's walk never reads it either way.
    let empty = Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vec![])));
    for r in &rows {
        let row = it
            .call(
                "सङ्केतनॱअभिधानसङ्केतः",
                vec![
                    octets(r.mnemonic.as_bytes()),
                    Value::Int(i128::from(r.width)),
                ],
                4_000_000_000,
            )
            .unwrap_or_else(|e| {
                panic!("the index finds {} at {} bits: {e:?}", r.mnemonic, r.width)
            });
        assert!(
            matches!(row, Value::Record(_)),
            "{} at {} bits is a row of the encoder's index, not {row:?}",
            r.mnemonic,
            r.width
        );
        let walks: [(&str, Vec<Value>); 5] = [
            ("सङ्केतनॱअवकाशसंख्या", vec![row.clone()]),
            ("सङ्केतनॱकोष्ठसंख्यानम्", vec![row.clone()]),
            ("सङ्केतनॱआवरणावकाशः", vec![row.clone(), Value::Int(0x7f)]),
            ("सङ्केतनॱनियततत्कालावकाशः", vec![row.clone(), Value::Int(0)]),
            ("सङ्केतनॱअग्रिमतत्कालावकाशः", vec![row.clone(), empty.clone()]),
        ];
        for (walk, args) in walks {
            let got = it.call(walk, args, 4_000_000_000).unwrap_or_else(|e| {
                panic!("{walk} on {} ({} bits) runs: {e:?}", r.mnemonic, r.width)
            });
            println!(
                "METRIC w302_walk {} {} {walk} = {got:?}",
                r.mnemonic, r.width
            );
            assert_eq!(
                got.as_int(),
                Some(0),
                "{walk} on {} ({} bits) must answer ०, the true slot count",
                r.mnemonic,
                r.width
            );
        }
    }
}

/// **DEFECT 2.** The `.t1` image's `e_flags` is what Rust writes for the same
/// object — `norvc` from the table for an uncompressed text.
#[test]
fn the_image_carries_the_flags_rust_writes() {
    let src = "योगः स्थिर०म् शून्यःन ७न ।\nयोगः स्थिर०म् शून्यःन ७न ।\n";
    let rust = rust_image(src).expect("Rust assembles the two lines");
    let mut it = load_chain();
    let b = build(&mut it, src);
    assert_eq!(b.exit, 0, "the control must build");
    let t1 = image(&mut it, b.object);
    assert!(!t1.is_empty(), "the chain must write an image");
    println!(
        "METRIC w302_e_flags rust={:#x} t1={:#x} norvc={:#x}",
        e_flags(&rust),
        e_flags(&t1),
        abi_row("norvc")
    );
    assert_eq!(
        e_flags(&rust),
        abi_row("norvc"),
        "Rust's flags are the table's norvc"
    );
    assert_eq!(
        e_flags(&t1),
        e_flags(&rust),
        "e_flags must be what Rust writes"
    );
}

/// **DEFECT 2, THE OTHER ARM.** The writer DERIVES the flags from the text, as
/// `kosha.rs`'s `text_has_compressed` does: a text holding a 16-bit instruction
/// is `rvc`. The chain lays out uncompressed, so this asks the writer itself and
/// Rust's writer with the same octets.
#[test]
fn the_writer_derives_the_flags_from_the_text() {
    let mut it = load_chain();
    // `c.nop` (0x0001, low bits 01) then a 32-bit `addi` — and the reverse
    // order, so the walk is graded on a 16-bit word in either place.
    let wide = 0x0070_0293u32.to_le_bytes();
    // Each case names the table row Rust must answer, so a Rust that wrote
    // `norvc` everywhere could not carry the t1 side along with it.
    let cases: [(&str, Vec<u8>, &str); 4] = [
        ("empty", Vec::new(), "norvc"),
        ("wide", [wide, wide].concat(), "norvc"),
        (
            "narrow-first",
            [&[0x01, 0x00][..], &wide[..], &[0x01, 0x00][..]].concat(),
            "rvc",
        ),
        (
            "narrow-last",
            [&wide[..], &[0x01, 0x00][..]].concat(),
            "rvc",
        ),
    ];
    for (label, text, row) in &cases {
        let rust = sadhana::kosha::write_debuggable_at(text, &[], &[], 0, &[], LOAD);
        let t1 = it
            .call(
                "कोशॱप्रतिबिम्बलेखनम्",
                vec![
                    octets(text),
                    octets(&[]),
                    Value::Int(0),
                    Value::Int(i128::from(LOAD)),
                ],
                4_000_000_000,
            )
            .expect("प्रतिबिम्बलेखनम् runs")
            .octets()
            .map(|o| o.as_slice().to_vec())
            .unwrap_or_default();
        println!(
            "METRIC w302_writer_flags {label} rust={:#x} t1={:#x}",
            e_flags(&rust),
            e_flags(&t1)
        );
        assert_eq!(
            e_flags(&rust),
            abi_row(row),
            "{label}: Rust's flags are the table's {row}"
        );
        assert!(!t1.is_empty(), "{label}: the writer must write an image");
        assert_eq!(
            e_flags(&t1),
            e_flags(&rust),
            "{label}: e_flags must be Rust's"
        );
    }
    assert_ne!(
        abi_row("norvc"),
        abi_row("rvc"),
        "the two rows must differ or the cases above separate nothing"
    );
}

/// **DEFECT 3.** A data directive in `ॱपाठ` is REFUSED, by P28, and nothing is
/// built — alone, and between two instructions, which is the case that used to
/// build an object with the directive's octets missing.
#[test]
fn a_data_directive_in_text_is_refused_by_p28() {
    let p28 = spec("diagnostics.tsv")
        .lines()
        .find(|l| l.starts_with("P28\t"))
        .map(|l| {
            l.split('\t')
                .nth(2)
                .expect("a Sanskrit message")
                .to_string()
        })
        .expect("spec/diagnostics.tsv has a P28 row");
    let tail = p28
        .split("{0}")
        .nth(1)
        .expect("the message has a {0}")
        .to_string();

    let alone = "॥ चतुरष्टकाः ११५ ॥\n";
    let between = "योगः स्थिर०म् शून्यःन ७न ।\n॥ चतुरष्टकाः ११५ ॥\nयोगः स्थिर०म् शून्यःन ७न ।\n";
    // THE CONTROL: the same directive in ॱदत्त is accepted by both sides.
    let control = "योगः स्थिर०म् शून्यःन ७न ।\n॥ कोष्ठकम् ॱदत्त ॥\n॥ चतुरष्टकाः ११५ ॥\n";

    let mut it = load_chain();
    let ok = build(&mut it, control);
    assert!(rust_image(control).is_ok(), "Rust accepts the control");
    assert_eq!(
        ok.exit, 0,
        "the control must build — else the refusal is the directive's shape"
    );

    for (label, src) in [("alone", alone), ("between", between)] {
        let rust = rust_image(src).expect_err("Rust refuses a data directive in ॱपाठ");
        assert!(
            rust.iter().any(|r| r.ends_with(&tail)),
            "{label}: Rust's refusal is P28's message, got {rust:?}"
        );

        let b = build(&mut it, src);
        let count = int_global(&it, "वाक्यविभागदोषसूचकाङ्क");
        let codes: Vec<String> = match it.global("वाक्यविभागदोषकोश")
        {
            Some(Value::Arena(a)) => {
                let a = a.borrow();
                (1..=usize::try_from(count).unwrap_or(0))
                    .filter_map(|i| a.get(i))
                    .map(|d| match d {
                        Value::Record(r) => r
                            .borrow()
                            .get("कूट")
                            .and_then(|v| {
                                v.octets()
                                    .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
                            })
                            .unwrap_or_default(),
                        other => format!("{other:?}"),
                    })
                    .collect()
            }
            other => panic!("`वाक्यविभागदोषकोश` is an arena, not {other:?}"),
        };
        println!(
            "METRIC w302_p28 {label} exit={} built={} codes={codes:?}",
            b.exit,
            b.text.is_some()
        );
        assert_eq!(
            codes,
            vec!["P28".to_string()],
            "{label}: the splitter's record names P28"
        );
        assert!(b.text.is_none(), "{label}: nothing may be built");
        assert_eq!(
            b.exit,
            int_global(&it, "वस्तुरचनावाक्यदोषभेद"),
            "{label}: the exit says the splitter refused a statement"
        );
    }
}

/// The codes the splitter recorded for the last build, in order.
fn splitter_codes(it: &Interpreter) -> Vec<String> {
    let count = int_global(it, "वाक्यविभागदोषसूचकाङ्क");
    match it.global("वाक्यविभागदोषकोश") {
        Some(Value::Arena(a)) => {
            let a = a.borrow();
            (1..=usize::try_from(count).unwrap_or(0))
                .filter_map(|i| a.get(i))
                .map(|d| match d {
                    Value::Record(r) => r
                        .borrow()
                        .get("कूट")
                        .and_then(|v| {
                            v.octets()
                                .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
                        })
                        .unwrap_or_default(),
                    other => format!("{other:?}"),
                })
                .collect()
        }
        other => panic!("`वाक्यविभागदोषकोश` is an arena, not {other:?}"),
    }
}

/// The Sanskrit message of one diagnostic, after its LAST placeholder — the
/// part every instance of the refusal ends with.
fn message_tail(code: &str) -> String {
    let row = spec("diagnostics.tsv")
        .lines()
        .find(|l| l.starts_with(&format!("{code}\t")))
        .map(|l| {
            l.split('\t')
                .nth(2)
                .expect("a Sanskrit message")
                .to_string()
        })
        .unwrap_or_else(|| panic!("spec/diagnostics.tsv has a {code} row"));
    let last = row.rfind('}').expect("the message has a placeholder");
    row[last + 1..].to_string()
}

/// **CODES BY NAME.** Three refusals of a data directive's operand or place —
/// P30 (a count written with ऋण), P19 (a numeral that is not one), P20 (a value
/// in ॱरिक्त) — were recorded with an EMPTY code: the globals they pass were
/// declared and never built, so since 3e2952ed made a recorded refusal refuse
/// the file, these refused WITHOUT A NAME. Each case is graded against Rust's
/// own refusal of the same text (its message's tail, from the table), and the
/// chain's record must be exactly the code.
#[test]
fn p30_p19_p20_are_recorded_by_name() {
    let cases = [
        ("P30", "॥ कोष्ठकम् ॱदत्त ॥\n॥ स्थानम् ऋण५ ॥\n"),
        ("P19", "॥ कोष्ठकम् ॱदत्त ॥\n॥ चतुरष्टकाः १२क ॥\n"),
        ("P20", "॥ कोष्ठकम् ॱरिक्त ॥\n॥ चतुरष्टकाः ११५ ॥\n"),
    ];
    let mut it = load_chain();
    // Every case is run before any assertion, so a red names all three.
    let mut wrong = Vec::new();
    for (code, src) in cases {
        let tail = message_tail(code);
        let rust = rust_image(src).expect_err("Rust refuses the case");
        assert!(
            rust.iter().any(|r| r.ends_with(&tail)),
            "{code}: Rust's refusal is {code}'s message (…{tail}), got {rust:?}"
        );
        let b = build(&mut it, src);
        let codes = splitter_codes(&it);
        let refused = int_global(&it, "वस्तुरचनावाक्यदोषभेद");
        println!(
            "METRIC w302_code {code} exit={} built={} codes={codes:?}",
            b.exit,
            b.text.is_some()
        );
        if codes != vec![code.to_string()] || b.text.is_some() || b.exit != refused {
            wrong.push(format!(
                "{code}: codes {codes:?}, built {}, exit {} (want [{code:?}], nothing built, exit {refused})",
                b.text.is_some(),
                b.exit
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "each refusal must be recorded by its code, nothing built:\n{}",
        wrong.join("\n")
    );
}
