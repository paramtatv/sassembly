//! Address materialisation — task `B-064`.
//!
//! A 32-bit address does not fit in any instruction, so it is built from two:
//! `auipc` takes the high 20 bits of the distance and `addi` the low 12.
//!
//! # Why pc-relative and not absolute
//!
//! Not a preference. The image loads at `0x8000_0000`, `lui` sign-extends its
//! immediate, and `0x8000_0000` sign-extends to `0xffff_ffff_8000_0000` — so an
//! absolute `lui`+`addi` cannot reach our own load address. GNU `ld` says
//! "relocation truncated to fit" and refuses to link it.
//!
//! # The expected values are the linker's
//!
//! `as` alone emits relocations for `%pcrel_hi`, so it cannot be the oracle
//! here. `ld --no-relax` resolves them, and the words below are what it
//! produced for the same shape — with `-Tdata` at the page after the text
//! since W-363, which is where `kosha::data_base` puts `.data`.

use sadhana::encode::encode_program;
use sadhana::parse::{Section, assemble_program};

/// Two instructions building the address of a datum eight bytes past the first.
const SOURCE: &str = "\
स्थानसापेक्षयोगः अर्थ०म् सङ्ख्याॱउपरिन ।
योगः अर्थ०म् अर्थ०न सङ्ख्याॱअधःन ।
॥ कोष्ठकम् ॱदत्त ॥
सङ्ख्याॱॱ
॥ चतुरष्टकाः १ ॥
";

#[test]
fn the_pair_matches_what_the_linker_resolves() {
    let p = assemble_program(SOURCE).expect("parses");
    let words = sadhana::encode::words(&encode_program(&p).expect("encodes"));
    // From `ld --no-relax -Ttext=0x80000000 -Tdata=0x80001000` on (W-363:
    // `.data` at the page after the text; it was `addi a0, a0, 8` behind an
    // eight-aligned data base):
    //     1: auipc a0, %pcrel_hi(d)
    //        addi  a0, a0, %pcrel_lo(1b)
    //     d: .word 1     (in .data)
    assert_eq!(words[0], 0x0000_1517, "auipc a0, 0x1");
    assert_eq!(words[1], 0x0005_0513, "addi a0, a0, 0");
}

#[test]
fn a_label_in_the_data_section_knows_it() {
    let p = assemble_program(SOURCE).expect("parses");
    let l = &p.labels[0];
    assert_eq!(l.section, Section::Data);
    assert_eq!(l.data_offset, 0);
}

#[test]
fn a_name_ending_in_a_sigil_can_still_be_a_label() {
    // `दत्तम्` ends in `म्` and lexes as an operand — "दत्त, destination". ADR-0004
    // settles the same collision for verbs by position, and a label is settled
    // the same way: one token before `ॱॱ` is a name whatever it ends in.
    // Requiring otherwise would bar most Sanskrit neuter nouns from being
    // labels, which is the ban ADR-0005 refused for operands.
    let p = assemble_program("॥ कोष्ठकम् ॱदत्त ॥\nदत्तम्ॱॱ\n॥ अष्टकाः ७ ॥\n").expect("parses");
    assert_eq!(p.labels[0].name, "दत्तम्");
}

#[test]
fn the_low_half_measures_from_the_instruction_before_it() {
    // `ॱअधः` completes the register an `auipc` built, so it measures from the
    // auipc's address rather than its own. GNU spells that `%pcrel_lo(1b)`,
    // naming the auipc by back-reference; here the two must be adjacent and
    // the convention is positional.
    //
    // Three instructions, so the datum is further away and the low half is
    // larger — if it measured from its own address the value would be four
    // less than the linker's.
    let src = "\
योगः शून्यःम् शून्यःन ०न ।
स्थानसापेक्षयोगः अर्थ०म् सङ्ख्याॱउपरिन ।
योगः अर्थ०म् अर्थ०न सङ्ख्याॱअधःन ।
॥ कोष्ठकम् ॱदत्त ॥
सङ्ख्याॱॱ
॥ चतुरष्टकाः १ ॥
";
    let p = assemble_program(src).expect("parses");
    let words = sadhana::encode::words(&encode_program(&p).expect("encodes"));
    // auipc is at +4, data at +4096 (the page after 12 octets of text):
    // distance 4092, so `ld` (same flags as above) writes auipc a0, 0x1 and
    // addi a0, a0, -4. Measured from its own pc, the low half would be -8.
    assert_eq!(words[1], 0x0000_1517, "auipc a0, 0x1");
    assert_eq!(
        (words[2] as i32) >> 20,
        -4,
        "addi immediate completes the distance from the auipc"
    );
}

#[test]
fn an_undefined_name_is_named_rather_than_encoded_as_zero() {
    let src = "स्थानसापेक्षयोगः अर्थ०म् अनुपस्थितःॱउपरिन ।";
    let p = assemble_program(src).expect("parses");
    let e = encode_program(&p).expect_err("must not encode");
    assert!(e[0].reason.contains("अनुपस्थितः"), "got: {}", e[0].reason);
}
