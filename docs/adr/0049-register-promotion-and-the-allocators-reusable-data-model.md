# ADR-0049: Register promotion, and the allocator's reusable data model

Status: accepted for RV64 (W-regalloc); x86-64 and ARM64 are extension instances, not built.

## Decision
The T1 compiler promotes a routine's hottest frame-slot locals to callee-saved registers and
puts integer temps that cross no call in the argument registers. It is ON by default in the
Rust twin (`regalloc.rs`, `riscv64.rs`) and in `utsarjana.t1` / `yantrotsarjana.t1`, which emit
identical text (checked by `measure_corpus_twin_emit`, 18/18).

## Algorithm (identical in both twins)
1. Loop depth per block from back edges (target id <= own id). Weight of a slot = sum of
   10^min(depth,4) over its Loads and Stores. Slots a float touches are never promoted.
2. Rank by weight desc, slot asc; the first `promote_slots` with weight >= `min_weight`; the chosen
   slots, in slot order, take the top callee-saved registers.
3. A Load whose uses are all in its block with no Store of the slot between it and its last use IS the
   register. A Store whose value has no other use, defined by a single-register op, with no read of the
   slot in between, is written by that op directly.
4. An integer value defined after the last Param, with no Call in (definition, last use] (a terminator
   read extends the last use to the block end), takes the lowest caller-saved register whose previous
   holder ended before its definition.
5. The temp scan then runs over the remaining callee-saved registers. A routine whose promoted frame
   exceeds the 2047-octet offset limit is allocated again without promotion.

## The interface a second target plugs into
`RegFile` in `regalloc.rs` is READ by the Rust planner (`RV64_REGFILE`; `plan_promotion`,
`allocate_with_promotion` and the call-free-temp rule take it as a parameter, and `riscv64.rs` derives
`ALLOCATABLE` and the argument-register band from it). WHAT IS PLUGGABLE NOW, exactly:
- `callee_saved` (allocatable callee-saved count, numbered 0..n), `caller_saved` (allocatable
  caller-saved count, numbered n..n+m: RV64 spells them a0..a7), `promote_slots`, `min_weight`;
- the result: `Promotion::slot_reg` / `abstract_register(slot)` maps a promoted slot to an abstract
  allocator number, and `allocate_with_promotion` returns a normal `AllocationMap` whose
  `Location::Register(r)` uses the same numbers (r >= callee_saved means caller-saved). A backend only
  has to map a number to a register name and to save the callee-saved ones it uses.
NOT yet pluggable: the `.t1` twin keeps these four numbers as globals/constants in `utsarjana.t1`
(`प्रवर्धनस्थानाधिकम्`, `प्रवर्धनन्यूनभारः`, the 12 and 8 in the routines), so a second `.t1` target
means passing them in; reserved scratch is not a table row (RV64 t0..t6 are simply never numbered).
The IR side is target-free: positions, per-value use counts and first/last use, block ranges, call
positions, last Param position.

## Which values get an argument (caller-saved) register
Only values that are defined after the last Param, live entirely inside their defining block (no use
in another block, none before the definition, a terminator read only by the same block), and have no
Call in (definition, last use]. A value that crosses blocks is never given one: intervals are linear in
block-id order and are NOT extended over back edges, so a latch temp could take the register a
pre-loop value still needs. Tests: `a_value_used_outside_its_defining_block_is_not_given_an_argument_register`
and `crates/sadhana-t1/tests/regalloc_twin.rs` (twin byte-equality plus a mutated-ranking check).

## Extension points (deliberately unbuilt; RV64 needs none, so the image does not grow)
- fixed (precoloured) operands and per-op clobber sets: today the only clobber is "a Call clobbers every
  caller-saved register", expressed as the call-position prefix count in step 4. x86 idiv (rax/rdx) and
  shift counts (rcx) become more clobber positions plus a precolour on the operand's interval.
- tied operand (x86 dst = src1 when src1 dies): a preference when choosing the register in step 4/5, off
  for RV64 and ARM64.
- internal ABI: argument and return registers are the caller-saved numbers 0..7 in order; a target lists
  its own and the Param/Call lowering precolours them.
- spill weight by loop depth exists (step 1); rematerialising a constant instead of spilling is not built:
  RV64 constants already rematerialise (`li`), so it only matters for targets with costly immediates.

## Evidence
Kernel instruction counts N=6 (vs unpromoted): sieve -18.6%, matmul -13.8%, heapsort -10.6%, flac -11.3%;
compiler corpus Stage 2: -11.9% (55.385G -> 48.80G). Fixpoint results are recorded in the landing note.
