//! **`V-009` PART (ii): THE MATRIX AND TENSOR BUILT-INS.**
//!
//! `व्यूहॱआव्यूहगुणनम् ( फल , क , ख , M , K , N , L )` sets `फल` (M×N) to `क` (M×K)
//! times `ख` (K×N); `व्यूहॱव्युत्क्रमः ( फल , क , M , N , L )` sets `फल` (N×M) to
//! `क` (M×N) transposed; `व्यूहॱसमासः ( फल , क , ख , B , M , K , N , L )` is B
//! independent products over a leading batch index. `L` is `व्यूहॱपङ्क्तिप्रधानम्` or
//! `व्यूहॱस्तम्भप्रधानम्`, read as text at compile time (docs/v009-matrix-design.md, the
//! owner's rulings of 2026-10-06, "ALL AS RECOMMENDED"). Each answers the
//! element count of its result.
//!
//! THE ORACLE IS INDEPENDENT OF EVERY ENGINE: a plain Rust `f64` loop over the
//! LOGICAL indices of each layout — `s = +0.0`, then `s = s + a·b` for `k` in
//! order, `*` then `+`, never `mul_add` — with a NaN result replaced by RISC-V's
//! canonical NaN. The engines are the interpreter, the `.t1` chain's image and
//! the Rust twin's image on `yantra`, and both images on QEMU.
//!
//! THE SHAPES ARE THE DESIGN'S FALSIFIER: non-square, N of 7, 17 and 33 (VLMAX at
//! SEW 64, LMUL m8, VLEN 128 is 16), K = 0, empty results, both layouts, and the
//! batched product at batch 1 and 3. The data is order-sensitive (±2^53 beside
//! small values), and three probes make the rules a mutant would break visible
//! by construction: a row whose products are all −0 (the +0.0 seed), a row whose
//! sum changes with the order of the additions, and a row whose sum changes when
//! the multiply and add fuse.

#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{
    COLUMN_MAJOR, Interpreter, MATRIX_PRODUCT_MEMBER, MATRIX_TRANSPOSE_MEMBER, REFUSAL_ADHYASA,
    ROW_MAJOR, TENSOR_MEMBER, TRANSPOSED, VECTOR_BUILTIN_MODULE, VECTOR_LENGTH_REFUSAL,
};
use std::path::{Path, PathBuf};
use yantra::{Halt, Machine};

mod qemu_leg;

const FUEL: u64 = 80_000_000_000;
const MODULE: &str = "व्यूहपरीक्षण";
const CANONICAL_NAN: u64 = 0x7ff8_0000_0000_0000;

fn qualified(member: &str) -> String {
    format!("{VECTOR_BUILTIN_MODULE}\u{971}{member}")
}

// ── the cases ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Product,
    Tensor,
    Transpose,
}

/// One call: its kind, its dimensions (`batch, M, K, N` — the transpose uses
/// `M` and `N`), its layout, and the pool and the seeds its runs are filled from.
#[derive(Clone, Copy, Debug)]
struct Case {
    kind: Kind,
    batch: usize,
    m: usize,
    k: usize,
    n: usize,
    column_major: bool,
    /// `व्यूहॱपरिवर्तितम्`: the first operand stored K×M and read transposed.
    transposed: bool,
    pool: &'static [u64],
    seed: usize,
}

impl Case {
    fn product(m: usize, k: usize, n: usize, column_major: bool) -> Self {
        Self {
            kind: Kind::Product,
            batch: 1,
            m,
            k,
            n,
            column_major,
            transposed: false,
            pool: &ORDINARY,
            seed: m * 7 + k * 3 + n,
        }
    }
    fn tensor(batch: usize, m: usize, k: usize, n: usize, column_major: bool) -> Self {
        Self {
            kind: Kind::Tensor,
            batch,
            ..Self::product(m, k, n, column_major)
        }
    }
    fn transpose(m: usize, n: usize, column_major: bool) -> Self {
        Self {
            kind: Kind::Transpose,
            batch: 1,
            m,
            k: 0,
            n,
            column_major,
            transposed: false,
            pool: &SPECIAL,
            seed: m * 5 + n,
        }
    }
    /// The same product with its first operand in the TRANSPOSED layout.
    fn a_transposed(self) -> Self {
        Self {
            transposed: true,
            seed: self.seed + 1,
            ..self
        }
    }
    fn lens(&self) -> [usize; 3] {
        let b = self.batch;
        match self.kind {
            Kind::Transpose => [self.m * self.n, self.m * self.n, 0],
            _ => [
                b * self.m * self.n,
                b * self.m * self.k,
                b * self.k * self.n,
            ],
        }
    }
}

/// Finite values of mixed magnitude: ±0, subnormals, ±2^53 beside small values
/// (so a sum depends on the order of its additions), and the pair whose product
/// rounds differently fused. No NaN or infinity, which would swamp the sums.
const ORDINARY: [u64; 24] = [
    0x3FF0_0000_0000_0000, // 1.0
    0xC000_0000_0000_0000, // -2.0
    0x3FE0_0000_0000_0000, // 0.5
    0x400A_0000_0000_0000, // 3.25
    0x8000_0000_0000_0000, // -0.0
    0x0000_0000_0000_0000, // +0.0
    0x4340_0000_0000_0000, // 2^53
    0xC340_0000_0000_0000, // -2^53
    0x3FF0_0000_0400_0000, // 1 + 2^-30
    0x3FEF_FFFF_FF80_0000, // 1 - 2^-30
    0x3FB9_9999_9999_999A, // 0.1
    0xC01C_0000_0000_0000, // -7.0
    0x4341_C379_37E0_8000, // 1e16
    0xC341_C379_37E0_8000, // -1e16
    0x0000_0000_0000_0001, // the smallest subnormal
    0x000F_FFFF_FFFF_FFFF, // the largest subnormal
    0x01A5_6E1F_C2F8_F359, // 1e-300
    0x4008_0000_0000_0000, // 3.0
    0xBFE8_0000_0000_0000, // -0.75
    0x4197_D784_0000_0000, // 1e8
    0x3E45_798E_E230_8C3A, // 1e-8
    0x405E_DD2F_1A9F_BE77, // 123.456
    0xBFF0_0000_0000_0000, // -1.0
    0x4000_0000_0000_0000, // 2.0
];

/// What a TRANSPOSE moves: NaNs with payloads, a signalling NaN, ±0, ±∞ and
/// subnormals — bits that any arithmetic on the path would change.
const SPECIAL: [u64; 12] = [
    0x7FF0_0000_0000_0001, // a signalling NaN, payload 1
    0x7FF8_0000_DEAD_BEEF, // a quiet NaN with a payload
    0xFFF8_0000_0000_0007, // a negative quiet NaN with a payload
    0x8000_0000_0000_0000, // -0.0
    0x0000_0000_0000_0000, // +0.0
    0x7FF0_0000_0000_0000, // +inf
    0xFFF0_0000_0000_0000, // -inf
    0x0000_0000_0000_0001, // the smallest subnormal
    0x800F_FFFF_FFFF_FFFF, // the largest negative subnormal
    0x3FF0_0000_0000_0000, // 1.0
    0xC340_0000_0000_0000, // -2^53
    0x4059_0000_0000_0000, // 100.0
];

/// Element `e` of the case's run `which` (० result, १ first operand, २ second).
fn element(c: &Case, which: usize, e: usize) -> u64 {
    let p = c.pool.len();
    c.pool[(e * MUL[which] + c.seed + which * 5) % p]
}
const MUL: [usize; 3] = [3, 7, 11];

/// The oracle: the expected words of one case — the count, then every element
/// of the result run in storage order.
fn oracle(c: &Case) -> Vec<u64> {
    let [lc, la, lb] = c.lens();
    let a: Vec<u64> = (0..la).map(|e| element(c, 1, e)).collect();
    let b: Vec<u64> = (0..lb).map(|e| element(c, 2, e)).collect();
    let mut out: Vec<u64> = (0..lc).map(|e| element(c, 0, e)).collect();
    let (m, k, n) = (c.m, c.k, c.n);
    match c.kind {
        Kind::Transpose => {
            for i in 0..m {
                for j in 0..n {
                    // input M×N, output N×M, each in the case's layout
                    let (src, dst) = if c.column_major {
                        (i + j * m, j + i * n)
                    } else {
                        (i * n + j, j * m + i)
                    };
                    out[dst] = a[src];
                }
            }
        }
        _ => {
            for t in 0..c.batch {
                for i in 0..m {
                    for j in 0..n {
                        let mut s = 0.0f64;
                        for kk in 0..k {
                            let (ai, bi) = if c.column_major {
                                (t * m * k + i + kk * m, t * k * n + kk + j * k)
                            } else if c.transposed {
                                // A stored K×M: A[i, k] is at k·M + i
                                (t * m * k + kk * m + i, t * k * n + kk * n + j)
                            } else {
                                (t * m * k + i * k + kk, t * k * n + kk * n + j)
                            };
                            let p = f64::from_bits(a[ai]) * f64::from_bits(b[bi]);
                            s += p;
                        }
                        let ci = if c.column_major {
                            t * m * n + i + j * m
                        } else {
                            t * m * n + i * n + j
                        };
                        out[ci] = if s.is_nan() {
                            CANONICAL_NAN
                        } else {
                            s.to_bits()
                        };
                    }
                }
            }
        }
    }
    let mut w = vec![lc as u64];
    w.extend(out);
    w
}

// ── the probe program ───────────────────────────────────────────────────────

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

/// A decimal numeral, `ऋण` for a negative.
fn dec(n: i128) -> String {
    const DIGITS: [&str; 10] = ["०", "१", "२", "३", "४", "५", "६", "७", "८", "९"];
    let s: String = n
        .unsigned_abs()
        .to_string()
        .chars()
        .map(|c| DIGITS[c.to_digit(10).unwrap() as usize])
        .collect();
    if n < 0 { format!("ऋण{s}") } else { s }
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

/// One probe module: `अष्टक` imported, the printer, and `मुख्यम्` with `body`.
fn module(body: &str) -> Vec<(&'static str, String)> {
    vec![(
        MODULE,
        format!(
            "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

{PRINTER}
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति ० ।
    चरः घ ॱॱ प६४ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    चरः गति ॱॱ न६४ भवति ० ।
    चरः परिमाणम् ॱॱ अ६४ भवति ० ।
{body}    प्रत्यागमनम् ० ।
इति
"
        ),
    )]
}

/// The pool, `सञ्चयः`, filled once per probe (it must be the SAME pool every
/// case of the probe reads: one probe, one pool).
fn pool_lines(pool: &[u64]) -> String {
    let mut s = String::from("    चरः सञ्चयः ॱॱ अङ्कः अन्तः प६४ भवति ० ।\n");
    for (i, b) in pool.iter().enumerate() {
        s.push_str(&format!(
            "    घ भवति अष्टकॱप्लवसंचारः {} ।\n    सञ्चयः अङ्कः {} अन्तः भवति घ ।\n",
            hex(*b),
            dec(i as i128)
        ));
    }
    s
}

/// Lines that fill run `run` with `len` elements of `which`, by the formula
/// [`element`] computes — as a loop, not unrolled.
fn fill_lines(c: &Case, run: &str, which: usize, len: usize) -> String {
    format!(
        "    चरः {run} ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    क्रमः भवति ० ।
    यावत् क्रमः न्यूनम् {len} आदि
        गति भवति क्रमः गुणनम् {mul} ।
        गति भवति गति योगः {add} ।
        गति भवति गति शेषः {p} ।
        {run} अङ्कः क्रमः अन्तः भवति सञ्चयः अङ्कः गति अन्तः ।
        क्रमः भवति क्रमः योगः १ ।
    इति
",
        len = dec(len as i128),
        mul = dec(MUL[which] as i128),
        add = dec((c.seed + which * 5) as i128),
        p = dec(c.pool.len() as i128),
    )
}

/// Lines that print the count and then `len` elements of `run`.
fn print_lines(run: &str, len: usize) -> String {
    format!(
        "    अवगणना भवति प्लवमुद्रणम् परिमाणम् ।
    क्रमः भवति ० ।
    यावत् क्रमः न्यूनम् {len} आदि
        घ भवति {run} अङ्कः क्रमः अन्तः ।
        अवगणना भवति प्लवमुद्रणम् आरभ्य अष्टकॱप्लवसंचारः घ समाप्तम् ।
        क्रमः भवति क्रमः योगः १ ।
    इति
",
        len = dec(len as i128)
    )
}

fn layout(column_major: bool) -> String {
    qualified(if column_major {
        COLUMN_MAJOR
    } else {
        ROW_MAJOR
    })
}

/// The layout constant case `c` is called with.
fn layout_of(c: &Case) -> String {
    if c.transposed {
        qualified(TRANSPOSED)
    } else {
        layout(c.column_major)
    }
}

/// The call of case `c` on runs named with suffix `d`, its arguments written
/// as given (`dims` overrides the dimensions, for the refusal probes).
fn call_text(c: &Case, d: &str, dims: Option<&[i128]>, lay: &str) -> String {
    let (f, a, b) = (format!("फल{d}"), format!("क{d}"), format!("ख{d}"));
    let given: Vec<i128> = match c.kind {
        Kind::Product => vec![c.m as i128, c.k as i128, c.n as i128],
        Kind::Tensor => vec![c.batch as i128, c.m as i128, c.k as i128, c.n as i128],
        Kind::Transpose => vec![c.m as i128, c.n as i128],
    };
    let dims: Vec<String> = dims.unwrap_or(&given).iter().map(|v| dec(*v)).collect();
    let (member, runs) = match c.kind {
        Kind::Product => (MATRIX_PRODUCT_MEMBER, vec![f, a, b]),
        Kind::Tensor => (TENSOR_MEMBER, vec![f, a, b]),
        Kind::Transpose => (MATRIX_TRANSPOSE_MEMBER, vec![f, a]),
    };
    let mut args = runs;
    args.extend(dims);
    args.push(lay.to_string());
    format!("{} आरभ्य {} समाप्तम्", qualified(member), args.join(" ऽ "))
}

/// The body for `cases`: one pool, then per case its runs, the call, and the
/// count and result printed. Runs `फल<d>`, `क<d>`, `ख<d>` with `lens` (or
/// the shape's own).
fn case_lines(c: &Case, d: &str, lens: Option<[usize; 3]>, call: &str) -> String {
    let [lc, la, lb] = lens.unwrap_or(c.lens());
    let mut s = String::new();
    s.push_str(&fill_lines(c, &format!("फल{d}"), 0, lc));
    s.push_str(&fill_lines(c, &format!("क{d}"), 1, la));
    if c.kind != Kind::Transpose {
        s.push_str(&fill_lines(c, &format!("ख{d}"), 2, lb));
    }
    s.push_str(&format!("    परिमाणम् भवति {call} ।\n"));
    s.push_str(&print_lines(&format!("फल{d}"), lc));
    s
}

fn probe(cases: &[Case]) -> Vec<(&'static str, String)> {
    let pool = cases[0].pool;
    assert!(cases.iter().all(|c| c.pool == pool), "one pool per probe");
    let mut body = pool_lines(pool);
    for (i, c) in cases.iter().enumerate() {
        let d = dec(i as i128);
        body.push_str(&case_lines(
            c,
            &d,
            None,
            &call_text(c, &d, None, &layout_of(c)),
        ));
    }
    module(&body)
}

fn want(cases: &[Case]) -> Vec<u64> {
    cases.iter().flat_map(oracle).collect()
}

// ── the engines ─────────────────────────────────────────────────────────────

fn words(out: &[u8]) -> Vec<u64> {
    out.chunks(8)
        .map(|c| {
            let mut w = [0u8; 8];
            w[..c.len()].copy_from_slice(c);
            u64::from_le_bytes(w)
        })
        .collect()
}

fn show(w: &[u64]) -> Vec<String> {
    w.iter().map(|x| format!("{x:#018x}")).collect()
}

/// The first place two word lists differ, for a readable failure.
fn first_difference(got: &[u64], want: &[u64]) -> String {
    if got == want {
        return "equal".into();
    }
    let at = got
        .iter()
        .zip(want)
        .position(|(g, w)| g != w)
        .unwrap_or(got.len().min(want.len()));
    format!(
        "{} words against {}, first difference at word {at}: got {:x?}, want {:x?}",
        got.len(),
        want.len(),
        got.get(at),
        want.get(at)
    )
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

fn octets(b: &[u8]) -> sadhana::t1::nirvahana::Value {
    sadhana::t1::nirvahana::Value::Octets(sadhana::t1::nirvahana::Octets::new(b))
}

fn arena(vs: Vec<sadhana::t1::nirvahana::Value>) -> sadhana::t1::nirvahana::Value {
    sadhana::t1::nirvahana::Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
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
    // DISPLAY, NOT DEBUG: `{:?}` escapes every Devanagari vowel sign and
    // virama, and a test that looks for a name in the refusal must find it.
    assert!(
        !image.is_empty(),
        "the probe built no image: refusal {}, link [{}]",
        sadhana::t1::chain::refusal_site(&it).unwrap_or_else(|| "none recorded".into()),
        sadhana::t1::chain::link_refusals(&it).join("; ")
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
    for (i, (m, src)) in srcs.iter().enumerate() {
        front.lex(src).expect("lex");
        front.parse().expect("parse");
        front.resolve().expect("resolve");
        front.typecheck().expect("typecheck");
        front
            .build_ir()
            .unwrap_or_else(|e| panic!("build_ir refused: {e}"));
        // The entry is the FIRST source's: a module may span two sources
        // (`second_source`), and only the first declares the entry.
        let entry = (i == 0 && *m == MODULE).then_some("मुख्यम्");
        let module = front.module(m, entry).expect("the module builds");
        allocates |= riscv64::module_allocates(&module);
        let text = riscv64::emit_module(&module)
            .unwrap_or_else(|e| panic!("the Rust emitter refused: {e}"));
        objects.push(to_object(&text, Some(m)));
    }
    let startup = to_object(
        &riscv64::emit_startup_object_with_records(Some(&format!("{MODULE}मुख्यम्")), allocates),
        Some("यन्त्रारम्भ"),
    );
    let mut all = vec![startup];
    all.extend(objects);
    let linked = sadhana::samyojana::link_at(&all, LOAD)
        .unwrap_or_else(|es| panic!("the Rust path does not link: {}", es.join("; ")));
    sadhana::kosha::write_debuggable_at(&linked.text, &linked.data, &[], linked.bss, &[], LOAD)
}

fn run_image(image: &[u8]) -> (Halt, Vec<u8>) {
    let mut m = Machine::load_elf(image, yantra::ram_for(image)).expect("the image loads");
    let mut out: Vec<u8> = Vec::new();
    let h = m.run(2_000_000_000, &mut out);
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

/// Both native images, built, run on `yantra`, compared with `want` and with
/// each other; answers the two images for a QEMU leg.
fn assert_native(what: &str, srcs: &[(&str, String)], want: &[u64]) -> [Vec<u8>; 2] {
    let t1 = t1_image(srcs);
    let (h, out) = run_image(&t1);
    assert_eq!(status(&h), 0, "{what}: native (.t1) finish {h:?}");
    let got = words(&out);
    assert!(
        got == want,
        "{what}: native (.t1): {}",
        first_difference(&got, want)
    );
    let rust = rust_twin_image(srcs);
    let (h, out) = run_image(&rust);
    assert_eq!(status(&h), 0, "{what}: rust twin finish {h:?}");
    let got = words(&out);
    assert!(
        got == want,
        "{what}: rust twin: {}",
        first_difference(&got, want)
    );
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
    [t1, rust]
}

fn panic_text(p: Box<dyn std::any::Any + Send>) -> String {
    p.downcast_ref::<String>()
        .cloned()
        .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_string()))
        .unwrap_or_default()
}

// ── (1) the shapes, interpreted and native, both layouts ────────────────────

/// The design's shapes: non-square, N not a multiple of VLMAX = 16, K = 0, and
/// three empty results.
const SHAPES: [(usize, usize, usize); 8] = [
    (1, 1, 1),
    (3, 5, 7),
    (2, 17, 33),
    (17, 3, 16),
    (4, 33, 1),
    (5, 0, 4),
    (0, 4, 5),
    (3, 4, 0),
];

fn product_cases(column_major: bool) -> Vec<Case> {
    SHAPES
        .iter()
        .map(|&(m, k, n)| Case::product(m, k, n, column_major))
        .collect()
}

fn tensor_cases() -> Vec<Case> {
    vec![
        Case::tensor(1, 3, 5, 7, false),
        Case::tensor(3, 2, 17, 17, false),
        Case::tensor(3, 4, 3, 5, true),
        Case::tensor(2, 0, 3, 5, false),
        Case::tensor(3, 2, 5, 17, false).a_transposed(),
    ]
}

/// `व्यूहॱपरिवर्तितम्`: C = Aᵀ·B over the same shapes.
fn transposed_cases() -> Vec<Case> {
    product_cases(false)
        .into_iter()
        .map(Case::a_transposed)
        .collect()
}

fn transpose_cases(column_major: bool) -> Vec<Case> {
    [
        (1, 1),
        (3, 5),
        (5, 3),
        (2, 17),
        (17, 2),
        (16, 16),
        (33, 3),
        (0, 4),
        (4, 0),
    ]
    .iter()
    .map(|&(m, n)| Case::transpose(m, n, column_major))
    .collect()
}

fn all_probes() -> Vec<(String, Vec<Case>)> {
    vec![
        ("product, row-major".into(), product_cases(false)),
        ("product, column-major".into(), product_cases(true)),
        ("product, A transposed".into(), transposed_cases()),
        ("tensor".into(), tensor_cases()),
        ("transpose, row-major".into(), transpose_cases(false)),
        ("transpose, column-major".into(), transpose_cases(true)),
    ]
}

#[test]
fn v009_every_shape_matches_the_oracle_interpreted() {
    for (what, cases) in all_probes() {
        let got = interpret_result(&probe(&cases)).unwrap_or_else(|e| panic!("{what}: {e}"));
        let w = want(&cases);
        assert!(
            got == w,
            "{what}: interpreted: {}",
            first_difference(&got, &w)
        );
    }
}

/// THE FALSIFIER'S MAIN LEG: every shape and both layouts, native (`.t1` chain)
/// bits == Rust-twin bits == interpreter bits == the oracle, the two images
/// octet-identical — and both images on QEMU printing the same octets.
#[test]
fn v009_every_shape_matches_the_oracle_natively_and_on_qemu() {
    for (what, cases) in all_probes() {
        let srcs = probe(&cases);
        let w = want(&cases);
        let interp = interpret_result(&srcs).unwrap_or_else(|e| panic!("{what}: interpreter: {e}"));
        assert!(
            interp == w,
            "{what}: interpreted: {}",
            first_difference(&interp, &w)
        );
        let images = assert_native(&what, &srcs, &w);
        for (engine, image) in [".t1 chain", "rust twin"].iter().zip(images) {
            let theirs = qemu_leg::run(&image).unwrap_or_else(|e| panic!("{what}, {engine}: {e}"));
            let got = words(&theirs);
            assert!(
                got == w,
                "{what}, {engine} on qemu: {}",
                first_difference(&got, &w)
            );
        }
    }
}

/// A TRANSPOSE TWICE IS THE IDENTITY, bit for bit — NaN payloads, the
/// signalling NaN and −0 included — and A COLUMN-MAJOR TRANSPOSE OF M×N IS A
/// ROW-MAJOR TRANSPOSE OF N×M, octet for octet (the permutation `ir.t1` makes).
#[test]
fn v009_a_transpose_round_trip_is_the_identity_and_the_layouts_agree() {
    let t = qualified(MATRIX_TRANSPOSE_MEMBER);
    for column_major in [false, true] {
        for (m, n) in [(3usize, 5usize), (17, 2), (1, 33)] {
            let c = Case::transpose(m, n, column_major);
            let mut body = pool_lines(c.pool);
            body.push_str(&fill_lines(&c, "क", 1, m * n));
            body.push_str(&fill_lines(&c, "फल", 0, m * n));
            body.push_str(&fill_lines(&c, "ग", 0, m * n));
            body.push_str(&fill_lines(&c, "ख", 2, m * n));
            body.push_str(&format!(
                "    परिमाणम् भवति {t} आरभ्य फल ऽ क ऽ {m} ऽ {n} ऽ {l} समाप्तम् ।
    परिमाणम् भवति {t} आरभ्य ग ऽ फल ऽ {n} ऽ {m} ऽ {l} समाप्तम् ।
",
                m = dec(m as i128),
                n = dec(n as i128),
                l = layout(column_major)
            ));
            body.push_str(&print_lines("ग", m * n));
            // the other layout's transpose of the transposed shape: the same octets
            body.push_str(&format!(
                "    परिमाणम् भवति {t} आरभ्य ख ऽ क ऽ {n} ऽ {m} ऽ {l} समाप्तम् ।
",
                m = dec(m as i128),
                n = dec(n as i128),
                l = layout(!column_major)
            ));
            body.push_str(&print_lines("फल", m * n));
            body.push_str(&print_lines("ख", m * n));
            let srcs = module(&body);
            let input: Vec<u64> = (0..m * n).map(|e| element(&c, 1, e)).collect();
            let what = format!("{m}×{n}, column-major {column_major}");
            let got = interpret_result(&srcs).unwrap_or_else(|e| panic!("{what}: {e}"));
            let mut w = vec![(m * n) as u64];
            w.extend(&input);
            let half = m * n + 1;
            assert!(
                got[..half] == w[..],
                "{what}: round trip: {}",
                first_difference(&got[..half], &w)
            );
            assert_eq!(
                got[half + 1..2 * half],
                got[2 * half + 1..],
                "{what}: the two layouts' transposes differ"
            );
            let [t1, _] = assert_native(&what, &srcs, &got);
            let theirs = qemu_leg::run(&t1).unwrap_or_else(|e| panic!("{what}: {e}"));
            assert_eq!(words(&theirs), got, "{what}: qemu");
        }
    }
}

// ── (2) the three probes a broken rule cannot pass ──────────────────────────

/// Lines that fill `run` with `bits`, element by element.
fn explicit_fill(run: &str, bits: &[u64]) -> String {
    let mut s = format!("    चरः {run} ॱॱ अङ्कः अन्तः प६४ भवति ० ।\n");
    for (i, b) in bits.iter().enumerate() {
        s.push_str(&format!(
            "    घ भवति अष्टकॱप्लवसंचारः {} ।\n    {run} अङ्कः {} अन्तः भवति घ ।\n",
            hex(*b),
            dec(i as i128)
        ));
    }
    s
}

const ONE: u64 = 0x3FF0_0000_0000_0000;
const NEG_ONE: u64 = 0xBFF0_0000_0000_0000;
const POS_ZERO: u64 = 0;
const NEG_ZERO: u64 = 0x8000_0000_0000_0000;
const TWO_53: u64 = 0x4340_0000_0000_0000;
const NEG_TWO_53: u64 = 0xC340_0000_0000_0000;
const ONE_PLUS: u64 = 0x3FF0_0000_0400_0000; // 1 + 2^-30
const ONE_MINUS: u64 = 0x3FEF_FFFF_FF80_0000; // 1 - 2^-30
const INF: u64 = 0x7FF0_0000_0000_0000;

/// A row-major product M×K by K×N over explicit operands, the sum as `sum`
/// computes it — the oracle's arithmetic with the ORDER and the FUSION as
/// parameters, so the probes can be shown to tell the orders apart.
fn explicit_product(
    m: usize,
    k: usize,
    n: usize,
    a: &[u64],
    b: &[u64],
    seed: u64,
    reversed: bool,
    fused: bool,
) -> Vec<u64> {
    let mut out = Vec::new();
    for i in 0..m {
        for j in 0..n {
            let mut s = f64::from_bits(seed);
            for step in 0..k {
                let kk = if reversed { k - 1 - step } else { step };
                let (x, y) = (f64::from_bits(a[i * k + kk]), f64::from_bits(b[kk * n + j]));
                s = if fused { x.mul_add(y, s) } else { s + x * y };
            }
            out.push(if s.is_nan() {
                CANONICAL_NAN
            } else {
                s.to_bits()
            });
        }
    }
    out
}

/// (m, k, n, A, B): the SEED probe — every product −0, so the sum is +0 only
/// from the +0.0 seed; the ORDER probe — `(2^53 + 1) − 2^53` is `0` forwards and
/// `1` backwards; the FUSION probe — `−1 + (1+2^-30)(1−2^-30)` is `0` with two
/// roundings and `−2^-60` fused; and the IEEE probe — ∞·0, NaN, ±0.
fn rule_probes() -> Vec<(&'static str, usize, usize, usize, Vec<u64>, Vec<u64>)> {
    vec![
        (
            "seed",
            1,
            2,
            1,
            vec![NEG_ONE, ONE],
            vec![POS_ZERO, NEG_ZERO],
        ),
        (
            "order",
            1,
            3,
            1,
            vec![TWO_53, ONE, NEG_TWO_53],
            vec![ONE, ONE, ONE],
        ),
        (
            "fusion",
            1,
            2,
            1,
            vec![NEG_ONE, ONE_PLUS],
            vec![ONE, ONE_MINUS],
        ),
        (
            "ieee",
            2,
            2,
            2,
            vec![INF, POS_ZERO, NEG_ONE, ONE],
            vec![ONE, NEG_ZERO, POS_ZERO, INF],
        ),
    ]
}

fn rule_program() -> (Vec<(&'static str, String)>, Vec<u64>) {
    let mut body = String::new();
    let mut w = Vec::new();
    for (i, (_, m, k, n, a, b)) in rule_probes().into_iter().enumerate() {
        let d = dec(i as i128);
        body.push_str(&explicit_fill(&format!("क{d}"), &a));
        body.push_str(&explicit_fill(&format!("ख{d}"), &b));
        body.push_str(&explicit_fill(&format!("फल{d}"), &vec![ONE; m * n]));
        body.push_str(&format!(
            "    परिमाणम् भवति {} आरभ्य फल{d} ऽ क{d} ऽ ख{d} ऽ {} ऽ {} ऽ {} ऽ {} समाप्तम् ।\n",
            qualified(MATRIX_PRODUCT_MEMBER),
            dec(m as i128),
            dec(k as i128),
            dec(n as i128),
            layout(false)
        ));
        body.push_str(&print_lines(&format!("फल{d}"), m * n));
        w.push((m * n) as u64);
        w.extend(explicit_product(m, k, n, &a, &b, POS_ZERO, false, false));
    }
    (module(&body), w)
}

/// THE PROBES ARE LOAD-BEARING: each tells the rule it is named for from the
/// broken one, in the oracle's own arithmetic, before any engine is asked.
#[test]
fn v009_the_rule_probes_tell_each_rule_from_its_breach() {
    for (what, m, k, n, a, b) in rule_probes() {
        let right = explicit_product(m, k, n, &a, &b, POS_ZERO, false, false);
        let (seed, order, fused) = (
            explicit_product(m, k, n, &a, &b, NEG_ZERO, false, false),
            explicit_product(m, k, n, &a, &b, POS_ZERO, true, false),
            explicit_product(m, k, n, &a, &b, POS_ZERO, false, true),
        );
        println!(
            "{what}: {} | seed −0 {} | reversed {} | fused {}",
            show(&right).join(","),
            show(&seed).join(","),
            show(&order).join(","),
            show(&fused).join(",")
        );
        match what {
            "seed" => assert_ne!(right, seed, "the seed probe cannot see a −0 seed"),
            "order" => assert_ne!(right, order, "the order probe cannot see a reversed sum"),
            "fusion" => assert_ne!(right, fused, "the fusion probe cannot see a fused step"),
            _ => assert!(
                right.contains(&CANONICAL_NAN),
                "the IEEE probe makes no NaN"
            ),
        }
    }
}

/// …AND EVERY ENGINE GIVES THE RIGHT ONE: the seed, order, fusion and IEEE
/// probes on the interpreter, both images on `yantra`, and QEMU.
#[test]
fn v009_the_rule_probes_match_the_oracle_on_every_engine() {
    let (srcs, w) = rule_program();
    let got = interpret_result(&srcs).unwrap_or_else(|e| panic!("interpreter: {e}"));
    assert_eq!(show(&got), show(&w), "interpreted");
    let images = assert_native("rule probes", &srcs, &w);
    for image in images {
        let theirs = qemu_leg::run(&image).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(show(&words(&theirs)), show(&w), "qemu");
    }
}

// ── (3) the refusals ────────────────────────────────────────────────────────

/// One call of `c` with runs of lengths `lens` and dimensions `dims`, and the
/// count printed — a probe that must REFUSE before printing anything.
fn refusal_probe(
    c: &Case,
    lens: [usize; 3],
    dims: Option<&[i128]>,
    alias: Option<&str>,
) -> Vec<(&'static str, String)> {
    let mut body = pool_lines(c.pool);
    let mut call = call_text(c, "", dims, &layout_of(c));
    if let Some(a) = alias {
        // the result run is handed again in place of operand `a`
        call = call.replacen(&format!(" ऽ {a} "), " ऽ फल ", 1);
    }
    body.push_str(&case_lines(c, "", Some(lens), &call));
    module(&body)
}

fn refusal_cases() -> Vec<(String, Vec<(&'static str, String)>, u64)> {
    let p = Case::product(3, 4, 5, false);
    let pc = Case::product(3, 4, 5, true);
    let t = Case::transpose(2, 3, false);
    let wide = 1i128 << 32;
    let half = 1i128 << 31;
    vec![
        (
            "product, result short".into(),
            refusal_probe(&p, [14, 12, 20], None, None),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "product, A long".into(),
            refusal_probe(&p, [15, 13, 20], None, None),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "product, B short".into(),
            refusal_probe(&p, [15, 12, 19], None, None),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "column-major, B short".into(),
            refusal_probe(&pc, [15, 12, 19], None, None),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "a dimension of 2^32".into(),
            refusal_probe(&p, [0, 0, 0], Some(&[wide, 0, 0]), None),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "a negative dimension".into(),
            refusal_probe(&p, [0, 0, 0], Some(&[-1, 0, 0]), None),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "tensor, a batch of 2^32".into(),
            refusal_probe(
                &Case::tensor(2, 1, 1, 1, false),
                [0, 0, 0],
                Some(&[wide, 0, 0, 0]),
                None,
            ),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "tensor B=2^31 M=2^31 K=4 N=0: each dimension below 2^32, M*K is 2^33 and B*M*K \
             wraps to 0 (the pairwise test, the review's probe)"
                .into(),
            refusal_probe(
                &Case::tensor(2, 1, 1, 1, false),
                [0, 0, 0],
                Some(&[half, half, 4, 0]),
                None,
            ),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "transpose, result short".into(),
            refusal_probe(&t, [5, 6, 0], None, None),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "transpose, operand long".into(),
            refusal_probe(&t, [6, 7, 0], None, None),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "product in place, C = A".into(),
            refusal_probe(&Case::product(3, 3, 3, false), [9, 9, 9], None, Some("क")),
            REFUSAL_ADHYASA,
        ),
        (
            "product in place, C = B".into(),
            refusal_probe(&Case::product(3, 3, 3, false), [9, 9, 9], None, Some("ख")),
            REFUSAL_ADHYASA,
        ),
        (
            "transpose in place".into(),
            refusal_probe(&t, [6, 6, 0], None, Some("क")),
            REFUSAL_ADHYASA,
        ),
    ]
}

fn is_refusal(h: &Halt, code: u64) -> bool {
    matches!(
        h,
        Halt::Finisher {
            value,
            status: Some(s)
        } if *value == sadhana::t1::riscv64::fail_word(code) && *s == code
    )
}

/// EVERY REFUSAL ON EVERY ENGINE, IDENTICALLY, before anything is printed: the
/// interpreter's cause names the code; both images halt at the finisher with
/// the code's FAIL word and the code as status; QEMU exits with its low octet.
#[test]
fn v009_the_refusals_are_the_same_on_every_engine() {
    for (what, srcs, code) in refusal_cases() {
        let shown = format!("{code:#x}");
        let e = interpret_result(&srcs).expect_err(&format!("{what}: the interpreter refuses"));
        assert!(e.contains(&shown), "{what}: interpreter: {e}");
        for (side, image) in [("t1", t1_image(&srcs)), ("rust", rust_twin_image(&srcs))] {
            let (h, out) = run_image(&image);
            assert!(is_refusal(&h, code), "{what}: {side} halted {h:?}");
            assert!(out.is_empty(), "{what}: {side} printed before refusing");
            if side == "t1" {
                let q = qemu_leg::run(&image);
                let exit = format!("exit status: {}", code & 0xff);
                assert!(
                    matches!(&q, Err(e) if e.contains(&exit)),
                    "{what}: qemu must exit {}, got {q:?}",
                    code & 0xff
                );
            }
        }
    }
}

/// THE CONTROLS: the same calls with every length right, and an EMPTY result
/// handed in place of its operand (nothing is written, so nothing is refused),
/// finish with status ० and print the count on every engine.
#[test]
fn v009_the_refusal_probes_run_when_nothing_is_wrong() {
    let p = Case::product(3, 4, 5, false);
    let t = Case::transpose(2, 3, false);
    for (what, srcs, count) in [
        ("product", refusal_probe(&p, p.lens(), None, None), 15u64),
        ("transpose", refusal_probe(&t, t.lens(), None, None), 6),
        (
            "empty product in place",
            refusal_probe(&Case::product(0, 3, 3, false), [0, 0, 9], None, Some("क")),
            0,
        ),
        (
            "empty transpose in place",
            refusal_probe(&Case::transpose(0, 0, false), [0, 0, 0], None, Some("क")),
            0,
        ),
    ] {
        let got = interpret_result(&srcs).unwrap_or_else(|e| panic!("{what}: {e}"));
        assert_eq!(got.first(), Some(&count), "{what}: interpreted");
        let [t1, _] = assert_native(what, &srcs, &got);
        assert_eq!(
            words(&qemu_leg::run(&t1).unwrap_or_else(|e| panic!("{what}: {e}"))),
            got,
            "{what}: qemu"
        );
    }
}

/// THE BUILD-TIME REFUSALS, the one named cause `FileMismatch` on both
/// compilers and the interpreter: a layout that is not a layout constant, a
/// layout constant used as a value, a run that is not `प६४`, a float
/// dimension — and a call of the wrong arity, refused by every engine.
#[test]
fn v009_a_malformed_call_is_refused_at_build() {
    let p = qualified(MATRIX_PRODUCT_MEMBER);
    let decls = "    चरः फल ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः क ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः प६४ भवति ० ।
    चरः न्य ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    चरः ल ॱॱ न६४ भवति ० ।
";
    let row = layout(false);
    let t = qualified(MATRIX_TRANSPOSE_MEMBER);
    let tr = qualified(TRANSPOSED);
    for (what, line, file_mismatch) in [
        (
            "a layout variable",
            format!("    परिमाणम् भवति {p} आरभ्य फल ऽ क ऽ ख ऽ ० ऽ ० ऽ ० ऽ ल समाप्तम् ।\n"),
            true,
        ),
        ("a layout as a value", format!("    ल भवति {row} ।\n"), true),
        (
            "an integer run",
            format!("    परिमाणम् भवति {p} आरभ्य फल ऽ न्य ऽ ख ऽ ० ऽ ० ऽ ० ऽ {row} समाप्तम् ।\n"),
            true,
        ),
        (
            "a float dimension",
            format!("    परिमाणम् भवति {p} आरभ्य फल ऽ क ऽ ख ऽ घ ऽ ० ऽ ० ऽ {row} समाप्तम् ।\n"),
            true,
        ),
        (
            "a transpose with the transposed layout",
            format!("    परिमाणम् भवति {t} आरभ्य फल ऽ क ऽ ० ऽ ० ऽ {tr} समाप्तम् ।\n"),
            true,
        ),
        (
            "six arguments",
            format!("    परिमाणम् भवति {p} आरभ्य फल ऽ क ऽ ख ऽ ० ऽ ० ऽ {row} समाप्तम् ।\n"),
            false,
        ),
    ] {
        let srcs = module(&format!("{decls}{line}"));
        let interp = interpret_result(&srcs);
        let e = interp.expect_err(&format!("{what}: the interpreter refuses"));
        if file_mismatch {
            assert!(e.contains("FileMismatch"), "{what}: interpreter: {e}");
        }
        for (side, r) in [
            (
                ".t1",
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t1_image(&srcs))),
            ),
            (
                "rust",
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| rust_twin_image(&srcs))),
            ),
        ] {
            let why = r.map(|_| String::from("built")).unwrap_or_else(panic_text);
            assert!(
                why.contains("FileMismatch"),
                "{what}: the {side} compiler: {why:.300}"
            );
        }
    }
}

// ── (4) the mutants, each shown red ─────────────────────────────────────────

/// A kernel row, as the mutants write it: `[verb,a,b,c]`. The kernel is ONE
/// compact table in `yantrotsarjana.t1` (two literals); a mutant decodes it
/// into these rows, replaces rows, and encodes it back (`kernel_chain`).
fn row(v: u16, a: u16, b: u16, c: u16) -> String {
    format!("[{v},{a},{b},{c}]")
}

/// The table's alphabet: `अ`..`ह` are 0..52, `ॲ`..`ॼ` 53..63.
fn letter_value(c: char) -> u16 {
    let cp = c as u32;
    u16::try_from(if cp >= 0x972 {
        cp - 0x972 + 53
    } else {
        cp - 0x905
    })
    .unwrap()
}
fn letter(v: u16) -> char {
    char::from_u32(if v < 53 {
        0x905 + u32::from(v)
    } else {
        0x972 + u32::from(v) - 53
    })
    .unwrap()
}
fn label_slot(verb: u16) -> usize {
    match verb {
        9 | 10 => 3,
        11 => 2,
        _ => 0,
    }
}

/// The literal of the `.t1` local `name` (`चरः <name> … भवति उक्तम् <lit> इति`).
fn literal_of<'a>(src: &'a str, name: &str) -> &'a str {
    let head = format!("चरः {name} ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् ");
    let at = src.find(&head).expect("the kernel literal") + head.len();
    let rest = &src[at..];
    &rest[..rest.find(" इति").unwrap()]
}

/// The kernel's rows decoded from `yantrotsarjana.t1`'s two literals, as text.
fn kernel_rows(src: &str) -> String {
    let ops: Vec<u16> = literal_of(src, "सूची").chars().map(letter_value).collect();
    let letters: Vec<u16> = literal_of(src, "संचयः").chars().map(letter_value).collect();
    let mut out = String::new();
    for r in letters.chunks(4) {
        let mut row_ = [r[0], r[1], r[2], r[3]];
        if r[0] > 0 {
            for p in 1..4 {
                let v = r[p];
                row_[p] = if v == 0 {
                    0
                } else if p == label_slot(r[0]) {
                    v + 1792
                } else {
                    let k = usize::from(v - 1) * 2;
                    ops[k] * 64 + ops[k + 1]
                };
            }
        }
        out.push_str(&row(row_[0], row_[1], row_[2], row_[3]));
    }
    out
}

/// Rows (as text) encoded back into the two literals, the dictionary rebuilt.
fn encode_rows(rows: &str) -> (String, String) {
    let rows: Vec<[u16; 4]> = rows
        .split(']')
        .filter(|r| !r.is_empty())
        .map(|r| {
            let v: Vec<u16> = r
                .trim_start_matches('[')
                .split(',')
                .map(|x| x.parse().unwrap())
                .collect();
            [v[0], v[1], v[2], v[3]]
        })
        .collect();
    let mut dict: Vec<u16> = rows
        .iter()
        .filter(|r| r[0] > 0)
        .flat_map(|r| {
            (1..4)
                .filter(|p| r[*p] != 0 && *p != label_slot(r[0]))
                .map(|p| r[p])
                .collect::<Vec<_>>()
        })
        .collect();
    dict.sort_unstable();
    dict.dedup();
    assert!(dict.len() <= 63, "a mutant needs {} operands", dict.len());
    let mut table = String::new();
    for r in &rows {
        table.push(letter(r[0]));
        for p in 1..4 {
            let v = if r[0] == 0 || r[p] == 0 {
                r[p]
            } else if p == label_slot(r[0]) {
                r[p] - 1792
            } else {
                u16::try_from(dict.iter().position(|x| *x == r[p]).unwrap() + 1).unwrap()
            };
            table.push(letter(v));
        }
    }
    let ops: String = dict
        .iter()
        .flat_map(|x| [letter(x / 64), letter(x % 64)])
        .collect();
    (table, ops)
}

/// `CHAIN` with the kernel's rows mutated: each `(live, mutant)` in turn, each
/// `live` matching exactly one run of rows.
fn kernel_chain(mutations: &[(String, String)]) -> Vec<(&'static str, String)> {
    CHAIN
        .iter()
        .map(|(n, s)| {
            if *n != "yantrotsarjana.t1" {
                return (*n, (*s).to_string());
            }
            let mut rows = kernel_rows(s);
            for (live, mutant) in mutations {
                assert_eq!(rows.matches(live.as_str()).count(), 1, "one site: {live}");
                rows = rows.replacen(live.as_str(), mutant, 1);
            }
            let (table, ops) = encode_rows(&rows);
            let src = s.replacen(literal_of(s, "संचयः"), &table, 1).replacen(
                literal_of(s, "सूची"),
                &ops,
                1,
            );
            (*n, src)
        })
        .collect()
}

/// THE MUTANT HARNESS IS EXACT: the kernel decoded and encoded back with no
/// mutation is the source's own two literals, letter for letter.
#[test]
fn v009_the_mutant_harness_round_trips_the_table() {
    let src = CHAIN
        .iter()
        .find(|(n, _)| *n == "yantrotsarjana.t1")
        .unwrap()
        .1;
    let (table, ops) = encode_rows(&kernel_rows(src));
    assert_eq!(table, literal_of(src, "संचयः"));
    assert_eq!(ops, literal_of(src, "सूची"));
}

/// A kernel operand: role (१ म्, २ न, ३ त्, ४ य्) × 256 + register.
fn reg(role: u16, r: &str) -> u16 {
    let n: u16 = match r {
        "zero" => 20,
        "sp" => 21,
        "ra" => 22,
        _ if r.starts_with('t') => r[1..].parse().unwrap(),
        _ => 10 + r[1..].parse::<u16>().unwrap(),
    };
    role * 256 + n
}
fn imm(v: i32) -> u16 {
    u16::try_from(1408 + v).unwrap()
}
// the verbs' codes
const ADD: u16 = 1;
const MULV: u16 = 2;
const OR: u16 = 3;
const SRLI: u16 = 4;
const SLLI: u16 = 5;
const SUB: u16 = 6;
const SD: u16 = 8;
const BEQ: u16 = 9;
const BNE: u16 = 10;
const SHAPE_LABEL: u16 = 1792 + 11;
const ALIAS_LABEL: u16 = 1792 + 12;

/// `CHAIN` with `file`'s ONE occurrence of `live` replaced by `mutant`.
fn mutated_chain(file: &str, live: &str, mutant: &str) -> Vec<(&'static str, String)> {
    CHAIN
        .iter()
        .map(|(n, s)| {
            if *n == file {
                assert_eq!(
                    s.matches(live).count(),
                    1,
                    "the mutation must match exactly one site of {file}: {live}"
                );
                (*n, s.replacen(live, mutant, 1))
            } else {
                (*n, (*s).to_string())
            }
        })
        .collect()
}

/// The `.t1` chain's image of `srcs` under a mutated chain, run on `yantra`.
fn native_mutated(chain: &[(&'static str, String)], srcs: &[(&str, String)]) -> (Halt, Vec<u64>) {
    let refs: Vec<(&str, &str)> = chain.iter().map(|(n, s)| (*n, s.as_str())).collect();
    let (h, out) = run_image(&t1_image_with(&refs, srcs));
    (h, words(&out))
}

/// A mutant of the kernel's rows (`live` → `mutant`, in `yantrotsarjana.t1`)
/// must print something other than the oracle on `red`, and the oracle's own
/// answer on `green` (where the breach cannot show).
fn kernel_mutant(name: &str, live: &str, mutant: &str, red: &[Case], green: &[Case]) {
    let chain = kernel_chain(&[(live.to_string(), mutant.to_string())]);
    for c in red {
        let (h, got) = native_mutated(&chain, &probe(&[*c]));
        println!("{name}, {c:?}: {h:?}");
        assert!(
            !matches!(
                h,
                Halt::Finisher {
                    status: Some(0),
                    ..
                }
            ) || got != oracle(c),
            "mutant {name} printed the right answer for {c:?}"
        );
    }
    for c in green {
        let (h, got) = native_mutated(&chain, &probe(&[*c]));
        assert_eq!(status(&h), 0, "mutant {name} on {c:?}");
        assert_eq!(
            got,
            oracle(c),
            "mutant {name} is wrong where it cannot be: {c:?}"
        );
    }
}

/// THE TAIL SKIPPED: a row's column count rounded down to whole strips of 16.
#[test]
fn v009_mutant_the_tail_skipped_is_red() {
    let live = row(ADD, reg(1, "t1"), reg(2, "a6"), imm(0));
    let mutant = row(SRLI, reg(1, "t1"), reg(2, "a6"), imm(4))
        + &row(SLLI, reg(1, "t1"), reg(2, "t1"), imm(4));
    kernel_mutant(
        "tail",
        &live,
        &mutant,
        &[
            Case::product(2, 3, 17, false),
            Case::product(3, 5, 7, false),
            Case::product(1, 2, 33, false),
        ],
        &[Case::product(2, 3, 16, false)],
    );
}

/// THE WRONG STRIDE IN B: one element instead of a row per `k`.
#[test]
fn v009_mutant_the_wrong_b_stride_is_red() {
    let live = row(ADD, reg(1, "t5"), reg(2, "t5"), reg(2, "a7"));
    let mutant = row(ADD, reg(1, "t5"), reg(2, "t5"), imm(8));
    kernel_mutant(
        "B stride",
        &live,
        &mutant,
        &[Case::product(3, 5, 7, false)],
        &[Case::product(3, 5, 1, false)],
    );
}

/// THE WRONG STRIDE IN THE TRANSPOSE: M·8 for N·8, so only a square is right.
#[test]
fn v009_mutant_the_wrong_transpose_stride_is_red() {
    let live = row(SLLI, reg(1, "a7"), reg(2, "a3"), imm(3));
    let mutant = row(SLLI, reg(1, "a7"), reg(2, "a2"), imm(3));
    kernel_mutant(
        "transpose stride",
        &live,
        &mutant,
        &[Case::transpose(2, 17, false), Case::transpose(3, 5, true)],
        &[Case::transpose(16, 16, false)],
    );
}

/// THE WRONG SEED, −0.0 for +0.0: red on the seed probe only.
#[test]
fn v009_mutant_the_wrong_seed_is_red() {
    let live = row(SD, reg(4, "sp"), imm(0), reg(2, "zero"));
    let mutant = row(ADD, reg(1, "t4"), reg(2, "zero"), imm(-1))
        + &row(SLLI, reg(1, "t4"), reg(2, "t4"), imm(63))
        + &row(SD, reg(4, "sp"), imm(0), reg(2, "t4"));
    let chain = kernel_chain(&[(live, mutant)]);
    let (srcs, w) = rule_program();
    let (h, got) = native_mutated(&chain, &srcs);
    assert_eq!(status(&h), 0, "{h:?}");
    assert_ne!(got, w, "the −0 seed printed the right answer");
    // only the seed probe moved: its one element, word १
    let diff: Vec<usize> = (0..w.len()).filter(|i| got[*i] != w[*i]).collect();
    assert_eq!(diff, vec![1], "the −0 seed moved {diff:?}");
    assert_eq!(got[1], NEG_ZERO);
}

/// THE WRONG ORDER: the `k` loop run backwards — red on the order probe.
#[test]
fn v009_mutant_the_sum_reversed_is_red() {
    let chain = kernel_chain(&[
        (
            // A's k step is a3 (8, or M·8 for the transposed layout)
            row(ADD, reg(1, "t4"), reg(2, "a1"), imm(0)),
            row(MULV, reg(1, "t4"), reg(2, "a5"), reg(2, "a3"))
                + &row(ADD, reg(1, "t4"), reg(2, "t4"), reg(2, "a1"))
                + &row(SUB, reg(1, "t4"), reg(2, "t4"), reg(2, "a3")),
        ),
        (
            row(ADD, reg(1, "t5"), reg(2, "t2"), imm(0)),
            row(MULV, reg(1, "t5"), reg(2, "a5"), reg(2, "a7"))
                + &row(ADD, reg(1, "t5"), reg(2, "t5"), reg(2, "t2"))
                + &row(SUB, reg(1, "t5"), reg(2, "t5"), reg(2, "a7")),
        ),
        (
            row(ADD, reg(1, "t4"), reg(2, "t4"), reg(2, "a3")),
            row(SUB, reg(1, "t4"), reg(2, "t4"), reg(2, "a3")),
        ),
        (
            row(ADD, reg(1, "t5"), reg(2, "t5"), reg(2, "a7")),
            row(SUB, reg(1, "t5"), reg(2, "t5"), reg(2, "a7")),
        ),
    ]);
    let (srcs, w) = rule_program();
    let (h, got) = native_mutated(&chain, &srcs);
    assert_eq!(status(&h), 0, "{h:?}");
    // the order probe is case १: words २ (count) and ३ (its element)
    assert_ne!(got[3], w[3], "the reversed sum printed the forward answer");
    let rev = explicit_product(
        1,
        3,
        1,
        &[TWO_53, ONE, NEG_TWO_53],
        &[ONE, ONE, ONE],
        POS_ZERO,
        true,
        false,
    );
    assert_eq!(
        got[3], rev[0],
        "the mutant is the reversed sum, no other change"
    );
}

/// THE LAYOUT IGNORED (`ir.t1` drops the permutation): a column-major product
/// is computed row-major, so a non-square one goes red; row-major stays green.
#[test]
fn v009_mutant_the_layout_ignored_is_red() {
    // the permutation's one test (ir.t1, `आव्यूहान्तर्निहितरचना`), never taken
    let live = "    यदि प्रधानम् समम् २ आदि\n        चरः विनिमयः";
    let mutant = "    यदि प्रधानम् समम् ४ आदि\n        चरः विनिमयः";
    let chain = mutated_chain("ir.t1", live, mutant);
    let red = Case::product(3, 5, 7, true);
    let (h, got) = native_mutated(&chain, &probe(&[red]));
    assert!(
        status(&h) != 0 || got != oracle(&red),
        "the layout is not load-bearing"
    );
    let green = Case::product(3, 5, 7, false);
    let (h, got) = native_mutated(&chain, &probe(&[green]));
    assert_eq!(status(&h), 0);
    assert_eq!(got, oracle(&green));
}

/// THE SHAPE TEST REMOVED (A's length no longer compared): the long-A probe
/// is no longer refused.
#[test]
fn v009_mutant_without_the_shape_test_is_red() {
    let live = row(BNE, reg(2, "t4"), reg(3, "t1"), SHAPE_LABEL);
    let chain = kernel_chain(&[(live, String::new())]);
    let p = Case::product(3, 4, 5, false);
    let (h, _) = native_mutated(&chain, &refusal_probe(&p, [15, 13, 20], None, None));
    println!("{h:?}");
    assert!(
        !is_refusal(&h, VECTOR_LENGTH_REFUSAL),
        "the shape test is not load-bearing"
    );
}

/// THE ALIAS TEST REMOVED: a product in place is no longer refused.
#[test]
fn v009_mutant_without_the_alias_test_is_red() {
    // the PRODUCT's test of C against A (the transpose has its own, unchanged)
    let second = row(BEQ, reg(2, "a0"), reg(3, "a2"), ALIAS_LABEL);
    let live = row(BEQ, reg(2, "a0"), reg(3, "a1"), ALIAS_LABEL) + &second;
    let chain = kernel_chain(&[(live, second)]);
    let (h, _) = native_mutated(
        &chain,
        &refusal_probe(&Case::product(3, 3, 3, false), [9, 9, 9], None, Some("क")),
    );
    println!("{h:?}");
    assert!(
        !is_refusal(&h, REFUSAL_ADHYASA),
        "the alias test is not load-bearing"
    );
}

/// THE PAIRWISE TEST REMOVED (the review's follow-up 2): the three products
/// of two dimensions no longer ORed into the below-2^32 test. Then B=2^31,
/// M=2^31, K=4, N=0 — every dimension below 2^32, M·K = 2^33 — wraps B·M·K to
/// 0 against A's length 0, and the call is no longer refused. A dimension of
/// 2^32 is still refused (the dimension test is untouched), so the mutant
/// breaks exactly the pairwise rule.
#[test]
fn v009_mutant_without_the_pairwise_test_is_red() {
    let live = [reg(2, "t4"), reg(2, "t5"), reg(2, "t6")]
        .map(|r| row(OR, reg(1, "t3"), reg(2, "t3"), r))
        .concat();
    let chain = kernel_chain(&[(live, String::new())]);
    let half = 1i128 << 31;
    let wide = 1i128 << 32;
    let tensor = Case::tensor(2, 1, 1, 1, false);
    let (h, _) = native_mutated(
        &chain,
        &refusal_probe(&tensor, [0, 0, 0], Some(&[half, half, 4, 0]), None),
    );
    println!("{h:?}");
    assert!(
        !is_refusal(&h, VECTOR_LENGTH_REFUSAL),
        "the pairwise test is not load-bearing"
    );
    let (h, _) = native_mutated(
        &chain,
        &refusal_probe(&tensor, [0, 0, 0], Some(&[wide, 0, 0, 0]), None),
    );
    assert!(
        is_refusal(&h, VECTOR_LENGTH_REFUSAL),
        "the mutant broke more than the pairwise test: {h:?}"
    );
}

// ── (5) the names, and the kernel's text ────────────────────────────────────

fn crate_source(rel: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(rel),
    )
    .unwrap_or_else(|e| panic!("{rel}: {e}"))
}

/// The body of the `.t1` routine `name`.
fn t1_routine<'a>(src: &'a str, name: &str) -> &'a str {
    let head = src
        .find(&format!("वृत्तिः {name} आदाय"))
        .unwrap_or_else(|| panic!("no routine {name}"));
    let body = &src[head..];
    &body[..body.find("\nइति\n").expect("the routine closes")]
}

/// Every word of every `उक्तम् … इति` literal in `body` (a literal may be a
/// space-separated LIST, read by `अर्थॱसूचीपदक्रमः`).
fn literals(body: &str) -> std::collections::BTreeSet<String> {
    body.split("उक्तम् ")
        .skip(1)
        .flat_map(|r| {
            r.split_whitespace()
                .take_while(|w| *w != "इति")
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// THE NAMES ARE ONE LIST in every place that spells them — `artha.t1`'s
/// trust, `ir.t1`'s matcher and layout reader, `shrinkhala.t1`'s kernel names
/// and `nirvahana.rs` (which `chain.rs` and this test read).
#[test]
fn v009_the_matrix_names_are_the_same_in_every_source() {
    let artha = crate_source("crates/sadhana-t1/src/artha.t1");
    let ir = crate_source("crates/sadhana-t1/src/ir.t1");
    let shr = crate_source("crates/sadhana-t1/src/shrinkhala.t1");
    let set = |v: &[&str]| {
        v.iter()
            .map(|s| (*s).to_string())
            .collect::<std::collections::BTreeSet<_>>()
    };
    assert_eq!(
        literals(t1_routine(&artha, "आव्यूहसदस्यः")),
        set(&[
            MATRIX_PRODUCT_MEMBER,
            MATRIX_TRANSPOSE_MEMBER,
            TENSOR_MEMBER,
            ROW_MAJOR,
            COLUMN_MAJOR,
            TRANSPOSED
        ])
    );
    let matcher = literals(t1_routine(&ir, "व्यूहान्तर्निहितम्"));
    for m in [
        MATRIX_PRODUCT_MEMBER,
        MATRIX_TRANSPOSE_MEMBER,
        TENSOR_MEMBER,
    ] {
        assert!(matcher.contains(m), "ir.t1's matcher lacks {m}");
    }
    assert_eq!(
        literals(t1_routine(&ir, "प्रधानभेदः")),
        set(&[
            &qualified(ROW_MAJOR),
            &qualified(COLUMN_MAJOR),
            &qualified(TRANSPOSED)
        ])
    );
    let kernel_names = literals(t1_routine(&shr, "नामसञ्चयः"));
    for m in [MATRIX_PRODUCT_MEMBER, MATRIX_TRANSPOSE_MEMBER] {
        assert!(
            kernel_names.contains(m),
            "shrinkhala.t1 does not name the kernel {m}"
        );
    }
}

/// THE KERNEL'S TEXT IS THE DESIGN'S: ONE copy per module however many calls,
/// both entries exported, `v0` never named, and a module with no matrix call
/// carries none of it.
#[test]
fn v009_one_kernel_per_module_and_none_without_a_call() {
    let text = rust_module_text(&probe(&product_cases(false)[..3]));
    let entry = format!("॥ वैश्विकम् {MODULE}{MATRIX_PRODUCT_MEMBER} ॥");
    assert_eq!(
        text.matches(&entry).count(),
        1,
        "one product kernel:\n{text}"
    );
    assert_eq!(
        text.matches(&format!("॥ वैश्विकम् {MODULE}{MATRIX_TRANSPOSE_MEMBER} ॥"))
            .count(),
        1
    );
    let v0 = format!("{}०", sadhana::t1::riscv64::VECTOR_REGISTER_STEM);
    for line in text.lines() {
        for word in line.split_whitespace() {
            assert_ne!(
                word.trim_end_matches(['म', 'न', 'त', 'य', '्']),
                v0,
                "v0 is named: {line}"
            );
        }
    }
    let none = rust_module_text(&module("    परिमाणम् भवति ० ।\n"));
    assert!(
        !none.contains(MATRIX_PRODUCT_MEMBER),
        "a module with no matrix call carries the kernel"
    );
}

/// THE TRANSPOSED LAYOUT IGNORED: the kernel never swaps A's strides, so an
/// Aᵀ product of a non-square A goes red, and a row-major one stays green.
#[test]
fn v009_mutant_the_transposed_layout_ignored_is_red() {
    const JAL: u16 = 11;
    let live = row(BEQ, reg(2, "a7"), reg(3, "zero"), 1792 + 17);
    let mutant = row(JAL, reg(1, "zero"), 1792 + 17, 0);
    kernel_mutant(
        "Aᵀ ignored",
        &live,
        &mutant,
        &[
            Case::product(3, 5, 7, false).a_transposed(),
            Case::tensor(3, 2, 5, 17, false).a_transposed(),
        ],
        &[Case::product(3, 5, 7, false)],
    );
}

/// THE KERNEL'S OUTPUT, DUMPED for a byte-for-byte comparison across commits
/// (the compaction of 2026-10-06 must change no octet): the Rust emitter's
/// module text, the `.t1` chain's image and the Rust twin's image of one
/// matrix-using module, written under `$V009_DUMP`. Run explicitly.
#[test]
#[ignore = "diagnostic: writes files; run with V009_DUMP=<dir> to compare two commits"]
fn v009_dump_the_kernel_output() {
    let Some(dir) = std::env::var_os("V009_DUMP") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut cases = product_cases(false);
    cases.extend(transposed_cases());
    let srcs = probe(&cases);
    std::fs::write(dir.join("rust.t0"), rust_module_text(&srcs)).unwrap();
    std::fs::write(dir.join("t1.elf"), t1_image(&srcs)).unwrap();
    std::fs::write(dir.join("rust.elf"), rust_twin_image(&srcs)).unwrap();
    let t = probe(&transpose_cases(true));
    std::fs::write(dir.join("t1-transpose.elf"), t1_image(&t)).unwrap();
    std::fs::write(dir.join("rust-transpose.t0"), rust_module_text(&t)).unwrap();
}

// ── (6) THE RESERVED KERNEL NAMES (the review's follow-up 1, `W-381`'s rule) ─

/// The phrase every engine's refusal carries after the member's name.
const RESERVED: &str = "is reserved for the matrix built-in in a module that uses it";

/// THE REVIEW'S LABEL-COLLISION PROBE: a user routine named `routine`
/// (answering its argument plus 7), called with 5 and its answer printed,
/// in a module that makes a 1×1 product call when `use_matrix`.
fn collision_source(routine: &str, use_matrix: bool) -> Vec<(&'static str, String)> {
    let c = Case::product(1, 1, 1, false);
    let mut body = pool_lines(c.pool);
    if use_matrix {
        body.push_str(&case_lines(
            &c,
            "",
            None,
            &call_text(&c, "", None, &layout(false)),
        ));
    }
    body.push_str(&format!(
        "    अवगणना भवति {routine} ५ ।\n    अवगणना भवति प्लवमुद्रणम् अवगणना ।\n"
    ));
    let mut srcs = module(&body);
    let user = format!(
        "वृत्तिः {routine} आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि\n    प्रत्यागमनम् मूल्यम् योगः ७ ।\nइति\n\n"
    );
    let s = &mut srcs[0].1;
    let at = s.find("सार्वजनिक वृत्तिः मुख्यम्").unwrap();
    s.insert_str(at, &user);
    srcs
}

/// A ROUTINE NAMED LIKE THE KERNEL, IN A MODULE THAT MAKES A MATRIX CALL, IS
/// REFUSED BY EVERY ENGINE. A module that calls any matrix built-in is given
/// both kernel routines, named by the product's and the transpose's members, so
/// a user routine of either name collides with one natively. The interpreter
/// mirrors that rule exactly and refuses at load; the two compilers refuse at
/// build. Every refusal names the member and says it is reserved.
#[test]
fn v009_a_routine_named_like_the_kernel_is_refused_by_every_engine() {
    for member in [MATRIX_PRODUCT_MEMBER, MATRIX_TRANSPOSE_MEMBER] {
        let srcs = collision_source(member, true);
        let said = format!("`{member}` {RESERVED}");
        let e = interpret_result(&srcs)
            .expect_err(&format!("{member}: the interpreter refuses the load"));
        assert!(e.contains(&said), "{member}: interpreter: {e}");
        let builds: [(&str, fn(&[(&str, String)]) -> Vec<u8>); 2] =
            [(".t1", t1_image), ("rust", rust_twin_image)];
        for (side, build) in builds {
            let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| build(&srcs)))
                .map(|_| ())
                .map_err(panic_text);
            let text = refused.expect_err(&format!("{member}: the {side} build refuses"));
            assert!(text.contains(&said), "{member}: {side}: {text}");
        }
    }
}

/// THE CONTROLS: each member's name as a routine in a module with NO matrix
/// call, and the tensor member's — which names no kernel routine — beside a
/// matrix call, run on every engine (QEMU included) and print the same words.
#[test]
fn v009_the_kernel_names_are_free_without_a_matrix_call() {
    let mut cases: Vec<(&str, bool)> = [
        MATRIX_PRODUCT_MEMBER,
        MATRIX_TRANSPOSE_MEMBER,
        TENSOR_MEMBER,
    ]
    .map(|m| (m, false))
    .to_vec();
    cases.push((TENSOR_MEMBER, true));
    for (member, matrix) in cases {
        let what = format!("routine {member}, matrix call {matrix}");
        let srcs = collision_source(member, matrix);
        let got = interpret_result(&srcs).unwrap_or_else(|e| panic!("{what}: {e}"));
        assert_eq!(got.last(), Some(&12), "{what}: the routine answered 5 + 7");
        let [t1, _] = assert_native(&what, &srcs, &got);
        assert_eq!(
            words(&qemu_leg::run(&t1).unwrap_or_else(|e| panic!("{what}: {e}"))),
            got,
            "{what}: qemu"
        );
    }
}

// ── (6b) A KERNEL MEMBER AS A GLOBAL, AND IN A SECOND SOURCE (follow-ups 2) ──

/// A module GLOBAL named `name`, public or private, holding 9 and printed from
/// the entry, in a module that makes a 1×1 product call when `use_matrix`.
fn global_source(name: &str, use_matrix: bool, public: bool) -> Vec<(&'static str, String)> {
    let c = Case::product(1, 1, 1, false);
    let mut body = pool_lines(c.pool);
    if use_matrix {
        body.push_str(&case_lines(
            &c,
            "",
            None,
            &call_text(&c, "", None, &layout(false)),
        ));
    }
    body.push_str(&format!("    अवगणना भवति प्लवमुद्रणम् {name} ।\n"));
    let mut srcs = module(&body);
    let global = format!(
        "{}चरः {name} ॱॱ न६४ भवति {} ।\n\n",
        if public {
            "सार्वजनिक "
        } else {
            ""
        },
        dec(9)
    );
    let s = &mut srcs[0].1;
    let at = s.find("सार्वजनिक वृत्तिः मुख्यम्").unwrap();
    s.insert_str(at, &global);
    srcs
}

/// The module split over TWO sources: the first makes the matrix call when
/// `use_matrix` (and so carries the kernel), the second declares a routine
/// named `routine`.
fn second_source(routine: &str, use_matrix: bool) -> Vec<(&'static str, String)> {
    let mut srcs = collision_source("", use_matrix);
    let first = &mut srcs[0].1;
    // drop the empty-named routine and its call, which collision_source wrote
    let user = format!(
        "वृत्तिः {routine} आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि\n    प्रत्यागमनम् मूल्यम् योगः ७ ।\nइति\n\n",
        routine = ""
    );
    *first = first.replacen(&user, "", 1);
    let call = first
        .lines()
        .find(|l| l.contains(" ५ ।"))
        .unwrap()
        .to_string()
        + "\n";
    *first = first.replacen(&call, "", 1);
    let head = &first[..first.find("॥").unwrap() + "॥".len()];
    let second = format!(
        "{head}\n\nवृत्तिः {routine} आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि\n    प्रत्यागमनम् मूल्यम् योगः ७ ।\nइति\n\n"
    );
    srcs.push((MODULE, second));
    srcs
}

/// The refusal text of a native build of `srcs`, read from its panic.
fn native_refusal(
    side: &str,
    build: fn(&[(&str, String)]) -> Vec<u8>,
    srcs: &[(&str, String)],
) -> String {
    let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| build(srcs)))
        .map(|_| ())
        .map_err(panic_text);
    refused.expect_err(&format!("the {side} build refuses"))
}

/// A GLOBAL NAMED LIKE THE KERNEL, BESIDE A MATRIX CALL, IS REFUSED BY EVERY
/// ENGINE (the review of the follow-ups: it ran in the interpreter, printing
/// 9, and was refused natively only at link as a duplicate symbol). Public and
/// private alike: both are labels `<module><name>`.
#[test]
fn v009_a_global_named_like_the_kernel_is_refused_by_every_engine() {
    for member in [MATRIX_PRODUCT_MEMBER, MATRIX_TRANSPOSE_MEMBER] {
        for public in [false, true] {
            let what = format!("global {member}, public {public}");
            let srcs = global_source(member, true, public);
            let said = format!("`{member}` {RESERVED}");
            let e = interpret_result(&srcs)
                .expect_err(&format!("{what}: the interpreter refuses the load"));
            assert!(e.contains(&said), "{what}: interpreter: {e}");
            let builds: [(&str, fn(&[(&str, String)]) -> Vec<u8>); 2] =
                [(".t1", t1_image), ("rust", rust_twin_image)];
            for (side, build) in builds {
                let text = native_refusal(side, build, &srcs);
                assert!(text.contains(&said), "{what}: {side}: {text}");
            }
        }
    }
}

/// THE CONTROLS: the same globals with no matrix call, and a global named by
/// the tensor member beside one, run on every engine (QEMU included).
#[test]
fn v009_a_global_named_like_the_kernel_is_free_without_a_matrix_call() {
    let mut cases: Vec<(&str, bool, bool)> = Vec::new();
    for member in [MATRIX_PRODUCT_MEMBER, MATRIX_TRANSPOSE_MEMBER] {
        cases.push((member, false, false));
        cases.push((member, false, true));
    }
    cases.push((TENSOR_MEMBER, true, true));
    for (member, matrix, public) in cases {
        let what = format!("global {member}, matrix call {matrix}, public {public}");
        let srcs = global_source(member, matrix, public);
        let got = interpret_result(&srcs).unwrap_or_else(|e| panic!("{what}: {e}"));
        assert_eq!(got.last(), Some(&9), "{what}: the global's value");
        let [t1, _] = assert_native(&what, &srcs, &got);
        assert_eq!(
            words(&qemu_leg::run(&t1).unwrap_or_else(|e| panic!("{what}: {e}"))),
            got,
            "{what}: qemu"
        );
    }
}

/// A KERNEL-NAMED ROUTINE IN A SECOND SOURCE OF A MODULE WHOSE FIRST SOURCE
/// MAKES THE MATRIX CALL: the interpreter merges the two and refuses at load;
/// natively the two objects both define the label, refused at link — and that
/// refusal says the name is reserved too.
#[test]
fn v009_a_kernel_named_routine_in_a_second_source_is_refused_by_every_engine() {
    for member in [MATRIX_PRODUCT_MEMBER, MATRIX_TRANSPOSE_MEMBER] {
        let srcs = second_source(member, true);
        let said = format!("`{member}` {RESERVED}");
        let e = interpret_result(&srcs)
            .expect_err(&format!("{member}: the interpreter refuses the load"));
        assert!(e.contains(&said), "{member}: interpreter: {e}");
        let builds: [(&str, fn(&[(&str, String)]) -> Vec<u8>); 2] =
            [(".t1", t1_image), ("rust", rust_twin_image)];
        for (side, build) in builds {
            let text = native_refusal(side, build, &srcs);
            assert!(text.contains(&said), "{member}: {side}: {text}");
        }
    }
}

/// The definite clause of the reserved wording: said only where the module's
/// matrix kernel is KNOWN to be one of the two definitions.
const DEFINITE: &str = "this module makes a";

/// A routine named `name` (answering its argument plus 7), as a declaration.
fn routine_decl(name: &str) -> String {
    format!(
        "वृत्तिः {routine} आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि\n    प्रत्यागमनम् मूल्यम् योगः ७ ।\nइति\n\n",
        routine = name
    )
}

/// A private global named `name` holding 9, as a declaration.
fn global_decl(name: &str) -> String {
    format!("चरः {name} ॱॱ न६४ भवति {} ।\n\n", dec(9))
}

/// A source of `module` (the probe's header, renamed) holding `decls`.
fn extra_source(first: &str, module: &'static str, decls: &str) -> (&'static str, String) {
    let head = &first[..first.find("॥").unwrap() + "॥".len()];
    (
        module,
        format!("{}\n\n{decls}", head.replacen(MODULE, module, 1)),
    )
}

/// THREE DUPLICATES WITH NO MATRIX CALL ANYWHERE (the follow-ups 2 review):
/// (a) two modules whose labels concatenate to one word — the probe module's
/// first eight letters as a module with a
/// global `<tail><product member>` and the probe module with a global named by
/// the product member; (b) a kernel-named routine in BOTH sources of one
/// module; (c) both kernel-named routines in one source and a kernel-named
/// global in the other. Every native build refuses them as a duplicate symbol,
/// and no refusal may say the module makes a matrix call: the `.t1` report says
/// at most the conditional, and the Rust linker — which sees the objects — says
/// nothing of the kernel at all.
fn duplicates_without_the_kernel() -> Vec<(&'static str, Vec<(&'static str, String)>)> {
    let split = MODULE.char_indices().nth(8).unwrap().0;
    let (head, tail) = MODULE.split_at(split);
    let mut a = global_source(MATRIX_PRODUCT_MEMBER, false, false);
    let other = extra_source(
        &a[0].1,
        head,
        &global_decl(&format!("{tail}{MATRIX_PRODUCT_MEMBER}")),
    );
    a.push(other);
    let mut b = collision_source(MATRIX_PRODUCT_MEMBER, false);
    let other = extra_source(&b[0].1, MODULE, &routine_decl(MATRIX_PRODUCT_MEMBER));
    b.push(other);
    let mut c = collision_source(MATRIX_PRODUCT_MEMBER, false);
    let at = c[0].1.find("वृत्तिः ").unwrap();
    c[0].1
        .insert_str(at, &routine_decl(MATRIX_TRANSPOSE_MEMBER));
    let other = extra_source(&c[0].1, MODULE, &global_decl(MATRIX_PRODUCT_MEMBER));
    c.push(other);
    vec![
        ("(a) two modules, one concatenated label", a),
        ("(b) one routine name in both sources", b),
        (
            "(c) both kernel names in one source, a global in the other",
            c,
        ),
    ]
}

#[test]
fn v009_a_duplicate_without_the_kernel_never_claims_a_matrix_call() {
    // Every case and both natives, the findings REPORTED WHOLE.
    let mut bad = Vec::new();
    for (what, srcs) in duplicates_without_the_kernel() {
        let t1 = native_refusal(".t1", t1_image, &srcs);
        if !t1.contains("code 2") || t1.contains(DEFINITE) {
            bad.push(format!("{what}: .t1: {t1}"));
        }
        let rust = native_refusal("rust", rust_twin_image, &srcs);
        if !rust.contains("more than one object")
            || rust.contains(DEFINITE)
            || rust.contains(RESERVED)
        {
            bad.push(format!("{what}: rust: {rust}"));
        }
    }
    assert!(
        bad.is_empty(),
        "a duplicate with no kernel is misreported:\n{}",
        bad.join("\n")
    );
}

// ── (7) THE REVIEW'S EDGE SHAPES AND REFUSALS (adapted from its probes at ecf53307) ─

fn edge_product_cases() -> Vec<Case> {
    vec![
        Case::product(1, 1, 300, false),
        Case::product(1, 300, 1, false),
        Case::product(300, 1, 2, false),
        Case::product(1, 7, 40, false),
        Case::product(257, 2, 3, true),
        Case::product(1, 5, 300, true),
        Case::product(1, 2, 300, false).a_transposed(),
        Case::product(300, 2, 1, false).a_transposed(),
        Case::product(1, 9, 1, false).a_transposed(),
        Case::tensor(0, 3, 4, 5, false),
        Case::tensor(2, 1, 1, 1, false),
        Case::tensor(2, 3, 4, 1, true),
        Case::tensor(2, 1, 300, 1, false).a_transposed(),
        Case::tensor(2, 260, 1, 2, false).a_transposed(),
    ]
}

fn edge_transpose_cases() -> Vec<Case> {
    vec![
        Case::transpose(1, 300, false),
        Case::transpose(300, 1, false),
        Case::transpose(257, 3, true),
        Case::transpose(3, 257, false),
        Case::transpose(1, 1, true),
    ]
}

/// All engines (interp, .t1 image, rust image on yantra, both on qemu) vs the oracle.
fn check_everywhere(what: &str, cases: &[Case]) {
    let srcs = probe(cases);
    let w = want(cases);
    let interp = interpret_result(&srcs).unwrap_or_else(|e| panic!("{what}: interpreter: {e}"));
    assert!(
        interp == w,
        "{what}: interpreted: {}",
        first_difference(&interp, &w)
    );
    let images = assert_native(what, &srcs, &w);
    for (engine, image) in [".t1", "rust"].iter().zip(images) {
        let got = words(&qemu_leg::run(&image).unwrap_or_else(|e| panic!("{what} {engine}: {e}")));
        assert!(
            got == w,
            "{what}, {engine} on qemu: {}",
            first_difference(&got, &w)
        );
    }
}

#[test]
fn v009_edge_shapes_products_on_every_engine() {
    for c in edge_product_cases() {
        check_everywhere(&format!("{c:?}"), &[c]);
    }
}

#[test]
fn v009_edge_shapes_transposes_on_every_engine() {
    for c in edge_transpose_cases() {
        check_everywhere(&format!("{c:?}"), &[c]);
    }
}

/// A hand-computed Aᵀ·B, independent of the test's oracle: A stored K×M = 2×3
/// [[1,2,3],[4,5,6]], B = 2×1 [[1],[10]], so C = Aᵀ·B = [41, 52, 63].
#[test]
fn v009_transposed_is_a_transpose_times_b_by_hand() {
    let f = |x: f64| x.to_bits();
    let a: Vec<u64> = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0].map(f).to_vec();
    let b: Vec<u64> = [1.0, 10.0].map(f).to_vec();
    let mut body = explicit_fill("क", &a);
    body.push_str(&explicit_fill("ख", &b));
    body.push_str(&explicit_fill("फल", &[0, 0, 0]));
    body.push_str(&format!(
        "    परिमाणम् भवति {} आरभ्य फल ऽ क ऽ ख ऽ ३ ऽ २ ऽ १ ऽ {} समाप्तम् ।\n",
        qualified(MATRIX_PRODUCT_MEMBER),
        qualified(TRANSPOSED)
    ));
    body.push_str(&print_lines("फल", 3));
    let srcs = module(&body);
    let w = vec![3, f(41.0), f(52.0), f(63.0)];
    let got = interpret_result(&srcs).unwrap();
    assert_eq!(show(&got), show(&w), "interpreted");
    let images = assert_native("hand Aᵀ", &srcs, &w);
    for image in images {
        assert_eq!(
            show(&words(&qemu_leg::run(&image).unwrap())),
            show(&w),
            "qemu"
        );
    }
}

fn refusal_on_every_engine(what: &str, srcs: &[(&'static str, String)], code: u64) {
    let shown = format!("{code:#x}");
    let e = interpret_result(srcs).expect_err(&format!("{what}: the interpreter refuses"));
    assert!(e.contains(&shown), "{what}: interpreter: {e}");
    for (side, image) in [("t1", t1_image(srcs)), ("rust", rust_twin_image(srcs))] {
        let (h, out) = run_image(&image);
        assert!(is_refusal(&h, code), "{what}: {side} halted {h:?}");
        assert!(out.is_empty(), "{what}: {side} printed before refusing");
        let q = qemu_leg::run(&image);
        let exit = format!("exit status: {}", code & 0xff);
        assert!(
            matches!(&q, Err(e) if e.contains(&exit)),
            "{what}: {side} qemu {q:?}"
        );
    }
}

#[test]
fn v009_more_refusals_are_the_same_on_every_engine() {
    let cases: Vec<(&str, Vec<(&'static str, String)>, u64)> = vec![
        (
            "tensor B=0 M=2^20 K=2^20 N=1",
            refusal_probe(
                &Case::tensor(2, 1, 1, 1, false),
                [0, 0, 0],
                Some(&[0, 1 << 20, 1 << 20, 1]),
                None,
            ),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "product M=2^32-1 K=1 N=0 lens [0, 2^32-1?]: A short",
            refusal_probe(
                &Case::product(1, 1, 1, false),
                [0, 0, 0],
                Some(&[(1 << 32) - 1, 1, 0]),
                None,
            ),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "tensor in place C = A",
            refusal_probe(&Case::tensor(2, 2, 2, 2, false), [8, 8, 8], None, Some("क")),
            REFUSAL_ADHYASA,
        ),
        (
            "tensor in place C = B",
            refusal_probe(&Case::tensor(2, 2, 2, 2, false), [8, 8, 8], None, Some("ख")),
            REFUSAL_ADHYASA,
        ),
        (
            "column-major in place C = B",
            refusal_probe(&Case::product(3, 3, 3, true), [9, 9, 9], None, Some("ख")),
            REFUSAL_ADHYASA,
        ),
        (
            "column-major in place C = A",
            refusal_probe(&Case::product(3, 3, 3, true), [9, 9, 9], None, Some("क")),
            REFUSAL_ADHYASA,
        ),
        (
            "transposed in place C = A",
            refusal_probe(
                &Case::product(3, 3, 3, false).a_transposed(),
                [9, 9, 9],
                None,
                Some("क"),
            ),
            REFUSAL_ADHYASA,
        ),
        (
            "column-major transpose in place",
            refusal_probe(&Case::transpose(2, 3, true), [6, 6, 0], None, Some("क")),
            REFUSAL_ADHYASA,
        ),
        (
            "aliased AND B short: shape wins",
            refusal_probe(&Case::product(3, 3, 3, false), [9, 9, 8], None, Some("क")),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "column-major A short",
            refusal_probe(&Case::product(3, 4, 5, true), [15, 11, 20], None, None),
            VECTOR_LENGTH_REFUSAL,
        ),
        (
            "transposed C short",
            refusal_probe(
                &Case::product(3, 4, 5, false).a_transposed(),
                [14, 12, 20],
                None,
                None,
            ),
            VECTOR_LENGTH_REFUSAL,
        ),
    ];
    let mut bad = Vec::new();
    for (what, srcs, code) in cases {
        if let Err(p) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            refusal_on_every_engine(what, &srcs, code)
        })) {
            bad.push(format!("{what}: {}", panic_text(p)));
        }
    }
    assert!(bad.is_empty(), "refusal findings:\n{}", bad.join("\n"));
}

/// C = A·A (the two INPUTS the same run) is not an alias of the result: it runs.
#[test]
fn v009_the_same_input_twice_is_allowed() {
    let c = Case::product(3, 3, 3, false);
    let call = call_text(&c, "", None, &layout(false)).replacen(" ऽ ख ", " ऽ क ", 1);
    let mut body = pool_lines(c.pool);
    body.push_str(&case_lines(&c, "", Some([9, 9, 9]), &call));
    let srcs = module(&body);
    let a: Vec<u64> = (0..9).map(|e| element(&c, 1, e)).collect();
    let mut w = vec![9u64];
    w.extend(explicit_product(3, 3, 3, &a, &a, POS_ZERO, false, false));
    let got = interpret_result(&srcs).unwrap();
    assert_eq!(show(&got), show(&w), "interpreted");
    let images = assert_native("A·A", &srcs, &w);
    for image in images {
        assert_eq!(show(&words(&qemu_leg::run(&image).unwrap())), show(&w));
    }
}
