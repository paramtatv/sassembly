//! **`W-381` STAGE 3a — ONE INTEGER SEMANTICS ON EVERY ENGINE: 64-BIT
//! TWO'S-COMPLEMENT WRAPPING, AND `न६४` ORDERED UNSIGNED.**
//!
//! The owner's ruling P1 (2026-10-05): *"64-bit two's-complement WRAPPING is the
//! defined semantics, the native unsigned compare for न६४ is corrected, and
//! explicit checked/trapping operators are provided."* This file holds the first
//! two halves; the checked operators wait for their names.
//!
//! EVERY ASSERTION IS AN ABSOLUTE VALUE, on the interpreter, on both images
//! (`.t1` chain, Rust twin) under `yantra`, and on QEMU — never "the engines
//! agree". `W-333`'s lesson: before that fix both engines answered the same
//! wrong number, and every agreement check was blind to it.
//!
//! THE DEFINED SEMANTICS, AS BUILT (what the RV64 hardware does, adopted by the
//! interpreter):
//!
//! - `योगः`, `वियोगः`, `गुणनम्` keep the low 64 bits (`add`/`sub`/`mul`).
//! - A shift uses its count MODULO 64 (`sll`/`srl`/`sra` read the low six bits):
//!   `१ वामसृ ६४` is १, `१ वामसृ ६५` is २, a shift by ६४ is the operand.
//! - An ordering comparison (`न्यूनम्`, `अधिकम्`, `बृहत्समम्`) is UNSIGNED when
//!   either operand is a NAME declared unsigned (`न६४` and the rest of
//!   `UNSIGNED_INTEGER_TYPES`), and SIGNED otherwise — the rule `W-333` set for
//!   `दक्षिणसृ`, read from the same declaration on both engines.
//! - `समम्`/`असमम्` compare the 64-bit word, so `० वियोगः १` equals २^६४−१.
//! - `विभाजनम्` is the signed `div`, as natively before this change: ऋण२^६३ ÷
//!   ऋण१ is ऋण२^६३ (RV64's defined overflow quotient). Division and remainder BY
//!   ZERO are NOT changed here: the interpreter still refuses them and the
//!   native code answers RV64's all-ones / the dividend (`b1 p19`, still a KNOWN
//!   row in `w381_engine_agreement.rs`, put to the owner).
//!
//! The fixtures are kept on a Linux x86-64 host, copied into
//! `tests/data/b1/` byte for byte, as `.t1probe` (a `.t1` under `crates/` is
//! read as a corpus module by `t1_modules.rs`, and every probe declares the
//! same one); the derived programs below change ONE line of a fixture each, and
//! [`derive`] refuses if that line is not there exactly once.

mod qemu_leg;

use sadhana::t1::agreement::STATUS_MASK;
use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::{Halt, Machine};

const FUEL: u64 = 80_000_000_000;
const MODULE: &str = "परकर्तृ";

const P14: &str = include_str!("data/b1/p14_add_overflow.t1probe");
const P15: &str = include_str!("data/b1/p15_unsigned_cmp.t1probe");
const P16: &str = include_str!("data/b1/p16_underflow_cmp.t1probe");
const P17: &str = include_str!("data/b1/p17_shl64.t1probe");
const P18: &str = include_str!("data/b1/p18_srl_srA.t1probe");
const P19: &str = include_str!("data/b1/p19_div0.t1probe");
const P21: &str = include_str!("data/b1/p21_mul_wrap.t1probe");
const P20: &str = include_str!("data/b1/p20_octet_store_300.t1probe");

/// The two comparison words no fixture spells, as the interpreter's parser
/// reads them (`nirvahana.rs`, the comparison arm).
const GT: &str = "अधिकम्";
const GE: &str = "बृहत्समम्";

/// `W-381` stage 3, the owner's four checked members (copied from the ruling by
/// script, never retyped), the import line and the grouping words of
/// `v005_floats.rs`'s float calls, and the remainder operator.
const CHECKED_ADD: &str = "अष्टकॱसुरक्षितयोगः";
const CHECKED_SUB: &str = "अष्टकॱसुरक्षितवियोगः";
const CHECKED_MUL: &str = "अष्टकॱसुरक्षितगुणनम्";
const CHECKED_SHL: &str = "अष्टकॱसुरक्षितवामसरणम्";
const IMPORT: &str = "आयातः अष्टक ।";
const REM: &str = "शेषः";

/// `<builtin> आरभ्य a ऽ b समाप्तम्`, the grouped call form.
fn call(f: &str, a: &str, b: &str) -> String {
    format!("{f} आरभ्य {a} ऽ {b} समाप्तम्")
}

/// `src` with the import line after the module line.
fn imported(src: &str) -> String {
    let (head, rest) = src.split_once('\n').expect("a module line");
    format!("{head}\n{IMPORT}\n{rest}")
}

/// The overflow and division-by-zero refusal codes (owner rulings 2026-10-06).
const OVERFLOW: u64 = 0x35c;
const DIV_ZERO: u64 = 0x35e;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

/// A decimal numeral in the corpus's digits.
fn dec(n: u64) -> String {
    const DIGITS: [&str; 10] = ["०", "१", "२", "३", "४", "५", "६", "७", "८", "९"];
    n.to_string()
        .chars()
        .map(|c| DIGITS[c.to_digit(10).unwrap() as usize])
        .collect()
}

/// The `n`th whitespace token of line `line` of `src` — how a derived program
/// takes its words from a fixture instead of retyping them.
fn tok(src: &str, line: usize, n: usize) -> &str {
    src.lines()
        .nth(line)
        .and_then(|l| l.split_whitespace().nth(n))
        .unwrap_or_else(|| panic!("no token {n} on line {line}"))
}

/// `src` with each `(from, to)` replaced; every `from` must occur EXACTLY once,
/// so a fixture that changed cannot leave a derived program silently unchanged.
fn derive(src: &str, edits: &[(&str, &str)]) -> String {
    let mut out = src.to_string();
    for (from, to) in edits {
        assert_eq!(
            out.matches(from).count(),
            1,
            "`{from}` must occur once in\n{out}"
        );
        out = out.replacen(from, to, 1);
    }
    out
}

/// The negative numeral of magnitude `n`, its sign word read off `p18`'s `ऋण१`.
fn negative(n: u64) -> String {
    let minus_one = tok(P18, 3, 5);
    let sign = minus_one
        .strip_suffix(dec(1).as_str())
        .expect("a sign word, then the digit one");
    format!("{sign}{}", dec(n))
}

fn interpreted(src: &str) -> Result<i128, String> {
    let mut all: Vec<(&str, &str)> = CHAIN.to_vec();
    all.push(("परकर्तृ.t1", src));
    let mut it = Interpreter::load(&all, &spec_root()).map_err(|e| e.reason)?;
    match it.call(&format!("{MODULE}\u{971}मुख्यम्"), Vec::new(), FUEL) {
        Ok(Value::Int(n)) => Ok(n),
        Ok(v) => Err(format!("answered {v:?}")),
        Err(e) => Err(e.reason),
    }
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

/// `src`'s entry returns `want` (a small non-zero status) on the interpreter,
/// on both images under `yantra`, and on both images under QEMU. Every engine
/// is run before anything is asserted, so a failure names all four answers.
fn returns_everywhere(what: &str, src: &str, want: u64) {
    assert!((1..=255).contains(&want), "a status QEMU can carry whole");
    let mut bad = Vec::new();
    let interp = interpreted(src);
    println!("{what}: interpreter {interp:?}");
    match &interp {
        Ok(n) if (*n as u128) & STATUS_MASK == u128::from(want) => {}
        other => bad.push(format!("interpreter: {other:?}")),
    }
    for (engine, img) in [("t1 image", t1_image(src)), ("rust twin", rust_twin(src))] {
        let mut m = Machine::load_elf(&img, yantra::ram_for(&img)).expect("the image loads");
        let mut out = Vec::new();
        let h = m.run(200_000_000, &mut out);
        println!("{what}: {engine}, yantra {h:?}");
        if !matches!(h, Halt::Finisher { status: Some(s), .. } if s == want) {
            bad.push(format!("{engine} on yantra: {h:?}"));
        }
        let q = qemu_leg::run(&img);
        println!("{what}: {engine}, qemu {q:?}");
        let exit = format!("exit status: {want}");
        if !matches!(&q, Err(e) if e.contains(&exit)) {
            bad.push(format!("{engine} on qemu: {q:?}"));
        }
    }
    assert!(
        bad.is_empty(),
        "{what} must return {want} on every engine:\n{}",
        bad.join("\n")
    );
}

/// `src`'s entry is REFUSED with `code` on every engine: the interpreter's
/// refusal carries the code (`0x…`), both images halt `yantra` on the finisher's
/// FAIL form with `code` as the status and print nothing, and QEMU exits with
/// the code's low octet.
fn refuses_everywhere(what: &str, src: &str, code: u64) -> u64 {
    let mut bad = Vec::new();
    let interp = interpreted(src);
    println!("{what}: interpreter {interp:?}");
    match &interp {
        Err(e) if e.contains(&format!("{code:#x}")) => {}
        other => bad.push(format!("interpreter: {other:?}")),
    }
    for (engine, img) in [("t1 image", t1_image(src)), ("rust twin", rust_twin(src))] {
        let mut m = Machine::load_elf(&img, yantra::ram_for(&img)).expect("the image loads");
        let mut out = Vec::new();
        let h = m.run(200_000_000, &mut out);
        println!("{what}: {engine}, yantra {h:?}");
        if h != (Halt::Finisher {
            value: (code << 16) | 0x3333,
            status: Some(code),
        }) || !out.is_empty()
        {
            bad.push(format!("{engine} on yantra: {h:?}, printed {out:?}"));
        }
        let q = qemu_leg::run(&img);
        println!("{what}: {engine}, qemu {q:?}");
        let exit = format!("exit status: {}", code & 0xff);
        if !matches!(&q, Err(e) if e.contains(&exit)) {
            bad.push(format!("{engine} on qemu: {q:?}"));
        }
    }
    assert!(
        bad.is_empty(),
        "{what} must be refused with {code:#x} on every engine:\n{}",
        bad.join("\n")
    );
    code
}

// ── wrapping ───────────────────────────────────────────────────────────────────

/// `b1 p14`: `अ६४` MAX `योगः १` wraps to MIN, which is below ०.
#[test]
fn w381_s3_signed_add_wraps_past_max() {
    returns_everywhere("p14", P14, 1);
}

/// `b1 p21`: २^६२ `गुणनम्` ८ is २^६५, whose low 64 bits are ०.
#[test]
fn w381_s3_multiply_keeps_the_low_sixty_four_bits() {
    returns_everywhere("p21", P21, 1);
}

/// (२^६४−१)² ≡ १ (mod २^६४): a product whose exact value does not fit even 128
/// bits — the interpreter must wrap, not overflow its own wide integer.
#[test]
fn w381_s3_multiply_of_two_all_ones_words_is_one() {
    let all_ones = dec(u64::MAX);
    let mul = tok(P21, 4, 6);
    let eq = tok(P21, 5, 2);
    let src = derive(
        P21,
        &[
            (
                &format!("{} {} {}", dec(1), tok(P21, 3, 6), dec(62)),
                &all_ones,
            ),
            (&format!("{mul} {}", dec(8)), &format!("{mul} {all_ones}")),
            (&format!("{eq} {}", dec(0)), &format!("{eq} {}", dec(1))),
        ],
    );
    returns_everywhere("all-ones squared", &src, 1);
}

/// RV64's defined overflow quotient: ऋण२^६३ `विभाजनम्` ऋण१ is ऋण२^६३, below ०.
#[test]
fn w381_s3_min_divided_by_minus_one_is_min() {
    let div = tok(P19, 5, 6);
    let add = tok(P14, 4, 3);
    let src = derive(
        P14,
        &[
            (&dec(i64::MAX as u64), &negative(1 << 63)),
            (
                &format!("{add} {}", dec(1)),
                &format!("{div} {}", negative(1)),
            ),
        ],
    );
    returns_everywhere("MIN / -1", &src, 1);
}

// ── shift counts ───────────────────────────────────────────────────────────────

/// `b1 p17`: the count is taken modulo ६४, so `१ वामसृ ६४` is १.
#[test]
fn w381_s3_shift_left_by_sixty_four_is_by_zero() {
    returns_everywhere("p17", P17, 1);
}

/// And `१ वामसृ ६५` is २ — the count is masked, not saturated.
#[test]
fn w381_s3_shift_left_by_sixty_five_is_by_one() {
    let src = derive(
        P17,
        &[(&format!(" {} ", dec(64)), &format!(" {} ", dec(65)))],
    );
    returns_everywhere("1 << 65", &src, 2);
}

/// `b1 p18` with both right shifts by ६४: the logical one (`न६४`) and the
/// arithmetic one (`अ६४`) each shift by ० and return their operand, so `ब` is
/// ऋण१ and the answer is all-ones `योगः १०` = ९.
#[test]
fn w381_s3_right_shifts_by_sixty_four_are_by_zero() {
    let shr = tok(P18, 5, 6);
    let src = derive(
        P18,
        &[
            (
                &format!("{} {shr} {}", tok(P18, 5, 5), dec(63)),
                &format!("{} {shr} {}", tok(P18, 5, 5), dec(64)),
            ),
            (
                &format!("{} {shr} {}", tok(P18, 6, 5), dec(63)),
                &format!("{} {shr} {}", tok(P18, 6, 5), dec(64)),
            ),
        ],
    );
    returns_everywhere("right shifts by 64", &src, 9);
}

// ── the unsigned compare ───────────────────────────────────────────────────────

/// `b1 p15`: a `न६४` holding २^६३ is NOT below ०.
#[test]
fn w381_s3_unsigned_name_below_zero_is_false() {
    returns_everywhere("p15", P15, 2);
}

/// `b1 p16`: ० `वियोगः` १ wraps to २^६४−१, which is not below ० — the row both
/// engines answered wrongly, and alike, before this stage.
#[test]
fn w381_s3_wrapped_underflow_is_not_below_zero() {
    returns_everywhere("p16", P16, 2);
}

/// `W-373` (f): `न६४` MAX `अधिकम्` १०० is TRUE.
#[test]
fn w381_s3_unsigned_max_is_greater_than_a_hundred() {
    let lt = tok(P15, 4, 2);
    let src = derive(
        P15,
        &[
            (
                &format!("{} {} {}", dec(1), tok(P15, 3, 6), dec(63)),
                &dec(u64::MAX),
            ),
            (&format!("{lt} {}", dec(0)), &format!("{GT} {}", dec(100))),
        ],
    );
    returns_everywhere("MAX > 100", &src, 1);
}

/// `बृहत्समम्` over the same २^६३: unsigned, so TRUE.
#[test]
fn w381_s3_unsigned_name_at_least_zero_is_true() {
    let lt = tok(P15, 4, 2);
    let src = derive(
        P15,
        &[(&format!("{lt} {}", dec(0)), &format!("{GE} {}", dec(0)))],
    );
    returns_everywhere("2^63 >= 0", &src, 1);
}

/// The unsigned name on the RIGHT: `० अधिकम् क` over २^६३ is FALSE unsigned
/// (and would be TRUE signed).
#[test]
fn w381_s3_unsigned_name_on_the_right_is_unsigned_too() {
    let k = tok(P15, 4, 1);
    let lt = tok(P15, 4, 2);
    let src = derive(
        P15,
        &[(
            &format!("{k} {lt} {}", dec(0)),
            &format!("{} {GT} {k}", dec(0)),
        )],
    );
    returns_everywhere("0 > 2^63", &src, 2);
}

/// CONTROL: two numerals are compared SIGNED — the rule is about a NAME — so
/// ऋण१ `न्यूनम्` ० holds.
#[test]
fn w381_s3_a_numeral_comparison_stays_signed() {
    let k = tok(P16, 5, 1);
    let lt = tok(P16, 5, 2);
    let src = derive(
        P16,
        &[(
            &format!("{k} {lt} {}", dec(0)),
            &format!("{} {lt} {}", negative(1), dec(0)),
        )],
    );
    returns_everywhere("-1 < 0, numerals", &src, 1);
}

/// `समम्` compares the 64-bit WORD: ० `वियोगः` १ equals २^६४−१.
#[test]
fn w381_s3_equality_is_of_the_sixty_four_bit_word() {
    let lt = tok(P16, 5, 2);
    let eq = tok(P21, 5, 2);
    let src = derive(
        P16,
        &[(
            &format!("{lt} {}", dec(0)),
            &format!("{eq} {}", dec(u64::MAX)),
        )],
    );
    returns_everywhere("0 - 1 == 2^64 - 1", &src, 1);
}

// ── division and remainder by zero (owner ruling (b)) ──────────────────────────

/// `b1 p19`: `७ विभाजनम् ०` through a name is REFUSED on both engines (its names
/// are unsigned; the signed pair follows).
#[test]
fn w381_s3_division_by_zero_is_refused() {
    // The `0x35e` witness (`refusal_cause_witnesses.rs`).
    assert_eq!(refuses_everywhere("p19", P19, DIV_ZERO), DIV_ZERO);
}

/// The remainder by zero likewise.
#[test]
fn w381_s3_remainder_by_zero_is_refused() {
    let div = tok(P19, 5, 6);
    let src = derive(P19, &[(div, REM)]);
    refuses_everywhere("7 % 0", &src, DIV_ZERO);
}

/// A literal ० divisor is still tested (only a NON-ZERO numeral skips the test).
#[test]
fn w381_s3_division_by_a_literal_zero_is_refused() {
    let div = tok(P19, 5, 6);
    let src = derive(
        P19,
        &[(
            &format!("{div} {}", tok(P19, 5, 7)),
            &format!("{div} {}", dec(0)),
        )],
    );
    refuses_everywhere("7 / literal 0", &src, DIV_ZERO);
}

/// `p19` with its three names declared `अ६४` (signed) instead of `न६४`.
fn p19_signed(src: &str) -> String {
    let (u, i) = ("ॱॱ न६४", "ॱॱ अ६४");
    assert_eq!(src.matches(u).count(), 3, "p19 declares three names\n{src}");
    src.replace(u, i)
}

/// The division by zero over SIGNED names is refused too.
#[test]
fn w381_s3_signed_division_by_zero_is_refused() {
    refuses_everywhere("signed 7 / 0", &p19_signed(P19), DIV_ZERO);
}

/// And the signed remainder by zero.
#[test]
fn w381_s3_signed_remainder_by_zero_is_refused() {
    let div = tok(P19, 5, 6);
    let src = p19_signed(&derive(P19, &[(div, REM)]));
    refuses_everywhere("signed 7 % 0", &src, DIV_ZERO);
}

/// CONTROL: a non-zero numeral divisor divides: ७ ÷ ७ = १, and `ल योगः १` is २.
#[test]
fn w381_s3_division_by_a_nonzero_literal_divides() {
    let div = tok(P19, 5, 6);
    let src = derive(
        P19,
        &[(
            &format!("{div} {}", tok(P19, 5, 7)),
            &format!("{div} {}", dec(7)),
        )],
    );
    returns_everywhere("7 / literal 7", &src, 2);
}

// ── the checked built-ins ──────────────────────────────────────────────────────

/// `p17` with `क वामसृ श` replaced by a checked call over `क` and `श`, and `क`'s
/// value set to `k`: the entry returns that call's value.
fn p17_checked(f: &str, k: u64) -> String {
    let shl = format!("{} {} {}", tok(P17, 5, 5), tok(P17, 5, 6), tok(P17, 5, 7));
    let init = format!("{} {} ।", tok(P17, 3, 4), dec(1));
    imported(&derive(
        P17,
        &[
            (&init, &format!("{} {} ।", tok(P17, 3, 4), dec(k))),
            (&shl, &call(f, tok(P17, 5, 5), tok(P17, 5, 7))),
        ],
    ))
}

#[test]
fn w381_s3_checked_add_adds() {
    returns_everywhere("checked 1 + 64", &p17_checked(CHECKED_ADD, 1), 65);
}

#[test]
fn w381_s3_checked_sub_subtracts() {
    returns_everywhere("checked 100 - 64", &p17_checked(CHECKED_SUB, 100), 36);
}

#[test]
fn w381_s3_checked_mul_multiplies() {
    returns_everywhere("checked 3 * 64", &p17_checked(CHECKED_MUL, 3), 192);
}

/// A checked shift by ६४ is refused (ruling (a)).
#[test]
fn w381_s3_checked_shift_by_sixty_four_is_refused() {
    refuses_everywhere("checked 1 << 64", &p17_checked(CHECKED_SHL, 1), OVERFLOW);
}

/// A checked shift by ६३ that pushes a one out is NOT refused: ३ << ६३ is
/// २^६३, read back by the logical shift of a `न६४` name as १.
#[test]
fn w381_s3_checked_shift_does_not_refuse_shifted_out_bits() {
    let shl = tok(P21, 3, 6);
    let eq = tok(P21, 5, 2);
    let shr = tok(P18, 5, 6);
    let src = imported(&derive(
        P21,
        &[
            (
                &format!("{} {shl} {}", dec(1), dec(62)),
                &call(CHECKED_SHL, &dec(3), &dec(63)),
            ),
            (
                &format!("{} {}", tok(P21, 4, 6), dec(8)),
                &format!("{shr} {}", dec(63)),
            ),
            (&format!("{eq} {}", dec(0)), &format!("{eq} {}", dec(1))),
        ],
    ));
    returns_everywhere("checked 3 << 63", &src, 1);
}

/// `p14` with `क योगः १` replaced by `f` over `a` and `b`, and `क`'s first value
/// replaced by `init`.
fn p14_checked(f: &str, init: &str, a: &str, b: &str) -> String {
    let add = format!("{} {} {}", tok(P14, 4, 2), tok(P14, 4, 3), tok(P14, 4, 4));
    imported(&derive(
        P14,
        &[(&dec(i64::MAX as u64), init), (&add, &call(f, a, b))],
    ))
}

#[test]
fn w381_s3_checked_add_refuses_max_plus_one() {
    let k = tok(P14, 3, 1);
    // The `0x35c` witness (`refusal_cause_witnesses.rs`).
    let code = refuses_everywhere(
        "checked MAX + 1",
        &p14_checked(CHECKED_ADD, &dec(i64::MAX as u64), k, &dec(1)),
        OVERFLOW,
    );
    assert_eq!(code, OVERFLOW);
}

#[test]
fn w381_s3_checked_sub_refuses_min_minus_one() {
    let k = tok(P14, 3, 1);
    refuses_everywhere(
        "checked MIN - 1",
        &p14_checked(CHECKED_SUB, &negative(1 << 63), k, &dec(1)),
        OVERFLOW,
    );
}

#[test]
fn w381_s3_checked_mul_refuses_max_times_two() {
    let k = tok(P14, 3, 1);
    refuses_everywhere(
        "checked MAX * 2",
        &p14_checked(CHECKED_MUL, &dec(i64::MAX as u64), k, &dec(2)),
        OVERFLOW,
    );
}

/// ऋण१ × MIN is २^६३, one past MAX: the case a quotient test alone misses.
#[test]
fn w381_s3_checked_mul_refuses_minus_one_times_min() {
    let k = tok(P14, 3, 1);
    refuses_everywhere(
        "checked -1 * MIN",
        &p14_checked(CHECKED_MUL, &negative(1 << 63), &negative(1), k),
        OVERFLOW,
    );
}

/// CONTROL: MIN × १ is MIN, below ०, and not refused.
#[test]
fn w381_s3_checked_mul_of_min_by_one_is_min() {
    let k = tok(P14, 3, 1);
    returns_everywhere(
        "checked MIN * 1",
        &p14_checked(CHECKED_MUL, &negative(1 << 63), k, &dec(1)),
        1,
    );
}

// ── owner rulings 2026-10-06 (a) and (b) ───────────────────────────────────────

/// Ruling (a): `b1 p20`, ३०० stored into an octet run keeps its low eight bits,
/// ४४, on every engine (the interpreter used to refuse it).
#[test]
fn w381_s3_an_octet_store_keeps_the_low_eight_bits() {
    returns_everywhere("p20", P20, 44);
}

/// `p21` with `क` = २^६४−१ (a `न६४` name), `गुणनम् ८` replaced by `op` over a
/// numeral, and the `समम्` test against `want`: १ when the quotient or remainder
/// is `want`.
fn p21_unsigned(op: &str, by: u64, want: u64) -> String {
    let eq = tok(P21, 5, 2);
    derive(
        P21,
        &[
            (
                &format!("{} {} {}", dec(1), tok(P21, 3, 6), dec(62)),
                &dec(u64::MAX),
            ),
            (
                &format!("{} {}", tok(P21, 4, 6), dec(8)),
                &format!("{op} {}", dec(by)),
            ),
            (&format!("{eq} {}", dec(0)), &format!("{eq} {}", dec(want))),
        ],
    )
}

/// Ruling (b): over a name declared `न६४`, `विभाजनम्` is `divu` — MAX ÷ १० is
/// १८४४६७४४०७३७०९५५१६१, not the signed reading's ०.
#[test]
fn w381_s3_unsigned_division_of_max_by_ten() {
    let div = tok(P19, 5, 6);
    returns_everywhere("MAX / 10", &p21_unsigned(div, 10, u64::MAX / 10), 1);
}

/// And `शेषः` is `remu` — MAX % १० is ५, not the signed reading's ऋण१.
#[test]
fn w381_s3_unsigned_remainder_of_max_by_ten() {
    returns_everywhere("MAX % 10", &p21_unsigned(REM, 10, u64::MAX % 10), 1);
}

/// The unsigned name as the DIVISOR only: १०० ÷ (२^६४−१) is ० unsigned (the
/// signed reading is ऋण१००), so `ल योगः १` is १.
#[test]
fn w381_s3_an_unsigned_divisor_alone_divides_unsigned() {
    let div = tok(P19, 5, 6);
    let src = derive(
        P19,
        &[
            (
                &format!("{} {} ।", tok(P19, 4, 4), dec(0)),
                &format!("{} {} ।", tok(P19, 4, 4), dec(u64::MAX)),
            ),
            (
                &format!("{} {div} {}", tok(P19, 5, 5), tok(P19, 5, 7)),
                &format!("{} {div} {}", dec(100), tok(P19, 5, 7)),
            ),
        ],
    );
    returns_everywhere("100 / MAX", &src, 1);
}

/// CONTROL: over a SIGNED name the division stays signed: ऋण७ ÷ २ is ऋण३, below ०.
#[test]
fn w381_s3_a_signed_division_stays_signed() {
    let div = tok(P19, 5, 6);
    let add = format!("{} {} {}", tok(P14, 4, 2), tok(P14, 4, 3), tok(P14, 4, 4));
    let src = derive(
        P14,
        &[
            (&dec(i64::MAX as u64), &negative(7)),
            (&add, &format!("{} {div} {}", tok(P14, 4, 2), dec(2))),
        ],
    );
    returns_everywhere("-7 / 2 signed", &src, 1);
}

// The ÷० refusal still comes first over an unsigned name (p19's are `न६४`):
// asserted by `w381_s3_division_by_zero_is_refused` and its remainder twin.

// ── owner rulings 2026-10-06: narrow stores and loads, every width ─────────────

/// A run of `ty` (`अङ्कः अन्तः ty`) written `stored` at index ० and read back
/// into an `अ६४` name `ल`, which must EQUAL `want`: १ if it does, २ if not.
/// Built from `p20` (the run, the store) and `p21` (the test and returns).
fn narrow_round_trip(ty: &str, stored: &str, want: &str) -> String {
    let octet = tok(P20, 3, 5);
    let i64_name = tok(P14, 3, 3);
    let l = tok(P21, 4, 1);
    let read = P20.lines().nth(5).expect("p20's return line");
    let (ret, index) = read.trim_start().split_once(' ').expect("a return");
    let let_line = format!(
        "    {} {l} {} {i64_name} {} {index}",
        tok(P14, 3, 0),
        tok(P14, 3, 2),
        tok(P14, 3, 4)
    );
    let tail: Vec<&str> = P21.lines().skip(5).collect();
    let tail = tail.join("\n").replace(
        &format!("{} {}", tok(P21, 5, 2), dec(0)),
        &format!("{} {want}", tok(P21, 5, 2)),
    );
    let head = derive(P20, &[(octet, ty), (&dec(300), stored)]);
    let head = head.replace(read, &let_line);
    let body_end = head.rfind(tok(P20, 6, 0)).expect("p20's closing word");
    assert!(ret.starts_with(tok(P21, 6, 0)));
    format!("{}{tail}\n", &head[..body_end])
}

/// The five narrow element types: `न८ न१६ न३२` from the interpreter's own
/// unsigned list, and `अ१६ अ३२` made from them with `अ६४`'s signed letter.
fn narrow(bits: u32, signed: bool) -> String {
    let u = sadhana::t1::nirvahana::UNSIGNED_INTEGER_TYPES
        .iter()
        .find(|t| t.ends_with(&dec(u64::from(bits))))
        .expect("an unsigned width");
    if signed {
        let a = tok(P14, 3, 3).chars().next().expect("अ");
        format!("{a}{}", u.chars().skip(1).collect::<String>())
    } else {
        (*u).to_string()
    }
}

fn num(n: i128) -> String {
    if n < 0 {
        negative(n.unsigned_abs() as u64)
    } else {
        dec(n as u64)
    }
}

/// Every width × signedness, store then load, with a value wider than the
/// element and with a negative one: the low bits are kept on the store
/// (ruling: truncate), and the load zero-extends an unsigned element and
/// sign-extends a signed one (ruling: `lhu`/`lwu` for `न१६`/`न३२`).
#[test]
fn w381_s3_narrow_runs_round_trip_every_width_and_signedness() {
    let wide: i128 = (1 << 32) + (1 << 15) + 44;
    let wide31: i128 = (1 << 32) + (1 << 31) + 44;
    let cases: &[(u32, bool, i128, i128)] = &[
        (8, false, 300, 44),
        (8, false, -1, 255),
        (16, false, wide, 32812),
        (16, true, wide, 32812 - 65536),
        (16, false, -5, 65531),
        (16, true, -5, -5),
        (32, false, wide31, (1 << 31) + 44),
        (32, true, wide31, (1 << 31) + 44 - (1 << 32)),
        (32, false, -5, (1 << 32) - 5),
        (32, true, -5, -5),
    ];
    let mut bad = Vec::new();
    for &(bits, signed, stored, want) in cases {
        let ty = narrow(bits, signed);
        let src = narrow_round_trip(&ty, &num(stored), &num(want));
        let what = format!("{ty} <- {stored}, want {want}");
        if std::panic::catch_unwind(|| returns_everywhere(&what, &src, 1)).is_err() {
            bad.push(what);
        }
    }
    assert!(
        bad.is_empty(),
        "narrow round trips that do not agree: {bad:#?}"
    );
}

/// A 32-bit MASK with bit 31 set (`0xfff00000`, encode.t1's `आवरण`) stored into
/// an UNSIGNED `न३२` run reads back as ४२९३९१८७२० — the unsigned value, as the
/// raw field holds it — on the interpreter, both images and QEMU (`sw` then
/// `lwu`). An `अ३२` run would give ऋण१०४८५७६ and never equal the field: the
/// `encode.t1` defect (E11/E15/E16 never fired) fixed by retyping `पूरितानि`.
#[test]
fn w381_s3_a_mask_with_bit_31_round_trips_through_an_unsigned_run() {
    let mask = dec(0xfff0_0000);
    let src = narrow_round_trip(&narrow(32, false), &mask, &mask);
    returns_everywhere("न३२ <- 0xfff00000", &src, 1);
}
