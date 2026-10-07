//! **`V-005`: 64-BIT IEEE-754 FLOATS IN T1, ON BOTH ENGINES.**
//!
//! The owner's locked shape (2026-10-03): CALL-FORM BUILT-INS with no grammar
//! change, named with the tree's own float words (`spec/encodings-riscv64.tsv`),
//! values in the FLOAT REGISTERS natively, and RISC-V's canonical NaN
//! (`0x7ff8000000000000`) from the interpreter for every NaN result.
//!
//! A built-in is a qualified call into `अष्टक`, exactly as the output channel
//! `अष्टकॱमुद्रणम्` is, and is recognised by its EXACT qualified name on both
//! sides: `nirvahana.rs`'s `float_builtin` and `ir.t1`'s call arm.
//!
//! EVERY NUMERAL BELOW IS GENERATED FROM A `u64` BY [`hex`], never typed in
//! Devanagari, so a probe cannot carry a transcription slip the assertion then
//! agrees with.
//!
//! The native side is compiled HERE, by the current `.t1` compiler running in the
//! interpreter (`CHAIN`), so the image carries the lowering this tree says.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::{Halt, Machine};

mod qemu_leg;

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

/// A 64-bit pattern as a T1 numeral: `०षोड्` and the hex digits (`grammar-t1.ebnf`
/// `hex_digit`: the decimal digits, then `अ`..`ऊ` for a..f). A pattern with the
/// top bit set is written as the NEGATIVE of its two's complement, `ऋण०षोड्…`,
/// because a numeral's magnitude is read as a signed 64-bit value.
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
    let digits = format!("{mag:x}");
    for c in digits.chars() {
        s.push_str(DIGITS[c.to_digit(16).unwrap() as usize]);
    }
    format!("{}०षोड्{s}", if neg { "ऋण" } else { "" })
}

/// A letter suffix for the `k`th generated name (from ०). Names carry no digits:
/// a digit inside a word is not an `aksara` the lexer keeps in the word.
fn letter(k: usize) -> &'static str {
    const L: [&str; 20] = [
        "क", "ख", "ग", "घ", "च", "छ", "ज", "झ", "ट", "ठ", "ड", "ढ", "त", "थ", "द", "ध", "ब", "भ",
        "म", "ल",
    ];
    L[k]
}

/// A decimal numeral for a small integer.
fn dec(n: u64) -> String {
    const DIGITS: [&str; 10] = ["०", "१", "२", "३", "४", "५", "६", "७", "८", "९"];
    n.to_string()
        .chars()
        .map(|c| DIGITS[c.to_digit(10).unwrap() as usize])
        .collect()
}

fn bits(x: f64) -> u64 {
    x.to_bits()
}

const CANONICAL_NAN: u64 = 0x7ff8_0000_0000_0000;

/// The routine every probe prints a 64-bit answer with: eight little-endian
/// octets through the output channel.
///
/// WHY NOT THE EXIT STATUS: the startup reports the entry's result to the
/// finisher as `0x3333 | status << 16`, so the status channel keeps only the
/// LOW 48 BITS — `0x3FD3333333333334` came back as `0x333333333334`. A bit
/// pattern of a double needs all sixty-four, so it is printed and the entry
/// answers ०.
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

/// THE ASSEMBLER'S OWN REFUSAL, by name: `सङ्केतन`'s first diagnostic of the
/// last object it was asked for (`अन्तिमसङ्केतनदोषः` — its code from
/// `spec/diagnostics.tsv`, its line and its reason), or `None` when it refused
/// nothing. A fresh interpreter per build, so the record is this probe's.
fn encode_site(it: &Interpreter) -> Option<String> {
    // Module globals are keyed by their bare name (`chain.rs` reads
    // `यन्त्रनिषेधभेद` the same way).
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

// ── falsifier (a): 0.1 + 0.2, bit-moved in and out ───────────────────────────

fn probe_a() -> String {
    format!(
        "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

{PRINTER}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {a} ।
    चरः ख ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {b} ।
    चरः ग ॱॱ प६४ भवति अष्टकॱप्लवयोगः आरभ्य क ऽ ख समाप्तम् ।
    चरः फलम् ॱॱ न६४ भवति अष्टकॱप्लवसंचारः ग ।
    चरः अवगणना ॱॱ न६४ भवति प्लवमुद्रणम् फलम् ।
    प्रत्यागमनम् ० ।
इति
",
        a = hex(0x3FB9_9999_9999_999A),
        b = hex(0x3FC9_9999_9999_999A),
    )
}

const A_WANT: u64 = 0x3FD3_3333_3333_3334;

// ── falsifiers (b) and (c): every built-in once, printed as eight octets each ──

/// One case: its name, the T1 statement lines computing `फलम्` (a `न६४` bit
/// pattern) and the bits both engines must print.
struct Case {
    name: &'static str,
    body: String,
    want: u64,
}

fn cases() -> Vec<Case> {
    let one = hex(bits(1.0));
    let tenth = hex(0x3FB9_9999_9999_999A);
    let fifth = hex(0x3FC9_9999_9999_999A);
    vec![
        Case {
            name: "(b) +inf plus -inf is the canonical NaN",
            body: format!(
                "चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {} ।
    चरः ख ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {} ।
    चरः ग ॱॱ प६४ भवति अष्टकॱप्लवयोगः आरभ्य क ऽ ख समाप्तम् ।
    फलम् भवति अष्टकॱप्लवसंचारः ग ।",
                hex(bits(f64::INFINITY)),
                hex(bits(f64::NEG_INFINITY))
            ),
            want: CANONICAL_NAN,
        },
        Case {
            name: "sub: 1.0 - 0.1",
            body: format!(
                "चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {one} ।
    चरः ख ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {tenth} ।
    चरः ग ॱॱ प६४ भवति अष्टकॱप्लववियोगः आरभ्य क ऽ ख समाप्तम् ।
    फलम् भवति अष्टकॱप्लवसंचारः ग ।"
            ),
            want: 0x3FEC_CCCC_CCCC_CCCD,
        },
        Case {
            name: "mul: 0.1 * 3.0",
            body: format!(
                "चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {tenth} ।
    चरः ख ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {} ।
    चरः ग ॱॱ प६४ भवति अष्टकॱप्लवगुणनम् आरभ्य क ऽ ख समाप्तम् ।
    फलम् भवति अष्टकॱप्लवसंचारः ग ।",
                hex(bits(3.0))
            ),
            want: 0x3FD3_3333_3333_3334,
        },
        Case {
            // Fused: 0.1's representation error times ten survives, 2^-54. An
            // unfused multiply then add rounds 1.0000000000000000555 to 1.0 and
            // answers +0.0 — the two lowerings are told apart by this case.
            name: "fmadd: 0.1 * 10.0 + -1.0, rounded once",
            body: format!(
                "चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {tenth} ।
    चरः ख ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {} ।
    चरः ग ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {} ।
    चरः घ ॱॱ प६४ भवति अष्टकॱप्लवगुणयोगः आरभ्य क ऽ ख ऽ ग समाप्तम् ।
    फलम् भवति अष्टकॱप्लवसंचारः घ ।",
                hex(bits(10.0)),
                hex(bits(-1.0))
            ),
            want: 0x3C90_0000_0000_0000,
        },
        Case {
            name: "div: 1.0 / 3.0",
            body: format!(
                "चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {one} ।
    चरः ख ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {} ।
    चरः ग ॱॱ प६४ भवति अष्टकॱप्लवभागः आरभ्य क ऽ ख समाप्तम् ।
    फलम् भवति अष्टकॱप्लवसंचारः ग ।",
                hex(bits(3.0))
            ),
            want: 0x3FD5_5555_5555_5555,
        },
        Case {
            name: "sqrt: sqrt(2.0)",
            body: format!(
                "चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {} ।
    चरः ग ॱॱ प६४ भवति अष्टकॱप्लववर्गमूलम् क ।
    फलम् भवति अष्टकॱप्लवसंचारः ग ।",
                hex(bits(2.0))
            ),
            want: 0x3FF6_A09E_667F_3BCD,
        },
        Case {
            name: "sqrt(-1.0) is the canonical NaN",
            body: format!(
                "चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {} ।
    चरः ग ॱॱ प६४ भवति अष्टकॱप्लववर्गमूलम् क ।
    फलम् भवति अष्टकॱप्लवसंचारः ग ।",
                hex(bits(-1.0))
            ),
            want: CANONICAL_NAN,
        },
        Case {
            // 1: 0.1 < 0.2; 2 (absent): NaN == NaN; 4: -0 <= +0; 8: -0 == +0;
            // 16 (absent): 0.2 < 0.1.
            name: "compare: flt, feq on NaN, fle and feq on signed zeros",
            body: format!(
                "चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {tenth} ।
    चरः ख ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {fifth} ।
    चरः ज ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {nan} ।
    चरः च ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {negzero} ।
    चरः छ ॱॱ प६४ भवति अष्टकॱप्लवसंचारः ० ।
    यदि अष्टकॱप्लवन्यूनम् आरभ्य क ऽ ख समाप्तम् आदि
        फलम् भवति फलम् योगः १ ।
    इति
    यदि अष्टकॱप्लवसमम् आरभ्य ज ऽ ज समाप्तम् आदि
        फलम् भवति फलम् योगः २ ।
    इति
    यदि अष्टकॱप्लवानधिकम् आरभ्य च ऽ छ समाप्तम् आदि
        फलम् भवति फलम् योगः ४ ।
    इति
    यदि अष्टकॱप्लवसमम् आरभ्य च ऽ छ समाप्तम् आदि
        फलम् भवति फलम् योगः ८ ।
    इति
    यदि अष्टकॱप्लवन्यूनम् आरभ्य ख ऽ क समाप्तम् आदि
        फलम् भवति फलम् योगः १६ ।
    इति",
                nan = hex(CANONICAL_NAN),
                negzero = hex(bits(-0.0)),
            ),
            want: 13,
        },
        Case {
            name: "convert: integer 3 to a float",
            body: "चरः ग ॱॱ प६४ भवति अष्टकॱप्लवरूपान्तरम् ३ ।
    फलम् भवति अष्टकॱप्लवसंचारः ग ।"
                .to_string(),
            want: 0x4008_0000_0000_0000,
        },
        Case {
            name: "convert: integer -7 to a float",
            body: "चरः क ॱॱ अ६४ भवति ऋण७ ।
    चरः ग ॱॱ प६४ भवति अष्टकॱप्लवरूपान्तरम् क ।
    फलम् भवति अष्टकॱप्लवसंचारः ग ।"
                .to_string(),
            want: 0xC01C_0000_0000_0000,
        },
        Case {
            name: "convert: 2.5 to an integer rounds to even, 2",
            body: format!(
                "चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {} ।
    फलम् भवति अष्टकॱप्लवरूपान्तरम् क ।",
                hex(bits(2.5))
            ),
            want: 2,
        },
        Case {
            name: "convert: -3.5 to an integer rounds to even, -4",
            body: format!(
                "चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {} ।
    फलम् भवति अष्टकॱप्लवरूपान्तरम् क ।",
                hex(bits(-3.5))
            ),
            want: (-4i64) as u64,
        },
        Case {
            name: "convert: NaN to an integer saturates to i64::MAX",
            body: format!(
                "चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {} ।
    फलम् भवति अष्टकॱप्लवरूपान्तरम् क ।",
                hex(CANONICAL_NAN)
            ),
            want: i64::MAX as u64,
        },
    ]
}

/// One routine per case, each answering its `फलम्`, and `मुख्यम्` printing every
/// answer as eight little-endian octets through the output channel.
fn probe_cases() -> String {
    let mut s = format!(
        "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

{PRINTER}"
    );
    let all = cases();
    for (i, c) in all.iter().enumerate() {
        s.push_str(&format!(
            "
वृत्तिः प्लवप्रश्न{n} ददाति न६४ आदि
    चरः फलम् ॱॱ न६४ भवति ० ।
    {body}
    प्रत्यागमनम् फलम् ।
इति
",
            n = letter(i),
            body = c.body
        ));
    }
    s.push_str(
        "
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
",
    );
    for i in 0..all.len() {
        s.push_str(&format!(
            "    अवगणना भवति प्लवमुद्रणम् प्लवप्रश्न{} ।\n",
            letter(i)
        ));
    }
    s.push_str("    प्रत्यागमनम् ० ।\nइति\n");
    s
}

// ── falsifier (d)'s subject: thirteen floats live across a call ──────────────

/// `प्लवाह्वेयम्` adds the floats 1..13 with all thirteen live at once and
/// answers the BITS of the sum; `मुख्यम्` loads its own thirteen (101..113), and
/// with all of them live calls `प्लवाह्वेयम्` from the innermost argument, then
/// adds. Twelve `fs` registers hold twelve; the thirteenth value is SPILLED, and
/// travels through the float store and load or the answer is wrong. The callee
/// using the float file too is what makes the caller's values worth checking:
/// its prologue must save the `fs` registers it takes.
fn probe_live() -> String {
    fn nest(names: &[String], innermost: &str) -> String {
        match names.split_first() {
            None => innermost.to_string(),
            Some((first, rest)) => format!(
                "अष्टकॱप्लवयोगः आरभ्य {first} ऽ {} समाप्तम्",
                nest(rest, innermost)
            ),
        }
    }
    let lets = |base: u64| -> (String, Vec<String>) {
        let mut text = String::new();
        let mut names = Vec::new();
        for k in 1..=13u64 {
            let n = format!("धारक{}", letter(k as usize - 1));
            text.push_str(&format!(
                "    चरः {n} ॱॱ प६४ भवति अष्टकॱप्लवरूपान्तरम् {} ।\n",
                dec(base + k)
            ));
            names.push(n);
        }
        (text, names)
    };
    let (callee_lets, callee_names) = lets(0);
    let (main_lets, main_names) = lets(100);
    let zero = "अष्टकॱप्लवरूपान्तरम् ०".to_string();
    let callee_sum = nest(&callee_names, &zero);
    // BRACKETED, because the self-hosted parser curries: `f g ०` is `(f g) ०`, a
    // two-argument call, and only ARITY tells it from `f (g ०)` (`parse.t1`'s
    // W-187 margin). The interpreter's parser reads arity and would accept the
    // bare form; the native front end would not, so the probe says which.
    let main_sum = nest(&main_names, "अष्टकॱप्लवसंचारः आरभ्य प्लवाह्वेयम् ० समाप्तम्");
    format!(
        "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

{PRINTER}
वृत्तिः प्लवाह्वेयम् आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि
{callee_lets}    चरः योगफलम् ॱॱ प६४ भवति {callee_sum} ।
    प्रत्यागमनम् अष्टकॱप्लवसंचारः योगफलम् ।
इति

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
{main_lets}    चरः योगफलम् ॱॱ प६४ भवति {main_sum} ।
    चरः अवगणना ॱॱ न६४ भवति प्लवमुद्रणम् आरभ्य अष्टकॱप्लवसंचारः योगफलम् समाप्तम् ।
    प्रत्यागमनम् ० ।
इति
"
    )
}

/// 101 + … + 113 = 1,391, plus the callee's 1 + … + 13 = 91.
fn live_want() -> u64 {
    bits(1391.0 + 91.0)
}

// ── the two engines ──────────────────────────────────────────────────────────

/// Run `मुख्यम्` interpreted, with the chain loaded beside the probe as
/// `t1_image --load` does. Answers the result and the octets printed.
fn interpret(src: &str) -> (Value, Vec<u8>) {
    let file = format!("{MODULE}.t1");
    let mut srcs: Vec<(&str, &str)> = CHAIN.to_vec();
    srcs.push((file.as_str(), src));
    let mut it = Interpreter::load(&srcs, &spec_root())
        .unwrap_or_else(|e| panic!("the probe must load: {}", e.reason));
    let v = it
        .call(&format!("{MODULE}\u{971}मुख्यम्"), Vec::new(), FUEL)
        .unwrap_or_else(|e| panic!("the probe must run interpreted: {}", e.reason));
    (v, it.sink().to_vec())
}

/// Compile `src` with the given compiler sources and run the image natively.
fn native_with(chain: &[(&str, &str)], src: &str) -> (Halt, Vec<u8>) {
    let image = t1_image_with(chain, src);
    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let h = m.run(200_000_000, &mut out);
    (h, out)
}

/// Compile `src` with the given compiler sources: the image `native_with` runs.
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
        "the probe built no image: refusal {:?}, encoder {:?}, link {:?}",
        sadhana::t1::chain::refusal_site(&it),
        encode_site(&it),
        sadhana::t1::chain::link_refusals(&it)
    );
    image
}

fn native(src: &str) -> (Halt, Vec<u8>) {
    native_with(CHAIN, src)
}

/// THE RUST TWIN: the same `.t1` front end (`Front`, which drives `ir.t1`), then
/// `riscv64.rs`'s emitter, the Rust assembler and the Rust linker — the five
/// calls `t1_whole_chain.rs`'s twin test names — and the image run on yantra.
/// Answers the image too, so a caller can compare it with the `.t1` side's.
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

fn rust_twin(src: &str) -> (Halt, Vec<u8>) {
    let image = rust_twin_image(src);
    let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let h = m.run(200_000_000, &mut out);
    (h, out)
}

fn status(h: &Halt) -> u64 {
    match h {
        Halt::Finisher {
            status: Some(s), ..
        } => *s,
        other => panic!("the image did not finish with a status: {other:?}"),
    }
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

fn assert_cases(engine: &str, out: &[u8]) {
    let all = cases();
    let got = words(out);
    assert_eq!(
        got.len(),
        all.len(),
        "{engine}: {} words printed for {} cases ({} octets)",
        got.len(),
        all.len(),
        out.len()
    );
    let mut wrong = Vec::new();
    for (c, g) in all.iter().zip(&got) {
        if *g != c.want {
            wrong.push(format!(
                "  {}: got {g:#018x}, want {:#018x}",
                c.name, c.want
            ));
        }
    }
    assert!(wrong.is_empty(), "{engine}:\n{}", wrong.join("\n"));
}

/// The ONE word a probe printed, after a clean finish (status ०). A probe
/// answers through the output channel, never the status (see [`PRINTER`]).
fn one_word(engine: &str, h: &Halt, out: &[u8]) -> u64 {
    assert_eq!(
        status(h),
        0,
        "{engine}: the probe must finish cleanly: {h:?}"
    );
    let w = words(out);
    assert_eq!(
        w.len(),
        1,
        "{engine}: one word printed, got {} octets",
        out.len()
    );
    w[0]
}

/// The same for the interpreter: the entry answers ० and the sink holds the word.
fn one_word_interpreted(v: &Value, out: &[u8]) -> u64 {
    assert_eq!(
        v.as_int(),
        Some(0),
        "the probe's entry answers ०, got {v:?}"
    );
    let w = words(out);
    assert_eq!(
        w.len(),
        1,
        "interpreted: one word printed, got {} octets",
        out.len()
    );
    w[0]
}

// ── the tests ───────────────────────────────────────────────────────────────

#[test]
fn v005_a_point_one_plus_point_two_interpreted() {
    let (v, out) = interpret(&probe_a());
    assert_eq!(
        one_word_interpreted(&v, &out),
        A_WANT,
        "0.1 + 0.2 interpreted must be 0x3FD3333333333334"
    );
}

#[test]
fn v005_a_point_one_plus_point_two_natively() {
    let (h, out) = native(&probe_a());
    assert_eq!(
        one_word("native", &h, &out),
        A_WANT,
        "0.1 + 0.2 natively must be 0x3FD3333333333334 (the end-to-end yantra \
         read-back moved here from V-004)"
    );
}

#[test]
fn v005_every_builtin_interpreted() {
    let (_, out) = interpret(&probe_cases());
    assert_cases("interpreted", &out);
}

#[test]
fn v005_every_builtin_natively_and_the_engines_agree() {
    let src = probe_cases();
    let (h, out) = native(&src);
    assert_eq!(status(&h), 0, "the probe must finish cleanly: {h:?}");
    assert_cases("native", &out);
    let (_, interpreted) = interpret(&src);
    assert_eq!(interpreted, out, "the two engines printed different octets");
}

#[test]
fn v005_thirteen_floats_live_across_a_call_interpreted() {
    let (v, out) = interpret(&probe_live());
    assert_eq!(
        one_word_interpreted(&v, &out),
        live_want(),
        "1482.0 interpreted"
    );
}

#[test]
fn v005_thirteen_floats_live_across_a_call_natively() {
    let (h, out) = native(&probe_live());
    assert_eq!(
        one_word("native", &h, &out),
        live_want(),
        "1482.0 natively: thirteen float values live across a call, one of them \
         spilled through the float store and load"
    );
}

// ── the Rust twin (`riscv64.rs`), over the same three probes ─────────────────

#[test]
fn v005_a_point_one_plus_point_two_through_the_rust_twin() {
    let (h, out) = rust_twin(&probe_a());
    assert_eq!(
        one_word("rust twin", &h, &out),
        A_WANT,
        "0.1 + 0.2 through riscv64.rs must be 0x3FD3333333333334"
    );
}

#[test]
fn v005_every_builtin_through_the_rust_twin() {
    let (h, out) = rust_twin(&probe_cases());
    assert_eq!(status(&h), 0, "the probe must finish cleanly: {h:?}");
    assert_cases("rust twin", &out);
}

#[test]
fn v005_thirteen_floats_live_across_a_call_through_the_rust_twin() {
    let (h, out) = rust_twin(&probe_live());
    assert_eq!(
        one_word("rust twin", &h, &out),
        live_want(),
        "1482.0 through riscv64.rs: one float value spilled through the float \
         store and load"
    );
}

/// `V-009` part (i-b): THE FLOAT IMAGE ON QEMU. Every built-in's probe, compiled
/// by both emitters, runs on `qemu-system-riscv64` (`-bios none`, M-mode, FS
/// `Off` at reset) and prints the octets `yantra` prints. Before the startup set
/// `sstatus.FS`, the first float instruction was an illegal-instruction trap.
#[test]
fn v009_the_float_image_runs_on_qemu_and_prints_what_yantra_prints() {
    let src = probe_cases();
    for (engine, image) in [
        (".t1 chain", t1_image_with(CHAIN, &src)),
        ("rust twin", rust_twin_image(&src)),
    ] {
        let mut m = Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
        let mut ours: Vec<u8> = Vec::new();
        let h = m.run(200_000_000, &mut ours);
        assert_eq!(status(&h), 0, "{engine}: yantra must finish cleanly: {h:?}");
        let theirs = qemu_leg::run(&image).unwrap_or_else(|e| panic!("{engine}: {e}"));
        assert_cases(&format!("{engine} on qemu"), &theirs);
        assert_eq!(
            theirs, ours,
            "{engine}: qemu and yantra printed different octets"
        );
    }
}

/// THE TWIN, OCTET FOR OCTET: the `.t1` emitter and `riscv64.rs` must build the
/// same loadable segment for every probe (`W-236`'s contract, now over the
/// float file too). Compared as `t1_whole_chain.rs` compares — the PT_LOAD
/// segment and `e_entry`, not the file, whose section headers differ by design.
#[test]
fn v005_the_two_emitters_build_the_same_segment() {
    fn seg(img: &[u8]) -> (u64, Vec<u8>) {
        let ph_off = u64::from_le_bytes(img[32..40].try_into().unwrap()) as usize;
        let p_offset =
            u64::from_le_bytes(img[ph_off + 8..ph_off + 16].try_into().unwrap()) as usize;
        let p_filesz =
            u64::from_le_bytes(img[ph_off + 32..ph_off + 40].try_into().unwrap()) as usize;
        let entry = u64::from_le_bytes(img[24..32].try_into().unwrap());
        (entry, img[p_offset..p_offset + p_filesz].to_vec())
    }
    for (name, src) in [
        ("a", probe_a()),
        ("cases", probe_cases()),
        ("live", probe_live()),
    ] {
        let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
        it.call(
            "शृङ्खलाॱप्रवेशन्यासः",
            vec![octets(MODULE.as_bytes()), octets("मुख्यम्".as_bytes())],
            1_000_000_000,
        )
        .expect("the entry is named");
        let mine = it
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
        assert!(!mine.is_empty(), "{name}: no .t1 image");
        let theirs = rust_twin_image(&src);
        let (me, ms) = seg(&mine);
        let (re, rs) = seg(&theirs);
        assert_eq!(
            me, re,
            "{name}: the two images enter at different addresses"
        );
        if ms != rs {
            let at = ms
                .iter()
                .zip(rs.iter())
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| ms.len().min(rs.len()));
            panic!(
                "{name}: the loadable segments differ — .t1 {} octets, Rust {} \
                 octets, first difference at {at}",
                ms.len(),
                rs.len()
            );
        }
    }
}

// ── the mutants ──────────────────────────────────────────────────────────────

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

/// What a mutated chain did with a probe: the words a CLEAN run printed, or the
/// reason it did not finish cleanly — a refused build (the panic's message) or
/// a halt other than status ०. Printed by every mutant test, because a red that
/// is a REFUSAL and a red that is a WRONG ANSWER are different evidence, and a
/// reader of the run is owed which one it was.
fn native_mutated(chain: &[(&'static str, String)], src: &str) -> Result<Vec<u64>, String> {
    let refs: Vec<(&str, &str)> = chain.iter().map(|(n, s)| (*n, s.as_str())).collect();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| native_with(&refs, src)));
    match r {
        Ok((
            Halt::Finisher {
                status: Some(0), ..
            },
            out,
        )) => Ok(words(&out)),
        Ok((h, _)) => Err(format!("halted {h:?}")),
        Err(p) => Err(p
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_string()))
            .unwrap_or_else(|| "a panic with no message".to_string())),
    }
}

/// A mutant that must stop at the ASSEMBLER, by the named diagnostic `code`
/// (`spec/diagnostics.tsv`) — not merely "no image". Mutants 3 and 4 put a
/// float register where the `.t1` encoder's integer form expects an `x` one,
/// and the encoder lays the line out at one length and emits it at another:
/// `E23`, layout and emission disagree. The record carries the code and no
/// arguments (its line reads ०), which is said rather than hidden — the CODE is
/// the named refusal, and a red reached by a missing entry symbol alone no
/// longer passes these tests.
fn assert_encoder_refused(mutant: &str, got: &Result<Vec<u64>, String>, code: &str) {
    match got {
        Err(e) => assert!(
            e.contains(&format!("encoder Some(\"{code}")),
            "{mutant}: must be refused by the encoder's {code}, was: {e}"
        ),
        Ok(w) => panic!("{mutant}: ran and printed {w:x?}"),
    }
}

fn report(mutant: &str, got: &Result<Vec<u64>, String>) {
    match got {
        Ok(w) => println!(
            "{mutant}: built and ran cleanly, printed {:x?} (a WRONG ANSWER is the red)",
            w
        ),
        Err(e) => println!(
            "{mutant}: did not run cleanly (a REFUSAL is the red): {}",
            e.chars().take(400).collect::<String>()
        ),
    }
}

/// MUTANT 1: the float add lowered to the INTEGER add kind. Must not answer
/// 0x3FD3333333333334.
#[test]
fn v005_mutant_float_add_as_integer_add_is_red() {
    let chain = mutated_chain("ir.t1", MUTANT_ADD_LIVE, MUTANT_ADD_DEAD);
    let got = native_mutated(&chain, &probe_a());
    report("float add as integer add", &got);
    assert!(
        matches!(&got, Err(e) if e.contains("FileMismatch")),
        "float add as integer add: must be refused by the emitters' FileMismatch, was {got:?}"
    );
    assert_ne!(
        got,
        Ok(vec![A_WANT]),
        "a float add lowered as an integer add must not produce 0.1 + 0.2"
    );
}

/// MUTANT 2: the built-in matched by KIND only — any qualified `अष्टक` call of
/// the float family taken for the add. Must not reproduce every case.
#[test]
fn v005_mutant_builtin_matched_by_kind_only_is_red() {
    let chain = mutated_chain("ir.t1", MUTANT_KIND_LIVE, MUTANT_KIND_DEAD);
    let got = native_mutated(&chain, &probe_cases());
    report("built-in matched by kind only", &got);
    assert!(
        matches!(&got, Err(e) if e.contains("FileMismatch")),
        "built-in matched by kind only: must be refused by the emitters' FileMismatch, was {got:?}"
    );
    let want: Vec<u64> = cases().iter().map(|c| c.want).collect();
    assert_ne!(
        got,
        Ok(want),
        "a built-in matched by kind only must not reproduce every case"
    );
}

/// MUTANT 3: a spilled float value read back with the INTEGER load. The
/// thirteen-live probe must not answer 1482.0.
#[test]
fn v005_mutant_float_spill_with_integer_load_is_red() {
    let chain = mutated_chain("yantrotsarjana.t1", MUTANT_SPILL_LIVE, MUTANT_SPILL_DEAD);
    let got = native_mutated(&chain, &probe_live());
    report("float spill with the integer load", &got);
    assert_encoder_refused("float spill with the integer load", &got, "E23");
    assert_ne!(
        got,
        Ok(vec![live_want()]),
        "a float spill read with the integer load must not produce the sum"
    );
}

/// THE SPILL IS REAL: the thirteen-live probe's two float routines each need
/// at least one FLOAT spill slot, by the allocator `riscv64.rs` runs. Without it the spill
/// mutant above could pass by never reaching the path it mutates; with it, and
/// with the two emitters' segments identical, the `.t1` side spills too.
#[test]
fn v005_the_live_probe_spills_a_float() {
    use sadhana::t1::chain::Front;
    use sadhana::t1::regalloc::{RegClass, allocate_registers_for};
    use sadhana::t1::riscv64;
    let mut front = Front::load(&spec_root()).expect("Front loads");
    front.lex(&probe_live()).expect("lex");
    front.parse().expect("parse");
    front.resolve().expect("resolve");
    front.typecheck().expect("typecheck");
    front.build_ir().expect("build_ir");
    let module = front
        .module(MODULE, Some("मुख्यम्"))
        .expect("the module builds");
    let classify =
        |_: sadhana::t1::ir::ValueId, i: &sadhana::t1::ir::Instruction| riscv64::value_class(i);
    let spills: Vec<usize> = module
        .functions
        .iter()
        .map(|f| {
            allocate_registers_for(
                f,
                riscv64::allocatable(RegClass::Float),
                RegClass::Float,
                &classify,
            )
            .num_spills
        })
        .collect();
    println!("float spills per routine: {spills:?}");
    // Three routines: the printer holds no float, and `प्लवाह्वेयम्` and
    // `मुख्यम्` each hold thirteen live at once against twelve fs registers.
    assert_eq!(
        spills.iter().filter(|s| **s >= 1).count(),
        2,
        "the callee and the entry must each spill a float: {spills:?}"
    );
}

// The mutation sites, quoted from the `.t1` sources (each must match once).
//
// MUTANT 1 lowers every float op to the INTEGER add kind at the one appender
// they are all built through, so the add becomes `योगः` over two bit patterns.
const MUTANT_ADD_LIVE: &str = "आज्ञायोजनम् आरभ्य प्लवाज्ञाभेद ऽ फलम् ऽ ० ऽ ० ऽ वाम ऽ दक्षिण ऽ ० समाप्तम् ।";
const MUTANT_ADD_DEAD: &str = "आज्ञायोजनम् आरभ्य योगाज्ञाभेद ऽ फलम् ऽ ० ऽ ० ऽ वाम ऽ दक्षिण ऽ ० समाप्तम् ।";
// MUTANT 2 matches the add by the callee's MODULE alone: any `अष्टक` member is
// taken for `प्लवयोगः` whatever its name, which is what matching the intrinsic
// family by kind rather than by name amounts to.
const MUTANT_KIND_LIVE: &str = "यदि सदस्यपाठ समम् उक्तम् प्लवयोगः इति आदि प्रत्यागमनम् प्लवयोगोपभेद । इति";
const MUTANT_KIND_DEAD: &str = "यदि सदस्यपाठ ॱ दैर्घ्य अधिकम् ० आदि प्रत्यागमनम् प्लवयोगोपभेद । इति";
// MUTANT 3 reloads a spilled float with the INTEGER load.
const MUTANT_SPILL_LIVE: &str = "उत्सर्जनॱपाठयोजनम् उक्तम् प्लवाहारः इति ।";
const MUTANT_SPILL_DEAD: &str = "उत्सर्जनॱपाठयोजनम् उक्तम् आहारः इति ।";

// ── what this row does NOT do yet, pinned as a REFUSAL ───────────────────────

/// A float in an integer place, refused on both emitters BY NAME.
fn assert_file_mismatch_refused(what: &str, src: &str) {
    let chain: Vec<(&'static str, String)> =
        CHAIN.iter().map(|(n, s)| (*n, (*s).to_string())).collect();
    let t1 = native_mutated(&chain, src);
    report(&format!("{what}, .t1 emitter"), &t1);
    match &t1 {
        Err(e) => assert!(
            e.contains("FileMismatch"),
            "{what}: the .t1 emitter must refuse with FileMismatch, refused: {e}"
        ),
        Ok(w) => panic!("{what}: the .t1 emitter built and ran it, printing {w:x?}"),
    }
    let rust = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rust_twin(src)));
    let why = match &rust {
        Ok(_) => String::from("it ran"),
        Err(p) => p.downcast_ref::<String>().cloned().unwrap_or_default(),
    };
    println!(
        "{what}, riscv64.rs: {}",
        why.chars().take(300).collect::<String>()
    );
    assert!(
        why.contains("FileMismatch"),
        "{what}: riscv64.rs must refuse with FileMismatch, got: {why}"
    );
}

/// A FLOAT STORED INTO AN INTEGER RUN'S ELEMENT IS REFUSED, BY NAME, ON EVERY
/// ENGINE. Under V-005 a float through memory was refused natively by the
/// emitters' `Refusal::FileMismatch` while the INTERPRETER STORED IT — a known
/// divergence this test's margin recorded. `V-008` part 1 (owner ruling O1,
/// 2026-10-05) stores floats in memory, a `प६४` slot taking only a float and any
/// other slot never one: this run is `अङ्कः अन्तः न६४`, so the float is now
/// refused by `ir.t1` (shape ५), the interpreter and the Rust twin alike. The
/// stores that work are `v008_float_memory.rs`'s.
#[test]
fn v005_a_float_stored_through_memory_is_refused_by_name() {
    assert_refused_everywhere(
        "a float stored into a run element",
        &[(
            MODULE,
            format!(
                "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ प६४ भवति अष्टकॱप्लवरूपान्तरम् ३ ।
    चरः र ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    र अङ्कः ० अन्तः भवति क ।
    प्रत्यागमनम् ० ।
इति
"
            ),
        )],
    );
}

// ── (1) the `fa` calling convention ─────────────────────────────────────────

/// The routine every ABI probe calls: three floats and an integer, the
/// integer BETWEEN floats so the two files' counters must be independent;
/// answers `fmadd(a, b, c) + float(n)`, computed from all four.
fn mixed_routine(public: bool) -> String {
    format!(
        "{}वृत्तिः मिश्रम् आदाय प्रथमः ॱॱ प६४ ऽ पूर्णः ॱॱ न६४ ऽ द्वितीयः ॱॱ प६४ ऽ तृतीयः ॱॱ प६४ ददाति प६४ आदि
    चरः गुणफलम् ॱॱ प६४ भवति अष्टकॱप्लवगुणयोगः आरभ्य प्रथमः ऽ द्वितीयः ऽ तृतीयः समाप्तम् ।
    प्रत्यागमनम् अष्टकॱप्लवयोगः आरभ्य गुणफलम् ऽ अष्टकॱप्लवरूपान्तरम् पूर्णः समाप्तम् ।
इति
",
        if public { "सार्वजनिक " } else { "" }
    )
}

/// `मुख्यम्`'s body for the mixed call. (An integer handed to a `प६४`
/// parameter is a `FileMismatch` refusal, not a bit move — see the refusal
/// tests below.)
fn mixed_calls(callee: &str) -> String {
    format!(
        "    चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {a} ।
    चरः ख ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {b} ।
    चरः ग ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {c} ।
    चरः फलम् ॱॱ प६४ भवति {callee} आरभ्य क ऽ ७ ऽ ख ऽ ग समाप्तम् ।
    चरः अवगणना ॱॱ न६४ भवति प्लवमुद्रणम् आरभ्य अष्टकॱप्लवसंचारः फलम् समाप्तम् ।
",
        a = hex(bits(1.5)),
        b = hex(bits(2.25)),
        c = hex(bits(0.125)),
    )
}

fn mixed_want() -> Vec<u64> {
    vec![bits(1.5f64.mul_add(2.25, 0.125) + 7.0)]
}

fn probe_abi_local() -> String {
    format!(
        "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

{PRINTER}
{}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
{}    प्रत्यागमनम् ० ।
इति
",
        mixed_routine(false),
        mixed_calls("मिश्रम्")
    )
}

const HELPER: &str = "प्लवसहाय";

fn probe_abi_helper() -> String {
    format!(
        "मण्डलम् {HELPER} ॥
आयातः अष्टक ।

{}",
        mixed_routine(true)
    )
}

fn probe_abi_cross() -> String {
    format!(
        "मण्डलम् {MODULE} ॥
आयातः अष्टक ।
आयातः {HELPER} ।

{PRINTER}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
{}    प्रत्यागमनम् ० ।
इति
",
        mixed_calls(&format!("{HELPER}\u{971}मिश्रम्"))
    )
}

/// NINE floats and NINE integers, alternating, so the ninth of EACH goes to the
/// ONE shared stack (`abi.rs`): the float first, at slot ०, the integer at
/// slot ८. Answers the floats' sum plus the integers' sum as a float.
fn probe_abi_nine() -> String {
    let mut params = Vec::new();
    for k in 1..=9 {
        params.push(format!("धक{} ॱॱ प६४", letter(k - 1)));
        params.push(format!("नक{} ॱॱ न६४", letter(k - 1)));
    }
    let mut fsum = "अष्टकॱप्लवरूपान्तरम् पूर्णयोगः".to_string();
    for k in (1..=9).rev() {
        fsum = format!("अष्टकॱप्लवयोगः आरभ्य धक{} ऽ {fsum} समाप्तम्", letter(k - 1));
    }
    let isum = (1..=9)
        .map(|k| format!("नक{}", letter(k - 1)))
        .collect::<Vec<_>>()
        .join(" योगः ");
    let mut args = Vec::new();
    for k in 1..=9u64 {
        args.push(hex(bits(0.5 * k as f64)));
        args.push(dec(k));
    }
    // the floats travel as bit-moved values, so name them first
    let mut lets = String::new();
    let mut call_args = Vec::new();
    for (i, a) in args.iter().enumerate() {
        if i % 2 == 0 {
            let n = format!("मूल{}", letter(i / 2));
            lets.push_str(&format!("    चरः {n} ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {a} ।\n"));
            call_args.push(n);
        } else {
            call_args.push(a.clone());
        }
    }
    format!(
        "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

{PRINTER}
वृत्तिः नवयोगः आदाय {} ददाति प६४ आदि
    चरः पूर्णयोगः ॱॱ न६४ भवति {isum} ।
    प्रत्यागमनम् {fsum} ।
इति

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
{lets}    चरः फलम् ॱॱ प६४ भवति नवयोगः आरभ्य {} समाप्तम् ।
    चरः अवगणना ॱॱ न६४ भवति प्लवमुद्रणम् आरभ्य अष्टकॱप्लवसंचारः फलम् समाप्तम् ।
    प्रत्यागमनम् ० ।
इति
",
        params.join(" ऽ "),
        call_args.join(" ऽ ")
    )
}

/// 0.5 + 1.0 + … + 4.5 = 22.5, plus 1 + … + 9 = 45.
fn nine_want() -> Vec<u64> {
    vec![bits(22.5 + 45.0)]
}

/// Interpreted, with every given source loaded beside the chain.
fn interpret_all(srcs: &[(&str, String)]) -> (Value, Vec<u8>) {
    let files: Vec<(String, &str)> = srcs
        .iter()
        .map(|(m, s)| (format!("{m}.t1"), s.as_str()))
        .collect();
    let mut all: Vec<(&str, &str)> = CHAIN.to_vec();
    for (f, s) in &files {
        all.push((f.as_str(), s));
    }
    let mut it = Interpreter::load(&all, &spec_root())
        .unwrap_or_else(|e| panic!("the probe must load: {}", e.reason));
    let v = it
        .call(&format!("{MODULE}\u{971}मुख्यम्"), Vec::new(), FUEL)
        .unwrap_or_else(|e| panic!("the probe must run interpreted: {}", e.reason));
    (v, it.sink().to_vec())
}

/// Natively through the `.t1` chain, every given module compiled together.
fn native_all_with(chain: &[(&str, &str)], srcs: &[(&str, String)]) -> (Halt, Vec<u8>) {
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
    run_image(&image)
}

fn run_image(image: &[u8]) -> (Halt, Vec<u8>) {
    let mut m = Machine::load_elf(image, yantra::ram_for(image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let h = m.run(200_000_000, &mut out);
    (h, out)
}

/// The Rust twin over several modules in one `Front`, every module gathered
/// before any is lowered, so caller-first and callee-first build the same.
fn rust_twin_all_image(srcs: &[(&str, String)]) -> Vec<u8> {
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
    // COLLECT FIRST, COMPILE SECOND: every module's declarations are in the
    // store before any module is lowered, so the ORDER the modules are given in
    // cannot change a call site's view of its callee (`Front::gather`).
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

/// One ABI probe on all three engines, the words each printed, and the two
/// emitters' segments compared.
fn assert_abi(what: &str, srcs: &[(&str, String)], want: &[u64]) {
    let (v, out) = interpret_all(srcs);
    assert_eq!(v.as_int(), Some(0), "{what}: interpreted entry answers ०");
    assert_eq!(words(&out), want, "{what}: interpreted");
    let (h, out) = native_all_with(CHAIN, srcs);
    assert_eq!(status(&h), 0, "{what}: native finish {h:?}");
    assert_eq!(words(&out), want, "{what}: native (.t1)");
    let rust_image = rust_twin_all_image(srcs);
    let (h, out) = run_image(&rust_image);
    assert_eq!(status(&h), 0, "{what}: rust twin finish {h:?}");
    assert_eq!(words(&out), want, "{what}: rust twin");
    // and the segments
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets(MODULE.as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    let mine = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(srcs.iter().map(|(_, s)| octets(s.as_bytes())).collect()),
                arena(srcs.iter().map(|(m, _)| octets(m.as_bytes())).collect()),
                Value::Int(srcs.len() as i128),
            ],
            FUEL,
        )
        .expect("runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    let (me, ms) = segment(&mine);
    let (re, rs) = segment(&rust_image);
    assert_eq!(me, re, "{what}: entries differ");
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

#[test]
fn v005_fa_three_floats_and_an_int_within_a_module() {
    assert_abi(
        "within a module",
        &[(MODULE, probe_abi_local())],
        &mixed_want(),
    );
}

#[test]
fn v005_fa_three_floats_and_an_int_across_modules() {
    assert_abi(
        "across modules",
        &[(HELPER, probe_abi_helper()), (MODULE, probe_abi_cross())],
        &mixed_want(),
    );
}

#[test]
fn v005_fa_nine_floats_and_nine_ints_share_the_stack() {
    assert_abi("nine and nine", &[(MODULE, probe_abi_nine())], &nine_want());
}

/// MUTANT 4: every argument and parameter placed as an INTEGER — the float
/// handed over in an `अर्थ` register. Must not reproduce the answers.
#[test]
fn v005_mutant_float_passed_in_an_int_register_is_red() {
    let chain = mutated_chain("yantrotsarjana.t1", MUTANT_FA_LIVE, MUTANT_FA_DEAD);
    let got = native_mutated(&chain, &probe_abi_local());
    report("float passed in an int register", &got);
    assert_encoder_refused("float passed in an int register", &got, "E23");
    assert_ne!(
        got,
        Ok(mixed_want()),
        "a float in an int register must not answer right"
    );
}
const MUTANT_FA_LIVE: &str = "        यदि यन्त्रवर्गक्रमकोश अङ्कः क्रमः अन्तः समम् १ आदि";
const MUTANT_FA_DEAD: &str = "        यदि यन्त्रवर्गक्रमकोश अङ्कः क्रमः अन्तः समम् ९ आदि";

// ── (2) the spaced spelling, on every engine ─────────────────────────────────

/// `अष्टक ॱ प्लवयोगः`, spaced: the same name as `अष्टकॱप्लवयोगः`, and both
/// engines must take it — the interpreter's parser always did; `ir.t1` now
/// reads the folded member token as well as the one-token text.
#[test]
fn v005_the_spaced_spelling_agrees_on_every_engine() {
    let src = probe_a().replace(
        "अष्टकॱप्लवयोगः आरभ्य क ऽ ख समाप्तम्",
        "अष्टक ॱ प्लवयोगः आरभ्य क ऽ ख समाप्तम्",
    );
    assert!(src.contains("अष्टक ॱ प्लवयोगः"), "the probe spells it spaced");
    assert_abi("the spaced spelling", &[(MODULE, src)], &[A_WANT]);
}

#[test]
fn v005_fa_across_modules_caller_first() {
    assert_abi(
        "across modules, caller first",
        &[(MODULE, probe_abi_cross()), (HELPER, probe_abi_helper())],
        &mixed_want(),
    );
}

// ── FileMismatch: one named cause, all three engines ────────────────────────

/// The interpreter's run of a probe: `Ok` with its words, or the refusal text.
fn interpret_result(srcs: &[(&str, String)]) -> Result<Vec<u64>, String> {
    let files: Vec<(String, &str)> = srcs
        .iter()
        .map(|(m, s)| (format!("{m}.t1"), s.as_str()))
        .collect();
    let mut all: Vec<(&str, &str)> = CHAIN.to_vec();
    for (f, s) in &files {
        all.push((f.as_str(), s));
    }
    let mut it = Interpreter::load(&all, &spec_root()).map_err(|e| e.reason)?;
    it.call(&format!("{MODULE}\u{971}मुख्यम्"), Vec::new(), FUEL)
        .map_err(|e| e.reason)?;
    Ok(words(it.sink()))
}

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<String>()
        .cloned()
        .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_default()
}

/// A probe every engine must REFUSE with `FileMismatch`: the interpreter by its
/// run error, the `.t1` native build by `refusal_site` (`ir: FileMismatch …`),
/// and the Rust twin by `Front::build_ir`'s error.
fn assert_refused_everywhere(what: &str, srcs: &[(&str, String)]) {
    let interp = interpret_result(srcs);
    println!("{what}, interpreter: {interp:?}");
    match &interp {
        Err(e) => assert!(
            e.contains("FileMismatch"),
            "{what}: interpreter refused otherwise: {e}"
        ),
        Ok(w) => panic!("{what}: the interpreter ran it, printing {w:x?}"),
    }
    let t1 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        native_all_with(CHAIN, srcs)
    }));
    let t1 = t1.map(|_| ()).map_err(panic_text);
    println!("{what}, .t1: {t1:?}");
    match &t1 {
        Err(e) => assert!(
            e.contains("FileMismatch"),
            "{what}: .t1 refused otherwise: {e}"
        ),
        Ok(()) => panic!("{what}: the .t1 emitter built it"),
    }
    let rust = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rust_twin_all_image(srcs)));
    let rust = rust.map(|_| ()).map_err(panic_text);
    println!("{what}, rust: {rust:?}");
    match &rust {
        Err(e) => assert!(
            e.contains("FileMismatch"),
            "{what}: riscv64.rs refused otherwise: {e}"
        ),
        Ok(()) => panic!("{what}: the Rust twin built it"),
    }
}

fn one_routine(body: &str) -> Vec<(&'static str, String)> {
    vec![(
        MODULE,
        format!(
            "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

{PRINTER}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
{body}    प्रत्यागमनम् ० ।
इति
"
        ),
    )]
}

/// THE REVIEW'S BLOCKER: a float bound to an INTEGER local, then integer
/// arithmetic on it. Before the ruling the interpreter refused the `योगः` and
/// both native emitters silently computed `0x4008000000000001`.
#[test]
fn v005_a_float_bound_to_an_integer_local_is_refused_everywhere() {
    assert_refused_everywhere(
        "a float bound to an integer local",
        &one_routine(&format!(
            "    चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {} ।
    चरः पूर्णम् ॱॱ न६४ भवति क ।
    चरः वर्धितम् ॱॱ न६४ भवति पूर्णम् योगः १ ।
    चरः अवगणना ॱॱ न६४ भवति प्लवमुद्रणम् वर्धितम् ।
",
            hex(bits(3.0))
        )),
    );
}

#[test]
fn v005_an_integer_bound_to_a_float_local_is_refused_everywhere() {
    assert_refused_everywhere(
        "an integer bound to a float local",
        &one_routine(
            "    चरः क ॱॱ प६४ भवति ३ ।
    चरः अवगणना ॱॱ न६४ भवति प्लवमुद्रणम् आरभ्य अष्टकॱप्लवसंचारः क समाप्तम् ।
",
        ),
    );
}

#[test]
fn v005_a_float_assigned_to_an_integer_local_is_refused_everywhere() {
    assert_refused_everywhere(
        "a float assigned to an integer local",
        &one_routine(&format!(
            "    चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः {} ।
    चरः पूर्णम् ॱॱ न६४ भवति ० ।
    पूर्णम् भवति क ।
    चरः अवगणना ॱॱ न६४ भवति प्लवमुद्रणम् पूर्णम् ।
",
            hex(bits(3.0))
        )),
    );
}

#[test]
fn v005_an_integer_assigned_to_a_float_local_is_refused_everywhere() {
    assert_refused_everywhere(
        "an integer assigned to a float local",
        &one_routine(
            "    चरः क ॱॱ प६४ भवति अष्टकॱप्लवरूपान्तरम् २ ।
    क भवति ५ ।
    चरः अवगणना ॱॱ न६४ भवति प्लवमुद्रणम् आरभ्य अष्टकॱप्लवसंचारः क समाप्तम् ।
",
        ),
    );
}

/// AN ARGUMENT AGAINST ITS DECLARED PARAMETER: the integer ० handed to a `प६४`
/// parameter — the case both engines used to MOVE by its bits — and a float
/// handed to an integer one.
#[test]
fn v005_an_argument_in_the_wrong_file_is_refused_everywhere() {
    let mut srcs = vec![(MODULE, probe_abi_local())];
    srcs[0].1 = srcs[0].1.replace(
        "मिश्रम् आरभ्य क ऽ ७ ऽ ख ऽ ग समाप्तम्",
        "मिश्रम् आरभ्य क ऽ ७ ऽ ख ऽ ० समाप्तम्",
    );
    assert!(
        srcs[0].1.contains("ऽ ख ऽ ० समाप्तम्"),
        "the probe hands ० to a प६४"
    );
    assert_refused_everywhere("an integer to a float parameter", &srcs);
    let mut srcs = vec![(MODULE, probe_abi_local())];
    srcs[0].1 = srcs[0].1.replace(
        "मिश्रम् आरभ्य क ऽ ७ ऽ ख ऽ ग समाप्तम्",
        "मिश्रम् आरभ्य क ऽ ग ऽ ख ऽ ग समाप्तम्",
    );
    assert!(
        srcs[0].1.contains("ऽ ग ऽ ख ऽ ग समाप्तम्"),
        "the probe hands a float to न६४"
    );
    assert_refused_everywhere("a float to an integer parameter", &srcs);
}

/// A RETURNED VALUE AGAINST THE DECLARED RETURN TYPE: `प्रत्यागमनम् ०` in a
/// `ददाति प६४` routine — which both engines used to MOVE by its bits — and a
/// float returned from a `ददाति न६४` one.
#[test]
fn v005_a_return_in_the_wrong_file_is_refused_everywhere() {
    let probe = |routine: &str| {
        vec![(
            MODULE,
            format!(
                "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

{PRINTER}
{routine}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति प्लवमुद्रणम् प्लवत्रयम् ।
    प्रत्यागमनम् ० ।
इति
"
            ),
        )]
    };
    assert_refused_everywhere(
        "an integer returned from a प६४ routine",
        &probe(
            "वृत्तिः प्लवफलकम् ददाति प६४ आदि
    प्रत्यागमनम् ० ।
इति
वृत्तिः प्लवत्रयम् ददाति न६४ आदि
    प्रत्यागमनम् अष्टकॱप्लवसंचारः प्लवफलकम् ।
इति
",
        ),
    );
    assert_refused_everywhere(
        "a float returned from a न६४ routine",
        &probe(
            "वृत्तिः प्लवत्रयम् ददाति न६४ आदि
    चरः क ॱॱ प६४ भवति अष्टकॱप्लवरूपान्तरम् ३ ।
    प्रत्यागमनम् क ।
इति
",
        ),
    );
}

// ── the emitters' FileMismatch in its other shapes ──────────────────────────

/// A FLOAT INTO AN INTEGER FIELD OR GLOBAL, OR IN A CONDITION, refused by
/// `FileMismatch` on EVERY engine. **THE DIVERGENCE THIS MARGIN USED TO STATE IS
/// CLOSED** (`V-008` part 1): the interpreter stored the float into the field
/// and the global while both native emitters refused. Now the field and the
/// global are DECLARED `न६४`, and a float stored into either is refused by
/// `ir.t1` (shapes ४ and ६), the interpreter and the Rust twin. The branch
/// condition stays the two emitters' `FileMismatch` alone: the interpreter
/// refuses it too, but as an integer operator handed a float — its own V-005
/// message, not this cause, and not this row's to rename.
#[test]
fn v005_a_float_into_a_field_a_global_or_a_branch_compare_is_refused_by_name() {
    assert_refused_everywhere(
        "a float stored into a record field",
        &[(
            MODULE,
            format!(
                "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

संरचना धारकः आरभ्य
    मानम् ॱॱ न६४
समाप्तम् ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ प६४ भवति अष्टकॱप्लवरूपान्तरम् ३ ।
    चरः र ॱॱ धारकः भवति ० ।
    र ॱ मानम् भवति क ।
    प्रत्यागमनम् ० ।
इति
"
            ),
        )],
    );
    assert_refused_everywhere(
        "a float stored into a module-level global",
        &[(
            MODULE,
            format!(
                "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

चरः सञ्चितम् ॱॱ न६४ भवति ० ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ प६४ भवति अष्टकॱप्लवरूपान्तरम् ३ ।
    सञ्चितम् भवति क ।
    प्रत्यागमनम् ० ।
इति
"
            ),
        )],
    );
    // A BARE float condition (`यदि क`) never reaches an emitter: the type
    // checker refuses a non-बूल condition first. The shape that does reach one
    // is an INTEGER comparison of floats in a condition — a fused compare-and-
    // branch on two `f` registers — which the interpreter refuses too (its
    // integer operators refuse a float).
    assert_file_mismatch_refused(
        "a float compared by an integer operator in a branch condition",
        &format!(
            "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ प६४ भवति अष्टकॱप्लवरूपान्तरम् ३ ।
    चरः फलम् ॱॱ न६४ भवति ० ।
    यदि क समम् क आदि
        फलम् भवति १ ।
    इति
    प्रत्यागमनम् फलम् ।
इति
"
        ),
    );
}
