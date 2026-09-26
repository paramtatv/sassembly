//! **विश्लेषणम्** — the disassembler, task `B-013`.
//!
//! विश्लेषणम् is "taking apart, analysis", the counterpart to साधनम् which puts
//! together. It reads a machine word and says which instruction it is and what
//! its operands were.
//!
//! # This is a test instrument before it is a tool
//!
//! Nothing in SANSOS needs to read machine code yet. What it needs is a second,
//! independent consumer of `spec/encodings-riscv64.tsv` — because the encoder
//! only ever uses a table in ONE direction, and a direction is exactly where
//! this project keeps finding defects.
//!
//! Four mask bugs shipped and were caught late: `slli` with the probe's own
//! registers fused into its opcode, 174 masks claiming operand bits as fixed
//! encoding, `beq` and `jal` swallowing their own displacements, and every
//! signed field missing its sign bit. Each was invisible while encoding,
//! because the encoder starts from a pattern whose operand bits are zero and
//! ORs values in — it never asks whether the mask is *right*, only whether the
//! bits it sets are free.
//!
//! Decoding asks the other question. `(word & mask) == pattern` is a claim
//! about every bit at once, and the round-trip in `tests/roundtrip.rs` is the
//! fixpoint: take a word GNU `as` produced, pull it apart, put it back, and
//! require the same word.
//!
//! It decodes by the mask column of `spec/encodings-riscv64.tsv` and not by the
//! complement of the field masks, deliberately. The complement cannot disagree
//! with the fields and so would check nothing; the stored mask is the value
//! that was wrong in all four defects above, and this is the first code that
//! reads it.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::encode::{Encoding, Slot, encodings};

/// One instruction, taken apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// The RISC-V mnemonic that matched.
    pub insn: String,
    /// The Sassembly family it belongs to.
    pub family: String,
    /// One value per operand slot, in the encoding's written order.
    ///
    /// Signed fields are sign-extended; unsigned ones are not.
    pub operands: Vec<(String, i64)>,
}

/// Pull `value` back out of a word using the slot's derived bit map.
///
/// The exact inverse of [`Slot::place`]: that walks value-bit to encoding-bit
/// setting bits, this walks the same pairs reading them.
#[must_use]
fn extract(slot: &Slot, word: u32) -> i64 {
    let mut value: u64 = 0;
    for (from, to) in &slot.map {
        if word >> to & 1 == 1 {
            value |= 1 << from;
        }
    }
    // A displacement and a signed immediate are two's complement, and the top
    // MAPPED bit is the sign — not bit 63. Reading `addi`'s -1 as 4095 would
    // round-trip perfectly and still be the wrong number.
    if slot.kind == "disp" || slot.kind == "simm" {
        let width = slot.map.iter().map(|(f, _)| *f).max().map_or(0, |m| m + 1);
        if width > 0 && width < 64 && value >> (width - 1) & 1 == 1 {
            return (value as i64) - (1i64 << width);
        }
    }
    // A biased field holds its values from `bias` upward, and [`Slot::place`]
    // drops the value bits the map does not reach: `x8`..`x15` go into three
    // bits as 0..7 because 8 has no mapping. Reading those three bits back
    // gives 0..7, and 0..7 are the wrong registers — the bytes round-trip and
    // `s0` is reported as `x0` (`W-233`, all 27 of W-211's compressed
    // exceptions). The value that was placed is the one in
    // `bias..bias + span` congruent to what was read, so the span is added
    // back until the reading is in range. `c.add`'s five-bit `rs2` has bias 1
    // and reads 1..31 as themselves; only a reading below the bias moves.
    if slot.bias != 0 {
        let width = slot.map.iter().map(|(f, _)| *f).max().map_or(0, |m| m + 1);
        let bias = u64::from(slot.bias);
        if width > 0 && width < 64 && value < bias {
            let span = 1u64 << width;
            value += span * (bias - value).div_ceil(span);
        }
    }
    value as i64
}

/// How many BYTES the instruction starting with this halfword occupies.
///
/// RISC-V puts the length in the low bits of the first halfword: anything whose
/// bottom two bits are not `11` is a 16-bit compressed form. That is why a
/// decoder can walk a mixed-width stream at all without being told where the
/// boundaries are — and why walking it two bytes at a time and hoping is not
/// necessary (`B-058b1`).
#[must_use]
pub fn width_of(first_halfword: u16) -> usize {
    if first_halfword & 0b11 == 0b11 { 4 } else { 2 }
}

/// Take one 16-bit compressed instruction apart.
///
/// Separate from [`decode`] rather than folded into it because the two match
/// against different halves of the table and a 16-bit pattern compared against
/// a 32-bit word would match on the high bits being zero — which is every
/// `c.nop`-shaped encoding claiming every word that happens to start with one.
#[must_use]
pub fn decode16(half: u16) -> Option<Decoded> {
    let word = u32::from(half);
    let mut found: Vec<&Encoding> = encodings()
        .iter()
        .filter(|e| e.bits == 16 && word & e.mask == e.pattern)
        .collect();
    // Same rule as the wide half: the most constrained match is the more
    // specific claim. `c.nop` is `c.addi` with rd and imm zero, and both fit
    // 0x0001 — the one fixing more bits is the one meant.
    found.sort_by_key(|e| core::cmp::Reverse(e.mask.count_ones()));
    let e = found.first()?;
    let operands = e
        .slots
        .iter()
        .map(|s| (s.kind.clone(), extract(s, word)))
        .collect();
    Some(Decoded {
        insn: e.insn.clone(),
        family: e.family.clone(),
        operands,
    })
}

/// Decode the instruction at `offset` in a byte stream, and say how wide it was.
///
/// The census and the duplicate detector both walk instructions and both assume
/// four bytes each (`B-058b2`). This is what they will use when the encoder can
/// emit compressed forms; until then it is how the corpus is checked.
#[must_use]
pub fn decode_at(bytes: &[u8], offset: usize) -> Option<(Decoded, usize)> {
    let half = u16::from_le_bytes([*bytes.get(offset)?, *bytes.get(offset + 1)?]);
    match width_of(half) {
        2 => decode16(half).map(|d| (d, 2)),
        _ => {
            let hi = u16::from_le_bytes([*bytes.get(offset + 2)?, *bytes.get(offset + 3)?]);
            let word = u32::from(half) | u32::from(hi) << 16;
            decode(word).map(|d| (d, 4))
        }
    }
}

/// Every 32-bit encoding whose fixed bits match `word`.
///
/// More than one is a fault in the table rather than an ambiguity in the ISA:
/// two instructions that constrain the same bits to the same values are the
/// same instruction. `tests/roundtrip.rs` asserts this never returns two.
#[must_use]
pub fn candidates(word: u32) -> Vec<Encoding> {
    encodings()
        .iter()
        .filter(|e| e.bits == 32 && word & e.mask == e.pattern)
        .cloned()
        .collect()
}

/// Take one word apart.
///
/// Returns `None` when nothing matches — an undefined encoding, or a 16-bit
/// compressed form, which [`decode16`] reads instead.
#[must_use]
pub fn decode(word: u32) -> Option<Decoded> {
    let mut found = candidates(word);
    // Prefer the most constrained match. `ecall` fixes every bit and shares its
    // opcode with the CSR instructions; the one that constrains more bits is
    // the more specific claim about this word.
    found.sort_by_key(|e| core::cmp::Reverse(e.mask.count_ones()));
    let e = found.into_iter().next()?;
    let operands = e
        .slots
        .iter()
        .map(|s| (s.kind.clone(), extract(s, word)))
        .collect();
    Some(Decoded {
        insn: e.insn,
        family: e.family,
        operands,
    })
}

/// Put a decoded instruction back together.
///
/// The other half of the fixpoint. If this does not reproduce the word it was
/// decoded from, the table is inconsistent with itself in a way the encoder
/// alone can never notice.
#[must_use]
pub fn reassemble(d: &Decoded) -> Option<u32> {
    let all = encodings();
    let e = all.iter().find(|e| e.insn == d.insn && e.bits == 32)?;
    if e.slots.len() != d.operands.len() {
        return None;
    }
    let mut word = e.pattern;
    for (slot, (_, value)) in e.slots.iter().zip(&d.operands) {
        word |= slot.place(*value as u64);
    }
    Some(word)
}

/// Put a decoded 16-bit instruction back together.
///
/// The half [`reassemble`] did not have: `W-211`'s census carried this as a
/// test-local routine and said so, because the disassembler could take a
/// compressed form apart and not put one back (`W-233`).
///
/// Unlike the wide half this asks [`Slot::fits`] before placing, and it must:
/// a compressed register field drops the value bits its map does not reach, so
/// `x0` and `x8` both place as `000` in a three-bit field. Refusing a value the
/// field does not accept is what makes a decoder that read `s0` as `x0` fail
/// here instead of round-tripping the bytes and hiding the wrong register.
#[must_use]
pub fn reassemble16(d: &Decoded) -> Option<u16> {
    let all = encodings();
    let e = all.iter().find(|e| e.insn == d.insn && e.bits == 16)?;
    if e.slots.len() != d.operands.len() {
        return None;
    }
    let mut word = e.pattern;
    for (slot, (_, value)) in e.slots.iter().zip(&d.operands) {
        if !slot.fits(*value as u64) {
            return None;
        }
        word |= slot.place(*value as u64);
    }
    u16::try_from(word).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `c.ld s0, 0(s1)` = 0x6080, from `spec/compressed-t0.tsv`: the
    /// three-bit register fields hold `x8`..`x15` biased by 8, so both fields
    /// are 0 and 1 in the bits and 8 and 9 as registers.
    const C_LD_S0_S1: u16 = 0x6080;

    #[test]
    fn a_compressed_load_naming_x8_decodes_to_x8() {
        let d = decode16(C_LD_S0_S1).expect("decodes");
        assert_eq!(d.insn, "c.ld");
        let regs: Vec<i64> = d
            .operands
            .iter()
            .filter(|(k, _)| k == "reg")
            .map(|(_, v)| *v)
            .collect();
        assert_eq!(regs, vec![8, 9], "rd' is s0 = x8, rs1' is s1 = x9");
        assert_eq!(reassemble16(&d), Some(C_LD_S0_S1), "and it goes back");
    }

    #[test]
    fn a_compressed_register_read_as_zero_is_refused_on_the_way_back() {
        // REFUSED: what the decoder said before `W-233` — the field bits with
        // no bias, so `s0` read as `x0`. The bytes would round-trip perfectly
        // (0 places into three bits as 000, exactly as 8 does) and the
        // register would still be the wrong one. `x0` is not a value a biased
        // field accepts, so putting it back must fail rather than hide that.
        let wrong = Decoded {
            insn: "c.ld".into(),
            family: "load".into(),
            operands: vec![("reg".into(), 0), ("imm".into(), 0), ("reg".into(), 1)],
        };
        assert_eq!(
            reassemble16(&wrong),
            None,
            "a register below the field's lowest accepted value must not reassemble"
        );
    }

    #[test]
    fn a_known_word_comes_apart_correctly() {
        // add x1, x2, x3 = 0x003100b3 — the value tools/check-toolchain.sh
        // verifies the whole oracle against.
        let d = decode(0x0031_00b3).expect("decodes");
        assert_eq!(d.insn, "add");
        let regs: Vec<i64> = d.operands.iter().map(|(_, v)| *v).collect();
        assert_eq!(regs, vec![1, 2, 3], "rd, rs1, rs2");
    }

    #[test]
    fn a_negative_immediate_comes_back_negative() {
        // addi a0, a1, -1 = 0xfff58513. Read unsigned this is 4095, which
        // reassembles to the same word and is still the wrong number.
        let d = decode(0xfff5_8513).expect("decodes");
        assert_eq!(d.insn, "addi");
        assert_eq!(d.operands[2].1, -1, "the immediate is signed");
    }

    #[test]
    fn a_backward_branch_comes_back_negative() {
        // From spec/bare-metal.sas, the loop branch: blt t1, t2, -8.
        let d = decode(0xfe73_4ce3).expect("decodes");
        assert_eq!(d.insn, "blt");
        assert_eq!(d.operands[2].1, -8, "the displacement is signed");
    }

    #[test]
    fn an_undefined_word_is_not_invented() {
        assert!(
            decode(0xffff_ffff).is_none(),
            "all ones is not an instruction"
        );
    }
}
