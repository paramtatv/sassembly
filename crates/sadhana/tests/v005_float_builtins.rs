//! `V-005` — the float built-ins' NAMES ARE THE ENCODING TABLE'S WORDS, and the
//! interpreter's arithmetic is IEEE with RISC-V's canonical NaN.
//!
//! The names were copied from `spec/encodings-riscv64.tsv`'s third column; this
//! reads the file and holds every one to the row of the mnemonic it stands for,
//! so a name typed from memory (or a table row that moves) is red here first.

use sadhana::t1::nirvahana::{FLOAT_BUILTIN_MODULE, FLOAT_BUILTINS, Interpreter, Value};
use std::path::Path;

#[test]
fn the_float_builtin_names_are_the_encoding_tables_words() {
    let tsv = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/encodings-riscv64.tsv"),
    )
    .expect("the encoding table reads");
    let word_of = |mnemonic: &str| -> String {
        tsv.lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| l.split('\t').collect::<Vec<_>>())
            .find(|f| f.first() == Some(&mnemonic))
            .and_then(|f| f.get(2).map(|w| (*w).to_string()))
            .unwrap_or_else(|| panic!("no row for {mnemonic}"))
    };
    for (name, _, _, mnemonics) in FLOAT_BUILTINS {
        for m in mnemonics.split(' ') {
            assert_eq!(
                word_of(m),
                name,
                "the built-in for {m} must be spelled as the table spells it"
            );
        }
    }
    // The six arithmetic words the owner's ruling names, by mnemonic.
    for m in ["fadd.d", "fsub.d", "fmul.d", "fmadd.d", "fdiv.d", "fsqrt.d"] {
        let w = word_of(m);
        assert!(
            FLOAT_BUILTINS.iter().any(|(n, ..)| *n == w),
            "{m}'s word `{w}` must be a built-in"
        );
    }
}

/// Run one routine body interpreted and answer what it returns.
fn run(body: &str) -> Value {
    let src = format!(
        "मण्डलम् प्लवनिर्वाहपरीक्षण ॥
आयातः {FLOAT_BUILTIN_MODULE} ।
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
{body}
इति
"
    );
    let mut it = Interpreter::load(&[("probe.t1", src.as_str())], Path::new("."))
        .unwrap_or_else(|e| panic!("loads: {}", e.reason));
    it.call("मुख्यम्", Vec::new(), 1_000_000)
        .unwrap_or_else(|e| panic!("runs: {}", e.reason))
}

/// `+inf + -inf` is RISC-V's canonical NaN, 0x7ff8000000000000 — and NOT the
/// NaN x86 produces for the same sum (0xfff8000000000000, the sign set), which
/// is what an uncanonicalised `f64` add answers on the host running this test.
#[test]
fn infinity_minus_infinity_is_the_canonical_nan() {
    let v = run("    चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः ०षोड्७ऊऊ००००००००००००० ।
    चरः ख ॱॱ प६४ भवति अष्टकॱप्लवसंचारः ऋण०षोड्१००००००००००००० ।
    चरः ग ॱॱ प६४ भवति अष्टकॱप्लवयोगः आरभ्य क ऽ ख समाप्तम् ।
    प्रत्यागमनम् अष्टकॱप्लवसंचारः ग ।");
    assert_eq!(v.as_int(), Some(0x7ff8_0000_0000_0000));
}

/// An integer operator never adds two floats' bit patterns.
#[test]
fn an_integer_operator_refuses_a_float() {
    let src = format!(
        "मण्डलम् प्लवनिर्वाहपरीक्षण ॥
आयातः {FLOAT_BUILTIN_MODULE} ।
सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ प६४ भवति अष्टकॱप्लवसंचारः १ ।
    चरः ग ॱॱ प६४ भवति क योगः क ।
    प्रत्यागमनम् ० ।
इति
"
    );
    let mut it = Interpreter::load(&[("probe.t1", src.as_str())], Path::new("."))
        .unwrap_or_else(|e| panic!("loads: {}", e.reason));
    let e = it
        .call("मुख्यम्", Vec::new(), 1_000_000)
        .expect_err("an integer add of two floats must refuse");
    assert!(e.reason.contains("V-005"), "{}", e.reason);
}
