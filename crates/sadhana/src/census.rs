//! Dead-code census — task `B-023`, doc 18 §6a.3.
//!
//! Which instructions can be reached from the entry point, and which cannot.
//! The budget is **zero**: `purpose.md` says software accretes because each
//! layer was reasonable when added and nothing is ever deleted, so a byte no
//! control path reaches is mass that shipped for no reason.
//!
//! This is the first thing to use `विश्लेषणम्` for its own sake rather than as
//! a test instrument. Reachability needs to know what each word *is* — which
//! ones branch, where they branch to, which ones end a path — and that is the
//! decoder's question.
//!
//! # What it cannot see, and says so
//!
//! An indirect jump goes wherever a register points, and no static reading of
//! the instruction can say where. Rather than assume it goes nowhere (which
//! would report live code as dead) or that it goes everywhere (which would
//! report nothing at all), every `jalr` is recorded in [`Census::indirect`] and
//! the caller is told the analysis was blind at that point.
//!
//! A census that quietly guessed would be worse than none: the whole value of a
//! zero budget is that a report of zero means zero.

extern crate alloc;

use alloc::vec::Vec;

use crate::vishlesana::decode;

/// The result of walking a program's control flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Census {
    /// One flag per instruction: was it reached.
    pub reached: Vec<bool>,
    /// Instructions whose target could not be determined statically.
    pub indirect: Vec<usize>,
    /// Words that did not decode at all.
    pub undecodable: Vec<usize>,
}

impl Census {
    /// Instruction indices no path reaches.
    #[must_use]
    pub fn dead(&self) -> Vec<usize> {
        self.reached
            .iter()
            .enumerate()
            .filter(|(_, r)| !**r)
            .map(|(i, _)| i)
            .collect()
    }

    /// Bytes no path reaches.
    #[must_use]
    pub fn dead_bytes(&self) -> usize {
        self.dead().len() * 4
    }

    /// Whether the analysis saw the whole program.
    ///
    /// False when an indirect jump or an undecodable word means "unreached"
    /// cannot be trusted to mean "unreachable".
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.indirect.is_empty() && self.undecodable.is_empty()
    }
}

/// Walk control flow from instruction zero.
///
/// The entry point is the first instruction because that is where the machine
/// starts: `kosha` maps the text at the reset vector (`B-010`).
#[must_use]
pub fn census(words: &[u32]) -> Census {
    let mut reached = alloc::vec![false; words.len()];
    let mut indirect = Vec::new();
    let mut undecodable = Vec::new();
    let mut queue = Vec::new();
    if !words.is_empty() {
        queue.push(0usize);
    }

    while let Some(i) = queue.pop() {
        if i >= words.len() || reached[i] {
            continue;
        }
        reached[i] = true;

        let Some(d) = decode(words[i]) else {
            undecodable.push(i);
            continue; // an unknown word ends the path rather than inventing one
        };

        // The displacement, in instructions rather than bytes.
        let step = |kind: &str| -> Option<isize> {
            d.operands
                .iter()
                .find(|(k, _)| k == kind)
                .map(|(_, v)| (*v / 4) as isize)
        };

        let name = d.insn.as_str();
        match name {
            // A conditional branch goes both ways, and both are real paths.
            "beq" | "bne" | "blt" | "bge" | "bltu" | "bgeu" => {
                if let Some(s) = step("disp") {
                    queue.push(i.wrapping_add_signed(s));
                }
                queue.push(i + 1);
            }
            // `jal` with rd = zero is a plain jump and nothing follows it. With
            // any other rd it is a call, and the instruction after it is where
            // the callee returns to — so that path is live even though no
            // branch points at it.
            "jal" => {
                if let Some(s) = step("disp") {
                    queue.push(i.wrapping_add_signed(s));
                }
                let rd = d.operands.first().map_or(0, |(_, v)| *v);
                if rd != 0 {
                    queue.push(i + 1);
                }
            }
            // Indirect: the target is in a register. Recorded rather than
            // guessed; `is_complete` reports the blindness.
            "jalr" => {
                indirect.push(i);
                let rd = d.operands.first().map_or(0, |(_, v)| *v);
                if rd != 0 {
                    queue.push(i + 1);
                }
            }
            _ => queue.push(i + 1),
        }
    }

    indirect.sort_unstable();
    undecodable.sort_unstable();
    Census {
        reached,
        indirect,
        undecodable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `addi zero, zero, 0` — a word that does nothing and falls through.
    const NOP: u32 = 0x0000_0013;
    /// `jal zero, 0` — jump to self, ending the path.
    const SPIN: u32 = 0x0000_006f;

    #[test]
    fn a_straight_line_is_entirely_reached() {
        let c = census(&[NOP, NOP, NOP]);
        assert!(c.dead().is_empty());
        assert!(c.is_complete());
    }

    #[test]
    fn nothing_after_an_unconditional_jump_is_reached() {
        // SPIN jumps to itself, so the two words after it are unreachable —
        // the exact shape of code left behind by a deleted call site.
        let c = census(&[NOP, SPIN, NOP, NOP]);
        assert_eq!(c.dead(), vec![2, 3]);
        assert_eq!(c.dead_bytes(), 8);
    }

    #[test]
    fn both_arms_of_a_branch_are_reached() {
        // beq zero, zero, +8 — skips one instruction, and both the skipped one
        // and the target are live.
        let beq = 0x0004_0463; // beq x8, x0, 8
        let c = census(&[beq, NOP, NOP]);
        assert!(
            c.dead().is_empty(),
            "a branch does not kill its fall-through"
        );
    }

    #[test]
    fn an_indirect_jump_is_reported_rather_than_assumed() {
        // jalr zero, 0(ra) — a return. Where it goes is not in the instruction.
        let jalr = 0x0000_8067;
        let c = census(&[NOP, jalr, NOP]);
        assert_eq!(c.indirect, vec![1]);
        assert!(!c.is_complete(), "the analysis was blind and must say so");
    }

    #[test]
    fn an_undecodable_word_ends_the_path_and_is_named() {
        let c = census(&[NOP, 0xffff_ffff, NOP]);
        assert_eq!(c.undecodable, vec![1]);
        assert!(!c.is_complete());
    }
}
