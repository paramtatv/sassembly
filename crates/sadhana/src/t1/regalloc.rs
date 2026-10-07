use crate::t1::ir::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    Register(u8),
    Spill(usize),
}

/// `V-004` — WHICH REGISTER FILE A VALUE LIVES IN. RISC-V has two, and they
/// are allocated INDEPENDENTLY: `x1..x31` and `f0..f31` are separate numbers,
/// so the same `Location::Register(n)` names a different register in each.
/// The class is not carried inside `Location` on purpose — every emitter that
/// exists today (`riscv64.rs`, `x86_64.rs`) reads an integer map and must keep
/// compiling unchanged.
///
/// **A `Location::Register(n)` IS A ROLE INDEX AND NOT A HARDWARE NUMBER**, and
/// this comment said otherwise until `V-004` part 3 read the scan: `:0..num_registers`
/// below is what fills `free_registers`, and `num_registers` is the size of ONE
/// ROLE — twelve `स्थिर` for the integer file. So `Register(3)` is `स्थिर३`,
/// which is `s3`, which is `x19`; it was never `x3`. The float side is the same
/// shape one role over: `fs3` is `f19`. The translation is
/// `riscv64::class_register_name`, which is the only place that knows it.
///
/// The float register root is `प्लव` and not `भिन्न` (owner ruling Q1,
/// 2026-09-29); the names themselves belong to `V-005`, which is what will
/// supply a classifier that answers `Float`. **Nothing in the tree answers
/// `Float` yet** — the Rust T1 IR has no float instruction kind — so this
/// enum's second arm is reached only by a caller that passes its own
/// classifier, which is exactly how the test below exercises it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegClass {
    Int,
    Float,
}

pub struct AllocationMap {
    pub locations: HashMap<ValueId, Location>,
    pub num_spills: usize,
}

pub fn allocate_registers(func: &Function, num_registers: u8) -> AllocationMap {
    allocate_registers_for(func, num_registers, RegClass::Int, &|_, _| RegClass::Int)
}

/// `V-004` — THE LINEAR SCAN OVER **ONE** REGISTER FILE, so that two calls give
/// two files.
///
/// `class` names the file being filled and `classify` answers, for each value
/// the function defines, which file it belongs in. Values of any OTHER class
/// are not merely skipped at assignment time — they are left out of the
/// interval set entirely, so they neither hold a register nor consume a spill
/// slot in this map. That is the whole of the second-file requirement: run it
/// once per class and the two scans share nothing, which is why the spill
/// numbering in each returned map starts at zero and counts only its own.
///
/// `classify` takes the defining instruction AND the `ValueId`, because a
/// float-typed value is not always distinguishable from its opcode —
/// `Param(2)` and `Call(..)` define whatever the signature says they define,
/// and the signature is not in the `Instruction`.
///
/// `allocate_registers` is this function with a classifier that answers `Int`
/// for everything, which is also what the tree means today: `Instruction` has
/// no float kind, so the integer scan sees every value and the result is
/// byte-for-byte the old one. The two tests below that predate this split are
/// the evidence for that and were not touched.
pub fn allocate_registers_for(
    func: &Function,
    num_registers: u8,
    class: RegClass,
    classify: &dyn Fn(ValueId, &Instruction) -> RegClass,
) -> AllocationMap {
    let mut lifetimes: HashMap<ValueId, (usize, usize)> = HashMap::new();
    let mut linear_seq = Vec::new();
    let mut dummies: HashSet<usize> = HashSet::new();

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
            dummies.insert(linear_seq.len() - 1);
        }
    }
    // `V-005` — THE DUMMY TAKES ITS VALUE'S OWN FILE. The entry above is
    // `ConstInt(0)`, which classifies as an integer, so a returned FLOAT used to
    // be inserted into the INTEGER map at this position too — and
    // `Routine::location` asks the integer map first, so the float op that
    // defined it wrote an `x` register. Harmless while no float could be
    // returned; a float result in `fa0` made it live. So the dummy is classified
    // by the instruction that really defines the value (the same `classify`),
    // which leaves every integer routine's allocation exactly as it was.
    let real_class: HashMap<ValueId, RegClass> = linear_seq
        .iter()
        .enumerate()
        .filter(|(i, _)| !dummies.contains(i))
        .map(|(_, (v, inst))| (*v, classify(*v, inst)))
        .collect();

    // 2. Compute lifetimes [start, end] indices
    for (i, (def_val, inst)) in linear_seq.iter().enumerate() {
        // Definition — only if this value belongs to the file being filled.
        // An out-of-class value never enters `lifetimes`, so the `get_mut`
        // below misses it and it costs this scan neither a register nor a
        // spill slot.
        //
        // IT IS A `GUARD` AND NOT A `CONTINUE`, and the difference is a live
        // wrong answer. An out-of-class INSTRUCTION still USES in-class
        // operands — a float store's address, an index multiplied into a
        // float load — and skipping the loop below would end those operands'
        // intervals early, freeing a register a later instruction still reads.
        let def_class = if dummies.contains(&i) {
            real_class
                .get(def_val)
                .copied()
                .unwrap_or_else(|| classify(*def_val, inst))
        } else {
            classify(*def_val, inst)
        };
        if def_class == class {
            lifetimes.entry(*def_val).or_insert((i, i));
        }

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

    /// `V-004` — THE SECOND REGISTER FILE IS A SECOND SCAN, AND THE TWO SHARE
    /// NOTHING.
    ///
    /// Six values all live to the end of the block, so pressure is six in a
    /// single file. Three are classified `Float`. Run the scan twice, two
    /// registers per file:
    ///
    /// * each map holds EXACTLY its own three values — the refusal half, and
    ///   the one that fails if `allocate_registers_for` merely skipped
    ///   out-of-class values at assignment time instead of leaving them out
    ///   of the interval set;
    /// * each map spills exactly one, counting from `0` — so `f`-spill slot 0
    ///   and `x`-spill slot 0 are different slots and the numbering does not
    ///   run on from the other file;
    /// * both files hand out register numbers `0` and `1` — `Location` carries
    ///   no class, so the same number in the two maps is `x` in one and `f` in
    ///   the other.
    ///
    /// AND THE CONTROL: classifying everything `Int` must reproduce
    /// `allocate_registers` exactly, which is the claim that the split changed
    /// no behaviour for the tree as it stands today.
    #[test]
    fn v004_a_float_class_allocates_from_its_own_file_and_its_own_spill_slots() {
        fn build() -> Function {
            let mut block = Block {
                id: BlockId(0),
                insts: vec![],
                terminator: None,
            };
            for i in 1..=6 {
                block
                    .insts
                    .push((ValueId(i), Instruction::ConstInt(i as i64)));
            }
            // Keep all six live to the end: three pairwise adds reading them.
            block
                .insts
                .push((ValueId(7), Instruction::Add(ValueId(1), ValueId(2))));
            block
                .insts
                .push((ValueId(8), Instruction::Add(ValueId(3), ValueId(4))));
            block
                .insts
                .push((ValueId(9), Instruction::Add(ValueId(5), ValueId(6))));
            let mut blocks = HashMap::new();
            blocks.insert(BlockId(0), block);
            Function {
                name: SymbolId(0),
                blocks,
                entry_block: BlockId(0),
            }
        }

        // 2, 4 and 6 are the float values; 7, 8 and 9 are the adds and are
        // integer, so each file sees three long-lived values plus its share of
        // the short-lived adds.
        let floats = [ValueId(2), ValueId(4), ValueId(6)];
        let classify = |v: ValueId, _: &Instruction| {
            if floats.contains(&v) {
                RegClass::Float
            } else {
                RegClass::Int
            }
        };

        let func = build();
        let ints = allocate_registers_for(&func, 2, RegClass::Int, &classify);
        let fls = allocate_registers_for(&func, 2, RegClass::Float, &classify);

        // Disjoint, and complete — every value is in exactly one map.
        for v in 1..=9usize {
            let v = ValueId(v);
            let in_int = ints.locations.contains_key(&v);
            let in_flt = fls.locations.contains_key(&v);
            assert!(
                in_int ^ in_flt,
                "{v:?}: integer map {in_int}, float map {in_flt} — a value must \
                 be in exactly one register file"
            );
            assert_eq!(
                in_flt,
                floats.contains(&v),
                "{v:?} landed in the wrong file"
            );
        }

        // Each file numbers its own spill slots from zero.
        assert_eq!(
            fls.num_spills, 1,
            "float file: three live values, two registers"
        );
        assert!(
            ints.num_spills >= 1,
            "integer file spills on its own account, not the float file's"
        );
        assert!(
            fls.locations.values().any(|l| *l == Location::Spill(0)),
            "the float file's first spill is slot 0, not a slot after the integer file's"
        );

        // The same register NUMBER appears in both files and means a different
        // physical register in each.
        for file in [&ints, &fls] {
            let regs: Vec<u8> = file
                .locations
                .values()
                .filter_map(|l| match l {
                    Location::Register(r) => Some(*r),
                    Location::Spill(_) => None,
                })
                .collect();
            assert!(
                regs.contains(&0) && regs.contains(&1),
                "both files hand out 0 and 1; got {regs:?}"
            );
        }

        // THE CONTROL: all-integer must be the old allocator, exactly.
        let all_int = allocate_registers_for(&func, 2, RegClass::Int, &|_, _| RegClass::Int);
        let old = allocate_registers(&func, 2);
        let mut a: Vec<_> = all_int.locations.into_iter().collect();
        let mut b: Vec<_> = old.locations.into_iter().collect();
        a.sort_by_key(|(id, _)| *id);
        b.sort_by_key(|(id, _)| *id);
        assert_eq!(a, b, "the all-integer classifier is the old allocator");
        assert_eq!(all_int.num_spills, old.num_spills);
    }

    /// `V-004` — AN IN-CLASS VALUE READ BY AN OUT-OF-CLASS INSTRUCTION STAYS
    /// LIVE.
    ///
    /// This is the case a `continue` on the class guard gets wrong, and it is
    /// a silent wrong answer rather than a crash: `ValueId(1)` is integer and
    /// its only reader is `ValueId(9)`, a FLOAT value, so an integer scan that
    /// stopped at the class check would end `1`'s interval at its definition,
    /// free its register immediately, and hand it to `ValueId(3)` while `9`
    /// still reads it.
    ///
    /// **THE MUTATION IS THE ONLY EVIDENCE THIS TEST IS WORTH ANYTHING, AND IT
    /// FAILED THE FIRST TIME IT WAS RUN.** Written with three registers it
    /// stayed GREEN under the `continue` form, because four integer values in
    /// three registers never forces a reuse and the shortened interval costs
    /// nothing. At TWO the mutant recycles `ValueId(1)`'s register for
    /// `ValueId(3)` and this goes red, which is what was wanted.
    #[test]
    fn v004_an_out_of_class_reader_still_extends_an_in_class_interval() {
        let mut block = Block {
            id: BlockId(0),
            insts: vec![],
            terminator: None,
        };
        block.insts.push((ValueId(1), Instruction::ConstInt(1)));
        block.insts.push((ValueId(2), Instruction::ConstInt(2)));
        block.insts.push((ValueId(3), Instruction::ConstInt(3)));
        // The only reader of 1 is 9, and 9 is float.
        block
            .insts
            .push((ValueId(9), Instruction::Add(ValueId(1), ValueId(2))));
        block
            .insts
            .push((ValueId(4), Instruction::Add(ValueId(2), ValueId(3))));
        let mut blocks = HashMap::new();
        blocks.insert(BlockId(0), block);
        let func = Function {
            name: SymbolId(0),
            blocks,
            entry_block: BlockId(0),
        };

        let classify = |v: ValueId, _: &Instruction| {
            if v == ValueId(9) {
                RegClass::Float
            } else {
                RegClass::Int
            }
        };
        // TWO registers, not three: WITHOUT PRESSURE THERE IS NOTHING TO
        // RECYCLE and this test passes under the mutation it exists to catch.
        // Measured — at three registers the `continue` form is GREEN here.
        let ints = allocate_registers_for(&func, 2, RegClass::Int, &classify);

        // THE DISCRIMINATOR IS THE SPILL COUNT, NOT `r1 != r3`. Inequality
        // passes both ways here and was measured doing so: under the mutant
        // `ValueId(1)` dies at its definition, `ValueId(3)` takes the freed
        // register, and the two are still different locations. What the
        // shortened interval actually removes is the CONFLICT -- with
        // `ValueId(1)` live across `ValueId(3)`'s definition the two
        // registers are both held and the third value must go to a slot.
        assert_eq!(
            ints.num_spills, 1,
            "ValueId(1) is still read by the float add at index 3, so it is \
             live across ValueId(3)'s definition and two registers cannot \
             hold three live values; a scan that dropped the use because its \
             reader is out of class spills nothing. locations {:?}",
            ints.locations
        );
        assert!(
            !ints.locations.contains_key(&ValueId(9)),
            "the float value itself is not in the integer file"
        );
    }

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
