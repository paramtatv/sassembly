//! Duplicate-function detector — task `B-024`, doc 18 §6a.3.
//!
//! The same body emitted twice is mass paid for twice, and the budget is zero.
//! `purpose.md`: a modern build is 100–1000× its shipped size, and duplication
//! is one of the ways that happens — a generic instantiated at two types that
//! compile to identical code, a function inlined and also kept, a helper copied
//! rather than shared.
//!
//! # Comparing bytes would not work, and decoding is why it does
//!
//! Two identical functions at different addresses do not have identical bytes.
//! Any internal branch encodes a displacement, and a displacement is measured
//! from the instruction that carries it, so the same loop at two addresses is
//! two different words.
//!
//! `विश्लेषणम्` returns a displacement as a signed distance rather than as the
//! bits holding it, so decoding normalises exactly the thing that differs.
//! Comparing decoded forms finds bodies that are the same *program*, which is
//! the question worth asking.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::vishlesana::decode;

/// A function found in the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    /// Instruction index it starts at.
    pub start: usize,
    /// How many instructions long.
    pub len: usize,
}

/// Two functions with the same body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Duplicate {
    /// The earlier one.
    pub first: usize,
    /// The later one, whose bytes are the waste.
    pub second: usize,
    /// Instructions in each.
    pub len: usize,
}

/// What one instruction is, with addresses normalised away.
type Shape = (String, Vec<i64>);

/// Every address something calls, plus the entry point.
///
/// A function is what a `jal` with a link register targets. A branch target is
/// not a function — it is a label inside one — and treating the two alike would
/// report every loop body as a candidate.
#[must_use]
fn function_starts(words: &[u32]) -> Vec<usize> {
    let mut starts = Vec::new();
    if !words.is_empty() {
        starts.push(0usize);
    }
    for (i, w) in words.iter().enumerate() {
        let Some(d) = decode(*w) else { continue };
        if d.insn != "jal" {
            continue;
        }
        // rd = zero is a plain jump, not a call.
        if d.operands.first().is_none_or(|(_, v)| *v == 0) {
            continue;
        }
        if let Some((_, disp)) = d.operands.iter().find(|(k, _)| k == "disp") {
            let target = i as i64 + disp / 4;
            if target >= 0 && (target as usize) < words.len() {
                starts.push(target as usize);
            }
        }
    }
    starts.sort_unstable();
    starts.dedup();
    starts
}

/// Functions, each running from its start to a return.
///
/// A body ends at `jalr` with no link register — a return. A function with no
/// return is taken to run to the next start, which is what an entry point that
/// never returns looks like.
#[must_use]
pub fn functions(words: &[u32]) -> Vec<Function> {
    let starts = function_starts(words);
    let mut out = Vec::new();
    for (n, start) in starts.iter().enumerate() {
        let limit = starts.get(n + 1).copied().unwrap_or(words.len());
        let mut end = limit;
        for (offset, w) in words[*start..limit].iter().enumerate() {
            let Some(d) = decode(*w) else { continue };
            // A return: `jalr` with no link register.
            if d.insn == "jalr" && d.operands.first().is_some_and(|(_, v)| *v == 0) {
                end = start + offset + 1;
                break;
            }
        }
        out.push(Function {
            start: *start,
            len: end - start,
        });
    }
    out
}

/// The decoded shape of a run of instructions.
fn shape(words: &[u32], f: &Function) -> Option<Vec<Shape>> {
    words
        .get(f.start..f.start + f.len)?
        .iter()
        .map(|w| decode(*w).map(|d| (d.insn, d.operands.into_iter().map(|(_, v)| v).collect())))
        .collect()
}

/// Functions whose bodies are identical once addresses are normalised away.
#[must_use]
pub fn duplicates(words: &[u32]) -> Vec<Duplicate> {
    let fns = functions(words);
    let mut out = Vec::new();
    for (i, a) in fns.iter().enumerate() {
        // A one-instruction function is a return or a jump, and every such
        // stub matching every other says nothing worth acting on.
        if a.len < 2 {
            continue;
        }
        let Some(sa) = shape(words, a) else { continue };
        for b in fns.iter().skip(i + 1) {
            if b.len != a.len {
                continue;
            }
            if shape(words, b).as_ref() == Some(&sa) {
                out.push(Duplicate {
                    first: a.start,
                    second: b.start,
                    len: a.len,
                });
            }
        }
    }
    out
}

/// Bytes a duplicate body costs.
#[must_use]
pub fn wasted_bytes(dups: &[Duplicate]) -> usize {
    dups.iter().map(|d| d.len * 4).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assemble Sassembly rather than hand-rolling J-type immediates.
    ///
    /// The first version of these tests built a `jal` by shifting bits into
    /// place by hand and got the target wrong by one instruction — the exact
    /// mistake `B-042` and `B-055` were about. The assembler is right there.
    fn words(src: &str) -> Vec<u32> {
        let p = crate::parse::assemble_program(src).expect("parses");
        crate::encode::words(&crate::encode::encode_program(&p).expect("encodes"))
    }

    #[test]
    fn a_call_target_is_a_function_and_a_branch_target_is_not() {
        // A branch inside a body must not look like a function, or every loop
        // is reported as a candidate.
        let w = words(
            "लङ्घनम् पुनःस्थानम्म् कार्यम्य् ।\n\
             समलङ्घनम् शून्यःन शून्यःत् चिह्नम्य् ।\n\
             चिह्नम्ॱॱ\n\
             योगः अर्थ०म् शून्यःन ०न ।\n\
             कार्यम्ॱॱ\n\
             योगः अर्थ०म् शून्यःन ७न ।\n\
             सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।\n",
        );
        assert_eq!(
            function_starts(&w),
            vec![0, 3],
            "the call target, not the branch target"
        );
    }

    #[test]
    fn two_identical_bodies_are_found() {
        let w = words(
            "लङ्घनम् पुनःस्थानम्म् एकम्य् ।\n\
             लङ्घनम् पुनःस्थानम्म् द्वितीयम्य् ।\n\
             लङ्घनम् शून्यःम् चक्रःय् ।\n\
             एकम्ॱॱ\n\
             योगः अर्थ०म् शून्यःन ७न ।\n\
             सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।\n\
             द्वितीयम्ॱॱ\n\
             योगः अर्थ०म् शून्यःन ७न ।\n\
             सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।\n\
             चक्रःॱॱ\n\
             लङ्घनम् शून्यःम् चक्रःय् ।\n",
        );
        let d = duplicates(&w);
        assert_eq!(d.len(), 1, "one pair, got {d:?}");
        assert_eq!(d[0].len, 2);
        assert_eq!(wasted_bytes(&d), 8);
    }

    #[test]
    fn bodies_that_differ_are_not_reported() {
        let w = words(
            "लङ्घनम् पुनःस्थानम्म् एकम्य् ।\n\
             लङ्घनम् पुनःस्थानम्म् द्वितीयम्य् ।\n\
             लङ्घनम् शून्यःम् चक्रःय् ।\n\
             एकम्ॱॱ\n\
             योगः अर्थ०म् शून्यःन ७न ।\n\
             सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।\n\
             द्वितीयम्ॱॱ\n\
             योगः अर्थ०म् शून्यःन ८न ।\n\
             सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।\n\
             चक्रःॱॱ\n\
             लङ्घनम् शून्यःम् चक्रःय् ।\n",
        );
        assert!(duplicates(&w).is_empty(), "7 and 8 are different programs");
    }

    #[test]
    fn a_stub_of_one_instruction_is_not_a_duplicate() {
        // Every `ret` matches every other `ret`, and saying so is noise.
        let w = words(
            "लङ्घनम् पुनःस्थानम्म् एकम्य् ।\n\
             लङ्घनम् पुनःस्थानम्म् द्वितीयम्य् ।\n\
             एकम्ॱॱ\n\
             सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।\n\
             द्वितीयम्ॱॱ\n\
             सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।\n",
        );
        assert!(duplicates(&w).is_empty());
    }
}
