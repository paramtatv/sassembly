//! Labels and the symbol table — task `B-007`.
//!
//! Until now every Sassembly program was straight-line: the assembler could
//! encode an instruction but not a *place*, so nothing could jump anywhere and
//! no program could loop or call.
//!
//! Two passes, because a jump may name a label defined later. Pass one assigns
//! an address to every instruction; pass two encodes each one and resolves a
//! label to the distance from the instruction that names it.
//!
//! The expected words come from `riscv64-elf-as` assembling the equivalent
//! RISC-V, so a backward jump of −4 and a forward jump of +8 are checked
//! against an oracle rather than against arithmetic done here.

use sadhana::encode::encode_program;
use sadhana::parse::assemble_program;

/// The program used throughout, with its oracle bytes.
///
/// ```text
///         addi t0, zero, 10
/// back:   addi t0, t0, 1
///         jal  zero, back       backward, -4
///         jal  ra, ahead        forward, +8
///         addi t1, zero, 0
/// ahead:  addi t2, zero, 7
/// ```
const SOURCE: &str = "\
योगः क्षणिक०म् शून्यःन १०न ।
पुनरावृत्तिःॱॱ
योगः क्षणिक०म् क्षणिक०न १न ।
लङ्घनम् शून्यःम् पुनरावृत्तिःय् ।
लङ्घनम् पुनःस्थानम्म् अग्रेय् ।
योगः क्षणिक१म् शून्यःन ०न ।
अग्रेॱॱ
योगः क्षणिक२म् शून्यःन ७न ।
";

/// From `riscv64-elf-as`. Nothing here was computed by hand.
const EXPECTED: [u32; 6] = [
    0x00a0_0293, // addi t0, zero, 10
    0x0012_8293, // addi t0, t0, 1
    0xffdf_f06f, // j    back      -- displacement -4
    0x0080_00ef, // jal  ra, ahead -- displacement +8
    0x0000_0313, // addi t1, zero, 0
    0x0070_0393, // addi t2, zero, 7
];

#[test]
fn a_label_is_lexed_off_its_name() {
    // `पुनरावृत्तिःॱॱ` is written against the name exactly as `back:` is. It
    // used to lex as one Word, so no label could be defined at all.
    let p = assemble_program(SOURCE).expect("parses");
    let names: Vec<&str> = p.labels.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["पुनरावृत्तिः", "अग्रे"]);
}

#[test]
fn a_label_marks_the_instruction_that_follows_it() {
    let p = assemble_program(SOURCE).expect("parses");
    assert_eq!(p.instructions.len(), 6);
    // `पुनरावृत्तिः` sits before the second instruction, `अग्रे` before the sixth.
    assert_eq!(p.labels[0].at, 1);
    assert_eq!(p.labels[1].at, 5);
}

#[test]
fn the_program_assembles_to_what_gnu_as_emits() {
    let p = assemble_program(SOURCE).expect("parses");
    let words = sadhana::encode::words(&encode_program(&p).unwrap_or_else(|e| {
        panic!(
            "did not encode:\n  {}",
            e.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n  ")
        )
    }));
    assert_eq!(words.len(), EXPECTED.len());
    for (i, (got, want)) in words.iter().zip(EXPECTED.iter()).enumerate() {
        assert_eq!(
            got, want,
            "instruction {i}: ours {got:#010x}, as {want:#010x}"
        );
    }
    println!("METRIC symbol_table_program_words {}", words.len());
}

#[test]
fn a_backward_jump_is_negative_and_a_forward_one_is_not() {
    // The two directions are the reason this needs two passes and signed
    // displacements, and `B-055` only reached the sign bit with a negative
    // probe. Stated separately so a regression names the direction.
    let p = assemble_program(SOURCE).expect("parses");
    let words = sadhana::encode::words(&encode_program(&p).expect("encodes"));
    assert_eq!(words[2], 0xffdf_f06f, "backward jump to पुनरावृत्तिः");
    assert_eq!(words[3], 0x0080_00ef, "forward jump to अग्रे");
}

#[test]
fn an_undefined_label_is_named_rather_than_encoded_as_zero() {
    let src = "लङ्घनम् शून्यःम् अनुपस्थितःय् ।";
    let p = assemble_program(src).expect("parses");
    let errs = encode_program(&p).expect_err("must not encode");
    assert!(
        errs[0].reason.contains("अनुपस्थितः"),
        "the message should name the label, got: {}",
        errs[0].reason
    );
}

#[test]
fn a_target_must_be_marked_sampradana() {
    // D-02-C says the role is in the token. A target marked करण would make
    // "jump to" and "jump from" spellable the same way, so it is refused.
    let src = "पुनरावृत्तिःॱॱ\nलङ्घनम् शून्यःम् पुनरावृत्तिःन ।";
    let p = assemble_program(src).expect("parses");
    let errs = encode_program(&p).expect_err("must not encode");
    assert!(
        errs[0].reason.contains("सम्प्रदान"),
        "got: {}",
        errs[0].reason
    );
}

#[test]
fn a_label_needs_exactly_one_name() {
    // The codes, not the wording: `B-078e` moved both of these into
    // `spec/diagnostics.tsv`, and matching on an English phrase asserted the
    // one language the parser had stopped speaking by default.
    let code = |src: &str| {
        let tokens = sadhana::lex::lex(src).expect("lexes");
        sadhana::parse::parse(&tokens).expect_err("must not parse")[0].code
    };
    assert_eq!(code("क ख ॱॱ"), "P07", "two words before the mark");
    assert_eq!(code("ॱॱ"), "P06", "no word before the mark");

    // And the name at fault travels with it, in either language.
    let errs = assemble_program("क ख ॱॱ").expect_err("must not parse");
    assert!(errs[0].contains("क ख"), "got: {}", errs[0]);
}
