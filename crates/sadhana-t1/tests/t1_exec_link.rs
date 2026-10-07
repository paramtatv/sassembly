//! **THE LINKER'S `.t1` BODY IS EXECUTED HERE** — task `D-002b`, the three
//! stubs at the foot of `crates/sadhana-t1/src/samyojana.t1`.
//!
//! # Why a file of its own
//!
//! `tests/t1_execution.rs` is the crate's other executing test file and it is
//! not touched: five agents editing one test file in one round made the merge
//! cost more than the code. Every helper this file needs is copied into it, and
//! the duplication is the point — two files that share nothing cannot conflict.
//!
//! # What is asserted
//!
//! Not "a routine returns something". Two objects LINK INTO ONE IMAGE with
//! their symbols resolved: the text is the two texts end to end, `मुख्यम्` and
//! `सहायः` get addresses derived from the load address and the first object's
//! length, and a `jal` whose target is in the *other* object is rewritten so
//! that its displacement reaches. That last one is the whole of separate
//! compilation, and it is checked by decoding the patched word back with
//! `विश्लेषण ॱ विश्लेषणम्` — a second, independent reader of the same derived
//! table — rather than against a displacement computed a second time here.
//!
//! Every assertion is followed by a mutation that breaks one comparison in the
//! routine it exercises and requires the answer to change. The mutations go
//! through [`mutate`], which fails if the text it is given is not in the source
//! **exactly once**: a mutation that silently matched nothing would be the same
//! lie one layer up.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

// ─────────────────────────────────────────────────────────────────────────
// Loading
// ─────────────────────────────────────────────────────────────────────────

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// Replace `from` with `to` in `text`, and **fail if `from` is not there
/// exactly once.**
fn mutate(text: &str, from: &str, to: &str) -> String {
    let n = text.matches(from).count();
    assert_eq!(
        n, 1,
        "the mutation `{from}` -> `{to}` matches {n} places in the source; \
         a mutation test is only evidence when it changes exactly one"
    );
    text.replace(from, to)
}

/// The six modules a link needs: the linker, the object and symbol models,
/// the encoding table reader, the disassembler `संस्कारः` decodes with — and
/// the LEXER, since 2026-09-14, because `संयोजनॱभेदपङ्क्तिः` reads the
/// relocation table through `पदविभागॱसमावेशपाठः` and that is declared in
/// `lex.t1`. Sixth file with the same gap; `lex.t1` imports nothing, so adding
/// it closes nothing and cannot cycle.
fn linker_with(samyojana: &str) -> Interpreter {
    Interpreter::load(
        &[
            ("lex.t1", &source("lex.t1")),
            ("samyojana.t1", samyojana),
            ("vastu.t1", &source("vastu.t1")),
            ("kosha.t1", &source("kosha.t1")),
            ("encode.t1", &source("encode.t1")),
            ("vishlesana.t1", &source("vishlesana.t1")),
        ],
        &spec_root(),
    )
    .expect("the five T1 sources load")
}

fn linker() -> Interpreter {
    linker_with(&source("samyojana.t1"))
}

// ─────────────────────────────────────────────────────────────────────────
// Reading values back
// ─────────────────────────────────────────────────────────────────────────

fn member(v: &Value, name: &str) -> Value {
    match v {
        Value::Record(r) => r
            .borrow()
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("no member `{name}`")),
        other => panic!("{other:?} is not a record, so it has no `{name}`"),
    }
}

fn as_text(v: &Value) -> String {
    match v {
        Value::Octets(o) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        other => panic!("{other:?} is not a run of octets"),
    }
}

/// A `बूल` global's value (`W-359`'s `निधानविफलम्`).
fn truth(v: Option<&Value>) -> bool {
    match v {
        Some(Value::Bool(b)) => *b,
        Some(v) => v.as_int().is_some_and(|i| i != 0),
        None => false,
    }
}

fn as_bytes(v: &Value) -> Vec<u8> {
    match v {
        Value::Octets(o) => o.as_slice().to_vec(),
        other => panic!("{other:?} is not a run of octets"),
    }
}

fn arena(v: &Value) -> Vec<Value> {
    match v {
        Value::Arena(a) => a.borrow().clone(),
        other => panic!("{other:?} is not an arena"),
    }
}

/// The live entries of an arena. **An arena in this language cannot be empty**
/// — `भवति ०` gives it one slot and nothing shrinks it — so ० entries and १
/// both report `ॱ दैर्घ्य` १, and only entry ० tells them apart.
fn live(v: &Value) -> Vec<Value> {
    let cells = arena(v);
    if cells.first().is_none_or(Value::is_nil) {
        return Vec::new();
    }
    cells.into_iter().filter(|c| !c.is_nil()).collect()
}

fn octets(s: &[u8]) -> Value {
    Value::Octets(Octets::new(s))
}

fn text(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

fn record(fields: &[(&str, Value)]) -> Value {
    let mut m: HashMap<String, Value> = HashMap::new();
    for (k, v) in fields {
        m.insert((*k).to_string(), v.clone());
    }
    Value::Record(Rc::new(RefCell::new(m)))
}

/// A zero-based arena, which is what `संयोजन` and `विश्लेषण` both build and
/// walk — `वाक्यविभाग`'s one-based convention is that file's, not this one's.
/// An EMPTY list is an empty arena, length ०, which is what a fresh `भवति ०`
/// run is since `W-355`; until then it was one `Nil`, the old zero run.
fn list(items: Vec<Value>) -> Value {
    Value::Arena(Rc::new(RefCell::new(items)))
}

// ─────────────────────────────────────────────────────────────────────────
// Building objects, in the shape `वास्तु` declares
// ─────────────────────────────────────────────────────────────────────────

/// `वास्तु ॱ स्थापन`, mirrored as `संयोजन` mirrors it — the constants
/// `पाठनिवेशः` … `अन्यनिवेशः` declare, which are the ENUM'S ORDINALS PLUS ONE.
///
/// **THIS SAID "the enum itself is not a value this interpreter carries" UNTIL
/// 2026-09-07 AND THAT WAS FALSE.** Probed: bare `पाठ्यम्` reads `Int(0)`,
/// `दत्तम्` `Int(1)`, `अन्यत्` `Int(5)`. Two real constraints sit beside it and
/// are why the sentence looked true:
///
/// * **A QUALIFIED variant does NOT resolve** — `वास्तुॱपाठ्यम्` is `None`.
/// * **`W-224`: only a name every declaring module agrees on gets a bare key**,
///   so `अनिर्दिष्टम्` — declared elsewhere too — is `None` as well.
///
/// The falsehood mattered: that sentence is exactly what would stop the next
/// person writing the test that pins these constants to the enum, which
/// `t1_vastu_build.rs` now does. **The numbers below stay as they are** — they
/// are Placement+1 and `संयोजन` expects Placement+1, because a fresh record is
/// `भवति ०` and Text=1 keeps an unfilled record refusable.
const PLACE_TEXT: i128 = 1;
const PLACE_DATA: i128 = 2;
const PLACE_BSS: i128 = 3;
const PLACE_DEBUG_LINE: i128 = 4;
const PLACE_UNDEFINED: i128 = 5;

/// `कोश ॱ संज्ञाखण्ड`, mirrored as `संयोजन` mirrors it.
const SEC_TEXT: i128 = 1;
const SEC_DATA: i128 = 2;
const SEC_BSS: i128 = 3;

fn symbol(name: &str, value: i128, placement: i128, global: bool) -> Value {
    record(&[
        ("नाम", text(name)),
        ("मूल्यम्", Value::Int(value)),
        ("खण्डाङ्कः", Value::Int(0)),
        ("स्थापनम्", Value::Int(placement)),
        ("वैश्विकम्", Value::Bool(global)),
    ])
}

/// The ELF null symbol at index ०, which every object carries and no link
/// defines — `वास्तु`'s own comment at `संज्ञाः`.
fn null_symbol() -> Value {
    symbol("", 0, PLACE_UNDEFINED, false)
}

fn relocation(at: i128, sym: i128, kind: i128, addend: i128) -> Value {
    record(&[
        ("स्थानाङ्कः", Value::Int(at)),
        ("संज्ञाङ्कः", Value::Int(sym)),
        ("भेदः", Value::Int(kind)),
        ("योज्यम्", Value::Int(addend)),
    ])
}

#[derive(Default)]
struct ObjectSpec {
    text: Vec<u8>,
    data: Vec<u8>,
    bss: i128,
    symbols: Vec<Value>,
    relocations: Vec<Value>,
    data_relocations: Vec<Value>,
    debug: Vec<(String, Vec<u8>)>,
    debug_relocations: Vec<(String, Vec<Value>)>,
}

impl ObjectSpec {
    fn build(self) -> Value {
        record(&[
            ("पाठ्यम्", octets(&self.text)),
            ("दत्तम्", octets(&self.data)),
            ("संज्ञाः", list(self.symbols)),
            ("पुनःस्थापनानि", list(self.relocations)),
            ("शून्यक्षेत्रम्", Value::Int(self.bss)),
            (
                "शोधनम्",
                list(
                    self.debug
                        .iter()
                        .map(|(n, b)| record(&[("नाम", text(n)), ("दत्तम्", octets(b))]))
                        .collect(),
                ),
            ),
            (
                "शोधनपुनःस्थापनानि",
                list(
                    self.debug_relocations
                        .iter()
                        .map(|(n, rs)| {
                            record(&[("नाम", text(n)), ("पुनःस्थापनानि", list(rs.clone()))])
                        })
                        .collect(),
                ),
            ),
            ("दत्तपुनःस्थापनानि", list(self.data_relocations)),
        ])
    }
}

// ─────────────────────────────────────────────────────────────────────────
// The relocation numbers, read by THIS test out of the derived table, so the
// T1 reader is checked against the file and not against another copy of itself
// ─────────────────────────────────────────────────────────────────────────

fn relocation_number(name: &str) -> i128 {
    let table = std::fs::read_to_string(spec_root().join("relocations-riscv64.tsv"))
        .expect("spec/relocations-riscv64.tsv exists");
    table
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("name\t"))
        .filter_map(|l| l.split_once('\t'))
        .find(|(n, _)| *n == name)
        .and_then(|(_, v)| v.trim().parse::<i128>().ok())
        .unwrap_or_else(|| panic!("spec/relocations-riscv64.tsv carries no `{name}`"))
}

/// `कोश ॱ LOAD_ADDRESS`.
const LOAD: i128 = 0x8000_0000;

/// A `jal x1, 0` — `0x000000ef`. Its displacement slot is the one a
/// `R_RISCV_JAL` names, and it is zero in an object exactly as `संस्कारः`'s
/// note says: the field is rebuilt from the pattern, never OR-ed into.
const JAL_RA_ZERO: u32 = 0x0000_00ef;

fn le32(w: u32) -> Vec<u8> {
    w.to_le_bytes().to_vec()
}

/// Fuel. `विश्लेषण ॱ विश्लेषणम्` walks the whole 220-row encoding table for
/// each patch, which `t1_execution.rs` measures at ~600M for one decode; a
/// link that patches twice is given room for both and for the table readers
/// `भेदाङ्कः` runs three times before it starts.
const FUEL: u64 = 4_000_000_000;

fn call(it: &mut Interpreter, name: &str, args: Vec<Value>) -> Value {
    it.call(name, args, FUEL)
        .unwrap_or_else(|e| panic!("`{name}` runs: {e}"))
}

/// How many refusals the last link recorded, and their codes.
fn errors(it: &Interpreter) -> Vec<(i128, String)> {
    let n = it
        .global("संयोजनदोषसूचकाङ्क")
        .and_then(Value::as_int)
        .expect("the linker keeps an error cursor");
    let cells = arena(
        it.global("संयोजनदोषकोश")
            .expect("the linker keeps an error store"),
    );
    (1..=n as usize)
        .map(|i| {
            let e = &cells[i];
            (
                member(e, "कूट").as_int().expect("a code is a number"),
                as_text(&member(e, "नाम")),
            )
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────
// Two objects become one image
// ─────────────────────────────────────────────────────────────────────────

/// The first object: eight octets of `.text` with a `jal` at ० that names
/// `सहायः`, which it does not define, plus a global `मुख्यम्` at ०.
fn caller() -> ObjectSpec {
    let mut text = le32(JAL_RA_ZERO);
    text.extend_from_slice(&le32(0x0000_0013)); // nop — `addi x0, x0, 0`
    ObjectSpec {
        text,
        symbols: vec![
            null_symbol(),
            symbol("मुख्यम्", 0, PLACE_TEXT, true),
            symbol("सहायः", 0, PLACE_UNDEFINED, true),
        ],
        relocations: vec![relocation(0, 2, relocation_number("R_RISCV_JAL"), 0)],
        ..ObjectSpec::default()
    }
}

/// The second object: four octets of `.text` defining the global `सहायः`, and
/// eight octets of `.data`.
fn callee() -> ObjectSpec {
    ObjectSpec {
        text: le32(0x0000_8067), // ret — `jalr x0, x1, 0`
        data: vec![1, 2, 3, 4, 5, 6, 7, 8],
        symbols: vec![null_symbol(), symbol("सहायः", 0, PLACE_TEXT, true)],
        ..ObjectSpec::default()
    }
}

fn link(it: &mut Interpreter, objects: Vec<Value>) -> Value {
    call(it, "संयोजनॱसंयोजनम्", vec![list(objects)])
}

#[test]
fn two_objects_link_into_one_image_with_the_cross_object_call_resolved() {
    let mut it = linker();
    let image = link(&mut it, vec![caller().build(), callee().build()]);

    assert!(!image.is_nil(), "the link was refused: {:?}", errors(&it));
    assert_eq!(errors(&it), Vec::new(), "a clean link records no refusal");

    // ── the sections are the inputs, end to end ──
    let text = as_bytes(&member(&image, "पाठ्यम्"));
    assert_eq!(text.len(), 12, "eight octets of .text and then four");
    assert_eq!(
        &text[8..12],
        &le32(0x0000_8067)[..],
        "the second object's text follows the first, unchanged"
    );
    assert_eq!(
        as_bytes(&member(&image, "दत्तम्")),
        vec![1, 2, 3, 4, 5, 6, 7, 8],
        ".data is the objects' .data, end to end"
    );
    assert_eq!(
        member(&image, "बीजम्").as_int(),
        Some(0),
        "neither object reserves .bss"
    );

    // ── every name has an absolute address ──
    let names: HashMap<String, i128> = live(&member(&image, "नामानि"))
        .iter()
        .map(|e| {
            (
                as_text(&member(e, "नाम")),
                member(e, "स्थान").as_int().expect("an address is a number"),
            )
        })
        .collect();
    assert_eq!(
        names.get("मुख्यम्"),
        Some(&LOAD),
        "the first object's text begins at the load address"
    );
    assert_eq!(
        names.get("सहायः"),
        Some(&(LOAD + 8)),
        "the second object's text begins after the first object's eight octets"
    );
    assert_eq!(
        names.len(),
        2,
        "the undefined `सहायः` of the FIRST object is not a definition"
    );

    // ── the image carries a symbol table, in object order (`B-103`) ──
    let table = live(&member(&image, "सारणी"));
    let rows: Vec<(String, i128, i128, bool)> = table
        .iter()
        .map(|r| {
            (
                as_text(&member(r, "नाम")),
                member(r, "मूल्यम्").as_int().expect("a value is a number"),
                member(r, "खण्डः").as_int().expect("a section is a number"),
                matches!(member(r, "वैश्विकम्"), Value::Bool(true)),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            ("मुख्यम्".to_string(), LOAD, SEC_TEXT, true),
            ("सहायः".to_string(), LOAD + 8, SEC_TEXT, true),
        ],
        "an image linked from objects must carry its names or `nm` reports none"
    );

    // ── THE CROSS-OBJECT CALL. The `jal` at ० must now reach `सहायः`, eight
    //    octets on. Decoded back rather than compared to a displacement
    //    recomputed here: `विश्लेषण` is an independent reader of the same
    //    derived table, and a bit map typed twice is what this module exists
    //    not to do.
    let word = u32::from_le_bytes(text[0..4].try_into().expect("four octets"));
    assert_ne!(
        word, JAL_RA_ZERO,
        "the `jal` still holds the object's zero displacement, so nothing was patched"
    );
    let decoded = call(
        &mut it,
        "विश्लेषणॱविश्लेषणम्",
        vec![Value::Int(i128::from(word))],
    );
    assert!(!decoded.is_nil(), "the patched word no longer decodes");
    assert_eq!(
        as_text(&member(&decoded, "आज्ञा")),
        "jal",
        "the patch rebuilt the word as a different instruction"
    );
    let operands: Vec<i128> = live(&member(&decoded, "मूल्यानि"))
        .iter()
        .map(|o| member(o, "मूल्यम्").as_int().expect("an operand is a number"))
        .collect();
    assert!(
        operands.contains(&8),
        "the displacement must be ८ — `सहायः` at {:#x} from a pc of {:#x} — and \
         the operands decoded as {operands:?}",
        LOAD + 8,
        LOAD
    );
}

#[test]
fn the_cross_object_call_moves_with_the_image_and_the_displacement_does_not() {
    // `link_at`, task `C-001a1`. A symbol's value is absolute, so every
    // address moves; the relocations this applies are pc-relative and the base
    // cancels — which is exactly why an image can be moved at all.
    let mut it = linker();
    let at = 0x9000_0000i128;
    let image = call(
        &mut it,
        "संयोजनॱस्थानसंयोजनम्",
        vec![
            list(vec![caller().build(), callee().build()]),
            Value::Int(at),
        ],
    );
    assert!(!image.is_nil(), "the link was refused: {:?}", errors(&it));

    let names: HashMap<String, i128> = live(&member(&image, "नामानि"))
        .iter()
        .map(|e| {
            (
                as_text(&member(e, "नाम")),
                member(e, "स्थान").as_int().expect("an address is a number"),
            )
        })
        .collect();
    assert_eq!(names.get("मुख्यम्"), Some(&at), "the image moved");
    assert_eq!(names.get("सहायः"), Some(&(at + 8)), "and so did the callee");

    // The same image at the default address, for the bytes to be compared to.
    let mut other = linker();
    let default = link(&mut other, vec![caller().build(), callee().build()]);
    assert_eq!(
        as_bytes(&member(&image, "पाठ्यम्")),
        as_bytes(&member(&default, "पाठ्यम्")),
        "the text is byte-identical at two load addresses: every relocation \
         this applies is pc-relative and the base cancels"
    );
}

#[test]
fn a_global_defined_by_two_objects_is_refused_and_a_local_is_not() {
    // `B-014` and `B-103`. Silently taking one makes which file was listed
    // first into program behaviour — but a LOCAL defined twice is ordinary,
    // and refusing it was the defect that made this linker reject programs
    // `बन्धकः` linked without complaint.
    let one = |global: bool| ObjectSpec {
        text: le32(0x0000_0013),
        symbols: vec![null_symbol(), symbol("क", 0, PLACE_TEXT, global)],
        ..ObjectSpec::default()
    };

    let mut it = linker();
    let refused = link(&mut it, vec![one(true).build(), one(true).build()]);
    assert!(
        refused.is_nil(),
        "two definitions of a global must be refused"
    );
    let raised = errors(&it);
    assert_eq!(
        raised.len(),
        1,
        "one collision is one refusal, and it was {raised:?}"
    );
    assert_eq!(
        raised[0].1, "क",
        "the refusal must name the colliding symbol"
    );

    let mut it = linker();
    let linked = link(&mut it, vec![one(false).build(), one(false).build()]);
    assert!(
        !linked.is_nil(),
        "two files may each carry a local `क`: {:?}",
        errors(&it)
    );
    // Lossy by construction, and the field says so: the map holds one, the
    // table holds both.
    assert_eq!(live(&member(&linked, "नामानि")).len(), 1);
    assert_eq!(
        live(&member(&linked, "सारणी")).len(),
        2,
        "`सारणी` is the truth — two objects, two rows"
    );
}

#[test]
fn every_unresolved_name_is_reported_and_not_only_the_first() {
    // Rust's whole reason for `Result<_, Vec<String>>`: one run should report
    // one link's problems.
    let obj = ObjectSpec {
        text: [le32(JAL_RA_ZERO), le32(JAL_RA_ZERO)].concat(),
        symbols: vec![
            null_symbol(),
            symbol("क", 0, PLACE_UNDEFINED, true),
            symbol("ख", 0, PLACE_UNDEFINED, true),
        ],
        relocations: vec![
            relocation(0, 1, relocation_number("R_RISCV_JAL"), 0),
            relocation(4, 2, relocation_number("R_RISCV_JAL"), 0),
        ],
        ..ObjectSpec::default()
    };

    let mut it = linker();
    let refused = link(&mut it, vec![obj.build()]);
    assert!(refused.is_nil(), "neither name is defined by any object");
    let raised = errors(&it);
    assert_eq!(
        raised.iter().map(|(_, n)| n.as_str()).collect::<Vec<_>>(),
        vec!["क", "ख"],
        "the second unresolved name must be reported too, and it was {raised:?}"
    );
}

#[test]
fn a_relocation_kind_this_linker_does_not_apply_is_refused_and_not_approximated() {
    // A relocation applied by the wrong rule produces a program that runs and
    // goes somewhere else. `R_RISCV_HI20` is a real row of the derived table
    // and is NOT one of the three this linker applies — the absolute pair, not
    // the pc-relative one — so it is the honest choice for "unhandled".
    let unknown = relocation_number("R_RISCV_HI20");
    let obj = ObjectSpec {
        text: le32(JAL_RA_ZERO),
        symbols: vec![null_symbol(), symbol("क", 0, PLACE_TEXT, true)],
        relocations: vec![relocation(0, 1, unknown, 0)],
        ..ObjectSpec::default()
    };

    let mut it = linker();
    let refused = link(&mut it, vec![obj.build()]);
    assert!(refused.is_nil(), "an unhandled kind must be refused");
    assert_eq!(
        errors(&it).iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        vec![4],
        "अनुपयुक्तभेदकूटः — `relocation type {unknown} is not applied yet`"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The `auipc`/`addi` pair — `%pcrel_hi` and `%pcrel_lo` (`B-105`)
// ─────────────────────────────────────────────────────────────────────────

/// `auipc x5, 0` and `addi x5, x5, 0`, with a `nop` between them so that the
/// pair is **not adjacent**. That spacing is the whole point: a `%pcrel_lo`
/// names WHERE ITS `%pcrel_hi` IS, and following the record is what replaces
/// reading `pc - 4` on the assembler's promise that the two sit together.
const AUIPC_T0_ZERO: u32 = 0x0000_0297;
const ADDI_T0_T0_ZERO: u32 = 0x0002_8293;

/// The target is `0x1234` into the second object's `.text`, so that the upper
/// half is not zero and the two halves can be told apart at all.
const TARGET_IN_CALLEE: i128 = 0x1234;

fn pair_caller() -> ObjectSpec {
    ObjectSpec {
        text: [
            le32(AUIPC_T0_ZERO),
            le32(0x0000_0013),
            le32(ADDI_T0_T0_ZERO),
        ]
        .concat(),
        symbols: vec![
            null_symbol(),
            symbol("दूरम्", 0, PLACE_UNDEFINED, true),
            // The assembler's own label for the `auipc`: its VALUE is that
            // instruction's offset, which is what the `%pcrel_lo` names.
            symbol("ॱउपरि", 0, PLACE_TEXT, false),
        ],
        relocations: vec![
            relocation(0, 1, relocation_number("R_RISCV_PCREL_HI20"), 0),
            relocation(8, 2, relocation_number("R_RISCV_PCREL_LO12_I"), 0),
        ],
        ..ObjectSpec::default()
    }
}

fn pair_callee() -> ObjectSpec {
    ObjectSpec {
        text: le32(0x0000_8067),
        symbols: vec![
            null_symbol(),
            symbol("दूरम्", TARGET_IN_CALLEE, PLACE_TEXT, true),
        ],
        ..ObjectSpec::default()
    }
}

/// The `(kind, value)` pairs a word decodes to.
fn operands_of(it: &mut Interpreter, word: u32) -> Vec<(i128, i128)> {
    let d = call(it, "विश्लेषणॱविश्लेषणम्", vec![Value::Int(i128::from(word))]);
    assert!(!d.is_nil(), "{word:#010x} does not decode");
    live(&member(&d, "मूल्यानि"))
        .iter()
        .map(|o| {
            (
                member(o, "भेद").as_int().expect("a slot kind is a number"),
                member(o, "मूल्यम्").as_int().expect("an operand is a number"),
            )
        })
        .collect()
}

#[test]
fn the_two_halves_of_an_auipc_addi_pair_are_computed_against_the_auipcs_pc() {
    // The upper twenty are rounded so that adding a SIGNED lower twelve
    // reaches the target: `addi` sign-extends, so a low half above 0x7ff must
    // be borrowed against. The low half is measured from the `auipc`, not from
    // itself — computing it against its own pc would be wrong by four, and
    // here by eight, because the two are three words apart.
    let mut it = linker();
    let image = link(&mut it, vec![pair_caller().build(), pair_callee().build()]);
    assert!(!image.is_nil(), "the link was refused: {:?}", errors(&it));

    let text = as_bytes(&member(&image, "पाठ्यम्"));
    let target = LOAD + 12 + TARGET_IN_CALLEE; // callee's text base + value
    assert_eq!(
        addresses(&image).get("दूरम्"),
        Some(&target),
        "the callee's text follows the caller's twelve octets"
    );

    // What the pair must reach, computed here from the ABI's own rule.
    let whole = target - LOAD; // both halves measured from the auipc's pc
    let hi = (whole + 0x800) >> 12;
    let lo = whole - (hi << 12);
    assert_eq!(hi, 1, "the sample was chosen so the upper half is not zero");
    assert_eq!(
        hi * 0x1000 + lo,
        whole,
        "the two halves must sum to the whole"
    );

    let auipc = u32::from_le_bytes(text[0..4].try_into().expect("four octets"));
    let addi = u32::from_le_bytes(text[8..12].try_into().expect("four octets"));
    assert_ne!(auipc, AUIPC_T0_ZERO, "the upper half was not written");
    assert_ne!(addi, ADDI_T0_T0_ZERO, "the lower half was not written");
    assert_eq!(
        u32::from_le_bytes(text[4..8].try_into().expect("four octets")),
        0x0000_0013,
        "the `nop` between them carries no record and must be untouched"
    );

    let ups = operands_of(&mut it, auipc);
    assert!(
        ups.iter().any(|(_, v)| *v == hi),
        "the auipc must carry {hi}, and decoded as {ups:?}"
    );
    let lows = operands_of(&mut it, addi);
    assert!(
        lows.iter().any(|(_, v)| *v == lo),
        "the addi must carry {lo}, and decoded as {lows:?}"
    );
}

#[test]
fn a_pcrel_lo_that_reads_pc_minus_four_cannot_find_a_hi_that_is_not_adjacent() {
    // `B-105`. Reading `pc - 4` works only while the assembler happens to put
    // the pair together; following the record is the ABI's rule and is what
    // lets GNU `ld` read these objects. With the record ignored, the `nop`
    // between the two is enough to lose the `%pcrel_hi` entirely.
    let (image, raised) = link_mutated(
        "        चरः लक्षितम् ॱॱ अ६४ भवति संज्ञा ॱ मूल्यम् योगः लेखः ॱ योज्यम् ।",
        "        चरः लक्षितम् ॱॱ अ६४ भवति लेखः ॱ स्थानाङ्कः वियोगः ४ ।",
        vec![pair_caller().build(), pair_callee().build()],
    );
    assert!(image.is_nil(), "the pair can no longer be resolved");
    assert_eq!(
        raised.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        vec![11],
        "अनुपरिकूटः — `a %pcrel_lo names an address where no %pcrel_hi is`"
    );
}

/// The same pair, but the `%pcrel_lo`'s label sits at offset ४ and its addend
/// is ऋण४ — so `sym.value + addend` still names the `auipc` at ०.
///
/// This is the shape that separates the two readings of an addend: it belongs
/// to the ADDRESS the record names, not to the value the record writes.
fn pair_caller_with_addend() -> ObjectSpec {
    let mut o = pair_caller();
    o.symbols[2] = symbol("ॱउपरि", 4, PLACE_TEXT, false);
    o.relocations[1] = relocation(8, 2, relocation_number("R_RISCV_PCREL_LO12_I"), -4);
    o
}

#[test]
fn a_pcrel_los_addend_belongs_to_the_address_it_names_and_not_to_what_it_writes() {
    let mut it = linker();
    let plain = link(&mut it, vec![pair_caller().build(), pair_callee().build()]);
    assert!(!plain.is_nil(), "the link was refused: {:?}", errors(&it));

    let mut it = linker();
    let with_addend = link(
        &mut it,
        vec![pair_caller_with_addend().build(), pair_callee().build()],
    );
    assert!(
        !with_addend.is_nil(),
        "`ॱउपरि` + ऋण४ still names the auipc at ०: {:?}",
        errors(&it)
    );
    assert_eq!(
        as_bytes(&member(&with_addend, "पाठ्यम्")),
        as_bytes(&member(&plain, "पाठ्यम्")),
        "the addend moved WHICH %pcrel_hi is named and must not move the \
         twelve bits written; added a second time the low half would be off \
         by four"
    );
}

#[test]
fn dropping_the_addend_from_the_address_a_pcrel_lo_names_loses_its_hi() {
    // The other direction of the same rule: the addend is part of the address,
    // so a reader that drops it looks four octets past the `auipc` and finds
    // no `%pcrel_hi` at all.
    let (image, raised) = link_mutated(
        "        चरः लक्षितम् ॱॱ अ६४ भवति संज्ञा ॱ मूल्यम् योगः लेखः ॱ योज्यम् ।",
        "        चरः लक्षितम् ॱॱ अ६४ भवति संज्ञा ॱ मूल्यम् ।",
        vec![pair_caller_with_addend().build(), pair_callee().build()],
    );
    assert!(
        image.is_nil(),
        "without the addend the pair cannot be resolved"
    );
    assert_eq!(
        raised.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        vec![11],
        "अनुपरिकूटः"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `.rela.data` — an absolute pointer, never a displacement (`B-098`)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn a_data_relocation_writes_the_symbols_own_address_and_not_a_distance() {
    // A pointer in a table is dereferenced, never added to a pc.
    let obj = ObjectSpec {
        text: le32(0x0000_0013),
        data: vec![0; 8],
        symbols: vec![null_symbol(), symbol("क", 0, PLACE_TEXT, true)],
        data_relocations: vec![relocation(0, 1, relocation_number("R_RISCV_64"), 0)],
        ..ObjectSpec::default()
    };

    let mut it = linker();
    let image = link(&mut it, vec![obj.build()]);
    assert!(!image.is_nil(), "the link was refused: {:?}", errors(&it));
    let data = as_bytes(&member(&image, "दत्तम्"));
    assert_eq!(
        u64::from_le_bytes(data[0..8].try_into().expect("eight octets")),
        LOAD as u64,
        "`क` is at the load address and the pointer must hold that, absolutely"
    );

    // Past the end of `.data` is refused rather than written somewhere.
    let short = ObjectSpec {
        text: le32(0x0000_0013),
        data: vec![0; 4],
        symbols: vec![null_symbol(), symbol("क", 0, PLACE_TEXT, true)],
        data_relocations: vec![relocation(0, 1, relocation_number("R_RISCV_64"), 0)],
        ..ObjectSpec::default()
    };
    let mut it = linker();
    assert!(link(&mut it, vec![short.build()]).is_nil());
    assert_eq!(
        errors(&it).iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        vec![5],
        "दत्तातिक्रमकूटः — `a data relocation points past .data`"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The layout: `.data` eight-aligned after all the text, `.bss` after all the
// data (`B-099`)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_sections_are_laid_out_as_kosha_lays_an_image_out() {
    // A `.bss` reservation that overlapped another object's data would be a
    // buffer writing over initialised bytes, which RUNS.
    let one = ObjectSpec {
        text: vec![0x13, 0x00, 0x00, 0x00], // four octets: .data is aligned up to 8
        data: vec![9; 3],
        bss: 5,
        symbols: vec![
            null_symbol(),
            symbol("घ", 0, PLACE_DATA, true),
            symbol("ङ", 0, PLACE_BSS, true),
        ],
        ..ObjectSpec::default()
    };

    let mut it = linker();
    let image = link(&mut it, vec![one.build()]);
    assert!(!image.is_nil(), "the link was refused: {:?}", errors(&it));

    let names: HashMap<String, i128> = live(&member(&image, "नामानि"))
        .iter()
        .map(|e| {
            (
                as_text(&member(e, "नाम")),
                member(e, "स्थान").as_int().expect("an address is a number"),
            )
        })
        .collect();
    // W-363: `.data` is its own writable segment at the page after the text.
    assert_eq!(
        names.get("घ"),
        Some(&(LOAD + 4096)),
        "four octets of text, rounded up to a page, is where `.data` begins"
    );
    // `V-009` (i-b2): `.bss` starts on SIXTEEN, so the startup's stack — the
    // first thing in it — keeps `sp` 16-aligned. `कोश` still places `.bss` at
    // the data rounded to eight (4104), so the eight octets up to 4112 are
    // reported as `.bss` too: `बीजम्` is the 5 reserved plus that pad, and
    // `p_memsz` ends exactly at the reservation's end, 4117.
    assert_eq!(
        names.get("ङ"),
        Some(&(LOAD + 4096 + 16)),
        "three octets of data end at 4099, rounded up to 4112 for `.bss`"
    );
    assert_eq!(
        member(&image, "बीजम्").as_int(),
        Some(5 + 8),
        "the reservation is carried, in memory and not in the file, with the \
         pad from the data's eight to the `.bss` start's sixteen"
    );
    assert_eq!(
        live(&member(&image, "सारणी"))
            .iter()
            .map(|r| member(r, "खण्डः").as_int().unwrap_or(-1))
            .collect::<Vec<_>>(),
        vec![SEC_DATA, SEC_BSS],
        "a name's section in the image follows where the reader PLACED it"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `संस्कारः` on its own
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_patcher_rebuilds_from_the_pattern_and_refuses_what_it_cannot_reach() {
    let mut it = linker();
    // `सङ्केतन ॱ अन्तरभेद` — the slot kind a `R_RISCV_JAL` names.
    let disp = it
        .global("अन्तरभेद")
        .and_then(Value::as_int)
        .expect("सङ्केतन declares the displacement slot kind");

    let bytes = le32(JAL_RA_ZERO);
    let patched = call(
        &mut it,
        "संयोजनॱसंस्कारः",
        vec![
            octets(&bytes),
            Value::Int(0),
            Value::Int(disp),
            Value::Int(8),
        ],
    );
    // `W-359`: the patcher ALWAYS answers the run, and reports a refusal in
    // `संयोजनॱनिधानविफलम्` (it answered `शून्यम्` before the final ruling).
    assert!(
        !truth(it.global("निधानविफलम्")),
        "a reachable displacement must be written"
    );
    let word = u32::from_le_bytes(as_bytes(&patched)[0..4].try_into().expect("four octets"));
    let decoded = call(
        &mut it,
        "विश्लेषणॱविश्लेषणम्",
        vec![Value::Int(i128::from(word))],
    );
    assert_eq!(as_text(&member(&decoded, "आज्ञा")), "jal");
    assert!(
        live(&member(&decoded, "मूल्यानि"))
            .iter()
            .any(|o| member(o, "मूल्यम्").as_int() == Some(8)),
        "the displacement the patcher wrote must decode back as ८"
    );

    // A `jal` reaches ±1 MiB. Beyond that the patch is REFUSED — an
    // out-of-range immediate placed anyway loses its high bits, which is how a
    // branch 4096 bytes forward came to encode as one 4096 bytes backward.
    let mut it = linker();
    let far = call(
        &mut it,
        "संयोजनॱसंस्कारः",
        vec![
            octets(&bytes),
            Value::Int(0),
            Value::Int(disp),
            Value::Int(1 << 40),
        ],
    );
    assert!(
        truth(it.global("निधानविफलम्")) && as_bytes(&far) == bytes,
        "a displacement no `jal` can hold must be refused, the run unchanged"
    );
    assert_eq!(
        errors(&it).iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        vec![10],
        "अगम्यकूटः — `cannot reach {} bytes`",
        1i128 << 40
    );

    // Four octets that decode to nothing cannot be patched.
    let mut it = linker();
    let nothing = call(
        &mut it,
        "संयोजनॱसंस्कारः",
        vec![
            octets(&le32(0xffff_ffff)),
            Value::Int(0),
            Value::Int(disp),
            Value::Int(8),
        ],
    );
    assert!(truth(it.global("निधानविफलम्")) && as_bytes(&nothing) == le32(0xffff_ffff));
    assert_eq!(
        errors(&it).iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        vec![7],
        "अविश्लेष्यकूटः — `nothing decodes at ०`"
    );

    // And a read that runs off the end of the text.
    let mut it = linker();
    let past = call(
        &mut it,
        "संयोजनॱसंस्कारः",
        vec![
            octets(&[0, 0]),
            Value::Int(0),
            Value::Int(disp),
            Value::Int(8),
        ],
    );
    assert!(truth(it.global("निधानविफलम्")) && as_bytes(&past) == [0, 0]);
    assert_eq!(
        errors(&it).iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        vec![6],
        "पाठातिक्रमकूटः — `a relocation points past the text`"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The debug sections, carried through with their addresses fixed (`B-104`)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn the_debug_sections_are_carried_through_and_their_addresses_are_fixed() {
    // `-g` reached the object in `B-100b` and stopped there: `--संयोजय` wrote
    // text and data and dropped `.debug_line` with it, so a program linked from
    // objects had no line table at all.
    let abbrev = vec![0xAB, 0xCD];
    let with_debug = |text_len: usize, stmt_list: bool| {
        let mut symbols = vec![null_symbol(), symbol("क", 0, PLACE_TEXT, false)];
        if stmt_list {
            symbols.push(symbol("", 0, PLACE_DEBUG_LINE, false));
        }
        ObjectSpec {
            text: [0x13, 0x00, 0x00, 0x00].repeat(text_len / 4),
            symbols,
            debug: vec![
                (".debug_abbrev".into(), abbrev.clone()),
                (".debug_info".into(), vec![0; 12]),
                (".debug_line".into(), vec![0; 8]),
            ],
            debug_relocations: vec![
                (".debug_info".to_string(), {
                    let mut rs = vec![relocation(0, 1, 0, 0)];
                    if stmt_list {
                        rs.push(relocation(8, 2, 0, 0));
                    }
                    rs
                }),
                (".debug_line".to_string(), vec![relocation(0, 1, 0, 0)]),
            ],
            ..ObjectSpec::default()
        }
    };

    let mut it = linker();
    let image = link(
        &mut it,
        vec![with_debug(4, true).build(), with_debug(4, true).build()],
    );
    assert!(!image.is_nil(), "the link was refused: {:?}", errors(&it));

    let sections: Vec<(String, Vec<u8>)> = live(&member(&image, "शोधनम्"))
        .iter()
        .map(|s| (as_text(&member(s, "नाम")), as_bytes(&member(s, "दत्तम्"))))
        .collect();
    let names: Vec<&str> = sections.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        vec![".debug_abbrev", ".debug_info", ".debug_line"],
        "an image linked from objects must still carry its line table"
    );

    let abbrev_out = &sections[0].1;
    assert_eq!(
        abbrev_out, &abbrev,
        ".debug_abbrev is taken ONCE — every unit points at offset ०"
    );
    let info = &sections[1].1;
    assert_eq!(info.len(), 24, "two units of .debug_info, concatenated");
    let lines = &sections[2].1;
    assert_eq!(lines.len(), 16, "two line programs, concatenated");

    // `DW_AT_low_pc` — an eight-octet ADDRESS in .text, per object.
    assert_eq!(
        u64::from_le_bytes(info[0..8].try_into().expect("eight octets")),
        LOAD as u64
    );
    assert_eq!(
        u64::from_le_bytes(info[12..20].try_into().expect("eight octets")),
        LOAD as u64 + 4,
        "the second unit's low_pc is the second object's text base"
    );
    // `DW_AT_stmt_list` — FOUR octets, and an OFFSET into the merged
    // `.debug_line`, not an address. One symbol serving both would hand the
    // linker the wrong base for one of them (`B-104b`).
    assert_eq!(
        u32::from_le_bytes(info[8..12].try_into().expect("four octets")),
        0,
        "the first unit's line program is still at zero"
    );
    assert_eq!(
        u32::from_le_bytes(info[20..24].try_into().expect("four octets")),
        8,
        "the SECOND unit's line program is not, and this is the field B-104b \
         exists to move"
    );
    // `.debug_line`'s own record is a `DW_LNE_set_address`, a text address.
    assert_eq!(
        u64::from_le_bytes(lines[8..16].try_into().expect("eight octets")),
        LOAD as u64 + 4
    );
}

#[test]
fn objects_that_disagree_about_debug_abbrev_are_refused() {
    // Every unit points at abbrev offset ०, so two different tables cannot
    // both be reachable. Refused rather than silently using the first, which
    // would decode the second unit's DIEs by the wrong shape.
    let with = |table: Vec<u8>| ObjectSpec {
        text: le32(0x0000_0013),
        symbols: vec![null_symbol(), symbol("क", 0, PLACE_TEXT, false)],
        debug: vec![(".debug_abbrev".into(), table)],
        ..ObjectSpec::default()
    };

    let mut it = linker();
    let refused = link(&mut it, vec![with(vec![1]).build(), with(vec![2]).build()]);
    assert!(refused.is_nil(), "disagreeing tables must be refused");
    assert_eq!(
        errors(&it).iter().map(|(c, _)| *c).collect::<Vec<_>>(),
        vec![13],
        "संक्षेपविरोधकूटः"
    );

    let mut it = linker();
    assert!(
        !link(&mut it, vec![with(vec![1]).build(), with(vec![1]).build()]).is_nil(),
        "the SAME table twice is the ordinary case: {:?}",
        errors(&it)
    );
}

// ─────────────────────────────────────────────────────────────────────────
// MUTATIONS. Each breaks one comparison and requires the answer to change.
// ─────────────────────────────────────────────────────────────────────────

/// Link `objects` with `samyojana.t1` mutated, and answer the image (or
/// `शून्यम्`) together with the refusals raised.
fn link_mutated(from: &str, to: &str, objects: Vec<Value>) -> (Value, Vec<(i128, String)>) {
    let src = mutate(&source("samyojana.t1"), from, to);
    let mut it = linker_with(&src);
    let image = it
        .call("संयोजनॱसंयोजनम्", vec![list(objects)], FUEL)
        .unwrap_or_else(|e| panic!("the mutated linker still runs: {e}"));
    let raised = errors(&it);
    (image, raised)
}

/// The names of a linked image, by address.
fn addresses(image: &Value) -> HashMap<String, i128> {
    live(&member(image, "नामानि"))
        .iter()
        .map(|e| {
            (
                as_text(&member(e, "नाम")),
                member(e, "स्थान").as_int().expect("an address is a number"),
            )
        })
        .collect()
}

/// Two objects that need no relocation, so that a mutation of the LAYOUT is
/// measured by the layout and not by a patch failing downstream of it.
fn two_plain_objects() -> Vec<Value> {
    let one = ObjectSpec {
        text: [le32(0x0000_0013), le32(0x0000_0013)].concat(),
        symbols: vec![null_symbol(), symbol("मुख्यम्", 0, PLACE_TEXT, true)],
        ..ObjectSpec::default()
    };
    let two = ObjectSpec {
        text: le32(0x0000_8067),
        symbols: vec![null_symbol(), symbol("सहायः", 0, PLACE_TEXT, true)],
        ..ObjectSpec::default()
    };
    vec![one.build(), two.build()]
}

#[test]
fn pinning_the_text_cursor_makes_the_two_objects_overlap() {
    // `पाठसंयोगः` copies object n at `पाठारम्भः वस्तूनि क्रमः`. Written at ०
    // instead, the second object's text lands on the first's — which is what
    // "end to end" means, tested by breaking it.
    let (image, raised) = link_mutated(
        "फलम् भवति अष्टकसंयोगः आरभ्य फलम् ऽ पाठारम्भः वस्तूनि क्रमः ऽ वस्तु ॱ पाठ्यम् समाप्तम् ।",
        "फलम् भवति अष्टकसंयोगः आरभ्य फलम् ऽ ० ऽ वस्तु ॱ पाठ्यम् समाप्तम् ।",
        two_plain_objects(),
    );
    assert!(!image.is_nil(), "the mutated link still ran: {raised:?}");
    assert_eq!(
        as_bytes(&member(&image, "पाठ्यम्")).len(),
        8,
        "with the cursor pinned the second object overwrote the first and the \
         image is eight octets, not twelve"
    );
    // And unmutated it is twelve.
    let mut it = linker();
    assert_eq!(
        as_bytes(&member(&link(&mut it, two_plain_objects()), "पाठ्यम्")).len(),
        12
    );
}

#[test]
fn taking_the_load_address_out_of_a_symbols_base_moves_every_name() {
    // `स्थापनाधारः` answers `load + text_at[n]` for a `.text` definition. Drop
    // the load address and every name is an OFFSET presented as an ADDRESS —
    // which the image would then write into `.symtab` and a debugger believe.
    let (image, raised) = link_mutated(
        "        प्रत्यागमनम् भारणम् योगः पाठस्थानम् ।",
        "        प्रत्यागमनम् पाठस्थानम् ।",
        two_plain_objects(),
    );
    assert!(!image.is_nil(), "the mutated link still ran: {raised:?}");
    assert_eq!(
        addresses(&image).get("सहायः"),
        Some(&8),
        "without the load address `सहायः` is at ८, not at {:#x}",
        LOAD + 8
    );
}

#[test]
fn treating_a_global_as_a_local_accepts_a_name_defined_twice() {
    // `पुनरुक्तनिर्यातम्` opens by letting a LOCAL through: a label is local to
    // its file unless exported, precisely so two files may each carry a `चक्रः`
    // (`B-014`). Invert that one comparison and the two classes swap — which is
    // exactly `B-103`, a linker that refused the ordinary case and accepted the
    // one that makes link order into program behaviour.
    let one = || ObjectSpec {
        text: le32(0x0000_0013),
        symbols: vec![null_symbol(), symbol("क", 0, PLACE_TEXT, true)],
        ..ObjectSpec::default()
    };
    let (image, raised) = link_mutated(
        "    यदि वैश्विकम् समम् असत्यम् आदि",
        "    यदि वैश्विकम् समम् सत्यम् आदि",
        vec![one().build(), one().build()],
    );
    assert!(
        !image.is_nil(),
        "with the two classes swapped the collision is no longer refused, \
         which is exactly what the unmutated test asserts against: {raised:?}"
    );
}

#[test]
fn dropping_the_alignment_puts_bss_where_the_data_ended() {
    // `अष्टकसंरेखणम्` rounds a length up to the next multiple of eight. Without
    // it `.bss` following unaligned data overlaps another object's initialised
    // bytes, and a buffer written over live data still runs. (Until W-363 this
    // asserted `.data` at the text's end; `.data` is placed at the page after
    // the text by `कोशॱदत्तपृष्ठाधारः` now, so the eight is read off `.bss`.)
    let obj = || ObjectSpec {
        text: vec![0x13, 0x00, 0x00, 0x00],
        data: vec![9; 3],
        bss: 5,
        symbols: vec![
            null_symbol(),
            symbol("घ", 0, PLACE_DATA, true),
            symbol("ङ", 0, PLACE_BSS, true),
        ],
        ..ObjectSpec::default()
    };
    let (image, raised) = link_mutated(
        "    प्रत्यागमनम् मूल्यम् योगः आरभ्य ८ वियोगः शेषः समाप्तम् ।",
        "    प्रत्यागमनम् मूल्यम् ।",
        vec![obj().build()],
    );
    assert!(!image.is_nil(), "the mutated link still ran: {raised:?}");
    // `V-009` (i-b2): `बीजाधारः` now steps the eight-aligned end on by eight
    // when it is not on sixteen. With the eight gone, 4099 is not on sixteen
    // and steps to 4107 — still odd, still not the 4112 the unmutated link
    // gives, so the mutant stays caught.
    assert_eq!(
        addresses(&image).get("ङ"),
        Some(&(LOAD + 4096 + 3 + 8)),
        "unaligned, `.bss` begins at three octets of data plus the sixteen's step"
    );
}

#[test]
fn making_the_low_half_measure_from_its_own_pc_is_wrong_by_four() {
    // `उपरिपदस्थानम्`: `ॱअधः` completes the register the PRECEDING instruction
    // set, so the pair is positional and the low half is computed against the
    // `auipc`'s pc. This mutation is the "off by four" the routine's own
    // comment names, and it must change the number.
    let mut it = linker();
    let straight = call(
        &mut it,
        "संयोजनॱउपरिपदस्थानम्",
        vec![
            Value::Int(LOAD),
            Value::Int(16),
            Value::Int(0),
            Value::Int(0),
        ],
    )
    .as_int();
    assert_eq!(straight, Some(LOAD + 16));

    let src = mutate(
        &source("samyojana.t1"),
        "    चरः स्थानाङ्कः ॱॱ अ६४ भवति आधारः योगः संज्ञामूल्यम् ।",
        "    चरः स्थानाङ्कः ॱॱ अ६४ भवति आधारः योगः संज्ञामूल्यम् योगः ४ ।",
    );
    let mut it = linker_with(&src);
    let bent = it
        .call(
            "संयोजनॱउपरिपदस्थानम्",
            vec![
                Value::Int(LOAD),
                Value::Int(16),
                Value::Int(0),
                Value::Int(0),
            ],
            FUEL,
        )
        .expect("the mutated routine still runs")
        .as_int();
    assert_eq!(bent, Some(LOAD + 20), "the pc moved by four, as it must");
    assert_ne!(straight, bent);
}

#[test]
fn refusing_to_look_past_the_object_scope_loses_the_cross_object_call() {
    // `निर्णीतस्थानम्` asks the object's OWN names first and then what any
    // object exported. Stop at the first and a name defined in another file is
    // undefined — which is the whole of what separate compilation buys.
    let (image, raised) = link_mutated(
        "    प्रत्यागमनम् स्थानान्वेषणम् निर्यातानि नाम ।",
        "    प्रत्यागमनम् शून्यम् ।",
        vec![caller().build(), callee().build()],
    );
    assert!(image.is_nil(), "`सहायः` is not in the caller's own scope");
    assert_eq!(
        raised.iter().map(|(_, n)| n.as_str()).collect::<Vec<_>>(),
        vec!["सहायः"],
        "and the refusal must name it"
    );
}

#[test]
fn a_patch_that_does_not_start_from_the_pattern_is_not_what_this_module_does() {
    // `पदपुनर्निर्माणम्` rebuilds from `सङ्केतः ॱ आकृति`, the encoding's fixed
    // bits. Started from ० instead, the word loses its opcode and no longer
    // decodes as the instruction it was.
    let (image, raised) = link_mutated(
        "    चरः पदम् ॱॱ अ३२ भवति सङ्केतः ॱ आकृति ।",
        "    चरः पदम् ॱॱ अ३२ भवति ० ।",
        vec![caller().build(), callee().build()],
    );
    assert!(!image.is_nil(), "the mutated link still ran: {raised:?}");
    let text = as_bytes(&member(&image, "पाठ्यम्"));
    let word = u32::from_le_bytes(text[0..4].try_into().expect("four octets"));
    assert_ne!(
        word & 0x7f,
        0x6f,
        "without the pattern the `jal` opcode is gone; the word is {word:#010x}"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Symbol lookup step 2 — the linker's name table, characterised
// ─────────────────────────────────────────────────────────────────────────

/// A linker name table driven through `नामस्थानयोजनम्` (insert) and
/// `स्थानान्वेषणम्` (get), the two routines symbol-lookup step 2 rewrites.
///
/// **IT RUNS ON BOTH SIDES OF THE REWRITE.** Before step 2 the insert takes four
/// arguments and scans; after it, a fifth — the table's hash index, from
/// `नामसूचीरचना` — and probes. The arity is read off the loaded routine, so one
/// test pins the answers of both.
struct NameTable {
    it: Interpreter,
    table: Value,
    index: Option<Value>,
    count: i128,
}

impl NameTable {
    fn with(samyojana: &str, capacity: i128, fuel: u64) -> Result<Self, String> {
        let mut it = linker_with(samyojana);
        let arity = it
            .routines()
            .find(|r| r.name == "नामस्थानयोजनम्")
            .expect("the insert is declared")
            .arity();
        let index = match arity {
            4 => None,
            5 => Some(
                it.call("संयोजनॱनामसूचीरचना", vec![Value::Int(capacity)], fuel)
                    .map_err(|e| format!("the index builds: {e}"))?,
            ),
            n => panic!("`नामस्थानयोजनम्` takes {n} arguments, neither 4 nor 5"),
        };
        Ok(NameTable {
            it,
            table: list(Vec::new()),
            index,
            count: 0,
        })
    }

    fn new(capacity: i128) -> Self {
        Self::with(&source("samyojana.t1"), capacity, FUEL).expect("the table is made")
    }

    fn try_insert(&mut self, name: &[u8], at: i128, fuel: u64) -> Result<(), String> {
        let mut args = vec![
            self.table.clone(),
            Value::Int(self.count),
            octets(name),
            Value::Int(at),
        ];
        if let Some(ix) = &self.index {
            args.push(ix.clone());
        }
        self.table = self
            .it
            .call("संयोजनॱनामस्थानयोजनम्", args, fuel)
            .map_err(|e| format!("insert {:?}: {e}", String::from_utf8_lossy(name)))?;
        self.count = self
            .it
            .global("नामस्थानगणना")
            .and_then(Value::as_int)
            .expect("the insert publishes its count");
        Ok(())
    }

    fn insert(&mut self, name: &[u8], at: i128) {
        self.try_insert(name, at, FUEL)
            .unwrap_or_else(|e| panic!("{e}"));
    }

    fn try_get(&mut self, name: &[u8], fuel: u64) -> Result<Option<i128>, String> {
        let v = self
            .it
            .call(
                "संयोजनॱस्थानान्वेषणम्",
                vec![self.table.clone(), octets(name)],
                fuel,
            )
            .map_err(|e| format!("get {:?}: {e}", String::from_utf8_lossy(name)))?;
        Ok(if v.is_nil() {
            None
        } else {
            Some(v.as_int().expect("an address is a number"))
        })
    }

    fn get(&mut self, name: &[u8]) -> Option<i128> {
        self.try_get(name, FUEL).unwrap_or_else(|e| panic!("{e}"))
    }

    /// The arena, in order: every entry's name and address.
    fn rows(&self) -> Vec<(Vec<u8>, i128)> {
        arena(&self.table)
            .iter()
            .map(|e| {
                (
                    as_bytes(&member(e, "नाम")),
                    member(e, "स्थान").as_int().expect("an address"),
                )
            })
            .collect()
    }
}

/// **`नामस्थानयोजनम्` AND `स्थानान्वेषणम्` ANSWER THE SAME BEFORE AND AFTER THE
/// HASH INDEX** (symbol-lookup step 2, characterisation).
///
/// Replace-or-append is `BTreeMap::insert`: a name already present keeps its
/// place and takes the new address, a new one is appended at the count, and the
/// count the insert publishes moves only on an append. Get answers the
/// address, or nothing. Pinned on: an empty table, appends, a replace, the
/// first-inserted name found after many later ones, an absent name, a byte
/// prefix of a present name, a name one octet longer, a name differing in its
/// last octet only, and the EMPTY name (absent, then inserted and replaced).
/// The arena's ORDER and CONTENTS are asserted whole after every phase.
#[test]
fn the_name_table_answers_the_same_on_every_edge() {
    let first = "मुख्यम्".as_bytes().to_vec();
    let second = "सहायः".as_bytes().to_vec();
    let mut longer = first.clone();
    longer.extend_from_slice(&second);
    let mut last_off = first.clone();
    *last_off.last_mut().unwrap() ^= 1;
    let prefix = first[..first.len() - 1].to_vec();
    let many: Vec<Vec<u8>> = (0..40).map(|i| format!("चक्रः{i}").into_bytes()).collect();

    let mut t = NameTable::new(64);
    // (1) the EMPTY table
    assert_eq!(t.get(&first), None, "an empty table holds nothing");
    assert_eq!(t.get(b""), None, "not even the empty name");

    // (2) appends
    t.insert(&first, 10);
    assert_eq!(t.count, 1, "an append counts");
    t.insert(&second, 20);
    t.insert(&longer, 30);
    t.insert(&last_off, 40);
    assert_eq!(t.count, 4);
    let mut want: Vec<(Vec<u8>, i128)> = vec![
        (first.clone(), 10),
        (second.clone(), 20),
        (longer.clone(), 30),
        (last_off.clone(), 40),
    ];
    assert_eq!(t.rows(), want, "appended in order");
    for (name, at) in &want {
        assert_eq!(
            t.get(name),
            Some(*at),
            "{:?}",
            String::from_utf8_lossy(name)
        );
    }
    assert_eq!(t.get("अन्यत्".as_bytes()), None, "an absent name");
    assert_eq!(t.get(&prefix), None, "a byte prefix of a present name");
    assert_eq!(t.get(b""), None, "the empty name, not inserted");

    // (3) a REPLACE keeps the place and the count
    t.insert(&second, 99);
    assert_eq!(t.count, 4, "a replace does not count");
    want[1].1 = 99;
    assert_eq!(t.rows(), want, "replaced in place");
    assert_eq!(t.get(&second), Some(99));
    assert_eq!(t.get(&first), Some(10), "its neighbours keep theirs");
    assert_eq!(t.get(&longer), Some(30));

    // (4) the empty NAME is a name like any other
    t.insert(b"", 7);
    assert_eq!(t.count, 5);
    assert_eq!(t.get(b""), Some(7));
    t.insert(b"", 8);
    assert_eq!(t.count, 5, "the empty name replaced, not appended twice");
    assert_eq!(t.get(b""), Some(8));
    want.push((Vec::new(), 8));

    // (5) the first-inserted name is still found after many later ones
    for (i, name) in many.iter().enumerate() {
        t.insert(name, 1000 + i as i128);
        want.push((name.clone(), 1000 + i as i128));
    }
    assert_eq!(t.count, 45);
    assert_eq!(t.rows(), want, "order and contents, whole");
    for (name, at) in &want {
        assert_eq!(
            t.get(name),
            Some(*at),
            "{:?}",
            String::from_utf8_lossy(name)
        );
    }
    assert_eq!(t.get(&prefix), None);
    assert_eq!(t.get("चक्रः40".as_bytes()), None, "one past the last");
    println!("METRIC t1_name_table_edges {}", want.len());
}

/// `नामसारः` as the margin states it: every octet masked to eight bits, and the
/// running value to TWENTY on every step, so nothing passes २^२५ on either
/// engine. A reference model and not a second copy of the routine: the
/// routine is asked, and this is what it must answer.
fn name_hash_model(name: &[u8]) -> i128 {
    name.iter()
        .fold(0i128, |h, b| (h * 31 + i128::from(*b)) & 0xF_FFFF)
}

/// The whole collision scenario against one version of the linker, as a
/// `Result` so a mutant can be required to FAIL — on a wrong answer or on a
/// run that does not end within `fuel` (a probe that never reaches an empty
/// slot is a hang, and a hang must be a red, not a stuck test).
fn name_index_scenario(samyojana: &str, fuel: u64) -> Result<usize, String> {
    // (1) the hash, against the model, on names long enough that an unmasked
    //     step would leave twenty bits within a few octets
    let mut t = NameTable::with(samyojana, 4, fuel)?;
    let long: Vec<Vec<u8>> = vec![
        vec![0xff; 300],
        "सार्वजनिकवृत्तिःनामस्थानयोजनम्".repeat(6).into_bytes(),
        Vec::new(),
        vec![0x80],
    ];
    for name in &long {
        let got =
            t.it.call("संयोजनॱनामसारः", vec![octets(name)], fuel)
                .map_err(|e| format!("hash: {e}"))?
                .as_int()
                .ok_or("the hash is not a number")?;
        if got != name_hash_model(name) {
            return Err(format!(
                "hash of {} octets is {got:#x}, the model {:#x}",
                name.len(),
                name_hash_model(name)
            ));
        }
    }

    // (2) the index's size: a power of two at least twice the names
    let slots = match &t.index {
        Some(ix) => arena(ix).len() as i128,
        None => return Err("no index".into()),
    };
    if slots != 8 {
        return Err(format!("an index for 4 names has {slots} slots, not 8"));
    }

    // (3) SIX names that land in ONE slot of eight: five inserted, so the
    //     chain is five long — longer than the slots of one parity, which is
    //     what a probe stepping by two can reach — and the sixth asked for
    //     and absent
    let mut by_slot: HashMap<i128, Vec<Vec<u8>>> = HashMap::new();
    let mut same = Vec::new();
    for i in 0..10_000 {
        let name = format!("क{i}").into_bytes();
        let b = by_slot
            .entry(name_hash_model(&name) & (slots - 1))
            .or_default();
        b.push(name);
        if b.len() == 6 {
            same = b.clone();
            break;
        }
    }
    if same.len() != 6 {
        return Err("no six names share a slot".into());
    }
    for (i, name) in same[..5].iter().enumerate() {
        t.try_insert(name, 100 + i as i128, fuel)?;
    }
    if t.count != 5 || !errors(&t.it).is_empty() {
        return Err(format!(
            "five sharing names counted {}, refusals {:?}",
            t.count,
            errors(&t.it)
        ));
    }
    for (i, name) in same[..5].iter().enumerate() {
        let got = t.try_get(name, fuel)?;
        if got != Some(100 + i as i128) {
            return Err(format!("name {i} of a shared slot answered {got:?}"));
        }
    }
    let absent = t.try_get(&same[5], fuel)?;
    if absent.is_some() {
        return Err(format!(
            "an absent name in a full chain answered {absent:?}"
        ));
    }
    // a replace in the MIDDLE of the chain keeps place and count
    t.try_insert(&same[2], 7, fuel)?;
    if t.count != 5 {
        return Err(format!("a replace in the chain counted {}", t.count));
    }
    let want: Vec<(Vec<u8>, i128)> = same[..5]
        .iter()
        .enumerate()
        .map(|(i, n)| (n.clone(), if i == 2 { 7 } else { 100 + i as i128 }))
        .collect();
    if t.rows() != want {
        return Err("the table's order or contents moved".into());
    }
    for (name, at) in &want {
        let got = t.try_get(name, fuel)?;
        if got != Some(*at) {
            return Err(format!(
                "after the replace, a chained name answered {got:?}"
            ));
        }
    }
    Ok(same.len())
}

/// **NAMES THAT SHARE A SLOT ARE TOLD APART BY THEIR OCTETS** (symbol-lookup
/// step 2). Five names hashing to one slot of eight, inserted, found, one
/// replaced mid-chain; a sixth of the same slot is absent. Plus the hash
/// against its model and the index's size.
#[test]
fn the_name_index_tells_apart_names_that_share_a_slot() {
    let n = name_index_scenario(&source("samyojana.t1"), FUEL).unwrap_or_else(|e| panic!("{e}"));
    for (cap, slots) in [(0, 1), (1, 2), (3, 8), (4, 8), (5, 16), (64, 128)] {
        let t = NameTable::new(cap);
        let ix = t.index.as_ref().expect("the insert takes an index");
        assert_eq!(arena(ix).len(), slots, "an index for {cap} names");
        assert!(
            arena(ix).iter().all(|c| c.as_int() == Some(0)),
            "built empty"
        );
    }
    println!("METRIC t1_name_index_shared_slot {n}");
}

/// Each of the three mutants the step was asked to refute goes RED on the
/// scenario above: the slot mask one too wide, the twenty-bit hash mask
/// widened, the probe stepping by two (half the slots, so a chain of five
/// overfills its parity and the fifth insert finds no slot), and a hash hit taken
/// as a match without comparing the names.
#[test]
fn the_name_index_mutants_go_red() {
    let src = source("samyojana.t1");
    for (what, from, to) in [
        (
            "the slot mask is the length, not the length less one",
            "    चरः सीमा ॱॱ न६४ भवति नामसूचीकोश ॱ दैर्घ्य वियोगः १ ।",
            "    चरः सीमा ॱॱ न६४ भवति नामसूचीकोश ॱ दैर्घ्य ।",
        ),
        (
            "the hash is masked to twenty-four bits, not twenty",
            "        फलम् भवति फलम् युक् १०४८५७५ ।",
            "        फलम् भवति फलम् युक् १६७७७२१५ ।",
        ),
        (
            "the probe steps by two",
            "        क्रमः भवति आरभ्य क्रमः योगः १ समाप्तम् युक् सीमा ।",
            "        क्रमः भवति आरभ्य क्रमः योगः २ समाप्तम् युक् सीमा ।",
        ),
        (
            "a hash hit is taken without comparing the names",
            "        यदि प्रविष्टिः ॱ नाम समम् नाम आदि",
            "        यदि प्रविष्टिः असमम् शून्यम् आदि",
        ),
    ] {
        let mutant = mutate(&src, from, to);
        match name_index_scenario(&mutant, 50_000_000) {
            Ok(_) => panic!("mutant `{what}` passed the scenario"),
            Err(e) => println!("mutant `{what}`: red — {e}"),
        }
    }
}

/// **A FULL INDEX ANSWERS, AND AN OVER-FULL ONE REFUSES — NEITHER HANGS**
/// (review finding 1, symbol-lookup step 2). Every caller sizes its index to at
/// least twice its names, so no link fills one; this pins what a future
/// undersized caller would meet. An index of EIGHT slots (built for four names)
/// is filled with eight: all are found, and an absent name answers absent after
/// probing every slot. An index of ONE slot (built for none) takes one name, and
/// the second insert is REFUSED through the linker's refusal records (code १४),
/// the table and its count unchanged. Fuel is bounded, so a hang is a red.
#[test]
fn a_full_name_index_answers_absent_and_an_overfull_one_refuses() {
    let fuel = 50_000_000;
    let names: Vec<Vec<u8>> = (0..8).map(|i| format!("पूर्णम्{i}").into_bytes()).collect();
    let mut t = NameTable::with(&source("samyojana.t1"), 4, fuel).expect("the table is made");
    assert_eq!(arena(t.index.as_ref().expect("an index")).len(), 8);
    for (i, name) in names.iter().enumerate() {
        t.try_insert(name, 10 + i as i128, fuel)
            .unwrap_or_else(|e| panic!("{e}"));
    }
    assert_eq!(t.count, 8, "eight names fill eight slots");
    assert!(errors(&t.it).is_empty(), "{:?}", errors(&t.it));
    for (i, name) in names.iter().enumerate() {
        assert_eq!(t.try_get(name, fuel), Ok(Some(10 + i as i128)));
    }
    assert_eq!(
        t.try_get("अन्यत्".as_bytes(), fuel),
        Ok(None),
        "an absent name in a FULL index is absent, not a hang"
    );

    let mut t = NameTable::with(&source("samyojana.t1"), 0, fuel).expect("the table is made");
    t.try_insert("क".as_bytes(), 1, fuel)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(t.count, 1);
    t.try_insert("ख".as_bytes(), 2, fuel)
        .unwrap_or_else(|e| panic!("the second insert must refuse, not hang: {e}"));
    assert_eq!(t.count, 1, "a refused insert does not count");
    assert_eq!(
        errors(&t.it),
        vec![(14, "ख".to_string())],
        "refused by name, code १४"
    );
    assert_eq!(t.rows(), vec![("क".as_bytes().to_vec(), 1)]);
    assert_eq!(t.try_get("क".as_bytes(), fuel), Ok(Some(1)));
    assert_eq!(t.try_get("ख".as_bytes(), fuel), Ok(None));
}

/// The characterisation above, AT SCALE: 1,000 and 10,000 names, every
/// seventh one re-inserted with a new address after all are in, then the
/// arena's order and contents asserted whole and every name looked up. Runs on
/// both sides of the rewrite, so the probing path is pinned where chains form.
#[test]
fn the_name_table_answers_the_same_at_scale() {
    for n in [1_000usize, 10_000] {
        let mut t = NameTable::new(n as i128);
        let mut want: Vec<(Vec<u8>, i128)> = Vec::with_capacity(n);
        for i in 0..n {
            let name = format!("नाम{i}").into_bytes();
            t.insert(&name, i as i128);
            want.push((name, i as i128));
        }
        for i in (0..n).step_by(7) {
            t.insert(&want[i].0.clone(), 1_000_000 + i as i128);
            want[i].1 = 1_000_000 + i as i128;
        }
        assert_eq!(t.count, n as i128, "replaces do not count, n = {n}");
        assert!(t.rows() == want, "order and contents, whole, n = {n}");
        for (name, at) in &want {
            assert_eq!(t.get(name), Some(*at), "n = {n}");
        }
        assert_eq!(t.get(format!("नाम{n}").as_bytes()), None, "n = {n}");
        println!("METRIC t1_name_table_scale {n}");
    }
}
