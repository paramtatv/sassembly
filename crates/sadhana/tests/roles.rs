//! A field is chosen by rôle, never by arrival — D-02-C.
//!
//! Free operand order is the whole of decision D-02-C: the kāraka says what an
//! operand is, so position carries no meaning and cannot be misread. Every
//! instruction shape must therefore encode the same however its operands are
//! ordered.
//!
//! `परमाणुयोगः` is the shape that found this wrong. An atomic add has three
//! rôles — a destination, a value and an address — and the encoder gave `rs1`
//! to whichever of the value and the address it reached first. Both orders
//! collided, so it failed loudly rather than silently, which is the only reason
//! it was not a wrong program instead of a rejected one.

use sadhana::encode::encode_program;
use sadhana::parse::assemble_program;

fn word(src: &str) -> u32 {
    let p = assemble_program(src).expect("parses");
    sadhana::encode::words(&encode_program(&p).expect("encodes"))[0]
}

#[test]
fn an_atomic_encodes_the_same_in_either_order() {
    let value_first = word("परमाणुयोगःॱअ३२ क्षणिक०म् क्षणिक१न क्षणिक२त् ।");
    let address_first = word("परमाणुयोगःॱअ३२ क्षणिक०म् क्षणिक२त् क्षणिक१न ।");
    let destination_last = word("परमाणुयोगःॱअ३२ क्षणिक२त् क्षणिक१न क्षणिक०म् ।");
    assert_eq!(value_first, address_first);
    assert_eq!(value_first, destination_last);
    // From `riscv64-elf-as`: amoadd.w t0, t1, (t2).
    assert_eq!(value_first, 0x0063_a2af);
}

#[test]
fn a_store_encodes_the_same_in_either_order() {
    let a = word("निधानम्ॱअ६४ स्तूपसूचकःय् ८न क्षणिक०न ।");
    let b = word("निधानम्ॱअ६४ क्षणिक०न ८न स्तूपसूचकःय् ।");
    assert_eq!(a, b);
    assert_eq!(a, 0x0051_3423, "sd t0, 8(sp)");
}

#[test]
fn a_three_register_operation_keeps_its_sources_apart() {
    // With no address to claim rs1, the two करण operands take rs1 and rs2 in
    // written order — which is not a contradiction: they have the SAME rôle,
    // so nothing else could distinguish them, and `वियोगः` is not symmetric.
    let a = word("वियोगः क्षणिक०म् क्षणिक१न क्षणिक२न ।");
    assert_eq!(a, 0x4073_02b3, "sub t0, t1, t2");
    let swapped = word("वियोगः क्षणिक०म् क्षणिक२न क्षणिक१न ।");
    assert_ne!(a, swapped, "two operands of one rôle are ordered");
}

#[test]
fn a_branch_still_reads_its_standard_as_the_second_source() {
    // ADR-0008: on a branch, अपादान is the standard of comparison and takes
    // rs2. The rule that an address owns rs1 must not reach it.
    let w = word("पुनरावृत्तिःॱॱ\nन्यूनलङ्घनम् क्षणिक०न क्षणिक१त् पुनरावृत्तिःय् ।");
    assert_eq!(w, 0x0062_c063, "blt t0, t1, 0");
}
