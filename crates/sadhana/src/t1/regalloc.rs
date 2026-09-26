use crate::t1::ir::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    Register(u8),
    Spill(usize),
}

pub struct AllocationMap {
    pub locations: HashMap<ValueId, Location>,
    pub num_spills: usize,
}

pub fn allocate_registers(func: &Function, num_registers: u8) -> AllocationMap {
    let mut lifetimes: HashMap<ValueId, (usize, usize)> = HashMap::new();
    let mut linear_seq = Vec::new();

    // 1. Flatten CFG to a linear sequence (naive straight-line blocks)
    // We assume block 0 is entry and just iterate blocks sequentially for this simple pass
    // A real allocator would do a topological sort.
    let mut sorted_blocks: Vec<_> = func.blocks.keys().copied().collect();
    sorted_blocks.sort_by_key(|b| b.0);

    for block_id in sorted_blocks {
        let block = &func.blocks[&block_id];
        for (def_val, inst) in &block.insts {
            linear_seq.push((*def_val, inst.clone()));
        }
        if let Some(Terminator::Return(Some(v))) = block.terminator {
            linear_seq.push((v, Instruction::ConstInt(0))); // Dummy use
        }
    }

    // 2. Compute lifetimes [start, end] indices
    for (i, (def_val, inst)) in linear_seq.iter().enumerate() {
        // Definition
        lifetimes.entry(*def_val).or_insert((i, i));

        // Uses — `Instruction::operands`, so a kind added to the IR (`W-245`'s
        // eleven) extends its operands' intervals without a new arm here.
        for operand in inst.operands() {
            if let Some(lt) = lifetimes.get_mut(&operand) {
                lt.1 = i;
            }
        }
    }

    // 3. Linear Scan
    // Sort intervals by start point
    // The key is total, and the honest reason is narrower than it first looked.
    //
    // IT WAS REPORTED AS A LIVE REPRODUCIBILITY BUG: `lifetimes` is a `HashMap`,
    // Rust randomises its iteration per process, and a sort by `start` alone is
    // stable — so equal-start ties would be broken by hash order and the same IR
    // would allocate differently run to run. THAT IS NOT TRUE HERE, and the
    // premise is the part that fails. `:38` does
    // `lifetimes.entry(*def_val).or_insert((i, i))` exactly once per iteration of
    // the `enumerate` above, so EVERY START INDEX IS UNIQUE — there are no ties
    // to break, and the old key was already a total order. The hash order never
    // reached the output.
    //
    // The `id` stays in the key anyway, because it makes the property hold BY
    // CONSTRUCTION rather than by an invariant maintained thirty lines earlier:
    // if lifetime computation ever gives two values the same start — a phi node,
    // a block-entry live-in, a fused definition — the assignment stays a function
    // of the IR without anyone having to notice. It is defence, not a fix.
    let mut intervals: Vec<(ValueId, (usize, usize))> = lifetimes.into_iter().collect();
    intervals.sort_by_key(|&(id, (start, _))| (start, id));

    let mut locations = HashMap::new();
    let mut active: Vec<(ValueId, usize, u8)> = Vec::new(); // (id, end, reg)
    let mut free_registers: Vec<u8> = (0..num_registers).rev().collect();
    let mut next_spill = 0;

    for (vid, (start, end)) in intervals {
        // Expire old intervals
        active.retain(|&(_, a_end, reg)| {
            if a_end <= start {
                free_registers.push(reg);
                false
            } else {
                true
            }
        });

        if let Some(reg) = free_registers.pop() {
            // Allocate register
            locations.insert(vid, Location::Register(reg));
            active.push((vid, end, reg));
            // Keep active sorted by end time (descending, so last is largest end time)
            active.sort_by_key(|&(_, e, _)| e);
        } else {
            // Spill
            // We should spill the one that ends last
            if let Some(&(spill_vid, spill_end, spill_reg)) = active.last() {
                if spill_end > end {
                    // Spill the active one, assign its register to current vid
                    locations.insert(spill_vid, Location::Spill(next_spill));
                    next_spill += 1;

                    locations.insert(vid, Location::Register(spill_reg));

                    // Replace in active list
                    active.pop();
                    active.push((vid, end, spill_reg));
                    active.sort_by_key(|&(_, e, _)| e);
                } else {
                    // Spill current vid
                    locations.insert(vid, Location::Spill(next_spill));
                    next_spill += 1;
                }
            } else {
                // Spill current vid (shouldn't happen if num_registers > 0)
                locations.insert(vid, Location::Spill(next_spill));
                next_spill += 1;
            }
        }
    }

    AllocationMap {
        locations,
        num_spills: next_spill,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::t1::ast::SymbolId;

    #[test]
    fn test_allocate_registers_spill() {
        let mut block = Block {
            id: BlockId(0),
            insts: vec![],
            terminator: Some(Terminator::Return(Some(ValueId(0)))),
        };

        // Create 3 active intervals overlapping entirely
        block.insts.push((ValueId(1), Instruction::ConstInt(10)));
        block.insts.push((ValueId(2), Instruction::ConstInt(20)));
        block.insts.push((ValueId(3), Instruction::ConstInt(30)));

        // Uses all of them at the end so they all live until the end
        block
            .insts
            .push((ValueId(4), Instruction::Add(ValueId(1), ValueId(2))));
        block
            .insts
            .push((ValueId(5), Instruction::Add(ValueId(4), ValueId(3))));

        let mut blocks = HashMap::new();
        blocks.insert(BlockId(0), block);
        let func = Function {
            name: SymbolId(0),
            blocks,
            entry_block: BlockId(0),
        };

        // Allocate with only 2 registers available
        let alloc = allocate_registers(&func, 2);

        // We expect at least one spill since max pressure is 3 and we have 2 regs
        assert!(alloc.num_spills > 0);
    }

    /// The same IR must allocate the same registers. Twenty times.
    ///
    /// `W-144`, AND THIS TEST CANNOT CURRENTLY FAIL — which is stated here
    /// rather than left for someone to discover, because a test that cannot
    /// fail is decoration unless it says so.
    ///
    /// It was written for a reported bug that turned out not to exist: that
    /// `HashMap` iteration order leaked into register assignment through
    /// equal-start ties. It cannot, because `:38` inserts each value's start
    /// exactly once per `enumerate` index, so no two values share a start and
    /// there is nothing for hash order to break. The mutation confirms it —
    /// putting the old `sort_by_key(start)` back leaves this test GREEN.
    ///
    /// It is kept as a REGRESSION GUARD, not as evidence. The property it
    /// asserts — allocation is a function of the IR — is one this allocator
    /// happens to have today and could lose quietly: a phi node, a block-entry
    /// live-in, or any change that gives two values the same start would
    /// reintroduce exactly the tie that was feared, and this test would then be
    /// the thing that notices. Do not cite it as proof that ordering is sound.
    #[test]
    fn the_same_ir_allocates_the_same_registers_every_time() {
        fn build() -> Function {
            let mut block = Block {
                id: BlockId(0),
                insts: vec![],
                terminator: Some(Terminator::Return(Some(ValueId(0)))),
            };
            for i in 1..=8 {
                block
                    .insts
                    .push((ValueId(i), Instruction::ConstInt(i as i64)));
            }
            // Keep them all live to the end so the intervals overlap and the
            // allocator has to choose.
            block
                .insts
                .push((ValueId(9), Instruction::Add(ValueId(1), ValueId(2))));
            block
                .insts
                .push((ValueId(10), Instruction::Add(ValueId(3), ValueId(4))));
            let mut blocks = HashMap::new();
            blocks.insert(BlockId(0), block);
            Function {
                name: SymbolId(0),
                blocks,
                entry_block: BlockId(0),
            }
        }

        let first = allocate_registers(&build(), 4);
        // A fresh `HashMap` in the same process iterates in a different order
        // than the first one, which is exactly the condition that used to
        // change the answer.
        for run in 2..=20 {
            let again = allocate_registers(&build(), 4);
            assert_eq!(
                again.num_spills, first.num_spills,
                "run {run} spilled a different number of values than run 1"
            );
            let mut a: Vec<_> = first.locations.iter().collect();
            let mut b: Vec<_> = again.locations.iter().collect();
            a.sort_by_key(|(id, _)| **id);
            b.sort_by_key(|(id, _)| **id);
            assert_eq!(a, b, "run {run} assigned different registers than run 1");
        }
    }
}
