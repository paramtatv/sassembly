//! **`V-008` PART 2: THE ELEMENTWISE VECTOR BUILT-INS.**
//!
//! `व्यूहॱप्लवयोगः ( फल , क , ख )` sets `फल[i] = क[i] + ख[i]` for every `i`
//! below the length and answers the element count; `व्यूहॱप्लववियोगः`,
//! `व्यूहॱप्लवगुणनम्` and `व्यूहॱप्लवभागः` are its three siblings
//! (`docs/v008-vector-lowering-design.md`, ADR-0043 R3). The three arguments are
//! runs of `प६४`; their lengths must be equal, or the call is REFUSED — never
//! truncated — with the finisher word `0x35a` (`ir.t1`'s `व्यूहदैर्घ्यनिषेधः`).
//!
//! THE REFERENCE IS INDEPENDENT OF EVERY ENGINE: the expected bits are Rust's
//! own `f64` operation on the same input bits, one binary64 operation under
//! round-to-nearest-even per element, with a NaN result replaced by RISC-V's
//! canonical NaN (`0x7ff8000000000000`), which is what `vfadd.vv` and the scalar
//! `fadd.d` both write. The inputs include NaNs with payloads, a signalling
//! NaN, ±0, the smallest and largest subnormals, ±∞, and pairs whose result
//! overflows, underflows, or is an invalid operation.
//!
//! THE LENGTHS ARE THE DESIGN'S FALSIFIER: 0, 1, 5, 13, 16, 17 and 33. VLMAX at
//! SEW 64, LMUL m8 and VLEN 128 is 16, so 17 and 33 leave a tail strip, and 5
//! and 13 are not multiples of 2.
//!
//! THE INTERPRETER'S TWIN first, then THE NATIVE HALF: the strip-mined expansion
//! on yantra from both emitters, the two images octet for octet, the refusals
//! on both engines, and the three emitter mutants of the design's falsifier.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, VECTOR_LENGTH_REFUSAL};
use std::path::{Path, PathBuf};

mod qemu_leg;

const FUEL: u64 = 80_000_000_000;
const MODULE: &str = "व्यूहपरीक्षण";

/// The four member names, copied from `nirvahana.rs`'s `FLOAT_BUILTINS` table
/// (the V-005 float names) rather than typed here.
fn member(op: Op) -> &'static str {
    let fadd = match op {
        Op::Add => "fadd.d",
        Op::Sub => "fsub.d",
        Op::Mul => "fmul.d",
        Op::Div => "fdiv.d",
    };
    sadhana::t1::nirvahana::FLOAT_BUILTINS
        .iter()
        .find(|(_, _, _, m)| *m == fadd)
        .map(|(n, ..)| *n)
        .expect("the float table names the op")
}

fn qualified(op: Op) -> String {
    format!(
        "{}\u{971}{}",
        sadhana::t1::nirvahana::VECTOR_BUILTIN_MODULE,
        member(op)
    )
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

const OPS: [Op; 4] = [Op::Add, Op::Sub, Op::Mul, Op::Div];

const CANONICAL_NAN: u64 = 0x7ff8_0000_0000_0000;

/// The independent reference: one binary64 operation, a NaN canonicalised.
fn reference(op: Op, a: u64, b: u64) -> u64 {
    let (x, y) = (f64::from_bits(a), f64::from_bits(b));
    let r = match op {
        Op::Add => x + y,
        Op::Sub => x - y,
        Op::Mul => x * y,
        Op::Div => x / y,
    };
    if r.is_nan() {
        CANONICAL_NAN
    } else {
        r.to_bits()
    }
}

/// The input pool. Every special class the falsifier names, and ordinary
/// values whose sums and products round.
const POOL: [u64; 20] = [
    0x3FB9_9999_9999_999A, // 0.1
    0x3FC9_9999_9999_999A, // 0.2
    0x0000_0000_0000_0000, // +0
    0x8000_0000_0000_0000, // -0
    0x7FF0_0000_0000_0000, // +inf
    0xFFF0_0000_0000_0000, // -inf
    0x7FF8_DEAD_BEEF_0001, // quiet NaN with a payload
    0x7FF0_0000_0000_0001, // signalling NaN
    0x0000_0000_0000_0001, // smallest subnormal
    0x000F_FFFF_FFFF_FFFF, // largest subnormal
    0x8008_0000_0000_0000, // a negative subnormal
    0x0010_0000_0000_0000, // smallest normal
    0x7FEF_FFFF_FFFF_FFFF, // largest finite
    0xFFEF_FFFF_FFFF_FFFF, // most negative finite
    0x3FF0_0000_0000_0000, // 1
    0xC008_0000_0000_0000, // -3
    0x4009_21FB_5444_2D18, // pi
    0x3CB0_0000_0000_0000, // 2^-52
    0xBFE0_0000_0000_0001, // -0.5 - ulp
    0x4340_0000_0000_0001, // 2^53 + 2 (odd significand)
];

const NEG_ZERO: u64 = 0x8000_0000_0000_0000;
const POS_ZERO: u64 = 0;
const ONE: u64 = 0x3FF0_0000_0000_0000;
const HALF: u64 = 0x3FE0_0000_0000_0000;
const TWO: u64 = 0x4000_0000_0000_0000;
const MIN_SUB: u64 = 0x0000_0000_0000_0001;
const MAX_SUB: u64 = 0x000F_FFFF_FFFF_FFFF;
const MIN_NORMAL: u64 = 0x0010_0000_0000_0000;

/// The first elements are PAIRS CHOSEN FOR A RESULT CLASS, so even the short
/// lengths measure signed-zero and subnormal agreement: `-0 ± 0` and `-0 × 1`
/// give `-0` (or `+0`) by the sign rules; `min_sub + min_sub`, `max_sub − min_sub`,
/// `min_normal × ½` and `min_normal ÷ 2` give subnormals.
const HEAD: [(u64, u64); 6] = [
    (NEG_ZERO, NEG_ZERO),
    (NEG_ZERO, POS_ZERO),
    (NEG_ZERO, ONE),
    (MIN_SUB, MIN_SUB),
    (MAX_SUB, MIN_SUB),
    (MIN_NORMAL, HALF),
];

/// The two operands at element `i`: the chosen head, then `क` walking the pool
/// and `ख` walking it at another stride and offset, so every class meets
/// others (NaN payloads, infinities, overflow, cancellation).
fn operands(i: usize) -> (u64, u64) {
    if i < HEAD.len() {
        return HEAD[i];
    }
    if i == HEAD.len() {
        return (MIN_NORMAL, TWO);
    }
    (POOL[i % POOL.len()], POOL[(i * 7 + 3) % POOL.len()])
}

const LENGTHS: [usize; 7] = [0, 1, 5, 13, 16, 17, 33];

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// A 64-bit pattern as a T1 numeral (`v005_floats.rs`'s `hex`, copied).
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

/// One probe module, `अष्टक` imported and NOTHING ELSE: `व्यूहॱप्लवयोगः` needs no
/// import (owner ruling, option A) — `artha.t1`'s `व्यूहसदस्यः` admits exactly the
/// four vector built-ins, and `ir.t1` intercepts them by name.
fn module(body: &str) -> Vec<(&'static str, String)> {
    module_importing(&[], body)
}

/// The same module with `extra` modules imported too.
fn module_importing(extra: &[&str], body: &str) -> Vec<(&'static str, String)> {
    let imports: String = extra.iter().map(|m| format!("आयातः {m} ।\n")).collect();
    vec![(
        MODULE,
        format!(
            "मण्डलम् {MODULE} ॥
आयातः अष्टक ।
{imports}
{PRINTER}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    चरः घ ॱॱ प६४ भवति ० ।
{body}    प्रत्यागमनम् ० ।
इति
"
        ),
    )]
}

/// Lines that fill the `प६४` run `run` with `bits`, element by element.
fn fill(run: &str, bits: &[u64]) -> String {
    let mut s = String::new();
    for (i, b) in bits.iter().enumerate() {
        s.push_str(&format!(
            "    घ भवति अष्टकॱप्लवसंचारः {} ।\n    {run} अङ्कः {} अन्तः भवति घ ।\n",
            hex(*b),
            dec(i as u64)
        ));
    }
    s
}

/// Lines that print the count and then every element of `run`, `n` of them.
fn print_run(n: usize, run: &str) -> String {
    let mut s = String::from("    अवगणना भवति प्लवमुद्रणम् परिमाणम् ।\n");
    for i in 0..n {
        s.push_str(&format!(
            "    घ भवति {run} अङ्कः {} अन्तः ।\n    अवगणना भवति प्लवमुद्रणम् आरभ्य अष्टकॱप्लवसंचारः घ समाप्तम् ।\n",
            dec(i as u64)
        ));
    }
    s
}

/// `फल` (zeroed to the length), `क` and `ख` of length `n`; the call; the count
/// and every element of `फल` printed.
fn probe(op: Op, n: usize) -> Vec<(&'static str, String)> {
    let (a, b): (Vec<u64>, Vec<u64>) = (0..n).map(operands).unzip();
    let body = format!(
        "    चरः फल ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः क ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः प६४ भवति ० ।
{fz}{fa}{fb}    चरः परिमाणम् ॱॱ अ६४ भवति {call} आरभ्य फल ऽ क ऽ ख समाप्तम् ।
{print}",
        fz = fill("फल", &vec![0; n]),
        fa = fill("क", &a),
        fb = fill("ख", &b),
        call = qualified(op),
        print = print_run(n, "फल"),
    );
    module(&body)
}

fn want(op: Op, n: usize) -> Vec<u64> {
    let mut w = vec![n as u64];
    w.extend((0..n).map(|i| {
        let (a, b) = operands(i);
        reference(op, a, b)
    }));
    w
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

fn show(w: &[u64]) -> Vec<String> {
    w.iter().map(|x| format!("{x:#018x}")).collect()
}

// ── (1) the four ops over the falsifier's lengths ───────────────────────────

#[test]
fn v008_vector_ops_match_the_reference_at_every_length_interpreted() {
    for op in OPS {
        for n in LENGTHS {
            let what = format!("{} over {n}", qualified(op));
            let got = interpret_result(&probe(op, n))
                .unwrap_or_else(|e| panic!("{what}: the interpreter refused: {e}"));
            assert_eq!(show(&got), show(&want(op, n)), "{what}: interpreted");
        }
    }
}

/// THE POOL REACHES EVERY CLASS IT CLAIMS: over the longest probe, each op's
/// reference meets a canonical NaN, a signed zero, an infinity and a subnormal
/// result — so the comparisons above measured those agreements and did not
/// merely pass over ordinary numbers.
#[test]
fn v008_the_inputs_reach_every_special_result_class() {
    let n = *LENGTHS.iter().max().unwrap();
    for op in OPS {
        let r: Vec<f64> = want(op, n)[1..]
            .iter()
            .map(|b| f64::from_bits(*b))
            .collect();
        let nan = r.iter().filter(|x| x.is_nan()).count();
        let inf = r.iter().filter(|x| x.is_infinite()).count();
        let neg_zero = r
            .iter()
            .filter(|x| **x == 0.0 && x.is_sign_negative())
            .count();
        let sub = r.iter().filter(|x| x.is_subnormal()).count();
        println!("{op:?}: NaN {nan}, ±inf {inf}, -0 {neg_zero}, subnormal {sub}");
        assert!(
            nan > 0 && inf > 0 && neg_zero > 0 && sub > 0,
            "{op:?}: a class is unreached — NaN {nan}, inf {inf}, -0 {neg_zero}, subnormal {sub}"
        );
    }
}

// ── (2) in place ────────────────────────────────────────────────────────────

/// `फल` may BE an operand run: `क = क + ख` over a length with a tail strip.
#[test]
fn v008_a_vector_op_in_place_matches_the_reference_interpreted() {
    let n = 17;
    let (a, b): (Vec<u64>, Vec<u64>) = (0..n).map(operands).unzip();
    let body = format!(
        "    चरः क ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः प६४ भवति ० ।
{fa}{fb}    चरः परिमाणम् ॱॱ अ६४ भवति {call} आरभ्य क ऽ क ऽ ख समाप्तम् ।
{print}",
        fa = fill("क", &a),
        fb = fill("ख", &b),
        call = qualified(Op::Add),
        print = print_run(n, "क"),
    );
    let got = interpret_result(&module(&body)).unwrap_or_else(|e| panic!("in place: {e}"));
    assert_eq!(show(&got), show(&want(Op::Add, n)), "in place");
}

// ── (3) the refusals ────────────────────────────────────────────────────────

/// Three runs of lengths `lens`, and the call.
fn mismatch_probe(lens: [usize; 3]) -> Vec<(&'static str, String)> {
    let body = format!(
        "    चरः फल ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः क ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः प६४ भवति ० ।
{f0}{f1}{f2}    चरः परिमाणम् ॱॱ अ६४ भवति {call} आरभ्य फल ऽ क ऽ ख समाप्तम् ।
    अवगणना भवति प्लवमुद्रणम् परिमाणम् ।
",
        f0 = fill("फल", &vec![0; lens[0]]),
        f1 = fill(
            "क",
            &(0..lens[1]).map(|i| operands(i).0).collect::<Vec<_>>()
        ),
        f2 = fill(
            "ख",
            &(0..lens[2]).map(|i| operands(i).1).collect::<Vec<_>>()
        ),
        call = qualified(Op::Add),
    );
    module(&body)
}

/// UNEQUAL LENGTHS REFUSE — whichever run is the odd one, and with a fresh
/// (nil, length ०) run among them — by the finisher word's cause, never by
/// truncating to the shortest.
#[test]
fn v008_unequal_lengths_are_refused_interpreted() {
    let code = format!("{VECTOR_LENGTH_REFUSAL:#x}");
    for lens in [
        [5, 5, 4],
        [4, 5, 5],
        [5, 4, 5],
        [0, 17, 17],
        [17, 17, 0],
        [16, 17, 17],
    ] {
        let got = interpret_result(&mismatch_probe(lens));
        println!("{lens:?}: {got:x?}");
        match got {
            Err(e) => assert!(
                e.contains(&code) && e.contains("unequal length"),
                "{lens:?}: refused for another cause: {e}"
            ),
            Ok(w) => panic!("{lens:?}: ran and printed {w:x?}"),
        }
    }
}

/// THE CONTROL: equal lengths through the same probe run and print the count.
#[test]
fn v008_equal_lengths_through_the_mismatch_probe_run_interpreted() {
    for n in [0, 5, 17] {
        let got = interpret_result(&mismatch_probe([n, n, n]))
            .unwrap_or_else(|e| panic!("{n}: refused: {e}"));
        assert_eq!(got, vec![n as u64], "{n}: the count");
    }
}

/// AN ARGUMENT THAT IS NOT A RUN OF `प६४` is the one named cause `FileMismatch`
/// — an integer run, in each of the three positions.
#[test]
fn v008_a_run_that_is_not_float_is_a_file_mismatch_interpreted() {
    for pos in 0..3 {
        let names = ["फल", "क", "ख"];
        let mut decls = String::new();
        for (k, n) in names.iter().enumerate() {
            let ty = if k == pos { "न६४" } else { "प६४" };
            decls.push_str(&format!("    चरः {n} ॱॱ अङ्कः अन्तः {ty} भवति ० ।\n"));
        }
        let body = format!(
            "{decls}    चरः परिमाणम् ॱॱ अ६४ भवति {} आरभ्य फल ऽ क ऽ ख समाप्तम् ।\n",
            qualified(Op::Add)
        );
        let got = interpret_result(&module(&body));
        println!("position {pos}: {got:x?}");
        match got {
            Err(e) => assert!(
                e.contains("FileMismatch"),
                "position {pos}: refused for another cause: {e}"
            ),
            Ok(w) => panic!("position {pos}: ran and printed {w:x?}"),
        }
    }
}

// ════════════════════════════ THE NATIVE HALF ════════════════════════════
//
// The `.t1` chain (compiled HERE by the current `.t1` compiler running in the
// interpreter) and the Rust twin (`Front` → `riscv64.rs` → the Rust assembler
// and linker), both run on `yantra`: their output must be the reference's bits,
// and their two loadable segments must be the same octets. Helpers copied from
// `v008_float_memory.rs`.

use yantra::{Halt, Machine};

fn octets(b: &[u8]) -> sadhana::t1::nirvahana::Value {
    sadhana::t1::nirvahana::Value::Octets(sadhana::t1::nirvahana::Octets::new(b))
}

fn arena(vs: Vec<sadhana::t1::nirvahana::Value>) -> sadhana::t1::nirvahana::Value {
    sadhana::t1::nirvahana::Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

fn encode_site(it: &Interpreter) -> Option<String> {
    use sadhana::t1::nirvahana::Value;
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
                sadhana::t1::nirvahana::Value::Int(srcs.len() as i128),
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

fn t1_image(srcs: &[(&str, String)]) -> Vec<u8> {
    t1_image_with(CHAIN, srcs)
}

/// The Rust emitter's text for the probe's module.
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
        front
            .build_ir()
            .unwrap_or_else(|e| panic!("build_ir refused: {e}"));
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

fn run_image(image: &[u8]) -> (Halt, Vec<u8>) {
    let mut m = Machine::load_elf(image, yantra::ram_for(image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let h = m.run(400_000_000, &mut out);
    (h, out)
}

fn segment(img: &[u8]) -> (u64, Vec<u8>) {
    let ph_off = u64::from_le_bytes(img[32..40].try_into().unwrap()) as usize;
    let p_offset = u64::from_le_bytes(img[ph_off + 8..ph_off + 16].try_into().unwrap()) as usize;
    let p_filesz = u64::from_le_bytes(img[ph_off + 32..ph_off + 40].try_into().unwrap()) as usize;
    let entry = u64::from_le_bytes(img[24..32].try_into().unwrap());
    (entry, img[p_offset..p_offset + p_filesz].to_vec())
}

fn status(h: &Halt) -> u64 {
    match h {
        Halt::Finisher {
            status: Some(s), ..
        } => *s,
        other => panic!("the image did not finish with a status: {other:?}"),
    }
}

/// Both native images run and print `want`, and are the same loadable segment.
fn assert_native(what: &str, srcs: &[(&str, String)], want: &[u64]) {
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
    assert!(
        ms == rs,
        "{what}: segments differ — .t1 {} octets, Rust {}",
        ms.len(),
        rs.len()
    );
}

// ── (4) the four ops on yantra, both emitters, every length ─────────────────

/// THE FALSIFIER'S LENGTHS ON THE MACHINE: each op over 0, 1, 5, 13, 16, 17 and
/// 33 elements, native (`.t1` chain) bits == Rust-twin bits == interpreter bits
/// == the independent reference, and the two images octet-identical.
#[test]
fn v008_vector_ops_match_the_reference_at_every_length_natively() {
    for op in OPS {
        for n in LENGTHS {
            let what = format!("{} over {n}", qualified(op));
            let srcs = probe(op, n);
            let w = want(op, n);
            let interp =
                interpret_result(&srcs).unwrap_or_else(|e| panic!("{what}: interpreter: {e}"));
            assert_eq!(show(&interp), show(&w), "{what}: interpreted");
            assert_native(&what, &srcs, &w);
        }
    }
}

/// THE EXPANSION IS THE DESIGN'S: one `vsetvli` at e64/m8, the strips loaded into
/// `v8`/`v16`, computed into `v24` with `v8` first (GNU's `vs2`), stored, and
/// `v0` never named — read off the Rust emitter's text.
#[test]
fn v008_the_expansion_uses_the_design_registers_and_never_v0() {
    let text = rust_module_text(&probe(Op::Sub, 17));
    let v = |n: i64| {
        format!(
            "{}{}",
            sadhana::t1::riscv64::VECTOR_REGISTER_STEM,
            sadhana::t1::riscv64::devanagari(n)
        )
    };
    let vset = sadhana::t1::riscv64::VECTOR_SET_LENGTH;
    let count = |needle: &str| text.lines().filter(|l| l.contains(needle)).count();
    assert_eq!(
        count(&format!("{vset} ")),
        1,
        "one vsetvli per vector op:\n{text}"
    );
    assert!(text.contains(&format!("{}न ।", sadhana::t1::riscv64::devanagari(219))));
    assert!(text.contains(&format!(
        "{} {}म् {}न {}न ।",
        member(Op::Sub),
        v(24),
        v(8),
        v(16)
    )));
    for line in text.lines() {
        for word in line.split_whitespace() {
            let base = word.trim_end_matches(['म', 'न', 'त', 'य', '्']);
            assert_ne!(base, v(0), "v0 is named: {line}");
        }
    }
}

/// `V-009` part (i-b): THE VECTOR IMAGE ON QEMU — the design falsifier's QEMU
/// leg, which V-008 recorded as not run. `व्यूहॱप्लवयोगः` over 17 elements (one
/// whole strip of 16 at e64/m8, VLEN 128, and a tail of 1), compiled by both
/// emitters, runs on `qemu-system-riscv64` (`-bios none`, M-mode, FS and VS
/// `Off` at reset) and prints the reference's bits, the octets `yantra` prints.
#[test]
fn v009_the_vector_image_runs_on_qemu_and_prints_what_yantra_prints() {
    let (op, n) = (Op::Add, 17);
    let srcs = probe(op, n);
    let w = want(op, n);
    for (engine, image) in [
        (".t1 chain", t1_image(&srcs)),
        ("rust twin", rust_twin_image(&srcs)),
    ] {
        let (h, ours) = run_image(&image);
        assert_eq!(status(&h), 0, "{engine}: yantra must finish cleanly: {h:?}");
        assert_eq!(show(&words(&ours)), show(&w), "{engine} on yantra");
        let theirs = qemu_leg::run(&image).unwrap_or_else(|e| panic!("{engine}: {e}"));
        assert_eq!(show(&words(&theirs)), show(&w), "{engine} on qemu");
        assert_eq!(
            theirs, ours,
            "{engine}: qemu and yantra printed different octets"
        );
    }
}

// ── (5) in place, natively ──────────────────────────────────────────────────

#[test]
fn v008_a_vector_op_in_place_matches_the_reference_natively() {
    let n = 33;
    let (a, b): (Vec<u64>, Vec<u64>) = (0..n).map(operands).unzip();
    let body = format!(
        "    चरः क ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः प६४ भवति ० ।
{fa}{fb}    चरः परिमाणम् ॱॱ अ६४ भवति {call} आरभ्य क ऽ क ऽ ख समाप्तम् ।
{print}",
        fa = fill("क", &a),
        fb = fill("ख", &b),
        call = qualified(Op::Div),
        print = print_run(n, "क"),
    );
    assert_native("in place", &module(&body), &want(Op::Div, n));
}

// ── (6) the refusals, natively ──────────────────────────────────────────────

fn is_length_refusal(h: &Halt) -> bool {
    matches!(
        h,
        Halt::Finisher {
            value,
            status: Some(s)
        } if *value == sadhana::t1::riscv64::fail_word(VECTOR_LENGTH_REFUSAL)
            && *s == VECTOR_LENGTH_REFUSAL
    )
}

/// UNEQUAL LENGTHS REFUSE ON BOTH ENGINES, IDENTICALLY: the interpreter's cause
/// names `0x35a`, and both native images halt at the finisher with that code's
/// FAIL-form word and the code as status (`W-381`; it was the raw word with no
/// status, which QEMU ran past) — whichever run is the odd one, a fresh (length ०) run
/// included — before printing anything.
#[test]
fn v008_unequal_lengths_are_refused_by_both_engines() {
    let code = format!("{VECTOR_LENGTH_REFUSAL:#x}");
    for lens in [
        [5, 5, 4],
        [4, 5, 5],
        [5, 4, 5],
        [0, 17, 17],
        [17, 17, 0],
        [16, 17, 17],
    ] {
        let srcs = mismatch_probe(lens);
        let e = interpret_result(&srcs).expect_err("the interpreter refuses");
        assert!(e.contains(&code), "{lens:?}: interpreter: {e}");
        for (side, image) in [("t1", t1_image(&srcs)), ("rust", rust_twin_image(&srcs))] {
            let (h, out) = run_image(&image);
            assert!(
                is_length_refusal(&h),
                "{lens:?}: {side} halted {h:?}, printing {:x?}",
                words(&out)
            );
            assert!(out.is_empty(), "{lens:?}: {side} printed before refusing");
        }
    }
}

/// THE CONTROL for the refusal: equal lengths through the same probe finish
/// with status ० and print the count, on both images.
#[test]
fn v008_equal_lengths_through_the_mismatch_probe_finish_natively() {
    for n in [0usize, 5, 17] {
        assert_native(
            &format!("equal {n}"),
            &mismatch_probe([n, n, n]),
            &[n as u64],
        );
    }
}

/// A RUN THAT IS NOT `प६४` is `FileMismatch` at BUILD on both native front ends:
/// the `.t1` chain's refusal site and the Rust twin's `build_ir`.
#[test]
fn v008_a_run_that_is_not_float_is_refused_at_build() {
    for pos in 0..3 {
        let names = ["फल", "क", "ख"];
        let mut decls = String::new();
        for (k, n) in names.iter().enumerate() {
            let ty = if k == pos { "न६४" } else { "प६४" };
            decls.push_str(&format!("    चरः {n} ॱॱ अङ्कः अन्तः {ty} भवति ० ।\n"));
        }
        let srcs = module(&format!(
            "{decls}    चरः परिमाणम् ॱॱ अ६४ भवति {} आरभ्य फल ऽ क ऽ ख समाप्तम् ।\n",
            qualified(Op::Add)
        ));
        let t1 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t1_image(&srcs)));
        let t1 = t1.map(|_| ()).map_err(panic_text);
        assert!(
            t1.as_ref().is_err_and(|e| e.contains("FileMismatch")),
            "position {pos}: .t1 chain {t1:?}"
        );
        let rust =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rust_twin_image(&srcs)));
        let rust = rust.map(|_| ()).map_err(panic_text);
        assert!(
            rust.as_ref().is_err_and(|e| e.contains("FileMismatch")),
            "position {pos}: Rust twin {rust:?}"
        );
    }
}

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<String>()
        .cloned()
        .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_default()
}

// ── (7) the three emitter mutants, each shown red ───────────────────────────

/// A RISC-V instruction's Sassembly mnemonic, READ from the encoding table.
fn mnemonic_of(insn: &str) -> String {
    std::fs::read_to_string(spec_root().join("encodings-riscv64.tsv"))
        .expect("read the encoding table")
        .lines()
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .find(|f| f.len() >= 3 && f[0] == insn)
        .map(|f| f[2].to_string())
        .unwrap_or_else(|| panic!("{insn} is in spec/encodings-riscv64.tsv"))
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

/// The site the loop is entered at in `yantrotsarjana.t1` — the one `समलङ्घनम्`
/// on the count to the expansion's `अतिक्रम` label that precedes the loop label.
const LOOP_ENTRY: &str = "    यन्त्रखण्डलक्ष्यम् आरभ्य फलाङ्कः ऽ उक्तम् अतिक्रम इति ऽ ऋण१ समाप्तम् ।
    ॰ the loop.";

/// MUTANT (i) — THE STRIP COUNT FIXED AT VLMAX, THE TAIL SKIPPED: the count is
/// rounded down to a multiple of 16 before the loop (`युक्तम् … ऋण१६`), so only
/// whole strips run. Lengths 5, 13, 17 and 33 must go red; 0 and 16 cannot.
#[test]
fn v008_mutant_strip_count_fixed_at_vlmax_with_the_tail_skipped_is_red() {
    let mutant = format!(
        "    यन्त्रखण्डलक्ष्यम् आरभ्य फलाङ्कः ऽ उक्तम् अतिक्रम इति ऽ ऋण१ समाप्तम् ।
    उत्सर्जनॱपाठयोजनम् उक्तम् {andi} क्षणिक३म् क्षणिक३न ऋण१६न इति ।
    यन्त्राज्ञान्तः ।
    ॰ the loop.",
        andi = mnemonic_of("andi")
    );
    let chain = mutated_chain("yantrotsarjana.t1", LOOP_ENTRY, &mutant);
    for n in [5usize, 13, 17, 33] {
        let got = native_mutated(&chain, &probe(Op::Add, n));
        println!("mutant (i), {n}: {got:x?}");
        assert!(
            got.as_ref().map_or(true, |w| *w != want(Op::Add, n)),
            "mutant (i) printed the right answer at length {n}"
        );
    }
    for n in [0usize, 16] {
        let got = native_mutated(&chain, &probe(Op::Add, n));
        assert_eq!(
            got,
            Ok(want(Op::Add, n)),
            "mutant (i) is wrong where it cannot be: {n}"
        );
    }
}

/// MUTANT (ii) — THE POINTER ADVANCE OFF BY ONE ELEMENT: `vl × 8 + 8`. A
/// single strip never advances, so 17 and 33 — the lengths with a second strip —
/// must go red.
#[test]
fn v008_mutant_pointer_advance_off_by_one_element_is_red() {
    let live = "    यन्त्रसङ्ख्यापदम् ३ ।
    यन्त्राज्ञान्तः ।
    चरः अग्रक्रमः ॱॱ न६४ भवति ० ।";
    let mutant = format!(
        "    यन्त्रसङ्ख्यापदम् ३ ।
    यन्त्राज्ञान्तः ।
    उत्सर्जनॱपाठयोजनम् उक्तम् {add} क्षणिक५म् क्षणिक५न ८न इति ।
    यन्त्राज्ञान्तः ।
    चरः अग्रक्रमः ॱॱ न६४ भवति ० ।",
        add = mnemonic_of("addi")
    );
    let chain = mutated_chain("yantrotsarjana.t1", live, &mutant);
    for n in [17usize, 33] {
        let got = native_mutated(&chain, &probe(Op::Add, n));
        println!("mutant (ii), {n}: {got:x?}");
        assert!(
            got.as_ref().map_or(true, |w| *w != want(Op::Add, n)),
            "mutant (ii) printed the right answer at length {n}"
        );
    }
}

/// MUTANT (iii) — THE EQUAL-LENGTH TEST REMOVED: the expansion no longer
/// compares the operand lengths with the result's. It must NOT refuse `[5, 5, 4]`
/// any more — the refusal test's subject is shown load-bearing.
#[test]
fn v008_mutant_without_the_equal_length_test_is_red() {
    let bne = mnemonic_of("bne");
    let live = format!(
        "        यदि दैर्घ्यक्रमः अधिकम् ० आदि
            उत्सर्जनॱपाठयोजनम् उक्तम् {bne} इति ।"
    );
    let mutant = format!(
        "        यदि दैर्घ्यक्रमः अधिकम् ३ आदि
            उत्सर्जनॱपाठयोजनम् उक्तम् {bne} इति ।"
    );
    let chain = mutated_chain("yantrotsarjana.t1", &live, &mutant);
    let refs: Vec<(&str, &str)> = chain.iter().map(|(n, s)| (*n, s.as_str())).collect();
    let (h, out) = run_image(&t1_image_with(&refs, &mismatch_probe([5, 5, 4])));
    println!("mutant (iii): {h:?}, {:x?}", words(&out));
    assert!(
        !is_length_refusal(&h),
        "mutant (iii) still refused by 0x35a — the length test is not load-bearing"
    );
}

// ── (8) the qualifier needs no import, and the trust stays narrow ────────────

/// THE IMPORT IS NOT REQUIRED (owner ruling, option A) — and still accepted: the
/// probe every native test above uses imports only `अष्टक`; this one adds
/// `आयातः व्यूह ।` and must give the same bits on both engines and both images.
#[test]
fn v008_a_program_that_imports_the_qualifier_is_still_green() {
    let n = 17;
    let base = probe(Op::Mul, n);
    let body = base[0]
        .1
        .split_once("    चरः घ ॱॱ प६४ भवति ० ।\n")
        .and_then(|(_, rest)| rest.rsplit_once("    प्रत्यागमनम् ० ।"))
        .map(|(b, _)| b.to_string())
        .expect("the probe body");
    let srcs = module_importing(&["व्यूह"], &body);
    assert!(srcs[0].1.contains("आयातः व्यूह ।"));
    let w = want(Op::Mul, n);
    let interp = interpret_result(&srcs).unwrap_or_else(|e| panic!("interpreter: {e}"));
    assert_eq!(show(&interp), show(&w), "interpreted");
    assert_native("with the import", &srcs, &w);
}

/// The front half of the `.t1` chain on `srcs`, or the panic text — the Rust
/// twin's resolver IS this one (`Front` drives `artha.t1`).
fn build_refusal(srcs: &[(&str, String)]) -> (String, String) {
    let t1 = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t1_image(srcs)))
        .map(|_| String::from("built"))
        .unwrap_or_else(panic_text);
    let rust = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rust_twin_image(srcs)))
        .map(|_| String::from("built"))
        .unwrap_or_else(panic_text);
    (t1, rust)
}

/// THE TRUST IS NARROW: a qualifier no module declares and nothing imports is
/// still refused AT RESOLVE by name on both compilers — another undeclared
/// qualifier with a float member, and `व्यूह` itself with a member that is not one
/// of the four vector built-ins.
#[test]
fn v008_another_undeclared_qualifier_is_still_refused_at_resolve() {
    let decls = "    चरः फल ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः क ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः प६४ भवति ० ।
";
    let sqrt = sadhana::t1::nirvahana::FLOAT_BUILTINS
        .iter()
        .find(|(_, _, _, m)| *m == "fsqrt.d")
        .map(|(n, ..)| *n)
        .expect("fsqrt.d");
    for (callee, interp_refuses) in [
        (format!("कल्पितॱ{}", member(Op::Add)), false),
        (format!("व्यूहॱ{sqrt}"), true),
        (format!("व्यूह ॱ {sqrt}"), true),
    ] {
        let srcs = module(&format!(
            "{decls}    चरः परिमाणम् ॱॱ अ६४ भवति {callee} आरभ्य फल ऽ क ऽ ख समाप्तम् ।\n"
        ));
        // THE INTERPRETER, for the `व्यूह` qualifier, refuses with the
        // compilers' own cause; another unloaded qualifier keeps its late path
        // (a parse error there: no arity is known for it).
        let interp = interpret_result(&srcs);
        if interp_refuses {
            let e = interp.as_ref().expect_err("the interpreter refuses it");
            assert!(
                e.contains("has no declaration"),
                "{callee}: interpreter: {e}"
            );
        } else {
            assert!(interp.is_err(), "{callee}: the interpreter ran it");
        }
        let (t1, rust) = build_refusal(&srcs);
        println!("{callee}: .t1 {t1:.200} | rust {rust:.200}");
        for (side, why) in [(".t1", &t1), ("rust", &rust)] {
            assert!(
                why.contains("has no declaration"),
                "{callee}: the {side} compiler did not refuse it at resolve by name: {why}"
            );
        }
    }
}

// ── (9) the four names, one list in four places ─────────────────────────────

fn crate_source(rel: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(rel),
    )
    .unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// The body of the `.t1` routine `name` (from its `वृत्तिः` line to its `इति`).
fn t1_routine<'a>(src: &'a str, name: &str) -> &'a str {
    let head = src
        .find(&format!("वृत्तिः {name} आदाय"))
        .unwrap_or_else(|| panic!("no routine {name}"));
    let body = &src[head..];
    let end = body.find("\nइति\n").expect("the routine closes");
    &body[..end]
}

/// Every `समम् उक्तम् X इति` literal a routine compares a member against.
fn compared_literals(body: &str) -> std::collections::BTreeSet<String> {
    body.split("समम् उक्तम् ")
        .skip(1)
        .filter_map(|r| r.split_whitespace().next())
        .filter(|w| *w != "व्यूह")
        .map(str::to_string)
        .collect()
}

fn deva_number(s: &str) -> u32 {
    s.chars()
        .map(|c| c.to_digit(10).unwrap_or_else(|| (c as u32) - 0x966))
        .fold(0, |n, d| n * 10 + d)
}

/// THE FOUR VECTOR MEMBER NAMES ARE ONE LIST, read from all three places that
/// spell it (the `parse.t1` spaced fold is gone, ruling (B)): `artha.t1`'s resolver
/// trust (`व्यूहसदस्यः`), `ir.t1`'s matcher (the `प्लवसदस्यभेदः` names whose
/// sub-kind is at most `प्लवभागोपभेद`, which `व्यूहान्तर्निहितम्` keeps) and
/// `nirvahana.rs`'s `vector_builtin` (the `FLOAT_BUILTINS` rows of the variants
/// its `matches!` keeps). A fifth op added in one place and not the others —
/// or a name respelled in one — goes red here.
#[test]
fn v008_the_four_vector_names_are_the_same_in_every_source() {
    let artha = crate_source("crates/sadhana-t1/src/artha.t1");
    let ir = crate_source("crates/sadhana-t1/src/ir.t1");
    let nir = crate_source("crates/sadhana/src/t1/nirvahana.rs");

    let from_artha = compared_literals(t1_routine(&artha, "व्यूहसदस्यः"));

    // ir.t1: the constant the matcher's ceiling names, then every name the
    // member table maps to a sub-kind at or below it.
    let matcher = t1_routine(&ir, "व्यूहान्तर्निहितम्");
    let ceiling_name = matcher
        .split("यदि सदस्यभेदः अधिकम् ")
        .nth(1)
        .and_then(|r| r.split_whitespace().next())
        .expect("the matcher's ceiling");
    let value_of = |name: &str| -> u32 {
        let decl = format!("सार्वजनिक चरः {name} ॱॱ न६४ भवति ");
        let at = ir
            .find(&decl)
            .unwrap_or_else(|| panic!("{name} is declared"));
        deva_number(ir[at + decl.len()..].split_whitespace().next().unwrap())
    };
    let ceiling = value_of(ceiling_name);
    let table = t1_routine(&ir, "प्लवसदस्यभेदः");
    let from_ir: std::collections::BTreeSet<String> = table
        .lines()
        .filter_map(|l| {
            let name = l.split("समम् उक्तम् ").nth(1)?.split_whitespace().next()?;
            let kind = l.split("प्रत्यागमनम् ").nth(1)?.split_whitespace().next()?;
            (value_of(kind) <= ceiling).then(|| name.to_string())
        })
        .collect();

    // nirvahana.rs: the variants `vector_builtin` keeps, then their rows.
    let vb = &nir[nir.find("pub fn vector_builtin").expect("vector_builtin")..];
    let kept = &vb[vb.find("matches!(").unwrap()..vb.find(")\n        })").unwrap()];
    let variants: Vec<&str> = kept
        .split("FloatBuiltin::")
        .skip(1)
        .filter_map(|r| r.split(|c: char| !c.is_alphanumeric()).next())
        .collect();
    let rows = &nir[nir.find("pub const FLOAT_BUILTINS").unwrap()..];
    let rows = &rows[..rows.find("];").unwrap()];
    let from_nir: std::collections::BTreeSet<String> = rows
        .lines()
        .filter(|l| {
            variants
                .iter()
                .any(|v| l.contains(&format!("FloatBuiltin::{v},")))
        })
        .filter_map(|l| l.split('"').nth(1).map(str::to_string))
        .collect();

    println!("artha {from_artha:?}\nir {from_ir:?}\nnirvahana {from_nir:?}");
    assert_eq!(from_artha.len(), 4, "artha.t1 names {from_artha:?}");
    assert_eq!(from_ir, from_artha, "ir.t1 against artha.t1");
    assert_eq!(from_nir, from_artha, "nirvahana.rs against artha.t1");
}

// ── (10) the spaced spelling, and an unread count ──────────────────────────

/// THE SPACED SPELLING IS NOT THE BUILT-IN (owner ruling (B)): `व्यूह ॱ प्लवयोगः`
/// with `व्यूह` unbound is an ordinary field access of an undeclared name, refused
/// by all three engines with the same cause, "`व्यूह` … has no declaration". The
/// one-token spelling of the same call is the built-in (every test above).
#[test]
fn v008_the_spaced_spelling_of_an_unbound_qualifier_is_refused_everywhere() {
    for op in OPS {
        let spaced = format!(
            "{} \u{971} {}",
            sadhana::t1::nirvahana::VECTOR_BUILTIN_MODULE,
            member(op)
        );
        let mut srcs = probe(op, 5);
        let one_token = qualified(op);
        assert_eq!(srcs[0].1.matches(&one_token).count(), 1);
        srcs[0].1 = srcs[0].1.replacen(&one_token, &spaced, 1);
        let head = format!("`{}`", sadhana::t1::nirvahana::VECTOR_BUILTIN_MODULE);
        let interp = interpret_result(&srcs);
        let e = interp.expect_err("the interpreter refuses the spaced spelling");
        assert!(
            e.contains(&head) && e.contains("has no declaration"),
            "{spaced}: interpreter: {e}"
        );
        let (t1, rust) = build_refusal(&srcs);
        let shown = format!("{head:?}");
        let shown = shown.trim_matches('"');
        for (side, why) in [(".t1", &t1), ("rust", &rust)] {
            assert!(
                why.contains("has no declaration") && (why.contains(&head) || why.contains(shown)),
                "{spaced}: the {side} compiler: {why}"
            );
        }
    }
}

// ── (11) a BOUND `व्यूह` is an ordinary name ─────────────────────────────────

/// The reviewer's record: one field, `प्लवयोगः`, an integer.
const RECORD: &str = "संरचना अभिलेखः आरभ्य
    प्लवयोगः ॱॱ न६४
समाप्तम् ।
";

/// Read `व्यूह ॱ प्लवयोगः`, write ७ to it, read it again, printing both reads.
const READ_WRITE_READ: &str = "    चरः य ॱॱ न६४ भवति व्यूह ॱ प्लवयोगः ।
    अवगणना भवति प्लवमुद्रणम् य ।
    व्यूह ॱ प्लवयोगः भवति ७ ।
    य भवति व्यूह ॱ प्लवयोगः ।
    अवगणना भवति प्लवमुद्रणम् य ।
";

/// One module: `decls` (after the record), the printer, and `मुख्यम्` running
/// `body`.
fn bound_module(decls: &str, body: &str) -> Vec<(&'static str, String)> {
    vec![(
        MODULE,
        format!(
            "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

{RECORD}{decls}
{PRINTER}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
{body}    प्रत्यागमनम् ० ।
इति
"
        ),
    )]
}

/// A `व्यूह` THE PROGRAM BINDS WINS — as a LOCAL, a PARAMETER and a module
/// GLOBAL: `व्यूह ॱ प्लवयोगः` reads and writes the record's field on every
/// engine (०, then ७), and the two images are the same octets. Folding it into
/// the built-in read an address (`0x80011010`) and crashed the write.
#[test]
fn v008_a_bound_vyuha_is_an_ordinary_record_on_every_engine() {
    let cases = [
        (
            "a local",
            bound_module(
                "",
                &format!("    चरः व्यूह ॱॱ अभिलेखः भवति ० ।\n{READ_WRITE_READ}"),
            ),
        ),
        (
            "a parameter",
            bound_module(
                &format!(
                    "वृत्तिः अभिलेखपरीक्षा आदाय व्यूह ॱॱ अभिलेखः ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
{READ_WRITE_READ}    प्रत्यागमनम् ० ।
इति
"
                ),
                "    चरः र ॱॱ अभिलेखः भवति ० ।
    अवगणना भवति अभिलेखपरीक्षा र ।
",
            ),
        ),
    ];
    for (what, srcs) in cases {
        let interp = interpret_result(&srcs).unwrap_or_else(|e| panic!("{what}: interpreter: {e}"));
        assert_eq!(interp, vec![0, 7], "{what}: interpreted");
        assert_native(what, &srcs, &[0, 7]);
    }
}

/// A MODULE GLOBAL `व्यूह` IS THE SAME PROGRAM AS ANY OTHER NAME. The interpreter
/// reads ० then ७. NATIVELY A RECORD-TYPED GLOBAL INITIALISED `भवति ०` IS A NULL
/// POINTER TODAY — the identical program with the global named `र` halts
/// `BadAccess` at address ० on both images, a divergence that predates V-008
/// and is not this row's — so the native assertion is that `व्यूह` changes
/// NOTHING: the `व्यूह` program and the `र` program halt alike and print alike
/// on both images, and every pair of images is the same octets. Folding
/// `व्यूह` into the built-in made the global's two reads `[0, 0]` (the write
/// lost) where `र` faults.
#[test]
fn v008_a_global_vyuha_is_the_same_program_as_any_other_global() {
    let named = |n: &str| {
        bound_module(
            &format!("चरः {n} ॱॱ अभिलेखः भवति ० ।\n"),
            &READ_WRITE_READ.replace(sadhana::t1::nirvahana::VECTOR_BUILTIN_MODULE, n),
        )
    };
    let vy = named(sadhana::t1::nirvahana::VECTOR_BUILTIN_MODULE);
    let ra = named("र");
    assert!(vy[0].1.contains("चरः व्यूह ॱॱ अभिलेखः"));
    let interp = interpret_result(&vy).unwrap_or_else(|e| panic!("interpreter: {e}"));
    assert_eq!(interp, vec![0, 7], "interpreted");
    let run = |srcs: &[(&str, String)]| {
        let t1 = t1_image(srcs);
        let rust = rust_twin_image(srcs);
        assert_eq!(segment(&t1), segment(&rust), "the two images differ");
        let (h, out) = run_image(&t1);
        (format!("{h:?}"), out)
    };
    let (hv, ov) = run(&vy);
    let (hr, or) = run(&ra);
    println!("व्यूह: {hv} {:x?}\nर: {hr} {:x?}", words(&ov), words(&or));
    // AND THE RESULT IS ONE OF THE TWO KNOWN ONES — the right answer, or today's
    // null-pointer fault with nothing printed — so a regression that broke both
    // globals alike cannot pass as "the same program".
    let today = format!("{:?}", Halt::BadAccess { pc: 0, addr: 0 });
    let faulted = hv.starts_with("BadAccess {") && hv.ends_with("addr: 0 }") && ov.is_empty();
    assert!(
        words(&ov) == vec![0, 7] || faulted,
        "the global व्यूह program gave {hv} {:x?}, neither [0, 7] nor today's {today}-shaped fault",
        words(&ov)
    );
    assert_eq!(
        (hv, ov),
        (hr, or),
        "a global named व्यूह is not the same program"
    );
}

/// A CALL WHOSE COUNT NOBODY READS STILL WRITES ITS RESULT RUN: the call is a
/// bare statement, its answer discarded, and the elements are printed after it.
/// No pass of the native pipeline deletes dead instructions today, so this is
/// green by the emitters alone; `ir.rs`'s `a_vector_op_survives_dead_code_
/// elimination` is the half that is red without `is_side_effecting`.
#[test]
fn v008_a_vector_call_whose_count_is_discarded_still_writes() {
    let n = 5;
    let (a, b): (Vec<u64>, Vec<u64>) = (0..n).map(operands).unzip();
    let body = format!(
        "    चरः फल ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः क ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः प६४ भवति ० ।
{fz}{fa}{fb}    {call} आरभ्य फल ऽ क ऽ ख समाप्तम् ।
    चरः परिमाणम् ॱॱ अ६४ भवति {n} ।
{print}",
        fz = fill("फल", &vec![0; n]),
        fa = fill("क", &a),
        fb = fill("ख", &b),
        call = qualified(Op::Sub),
        n = dec(n as u64),
        print = print_run(n, "फल"),
    );
    let srcs = module(&body);
    let w = want(Op::Sub, n);
    let interp = interpret_result(&srcs).unwrap_or_else(|e| panic!("interpreter: {e}"));
    assert_eq!(show(&interp), show(&w), "interpreted");
    assert_native("discarded count", &srcs, &w);
}

// ── (12) a spaced व्यूह is ordinary, and a REAL module व्यूह is a module ───────

/// The length-5 add probe with its call SPACED and the program IMPORTING `व्यूह`
/// (a module that does not exist): the spaced form is never the built-in, so all
/// three engines refuse; the refusal texts are printed, not forced alike.
#[test]
fn v008_the_spaced_spelling_with_an_import_is_not_the_builtin() {
    let v = sadhana::t1::nirvahana::VECTOR_BUILTIN_MODULE;
    let one_token = qualified(Op::Add);
    let spaced = format!("{v} \u{971} {}", member(Op::Add));
    let base = probe(Op::Add, 5);
    let body = base[0]
        .1
        .split_once("    चरः घ ॱॱ प६४ भवति ० ।\n")
        .and_then(|(_, rest)| rest.rsplit_once("    प्रत्यागमनम् ० ।"))
        .map(|(b, _)| b.replacen(&one_token, &spaced, 1))
        .expect("the probe body");
    let srcs = module_importing(&[v], &body);
    assert!(srcs[0].1.contains(&spaced) && !srcs[0].1.contains(&one_token));
    let interp = interpret_result(&srcs);
    let (t1, rust) = build_refusal(&srcs);
    println!("interpreter: {interp:?}\n.t1: {t1}\nrust: {rust}");
    assert!(
        interp.is_err(),
        "the interpreter ran the spaced form: {interp:?}"
    );
    assert_ne!(t1, "built", "the .t1 chain built the spaced form");
    assert_ne!(rust, "built", "the Rust twin built the spaced form");
}

/// A REAL MODULE NAMED `व्यूह`: `उत्तरम्` answers ४२, and `प्लवयोगः` takes three
/// `प६४` runs and answers ९९ without touching them — distinguishable from the
/// vector add, which answers the count and writes the result run.
fn real_vyuha() -> (&'static str, String) {
    (
        sadhana::t1::nirvahana::VECTOR_BUILTIN_MODULE,
        format!(
            "मण्डलम् {v} ॥

सार्वजनिक वृत्तिः उत्तरम् ददाति न६४ आदि
    प्रत्यागमनम् ४२ ।
इति

सार्वजनिक वृत्तिः {m} आदाय फल ॱॱ अङ्कः अन्तः प६४ ऽ क ॱॱ अङ्कः अन्तः प६४ ऽ ख ॱॱ अङ्कः अन्तः प६४ ददाति अ६४ आदि
    प्रत्यागमनम् ९९ ।
इति
",
            v = sadhana::t1::nirvahana::VECTOR_BUILTIN_MODULE,
            m = member(Op::Add),
        ),
    )
}

/// ADR-0043 RESERVES NOTHING: a real module `व्यूह` is called like any module,
/// in both spellings, on all three engines — `उत्तरम्` prints ४२ twice.
#[test]
fn v008_a_real_module_named_vyuha_is_an_ordinary_module() {
    let v = sadhana::t1::nirvahana::VECTOR_BUILTIN_MODULE;
    let body = format!(
        "    चरः प ॱॱ न६४ भवति {v} \u{971} उत्तरम् ।
    अवगणना भवति प्लवमुद्रणम् प ।
    प भवति {v}\u{971}उत्तरम् ।
    अवगणना भवति प्लवमुद्रणम् प ।
"
    );
    let mut srcs = module_importing(&[v], &body);
    srcs.push(real_vyuha());
    let interp = interpret_result(&srcs).unwrap_or_else(|e| panic!("interpreter: {e}"));
    assert_eq!(interp, vec![42, 42], "interpreted");
    assert_native("a real module व्यूह", &srcs, &[42, 42]);
}

/// RULING (B) DECIDES A REAL MODULE `व्यूह` THAT EXPORTS `प्लवयोगः`: the ONE-TOKEN
/// `व्यूहॱप्लवयोगः` is the vector BUILT-IN (the reference bits; the module's ९९
/// never answers), and the SPACED `व्यूह ॱ प्लवयोगः` is the MODULE'S ROUTINE
/// (९९, the result run untouched) — on all three engines.
#[test]
fn v008_with_a_real_module_one_token_is_the_builtin_and_spaced_is_the_routine() {
    let v = sadhana::t1::nirvahana::VECTOR_BUILTIN_MODULE;
    let n = 5;
    let one_token = qualified(Op::Add);
    let base = probe(Op::Add, n);
    let body = base[0]
        .1
        .split_once("    चरः घ ॱॱ प६४ भवति ० ।\n")
        .and_then(|(_, rest)| rest.rsplit_once("    प्रत्यागमनम् ० ।"))
        .map(|(b, _)| b.to_string())
        .expect("the probe body");

    let mut builtin = module_importing(&[v], &body);
    builtin.push(real_vyuha());
    let w = want(Op::Add, n);
    let interp =
        interpret_result(&builtin).unwrap_or_else(|e| panic!("one token: interpreter: {e}"));
    assert_eq!(show(&interp), show(&w), "one token: interpreted");
    assert_native("one token with a real module", &builtin, &w);

    let spaced = format!("{v} \u{971} {}", member(Op::Add));
    let mut routine = module_importing(&[v], &body.replacen(&one_token, &spaced, 1));
    routine.push(real_vyuha());
    assert!(routine[0].1.contains(&spaced));
    let mut w = vec![99u64];
    w.extend(std::iter::repeat_n(0, n));
    let interp = interpret_result(&routine).unwrap_or_else(|e| panic!("spaced: interpreter: {e}"));
    assert_eq!(show(&interp), show(&w), "spaced: interpreted");
    assert_native("spaced with a real module", &routine, &w);
}

// ── (13) the built-in cannot be forged by a module name ─────────────────────

/// A MODULE NAMED `व्यूहॱ` — the qualifier WITH its mark — exporting `प्लवयोगः`
/// (answering ९९), called spaced as `व्यूहॱ ॱ प्लवयोगः` on three runs of 1.0.
/// It is an ordinary module and an ordinary call: both compilers build it (the
/// label is `व्यूहॱप्लवयोगः`) and print ९९ with the result run untouched, and
/// the interpreter must agree. Before, the interpreter read its module field as
/// the built-in's mark and ran the VECTOR ADD (3, then 2.0 three times).
#[test]
fn v008_a_module_named_like_the_old_mark_is_an_ordinary_module() {
    let v = sadhana::t1::nirvahana::VECTOR_BUILTIN_MODULE;
    let forged = format!("{v}\u{971}");
    let add = member(Op::Add);
    let one = 1.0f64.to_bits();
    let body = format!(
        "    चरः फल ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः क ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः प६४ भवति ० ।
{fz}{fa}{fb}    चरः परिमाणम् ॱॱ अ६४ भवति {forged} \u{971} {add} आरभ्य फल ऽ क ऽ ख समाप्तम् ।
{print}",
        fz = fill("फल", &[0; 3]),
        fa = fill("क", &[one; 3]),
        fb = fill("ख", &[one; 3]),
        print = print_run(3, "फल"),
    );
    let mut srcs = module_importing(&[&forged], &body);
    srcs.push((
        Box::leak(forged.clone().into_boxed_str()),
        format!(
            "मण्डलम् {forged} ॥

सार्वजनिक वृत्तिः {add} आदाय फल ॱॱ अङ्कः अन्तः प६४ ऽ क ॱॱ अङ्कः अन्तः प६४ ऽ ख ॱॱ अङ्कः अन्तः प६४ ददाति अ६४ आदि
    प्रत्यागमनम् ९९ ।
इति
"
        ),
    ));
    let want = vec![99, 0, 0, 0];
    let interp = interpret_result(&srcs).unwrap_or_else(|e| panic!("interpreter: {e}"));
    assert_eq!(show(&interp), show(&want), "interpreted");
    assert_native("a module named व्यूहॱ", &srcs, &want);
}

/// THE LOADER'S REFUSAL NAMES A VECTOR CALL AS IT IS WRITTEN — `व्यूहॱप्लवयोगः`,
/// one mark — in a global initialiser it will not evaluate. It printed the mark
/// twice while the call node carried the mark in its module field.
#[test]
fn v008_a_vector_call_in_a_global_initialiser_is_named_as_written() {
    let call = qualified(Op::Add);
    let src = format!(
        "मण्डलम् {MODULE} ॥

चरः फल ॱॱ अङ्कः अन्तः प६४ भवति ० ।
चरः परिमाणम् ॱॱ अ६४ भवति {call} आरभ्य फल ऽ फल ऽ फल समाप्तम् ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    प्रत्यागमनम् ० ।
इति
"
    );
    let file = format!("{MODULE}.t1");
    let e = Interpreter::load(&[(file.as_str(), src.as_str())], &spec_root())
        .err()
        .expect("the loader refuses a call in a global initialiser")
        .reason;
    println!("{e}");
    let doubled = format!(
        "{}\u{971}\u{971}",
        sadhana::t1::nirvahana::VECTOR_BUILTIN_MODULE
    );
    assert!(
        e.contains(&format!("calls `{call}`")) && !e.contains(&doubled),
        "the call is not named as written: {e}"
    );
}
