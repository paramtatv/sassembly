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
//! `W-306` THEN USED THE MEASUREMENT: a far conditional is no longer REFUSED,
//! it is RELAXED — inverted over a `लङ्घनम्` that carries the far target, which
//! reaches ±1 MiB where the conditional reaches ±4 KiB. So the tests below that
//! once named a refusal now name the relaxed text, and they check the thing the
//! measurement was for: that the branch which is far is the one that gets the
//! two extra words.
//!
//! THE CASES THAT MUST STILL BE REFUSED, and the reason the fix is not "always
//! relax": (1) a NEAR routine must come out byte for byte as it did — the whole
//! corpus is near, and a relaxation applied where it is not needed rewrites
//! every image; (2) the inversion must be the SAME-SIGNEDNESS one — `Lt → Ge`,
//! never `Lt → Geu` — which is the crossed table `W-306`'s first half refused in
//! the twin and which this file now refuses at the CALLER, where a wrong entry
//! would change what the program does rather than what a table says.

use sadhana::t1::ast::SymbolId;
use sadhana::t1::ir::CmpOp;
use sadhana::t1::ir::{Block, BlockId, Function, Instruction, Terminator, ValueId};
use sadhana::t1::riscv64::{self, Module, Names};

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

/// The text, with a SHORT message when there is a refusal instead. The routines
/// here are 1,200 lines — 106 KB of escaped Devanagari — so `expect` on either
/// side of the `Result` buries the one fact the reader needs in the other.
fn emitted(m: &Module) -> String {
    match riscv64::emit_module(m) {
        Ok(text) => text,
        Err(r) => panic!("refused a routine it must relax: {r}"),
    }
}

/// The routine's own lines, from its first label to its `निर्गम` — `emit_module`
/// prepends the startup stub and appends the data, and neither is being measured.
fn conditional_lines(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| {
            l.starts_with("विषमलङ्घनम् ")
                || l.starts_with("समलङ्घनम् ")
                || l.starts_with("न्यूनलङ्घनम् ")
                || l.starts_with("अन्यूनलङ्घनम् ")
                || l.starts_with("अचिह्नन्यूनलङ्घनम् ")
                || l.starts_with("अचिह्नान्यूनलङ्घनम् ")
        })
        .collect()
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
fn the_second_conditional_to_a_join_is_relaxed_and_the_first_is_not() {
    // 1,200 padding instructions — each one word — put `BlockId(3)`'s branch
    // more than 4,096 bytes AFTER `BlockId(1)`, while `BlockId(0)`'s branch to
    // the same block is a couple of words before it.
    //
    // BOTH CONDITIONALS COME OUT RELAXED, and that is the two-pass shape rather
    // than an imprecision: one out-of-range conditional re-emits the WHOLE
    // routine relaxed, because relaxing a subset moves the addresses the subset
    // was chosen by. What the measurement buys is that a routine with NO far
    // branch is never re-emitted at all — `two_near_branches_to_one_join_…`
    // below is that half.
    let text = emitted(&module(vec![two_branches_to_one_join(1200)]));
    let conds = conditional_lines(&text);
    assert_eq!(conds.len(), 2, "two conditionals, one per branching block");
    for (n, l) in conds.iter().enumerate() {
        assert!(
            l.starts_with("समलङ्घनम् "),
            "relaxed, conditional {n} is the INVERSE of `विषमलङ्घनम्`: {l}"
        );
        assert!(
            l.ends_with("अतिक्रमय् ।"),
            "relaxed, conditional {n} carries its own skip label: {l}"
        );
    }
    // Each block's skip label is its OWN — keyed by the branching block, not by
    // the join both of them branch to, which would define one label twice.
    assert!(
        text.contains("परीकार्यम्पर्व०अतिक्रमॱॱ"),
        "BlockId(0)'s skip label"
    );
    assert!(
        text.contains("परीकार्यम्पर्व३अतिक्रमॱॱ"),
        "BlockId(3)'s skip label"
    );
    // And the far target rides the J-type, twice — once per relaxed branch.
    assert_eq!(
        text.lines()
            .filter(|l| *l == "लङ्घनम् शून्यःम् परीकार्यम्पर्व१य् ।")
            .count(),
        2,
        "each relaxed conditional is followed by a jump to the join"
    );
}

#[test]
fn two_near_branches_to_one_join_are_not_relaxed() {
    // THE CASE THAT MUST NOT MOVE AN OCTET. The shape is identical — two
    // conditionals into one join — and only the distance differs. Every routine
    // in the corpus is this case, so a relaxation that fired on the SHAPE rather
    // than on the measurement would rewrite every image in it.
    let text = emitted(&module(vec![two_branches_to_one_join(8)]));
    assert!(
        !text.contains("अतिक्रम"),
        "eight words of padding is inside ±4 KiB for both branches; nothing relaxes"
    );
    for l in conditional_lines(&text) {
        assert!(
            l.starts_with("विषमलङ्घनम् ") && l.ends_with("पर्व१य् ।"),
            "near, the conditional carries the TARGET and is not inverted: {l}"
        );
    }
}

/// One conditional, its target beyond ±4 KiB, with `pad` words between them —
/// `BlockId(0)` branches forward over the padding to `BlockId(2)`.
fn one_far_conditional(pad: usize, cond: Option<CmpOp>) -> Function {
    let mut v = 0usize;
    let mut blocks = std::collections::HashMap::new();
    let mut insts = Vec::new();
    let c = match cond {
        // FUSED: the last instruction of the block is the `Cmp` the branch
        // reads, so `fusable_compare` folds it into the branch word — which is
        // the arm the inversion table is actually consulted on.
        Some(op) => {
            let a = ValueId(v);
            v += 1;
            let b = ValueId(v);
            v += 1;
            let c = ValueId(v);
            v += 1;
            insts.push((a, Instruction::ConstInt(1)));
            insts.push((b, Instruction::ConstInt(2)));
            insts.push((c, Instruction::Cmp(op, a, b)));
            c
        }
        None => {
            let c = ValueId(v);
            v += 1;
            insts.push((c, Instruction::ConstInt(1)));
            c
        }
    };
    blocks.insert(
        BlockId(0),
        Block {
            id: BlockId(0),
            insts,
            terminator: Some(Terminator::CondBranch(c, BlockId(2), BlockId(1))),
        },
    );
    blocks.insert(BlockId(1), filler(1, 2, pad, &mut v));
    blocks.insert(
        BlockId(2),
        Block {
            id: BlockId(2),
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
fn a_lone_far_conditional_is_relaxed_rather_than_refused() {
    // The guard's original case. It was a refusal until `W-306`; it is now two
    // extra words.
    let text = emitted(&module(vec![one_far_conditional(1200, None)]));
    let conds = conditional_lines(&text);
    assert_eq!(conds.len(), 1);
    assert!(conds[0].starts_with("समलङ्घनम् "), "{}", conds[0]);
    assert!(conds[0].ends_with("परीकार्यम्पर्व०अतिक्रमय् ।"), "{}", conds[0]);
    assert!(text.contains("लङ्घनम् शून्यःम् परीकार्यम्पर्व२य् ।"));
    assert!(text.contains("परीकार्यम्पर्व०अतिक्रमॱॱ"));
}

#[test]
fn the_relaxed_inversion_keeps_the_signedness_of_the_comparison() {
    // THE CROSSED TABLE, REFUSED AT THE CALLER. `W-306`'s first half proved the
    // twins agree on `inverse_condition`; this proves the EMITTER consults it
    // rather than pairing the six branch words in the order they are declared.
    //
    // `Lt`'s inverse is `Ge` — both sign-extend — and never `Geu`. A crossed
    // table inverts the comparison AND its signedness at once, so `-1 < 1`
    // signed and `0xffff…f >= 1` unsigned would take the SAME arm and the
    // relaxed routine would compute a different answer from the near one it
    // replaced. That is a wrong program, not a wrong table, which is why the
    // case is carried here as well as in `t1_twin_agreement`.
    let pairs = [
        (CmpOp::Lt, "अन्यूनलङ्घनम् ", "अचिह्नान्यूनलङ्घनम् "),
        (CmpOp::Ltu, "अचिह्नान्यूनलङ्घनम् ", "अन्यूनलङ्घनम् "),
        (CmpOp::Ge, "न्यूनलङ्घनम् ", "अचिह्नन्यूनलङ्घनम् "),
        (CmpOp::Geu, "अचिह्नन्यूनलङ्घनम् ", "न्यूनलङ्घनम् "),
    ];
    for (op, want, crossed) in pairs {
        let text = emitted(&module(vec![one_far_conditional(1200, Some(op))]));
        let conds = conditional_lines(&text);
        assert_eq!(conds.len(), 1, "{op:?}: one conditional");
        assert!(
            conds[0].starts_with(want),
            "{op:?} relaxes to {want}, not to {crossed}: {}",
            conds[0]
        );
        assert!(
            !conds[0].starts_with(crossed),
            "{op:?} took the CROSSED inverse {crossed}: {}",
            conds[0]
        );
        assert!(conds[0].ends_with("परीकार्यम्पर्व०अतिक्रमय् ।"), "{}", conds[0]);
    }
}

#[test]
fn a_near_fused_conditional_is_not_inverted() {
    // The same four comparisons at eight words: the branch word is the
    // comparison ITSELF, and no `अतिक्रम` exists. Without this the test above
    // would pass on an emitter that inverted unconditionally.
    for (op, near) in [
        (CmpOp::Lt, "न्यूनलङ्घनम् "),
        (CmpOp::Ltu, "अचिह्नन्यूनलङ्घनम् "),
        (CmpOp::Ge, "अन्यूनलङ्घनम् "),
        (CmpOp::Geu, "अचिह्नान्यूनलङ्घनम् "),
    ] {
        let text = emitted(&module(vec![one_far_conditional(8, Some(op))]));
        assert!(!text.contains("अतिक्रम"), "{op:?}: near, nothing relaxes");
        let conds = conditional_lines(&text);
        assert_eq!(conds.len(), 1, "{op:?}");
        assert!(conds[0].starts_with(near), "{op:?}: {}", conds[0]);
    }
}

#[test]
fn a_backward_conditional_past_the_limit_is_relaxed_by_its_own_distance() {
    // A BACK-EDGE — the `यावत्` shape. The padding sits between the loop head
    // and the branch that jumps back to it, so the branch is the LAST block and
    // its target is the FIRST; the distance is negative and past −4 KiB. It is
    // here because the walk that attributes a branch to its block must still do
    // so in a routine whose ENTRY is labelled by a back-edge, and because the
    // relaxed jump is the backward one.
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
    let text = emitted(&module(vec![func]));
    let conds = conditional_lines(&text);
    assert_eq!(conds.len(), 1);
    assert!(conds[0].starts_with("समलङ्घनम् "), "{}", conds[0]);
    assert!(conds[0].ends_with("परीकार्यम्पर्व१अतिक्रमय् ।"), "{}", conds[0]);
    assert!(
        text.contains("लङ्घनम् शून्यःम् परीकार्यम्पर्व०य् ।"),
        "the far target is the loop head and it rides the J-type"
    );
}

// --- `W-332`: the relaxation census -------------------------------------------

/// The module of [`module`] with a SECOND routine, so that a census can say
/// which of two routines relaxed. `SymbolId(1)` is `परीकार्यम्` — the one every
/// test above names — and `SymbolId(2)` is `परीअन्यत्`.
fn module_of_two(first: Function, mut second: Function) -> Module {
    second.name = SymbolId(2);
    let mut names = Names::new();
    names.insert(SymbolId(1), ("परी".to_string(), "कार्यम्".to_string()));
    names.insert(SymbolId(2), ("परी".to_string(), "अन्यत्".to_string()));
    Module {
        globals: Vec::new(),
        name: "परी".to_string(),
        functions: vec![first, second],
        names,
        entry: None,
    }
}

/// The census, or a SHORT message when the emitter refused instead — same
/// reason [`emitted`] does it: these routines are 100 KB of Devanagari and an
/// `expect` on the wrong arm buries the fact the reader came for.
fn relaxations(m: &Module) -> Vec<String> {
    match riscv64::emit_module_and_relaxations(m) {
        Ok((_, relaxed)) => relaxed,
        Err(r) => panic!("refused a routine it must relax: {r}"),
    }
}

#[test]
fn a_module_with_no_far_conditional_relaxes_nothing_and_the_census_says_so() {
    // THE READING THE WHOLE CORPUS IS EXPECTED TO GIVE. Before `W-332` the only
    // evidence for it was the ABSENCE of `अतिक्रम` in the text — a grep over
    // emitted Devanagari, which answers "no skip label is written" and not "no
    // routine was re-emitted". The two differ the moment a future relaxation
    // spells its label differently.
    let census = relaxations(&module(vec![two_branches_to_one_join(8)]));
    assert!(
        census.is_empty(),
        "eight words of padding is inside ±4 KiB for both branches, so no \
         routine is re-emitted — the census names {census:?}"
    );
    println!("METRIC riscv_relaxed_routines {}", census.len());
}

#[test]
fn a_relaxed_routine_is_counted_once_however_many_of_its_branches_were_far() {
    // `two_branches_to_one_join(1200)` puts TWO conditionals out of range and
    // the test above pins that BOTH come out inverted. The census must still
    // read 1: the unit that relaxes is the ROUTINE — one far branch re-emits
    // the whole of it — so a count of 2 here would be a count of branches
    // wearing the name of a count of relaxations, and it would read as a
    // regression the first time a routine grew a third far branch.
    let census = relaxations(&module(vec![two_branches_to_one_join(1200)]));
    assert_eq!(
        census,
        vec!["परीकार्यम्".to_string()],
        "one routine relaxed, named once, however many of its branches were far"
    );
    println!("METRIC riscv_relaxed_routines {}", census.len());
}

#[test]
fn the_census_names_the_routine_that_relaxed_and_not_its_near_neighbour() {
    // THE CASE THE COUNT EXISTS FOR, and the one a byte-diff cannot answer.
    // Two routines in one module, identical in shape and differing only in the
    // distance between a conditional and its target. The far one is re-emitted
    // and the near one is not — and relaxing the far one ADDS TEXT, which moves
    // every address downstream of it, so the near routine's octets move even
    // though nothing about it was re-emitted. That is exactly why "the image
    // changed" is not attribution and why this reading is by NAME.
    let census = relaxations(&module_of_two(
        one_far_conditional(1200, None),
        one_far_conditional(8, None),
    ));
    assert_eq!(
        census,
        vec!["परीकार्यम्".to_string()],
        "the far routine relaxed; `परीअन्यत्` is near and must not appear"
    );

    // And the other way round, so the census is not simply reporting the FIRST
    // routine of every module it is handed.
    let census = relaxations(&module_of_two(
        one_far_conditional(8, None),
        one_far_conditional(1200, None),
    ));
    assert_eq!(
        census,
        vec!["परीअन्यत्".to_string()],
        "the far routine is the SECOND one here, and the census follows the \
         measurement rather than the emission order"
    );
    println!("METRIC riscv_relaxed_routines {}", census.len());
}

#[test]
fn emit_module_returns_the_same_text_as_the_census_entry_point() {
    // `emit_module` is now a wrapper that drops the census, and 53 callers read
    // it. This pins that the wrapper is a projection and not a second emitter —
    // if the two ever diverge, every test in the tree is measuring a different
    // emitter from the one this row's count is about.
    for pad in [8usize, 1200] {
        let m = module(vec![two_branches_to_one_join(pad)]);
        let (censused, _) = riscv64::emit_module_and_relaxations(&m).expect("emits");
        assert_eq!(
            riscv64::emit_module(&m).expect("emits"),
            censused,
            "pad {pad}: the wrapper drops the census and nothing else"
        );
    }
}
