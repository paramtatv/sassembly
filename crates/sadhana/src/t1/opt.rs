use crate::t1::ir::*;
use std::collections::HashSet;

/// What the optimiser refuses to run over. `W-203`: a pass that rewrites a
/// function must first be able to say the function is one — every terminator
/// target a block of this function (Rule X3, `research/22-two-conduits.md`
/// §4.3), every value a terminator reads defined somewhere in it. Silently
/// optimising a malformed function would strip the very instruction a dangling
/// reference needed and report success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrFault {
    /// A `Branch` or `CondBranch` names a block the function does not hold.
    TargetNotInFunction { from: BlockId, target: BlockId },
    /// A terminator — or, since `W-204`, an instruction — reads a value no
    /// instruction of the function defines.
    ValueNotDefined { block: BlockId, value: ValueId },
    /// `W-245`: a `Cmp` whose two operands are of different TYPES — one a
    /// comparison's own result (a बूल, ० or १) and the other an integer. The
    /// IR carries no types, so a value's type is what its defining instruction
    /// yields: `Cmp` a बूल, everything else an integer. `(अ न्यूनम् आ) समम् ३`
    /// is the shape refused; `(अ न्यूनम् आ) समम् (इ समम् ई)` is not.
    CmpOfDifferentTypes {
        block: BlockId,
        value: ValueId,
        left: ValueId,
        right: ValueId,
    },
}

impl std::fmt::Display for IrFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IrFault::TargetNotInFunction { from, target } => write!(
                f,
                "block {from:?} targets {target:?}, which is not a block of this function"
            ),
            IrFault::ValueNotDefined { block, value } => write!(
                f,
                "block {block:?} reads {value:?}, which no instruction defines"
            ),
            IrFault::CmpOfDifferentTypes {
                block,
                value,
                left,
                right,
            } => write!(
                f,
                "block {block:?}: {value:?} compares {left:?} with {right:?}, a बूल with an integer"
            ),
        }
    }
}

impl std::error::Error for IrFault {}

/// Run the passes over one function, or refuse the function.
pub fn optimize_function(func: &mut Function) -> Result<(), IrFault> {
    verify(func)?;
    dce_pass(func);
    Ok(())
}

/// The REFUSED case, checked before any pass rewrites anything.
fn verify(func: &Function) -> Result<(), IrFault> {
    let defined: HashSet<ValueId> = func
        .blocks
        .values()
        .flat_map(|b| b.insts.iter().map(|(v, _)| *v))
        .collect();
    // `W-245`: a value's TYPE is what defines it — a `Cmp` yields a बूल.
    let is_bool: std::collections::HashMap<ValueId, bool> = func
        .blocks
        .values()
        .flat_map(|b| {
            b.insts
                .iter()
                .map(|(v, i)| (*v, matches!(i, Instruction::Cmp(..))))
        })
        .collect();
    // `HashMap` iteration order is per-process random; the FIRST fault must
    // be the same one every run (`W-144`'s reason for `Ord` on `ValueId`).
    let mut ids: Vec<BlockId> = func.blocks.keys().copied().collect();
    ids.sort_by_key(|b| b.0);
    for id in ids {
        let block = &func.blocks[&id];
        let (targets, reads): (Vec<BlockId>, Option<ValueId>) = match block.terminator {
            Some(Terminator::CondBranch(c, t, e)) => (vec![t, e], Some(c)),
            Some(Terminator::Branch(t)) => (vec![t], None),
            Some(Terminator::Return(v)) => (Vec::new(), v),
            Some(Terminator::Unreachable) | None => (Vec::new(), None),
        };
        for target in targets {
            if !func.blocks.contains_key(&target) {
                return Err(IrFault::TargetNotInFunction {
                    from: block.id,
                    target,
                });
            }
        }
        // `W-204`: an INSTRUCTION's operands are checked as a terminator's
        // are. A `Call`'s arguments and a binary kind's two operands must be
        // values some instruction of the function defines; "reads a value
        // nothing defines" is one fault whoever the reader is, so it is the
        // same variant. A Call is a DCE root, and a root pointing at nothing
        // would otherwise be "kept" and handed to the emitter. `W-245`: the
        // operand list is `Instruction::operands`, so the eleven kinds that row
        // added are checked by the same loop; and a `Cmp` of a बूल with an
        // integer is refused by name.
        for (v, inst) in &block.insts {
            for value in inst.operands() {
                if !defined.contains(&value) {
                    return Err(IrFault::ValueNotDefined {
                        block: block.id,
                        value,
                    });
                }
            }
            if let Instruction::Cmp(_, l, r) = inst
                && is_bool[l] != is_bool[r]
            {
                return Err(IrFault::CmpOfDifferentTypes {
                    block: block.id,
                    value: *v,
                    left: *l,
                    right: *r,
                });
            }
        }
        if let Some(value) = reads
            && !defined.contains(&value)
        {
            return Err(IrFault::ValueNotDefined {
                block: block.id,
                value,
            });
        }
    }
    Ok(())
}

/// Dead Code Elimination (DCE) Pass
/// Removes instructions whose results are never used.
fn dce_pass(func: &mut Function) {
    let mut used_values = HashSet::new();

    // 1. Collect all uses from terminators. A `CondBranch` READS its
    //    condition (`W-198` added the variant; `W-203` added the root): without
    //    this line the condition's defining instruction is dead by DCE's
    //    reckoning and the terminator is left pointing at nothing.
    for block in func.blocks.values() {
        match block.terminator {
            Some(Terminator::Return(Some(val))) | Some(Terminator::CondBranch(val, _, _)) => {
                used_values.insert(val);
            }
            _ => {}
        }
    }

    // 2. Iteratively collect uses from instructions (backwards/fixpoint)
    let mut changed = true;
    while changed {
        changed = false;
        for block in func.blocks.values() {
            for (def_val, inst) in &block.insts {
                // If this instruction's result is used, mark its operands as used.
                // We also unconditionally keep Call instructions for side-effects,
                // but since we want to be simple, let's just mark operands of any used instruction.

                let is_used = used_values.contains(def_val);
                // `W-245`: a `Store` is a root as a `Call` is — the next `Load`
                // of its slot reads what it wrote — and every kind's operands
                // come from `Instruction::operands`.
                if is_used || inst.is_side_effecting() {
                    for operand in inst.operands() {
                        if used_values.insert(operand) {
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    // 3. Remove unused instructions
    for block in func.blocks.values_mut() {
        block
            .insts
            .retain(|(def_val, inst)| used_values.contains(def_val) || inst.is_side_effecting());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::t1::ast::SymbolId;
    use std::collections::HashMap;

    #[test]
    fn test_dce_removes_unused_add() {
        let v1 = ValueId(1);
        let v2 = ValueId(2);
        let v3 = ValueId(3); // Unused result of add

        let block = Block {
            id: BlockId(0),
            insts: vec![
                (v1, Instruction::ConstInt(10)),
                (v2, Instruction::ConstInt(20)),
                (v3, Instruction::Add(v1, v2)), // Unused Add
            ],
            terminator: Some(Terminator::Return(Some(v1))), // Only uses v1
        };

        let mut blocks = HashMap::new();
        blocks.insert(BlockId(0), block);

        let mut func = Function {
            name: SymbolId(0),
            blocks,
            entry_block: BlockId(0),
        };

        optimize_function(&mut func).expect("a well-formed function is optimised");

        let entry = &func.blocks[&func.entry_block];
        // Expect v2 and v3 to be stripped because they are dead
        assert_eq!(entry.insts.len(), 1);
        assert_eq!(entry.insts[0].0, v1);
    }

    /// The shape `W-198` pinned on the IR: `यदि` with no `अन्यथा`, the
    /// else-target IS the join. Entry computes the condition and branches on
    /// it; `then` falls through to `join`; `join` returns nothing.
    fn if_without_else() -> (Function, BlockId, BlockId, BlockId, ValueId) {
        let entry = BlockId(0);
        let then = BlockId(1);
        let join = BlockId(2);
        let lhs = ValueId(0);
        let rhs = ValueId(1);
        let cond = ValueId(2);
        let mut blocks = HashMap::new();
        blocks.insert(
            entry,
            Block {
                id: entry,
                insts: vec![
                    (lhs, Instruction::ConstInt(1)),
                    (rhs, Instruction::ConstInt(1)),
                    (cond, Instruction::Sub(lhs, rhs)),
                ],
                terminator: Some(Terminator::CondBranch(cond, then, join)),
            },
        );
        blocks.insert(
            then,
            Block {
                id: then,
                insts: Vec::new(),
                terminator: Some(Terminator::Branch(join)),
            },
        );
        blocks.insert(
            join,
            Block {
                id: join,
                insts: Vec::new(),
                terminator: Some(Terminator::Return(None)),
            },
        );
        let func = Function {
            name: SymbolId(0),
            blocks,
            entry_block: entry,
        };
        (func, entry, then, join, cond)
    }

    /// `W-203`: before this row, DCE rooted only `Return(Some(_))`, so a
    /// function whose only use of a value was a `CondBranch` lost the
    /// condition AND its operands — three instructions gone, terminator
    /// pointing at a value nothing defined. Nothing returns here, so every
    /// instruction survives ONLY because the branch reads the condition.
    #[test]
    fn a_cond_branch_keeps_the_instructions_its_condition_needs() {
        let (mut func, entry, _, _, cond) = if_without_else();

        optimize_function(&mut func).expect("a well-formed if is optimised");

        let kept: Vec<ValueId> = func.blocks[&entry].insts.iter().map(|(v, _)| *v).collect();
        assert_eq!(
            kept,
            vec![ValueId(0), ValueId(1), cond],
            "the condition and both operands it is computed from survive DCE"
        );
        let Some(Terminator::CondBranch(c, _, _)) = func.blocks[&entry].terminator else {
            panic!("the terminator is untouched");
        };
        assert!(
            func.blocks[&entry].insts.iter().any(|(v, _)| *v == c),
            "the terminator's condition is still defined in its block"
        );
    }

    /// Still stripped: an instruction the branch does NOT read. The root is
    /// the condition, not the whole block.
    #[test]
    fn a_cond_branch_does_not_keep_what_it_does_not_read() {
        let (mut func, entry, _, _, _) = if_without_else();
        let stray = ValueId(9);
        func.blocks
            .get_mut(&entry)
            .unwrap()
            .insts
            .push((stray, Instruction::ConstInt(7)));

        optimize_function(&mut func).expect("a well-formed if is optimised");

        assert!(
            !func.blocks[&entry].insts.iter().any(|(v, _)| *v == stray),
            "an unread constant before a CondBranch is dead"
        );
        assert_eq!(func.blocks[&entry].insts.len(), 3);
    }

    /// REFUSED: a `CondBranch` whose then-target is not a block of this
    /// function (Rule X3 — no jump-to-anywhere). The pass returns the fault
    /// and rewrites NOTHING: every instruction is still there afterwards.
    #[test]
    fn a_cond_branch_out_of_its_function_is_refused_untouched() {
        let (mut func, entry, _, join, cond) = if_without_else();
        let elsewhere = BlockId(77);
        func.blocks.get_mut(&entry).unwrap().terminator =
            Some(Terminator::CondBranch(cond, elsewhere, join));
        let before = func.blocks[&entry].insts.len();

        let fault =
            optimize_function(&mut func).expect_err("a jump out of the function is refused");

        assert_eq!(
            fault,
            IrFault::TargetNotInFunction {
                from: entry,
                target: elsewhere
            }
        );
        assert_eq!(
            fault.to_string(),
            "block BlockId(0) targets BlockId(77), which is not a block of this function"
        );
        assert_eq!(
            func.blocks[&entry].insts.len(),
            before,
            "a refused function is not rewritten"
        );
    }

    /// REFUSED: a `CondBranch` whose condition no instruction defines. This
    /// is exactly the state the old DCE would have LEFT a function in; now it
    /// is what the pass refuses to start from.
    #[test]
    fn a_cond_branch_on_an_undefined_value_is_refused() {
        let (mut func, entry, then, join, _) = if_without_else();
        let ghost = ValueId(40);
        func.blocks.get_mut(&entry).unwrap().terminator =
            Some(Terminator::CondBranch(ghost, then, join));

        let fault =
            optimize_function(&mut func).expect_err("a condition nothing defines is refused");

        assert_eq!(
            fault,
            IrFault::ValueNotDefined {
                block: entry,
                value: ghost
            }
        );
    }

    /// The same two checks hold for the terminators that were already there:
    /// a `Branch` out of the function, and a `Return` of an undefined value.
    #[test]
    fn a_plain_branch_out_of_its_function_is_refused_too() {
        let (mut func, _, then, _, _) = if_without_else();
        func.blocks.get_mut(&then).unwrap().terminator = Some(Terminator::Branch(BlockId(5)));
        assert_eq!(
            optimize_function(&mut func).expect_err("refused"),
            IrFault::TargetNotInFunction {
                from: then,
                target: BlockId(5)
            }
        );

        let (mut func, _, _, join, _) = if_without_else();
        func.blocks.get_mut(&join).unwrap().terminator =
            Some(Terminator::Return(Some(ValueId(41))));
        assert_eq!(
            optimize_function(&mut func).expect_err("refused"),
            IrFault::ValueNotDefined {
                block: join,
                value: ValueId(41)
            }
        );
    }

    /// `ग आरभ्य ३ समाप्तम्` as `ir.t1` lowers it — `v0 = ConstInt(3)`,
    /// `v1 = Call(ग, [v0])`, `Return(Some(v1))` — through the optimiser.
    fn a_lowered_call() -> (Function, BlockId, ValueId, ValueId) {
        let entry = BlockId(0);
        let three = ValueId(0);
        let result = ValueId(1);
        let mut blocks = HashMap::new();
        blocks.insert(
            entry,
            Block {
                id: entry,
                insts: vec![
                    (three, Instruction::ConstInt(3)),
                    (result, Instruction::Call(SymbolId(7), vec![three])),
                ],
                terminator: Some(Terminator::Return(Some(result))),
            },
        );
        (
            Function {
                name: SymbolId(0),
                blocks,
                entry_block: entry,
            },
            entry,
            three,
            result,
        )
    }

    /// `W-204`: a Call's operands are DCE roots and its result is a defined
    /// value. The lowered `ग आरभ्य ३ समाप्तम्` must come out of the optimiser
    /// whole — the `ConstInt` that only the call reads is NOT dead, and the
    /// `Return` reading the call's result is not refused.
    #[test]
    fn a_lowered_call_keeps_its_arguments_and_its_result_is_defined() {
        let (mut func, entry, three, result) = a_lowered_call();
        optimize_function(&mut func).expect("a lowered call is a well-formed function");
        let insts = &func.blocks[&entry].insts;
        assert_eq!(
            insts.len(),
            2,
            "both instructions survive: the argument is rooted by the call; got {insts:?}"
        );
        assert!(
            matches!(insts[0], (v, Instruction::ConstInt(3)) if v == three),
            "the argument `३` survives DCE because the Call reads it"
        );
        assert!(
            matches!(&insts[1], (v, Instruction::Call(_, args)) if *v == result && args == &vec![three]),
            "the Call survives with its one argument"
        );
    }

    /// THE REFUSED CASE, `W-204`: a Call reading a value no instruction
    /// defines is refused by `optimize_function` before any pass runs — an
    /// argument that was never lowered is the fault a builder that appended
    /// the Call before its arguments would leave behind, and DCE would
    /// otherwise "keep" a root that points at nothing.
    #[test]
    fn a_call_reading_an_undefined_value_is_refused() {
        let (mut func, entry, _three, result) = a_lowered_call();
        let ghost = ValueId(9);
        func.blocks.get_mut(&entry).unwrap().insts[1] =
            (result, Instruction::Call(SymbolId(7), vec![ghost]));
        // `Instruction` derives no `PartialEq`; the debug rendering is the witness.
        let before = format!("{:?}", func.blocks[&entry].insts);
        let fault = optimize_function(&mut func).expect_err("a Call reading ValueId(9) is refused");
        assert_eq!(
            fault,
            IrFault::ValueNotDefined {
                block: entry,
                value: ghost
            }
        );
        assert_eq!(
            format!("{:?}", func.blocks[&entry].insts),
            before,
            "refused means UNTOUCHED: no pass ran over the malformed function"
        );
    }

    /// `W-245`: `चरः क भवति ३ । प्रत्यागमनम् क ।` — a `Store` survives DCE
    /// although nothing reads the value it defines (the `Load` reads the SLOT),
    /// the `Load` survives because the return reads it, and a second `Load`
    /// nobody reads is stripped. Every new kind's operands are checked: a `Mul`
    /// of a value nothing defines is refused by the same name as a `Call`'s.
    #[test]
    fn a_store_is_a_root_an_unread_load_is_dead_and_every_new_kind_is_operand_checked() {
        let entry = BlockId(0);
        let (three, st, ld, stray, prod) =
            (ValueId(0), ValueId(1), ValueId(2), ValueId(3), ValueId(4));
        let mut blocks = HashMap::new();
        blocks.insert(
            entry,
            Block {
                id: entry,
                insts: vec![
                    (three, Instruction::ConstInt(3)),
                    (st, Instruction::Store(0, three)),
                    (ld, Instruction::Load(0)),
                    (stray, Instruction::Load(0)),
                ],
                terminator: Some(Terminator::Return(Some(ld))),
            },
        );
        let mut func = Function {
            name: SymbolId(0),
            blocks,
            entry_block: entry,
        };
        optimize_function(&mut func).expect("well-formed");
        let kept: Vec<ValueId> = func.blocks[&entry].insts.iter().map(|(v, _)| *v).collect();
        assert_eq!(
            kept,
            vec![three, st, ld],
            "the constant (rooted by the Store), the Store (a root) and the returned Load survive; the stray Load does not"
        );

        for make in [
            |g| Instruction::Mul(ValueId(0), g),
            |g| Instruction::Div(g, ValueId(0)),
            |g| Instruction::Rem(ValueId(0), g),
            |g| Instruction::Shl(g, ValueId(0)),
            |g| Instruction::Shr(ValueId(0), g),
            |g| Instruction::And(g, ValueId(0)),
            |g| Instruction::Or(ValueId(0), g),
            |g| Instruction::Xor(g, ValueId(0)),
            |g| Instruction::Cmp(CmpOp::Ltu, ValueId(0), g),
            |g| Instruction::Store(1, g),
        ] {
            let ghost = ValueId(77);
            func.blocks
                .get_mut(&entry)
                .unwrap()
                .insts
                .push((prod, make(ghost)));
            assert_eq!(
                optimize_function(&mut func).expect_err("a value nothing defines is refused"),
                IrFault::ValueNotDefined {
                    block: entry,
                    value: ghost
                }
            );
            func.blocks.get_mut(&entry).unwrap().insts.pop();
        }
    }

    /// `W-283` — **A `StoreAt` SURVIVES DCE, AND THIS IS THE ONLY KIND OF TEST
    /// THAT CAN CATCH THE FAILURE IT GUARDS.**
    ///
    /// `is_side_effecting` is a hand-written list, not an exhaustive `match`. A
    /// store kind missing from it defines a value nobody reads, so DCE deletes
    /// it — and the program still compiles, still assembles, still runs, and
    /// silently drops every write. There is no diagnostic on that path.
    ///
    /// **So asserting that a store EMITS proves nothing**: the emission is real
    /// and the deletion happens afterwards, in a later pass, to a correct
    /// instruction. The assertion has to be that the write is still there once
    /// the optimiser has run.
    ///
    /// Nothing here reads `addr`, `val` or the store. If `StoreAt` were absent
    /// from `is_side_effecting` the store would go first, and `addr` and `val`
    /// would follow it as newly-dead — the block would optimise to EMPTY, which
    /// is the shape this asserts against.
    #[test]
    fn a_store_at_survives_dead_code_elimination() {
        let entry = BlockId(0);
        let (addr, val, st) = (ValueId(0), ValueId(1), ValueId(2));
        let mut blocks = HashMap::new();
        blocks.insert(
            entry,
            Block {
                id: entry,
                insts: vec![
                    (addr, Instruction::AddrOfGlobal(SymbolId(0))),
                    (val, Instruction::ConstInt(7)),
                    (st, Instruction::StoreAt(addr, val)),
                ],
                terminator: Some(Terminator::Return(None)),
            },
        );
        let mut func = Function {
            name: SymbolId(0),
            blocks,
            entry_block: entry,
        };
        optimize_function(&mut func).expect("well-formed");
        let kept: Vec<ValueId> = func.blocks[&entry].insts.iter().map(|(v, _)| *v).collect();
        assert_eq!(
            kept,
            vec![addr, val, st],
            "a StoreAt is a DCE root; dropping it would delete a write with no diagnostic \
             anywhere, and would take the address and the stored value down with it"
        );

        // AND ITS OPERANDS ARE BOTH REAL USES. Listing only the address would
        // let DCE delete whatever computed the value while keeping the store
        // that writes it — a store of an undefined register, silent again.
        for make in [
            |g| Instruction::StoreAt(ValueId(0), g),
            |g| Instruction::StoreAt(g, ValueId(1)),
            |g| Instruction::LoadAt(g),
        ] {
            let ghost = ValueId(77);
            func.blocks
                .get_mut(&entry)
                .unwrap()
                .insts
                .push((ValueId(9), make(ghost)));
            assert_eq!(
                optimize_function(&mut func).expect_err("a value nothing defines is refused"),
                IrFault::ValueNotDefined {
                    block: entry,
                    value: ghost
                }
            );
            func.blocks.get_mut(&entry).unwrap().insts.pop();
        }
    }

    /// THE REFUSED CASE, `W-245`: a `Cmp` of a बूल with an integer —
    /// `(अ न्यूनम् आ) समम् ३` — is refused by name before any pass runs, and a
    /// `Cmp` of two बूलs or two integers is not.
    #[test]
    fn a_compare_of_a_bool_with_an_integer_is_refused_by_name() {
        let entry = BlockId(0);
        let (a, b, lt, three, mixed) = (ValueId(0), ValueId(1), ValueId(2), ValueId(3), ValueId(4));
        let mut blocks = HashMap::new();
        blocks.insert(
            entry,
            Block {
                id: entry,
                insts: vec![
                    (a, Instruction::ConstInt(1)),
                    (b, Instruction::ConstInt(2)),
                    (lt, Instruction::Cmp(CmpOp::Lt, a, b)),
                    (three, Instruction::ConstInt(3)),
                    (mixed, Instruction::Cmp(CmpOp::Eq, lt, three)),
                ],
                terminator: Some(Terminator::Return(Some(mixed))),
            },
        );
        let mut func = Function {
            name: SymbolId(0),
            blocks,
            entry_block: entry,
        };
        let fault = optimize_function(&mut func).expect_err("a बूल against an integer is refused");
        assert_eq!(
            fault,
            IrFault::CmpOfDifferentTypes {
                block: entry,
                value: mixed,
                left: lt,
                right: three
            }
        );
        assert_eq!(
            fault.to_string(),
            "block BlockId(0): ValueId(4) compares ValueId(2) with ValueId(3), a बूल with an integer"
        );
        // Two बूलs compare; two integers compare.
        let same = ValueId(5);
        let insts = &mut func.blocks.get_mut(&entry).unwrap().insts;
        insts[4] = (same, Instruction::Cmp(CmpOp::Lt, b, a));
        insts.push((mixed, Instruction::Cmp(CmpOp::Ne, lt, same)));
        optimize_function(&mut func).expect("बूल against बूल, integer against integer");
    }
}
