//! `W-235` — the emitted T1 IR RUNS on the machine (research/25 §5 R1's acceptance).
//!
//! `crates/sadhana/src/t1/riscv64.rs` writes T0 text; its own unit tests prove
//! every line assembles and the fixture links and decodes back. What they cannot
//! prove is that the program computes: `sadhana` cannot run `yantra`, because
//! `yantra` depends on `sadhana`. So the run is here, through the same four calls
//! `sadhana`'s `main` makes and `yantra-run`'s two numbers (1 MiB, a million
//! steps), and the status the finisher reports is the assertion.
//!
//! The fixture is `f(n) = if n then f(n−1) + n else 0`, `f(5)`: a `Param`, a
//! `CondBranch`, a `Sub`, a recursive `Call` — so a FRAME that actually saves and
//! restores — an `Add` and a `Return`. Its status is 15 or the emitter is wrong
//! somewhere the assembler cannot see.

use sadhana::encode::Target;
use sadhana::nidana::Language;
use sadhana::t1::ast::SymbolId;
use sadhana::t1::ir::{Block, BlockId, Function, Instruction, Terminator, ValueId};
use sadhana::t1::riscv64::{
    Module, Names, emit_module, emit_startup_object, fixture_recursive_sum, routine_label,
};
use sadhana::{assemble_object, link_objects, vastu};
use std::collections::HashMap;
use yantra::{Halt, Machine};

/// The limits `yantra-run` gives a program, IMPORTED rather than restated —
/// `W-262` made `yantra::DEFAULT_RAM` and `DEFAULT_STEPS` the one statement,
/// after five copies of these numbers drifted behind a comment claiming they
/// matched. A test needing different limits should say so and why.
const RAM: usize = yantra::DEFAULT_RAM;
const BUDGET: u64 = yantra::DEFAULT_STEPS;
/// Where every `spec/*.sas` boot proof is linked, and `e_entry` is the first instruction.
const LOAD: u64 = 0x8000_0000;

/// Emit, assemble, link, load, run — naming the refused line if the assembler refuses.
fn run(module: &Module) -> (String, Halt, Vec<u8>) {
    let text = emit_module(module).unwrap_or_else(|r| panic!("the emitter refused: {r}"));
    let bytes = assemble_object(
        &text,
        Some("परीक्षा"),
        Target::Uncompressed,
        false,
        Language::English,
    )
    .unwrap_or_else(|ds| {
        let named: Vec<String> = ds
            .iter()
            .map(|d| {
                let line = text.lines().nth(d.line.saturating_sub(1)).unwrap_or("");
                format!("line {}: `{line}` — {}", d.line, d.reason)
            })
            .collect();
        panic!(
            "सङ्केतन refused the emitted text:\n{}\n\n{text}",
            named.join("\n")
        )
    });
    let obj = vastu::read(&bytes).expect("the object reads back");
    // `W-243`: the image is one startup object (the stub and the stack, with the
    // entry's exported label) linked first, then the module's object.
    let entry = module
        .entry
        .map(|e| routine_label(&module.names, e).expect("the entry has a label"));
    let startup = assemble_object(
        &emit_startup_object(entry.as_deref()),
        Some("यन्त्रारम्भ"),
        Target::Uncompressed,
        false,
        Language::English,
    )
    .expect("the startup object assembles");
    let startup = vastu::read(&startup).expect("the startup object reads back");
    let elf =
        link_objects(&[startup, obj], LOAD).unwrap_or_else(|e| panic!("link: {}", e.join("; ")));
    let mut m = Machine::load_elf(&elf, RAM).expect("loads");
    let mut out = Vec::new();
    let halt = m.run(BUDGET, &mut out);
    (text, halt, out)
}

fn status(halt: &Halt) -> u32 {
    match halt {
        Halt::Finisher {
            status: Some(s), ..
        } => *s,
        other => panic!("the program did not reach the finisher with a status: {other:?}"),
    }
}

fn leaf(name: SymbolId, insts: Vec<(ValueId, Instruction)>, ret: ValueId) -> Function {
    let mut blocks = HashMap::new();
    blocks.insert(
        BlockId(0),
        Block {
            id: BlockId(0),
            insts,
            terminator: Some(Terminator::Return(Some(ret))),
        },
    );
    Function {
        name,
        blocks,
        entry_block: BlockId(0),
    }
}

fn module(functions: Vec<(SymbolId, &str, Function)>, entry: SymbolId) -> Module {
    let mut names = Names::new();
    for (sym, name, _) in &functions {
        names.insert(*sym, ("परीक्षा".into(), (*name).into()));
    }
    Module {
        globals: Vec::new(),
        name: "परीक्षा".into(),
        functions: functions.into_iter().map(|(_, _, f)| f).collect(),
        names,
        entry: Some(entry),
    }
}

/// THE ACCEPTANCE: `f(5)` → 15, through a frame that saved and restored `ra` and
/// two `स्थिर`s five levels deep.
#[test]
fn the_recursive_fixture_runs_on_yantra_to_status_15() {
    let (text, halt, out) = run(&fixture_recursive_sum());
    assert!(
        out.is_empty(),
        "the program writes nothing but the finisher"
    );
    assert_eq!(
        status(&halt),
        15,
        "f(5) = 5+4+3+2+1+0; halt was {halt:?}\n{text}"
    );
    println!("METRIC riscv64_fixture_status {}", status(&halt));
    println!("METRIC riscv64_fixture_lines {}", text.lines().count());
}

/// The three constant forms reach the machine with the right VALUE, which the
/// assembler cannot check: `addi`, `lui`+`addi` with the +1 carry (4095 = 1<<12 −
/// 1), a negative `lui` pair, and two 40-bit constants from the pool whose
/// difference is what survives the 16-bit status. (4095 − 4000) + (−5 + 7) +
/// ((1<<40) + 42 − (1<<40)) = 95 + 2 + 42 = 139.
#[test]
fn constants_of_every_width_reach_the_machine_with_their_value() {
    let main = SymbolId(1);
    let v = |n| ValueId(n);
    let f = leaf(
        main,
        vec![
            (v(0), Instruction::ConstInt(4095)),
            (v(1), Instruction::ConstInt(4000)),
            (v(2), Instruction::Sub(v(0), v(1))),
            (v(3), Instruction::ConstInt(-5)),
            (v(4), Instruction::ConstInt(7)),
            (v(5), Instruction::Add(v(3), v(4))),
            (v(6), Instruction::ConstInt((1 << 40) + 42)),
            (v(7), Instruction::ConstInt(1 << 40)),
            (v(8), Instruction::Sub(v(6), v(7))),
            (v(9), Instruction::Add(v(2), v(5))),
            (v(10), Instruction::Add(v(9), v(8))),
            (v(11), Instruction::ConstInt(-300_000)),
            (v(12), Instruction::ConstInt(300_000)),
            (v(13), Instruction::Add(v(11), v(12))),
            (v(14), Instruction::Add(v(10), v(13))),
        ],
        v(14),
    );
    let (text, halt, _) = run(&module(vec![(main, "मुख्य", f)], main));
    assert!(
        text.contains("ध्रुवकोशःॱॱ\nध्रुव०ॱॱ\n॥ अष्टाष्टकाः ०षोड्१००००००००२अ ॥"),
        "{text}"
    );
    assert_eq!(status(&halt), 139, "{halt:?}\n{text}");
}

/// The ninth argument travels on the stack (§2.5) and the callee reads it from
/// above its own frame: `nine(1..9)` returns `a8 + a0` = 9 + 1 = 10.
#[test]
fn a_ninth_argument_travels_on_the_stack_and_is_read_above_the_callee_frame() {
    let (nine, main) = (SymbolId(1), SymbolId(2));
    let mut insts: Vec<(ValueId, Instruction)> = (0..9)
        .map(|i| (ValueId(i), Instruction::Param(i)))
        .collect();
    insts.push((ValueId(9), Instruction::Add(ValueId(8), ValueId(0))));
    let f_nine = leaf(nine, insts, ValueId(9));
    let args: Vec<ValueId> = (0..9).map(ValueId).collect();
    let mut insts: Vec<(ValueId, Instruction)> = args
        .iter()
        .enumerate()
        .map(|(i, v)| (*v, Instruction::ConstInt(i as i64 + 1)))
        .collect();
    insts.push((ValueId(9), Instruction::Call(nine, args)));
    let f_main = leaf(main, insts, ValueId(9));
    let (text, halt, _) = run(&module(
        vec![(nine, "नवार्थम्", f_nine), (main, "मुख्य", f_main)],
        main,
    ));
    assert_eq!(status(&halt), 10, "{halt:?}\n{text}");
}

/// Spilled values (twenty live against twelve `स्थिर`s) come back from the frame
/// with their value: 1 + 2 + … + 20 = 210.
#[test]
fn spilled_values_come_back_from_the_frame() {
    let main = SymbolId(1);
    let n = 20;
    let mut insts: Vec<(ValueId, Instruction)> = (0..n)
        .map(|i| (ValueId(i), Instruction::ConstInt(i as i64 + 1)))
        .collect();
    let mut acc = ValueId(0);
    for i in 1..n {
        let v = ValueId(n + i);
        insts.push((v, Instruction::Add(acc, ValueId(i))));
        acc = v;
    }
    let f = leaf(main, insts, acc);
    let (text, halt, _) = run(&module(vec![(main, "मुख्य", f)], main));
    assert!(text.contains("आहारः क्षणिक"), "something spilled:\n{text}");
    assert_eq!(status(&halt), 210, "{halt:?}\n{text}");
}

/// A zero result is `0x5555`, status 0 — and an entryless module is the same stub
/// with no call (§2.6).
#[test]
fn a_zero_result_and_an_entryless_module_both_halt_with_status_0() {
    let main = SymbolId(1);
    let f = leaf(
        main,
        vec![(ValueId(0), Instruction::ConstInt(0))],
        ValueId(0),
    );
    let (_, halt, _) = run(&module(vec![(main, "मुख्य", f)], main));
    assert_eq!(
        halt,
        Halt::Finisher {
            value: 0x5555,
            status: Some(0)
        }
    );

    let mut empty = module(vec![], main);
    empty.entry = None;
    let (_, halt, _) = run(&empty);
    assert_eq!(
        halt,
        Halt::Finisher {
            value: 0x5555,
            status: Some(0)
        }
    );
}

/// A loop through a back-edge to a labelled block: count down from 6 by a
/// `CondBranch` whose else-target falls through. `Branch` back to the condition
/// block is the म-loop of research/22 §7 on the machine.
#[test]
fn a_back_edge_loop_runs_to_its_exit() {
    // entry(0): n = 6; one = 1; Branch(1)           -- falls through to cond
    // cond(1):  CondBranch(n, body(3), exit(2))     -- the else is NEXT, so its
    // exit(2):  z = 0; Return z                     --   jump is omitted (§2.7)
    // body(3):  m = n - one; Return m               -- n is not updated (no phi),
    // The value is 5: the body is entered once and  --   so the body returns once.
    // returns. What this measures is the block order, the two fallthroughs,
    // and a conditional whose then-target is not the next block.
    let main = SymbolId(1);
    let v = |n| ValueId(n);
    let mut blocks = HashMap::new();
    blocks.insert(
        BlockId(0),
        Block {
            id: BlockId(0),
            insts: vec![
                (v(0), Instruction::ConstInt(6)),
                (v(1), Instruction::ConstInt(1)),
            ],
            terminator: Some(Terminator::Branch(BlockId(1))),
        },
    );
    blocks.insert(
        BlockId(1),
        Block {
            id: BlockId(1),
            insts: vec![],
            terminator: Some(Terminator::CondBranch(v(0), BlockId(3), BlockId(2))),
        },
    );
    blocks.insert(
        BlockId(2),
        Block {
            id: BlockId(2),
            insts: vec![(v(3), Instruction::ConstInt(0))],
            terminator: Some(Terminator::Return(Some(v(3)))),
        },
    );
    blocks.insert(
        BlockId(3),
        Block {
            id: BlockId(3),
            insts: vec![(v(2), Instruction::Sub(v(0), v(1)))],
            terminator: Some(Terminator::Return(Some(v(2)))),
        },
    );
    let f = Function {
        name: main,
        blocks,
        entry_block: BlockId(0),
    };
    let (text, halt, _) = run(&module(vec![(main, "मुख्य", f)], main));
    assert!(
        !text.contains("लङ्घनम् शून्यःम् परीक्षामुख्यपर्व१य् ।"),
        "Branch(1) falls through:\n{text}"
    );
    assert!(
        !text.contains("लङ्घनम् शून्यःम् परीक्षामुख्यपर्व२य् ।"),
        "the else falls through:\n{text}"
    );
    assert!(
        text.contains("विषमलङ्घनम् स्थिर०न शून्यःत् परीक्षामुख्यपर्व३य् ।"),
        "the then is a bne:\n{text}"
    );
    assert_eq!(status(&halt), 5, "{halt:?}\n{text}");
}

/// **`W-254`'s FIRST UNIT, RE-SUBJECTED 2026-09-17.** A string literal's octets
/// reach the image, the value a program gets is their ADDRESS, and that address
/// is preceded by the literal's HEADER.
///
/// THE OLD SUBJECT WAS RETIRED BY TWO LANDINGS AND NEITHER RE-TOOK THE PIN.
/// Written at `dfb5b39b` (2026-09-05), when the pool packed blobs end to end, so
/// `addr(second) − addr(first)` WAS the first literal's length and the assertion
/// was 3. On 2026-09-13 `f8dd87d7` (`W-len`) put a length word at
/// `पाठक वियोगः ८` and `ee0d9996` a capacity word below that, each behind a
/// `संरेखः ८` — in `riscv64.rs:1444-1484` AND in the `.t1` twin
/// (`yantrotsarjana.t1:2371-2384`), both with margins stating why. The gap became
/// `align8(len) + 16` = 24, and this test has read 24-against-3 for four days.
/// **The emitter is right; the number was measuring a layout that no longer
/// exists.** Dated before it was touched: the two commits above, not an argument.
///
/// SO THE SUBJECT IS REPLACED RATHER THAN THE NUMBER RE-PINNED. `assert_eq!(gap,
/// 24)` has TWO states where the truth has FOUR, and every wrong layout reports
/// as *"expected 24"*. TWO lengths are run instead — one alignment must round up
/// (9 → 16) and one it must not (3 → 8) — and the PAIR says which half moved.
/// **EVERY ROW BELOW WAS MEASURED BY MUTATING `riscv64.rs`'s pool loop and
/// reading the two numbers back, not derived on paper:**
///
///     (24, 32)   correct                                   (today, asserted)
///     (16, 24)   the `अष्टाष्टकाः ०` capacity write deleted
///     (19, 25)   the `संरेखः ८` deleted — `दैर्घ्य`'s load would then fault
///     ( 3,  9)   all three deleted: the pool packs end to end again,
///                which is the 2026-09-05 layout the old pin's 3 came from
///
/// **AND THE SECOND ROW IS WHY THERE ARE TWO LENGTHS AND NOT ONE.** With the
/// capacity word deleted the NINE-octet case answers **24 — the exact number the
/// three-octet case asserts as CORRECT.** A one-length instrument would have
/// gone green on a pool missing a header word. That is not a worry about what
/// could happen; it is what the mutation printed.
///
/// THE MARGIN THIS ONE REPLACES SAID THE DEREFERENCE WAS IMPOSSIBLE: *"the IR
/// has no instruction that dereferences one … a memory load is the next unit's
/// business."* That unit landed — `W-283`'s `LoadAt` — so the two tests below
/// READ the header back instead of inferring it from a gap, which is what that
/// margin asked for and could not have.
#[test]
fn two_string_literals_are_laid_out_with_their_two_header_words_between_them() {
    let main = SymbolId(1);
    let (a, b, d) = (ValueId(0), ValueId(1), ValueId(2));
    let f = leaf(
        main,
        vec![
            (a, Instruction::ConstStr(b"\xe0\xa4\xae".to_vec())),
            (b, Instruction::ConstStr(b"\xe0\xa4\xa8".to_vec())),
            (d, Instruction::Sub(b, a)),
        ],
        d,
    );
    let (text, halt, out) = run(&module(vec![(main, "मुख्य", f)], main));
    assert!(
        out.is_empty(),
        "the program writes nothing but the finisher"
    );

    // The octets are the ones asked for: म is U+092E, UTF-8 e0 a4 ae = 224 164 174.
    assert!(
        text.contains("॥ अष्टकाः २२४ १६४ १७४ ॥"),
        "the first literal's octets are in the data section:\n{text}"
    );
    assert!(
        text.contains("॥ अष्टकाः २२४ १६४ १६८ ॥"),
        "the second literal's octets are too (न is e0 a4 a8):\n{text}"
    );
    // The address is materialised, not the octets: an addi completing the auipc,
    // where a pooled INTEGER would have a load.
    assert!(
        text.contains("स्थानसापेक्षयोगः क्षणिक०म् पाठ०ॱउपरिन ।"),
        "the pc-relative upper half names the pooled label:\n{text}"
    );
    assert!(
        text.contains("न पाठ०ॱअधःन ।"),
        "and the lower half is an addi, so the value is the address:\n{text}"
    );
    // PER LINE, because the routine is full of loads that have nothing to do
    // with this: the epilogue restores `पुनःस्थानम्` and the saved `स्थिर`s with
    // `आहारः`. Asking whether the TEXT contains a load answers a different
    // question from asking whether the instruction completing this `auipc` is
    // one, and the first version of this assertion asked the wrong one.
    assert!(
        !text
            .lines()
            .any(|l| l.contains("आहारः") && l.contains("पाठ०ॱअधः")),
        "a load completing the auipc would give the first eight octets, not the address:\n{text}"
    );
    // AND THE HEADER IS IN THE TEXT IN THE ORDER THE GAP CLAIMS, which is the
    // half a run cannot show: capacity, then the `शीर्ष` label, then the length,
    // then the blob's label. A pair of words in the other order gives the same 24.
    let ordered = "॥ संरेखः ८ ॥\n॥ अष्टाष्टकाः ० ॥\nपाठ०शीर्षॱॱ\n॥ अष्टाष्टकाः ३ ॥\nपाठ०ॱॱ\n";
    assert!(
        text.contains(ordered),
        "capacity ०, then `पाठ०शीर्ष`, then the length ३, then `पाठ०`:\n{text}"
    );

    assert_eq!(
        status(&halt),
        24,
        "align8(3) + the capacity and length words; halt was {halt:?}\n{text}"
    );
    println!("METRIC riscv64_string_literal_gap {}", status(&halt));

    // THE SECOND LENGTH, WHICH IS THE WHOLE REASON THERE ARE TWO. Nine octets
    // round up to sixteen, so a lost `संरेखः ८` answers 25 here and 19 there
    // while a lost header word answers 24 here — the same 24 the case above
    // asserts as CORRECT. Neither case alone can tell those two apart.
    let (a, b, d) = (ValueId(0), ValueId(1), ValueId(2));
    let f = leaf(
        main,
        vec![
            (a, Instruction::ConstStr(b"123456789".to_vec())),
            (b, Instruction::ConstStr(b"\xe0\xa4\xa8".to_vec())),
            (d, Instruction::Sub(b, a)),
        ],
        d,
    );
    let (text, halt, _) = run(&module(vec![(main, "मुख्य", f)], main));
    assert_eq!(
        status(&halt),
        32,
        "align8(9) is 16, so the gap is 32; halt was {halt:?}\n{text}"
    );
    println!(
        "METRIC riscv64_string_literal_gap_unaligned {}",
        status(&halt)
    );
}

/// **THE LENGTH HEADER READ BACK, WHICH THE GAP CAN ONLY INFER.** `W-283`'s
/// `LoadAt` dereferences an address held in a value, so the program computes
/// `addr − 8` and returns the word it finds there. **Nothing in the IR states
/// that number** — the only `ConstInt` in the routine is the 8 — so it comes
/// back only if `W-len`'s header is really at `पाठक वियोगः ८` and really holds
/// the literal's TRUE length rather than the ० a capacity carries.
///
/// TWO LENGTHS, BECAUSE ONE WOULD BE A COINCIDENCE. A header stuck at a constant,
/// or the capacity word read by mistake, answers the same thing for both; 3 and 9
/// can only both come back from the literal's own count.
///
/// FALSIFIED BY MUTATION: the emitter's `devanagari(b.len())` forced to
/// `devanagari(0)` — which is the ० that `W-len`'s own margin says it deliberately
/// is NOT — reds this at `left: 0, right: 3`, and leaves the capacity test below
/// GREEN. That separation is the point of there being two of them.
#[test]
fn a_literals_length_header_is_the_word_below_its_address() {
    for (octets, len) in [
        (b"\xe0\xa4\xae".to_vec(), 3u32),
        (b"123456789".to_vec(), 9u32),
    ] {
        let main = SymbolId(1);
        let (s, eight, p, l) = (ValueId(0), ValueId(1), ValueId(2), ValueId(3));
        let f = leaf(
            main,
            vec![
                (s, Instruction::ConstStr(octets.clone())),
                (eight, Instruction::ConstInt(8)),
                (p, Instruction::Sub(s, eight)),
                (l, Instruction::LoadAt(p)),
            ],
            l,
        );
        let (text, halt, _) = run(&module(vec![(main, "मुख्य", f)], main));
        assert_eq!(
            status(&halt),
            len,
            "the word at `पाठ० वियोगः ८` is the literal's own length, not ० and not a constant; \
             halt was {halt:?}\n{text}"
        );
    }
    println!("METRIC riscv64_string_length_header_read 2");
}

/// **AND THE CAPACITY WORD BELOW THAT, WITH A NEIGHBOUR CHOSEN SO ITS ABSENCE IS
/// LOUD.** `ee0d9996` writes ० there deliberately — *"a literal's first indexed
/// store copies it"* — and ० is the worst possible thing to assert, because
/// uninitialised padding reads ० too and a load from the wrong address usually
/// does as well. So the literal BEFORE the probe is eight octets whose
/// little-endian word is 7: with the capacity word present, `पाठ१ वियोगः १६` is
/// that ०; with it absent the same address lands in the NEIGHBOUR's blob and
/// answers 7. The assertion separates the header from an accident.
///
/// THE FIRST LITERAL IS NEVER USED AS A VALUE and it must still be pooled —
/// `emit_module` runs no DCE (`opt.rs` is not on this path), and the text
/// assertion on `पाठ१ॱॱ` is what would catch the day it does, because a dropped
/// entry would renumber the probe and read past the pool's start.
#[test]
fn a_literals_capacity_header_is_zero_and_not_its_neighbours_octets() {
    let main = SymbolId(1);
    let (n, s, sixteen, p, c) = (ValueId(0), ValueId(1), ValueId(2), ValueId(3), ValueId(4));
    let f = leaf(
        main,
        vec![
            (n, Instruction::ConstStr(vec![7, 0, 0, 0, 0, 0, 0, 0])),
            (s, Instruction::ConstStr(b"\xe0\xa4\xa8".to_vec())),
            (sixteen, Instruction::ConstInt(16)),
            (p, Instruction::Sub(s, sixteen)),
            (c, Instruction::LoadAt(p)),
        ],
        c,
    );
    let (text, halt, _) = run(&module(vec![(main, "मुख्य", f)], main));
    assert!(
        text.contains("पाठ१ॱॱ"),
        "the unused first literal is still pooled, so the probe is the SECOND entry:\n{text}"
    );
    assert!(
        text.contains("॥ अष्टकाः ७ ० ० ० ० ० ० ० ॥"),
        "and its octets are the eight that make the word 7:\n{text}"
    );
    assert_eq!(
        status(&halt),
        0,
        "`पाठ१ वियोगः १६` is the capacity ०; a 7 means the word is not there and the \
         neighbour's blob was read instead; halt was {halt:?}\n{text}"
    );
    println!("METRIC riscv64_string_capacity_header_read 0");
}

/// Identical literals share one pool entry, as identical integers do — so the
/// difference of their addresses is ZERO and only one blob is written.
#[test]
fn two_identical_literals_are_one_entry_and_one_address() {
    let main = SymbolId(1);
    let (a, b, d) = (ValueId(0), ValueId(1), ValueId(2));
    let f = leaf(
        main,
        vec![
            (a, Instruction::ConstStr(b"AB".to_vec())),
            (b, Instruction::ConstStr(b"AB".to_vec())),
            (d, Instruction::Sub(b, a)),
        ],
        d,
    );
    let (text, halt, _) = run(&module(vec![(main, "मुख्य", f)], main));
    assert_eq!(
        text.matches("॥ अष्टकाः ६५ ६६ ॥").count(),
        1,
        "the octets are written once:\n{text}"
    );
    assert!(
        !text.contains("पाठ१ॱॱ"),
        "and there is no second entry:\n{text}"
    );
    assert_eq!(status(&halt), 0, "same literal, same address\n{text}");
}
