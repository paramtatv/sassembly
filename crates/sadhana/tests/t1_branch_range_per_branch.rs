//! `W-306` — EVERY CONDITIONAL IS MEASURED AT ITS OWN ADDRESS.
//!
//! `riscv64::check_branch_ranges` refuses a routine whose text puts a
//! `CondBranch` further than RISC-V's B-type ±4 KiB from its target. It found
//! the branch line with
//! `lines.iter().position(|l| is_conditional(l) && l.ends_with(&operand))` —
//! the FIRST conditional in the whole routine naming that target — while the
//! refusal it raised named the block it was iterating. So TWO blocks branching
//! conditionally to ONE target were both measured at the first one's address,
//! and a near branch at the top of a routine certified every later branch to
//! the same join as near.
//!
//! That is not a corner: two conditionals into one join is the shape of a `यदि`
//! inside a `यावत्`, and `ir.t1` — the module whose far branch opened this row —
//! is full of them. The guard was blind on the commonest control flow the
//! language has, and blind in the UNSAFE direction: it accepted what the
//! assembler must then either refuse or encode as a wrapped immediate.
//!
//! THE CASE THAT MUST STILL BE REFUSED is the other half of this file, and it
//! is the reason the fix is not "drop the guard": a routine with ONE far
//! conditional, and a routine whose two branches to a join are BOTH far, must
//! still be refused, and a short routine with two branches to one join must
//! still be accepted. All four are below.

use sadhana::t1::ast::SymbolId;
use sadhana::t1::ir::{Block, BlockId, Function, Instruction, Terminator, ValueId};
use sadhana::t1::riscv64::{self, Module, Names, Refusal};

/// A block of `n` `ConstInt` instructions ending in `Branch(next)` — the padding
/// that puts distance between a branch and its target. `ConstInt` is used
/// because it reads no value, so the padding cannot change what the register
/// allocator does to the values the branches actually compare.
fn filler(id: usize, next: usize, n: usize, v: &mut usize) -> Block {
    let mut insts = Vec::new();
    for _ in 0..n {
        insts.push((ValueId(*v), Instruction::ConstInt(1)));
        *v += 1;
    }
    Block {
        id: BlockId(id),
        insts,
        terminator: Some(Terminator::Branch(BlockId(next))),
    }
}

/// The refusal, with a SHORT message when there is none. `Result::expect_err`
/// prints the `Ok` — here a 1,200-line routine, 106 KB of escaped Devanagari in
/// the failure report, which buries the one fact the reader needs.
fn refusal(m: &Module) -> Refusal {
    match riscv64::emit_module(m) {
        Err(r) => r,
        Ok(text) => panic!(
            "accepted a routine it must refuse ({} lines emitted)",
            text.lines().count()
        ),
    }
}

fn module(functions: Vec<Function>) -> Module {
    let mut names = Names::new();
    names.insert(SymbolId(1), ("परी".to_string(), "कार्यम्".to_string()));
    Module {
        globals: Vec::new(),
        name: "परी".to_string(),
        functions,
        names,
        entry: None,
    }
}

/// Two conditionals into ONE join, with the join placed BEFORE the padding so
/// that the first branch is near it and the second is `pad` words past it.
///
/// Blocks are emitted in id order, which is why the layout is 0 → 1 → 2 → 3 → 4
/// and not the order a reader would write it in: `BlockId(1)` is the JOIN, so
/// `BlockId(0)`'s forward branch to it is a couple of words, and `BlockId(3)`'s
/// branch back to it crosses the whole of `BlockId(2)`'s padding. Putting the
/// join last instead — the first shape this test had — makes BOTH branches far
/// and the test cannot tell the two rules apart: the retired rule refused it
/// too, naming `BlockId(0)`.
fn two_branches_to_one_join(pad: usize) -> Function {
    let mut v = 0usize;
    let mut blocks = std::collections::HashMap::new();
    // `ConstInt` as the condition keeps this the synthesized
    // `विषमलङ्घनम् … शून्यःत्` form rather than a fused compare, which is the
    // form with the fewest moving parts here.
    let c0 = ValueId(v);
    v += 1;
    blocks.insert(
        BlockId(0),
        Block {
            id: BlockId(0),
            insts: vec![(c0, Instruction::ConstInt(1))],
            terminator: Some(Terminator::CondBranch(c0, BlockId(1), BlockId(2))),
        },
    );
    blocks.insert(
        BlockId(1),
        Block {
            id: BlockId(1),
            insts: Vec::new(),
            terminator: Some(Terminator::Return(None)),
        },
    );
    blocks.insert(BlockId(2), filler(2, 3, pad, &mut v));
    let c1 = ValueId(v);
    blocks.insert(
        BlockId(3),
        Block {
            id: BlockId(3),
            insts: vec![(c1, Instruction::ConstInt(1))],
            terminator: Some(Terminator::CondBranch(c1, BlockId(1), BlockId(4))),
        },
    );
    blocks.insert(
        BlockId(4),
        Block {
            id: BlockId(4),
            insts: Vec::new(),
            terminator: Some(Terminator::Return(None)),
        },
    );
    Function {
        name: SymbolId(1),
        blocks,
        entry_block: BlockId(0),
    }
}

#[test]
fn the_second_conditional_to_a_join_is_measured_at_its_own_address() {
    // 1,200 padding instructions — each one word — put `BlockId(3)`'s branch
    // more than 4,096 bytes AFTER `BlockId(1)`, while `BlockId(0)`'s branch to
    // the same block is a couple of words before it. Under the retired rule both
    // were measured at `BlockId(0)`'s line and the routine was accepted.
    match refusal(&module(vec![two_branches_to_one_join(1200)])) {
        Refusal::BranchOutOfRange {
            from,
            target,
            bytes,
            ..
        } => {
            assert_eq!(
                from,
                BlockId(3),
                "the far branch is the one in BlockId(3); BlockId(0)'s is near"
            );
            assert_eq!(target, BlockId(1));
            assert!(
                bytes <= -4096,
                "measured at its own address, not at BlockId(0)'s: {bytes}"
            );
        }
        other => panic!("wrong refusal: {other:?}"),
    }
}

#[test]
fn two_near_branches_to_one_join_are_still_accepted() {
    // THE CASE THAT MUST NOT BE REFUSED. The shape is identical — two
    // conditionals into one join — and only the distance differs, so a fix that
    // refused on the shape rather than on the measurement would red here.
    riscv64::emit_module(&module(vec![two_branches_to_one_join(8)]))
        .expect("eight words of padding is inside ±4 KiB for both branches");
}

#[test]
fn a_lone_far_conditional_is_still_refused() {
    // The guard's original case, unchanged by this row: one conditional, its
    // target beyond ±4 KiB. Kept because the measurement moved underneath it.
    let mut v = 0usize;
    let mut blocks = std::collections::HashMap::new();
    let c = ValueId(v);
    // The padding's values continue from here: `filler` would otherwise start at
    // `ValueId(0)` again and the condition would share an id with a constant.
    v += 1;
    blocks.insert(
        BlockId(0),
        Block {
            id: BlockId(0),
            insts: vec![(c, Instruction::ConstInt(1))],
            terminator: Some(Terminator::CondBranch(c, BlockId(2), BlockId(1))),
        },
    );
    blocks.insert(BlockId(1), filler(1, 2, 1200, &mut v));
    blocks.insert(
        BlockId(2),
        Block {
            id: BlockId(2),
            insts: Vec::new(),
            terminator: Some(Terminator::Return(None)),
        },
    );
    let func = Function {
        name: SymbolId(1),
        blocks,
        entry_block: BlockId(0),
    };
    match refusal(&module(vec![func])) {
        Refusal::BranchOutOfRange { from, target, .. } => {
            assert_eq!(from, BlockId(0));
            assert_eq!(target, BlockId(2));
        }
        other => panic!("wrong refusal: {other:?}"),
    }
}

#[test]
fn a_backward_conditional_past_the_limit_is_refused_by_its_own_distance() {
    // A BACK-EDGE — the `यावत्` shape, and the sign of `bytes` is negative. The
    // padding sits between the loop head and the branch that jumps back to it,
    // so the branch is the LAST block and its target is the FIRST. Under the
    // retired rule this one branch was also the first conditional to its
    // target, so it was measured correctly; it is here because the walk that
    // replaces `position` must still attribute a branch in an UNLABELLED entry
    // block's successor, and a back-edge makes the entry itself labelled.
    let mut v = 0usize;
    let mut blocks = std::collections::HashMap::new();
    blocks.insert(BlockId(0), filler(0, 1, 1200, &mut v));
    let c = ValueId(v);
    blocks.insert(
        BlockId(1),
        Block {
            id: BlockId(1),
            insts: vec![(c, Instruction::ConstInt(1))],
            terminator: Some(Terminator::CondBranch(c, BlockId(0), BlockId(2))),
        },
    );
    blocks.insert(
        BlockId(2),
        Block {
            id: BlockId(2),
            insts: Vec::new(),
            terminator: Some(Terminator::Return(None)),
        },
    );
    let func = Function {
        name: SymbolId(1),
        blocks,
        entry_block: BlockId(0),
    };
    match refusal(&module(vec![func])) {
        Refusal::BranchOutOfRange {
            from,
            target,
            bytes,
            ..
        } => {
            assert_eq!(from, BlockId(1));
            assert_eq!(target, BlockId(0));
            assert!(bytes <= -4096, "a back-edge measures negative: {bytes}");
        }
        other => panic!("wrong refusal: {other:?}"),
    }
}
