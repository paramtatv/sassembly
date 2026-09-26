//! A fence domain is a domain only where a fence domain fits — task `B-107`.
//!
//! `पठनम्` ("reading") is one of the four ordering domains a `स्मृतिबन्धः`
//! names, and it is also an entirely ordinary Sanskrit noun — the kind of word
//! a program calls a routine. Both readings are wanted, and until this task
//! the encoder took the domain one everywhere:
//!
//! ```text
//! पठनम्ॱॱ                       defines fine
//! लङ्घनम् शून्यःम् पठनम्य् ।       `रूपं 1 संज्ञकैः 0 संख्याभिः च न सङ्गच्छते`
//! ```
//!
//! The label was counted as a domain set rather than as a label, so `jal` —
//! which takes one register and one displacement — matched nothing, and the
//! diagnostic blamed the SHAPE for a collision it never mentioned. Nothing in
//! the message said `पठनम्` meant something else here.
//!
//! The fix is not a reserved-word list. The encoding table already records
//! which operands are domain sets — `fence` is the one row whose
//! `operand_shape` reads `iorw, iorw` — so the reading is chosen by what the
//! family can take, and every other family reads the same word as a name.

use sadhana::encode::{encode_program, words};
use sadhana::parse::assemble_program;

fn assemble(src: &str) -> Vec<u32> {
    let p = assemble_program(src).expect("parses");
    words(&encode_program(&p).expect("encodes"))
}

/// The four domain names, from `spec/fence-domains-riscv64.tsv`.
///
/// Listed here rather than read from the file on purpose: this test asserts
/// that these particular words are usable as labels, and a test that derives
/// its own inputs from the table would keep passing if the table emptied.
const DOMAINS: [&str; 4] = ["आगमः", "निर्गमः", "पठनम्", "लेखनम्"];

#[test]
fn a_domain_name_may_label_a_place() {
    for name in DOMAINS {
        let src = format!("{name}ॱॱ\nलङ्घनम् शून्यःम् {name}य् ।\n");
        let w = assemble(&src);
        // `jal x0, .` — the label is at the jump's own address, so the
        // displacement is zero and the whole word is the pattern.
        assert_eq!(w.len(), 1, "{name}");
        assert_eq!(w[0], 0x0000_006f, "{name}: jal zero, 0");
    }
}

#[test]
fn a_domain_name_still_names_a_domain_in_a_fence() {
    // fence r, w — the reading a fence takes is unchanged.
    let w = assemble("स्मृतिबन्धः पठनम्त् लेखनम्य् ।");
    assert_eq!(w, vec![0x0210_000f]);
}

#[test]
fn a_domain_set_is_still_a_set_in_a_fence() {
    // fence rw, rw
    let w = assemble("स्मृतिबन्धः पठनम्ऽलेखनम्त् पठनम्ऽलेखनम्य् ।");
    assert_eq!(w, vec![0x0330_000f]);
}

#[test]
fn a_domain_name_may_be_called() {
    // A routine named "reading" that returns, called by name.
    let w = assemble("लङ्घनम् पुनःस्थानम्म् पठनम्य् ।\nपठनम्ॱॱ\nलङ्घनम् शून्यःम् पठनम्य् ।\n");
    assert_eq!(w.len(), 2);
    // jal ra, 4
    assert_eq!(w[0], 0x0040_00ef);
    // jal zero, 0 — at the label, jumping to itself.
    assert_eq!(w[1], 0x0000_006f);
}
