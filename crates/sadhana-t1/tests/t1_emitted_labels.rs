//! **`W-279` — WHAT LABEL DOES THE `.t1` CHAIN ACTUALLY EMIT?**
//!
//! # The gap this closes, stated as the ladder left it
//!
//! Rungs 102 and 103 of the `स्वपरीक्षा` ladder each rest on a claim about a
//! NAME, and neither claim was ever taken from the chain under test:
//!
//! * rung 102 — a cross-module call — claims the emitted branch targets
//!   `अष्टकशून्याष्टकयोजनम्`, the callee's own label.
//! * rung 103 — two module-level globals — claims the two loads address `कग`
//!   and `कच`, one storage each.
//!
//! **A RUNG CANNOT SEE EITHER, AND ITS OWN MARGIN SAYS SO.** A rung answers
//! `११००० + <object octets>`; what an octet LENGTH cannot see is WHICH STORAGE
//! EACH LOAD ADDRESSES. `कग` and `कच` are both eight octets, so an emitter that
//! addressed `कग` twice places the same eighteen instructions, answers the same
//! ११०७२, and computes १४ where the source says १२. The two labels therefore
//! rested on `tools/demo.sh` — the RUST chain — for three cycles, and the ledger
//! named moving them onto the `.t1` chain as the cheapest thing on the board.
//!
//! This file is that instrument. It is not a rung: it needs no image, no
//! twenty-minute build and no census walk. It hands `शृङ्खलाॱमण्डलसङ्कलनम्` —
//! the `.t1` driver's whole front half, source octets to Sassembly text — the
//! EXACT sources rungs 102 and 103 compile, and reads the labels out of the text
//! the `.t1` emitter wrote.
//!
//! # Why the fixtures are copied from the rungs character for character
//!
//! [`RUNG_103_SOURCE`] and [`RUNG_102_SOURCE`] are the whitespace-normalised
//! form of `शृङ्खला`'s own `प१ ⧺ प२` — what the literal becomes after
//! `पाठसंयोगः`, which is what `मण्डलसङ्कलनम्` is handed at run time. A fixture
//! written to be *like* the rung's source would make this file's green a
//! statement about a program no rung compiles.
//!
//! # THE INSTRUMENT HAS THREE STATES AND NOT TWO
//!
//! "The two loads address two labels" and "they do not" is a two-state reading,
//! and the truth here has three: nothing addressed at all, distinct labels, or
//! ONE label addressed twice. An emitter that lowered the whole `.data` section
//! away emits no address sequence, and a checker that only asked "are the labels
//! distinct?" would score `[]` as a pass — vacuously, because an empty list has
//! no duplicates. So [`addressed_labels`] answers the list IN ORDER and
//! [`repeats`] names any label that appears more than once, and the tests assert
//! on both: the list's contents AND the absence of a repeat.
//!
//! # AND THE CASE THAT MUST STILL BE REFUSED
//!
//! [`a_duplicated_address_is_named_rather_than_counted`] takes the text the
//! chain really emitted and rewrites `कच` to `कग` — the one-label-twice defect,
//! made by hand — and asserts the instrument reports it BY NAME. Without that,
//! nothing here separates an instrument that reads labels from one that returns
//! a plausible pair; the same mutation control `t1_exec_riscv.rs` uses for its
//! one-octet divergence, for the same reason.
//!
//! # WHAT THE FIRST RUN MEASURED, KEPT BECAUSE IT IS WHY THE READINGS ARE SHAPED
//! # AS THEY ARE
//!
//! Two of these three tests FAILED on their first run against this tree, and in
//! both cases the CHAIN was right and the instrument was narrow. Recorded
//! because a reading that was corrected before anyone saw it reads afterwards
//! like a reading that was always right:
//!
//! ```text
//!   exported labels   measured ["कघ","कग","कच"]   asserted ["कग","कच"]
//!   jump targets      measured ["अष्टकशून्याष्टकयोजनम्","कघनिर्गम"]
//!                     asserted ["अष्टकशून्याष्टकयोजनम्"]
//! ```
//!
//! `॥ वैश्विकम् ॥` exports ROUTINES as well as globals, and `लङ्घनम्` is the
//! mnemonic for a CALL and an unconditional BRANCH alike. Both corrections made
//! the instrument say MORE rather than filter: [`data_globals`] reads the
//! initialiser as well as the label, and [`jump_targets`] takes the destination
//! register — the ISA's own distinction between linking and not — so the branch
//! is asserted beside the call instead of being dropped.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

/// THE READER, SHARED WITH `t1_corpus_globals.rs`. See `emitted/mod.rs`: one
/// answer in this tree to what `॥ वैश्विकम् ॥` means, so the corpus sweep and
/// the rungs' own fixtures cannot drift apart and both stay green.
mod emitted;

use emitted::{
    Kind, RA, SP, Storage, ZERO, conditional_operand_slots, data_globals, devanagari,
    devanagari_int, is_data_literal, label_transfer,
};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

/// THIS FILE'S OWN LOADER, and it loads the SHIPPED MANIFEST.
///
/// `t1_exec_riscv.rs`'s loader takes a hand-written module list because it
/// drives `यन्त्रोत्सर्जन` in isolation; `w279_bare_type_lowering.rs`'s takes one
/// because it stops at built IR. Neither reaches `शृङ्खला`, and a list written
/// here would pass while `CHAIN` was missing a module — the failure
/// `t1_driver.rs:87`'s margin records having happened in two other files. So the
/// manifest is the one the product ships.
fn load() -> Interpreter {
    Interpreter::load(CHAIN, &spec_root()).expect("the shipped chain loads")
}

/// The `.t1` driver's front half over one source: the emitted Sassembly text and
/// the exit `शृङ्खला` recorded for it.
///
/// BOTH, NEVER JUST THE TEXT. `मण्डलसङ्कलनम्` answers the EMPTY run from six
/// different refusals and its own margin calls that history "returns that
/// recorded nothing"; `सङ्कलनविरामभेद` is the sentinel that tells them apart, and
/// a caller that read only the octets would report a refusal as an emitter that
/// happens to emit no labels.
fn compile(it: &mut Interpreter, src: &str, module: &str) -> (String, i128) {
    it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], 8_000_000_000)
        .expect("सङ्कलनारम्भः runs");
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(src), octets(module)],
            80_000_000_000,
        )
        .expect("मण्डलसङ्कलनम् runs");
    let text = match text.octets() {
        Some(o) => String::from_utf8(o.as_slice().to_vec()).expect("the emitter writes UTF-8"),
        None => panic!("मण्डलसङ्कलनम् answers a run of octets, not {text:?}"),
    };
    let exit = it
        .global("सङ्कलनविरामभेद")
        .and_then(Value::as_int)
        .expect("शृङ्खला records an exit for every call");
    (text, exit)
}

/// `सङ्कलनसिद्धभेद` — `shrinkhala.t1:123`, "emitted".
const EMITTED: i128 = 0;

// ── THE INSTRUMENT ──────────────────────────────────────────────────────────
//
// Three readings off one text, each a pure function so the mutation control
// below can drive them without the chain.

/// Every label the object EXPORTS: the `label` of `॥ वैश्विकम् {label} ॥`.
///
/// **ROUTINES AND GLOBALS BOTH, AND THAT IS THE READING RATHER THAN A LEAK.**
/// `riscv64.rs:519` exports every routine label and `:1429` every global, so the
/// answer here is the object's whole export list — for rung 103's source, the
/// routine `कघ` and then the two storages. The first version of this file
/// asserted two entries and measured three; the text was right and the
/// instrument was narrow, which is the direction an instrument is allowed to be
/// wrong in only once.
fn exported_labels(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| {
            // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY. `॥ वैश्विकम् X ॥` is
            // three fields and a label may be any Devanagari run.
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.as_slice() {
                ["॥", "वैश्विकम्", label, "॥"] => Some((*label).to_string()),
                _ => None,
            }
        })
        .collect()
}

/// Every label an ADDRESS SEQUENCE names, in emission order, WITH WHAT THE
/// SEQUENCE THEN DID WITH THE ADDRESS.
///
/// A PC-relative address is two lines and the register the FULL address lands
/// in is named by the SECOND (`riscv64.rs:1003` for a load, `:1088` for an
/// address taken):
///
/// ```text
///   स्थानसापेक्षयोगः {hi}म् {label}ॱउपरिन ।
///   योगः {addr}म् {hi}न {label}ॱअधःन ।
/// ```
///
/// **AND THE LINE AFTER IT IS THE ROLE, WHICH IS A THIRD STATE THIS FILE USED
/// TO COLLAPSE.** Measured on this tree: a source that WRITES a global then
/// READS it addresses that one storage TWICE, and both addressings are correct.
/// A reader that answered only the list of labels reports that program as
/// `[कग, कग]` — the exact shape [`repeats`] was built to name as a DEFECT. The
/// defect is two LOADS where the source reads two different globals; a store
/// and a load of one global is a program. So the role is read rather than
/// assumed:
///
/// * [`Role::Load`] — `आहारः {d}म् {addr}त् {off}न ।`
/// * [`Role::Store`] — `निधानम् {addr}य् {off}न {v}न ।`
/// * [`Role::Address`] — neither: the address itself is the value. A string
///   literal is this, and so is the base of the record arena.
///
/// **THIS ALSO SEES THE STRING POOL, AND WHAT IT SEES IS NOT WHAT THIS FILE
/// PREDICTED.** The previous margin here claimed `lower_string` addresses
/// `पाठकोशः`. Measured: a source returning `उक्तम् अब इति` addresses **`पाठ०`**,
/// the literal's OWN label — `पाठकोशः` is emitted as a pool label but nothing
/// addresses it. The prediction is kept above the correction because a claim
/// repaired before anyone read it afterwards looks like a claim that was always
/// right.
fn addressings(text: &str) -> Vec<(String, Role)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
        let f: Vec<&str> = l.split_whitespace().collect();
        let (hi, label) = match f.as_slice() {
            ["स्थानसापेक्षयोगः", r, operand, "।"] => {
                match operand.strip_suffix("ॱउपरिन") {
                    Some(label) => (r.strip_suffix("म्").unwrap_or(r), label),
                    None => continue,
                }
            }
            _ => continue,
        };
        // The low half, and it must name THIS label against THIS register — two
        // addressings emitted back to back would otherwise let the second's
        // register decide the first's role.
        let lo: Vec<&str> = lines
            .get(i + 1)
            .map(|l| l.split_whitespace().collect())
            .unwrap_or_default();
        let low_half = format!("{label}ॱअधःन");
        let addr = match lo.as_slice() {
            ["योगः", d, s, operand, "।"]
                if s.strip_suffix("न") == Some(hi) && **operand == low_half =>
            {
                d.strip_suffix("म्").unwrap_or(d)
            }
            // A high half with no low half forms no usable address. Reported as
            // an addressing all the same — dropping it would hide a truncated
            // pair, which is the one thing a `.text` length also cannot see.
            _ => {
                out.push((label.to_string(), Role::Address));
                continue;
            }
        };
        let third: Vec<&str> = lines
            .get(i + 2)
            .map(|l| l.split_whitespace().collect())
            .unwrap_or_default();
        let role = match third.as_slice() {
            ["आहारः", _, src, _, "।"] if src.strip_suffix("त्") == Some(addr) => {
                Role::Load
            }
            ["निधानम्", base, _, _, "।"] if base.strip_suffix("य्") == Some(addr) => {
                Role::Store
            }
            _ => Role::Address,
        };
        out.push((label.to_string(), role));
    }
    out
}

/// What an address sequence did with the address. See [`addressings`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Role {
    Load,
    Store,
    Address,
}

/// Every label an address sequence names, in emission order — the roles
/// dropped.
///
/// Kept beside [`addressings`] because the claims rungs 102 and 103 rest on are
/// about WHICH STORAGE, and [`repeats`] takes a list. A caller that cares which
/// way the access went asks [`addressings`] instead.
fn addressed_labels(text: &str) -> Vec<String> {
    addressings(text).into_iter().map(|(l, _)| l).collect()
}

/// Every label reached by `लङ्घनम् {rd}म् {target}य् ।` whose destination is
/// `rd` — in emission order, with the `य्` (कर्म, the place gone TO) stripped.
///
/// **THE LINK REGISTER IS WHAT SEPARATES A CALL FROM A BRANCH, AND THE FIRST
/// VERSION OF THIS FILE CONFLATED THEM.** One mnemonic serves both: a CALL
/// writes the return address (`riscv64.rs:70`, `पुनःस्थानम्`) and an
/// unconditional BRANCH discards it into `शून्यः` (`:69`, x0). Measured on rung
/// 102's own source, a matcher that ignored the destination answered
/// `[अष्टकशून्याष्टकयोजनम्, कघनिर्गम]` — the cross-module call AND the jump to
/// the routine's own exit block, which is not a call at all and is not even a
/// symbol another object could resolve.
fn jump_targets(text: &str, rd: &str) -> Vec<String> {
    let dest = format!("{rd}म्");
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.as_slice() {
                ["लङ्घनम्", d, target, "।"] if *d == dest => {
                    target.strip_suffix("य्").map(str::to_string)
                }
                _ => None,
            }
        })
        .collect()
}

/// THE THIRD STATE. Any label the list names more than once, with its count.
///
/// An empty answer from [`addressed_labels`] has no repeats either, so a test
/// must assert the LIST and this — see the header.
fn repeats(labels: &[String]) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for l in labels {
        if out.iter().any(|(n, _)| n == l) {
            continue;
        }
        let n = labels.iter().filter(|o| *o == l).count();
        if n > 1 {
            out.push((l.clone(), n));
        }
    }
    out
}

/// Every label this text DEFINES, in emission order — the `X` of a lone `Xॱॱ`.
///
/// Read with the same shape [`text_lines`] skips as a label, so the two readers
/// cannot disagree about what a label line is. Kept public to the tests because
/// [`transfers`] is only meaningful if no label is defined TWICE: with a
/// duplicate, "the target is earlier in the text" has two answers and the
/// direction below would be a statement about which one this reader happened to
/// find. Every caller asserts `repeats(&defined_labels(text))` is empty.
fn defined_labels(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| {
            // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
            let f: Vec<&str> = l.split_whitespace().collect();
            match f.as_slice() {
                [one] => one.strip_suffix("ॱॱ").map(str::to_string),
                _ => None,
            }
        })
        .collect()
}

/// **WHERE EVERY CONTROL TRANSFER GOES, AND WHETHER IT GOES BACKWARDS.**
///
/// A transfer to a label is any instruction whose LAST operand is a `…य्` —
/// the कर्म, the place gone TO. That shape and not a mnemonic list, because the
/// conditional mnemonics are an open set (`न्यूनलङ्घनम्` here, and the ISA has
/// more) while the operand role is fixed; a reader keyed on mnemonics answers a
/// SHORTER list for text carrying a branch nobody had taught it, and a short
/// list is indistinguishable from a lowering that placed no branch. The return
/// (`सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।`) is not one: its last operand is
/// an offset.
///
/// **THE PREDICATE ITSELF LIVES IN [`label_transfer`]**, in `emitted/mod.rs`,
/// so the corpus sweep cannot classify one line differently from this one.
fn transfers(text: &str) -> Vec<Transfer> {
    // The FIRST definition's line index per label, so a direction is never a
    // statement about which duplicate was found — the callers refuse duplicates
    // outright, and this keeps the reading determinate if one ever appears.
    let lines: Vec<&str> = text.lines().collect();
    let mut at: Vec<(String, usize)> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY — the same shape
        // [`defined_labels`] reads, so the two cannot disagree.
        let f: Vec<&str> = l.split_whitespace().collect();
        let Some(label) = (match f.as_slice() {
            [one] => one.strip_suffix("ॱॱ"),
            _ => None,
        }) else {
            continue;
        };
        if !at.iter().any(|(n, _)| n == label) {
            at.push((label.to_string(), i));
        }
    }
    // THE BLOCK A TRANSFER SITS IN, tracked as the text is walked. A transfer
    // is only as live as its block: see [`live_backward_targets`].
    let mut from = String::new();
    let mut out = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
        let f: Vec<&str> = l.split_whitespace().collect();
        if let [one] = f.as_slice()
            && let Some(label) = one.strip_suffix("ॱॱ")
        {
            from = label.to_string();
        }
        // THREE STATES AND NOT TWO, AND THE SECOND SPLIT IS `W-279`'S DEBT.
        // `लङ्घनम्` is the ISA's ONE unconditional label transfer, and the link
        // register says which of its two forms this is — written into
        // `पुनःस्थानम्` a CALL, discarded into `शून्यः` a JUMP, the distinction
        // [`jump_targets`]'s margin gives. EVERY OTHER mnemonic that names a
        // label is CONDITIONAL: control may continue past it, which is exactly
        // the fall-through [`reachability`] reads off the last instruction of a
        // block.
        //
        // THE CLASSIFICATION IS KEYED ON THE UNCONDITIONAL FORM AND THE OPEN
        // SET IS ITS COMPLEMENT, which INVERTS this function's own warning
        // above. Keyed the other way — a list of conditional mnemonics — a
        // branch nobody had taught this reader would read as unconditional, and
        // an emitter that lowered a `यदि` into a mnemonic the list did not carry
        // would answer ZERO conditionals, indistinguishable from a lowering that
        // placed no test at all. Keyed this way it is COUNTED. The ladder emits
        // two of them, `न्यूनलङ्घनम्` (rungs 98, 99, 100) and `समलङ्घनम्`
        // (rung 101); the ISA has more and this reader needs to know none of
        // them.
        //
        // AND THE PREDICATE ITSELF LIVES IN [`label_transfer`], so the operand
        // reader cannot classify one line differently from this one.
        let Some((mnemonic, target, kind)) = label_transfer(&f) else {
            continue;
        };
        // THREE STATES AND NOT TWO. A target this object never defines is not a
        // forward edge: it is a relocation (rung 102's cross-module call) or a
        // dangling branch, and a reader with only `Backward`/`Forward` reports
        // both as forward — the second being a defect a `.text` LENGTH also
        // cannot see.
        let dir = match at.iter().find(|(n, _)| n == target) {
            Some((_, j)) if *j < i => Dir::Backward,
            Some(_) => Dir::Forward,
            None => Dir::Unplaced,
        };
        out.push(Transfer {
            mnemonic: mnemonic.to_string(),
            target: target.to_string(),
            kind,
            dir,
            from: from.clone(),
        });
    }
    out
}

/// One label-targeting control transfer. See [`transfers`].
#[derive(Debug, PartialEq, Eq, Clone)]
struct Transfer {
    mnemonic: String,
    target: String,
    kind: Kind,
    dir: Dir,
    /// The label of the block this transfer sits in. See [`blocks`].
    from: String,
}

/// Where a transfer's target sits relative to the transfer. THREE states — see
/// [`transfers`] for why `Unplaced` is not folded into `Forward`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Dir {
    Backward,
    Forward,
    Unplaced,
}

/// Every label a transfer reaches BACKWARDS — the loop edge, named.
fn backward_targets(text: &str) -> Vec<String> {
    transfers(text)
        .into_iter()
        .filter(|t| t.dir == Dir::Backward)
        .map(|t| t.target)
        .collect()
}

/// **WHICH SECTION A LINE IS IN, AND THERE ARE THREE OF THEM.**
///
/// `॥ कोष्ठकम् ॱX ॥` switches sections and the emitter writes THREE names —
/// `ॱपाठ`, `ॱदत्त` and `ॱरिक्त` (`yantrotsarjana.t1:2131-2151`). A reader that
/// keyed on "not `ॱदत्त`" would count a `.bss` label as a basic block, so this
/// keys on the TEXT name positively. The emitter opens in `.text` implicitly —
/// measured: every fixture in this file emits its first label with no section
/// directive above it, and the only directive any of them writes is the
/// `ॱदत्त` near the foot.
fn text_section_lines(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_text = true;
    for l in text.lines() {
        // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
        let f: Vec<&str> = l.split_whitespace().collect();
        if let ["॥", "कोष्ठकम्", name, "॥"] = f.as_slice() {
            in_text = *name == "ॱपाठ";
            continue;
        }
        if in_text {
            out.push(l.trim().to_string());
        }
    }
    out
}

/// One basic block of the `.text`: the label that opens it and the instruction
/// lines it owns, in emission order. See [`blocks`].
#[derive(Debug, PartialEq, Eq, Clone)]
struct Block {
    label: String,
    insts: Vec<String>,
}

/// **THE `.text` CUT INTO BASIC BLOCKS — AND AN EMPTY ONE IS A BLOCK.**
///
/// A label opens a block; every instruction line until the next label belongs
/// to it. Directives do not open or close one: `॥ वैश्विकम् कङ ॥` sits between
/// a routine's return and the next routine's label, and a reader that closed a
/// block on it would answer a different count for the same program.
///
/// EMPTY BLOCKS ARE KEPT, because they are what reconciles the two instruments
/// this tree counts blocks with — see
/// [`the_block_count_the_two_instruments_disagree_on_is_the_empty_blocks`].
/// Dropping them here would hide the disagreement rather than resolve it.
fn blocks(text: &str) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    for l in text_section_lines(text) {
        // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
        let f: Vec<&str> = l.split_whitespace().collect();
        match f.as_slice() {
            [] => {}
            ["॥", .., "॥"] => {}
            [one] if one.ends_with("ॱॱ") => out.push(Block {
                label: one.trim_end_matches("ॱॱ").to_string(),
                insts: Vec::new(),
            }),
            [.., "।"] => {
                if let Some(b) = out.last_mut() {
                    b.insts.push(l.clone());
                }
            }
            _ => {}
        }
    }
    out
}

/// **WHICH BLOCKS THE OBJECT CAN ACTUALLY REACH, AND FALLTHROUGH IS AN EDGE.**
///
/// Answers `(reachable, dead)`, both in emission order, from a walk that starts
/// at the EXPORTED labels — `॥ वैश्विकम् X ॥` names a routine entry, and a
/// routine nobody calls is still reachable from outside the object.
/// [`exported_labels`] also names DATA labels, so the roots are intersected
/// with the blocks this `.text` defines.
///
/// A block has two kinds of successor and BOTH are followed:
///
/// ```text
///   the target of any … य् transfer it carries
///   the NEXT block, unless its last instruction ends the block
/// ```
///
/// An unconditional `लङ्घनम् शून्यःम् Xय्` and the return
/// `सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न` end it; a CALL does not, because the
/// callee comes back, and a CONDITIONAL does not, because its untaken arm is
/// the next block. Dropping the fallthrough edge is the defect
/// [`a_block_reached_only_by_fallthrough_is_not_read_as_dead`] refuses: two of
/// the five sources this file compiles have a block reached that way and no
/// branch pointing at it.
fn reachability(text: &str) -> (Vec<String>, Vec<String>) {
    let bs = blocks(text);
    let order: Vec<String> = bs.iter().map(|b| b.label.clone()).collect();
    let zero = format!("{ZERO}म्");
    let mut seen: Vec<String> = Vec::new();
    let mut stack: Vec<String> = exported_labels(text)
        .into_iter()
        .filter(|e| order.contains(e))
        .collect();
    while let Some(b) = stack.pop() {
        if seen.contains(&b) {
            continue;
        }
        seen.push(b.clone());
        let Some(i) = order.iter().position(|x| *x == b) else {
            continue;
        };
        let mut falls = true;
        for (n, l) in bs[i].insts.iter().enumerate() {
            // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
            let f: Vec<&str> = l.split_whitespace().collect();
            let last = n + 1 == bs[i].insts.len();
            match f[f.len() - 2].strip_suffix("य्") {
                Some(target) => {
                    stack.push(target.to_string());
                    if last && f[0] == "लङ्घनम्" && f[1] == zero {
                        falls = false;
                    }
                }
                None => {
                    if last && f[0] == "सापेक्षलङ्घनम्" && f[1] == zero
                    {
                        falls = false;
                    }
                }
            }
        }
        if falls && let Some(next) = order.get(i + 1) {
            stack.push(next.clone());
        }
    }
    let live: Vec<String> = order.iter().filter(|l| seen.contains(l)).cloned().collect();
    let dead: Vec<String> = order
        .iter()
        .filter(|l| !seen.contains(l))
        .cloned()
        .collect();
    (live, dead)
}

/// Every label a transfer FROM A REACHABLE BLOCK reaches backwards — the loop
/// edge the object can actually take.
///
/// [`backward_targets`] answers the same list for rung 99 and for rung 98, and
/// one of those two programs has a loop while the other has a jump in code
/// nothing reaches. See
/// [`rung_98_and_rung_99_agree_on_one_backward_edge_and_reachability_is_what_tells_them_apart`].
fn live_backward_targets(text: &str) -> Vec<String> {
    let (live, _) = reachability(text);
    transfers(text)
        .into_iter()
        .filter(|t| t.dir == Dir::Backward && live.contains(&t.from))
        .map(|t| t.target)
        .collect()
}

/// **EVERY BACKWARD EDGE THE OBJECT CAN NEVER TAKE — WITH THE BLOCK IT LEAVES,
/// THE LABEL IT REACHES AND WHETHER IT LINKS.**
///
/// The complement of [`live_backward_targets`] over the same walk, and it
/// answers the WHOLE edge rather than a target list. The reason is the table's
/// own weakest joint: rung 101 is held off rung 98's structural shape by the
/// COUNT of these and by nothing else, and a count is the least a table can
/// rest on — so what is being counted is read out.
///
/// THE MEMBERSHIP IS TAKEN AGAINST THE DEAD LIST AND NOT AGAINST `!live`. A
/// transfer standing before the first label of the `.text` carries the empty
/// block name, which is in NEITHER list, and a reader keyed on `!live` would
/// hand that back as a dead edge out of a block that does not exist. See
/// [`a_backward_edge_out_of_a_reachable_block_is_not_read_as_dead`], which
/// drives the struck reader rather than arguing about it.
fn dead_backward_edges(text: &str) -> Vec<(String, String, Kind)> {
    let (_, dead) = reachability(text);
    transfers(text)
        .into_iter()
        .filter(|t| t.dir == Dir::Backward && dead.contains(&t.from))
        .map(|t| (t.from, t.target, t.kind))
        .collect()
}

/// **WHAT A BLOCK NOTHING REACHES COSTS — AND WHETHER THE QUESTION WAS
/// ANSWERED AT ALL.** Three states, because the truth has three.
///
/// Rung 98 priced its dead code as a bare `usize`
/// (`t1_rung98_unreachable_instructions`, ONE of forty-one; since `W-279`,
/// NONE of forty, which is why this reading matters). A bare `usize`
/// is the instrument this reading cannot use, and the reason is the whole
/// point of taking it: the number a FIX would drive to zero is the same
/// number a BROKEN READER answers. A text the reader cannot cut into blocks —
/// a refusal, which emits no `.text` at all — sums to zero exactly as a rung
/// with no dead block does, so a two-state reader reports the emitter fix as
/// landed the moment it stops reading. Driven rather than argued by
/// [`a_rung_with_no_dead_block_answers_zero_and_a_refusal_answers_unread`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum DeadWeight {
    /// The text was NOT READ: it defines no `.text` block, or it carries a
    /// line shape [`text_lines`] does not recognise. Not a measurement.
    Unread,
    /// `n` instruction lines stand in blocks the walk of [`reachability`]
    /// never reaches. `Dead(0)` IS a measurement — the text was read whole and
    /// carries none.
    Dead(usize),
}

/// The price of [`reachability`]'s dead list, in instructions. See
/// [`DeadWeight`] for why this is not a `usize`.
fn dead_instructions(text: &str) -> DeadWeight {
    let bs = blocks(text);
    let (_, unknown) = text_lines(text);
    if bs.is_empty() || !unknown.is_empty() {
        return DeadWeight::Unread;
    }
    let (_, dead) = reachability(text);
    DeadWeight::Dead(
        bs.iter()
            .filter(|b| dead.contains(&b.label))
            .map(|b| b.insts.len())
            .sum(),
    )
}

// ── THE FIXTURES, COPIED FROM THE RUNGS ─────────────────────────────────────

/// Rung 103's source — `shrinkhala.t1`'s `स्वपरीक्षात्रिशततमी`, `प१ ⧺ प२`.
const RUNG_103_SOURCE: &str = "मण्डलम् क ॥ सार्वजनिक चरः ग ॱॱ न६४ भवति ७ । \
सार्वजनिक चरः च ॱॱ न६४ भवति ५ । सार्वजनिक वृत्तिः घ ददाति न६४ आदि \
प्रत्यागमनम् ग योगः च । इति";

/// Rung 102's source — `shrinkhala.t1`'s `स्वपरीक्षाद्विशततमी`, `प१ ⧺ प२`.
const RUNG_102_SOURCE: &str = "मण्डलम् क ॥ आयातः अष्टक । \
सार्वजनिक वृत्तिः घ ददाति न६४ आदि \
प्रत्यागमनम् अष्टकॱशून्याष्टकयोजनम् ७ । इति";

/// **RUNG 103'S CLAIM, TAKEN OFF THE CHAIN UNDER TEST.** Two globals, two
/// exported storages, and the two loads address DIFFERENT ones — `कग` then `कच`,
/// in the order the source reads them.
///
/// The label is `<module><name>` CONCATENATED, which is how `routine_label`
/// joins a routine's and how `shrinkhala.t1:438` hands the pair to
/// `यन्त्रनामयोजनम्`. `क` ⧺ `ग` is `कग`; a chain that minted `संज्ञा<id>` instead
/// — the spelling `W-253` had to remove from the call path — fails here by
/// NAME rather than by a count.
#[test]
fn the_t1_chain_names_each_global_it_reads_and_addresses_each_once() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_103_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the front half must reach the emitter; exit {exit} is a refusal \
         (shrinkhala.t1:118-154 numbers them)\n{text}"
    );

    // THE EXPORT LIST, WHOLE: the routine and then the two storages. Asserted as
    // a LIST and not as a membership, so a third global — or a routine that
    // stopped being exported — is named here rather than absorbed.
    let exported = exported_labels(&text);
    assert_eq!(
        exported,
        vec!["कघ".to_string(), "कग".to_string(), "कच".to_string()],
        "the object exports the routine `कघ` and one storage per global, each \
         label module and name CONCATENATED\n{text}"
    );

    // AND THE TWO STORAGES CARRY THEIR OWN INITIALISERS. ०षोड्७ and ०षोड्५ are
    // ७ and ५ — the source's two values, in `.data`, one each. A table stuck at
    // one entry gives two labels and ONE of these.
    let data = data_globals(&text);
    assert_eq!(
        data,
        vec![
            ("कग".to_string(), Storage::Word("०षोड्७".to_string())),
            ("कच".to_string(), Storage::Word("०षोड्५".to_string())),
        ],
        "each global's own initialiser reaches `.data` AS A VALUE — a scalar's \
         data word IS the number, where a run's is the ADDRESS of its storage \
         and the two are one directive apart\n{text}"
    );

    let addressed = addressed_labels(&text);
    println!("METRIC t1_emitted_global_addresses {}", addressed.len());
    println!("METRIC t1_emitted_data_globals {}", data.len());
    println!("METRIC t1_emitted_exported_labels {}", exported.len());
    assert_eq!(
        addressed,
        vec!["कग".to_string(), "कच".to_string()],
        "THE CLAIM NO RUNG CAN CHECK: one address sequence per global read, and \
         the second reads `कच` — an emitter that addressed `कग` twice places the \
         same eighteen instructions and answers the same ११०७२\n{text}"
    );
    assert!(
        repeats(&addressed).is_empty(),
        "no storage is addressed twice: {:?}\n{text}",
        repeats(&addressed)
    );
    // AND BOTH ADDRESSINGS ARE LOADS, which is what "reads two globals" means.
    // The empty `repeats` above is a statement about THIS SOURCE and not a law
    // of the emitter —
    // `a_global_write_addresses_the_same_storage_a_second_time_and_is_correct`
    // measures a program where one storage is addressed twice and both are
    // right. What separates them is the role, so the role is asserted here.
    assert_eq!(
        addressings(&text),
        vec![
            ("कग".to_string(), Role::Load),
            ("कच".to_string(), Role::Load),
        ],
        "two LOADS of two distinct storages; a `निधानम्` against either address \
         would be a write this source never wrote\n{text}"
    );

    // AND THE THREE LINES ARE A LOAD, not two-thirds of one. The high half, the
    // low half against it, then the load — `riscv64.rs:1003`. Asserted per label
    // so that a lowering which kept the address and dropped the `आहारः` (a
    // constant fold, the one direction this row can be surprised) is named here.
    for g in ["कग", "कच"] {
        assert!(
            text.contains(&format!("{g}ॱउपरिन ।")) && text.contains(&format!("{g}ॱअधःन ।")),
            "`{g}` is addressed by both halves of a PC-relative pair\n{text}"
        );
    }
}

/// **THE CASE THAT MUST STILL BE REFUSED**, and it is the reason the test above
/// can be believed. Take the text the chain really wrote and rewrite the second
/// global's label to the first's — the one-storage-twice defect, by hand — and
/// the instrument must NAME it. An extractor that answered a plausible pair
/// whatever the text said passes the test above and fails this one.
///
/// The empty case is here too: strip the address lines and the answer is the
/// empty list, NOT a pass. A checker that only asked "are they distinct?" would
/// score that green, because an empty list has no duplicates.
#[test]
fn a_duplicated_address_is_named_rather_than_counted() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_103_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the control must itself have emitted\n{text}"
    );

    let mutated = text.replace("कचॱउपरिन", "कगॱउपरिन");
    assert_ne!(
        mutated, text,
        "the mutation must change the text, or this control tests nothing"
    );
    let labels = addressed_labels(&mutated);
    assert_eq!(
        labels,
        vec!["कग".to_string(), "कग".to_string()],
        "the instrument reads the text it is given, not the source it came from"
    );
    assert_eq!(
        repeats(&labels),
        vec![("कग".to_string(), 2)],
        "a storage addressed twice is reported BY NAME and BY COUNT"
    );

    // THE THIRD STATE: nothing addressed at all.
    let stripped: String = text
        .lines()
        .filter(|l| !l.starts_with("स्थानसापेक्षयोगः"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        addressed_labels(&stripped).is_empty(),
        "an emitter that formed no addresses answers the empty list"
    );
    assert!(
        repeats(&addressed_labels(&stripped)).is_empty(),
        "and the empty list has no repeats — which is why `repeats` alone is \
         not a pass and the list's contents are asserted beside it"
    );
}

/// **RUNG 102'S CLAIM, TAKEN OFF THE CHAIN UNDER TEST.** The branch target of a
/// cross-module call is the CALLEE's label — `<callee module><callee routine>` —
/// and not the caller's, not a mint, not the import's.
///
/// `अष्टकशून्याष्टकयोजनम्` is the whole point: the module half is `अष्टक` where
/// every other label in this object begins `क`. A chain that labelled the callee
/// out of the CALLING module would link to storage in the wrong object and the
/// rung's octet count would not move by one.
#[test]
fn the_t1_chain_names_the_callee_module_of_a_cross_module_call() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_102_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the cross-module source must reach the emitter; exit {exit}\n{text}"
    );

    let targets = jump_targets(&text, RA);
    let branches = jump_targets(&text, ZERO);
    println!("METRIC t1_emitted_call_targets {}", targets.len());
    println!("METRIC t1_emitted_branch_targets {}", branches.len());
    assert_eq!(
        targets,
        vec!["अष्टकशून्याष्टकयोजनम्".to_string()],
        "the call branches to the CALLEE's own label, module half and all\n{text}"
    );
    assert!(
        targets[0].starts_with("अष्टक"),
        "the label's MODULE HALF is the callee's `अष्टक`; one formed out of the \
         CALLING module would begin `क` and would link to the wrong object \
         without changing the rung's octet count by one\n{text}"
    );
    // THE BRANCH IS NOT A CALL, and it is asserted rather than merely excluded:
    // the same mnemonic reaches the routine's own exit block through `शून्यः`,
    // and a reader that saw two call targets here would report a cross-module
    // call this source does not make.
    assert_eq!(
        branches,
        vec!["कघनिर्गम".to_string()],
        "the unconditional branch to the exit block is local and discards the \
         return address\n{text}"
    );
    // AND THE CALLER'S OWN ROUTINE IS LABELLED FROM ITS OWN MODULE, so the two
    // halves of the join are visibly different modules rather than one.
    assert!(
        text.contains("॥ वैश्विकम् कघ ॥"),
        "the caller's routine `घ` of module `क` is exported as `कघ`\n{text}"
    );
}

// ── THE THREE LABEL FORMS THE RUNGS DO NOT REACH ────────────────────────────
//
// Rungs 102 and 103 read SCALAR globals and call. Three label forms the corpus
// depends on are emitted by sources no rung compiles, and each was named as a
// gap rather than measured. These fixtures are the smallest source that reaches
// each one.

/// A module-level RUN global — `अङ्कः अन्तः न६४`, `ir.t1:999`'s
/// `खण्डसामर्थ्यम्` = १२८ elements — written and then read.
///
/// The corpus has 163 globals of this shape at 1 KiB each, so this is not a
/// corner: it is the form most of the compiler's own storage takes.
const RUN_GLOBAL_SOURCE: &str = "मण्डलम् क ॥ सार्वजनिक चरः ग ॱॱ अङ्कः अन्तः न६४ भवति ० । \
सार्वजनिक वृत्तिः घ ददाति न६४ आदि ग अङ्कः १ अन्तः भवति ९ । \
प्रत्यागमनम् ग अङ्कः १ अन्तः । इति";

/// Rung 103's own shape with ONE global, and a WRITE before the read.
///
/// `W-284`'s storage model lowers a global write as `AddrOfGlobal` then
/// `StoreAt`. Every rung on the ladder only READS, so the store side of the
/// address pair had never been read off the `.t1` chain.
const GLOBAL_WRITE_SOURCE: &str = "मण्डलम् क ॥ सार्वजनिक चरः ग ॱॱ न६४ भवति ७ । \
सार्वजनिक वृत्तिः घ ददाति न६४ आदि ग भवति ९ । प्रत्यागमनम् ग । इति";

/// A string literal — the third label class, and the one this file's margin
/// made a WRONG prediction about. See [`addressings`].
const STRING_LITERAL_SOURCE: &str =
    "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ ददाति पाठः आदि प्रत्यागमनम् उक्तम् अब इति । इति";

/// **`W-293` OFF THE CHAIN UNDER TEST: A RUN GLOBAL'S DATA WORD IS A POINTER,
/// AND THE STORAGE IT POINTS AT IS A SEPARATE BLOCK WITH A SIZE.**
///
/// Neither half of that is visible to a rung. `॥ अष्टाष्टकाः कगभण्डार ॥` and
/// `॥ अष्टाष्टकाः ०षोड्७ ॥` are both eight octets, and the storage lives in
/// `ॱरिक्त` — space in memory, absent from the FILE — so an emitter that placed
/// the pointer and no storage emits an object of exactly the same length and
/// answers exactly the same `११०००+n`. Every read of the global would then
/// resolve through a null base.
#[test]
fn a_run_global_gets_a_pointer_word_and_its_own_reserved_storage() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUN_GLOBAL_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "a run-typed module global must reach the emitter; exit {exit} is a \
         refusal (shrinkhala.t1:118-154 numbers them)\n{text}"
    );

    // THE EXPORT LIST GAINS A ROUTINE THE SOURCE NEVER WROTE. `कखण्डवृद्धिः` is
    // the module's own block-growth helper, labelled from the module half `क`
    // like everything else in this object — so a run global costs an exported
    // routine as well as a storage, and that is asserted rather than tolerated
    // as noise.
    let exported = exported_labels(&text);
    assert_eq!(
        exported,
        vec!["कघ".to_string(), "कखण्डवृद्धिः".to_string(), "कग".to_string()],
        "the routine, the growth helper a run drags in, and the storage\n{text}"
    );

    let data = data_globals(&text);
    println!("METRIC t1_emitted_run_global_data_globals {}", data.len());
    assert_eq!(
        data,
        vec![(
            "कग".to_string(),
            Storage::Run {
                store: "कगभण्डार".to_string(),
                octets: 1024,
            }
        )],
        "the data word NAMES `कगभण्डार` and that block really is placed, 1,024 \
         octets of it\n{text}"
    );

    // 1,024 IS DERIVED AND NOT QUOTED: `ir.t1:999` declares
    // `खण्डसामर्थ्यम् भवति १२८` and `ir.t1:4492` records the global's size as
    // `खण्डसामर्थ्यम् गुणनम् ८`. A capacity change should red this line by name.
    const CAPACITY: u64 = 128;
    let Storage::Run { octets, .. } = &data[0].1 else {
        panic!("a run global, not {:?}", data[0].1);
    };
    println!("METRIC t1_emitted_run_global_octets {octets}");
    assert_eq!(
        *octets,
        CAPACITY * 8,
        "the storage is `खण्डसामर्थ्यम् गुणनम् ८` octets — ir.t1:999 and :4492"
    );

    // AND THE WORD IS AN ADDRESS RATHER THAN A VALUE, WHICH IS `ADR-0013` AND
    // NOT A CONVENTION OF THIS FILE. Asserted beside a scalar's word so the two
    // states are separated here rather than argued.
    assert!(
        !is_data_literal("कगभण्डार"),
        "a data token beginning with a LETTER is a NAME — the address of what \
         it names, relocated at link time (parse.rs:903-915)"
    );
    assert!(
        is_data_literal("०षोड्७"),
        "and a scalar's is a literal, or this discriminator separates nothing"
    );

    // THE `ॱरिक्त` BLOCK, WHOLE AND CONTIGUOUS — the section and the alignment,
    // which no reading above can see. `ॱरिक्त` is why 163 corpus globals at
    // 1 KiB each cost no image size, and `॥ संरेखः १६ ॥` with the eight octets
    // of padding is why `कगभण्डार` lands १६-aligned rather than at १६k+८.
    let block: Vec<String> = vec![
        "॥ वैश्विकम् कग ॥",
        "कगॱॱ",
        "॥ अष्टाष्टकाः कगभण्डार ॥",
        "॥ कोष्ठकम् ॱरिक्त ॥",
        "॥ संरेखः १६ ॥",
        "॥ स्थानम् ८ ॥",
        "कगभण्डारशीर्षॱॱ",
        "॥ स्थानम् ८ ॥",
        "कगभण्डारॱॱ",
        "॥ स्थानम् १०२४ ॥",
        "॥ कोष्ठकम् ॱदत्त ॥",
        "॥ संरेखः १६ ॥",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    assert!(
        text.contains(&block.join("\n")),
        "the run global's twelve lines are emitted contiguously and in this \
         order — `W-293` for the pointer, `W-len` for the `शीर्ष` word in front \
         of the storage\n{text}"
    );

    // THE LENGTH HEADER IS A SEPARATE CLAIM FROM THE STORAGE. `W-len` puts one
    // word at `भण्डार वियोगः ८`, in FRONT of the storage, so the pointer does
    // not move and every existing access stays byte-identical.
    assert!(
        text.contains("कगभण्डारशीर्षॱॱ"),
        "the run carries its own length header\n{text}"
    );

    // THE WRITE AND THE TWO READS, BY ROLE. `ग अङ्कः १ अन्तः भवति ९` stores the
    // helper's new pointer back into the symbol word; the reads load it. The
    // same storage addressed four times is what an element access through a
    // pointer costs, and a list of labels alone cannot say that.
    let addr = addressings(&text);
    println!("METRIC t1_emitted_run_global_addressings {}", addr.len());
    assert_eq!(
        addr,
        vec![
            ("कग".to_string(), Role::Load),
            ("कग".to_string(), Role::Store),
            ("कग".to_string(), Role::Load),
            ("कग".to_string(), Role::Load),
            ("रचनासूचकः".to_string(), Role::Load),
            ("रचनाक्षेत्रम्".to_string(), Role::Address),
        ],
        "the symbol word is loaded, written back by the growth helper, then \
         loaded twice more; the helper reaches the record arena's cursor and \
         its base\n{text}"
    );
}

/// **THE CASES THAT MUST STILL BE REFUSED, AND THERE ARE THREE OF THEM BECAUSE
/// [`data_globals`] HAS THREE STATES.** Each is made by hand on the text the
/// chain really wrote, so an extractor that answered a plausible `Run` whatever
/// the text said passes the test above and fails here.
#[test]
fn a_pointer_without_its_storage_is_named_rather_than_read_as_an_initialiser() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUN_GLOBAL_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the control must itself have emitted\n{text}"
    );

    // STATE ONE — THE STORAGE IS NEVER PLACED. Delete the line that places it
    // and the data word is a pointer into nothing. **THE OBJECT'S LENGTH DOES
    // NOT MOVE**: `ॱरिक्त` is absent from the file, so this defect is invisible
    // to every rung on the ladder.
    let dangling: String = text
        .lines()
        .filter(|l| *l != "कगभण्डारॱॱ")
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(
        dangling, text,
        "the mutation must change the text, or this control tests nothing"
    );
    assert_eq!(
        data_globals(&dangling),
        vec![(
            "कग".to_string(),
            Storage::DanglingPointer("कगभण्डार".to_string())
        )],
        "a pointer whose storage this object never places is named as one — \
         NOT read as an initialiser called `कगभण्डार`, which is what the first \
         version of this file answered"
    );

    // STATE TWO — THE STORAGE IS PLACED AND TRUNCATED. Present, so a check that
    // asked only "is it there?" scores this green while every element past the
    // first writes into whatever follows it.
    let truncated = text.replace("॥ स्थानम् १०२४ ॥", "॥ स्थानम् ८ ॥");
    assert_ne!(truncated, text, "the mutation must change the text");
    assert_eq!(
        data_globals(&truncated),
        vec![(
            "कग".to_string(),
            Storage::Run {
                store: "कगभण्डार".to_string(),
                octets: 8,
            }
        )],
        "the SIZE is read, so a storage cut to one element is reported by its \
         size rather than by its presence"
    );

    // STATE THREE — A SCALAR. The same three opening lines, and the third
    // directive is the only thing that differs. Taken off the chain rather than
    // written here, so this is the two forms side by side on one instrument.
    let mut it = load();
    let (scalar, exit) = compile(&mut it, GLOBAL_WRITE_SOURCE, "क");
    assert_eq!(exit, EMITTED, "the scalar control must emit too\n{scalar}");
    assert_eq!(
        data_globals(&scalar),
        vec![("कग".to_string(), Storage::Word("०षोड्७".to_string()))],
        "and a scalar is a WORD, from the same reader, on the same tree"
    );

    // AND THE NUMERAL READER ROUND-TRIPS, so `octets: 1024` above is a number
    // this file can actually compute and not a string it happened to match.
    assert_eq!(devanagari(1024), "१०२४");
    assert_eq!(devanagari_int("१०२४"), Some(1024));
    assert_eq!(devanagari_int("कगभण्डार"), None, "a name is not a numeral");
    assert_eq!(devanagari(0), "०");
}

/// **THE STORE SIDE OF THE ADDRESS PAIR, AND THE PROGRAM THAT MAKES
/// [`repeats`] A QUESTION RATHER THAN A VERDICT.**
///
/// Every rung from 84 to 103 only READS a global, so `W-284`'s
/// `AddrOfGlobal` + `StoreAt` lowering had never been read off the `.t1` chain.
/// It puts the full address in `rd` directly and stores through it — the same
/// two opening lines as a load, and a different third.
///
/// **AND THIS SOURCE ADDRESSES ONE STORAGE TWICE, CORRECTLY.** Rung 103's test
/// asserts `repeats` is empty; this one asserts it is NOT, on a program that is
/// right. What separates them is the ROLE, which is why [`addressings`] reads
/// it.
#[test]
fn a_global_write_addresses_the_same_storage_a_second_time_and_is_correct() {
    let mut it = load();
    let (text, exit) = compile(&mut it, GLOBAL_WRITE_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "a global WRITE must reach the emitter; exit {exit}\n{text}"
    );

    let addr = addressings(&text);
    println!("METRIC t1_emitted_write_addressings {}", addr.len());
    assert_eq!(
        addr,
        vec![
            ("कग".to_string(), Role::Store),
            ("कग".to_string(), Role::Load),
        ],
        "`ग भवति ९` stores through the address and `प्रत्यागमनम् ग` loads \
         through it — one storage, two addressings, opposite directions\n{text}"
    );

    let labels = addressed_labels(&text);
    assert_eq!(
        repeats(&labels),
        vec![("कग".to_string(), 2)],
        "THE LABEL LIST ALONE CALLS THIS THE ONE-STORAGE-TWICE DEFECT. It is a \
         correct program. `repeats` names a candidate; the role decides.\n{text}"
    );

    // THE STORE'S OWN LINE, so `Role::Store` is anchored to a real instruction
    // rather than to "not a load". `निधानम् {addr}य् ०न {v}न` — the address in
    // `rd`, offset zero, and the value it writes.
    assert!(
        text.contains("स्थानसापेक्षयोगः स्थिर१म् कगॱउपरिन ।")
            && text.contains("योगः स्थिर१म् स्थिर१न कगॱअधःन ।")
            && text.contains("निधानम् स्थिर१य् ०न स्थिर०न ।"),
        "the write is an address pair into `rd` followed by a store through \
         it — `riscv64.rs:1088` and the `StoreAt` after it\n{text}"
    );

    // AND THE STORAGE IS STILL A SCALAR WORD. A write does not turn a global
    // into a run, and reading the initialiser is how this file can say so.
    assert_eq!(
        data_globals(&text),
        vec![("कग".to_string(), Storage::Word("०षोड्७".to_string()))],
        "`भवति ७` is still the initialiser; the write happens at run time\n{text}"
    );
}

/// **THE THIRD LABEL CLASS, AND THIS FILE PREDICTED THE WRONG NAME FOR IT.**
///
/// [`addressings`]'s previous margin said a source holding a string literal
/// would show `पाठकोशः` in the list. **Measured on this tree it shows `पाठ०`** —
/// the literal's own label. `पाठकोशः` is emitted, as the pool's opening label,
/// and nothing addresses it. The wrong prediction is kept in the margin above
/// because a claim repaired before anyone read it afterwards reads like a claim
/// that was always right.
///
/// The role is [`Role::Address`] and not [`Role::Load`]: a `पाठः` value IS the
/// address of its octets, so the pair is followed by no memory access at all.
#[test]
fn a_string_literal_addresses_its_own_label_and_not_the_pool() {
    let mut it = load();
    let (text, exit) = compile(&mut it, STRING_LITERAL_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "a string literal must reach the emitter; exit {exit}\n{text}"
    );

    let addr = addressings(&text);
    println!("METRIC t1_emitted_string_addressings {}", addr.len());
    assert_eq!(
        addr,
        vec![("पाठ०".to_string(), Role::Address)],
        "THE CORRECTION: the literal's own label, and the address itself is the \
         value — not `पाठकोशः`, which this file predicted\n{text}"
    );
    assert!(
        text.contains("पाठकोशःॱॱ"),
        "`पाठकोशः` IS emitted — it opens the pool — which is why predicting it \
         as the addressed label was plausible and still wrong\n{text}"
    );

    // NO GLOBAL AT ALL, ASSERTED RATHER THAN ASSUMED. This source declares none,
    // and the pool's `॥ अष्टाष्टकाः ६ ॥` sits two lines below a LABEL — so a
    // reader that matched the directive without the `॥ वैश्विकम् ॥` above it
    // would invent a global here.
    assert_eq!(
        data_globals(&text),
        Vec::new(),
        "the pool is not a global; the empty answer is the assertion\n{text}"
    );

    // THE POOL ENTRY, WITH ITS LENGTH AND ITS OCTETS — and the octets are
    // DERIVED from the source's own literal, so a chain that emitted a
    // different string fails by VALUE and not by a count.
    let literal = "अब";
    let bytes: Vec<String> = literal.bytes().map(|b| devanagari(b as u64)).collect();
    let expected = format!(
        "पाठ०शीर्षॱॱ\n॥ अष्टाष्टकाः {} ॥\nपाठ०ॱॱ\n॥ अष्टकाः {} ॥",
        devanagari(literal.len() as u64),
        bytes.join(" ")
    );
    println!("METRIC t1_emitted_string_octets {}", literal.len());
    assert!(
        text.contains(&expected),
        "the pool holds the literal's length then its UTF-8, as its own \
         source wrote it:\n{expected}\n{text}"
    );
}

// ── RUNG 103'S SCALAR, MADE FALSIFIABLE IN SECONDS ──────────────────────────
//
// **THE RUNG IS ANSWERED AND ITS ANSWER HAD NO CHEAP FALSIFIER.** Measured
// 2026-09-21 23:52Z and RE-TAKEN on this tree 2026-09-22: `tools/rung-answer.sh
// 103 स्वपरीक्षात्रिशततमी` builds the image, `yantra-run` halts
// `Finisher { value: 725627699, status: Some(11072) }`, the interpreted sink and
// the native trace are byte-identical at 914 records, and the `.err` carries no
// `StepLimit`. `(11072 << 16) | 0x3333 == 725627699`, so the finisher agrees
// with the status.
//
// ११०७२ is `११००० + <object octets>` — 72 octets, EIGHTEEN machine instructions
// at the four-octets-each the layout `सङ्केतनॱअसङ्कुचितम्` guarantees. That
// EIGHTEEN was the only unpinned number in the rung: reaching it cost a
// 35-minute image build, so the three statuses the rung's own falsifier list
// names as the INTERESTING direction —
//
// ```text
//   ११००० alone   the object came back EMPTY — the E02 shape
//   ११०४८         TWELVE instructions: the ONE-GLOBAL count
//   ११०७२         EIGHTEEN: two globals, each addressed and loaded
// ```
//
// — were distinguishable only by paying that build. All three are decided here
// off the same text the label tests already read, in under two seconds, and the
// two below ११०७२ are asserted as MEASUREMENTS of their own sources rather than
// as arithmetic about this one.
//
// WHY THAT MATTERS AND NOT ONLY THAT IT IS CHEAPER. The margin at
// `shrinkhala.t1`'s rung 103 registers ११०२४-or-thereabouts as what a CONSTANT
// FOLD looks like — a compiled emitter that answered a global's INITIALISER
// instead of loading its storage drops the three-instruction
// address-and-load sequence per global. A fold would land the count at TWELVE
// or below, which is exactly [`ONE_GLOBAL`]'s measurement, and the ladder
// cannot tell those two programs apart because it only ever sees a LENGTH.

/// The single-global source rung 103's own margin measures against — `ग` alone,
/// read once.
///
/// **THE ROW'S CONTROL, AND IT IS WHY RUNG 103 READS TWO GLOBALS AND NOT ONE.**
/// A rung on this source could not tell a symbol table with ONE reachable entry
/// from a symbol table. Two globals with DIFFERENT initialisers move the count
/// and the answer both.
const ONE_GLOBAL: &str = "मण्डलम् क ॥ सार्वजनिक चरः ग ॱॱ न६४ भवति ७ । \
सार्वजनिक वृत्तिः घ ददाति न६४ आदि प्रत्यागमनम् ग । इति";

/// [`RUNG_103_SOURCE`] with the DECLARATION of `च` deleted and the READ of it
/// left in place — the case that must still be REFUSED.
///
/// The same control rung 102's margin took by deleting its `आयातः`: it proves
/// the front half is really consulting the declarations rather than emitting
/// whatever the text mentions. It is refused at RESOLVE
/// (`सङ्कलनानिर्णयभेद`, २ — `shrinkhala.t1:124`) and answers the EMPTY run.
const UNDECLARED_GLOBAL: &str = "मण्डलम् क ॥ सार्वजनिक चरः ग ॱॱ न६४ भवति ७ । \
सार्वजनिक वृत्तिः घ ददाति न६४ आदि प्रत्यागमनम् ग योगः च । इति";

/// `सङ्कलनानिर्णयभेद` — `shrinkhala.t1:124`, "अर्थ refused it".
const RESOLVE_REFUSED: i128 = 2;

/// The status a rung of this family answers for an object of `n` octets. The
/// base is `स्वपरीक्षात्रिशततमी`'s own `प्रत्यागमनम् ११००० योगः अष्टकाः ॱ दैर्घ्य`.
const RUNG_103_BASE: u64 = 11_000;

/// Rung 102's base — `स्वपरीक्षाद्विशततमी`'s own
/// `प्रत्यागमनम् १०००० योगः अष्टकाः ॱ दैर्घ्य` (`shrinkhala.t1:2897`).
///
/// **THE BASE IS PER-RUNG AND THAT IS WHY [`rung_status`] TAKES IT.** A single
/// hard-coded ११००० would derive ११०५२ for rung 102's thirteen instructions —
/// a status no rung can answer — and the assertion would then be about this
/// file's arithmetic rather than about the emitter.
const RUNG_102_BASE: u64 = 10_000;

/// Four octets per instruction, and it is not an assumption: the layout rung 103
/// passes to `सङ्केतनॱस्थानविन्यासः` is `सङ्केतनॱअसङ्कुचितम्`, so no instruction
/// has a two-octet compressed form, and six answered rungs measured it
/// (29→११६ twice, 41→१६४, 44→८१७६, 39→९१५६, 13→१००५२).
const OCTETS_PER_INSTRUCTION: u64 = 4;

/// **THE COUNTER, AND IT HAS THREE STATES BECAUSE THE QUESTION HAS THREE.**
///
/// Returns the `.text` lines that assemble to ONE machine instruction, and —
/// separately — every non-blank line this reader does not recognise AT ALL.
///
/// A counter that answered only the first list would report a LOWER count for
/// text carrying a line shape nobody had taught it, and a low count is the
/// exact signature of the constant fold this file is here to refuse. Absence
/// through ignorance and absence through folding are one number to such a
/// reader. So the unrecognised lines come back beside the instructions and every
/// caller asserts BOTH.
///
/// The three recognised shapes, all split ON SPACES — Devanagari has no `\b`:
///
/// ```text
///   ॥ … ॥        a directive: exports, `॥ संरेखः १६ ॥`, `॥ अष्टाष्टकाः … ॥`.
///                DATA octets are this shape too, which is why a directive is
///                never counted: `.data` is not `.text`.
///   Xॱॱ          a label — ONE field, and the `ॱॱ` is part of the token.
///   … ।          an instruction.
/// ```
fn text_lines(text: &str) -> (Vec<String>, Vec<String>) {
    let mut insts = Vec::new();
    let mut unknown = Vec::new();
    for l in text.lines() {
        // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
        let f: Vec<&str> = l.split_whitespace().collect();
        match f.as_slice() {
            [] => {}
            ["॥", .., "॥"] => {}
            [one] if one.ends_with("ॱॱ") => {}
            [.., "।"] => insts.push(l.trim().to_string()),
            _ => unknown.push(l.trim().to_string()),
        }
    }
    (insts, unknown)
}

/// The status a rung with base `base` answers for text carrying `insts`
/// instructions.
fn rung_status(base: u64, insts: usize) -> u64 {
    base + OCTETS_PER_INSTRUCTION * insts as u64
}

/// **RUNG 103'S ११०७२, DERIVED FROM THE TEXT INSTEAD OF FROM A 35-MINUTE
/// IMAGE.** Eighteen instructions, no line unaccounted for.
#[test]
fn the_rung_103_source_emits_the_eighteen_instructions_its_status_counts() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_103_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the front half must reach the emitter; exit {exit} is a refusal\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    // THE UNRECOGNISED LINES FIRST, because a low instruction count caused by an
    // unread line shape must not be reported as a fold. See [`text_lines`].
    assert_eq!(
        unknown,
        Vec::<String>::new(),
        "every non-blank line of the emitted text must be a directive, a label \
         or an instruction — an unread shape makes the count below it a \
         statement about this reader and not about the emitter\n{text}"
    );

    println!("METRIC t1_rung103_text_instructions {}", insts.len());
    println!(
        "METRIC t1_rung103_derived_status {}",
        rung_status(RUNG_103_BASE, insts.len())
    );
    assert_eq!(
        insts.len(),
        18,
        "rung 103's object is 72 octets and the layout is uncompressed, so the \
         text carries EIGHTEEN instructions\n{}",
        insts.join("\n")
    );
    assert_eq!(
        rung_status(RUNG_103_BASE, insts.len()),
        11_072,
        "the derived status must be the one the image answered — measured \
         2026-09-21, `Finisher {{ value: 725627699, status: Some(11072) }}`, \
         914 trace records identical to the interpreted sink"
    );
}

/// **THE ONE-GLOBAL CONTROL MEASURES TWELVE, SO THE COUNTER IS NOT STUCK AT
/// EIGHTEEN — AND ११०४८ IS A REAL STATUS OF A REAL PROGRAM.**
///
/// The difference is SIX, which is 3 per global read
/// (`स्थानसापेक्षयोगः`, `योगः` against the low half, `आहारः`) plus one spill,
/// one reload and one `योगः` for the add. Rung 103's margin names ११०४८ as what
/// a fold or a one-entry symbol table would answer; this is the source that
/// really answers it, so the two are separated by MEASUREMENT here rather than
/// by arithmetic about a single text.
#[test]
fn the_one_global_control_measures_twelve_and_would_answer_the_folded_status() {
    let mut it = load();
    let (text, exit) = compile(&mut it, ONE_GLOBAL, "क");
    assert_eq!(
        exit, EMITTED,
        "the control must also emit; exit {exit}\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    assert_eq!(unknown, Vec::<String>::new(), "unread line shape\n{text}");

    println!("METRIC t1_one_global_text_instructions {}", insts.len());
    assert_eq!(
        insts.len(),
        12,
        "one global read is TWELVE instructions where two are eighteen\n{}",
        insts.join("\n")
    );
    assert_eq!(
        rung_status(RUNG_103_BASE, insts.len()),
        11_048,
        "and ११०४८ is rung 103's registered fold signature"
    );
    // AND IT ADDRESSES ONE STORAGE, which is the other half of why a rung on
    // this source could not see a one-entry symbol table.
    assert_eq!(addressed_labels(&text), vec!["कग".to_string()]);
    assert_eq!(
        data_globals(&text),
        vec![("कग".to_string(), Storage::Word("०षोड्७".to_string()))]
    );
}

/// **AND THE CASE THAT MUST STILL BE REFUSED: ११००० ALONE IS NOT AN ANSWER.**
///
/// The read of `च` with no declaration of it is refused at RESOLVE and the text
/// comes back EMPTY, so the derived status is ११००० — the E02 shape the rung's
/// falsifier list names first. Without this, nothing separates "the emitter
/// folded everything away" from "the front half never ran", and a reader could
/// take a refusal for a count.
#[test]
fn a_refused_source_emits_no_instructions_and_would_answer_the_base_alone() {
    let mut it = load();
    let (text, exit) = compile(&mut it, UNDECLARED_GLOBAL, "क");
    assert_eq!(
        exit, RESOLVE_REFUSED,
        "deleting a global's DECLARATION and leaving its READ must be refused \
         at RESOLVE — an exit of {exit} means the front half accepted it\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    assert_eq!(unknown, Vec::<String>::new(), "unread line shape\n{text}");
    assert_eq!(
        insts.len(),
        0,
        "a refusal emits no instructions\n{}",
        insts.join("\n")
    );
    assert_eq!(
        rung_status(RUNG_103_BASE, insts.len()),
        11_000,
        "so a refused rung answers ११००० alone and can never be read as ११०७२"
    );
}

/// **AND THE COUNTER'S THIRD STATE IS REACHABLE, DRIVEN WITHOUT THE CHAIN.**
///
/// Every text this file compiles today is wholly recognised, so the three
/// assertions above would read identically against a [`text_lines`] whose
/// `unknown` arm could never fire — and the count below an unread line shape
/// would then be reported as a fold. The mutation makes the state happen: rung
/// 103's real text with one instruction's closing `।` struck off. The line stops
/// being an instruction, and it must come back NAMED rather than vanish.
#[test]
fn an_unreadable_line_is_named_rather_than_dropped_from_the_count() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_103_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the fixture must emit before it can be mutated"
    );
    let (whole, _) = text_lines(&text);
    assert_eq!(
        whole.len(),
        18,
        "the unmutated count, restated at the mutation"
    );

    // The `.text` line the source's own add lowers to — struck of its `।` and
    // nothing else, so the ONLY difference is the shape.
    let sound = "योगः स्थिर१म् स्थिर०न स्थिर१न ।";
    assert!(
        text.contains(sound),
        "the mutation's target is in the text\n{text}"
    );
    let mutated = text.replacen(sound, "योगः स्थिर१म् स्थिर०न स्थिर१न", 1);

    let (insts, unknown) = text_lines(&mutated);
    assert_eq!(
        unknown,
        vec!["योगः स्थिर१म् स्थिर०न स्थिर१न".to_string()],
        "the unread line must be reported BY ITS TEXT — a reader that dropped \
         it would answer 17 instructions and ११०६८, which is indistinguishable \
         from a genuine fold"
    );
    assert_eq!(
        insts.len(),
        17,
        "and it is no longer counted as an instruction"
    );
}

// ── RUNG 102'S १००५२, THE TWIN HALF OF THE SAME HEADER ──────────────────────
//
// Rung 103's ११०७२ was pinned above; this is the other rung this file already
// compiles, and its status had the same shape of gap — a number the ladder
// could only reach by paying a 35-minute image build, with the margin's
// THIRTEEN standing as a dated claim rather than a measurement. It is measured
// here off the text [`the_t1_chain_names_the_callee_module_of_a_cross_module_call`]
// already reads.
//
// Measured 2026-09-21 on this tree, all three in the same 1.9 s run:
//
// ```text
//   १०००० alone   RESOLVE refused it — the E02 shape the margin names first
//   १००५२         THIRTEEN instructions: the cross-module call
//   १००५२         THIRTEEN AGAIN for a callee label of two glyphs instead of
//                 twenty-two — the ladder's standing claim, taken as a control
// ```
//
// The margin's own falsifier list says "strictly between १०००० and १००५२
// should be IMPOSSIBLE", so the only readings that separate a right answer
// from a right length are the instruction count and the label, and both are
// asserted below.

/// [`RUNG_102_SOURCE`] with the `आयातः` line DELETED and the call left in
/// place — the case that must still be REFUSED, and rung 102's own margin
/// measured it: "with that one line deleted and nothing else changed, the
/// chain REFUSES at RESOLVE".
const RUNG_102_NO_IMPORT: &str = "मण्डलम् क ॥ \
सार्वजनिक वृत्तिः घ ददाति न६४ आदि \
प्रत्यागमनम् अष्टकॱशून्याष्टकयोजनम् ७ । इति";

/// [`RUNG_102_SOURCE`] importing a module that does not exist AT ALL, with a
/// two-glyph callee — the control rung 102's margin registers as a measurement
/// and this file now takes: it "also reaches ASSEMBLE, with the SAME 13
/// instructions and the label `खच`".
///
/// **WHY IT IS HERE AND NOT PROSE.** `अष्टकशून्याष्टकयोजनम्` is twenty-two
/// glyphs and `खच` is two. If the count moved with the label, THIRTEEN would
/// be a fact about this fixture's spelling rather than about the lowering, and
/// every derived १००५२ in the ladder would be unfalsifiable. A cross-module
/// call's target is a RELOCATION the linker resolves, so its label costs the
/// `.text` nothing — that is the claim, and this is the control that holds it.
const RUNG_102_MISSING_MODULE: &str = "मण्डलम् क ॥ आयातः ख । \
सार्वजनिक वृत्तिः घ ददाति न६४ आदि \
प्रत्यागमनम् खॱच ७ । इति";

/// **RUNG 102'S १००५२, DERIVED FROM THE TEXT INSTEAD OF FROM AN IMAGE.**
/// Thirteen instructions, no line unaccounted for.
#[test]
fn the_rung_102_source_emits_the_thirteen_instructions_its_status_counts() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_102_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the cross-module source must reach the emitter; exit {exit} is a \
         refusal (shrinkhala.t1:118-154 numbers them)\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    // THE UNRECOGNISED LINES FIRST, for the reason [`text_lines`] gives: a low
    // count caused by an unread line shape must not be reported as a fold.
    assert_eq!(
        unknown,
        Vec::<String>::new(),
        "every non-blank line of the emitted text must be a directive, a label \
         or an instruction — an unread shape makes the count below it a \
         statement about this reader and not about the emitter\n{text}"
    );

    println!("METRIC t1_rung102_text_instructions {}", insts.len());
    println!(
        "METRIC t1_rung102_derived_status {}",
        rung_status(RUNG_102_BASE, insts.len())
    );
    assert_eq!(
        insts.len(),
        13,
        "rung 102's object is 52 octets and the layout is \
         `सङ्केतनॱअसङ्कुचितम्`, so the text carries THIRTEEN instructions — \
         measured 2026-09-21, and it is what the margin at \
         `shrinkhala.t1:2877` claimed\n{}",
        insts.join("\n")
    );
    assert_eq!(
        rung_status(RUNG_102_BASE, insts.len()),
        10_052,
        "the derived status must be rung 102's १००५२; a count of twelve or \
         fourteen would land १००४८ or १००५६, and the margin's falsifier list \
         rules out everything strictly between १०००० and १००५२"
    );
    // AND THE CALL IS STILL THE CALLEE'S OWN LABEL, restated at the count so
    // the two readings the rung cannot make are asserted TOGETHER: the margin
    // records that a cross-module call lowered as an intra-module one "would
    // still answer १००५२", so the count alone is not this rung's result.
    assert_eq!(
        jump_targets(&text, RA),
        vec!["अष्टकशून्याष्टकयोजनम्".to_string()],
        "thirteen instructions with the WRONG callee label is the defect the \
         status cannot see\n{text}"
    );
}

/// **AND THE LABEL COSTS THE TEXT NOTHING: TWO GLYPHS OR TWENTY-TWO, THE
/// COUNT IS THIRTEEN.**
///
/// The control [`RUNG_102_MISSING_MODULE`] documents. Without it, THIRTEEN is
/// a measurement of one spelling and the ladder's claim that the count is not
/// a function of the labels stays prose.
#[test]
fn a_cross_module_call_costs_the_same_thirteen_whatever_the_callee_is_called() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_102_MISSING_MODULE, "क");
    assert_eq!(
        exit, EMITTED,
        "importing a module that does not exist is NOT what RESOLVE checks — \
         it checks the import LIST, and LINK is what catches the rest; exit \
         {exit}\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    assert_eq!(unknown, Vec::<String>::new(), "unread line shape\n{text}");

    println!(
        "METRIC t1_rung102_short_label_text_instructions {}",
        insts.len()
    );
    // THE LABEL FIRST, because it is what makes the equal count mean anything.
    assert_eq!(
        jump_targets(&text, RA),
        vec!["खच".to_string()],
        "the call names the declared callee even though no such module \
         exists\n{text}"
    );
    assert!(
        "खच".chars().count() * 10 < "अष्टकशून्याष्टकयोजनम्".chars().count(),
        "the control only says something if the two labels differ in LENGTH by \
         an order of magnitude — 2 against 22"
    );
    assert_eq!(
        insts.len(),
        13,
        "and the count does not move: a cross-module call's target is a \
         relocation, so the label costs the `.text` nothing\n{}",
        insts.join("\n")
    );
    assert_eq!(rung_status(RUNG_102_BASE, insts.len()), 10_052);
}

/// **AND THE CASE THAT MUST STILL BE REFUSED: १०००० ALONE IS NOT AN ANSWER.**
///
/// Rung 102's own margin measured this — `आयातः` deleted, the call left in
/// place, refused at RESOLVE — and it is the first entry on the rung's
/// falsifier list ("१०००० alone — the object came back EMPTY; that is the E02
/// shape"). Without it, nothing separates a chain that never ran from an
/// emitter that folded the call away.
#[test]
fn rung_102_without_its_import_is_refused_and_would_answer_the_base_alone() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_102_NO_IMPORT, "क");
    assert_eq!(
        exit, RESOLVE_REFUSED,
        "deleting `आयातः अष्टक ।` and leaving the CALL must be refused at \
         RESOLVE — an exit of {exit} means the resolver is not consulting the \
         import list\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    assert_eq!(unknown, Vec::<String>::new(), "unread line shape\n{text}");
    assert_eq!(
        insts.len(),
        0,
        "a refusal emits no instructions\n{}",
        insts.join("\n")
    );
    assert!(
        jump_targets(&text, RA).is_empty(),
        "and no call target at all — the third state of that reader\n{text}"
    );
    assert_eq!(
        rung_status(RUNG_102_BASE, insts.len()),
        10_000,
        "so a refused rung answers १०००० alone and can never be read as १००५२"
    );
}

// ── RUNGS 99 AND 97: ONE OCTET LENGTH, TWO DIFFERENT PROGRAMS ───────────────
//
// `OCTETS_PER_INSTRUCTION`'s margin lists six answered rungs and one entry of
// that list is a COLLISION: 29 instructions twice. Rung 99 answers ६११६ and
// rung 97 answers ३११६, and both are ११६ octets of `.text` — the same count,
// from two sources that share no construct. Rung 99's own margin already flags
// the hazard ("a right length by coincidence"), and until this section nothing
// on the tree separated the two by CONTENT: swap the two rungs' sources and
// each still answers its own predicted status.
//
// They are told apart here by the two readings a LENGTH cannot make, and the
// readings are complementary rather than merely different:
//
// ```text
//   rung 97   ONE call (`कङ`, inside the object)   ZERO backward edges
//   rung 99   ZERO calls                          ONE backward edge (`कघपर्व१`)
// ```
//
// Measured 2026-09-22 on this tree, all six tests below inside one 5.6 s run
// of the whole file — no image, and no 84-to-104-minute native build, which is
// what rung 99's margin prices a walk of this ladder at.

/// Rung 99's source — `shrinkhala.t1`'s `स्वपरीक्षानवनवतिः`, `प१ ⧺ प२ ⧺ प३`.
///
/// A LOOP: `यावत्` with an accumulator, five blocks, ZERO calls. Verified
/// byte-identical to the rung's own joined pieces before it was pasted here,
/// for the reason this file's header gives — a fixture written to be *like* the
/// rung makes the green a statement about a program no rung compiles.
const RUNG_99_SOURCE: &str = "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ ददाति न६४ आदि चरः स ॱॱ न६४ भवति ० । चरः र \
ॱॱ न६४ भवति १ । यावत् र न्यूनम् ११ आदि स भवति स योगः र । र भवति र योगः १ । \
इति प्रत्यागमनम् स । इति";

/// Rung 97's source — `shrinkhala.t1`'s `स्वपरीक्षासप्तनवतिः`, `प१ ⧺ प२ ⧺ प३`.
///
/// TWO ROUTINES AND A CALL: four blocks, one call, and the callee is in the
/// SAME module — which is what separates it from rung 102, whose call is a
/// relocation. Byte-identical to the rung's joined pieces.
const RUNG_97_SOURCE: &str = "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ ददाति न६४ आदि प्रत्यागमनम् ङ २१ । इति \
सार्वजनिक वृत्तिः ङ आदाय स ॱॱ न६४ ददाति न६४ आदि प्रत्यागमनम् स योगः स । \
इति";

/// [`RUNG_99_SOURCE`] with the `इति` that closes the `यावत्` DELETED — the case
/// that must still be REFUSED, and rung 99's own margin is what names it.
///
/// That margin's claim is structural: "THREE PIECES, because this module has
/// TWO bare `इति` — one closing the `यावत्` and one closing the routine". Strike
/// the first and the routine's own `इति` is eaten by the loop, so the module
/// never closes.
///
/// **AND IT IS REFUSED AT PARSE, NOT AT RESOLVE** — measured, not assumed:
/// `सङ्कलनाघोषणाभेद` (१), "the source parsed to no declarations". Rungs 102 and
/// 103 both reach their base-alone through RESOLVE (२); this is the second road
/// to the same E02 shape, and a reader that knew only one would take a parse
/// failure for a resolver doing its job.
const RUNG_99_NO_LOOP_END: &str = "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ ददाति न६४ आदि चरः स ॱॱ न६४ भवति ० । चरः र \
ॱॱ न६४ भवति १ । यावत् र न्यूनम् ११ आदि स भवति स योगः र । र भवति र योगः १ । \
प्रत्यागमनम् स । इति";

/// [`RUNG_97_SOURCE`] with the callee routine's DECLARATION deleted and the CALL
/// left in place — the case that must still be REFUSED, rung 102's `आयातः`
/// control taken one scope inwards.
///
/// Rung 102 deletes an `आयातः` and is refused because the resolver consults the
/// IMPORT list. This deletes a routine of the module itself, so it asks whether
/// the resolver consults the module's OWN declarations for a call target.
/// Measured: `सङ्कलनानिर्णयभेद` (२), the same RESOLVE refusal.
const RUNG_97_NO_CALLEE: &str = "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ ददाति न६४ आदि प्रत्यागमनम् ङ २१ । इति";

/// Rung 99's base — `स्वपरीक्षानवनवतिः`'s own
/// `प्रत्यागमनम् ६००० योगः अष्टकाः ॱ दैर्घ्य` (`shrinkhala.t1:3125`). Its margin:
/// "a prefix no other rung uses — २०००, ३०००, ४००० and ५००० are taken".
const RUNG_99_BASE: u64 = 6_000;

/// Rung 97's base — `स्वपरीक्षासप्तनवतिः`'s own
/// `प्रत्यागमनम् ३००० योगः अष्टकाः ॱ दैर्घ्य` (`shrinkhala.t1:3205`).
const RUNG_97_BASE: u64 = 3_000;

/// `सङ्कलनाघोषणाभेद` — `shrinkhala.t1:119`, "the source parsed to no
/// declarations". NOT [`RESOLVE_REFUSED`]: see [`RUNG_99_NO_LOOP_END`].
const PARSE_REFUSED: i128 = 1;

/// **RUNG 99'S ६११६, DERIVED FROM THE TEXT, AND THE BACKWARD EDGE ITS STATUS
/// CANNOT SEE.** Twenty-nine instructions, no line unaccounted for, and the
/// loop's jump goes to a label the object has ALREADY passed.
#[test]
fn the_rung_99_source_emits_twenty_nine_instructions_and_exactly_one_backward_edge() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_99_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the loop source must reach the emitter; exit {exit} is a refusal \
         (shrinkhala.t1:118-154 numbers them)\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    // THE UNRECOGNISED LINES FIRST, for the reason [`text_lines`] gives.
    assert_eq!(
        unknown,
        Vec::<String>::new(),
        "every non-blank line of the emitted text must be a directive, a label \
         or an instruction — an unread shape makes the count below it a \
         statement about this reader and not about the emitter\n{text}"
    );
    // AND NO LABEL DEFINED TWICE, because that is what would make the direction
    // reading below indeterminate. See [`defined_labels`].
    assert_eq!(
        repeats(&defined_labels(&text)),
        Vec::<(String, usize)>::new(),
        "a label defined twice gives `earlier in the text` two answers\n{text}"
    );

    println!("METRIC t1_rung99_text_instructions {}", insts.len());
    println!(
        "METRIC t1_rung99_derived_status {}",
        rung_status(RUNG_99_BASE, insts.len())
    );
    assert_eq!(
        insts.len(),
        29,
        "rung 99's object is 116 octets and the layout is \
         `सङ्केतनॱअसङ्कुचितम्`, so the text carries TWENTY-NINE instructions — \
         the 1/5/16/0/29 `tools/demo.sh` prices `योगपर्यन्तम्.t1` at, now taken \
         off the `.t1` chain\n{}",
        insts.join("\n")
    );
    assert_eq!(
        rung_status(RUNG_99_BASE, insts.len()),
        6_116,
        "the derived status must be rung 99's ६११६; the margin's falsifier list \
         rules out everything strictly between ६००० and ६११६, so a count of 28 \
         or 30 would land ६११२ or ६१२० and overturn the prediction"
    );

    // ── AND NOW THE TWO READINGS THE LENGTH CANNOT MAKE ─────────────────────
    //
    // Rung 97 answers the SAME twenty-nine. These are what say this text is the
    // loop and not the call.
    assert_eq!(
        backward_targets(&text),
        vec!["कघपर्व१".to_string()],
        "the body must branch BACK to the test — the first backward edge any \
         rung of this ladder asked the compiled emitter to place, and rung 99's \
         margin names the exact line: `लङ्घनम् शून्यःम् कघपर्व१य् ।` at the foot \
         of पर्व२\n{text}"
    );
    assert!(
        jump_targets(&text, RA).is_empty(),
        "and ZERO calls — rung 99 is the only construct on the ladder that \
         tests control flow with no call in it at all, which is exactly what \
         its ११६ octets cannot say\n{text}"
    );
    // THE WHOLE TRANSFER LIST, not only the backward one: a reader that
    // answered the loop edge and silently dropped the forward ones would pass
    // the assertion above while seeing a program with no `यदि` in it.
    assert_eq!(
        transfers(&text)
            .iter()
            .map(|t| (t.kind, t.dir))
            .collect::<Vec<_>>(),
        vec![
            (Kind::Conditional, Dir::Forward),
            (Kind::Jump, Dir::Forward),
            (Kind::Jump, Dir::Backward),
            (Kind::Jump, Dir::Forward),
        ],
        "four transfers, no call, and the third is the loop edge. THE FIRST IS \
         THE ONLY CONDITIONAL ONE — `न्यूनलङ्घनम्`, the `यावत्`'s test — and the \
         BACK EDGE IS UNCONDITIONAL: the loop is closed by a jump and left by \
         the test, which is the opposite of the shape the name `loop edge` \
         suggests and the reason [`Kind`] had to stop calling both `Branch`\n{:#?}",
        transfers(&text)
    );
}

/// **RUNG 97'S ३११६ IS THE SAME TWENTY-NINE INSTRUCTIONS, AND THE CALL IS WHAT
/// TELLS IT FROM RUNG 99.** One call to `कङ`, resolved INSIDE the object, and
/// not one backward edge.
#[test]
fn the_rung_97_source_emits_the_same_twenty_nine_and_is_told_apart_by_its_call() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_97_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the two-routine source must reach the emitter; exit {exit}\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    assert_eq!(unknown, Vec::<String>::new(), "unread line shape\n{text}");
    assert_eq!(
        repeats(&defined_labels(&text)),
        Vec::<(String, usize)>::new(),
        "no label defined twice\n{text}"
    );

    println!("METRIC t1_rung97_text_instructions {}", insts.len());
    println!(
        "METRIC t1_rung97_derived_status {}",
        rung_status(RUNG_97_BASE, insts.len())
    );
    assert_eq!(
        insts.len(),
        29,
        "rung 97's object is 116 octets too — the 2/4/7/1/29 `tools/demo.sh` \
         prices `द्विगुणम्.t1` at\n{}",
        insts.join("\n")
    );
    assert_eq!(
        rung_status(RUNG_97_BASE, insts.len()),
        3_116,
        "the derived status must be rung 97's ३११६"
    );
    // THE COLLISION, STATED AS AN ASSERTION AND NOT AS PROSE. The two rungs
    // differ in their BASE and in nothing the text length can see.
    assert_eq!(
        rung_status(RUNG_99_BASE, insts.len()) - RUNG_99_BASE,
        rung_status(RUNG_97_BASE, insts.len()) - RUNG_97_BASE,
        "the two rungs' objects are the same length, so only the prefix \
         separates ३११६ from ६११६"
    );

    // ── AND THE READING THAT SEPARATES THEM ─────────────────────────────────
    assert_eq!(
        jump_targets(&text, RA),
        vec!["कङ".to_string()],
        "ONE call, and it names the callee's own label — `क` ⧺ `ङ`\n{text}"
    );
    assert!(
        backward_targets(&text).is_empty(),
        "and NOT ONE backward edge, where rung 99's identical length has \
         exactly one: rung 97's four blocks all flow forward\n{text}"
    );
    assert_eq!(
        transfers(&text)
            .iter()
            .map(|t| (t.kind, t.dir))
            .collect::<Vec<_>>(),
        vec![
            (Kind::Call, Dir::Forward),
            (Kind::Jump, Dir::Forward),
            (Kind::Jump, Dir::Forward),
        ],
        "one call and two exit jumps, all forward, and NOT ONE CONDITIONAL — \
         rung 97 is a routine that returns straight, and that absence is the \
         reading the ladder's table now carries\n{:#?}",
        transfers(&text)
    );
}

/// **AND `Dir::Unplaced` IS A STATE A REAL PROGRAM REACHES, NOT ONLY A
/// MUTATION.** Rung 97's call resolves inside the object; rung 102's identical
/// mnemonic does not, because its target is a relocation.
///
/// Without this the third state would be reachable only by hand, and a reader
/// whose `Unplaced` arm could not fire against any source the chain compiles
/// would report a DANGLING branch as a forward one — which is the defect a
/// `.text` length is equally blind to. The two calls are asserted TOGETHER so
/// the difference is the reading and not two separate greens.
#[test]
fn an_intra_module_call_is_placed_and_a_cross_module_one_is_not() {
    let mut it = load();

    let (near, exit) = compile(&mut it, RUNG_97_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 97 emits\n{near}");
    let (far, exit) = compile(&mut it, RUNG_102_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 102 emits\n{far}");

    let calls = |t: &str| {
        transfers(t)
            .into_iter()
            .filter(|x| x.kind == Kind::Call)
            .map(|x| (x.target, x.dir))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        calls(&near),
        vec![("कङ".to_string(), Dir::Forward)],
        "rung 97's callee is a label THIS object defines\n{near}"
    );
    assert_eq!(
        calls(&far),
        vec![("अष्टकशून्याष्टकयोजनम्".to_string(), Dir::Unplaced)],
        "rung 102's callee is a relocation — the object never defines it, and \
         that is why its label costs the `.text` nothing\n{far}"
    );
    // Both are ONE call by the same mnemonic, so `Unplaced` is not an artefact
    // of a different lowering.
    assert_eq!(
        transfers(&near)
            .iter()
            .filter(|x| x.kind == Kind::Call)
            .map(|x| x.mnemonic.clone())
            .collect::<Vec<_>>(),
        transfers(&far)
            .iter()
            .filter(|x| x.kind == Kind::Call)
            .map(|x| x.mnemonic.clone())
            .collect::<Vec<_>>(),
        "one mnemonic serves both; only the target's placement differs"
    );
}

/// **AND THE CASE THAT MUST STILL BE REFUSED FOR RUNG 99: ६००० ALONE, THROUGH
/// PARSE AND NOT THROUGH RESOLVE.**
///
/// Rung 99's margin says ६००० alone means the object came back empty. This is a
/// source that really answers it, and the refusal is [`PARSE_REFUSED`] — a road
/// to the E02 shape neither rung 102's nor rung 103's control takes.
#[test]
fn rung_99_without_the_loops_own_iti_is_refused_at_parse_and_would_answer_the_base_alone() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_99_NO_LOOP_END, "क");
    assert_eq!(
        exit, PARSE_REFUSED,
        "striking the `इति` that closes the `यावत्` leaves the module \
         unterminated, so the parse yields NO declarations — an exit of \
         {exit} means the loop swallowed the routine's close and the parser \
         accepted it\n{text}"
    );
    assert_ne!(
        PARSE_REFUSED, RESOLVE_REFUSED,
        "and it is a DIFFERENT refusal from rungs 102 and 103's, which is the \
         reason this control is worth its seconds"
    );

    let (insts, unknown) = text_lines(&text);
    assert_eq!(unknown, Vec::<String>::new(), "unread line shape\n{text}");
    assert_eq!(
        insts.len(),
        0,
        "a refusal emits no instructions\n{}",
        insts.join("\n")
    );
    assert!(
        transfers(&text).is_empty(),
        "and no transfer at all — not even the loop edge\n{text}"
    );
    assert_eq!(
        rung_status(RUNG_99_BASE, insts.len()),
        6_000,
        "so a refused rung answers ६००० alone and can never be read as ६११६"
    );
}

/// **AND THE CASE THAT MUST STILL BE REFUSED FOR RUNG 97: ३००० ALONE.**
///
/// The callee's declaration struck, the call left in place. Rung 102 proved the
/// resolver consults the IMPORT list; this proves it consults the module's own
/// routine declarations, which is what rung 97's call target actually needs.
#[test]
fn rung_97_without_its_callee_declaration_is_refused_and_would_answer_the_base_alone() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_97_NO_CALLEE, "क");
    assert_eq!(
        exit, RESOLVE_REFUSED,
        "deleting the callee ROUTINE and leaving the CALL must be refused at \
         RESOLVE — an exit of {exit} means the resolver is not consulting the \
         module's own declarations\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    assert_eq!(unknown, Vec::<String>::new(), "unread line shape\n{text}");
    assert_eq!(
        insts.len(),
        0,
        "a refusal emits no instructions\n{}",
        insts.join("\n")
    );
    assert!(
        jump_targets(&text, RA).is_empty(),
        "and no call target at all\n{text}"
    );
    assert_eq!(
        rung_status(RUNG_97_BASE, insts.len()),
        3_000,
        "so a refused rung answers ३००० alone and can never be read as ३११६"
    );
}

/// **AND THE BACKWARD EDGE CAN BE LOST WITHOUT THE COUNT MOVING, DRIVEN
/// WITHOUT THE CHAIN.**
///
/// Every text above places all its branch targets, so the assertions would read
/// identically against a [`transfers`] that classified an undefined target as
/// `Forward` — and the loop edge would then be reported as a forward jump. The
/// mutation makes the state happen: rung 99's real text with the DEFINITION of
/// `कघपर्व१` renamed and the branch to it untouched. The instruction count is
/// unchanged, which is the point — the rung still answers ६११६.
#[test]
fn a_branch_to_a_label_the_object_never_defines_is_named_rather_than_read_as_forward() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_99_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the fixture must emit before it can be mutated"
    );
    assert_eq!(
        backward_targets(&text),
        vec!["कघपर्व१".to_string()],
        "the unmutated reading, restated at the mutation"
    );

    // The label's DEFINITION line, renamed. The branch line is `कघपर्व१य्`, so
    // the `ॱॱ` suffix makes this hit the definition and nothing else.
    assert_eq!(
        text.matches("कघपर्व१ॱॱ").count(),
        1,
        "the mutation's target is defined exactly once\n{text}"
    );
    let mutated = text.replacen("कघपर्व१ॱॱ", "कघपर्व९ॱॱ", 1);

    let (insts, unknown) = text_lines(&mutated);
    assert_eq!(unknown, Vec::<String>::new(), "the shape is unchanged");
    assert_eq!(
        insts.len(),
        29,
        "and so is the count — ६११६ either way, which is why the rung cannot \
         see this"
    );
    assert!(
        backward_targets(&mutated).is_empty(),
        "the loop edge is gone"
    );
    assert_eq!(
        transfers(&mutated)
            .into_iter()
            .find(|t| t.target == "कघपर्व१")
            .map(|t| t.dir),
        Some(Dir::Unplaced),
        "and it must come back UNPLACED — a two-state reader would answer \
         `Forward` for a branch into nothing, which is a broken object \
         reported as a program\n{:#?}",
        transfers(&mutated)
    );
}

// ── RUNG 98: A BRANCH, AND THE BACKWARD EDGE THAT IS NOT A LOOP ─────────────
//
// Cycle 904 separated rungs 99 and 97 — the ladder's one colliding octet
// length — by BRANCH DIRECTION: rung 99 has exactly one backward edge and rung
// 97 has none. **THE VERY NEXT RUNG COLLIDES WITH RUNG 99 ON THAT NEW
// READING.** Rung 98 is a `यदि` with both arms returning and it has exactly one
// backward edge too, so `backward_targets` answers a one-element list for both
// — and one of the two programs is a loop while the other's jump sits in code
// nothing reaches.
//
// Measured 2026-09-22 on this tree, the whole file in one 5.6 s run, no image
// and no 84-to-104-minute native build:
//
// ```text
//   rung 99   backward कघपर्व१   its block is REACHABLE — the loop
//   rung 98   backward कङपर्व४   its block is कङपर्व५, which nothing reaches
// ```
//
// So the reading this section adds is REACHABILITY, and it is the first one on
// this tree that separates an instruction the processor can execute from an
// instruction that is only in the file. Rung 98 is also the first rung whose
// object carries one: FOUR of its 164 octets are in a dead block, and its
// status counts them exactly as it counts the other 160.
//
// **AND THE LEDGER'S TWO PREDICTIONS FOR THIS RUNG WERE BOTH WRONG, CORRECTED
// BY MEASUREMENT AND KEPT HERE BECAUSE A PREDICTION THAT IS QUIETLY DROPPED
// READS AFTERWARDS LIKE ONE THAT WAS NEVER MADE:**
//
// ```text
//   predicted  ZERO backward edges, since both arms return and every block exits
//   measured   ONE — the join the arms never fall into jumps BACK to the else arm
//
//   predicted  SEVEN blocks, from `tools/demo.sh` over `spec/demo/महत्तमम्.t1`
//   measured   NINE labels, of which TWO are empty; seven non-empty
// ```
//
// The second is not a disagreement once both instruments say enough — see
// [`the_block_count_the_two_instruments_disagree_on_is_the_empty_blocks`],
// which takes the same reading against rungs 99 and 97 and lands on the five
// and the four their own margins claim.

/// Rung 98's source — `shrinkhala.t1`'s `स्वपरीक्षाष्टनवतिः`, `प१ ⧺ प२ ⧺ प३ ⧺ प४`.
///
/// A BRANCH: two routines, a `यदि` whose taken arm and whose fall-through BOTH
/// return, and one intra-module call. Verified byte-identical to the rung's own
/// four joined pieces before it was pasted here — the rung has THREE bare `इति`
/// and each piece is split `इ` + `ति` so none terminates the literal early.
const RUNG_98_SOURCE: &str = "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ ददाति न६४ आदि प्रत्यागमनम् ङ आरभ्य ३ ऽ ८ समाप्तम् । इति \
सार्वजनिक वृत्तिः ङ आदाय व ॱॱ न६४ ऽ द ॱॱ न६४ ददाति न६४ आदि यदि व अधिकम् द आदि प्रत्यागमनम् व । \
इति प्रत्यागमनम् द । इति";

/// [`RUNG_98_SOURCE`] with the `इति` that closes the `यदि` DELETED — the case
/// that must still be REFUSED.
///
/// Rung 98's margin names ४००० alone as the E02 shape and says nothing about
/// which road reaches it, and cycle 904 measured that the ladder has at least
/// two: rungs 102 and 103 refuse at RESOLVE, rung 99 at PARSE. **MEASURED, NOT
/// ASSUMED: this is [`PARSE_REFUSED`] — and so are the other two strikes.** All
/// three of rung 98's bare `इति` were struck in turn and every one answered `१`,
/// which is the reason the assertion below is worth its seconds: a rung whose
/// only control took the other road would have left the claim untested.
const RUNG_98_NO_YADI_END: &str = "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ ददाति न६४ आदि प्रत्यागमनम् ङ आरभ्य ३ ऽ ८ समाप्तम् । इति \
सार्वजनिक वृत्तिः ङ आदाय व ॱॱ न६४ ऽ द ॱॱ न६४ ददाति न६४ आदि यदि व अधिकम् द आदि प्रत्यागमनम् व । \
प्रत्यागमनम् द । इति";

/// Rung 98's base — `स्वपरीक्षाष्टनवतिः`'s own
/// `प्रत्यागमनम् ४००० योगः अष्टकाः ॱ दैर्घ्य` (`shrinkhala.t1:3164`).
const RUNG_98_BASE: u64 = 4_000;

/// **RUNG 98'S ४१६४, DERIVED FROM THE TEXT, AND FOUR OF ITS OCTETS ARE IN A
/// BLOCK NOTHING REACHES.** Forty-one instructions, no line unaccounted for,
/// one intra-module call, and one backward edge that is not a loop.
#[test]
fn the_rung_98_source_emits_the_forty_instructions_its_status_counts() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_98_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the branch source must reach the emitter; exit {exit} is a refusal \
         (shrinkhala.t1:118-154 numbers them)\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    // THE UNRECOGNISED LINES FIRST, for the reason [`text_lines`] gives.
    assert_eq!(
        unknown,
        Vec::<String>::new(),
        "every non-blank line of the emitted text must be a directive, a label \
         or an instruction — an unread shape makes the count below it a \
         statement about this reader and not about the emitter\n{text}"
    );
    assert_eq!(
        repeats(&defined_labels(&text)),
        Vec::<(String, usize)>::new(),
        "a label defined twice gives `earlier in the text` two answers\n{text}"
    );

    println!("METRIC t1_rung98_text_instructions {}", insts.len());
    println!(
        "METRIC t1_rung98_derived_status {}",
        rung_status(RUNG_98_BASE, insts.len())
    );
    assert_eq!(
        insts.len(),
        40,
        "rung 98's object is 160 octets and the layout is \
         `सङ्केतनॱअसङ्कुचितम्`, so the text carries FORTY instructions. It carried \
         FORTY-ONE — the 2/7/12/1/41 `tools/demo.sh` prices `महत्तमम्.t1` at — \
         until `W-279` stopped the `यदि` arm writing its join लङ्घन into the \
         block a returning arm had already walked away from. The RUST chain \
         still places that instruction, so this is the first figure on which \
         the two chains disagree, and the `.t1` one is the shorter\n{}",
        insts.join("\n")
    );
    assert_eq!(
        rung_status(RUNG_98_BASE, insts.len()),
        4_160,
        "the derived status must be ४१६०; a count of 39 or 41 would land ४१५६ \
         or ४१६४, and ४१६४ is what this rung answered before `W-279` — so the \
         old pin is the one number a regression would most plausibly restore"
    );

    // ── THE WHOLE TRANSFER LIST, WITH THE BLOCK EACH ONE SITS IN ────────────
    //
    // Every entry is FORWARD. Until `W-279` there was one more, a backward edge
    // out of कङपर्व५ that nothing reached; the reachability reading below now
    // finds that block empty rather than carrying a jump.
    assert_eq!(
        transfers(&text)
            .iter()
            .map(|t| (t.kind, t.dir, t.from.clone()))
            .collect::<Vec<_>>(),
        vec![
            (Kind::Call, Dir::Forward, "कघ".to_string()),
            (Kind::Jump, Dir::Forward, "कघ".to_string()),
            (Kind::Conditional, Dir::Forward, "कङ".to_string()),
            (Kind::Jump, Dir::Forward, "कङ".to_string()),
            (Kind::Jump, Dir::Forward, "कङपर्व३".to_string()),
            (Kind::Jump, Dir::Forward, "कङपर्व४".to_string()),
        ],
        "one call, ONE CONDITIONAL — the `यदि`'s `न्यूनलङ्घनम्` out of कङ — and \
         four unconditional jumps. There was a SIXTH transfer: a BACKWARD edge \
         out of कङपर्व५, the join the two returning arms never fell into, and \
         `W-279` stopped emitting it; कङपर्व५ is now empty\n{:#?}",
        transfers(&text)
    );
    assert_eq!(
        jump_targets(&text, RA),
        vec!["कङ".to_string()],
        "ONE call, and it names the callee's own label — `क` ⧺ `ङ`\n{text}"
    );

    // ── AND THE READING A LENGTH CANNOT MAKE: WHAT IS NEVER EXECUTED ────────
    let (live, dead) = reachability(&text);
    assert_eq!(
        dead,
        vec!["कघपर्व१".to_string(), "कङपर्व५".to_string()],
        "two of the eight blocks are unreachable and BOTH ARE EMPTY: the block \
         `कघ` opens after its return, and the join कङपर्व५ the `यदि`'s returning \
         arm never falls into. There were THREE of nine — `W-279` discarded the \
         block the return opened inside the arm, and emptied the join\n{:#?}",
        blocks(&text)
    );
    assert_eq!(
        live.len() + dead.len(),
        blocks(&text).len(),
        "every block is one or the other"
    );
    let dead_insts: usize = blocks(&text)
        .iter()
        .filter(|b| dead.contains(&b.label))
        .map(|b| b.insts.len())
        .sum();
    println!("METRIC t1_rung98_unreachable_instructions {dead_insts}");
    assert_eq!(
        dead_insts,
        0,
        "and NOT ONE of the forty instructions is in a block nothing reaches. \
         This was ONE — four octets of rung 98's 164 that its status counted \
         exactly as it counted the other 160 — and `W-279` is why it is zero; \
         the two dead blocks that remain are both empty\n{:#?}",
        blocks(&text)
    );
}

/// **THE 98/99 COLLISION IS GONE, AND WHAT DISSOLVED IT WAS A FIX TO THE
/// EMITTER AND NOT A NEW READING.**
///
/// Cycle 904 added the backward-edge reading to break the ११६-octet collision
/// between rungs 99 and 97; cycle 905 added reachability to break the collision
/// THAT reading had — rung 98 and rung 99 each answered a one-element backward
/// list, and only one of the two objects could ever take its edge.
///
/// `W-279` removed rung 98's edge. It was the join लङ्घन the `यदि` arm wrote
/// into the block its returning arm had already walked away from, so the
/// reading that separated the pair is now separating nothing: rung 98 answers
/// the EMPTY backward list, like rung 97, and rung 99's edge is the only one
/// left on this half of the ladder. Kept — and renamed to what it now measures
/// — because the pair is the reason `live_backward_targets` exists, and a
/// reader deciding whether to keep that instrument needs the history in one
/// place rather than in a deleted test.
#[test]
fn rung_98_no_longer_carries_the_dead_backward_edge_that_rung_99s_live_one_was_read_against() {
    let mut it = load();
    let (branch, exit) = compile(&mut it, RUNG_98_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 98 emits\n{branch}");
    let (loop_, exit) = compile(&mut it, RUNG_99_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 99 emits\n{loop_}");

    // THE COLLISION THAT WAS, STATED AS AN ASSERTION AND NOT AS PROSE: it is
    // the BRANCH's edge that went, and the LOOP's that stayed.
    assert_eq!(
        backward_targets(&branch),
        Vec::<String>::new(),
        "rung 98 now has no backward edge at all — the one it had left \
         कङपर्व५, a block nothing reaches, and `W-279` stopped writing it\n{branch}"
    );
    assert_eq!(
        backward_targets(&loop_),
        vec!["कघपर्व१".to_string()],
        "rung 99 still has exactly one, so the two no longer collide in the \
         cycle-904 reading either\n{loop_}"
    );

    // AND RUNG 99'S IS STILL LIVE — the claim reachability was added to make,
    // and the half of this pair that a fix to the emitter must not touch.
    assert_eq!(
        live_backward_targets(&loop_),
        vec!["कघपर्व१".to_string()],
        "rung 99's edge is the `यावत्`'s own, and its block is on the path from \
         the routine's entry — the object really loops\n{loop_}"
    );
    assert!(
        live_backward_targets(&branch).is_empty(),
        "and rung 98 has none to be live, which it answered before the fix too \
         — so this reading alone cannot tell the two programs apart\n{branch}"
    );
    // THE CASE THAT MUST STILL BE REFUSED: the separation now rests on the
    // BACKWARD COUNT again, exactly as it did before cycle 905, and rung 97 is
    // where that road ran out. Named so the next reader does not take the green
    // above for the pair still being separated by reachability.
    assert_eq!(
        (
            backward_targets(&branch).len(),
            live_backward_targets(&branch).len()
        ),
        (0, 0),
        "rung 98 reads all-zero for backward edges — the SAME reading as rung \
         97, which is the collision cycle 904 broke with this instrument and \
         which only the instruction count now breaks\n{branch}"
    );
}

/// **THE BLOCK COUNT THE TWO INSTRUMENTS DISAGREE ON IS EXACTLY THE EMPTY
/// BLOCKS, MEASURED ON FIVE RUNGS — AND THE DISCOUNT THAT REMAINS IS PER
/// RETURNING ARM, NOT PER `यदि`.**
///
/// `tools/demo.sh` — the RUST chain — prices these five sources at seven,
/// seven, five, four and EIGHT blocks, and each rung's margin carries that
/// number. The first claim of this test is unchanged and is about the EMPTY
/// blocks: the emitter opens a fresh block after every `प्रत्यागमनम्` and writes
/// its label even when nothing is ever put in it, so the two instruments cannot
/// be compared until those are set aside.
///
/// **THE RULE HELD AND THE NUMBERS MOVED — `W-279`.** Rungs 98 and 100 read
/// NINE-with-two-empty until the `यदि` arm stopped opening a block after an arm
/// that had already returned (`478350f3`). They now read EIGHT-with-two-empty,
/// so the remainder falls from seven to SIX and the `.t1` chain places one
/// block fewer than the Rust one.
///
/// **THAT LEFT A SECOND RULE STATED ON EVIDENCE THAT COULD NOT SEPARATE IT
/// FROM ITS RIVAL, AND RUNG 101 IS THE RUNG THAT SEPARATES THEM.** The wording
/// this test carried was "ONE BLOCK FEWER on exactly the sources that branch" —
/// a discount of one per `यदि`. Rungs 98 and 100 each have exactly ONE branch
/// with exactly ONE returning arm, so per-`यदि` and per-returning-arm predict
/// the same number on both, and neither rung can tell them apart. Rung 101 —
/// two arms, EACH of which returns — is the first source on the ladder where
/// the two readings disagree, and the per-`यदि` reading predicts seven.
/// **MEASURED: 8 labels, 2 empty, remainder SIX — two fewer than the Rust
/// chain's eight, one fewer than the per-`यदि` rule allows.** The discount is
/// one block per ARM THAT RETURNS, and [`BLOCK_DISCOUNT_IS_PER_RETURNING_ARM`]
/// below states it over all five rungs at once.
///
/// This is a rule fit on five points of which exactly ONE is out of sample for
/// it, which is said plainly rather than dressed as five confirmations: 97 and
/// 99 have no branch and contribute a zero each, 98 and 100 are the two the old
/// wording was fit to, and 101 alone is the measurement that could have refuted
/// it and did not. A sixth rung with a two-armed branch where only ONE arm
/// returns would be the next real test, and no such source is on the ladder.
///
/// Stated as one assertion over five sources rather than five greens, because
/// what is being claimed is the RULE and a single rung cannot carry it.
#[test]
fn the_block_count_the_two_instruments_disagree_on_is_the_empty_blocks() {
    let mut it = load();
    let mut measured = Vec::new();
    for (rung, src) in [
        (100, RUNG_100_SOURCE),
        (98, RUNG_98_SOURCE),
        (99, RUNG_99_SOURCE),
        (97, RUNG_97_SOURCE),
        (101, RUNG_101_SOURCE),
    ] {
        let (text, exit) = compile(&mut it, src, "क");
        assert_eq!(exit, EMITTED, "rung {rung} emits\n{text}");
        let bs = blocks(&text);
        let empty = bs.iter().filter(|b| b.insts.is_empty()).count();
        println!("METRIC t1_rung{rung}_text_blocks {}", bs.len());
        println!("METRIC t1_rung{rung}_empty_blocks {empty}");
        measured.push((rung, bs.len(), empty, bs.len() - empty));
    }
    assert_eq!(
        measured,
        vec![
            (100, 8, 2, 6),
            (98, 8, 2, 6),
            (99, 6, 1, 5),
            (97, 6, 2, 4),
            (101, 8, 2, 6),
        ],
        "rung, labels emitted, empty ones, and the remainder. Rungs 99 and 97 \
         answer the FIVE and FOUR their margins claim from `tools/demo.sh` \
         over `योगपर्यन्तम्.t1` and `द्विगुणम्.t1`, and they are the two sources \
         with no `यदि`. Rungs 100 and 98 answered SEVEN, matching \
         `क्रमगुणितम्.t1` and `महत्तमम्.t1`, until `W-279` discarded the block a \
         `यदि` arm opens after a `प्रत्यागमनम्` it can never come back from. \
         RUNG 101 IS THE ROW THAT PRICES THAT DISCOUNT: its margin claims \
         EIGHT and the remainder is SIX, so the discount is TWO on a source \
         with two returning arms — see this test's margin"
    );

    // THE SEPARATION SAID AS A CLAIM, not left implicit in the tuple above, so
    // that a later cycle which changes either instrument reds HERE with the
    // rule named rather than only on a number.
    let remainder = |r: u64| {
        measured
            .iter()
            .find(|(rung, ..)| *rung == r)
            .unwrap_or_else(|| panic!("rung {r} is measured above"))
            .3
    };
    assert_eq!(
        BLOCK_DISCOUNT_IS_PER_RETURNING_ARM
            .iter()
            .map(|&(rung, rust_blocks, returning_arms)| (
                rung,
                remainder(rung),
                rust_blocks - returning_arms
            ))
            .collect::<Vec<_>>(),
        BLOCK_DISCOUNT_IS_PER_RETURNING_ARM
            .iter()
            .map(|&(rung, rust_blocks, returning_arms)| (
                rung,
                rust_blocks - returning_arms,
                rust_blocks - returning_arms
            ))
            .collect::<Vec<_>>(),
        "rung, the remainder MEASURED above, and the remainder PREDICTED by \
         `rust_blocks - returning_arms`. The Rust block counts are each rung's \
         own margin in `shrinkhala.t1` and the arm counts are read off the \
         `RUNG_*_SOURCE` fixtures in this file — see \
         [`BLOCK_DISCOUNT_IS_PER_RETURNING_ARM`]"
    );

    // AND THE READING THIS REPLACED IS PINNED AS REFUSED. A discount of one per
    // `यदि` predicts SEVEN for rung 101; the rung answers six. If this ever
    // reds, the emitter has stopped discarding one of the two join blocks — say
    // which arm got its block back and take the margin above with it; do not
    // delete this line.
    assert_ne!(
        remainder(101),
        7,
        "rung 101 under the per-`यदि` reading this test used to carry. It has \
         TWO returning arms and answers 8 - 2 = 6, which is the whole of the \
         evidence separating per-arm from per-branch"
    );
}

/// Rung, the block count its `shrinkhala.t1` margin reads off `tools/demo.sh` —
/// the RUST chain — and the number of BRANCH ARMS THAT RETURN in the rung's
/// source, counted from the `RUNG_*_SOURCE` fixture above and not from the
/// emitter's output.
///
/// Rungs 97 and 99 have no `यदि` at all and contribute a zero each. Rung 98's
/// `यदि` has no `अन्यथा` and its one arm returns; rung 100's is the same shape.
/// **Rung 101 is the only entry with two, and so the only entry that can tell
/// a per-arm discount from a per-branch one.**
const BLOCK_DISCOUNT_IS_PER_RETURNING_ARM: [(u64, usize, usize); 5] =
    [(100, 7, 1), (98, 7, 1), (99, 5, 0), (97, 4, 0), (101, 8, 2)];

/// **AND THE CASE THAT MUST STILL BE REFUSED FOR RUNG 98: ४००० ALONE, THROUGH
/// PARSE.**
///
/// The `इति` that closes the `यदि` struck, so the routine's own close is eaten
/// by the branch and the module never terminates. Measured rather than assumed
/// — see [`RUNG_98_NO_YADI_END`], where all three of the rung's bare `इति` were
/// struck in turn and all three answered [`PARSE_REFUSED`].
#[test]
fn rung_98_without_the_yadis_own_iti_is_refused_at_parse_and_would_answer_the_base_alone() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_98_NO_YADI_END, "क");
    assert_eq!(
        exit, PARSE_REFUSED,
        "striking the `इति` that closes the `यदि` leaves the module \
         unterminated, so the parse yields NO declarations — an exit of {exit} \
         means the branch swallowed the routine's close and the parser \
         accepted it\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    assert_eq!(unknown, Vec::<String>::new(), "unread line shape\n{text}");
    assert_eq!(
        insts.len(),
        0,
        "a refusal emits no instructions\n{}",
        insts.join("\n")
    );
    assert!(
        blocks(&text).is_empty(),
        "and not one block, so there is nothing for reachability to read\n{text}"
    );
    assert_eq!(
        rung_status(RUNG_98_BASE, insts.len()),
        4_000,
        "so a refused rung answers ४००० alone and can never be read as ४१६४"
    );
}

/// **AND A BLOCK REACHED ONLY BY FALLING INTO IT IS NOT DEAD, DRIVEN WITHOUT
/// THE CHAIN.**
///
/// Every assertion above would read identically against a [`reachability`] that
/// followed only BRANCH targets and ignored the fallthrough edge: no source
/// this file compiles reaches कङपर्व५ at all, so its `dead` verdict would be
/// right for the wrong reason, and the reader would then call rung 98's live
/// arms dead too — [`live_backward_targets`] would answer the empty list for
/// every program ever written, including a loop.
///
/// The mutation makes the state happen. Rung 98's real text with the
/// unconditional jump at the foot of कङपर्व४ struck: the block now FALLS into
/// कङपर्व५. One instruction fewer — ४१५६ — which is the point, because the
/// rung's status moves by the same four octets whether the jump was removed or
/// never placed.
///
/// **THE MUTATION USED TO MAKE THE OBJECT LOOP FOREVER AND NO LONGER DOES —
/// `W-279`.** कङपर्व५ carried a backward जump to कङपर्व४ that nothing reached;
/// falling into it closed a cycle. The fix emptied कङपर्व५, so the mutated
/// object now falls through it into `कङनिर्गम` and returns the wrong register.
/// The reader's case is UNCHANGED — कङपर्व५ is still reached by fallthrough and
/// by no branch at all — which is the whole of what this test drives.
#[test]
fn a_block_reached_only_by_fallthrough_is_not_read_as_dead() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_98_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the fixture must emit before it can be mutated"
    );

    // The jump line immediately above कङपर्व५'s definition, and nothing else:
    // the `ॱॱ` suffix makes the anchor the DEFINITION rather than a branch.
    let cut = "लङ्घनम् शून्यःम् कङनिर्गमय् ।\nकङपर्व५ॱॱ";
    assert_eq!(
        text.matches(cut).count(),
        1,
        "the mutation's site occurs exactly once\n{text}"
    );
    let mutated = text.replacen(cut, "कङपर्व५ॱॱ", 1);

    let (insts, unknown) = text_lines(&mutated);
    assert_eq!(unknown, Vec::<String>::new(), "the shape is unchanged");
    assert_eq!(
        insts.len(),
        39,
        "one instruction fewer, so the rung would answer ४१५६ — a status four \
         octets below the truth, from an object that returns the wrong register"
    );
    assert_eq!(
        rung_status(RUNG_98_BASE, insts.len()),
        4_156,
        "and ४१५६ is a status the margin's falsifier list does not rule out"
    );

    let (_, dead) = reachability(&mutated);
    assert_eq!(
        dead,
        vec!["कघपर्व१".to_string()],
        "कङपर्व५ is reached by FALLTHROUGH and by no branch at all — a reader \
         that followed only branch targets would still call it dead, and would \
         call every loop body on this tree dead with it\n{:#?}",
        blocks(&mutated)
    );
    // AND THE UNMUTATED TEXT IS WHERE कङपर्व५ IS DEAD, so the pair above and
    // below is the reading moving and not the block being absent either way.
    assert_eq!(
        reachability(&text).1,
        vec!["कघपर्व१".to_string(), "कङपर्व५".to_string()],
        "unmutated, कङपर्व५ is dead — nothing branches to it and the block \
         above ENDS with its jump to `कङनिर्गम`\n{text}"
    );
}

// ── RUNG 100: RECURSION, AND THE THIRD SHAPE OF BACKWARD EDGE ────────────────
//
// **THE LADDER'S COLLISION MOVED AGAIN, AND THIS IS WHERE IT LANDED.** Each
// reading this file has added was added because two rungs answered the same
// thing, and each new reading has then collided on the very next rung:
//
// ```text
//   cycle 904   rungs 97 and 99 both answer ११६ octets
//               → broken by BACKWARD EDGE COUNT: 99 has one, 97 has none
//   cycle 905   rungs 98 and 99 both answer ONE backward edge
//               → broken by REACHABILITY: 99's block is live, 98's is dead
//   this cycle  rungs 99 and 100 both answer ONE *LIVE* backward edge
//               → broken by KIND: 99's is a BRANCH, 100's is a CALL
// ```
//
// So rung 100's edge is the THIRD shape of backward edge on this ladder, after
// rung 99's live branch and rung 98's dead one, and the reading that names it
// is `Transfer::kind` — the link register, which `transfers` has carried since
// this file was written and which no rung had yet needed.
//
// **AND THE DEFECT THIS RUNG'S STATUS CANNOT SEE IS THE ONE WORTH THE SECONDS.**
// See [`a_recursive_call_lowered_as_a_jump_is_named_rather_than_counted`]: a
// self-call lowered as a plain jump is the SAME FORTY-THREE INSTRUCTIONS and
// the same ८१७२, the object never returns, and `live_backward_targets` — the
// cycle-905 reading — still answers `["कङ"]`. Only the kind moves.

/// Rung 100's source — `shrinkhala.t1`'s `स्वपरीक्षाशततमी`, `प१ ⧺ प२ ⧺ प३ ⧺ प४`.
///
/// RECURSION: two routines, and the second calls ITSELF with a smaller
/// argument — the only source on `7319bd11`'s table with TWO calls. Verified
/// byte-identical (594 octets) to the rung's own four joined pieces before it
/// was pasted here, by an extractor first checked against [`RUNG_98_SOURCE`]'s
/// 533. The rung has THREE bare `इति` — one closing `घ`, one closing the `यदि`,
/// one closing `ङ` — and each piece is split `इ` + `ति` so none terminates the
/// literal early.
const RUNG_100_SOURCE: &str = "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ ददाति न६४ आदि प्रत्यागमनम् ङ ५ । इति \
सार्वजनिक वृत्तिः ङ आदाय स ॱॱ न६४ ददाति न६४ आदि यदि स न्यूनम् २ आदि प्रत्यागमनम् १ । \
इति प्रत्यागमनम् स गुणनम् आरभ्य ङ आरभ्य स वियोगः १ समाप्तम् समाप्तम् । इति";

/// [`RUNG_100_SOURCE`] with the `इति` that closes the RECURSIVE ROUTINE `ङ`
/// deleted — the case that must still be REFUSED.
///
/// Rung 100's margin names ८००० alone as the E02 shape and says nothing about
/// which road reaches it, and the ladder has at least two: rungs 102 and 103
/// refuse at RESOLVE, rungs 98 and 99 at PARSE. **MEASURED, NOT ASSUMED: all
/// THREE of this rung's bare `इति` were struck in turn and every one answered
/// [`PARSE_REFUSED`] with zero instructions**, so ८००० is reached by rung 98's
/// road and not rungs 102/103's.
const RUNG_100_NO_ROUTINE_END: &str = "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ ददाति न६४ आदि प्रत्यागमनम् ङ ५ । इति \
सार्वजनिक वृत्तिः ङ आदाय स ॱॱ न६४ ददाति न६४ आदि यदि स न्यूनम् २ आदि प्रत्यागमनम् १ । \
इति प्रत्यागमनम् स गुणनम् आरभ्य ङ आरभ्य स वियोगः १ समाप्तम् समाप्तम् ।";

/// Rung 100's base — `स्वपरीक्षाशततमी`'s own
/// `प्रत्यागमनम् ८००० योगः अष्टकाः ॱ दैर्घ्य` (`shrinkhala.t1:3065`).
const RUNG_100_BASE: u64 = 8_000;

/// **RUNG 100'S ८१७६, DERIVED FROM THE TEXT, AND THE PREDICTION THAT THE
/// EMPTY-BLOCK RULE MADE ONE CYCLE AHEAD OF IT HOLDS.**
///
/// Forty-four instructions, no line unaccounted for, TWO calls to one label,
/// and a backward edge that is a CALL. Cycle 905 landed the rule that the `.t1`
/// emitter writes a label for the block it opens after every `प्रत्यागमनम्`
/// even when nothing lands in it; rung 100's margin claims SEVEN blocks from
/// the Rust chain, so that rule predicted NINE emitted labels with TWO empty
/// before this source was ever compiled here. It is nine and two.
#[test]
fn the_rung_100_source_emits_the_forty_three_instructions_its_status_counts() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_100_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the recursive source must reach the emitter; exit {exit} is a refusal \
         (shrinkhala.t1:118-154 numbers them)\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    // THE UNRECOGNISED LINES FIRST, for the reason [`text_lines`] gives.
    assert_eq!(
        unknown,
        Vec::<String>::new(),
        "every non-blank line of the emitted text must be a directive, a label \
         or an instruction — an unread shape makes the count below it a \
         statement about this reader and not about the emitter\n{text}"
    );
    assert_eq!(
        repeats(&defined_labels(&text)),
        Vec::<(String, usize)>::new(),
        "a label defined twice gives `earlier in the text` two answers\n{text}"
    );

    println!("METRIC t1_rung100_text_instructions {}", insts.len());
    println!(
        "METRIC t1_rung100_derived_status {}",
        rung_status(RUNG_100_BASE, insts.len())
    );
    assert_eq!(
        insts.len(),
        43,
        "rung 100's object is 172 octets and the layout is \
         `सङ्केतनॱअसङ्कुचितम्`, so the text carries FORTY-THREE instructions. It \
         carried FORTY-FOUR — the 2/7/14/2/44 `tools/demo.sh` prices \
         `क्रमगुणितम्.t1` at — until `W-279` stopped the `यदि` arm writing its \
         join लङ्घन into the block its returning arm had walked away from\n{}",
        insts.join("\n")
    );
    assert_eq!(
        rung_status(RUNG_100_BASE, insts.len()),
        8_172,
        "the derived status must be ८१७२; a count of 42 or 44 would land ८१६८ \
         or ८१७६, and ८१७६ is what this rung answered before `W-279` — the old \
         pin is the number a regression would most plausibly restore"
    );

    // ── THE WHOLE TRANSFER LIST, WITH KIND, DIRECTION AND BLOCK ─────────────
    //
    // The SIXTH entry is the finding: a BACKWARD edge whose kind is `Call`.
    // Nothing below rung 100 on this ladder emits one. There was an EIGHTH — a
    // backward branch out of कङपर्व५, rung 98's dead edge, which this object
    // carried too — and `W-279` stopped emitting it.
    assert_eq!(
        transfers(&text)
            .iter()
            .map(|t| (t.kind, t.dir, t.from.clone()))
            .collect::<Vec<_>>(),
        vec![
            (Kind::Call, Dir::Forward, "कघ".to_string()),
            (Kind::Jump, Dir::Forward, "कघ".to_string()),
            (Kind::Conditional, Dir::Forward, "कङ".to_string()),
            (Kind::Jump, Dir::Forward, "कङ".to_string()),
            (Kind::Jump, Dir::Forward, "कङपर्व३".to_string()),
            (Kind::Call, Dir::Backward, "कङपर्व४".to_string()),
            (Kind::Jump, Dir::Forward, "कङपर्व४".to_string()),
        ],
        "TWO calls — the forward one `घ` makes and the BACKWARD one `ङ` makes \
         to its own entry — ONE conditional, the recursion's own base test, and \
         four unconditional jumps. The BACKWARD CALL is the one edge on this \
         rung nothing else on the ladder has, and it is the one `W-279` must \
         not touch\n{:#?}",
        transfers(&text)
    );

    // AND THE CALL TARGET IS THE SAME LABEL TWICE, WHICH IS CORRECT HERE.
    //
    // Everywhere else in this file a repeat is the defect
    // [`a_duplicated_address_is_named_rather_than_counted`] names: one storage
    // addressed where the source says two. A repeat in the CALL list is the
    // opposite — it is what recursion looks like in the emitted text, and a
    // checker that scored `repeats` non-empty as a failure without asking which
    // list it read would refuse the only correct lowering of this rung.
    assert_eq!(
        jump_targets(&text, RA),
        vec!["कङ".to_string(), "कङ".to_string()],
        "two calls, both naming `क` ⧺ `ङ` — the second is `ङ` calling \
         ITSELF\n{text}"
    );
    assert_eq!(
        repeats(&jump_targets(&text, RA)),
        vec![("कङ".to_string(), 2)],
        "and the repeat is NAMED rather than left implicit in a count of two"
    );

    // ── AND THE READING A LENGTH CANNOT MAKE: WHAT IS NEVER EXECUTED ────────
    let (live, dead) = reachability(&text);
    assert_eq!(
        dead,
        vec!["कघपर्व१".to_string(), "कङपर्व५".to_string()],
        "two of the eight blocks are unreachable and both are EMPTY — exactly \
         rung 98's two, because rung 100's `यदि` has the same one-armed \
         shape\n{:#?}",
        blocks(&text)
    );
    assert_eq!(
        live.len() + dead.len(),
        blocks(&text).len(),
        "every block is one or the other"
    );
    let dead_insts: usize = blocks(&text)
        .iter()
        .filter(|b| dead.contains(&b.label))
        .map(|b| b.insts.len())
        .sum();
    println!("METRIC t1_rung100_unreachable_instructions {dead_insts}");
    assert_eq!(
        dead_insts,
        0,
        "and NOT ONE of the forty-three instructions is in a block nothing \
         reaches. This was ONE — four octets of rung 100's 176 that its status \
         counted exactly as it counted the other 172 — until `W-279`\n{:#?}",
        blocks(&text)
    );
}

/// **RUNGS 99 AND 100 AGREE ON `live_backward_targets`, AND THE LINK REGISTER
/// IS WHAT TELLS THEM APART.**
///
/// Cycle 905 added reachability to break the one-backward-edge collision
/// between rungs 98 and 99. This is the collision THAT reading has: rung 99 and
/// rung 100 each answer a one-element LIVE backward list, and one of the two
/// objects loops inside a routine while the other re-enters one. Asserted
/// TOGETHER, so the difference is the reading and not two separate greens.
#[test]
fn rungs_99_and_100_agree_on_one_live_backward_edge_and_the_call_is_what_tells_them_apart() {
    let mut it = load();
    let (recur, exit) = compile(&mut it, RUNG_100_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 100 emits\n{recur}");
    let (loop_, exit) = compile(&mut it, RUNG_99_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 99 emits\n{loop_}");

    // THE COLLISION, STATED AS AN ASSERTION AND NOT AS PROSE.
    assert_eq!(
        live_backward_targets(&recur).len(),
        live_backward_targets(&loop_).len(),
        "each has exactly one LIVE backward edge, so the cycle-905 reading \
         cannot separate them\nrung 100: {:?}\nrung 99: {:?}",
        live_backward_targets(&recur),
        live_backward_targets(&loop_)
    );

    // AND THE READING THAT SEPARATES THEM: the kind of that one edge.
    let live_backward = |text: &str| -> Vec<(Kind, String, String)> {
        let (live, _) = reachability(text);
        transfers(text)
            .into_iter()
            .filter(|t| t.dir == Dir::Backward && live.contains(&t.from))
            .map(|t| (t.kind, t.from, t.target))
            .collect()
    };
    assert_eq!(
        live_backward(&loop_),
        vec![(Kind::Jump, "कघपर्व२".to_string(), "कघपर्व१".to_string())],
        "rung 99's live backward edge is an unconditional JUMP — the `यावत्`'s \
         own, inside one routine, writing no return address. It is NOT the \
         loop's test: the test is the FORWARD `न्यूनलङ्घनम्` that leaves the \
         loop, and reading this edge as `Branch` hid that for six cycles\n{loop_}"
    );
    assert_eq!(
        live_backward(&recur),
        vec![(Kind::Call, "कङपर्व४".to_string(), "कङ".to_string())],
        "rung 100's is a CALL to the routine's OWN ENTRY — the third shape of \
         backward edge on this ladder, after rung 99's live jump and rung \
         98's dead one\n{recur}"
    );

    // AND RUNG 98'S DEAD BACKWARD BRANCH IS NO LONGER IN RUNG 100'S OBJECT —
    // `W-279`. It was, and its presence was what made this pair's separation a
    // statement about KIND rather than about a second edge: a reader without
    // reachability saw two edges here and one in rung 99. It now sees ONE and
    // ONE, so the kind is the ONLY reading left that separates the pair, and
    // the claim this test makes is stronger than it was, not weaker.
    assert_eq!(
        backward_targets(&recur),
        vec!["कङ".to_string()],
        "rung 100 carries ONE backward edge, the self-call; the dead branch out \
         of कङपर्व५ that stood beside it is gone and कङपर्व५ is empty\n{recur}"
    );
    assert_eq!(
        backward_targets(&recur).len(),
        backward_targets(&loop_).len(),
        "so rungs 99 and 100 now collide in `backward` as well as in \
         `live_backward`, and `live_backward_kind` is the whole of what tells \
         them apart\nrung 100: {:?}\nrung 99: {:?}",
        backward_targets(&recur),
        backward_targets(&loop_)
    );
}

/// **AND THE CASE THAT MUST STILL BE REFUSED FOR RUNG 100: ८००० ALONE, THROUGH
/// PARSE.**
///
/// The `इति` that closes the recursive routine `ङ` struck, so the module never
/// terminates. Measured rather than assumed — see [`RUNG_100_NO_ROUTINE_END`],
/// where all three of the rung's bare `इति` were struck in turn and all three
/// answered [`PARSE_REFUSED`] with zero instructions.
#[test]
fn rung_100_without_the_recursive_routines_iti_is_refused_at_parse_and_would_answer_the_base_alone()
{
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_100_NO_ROUTINE_END, "क");
    assert_eq!(
        exit, PARSE_REFUSED,
        "striking the `इति` that closes `ङ` leaves the module unterminated, so \
         the parse yields NO declarations — an exit of {exit} means the parser \
         accepted a routine nothing closed\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    assert_eq!(unknown, Vec::<String>::new(), "unread line shape\n{text}");
    assert_eq!(
        insts.len(),
        0,
        "a refusal emits no instructions\n{}",
        insts.join("\n")
    );
    assert!(
        blocks(&text).is_empty(),
        "and not one block, so there is nothing for reachability to read\n{text}"
    );
    assert_eq!(
        rung_status(RUNG_100_BASE, insts.len()),
        8_000,
        "so a refused rung answers ८००० alone and can never be read as ८१७६"
    );
}

/// **A RECURSIVE CALL LOWERED AS A PLAIN JUMP COSTS THE SAME FORTY-THREE
/// INSTRUCTIONS, AND THE CYCLE-905 READING CANNOT SEE IT EITHER.**
///
/// This is the defect rung 100 exists to catch and the one its STATUS is blind
/// to. `लङ्घनम् पुनःस्थानम्म् कङय्` writes the return address; `लङ्घनम् शून्यःम् कङय्`
/// does not. An emitter that took the second road for a self-call places the
/// same instruction at the same offset — the object is 172 octets either way,
/// the rung still answers ८१७२ — and the routine never comes back: every
/// recursion re-enters the entry and the saved-register discipline the margin
/// names is never unwound.
///
/// **AND IT IS NOT ONLY THE LENGTH THAT MISSES IT.** Measured on the mutated
/// text: `reachability` is unchanged, and `live_backward_targets` — the reading
/// cycle 905 added, and the one that told rungs 98 and 99 apart — still answers
/// `["कङ"]`. The edge is live and backward in both programs. Only `Kind` moves,
/// which is why this mutation is the control for THIS rung rather than another
/// reachability one.
///
/// Driven without the chain, like
/// [`a_block_reached_only_by_fallthrough_is_not_read_as_dead`]: the compiled
/// emitter is right, and what is being falsified is the reader.
#[test]
fn a_recursive_call_lowered_as_a_jump_is_named_rather_than_counted() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_100_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the fixture must emit before it can be mutated"
    );

    // The SELF-call and nothing else. `कघ`'s call to `कङ` is the same mnemonic
    // and the same target, so the anchor carries the line BELOW it — the
    // argument-register move that only the recursive site is followed by.
    let cut = "लङ्घनम् पुनःस्थानम्म् कङय् ।\nयोगः स्थिर२म् अर्थ०न ०न ।";
    assert_eq!(
        text.matches(cut).count(),
        1,
        "the mutation's site occurs exactly once\n{text}"
    );
    let mutated = text.replacen(cut, "लङ्घनम् शून्यःम् कङय् ।\nयोगः स्थिर२म् अर्थ०न ०न ।", 1);

    let (insts, unknown) = text_lines(&mutated);
    assert_eq!(unknown, Vec::<String>::new(), "the shape is unchanged");
    assert_eq!(
        insts.len(),
        43,
        "the SAME forty-three instructions — a link register is a field of one \
         instruction and not an instruction"
    );
    assert_eq!(
        rung_status(RUNG_100_BASE, insts.len()),
        8_172,
        "so the rung answers ८१७२ for an object that never returns, and the \
         margin's falsifier list — ८००० alone, anything between, anything \
         above — rules none of this out"
    );

    // NOR DOES THE READING THAT BROKE THE LAST COLLISION.
    assert_eq!(
        reachability(&mutated),
        reachability(&text),
        "the same blocks are live and the same two are dead"
    );
    assert_eq!(
        live_backward_targets(&mutated),
        vec!["कङ".to_string()],
        "and the live backward edge is still there, still naming the routine's \
         own entry — cycle 905's reading answers the same list for the correct \
         object and for this one"
    );

    // THE READING THAT DOES SEE IT, TWICE OVER.
    assert_eq!(
        jump_targets(&mutated, RA),
        vec!["कङ".to_string()],
        "ONE call where the correct object makes two — the self-call is gone \
         from the call list entirely\n{mutated}"
    );
    assert_eq!(
        transfers(&mutated)
            .into_iter()
            .filter(|t| t.dir == Dir::Backward)
            .map(|t| (t.kind, t.from, t.target))
            .collect::<Vec<_>>(),
        vec![(Kind::Jump, "कङपर्व४".to_string(), "कङ".to_string())],
        "and the backward edge that was a CALL is now an unconditional JUMP — \
         rung 100's object made to look like a loop, which is exactly rung 99's \
         live shape. It is the ONLY backward edge left in \
         the object since `W-279`; the dead one out of कङपर्व५ that used to \
         stand beside it is no longer emitted\n{mutated}"
    );
}

// ── THE LADDER'S SEVEN RUNGS IN ONE TABLE ───────────────────────────────────
//
// Rungs 97, 98, 99, 100, 101, 102 and 103 each answer a status, and the four
// readings that tell them apart were each added to break the collision the one
// before it left: `backward` for the loop rung 99's status cannot see,
// `live_backward` when rung 98 turned out to answer the same ONE backward edge
// from code nothing reaches, `live_backward_kind` when rung 100's one live
// backward edge turned out to be a CALL and not a loop. Each separation lived
// in its own test and nothing on the tree stated the table, so the rung that
// collides next is found a cycle late.
//
// # WHAT THE TABLE MEASURED THAT THE CYCLE ASKING FOR IT PREDICTED WRONG
//
// The request registered two predictions and the table falsifies BOTH, which is
// the reason to keep them written here rather than quietly take the answer:
//
// ```text
//   predicted  drop `live_backward_kind` and the table reds on the 99/100 pair
//   measured   it reds on NOTHING — 99 and 100 differ in four other readings
//              (29 vs 44 instructions, 1 vs 2 backward edges, 0 vs 2 calls)
//
//   predicted  rung 103 has no routine, so its `blocks` is EMPTY of `.text`
//   measured   THREE blocks and eighteen instructions — RUNG_103_SOURCE does
//              declare `सार्वजनिक वृत्तिः घ`, and `reachability` is never asked
//              for a walk with no roots by any rung on this ladder
// ```
//
// So the five-reading tuple the request named is WIDE, not minimal: over the
// first six rungs the instruction COUNT alone separates every pair but one, and
// the pair it leaves is 97/102 — the in-object call and the cross-module
// relocation, which carry an identical `(0, 0, —, 1)` control shape.
//
// # THE SEVENTH ROW, AND WHAT IT COST THE TABLE
//
// Rung 101 — a `यदि` with BOTH arms over an arithmetic test, the one construct
// of `7319bd11`'s table this file had never read — was added on the prediction
// that it would either land on a free shape or collide and force a new reading.
// **IT LANDED ON A FREE SHAPE, AND NO READING WAS ADDED**: `(2, 0, —, 1, [F])`,
// thirty-nine instructions. `W-279` has since taken the two dead edges and
// three of the instructions, so the row is now `(0, 0, —, 1, [F])` and
// thirty-six — a shape it shares with rungs 97 and 98, separated by length
// alone. The paragraphs below are cycle 909's reading and are kept above that
// correction, not beside it.
//
// ```text
//   measured   39 instructions, TWO backward edges, NEITHER live, one forward
//              call — and the 39 is the ladder's own registered figure for
//              this rung (`39→९१५६`), re-derived here from the emitted text
//   consequence `backward` stops being a one-pair reading. Struck, it used to
//              merge {97,98}; it now merges {97,98}, {97,101} and {98,101} —
//              rung 101 is separated from rung 98 by its DEAD backward edge
//              count alone, two against one, and by nothing else in the shape
// ```
//
// THAT IS THE FRAGILE THING THIS ROW PUTS ON THE TREE AND IT IS SAID OUT LOUD
// RATHER THAN LEFT IN THE TUPLE: an emitter that dropped one unreachable
// backward edge from rung 101 would merge it with rung 98, and the merge would
// be reported as a collision rather than as the dead-code change it is. The
// count of DEAD edges is doing structural work here, which is a weaker footing
// than any other separation on this table. It is left as measured, because the
// alternative — inventing a reading to carry 98 from 101 before the emitter has
// ever moved that edge — pins a shape nothing has observed.
//
// # THE SIXTH READING, AND WHY THE TABLE IS STATED TWICE
//
// [`Dir::Unplaced`] is what tells 97 from 102 and it was already in the
// instrument, unasserted at the table level. Adding the calls' DIRECTIONS makes
// the table separable WITHOUT the instruction count at all, and that is the
// claim worth pinning: the six rungs differ in CONTROL SHAPE and not merely in
// size. A table that separated them by length would go green for an emitter
// that lowered a loop as straight-line code of the same length.
//
// Hence two assertions over one table — the full tuple is distinct, and the
// STRUCTURAL projection (the same tuple with the instruction count struck) is
// distinct too — and the minimality demonstration is taken on the structural
// one, because that is where a reading is load-bearing.
//
// # THE EIGHTH READING IS DELIBERATELY NOT MINIMAL, AND THAT BREAKS THE RULE
// # ABOVE ON PURPOSE
//
// `conditional_operands` separates NO pair. The minimality demonstration is
// therefore no longer a demonstration that every column earns its place, and
// pretending otherwise would be the easier thing to write. It is here because
// the ladder's blind spot is not a collision: `अधिकम्` lowers to
// `न्यूनलङ्घनम्` WITH ITS OPERANDS EXCHANGED, so a lowering that dropped the
// exchange moves nothing in the seven columns above — same mnemonic, same count,
// same graph, same length — and computes `<` where rung 98's source wrote `>`.
// A table that only ever gained columns to break collisions would have stayed
// green through that, which is the general lesson: SEPARATION IS NOT COVERAGE.
// The strike is asserted anyway and LABELLED vacuous, the convention `backward`'s
// strike established.

/// The five readings the ladder separates a rung by, the sixth this table
/// needed, the SEVENTH `W-279` made necessary, and an EIGHTH that separates
/// nothing and is here for a defect no separation can see. See the block comment
/// above.
#[derive(Debug, PartialEq, Eq, Clone)]
struct RungReading {
    /// `.text` instruction lines — what the rung's status is derived from.
    instructions: usize,
    /// Every backward edge, live or not. See [`backward_targets`].
    backward: usize,
    /// Backward edges FROM A REACHABLE BLOCK. See [`live_backward_targets`].
    live_backward: usize,
    /// Whether each live backward edge links. See [`Kind`].
    live_backward_kind: Vec<Kind>,
    /// Calls — `लङ्घनम्` writing `पुनःस्थानम्`. See [`jump_targets`].
    calls: usize,
    /// Where each call's target sits, `Unplaced` being a relocation. See
    /// [`transfers`]: this is the reading that tells rung 97 from rung 102.
    call_dirs: Vec<Dir>,
    /// **CONDITIONAL TRANSFERS — WHETHER THE OBJECT TESTS ANYTHING AT ALL.**
    ///
    /// See [`Kind::Conditional`]. Added because `W-279` emptied the `backward`
    /// column and left the table unable to tell a routine that returns straight
    /// (rung 97) from one behind a `यदि` (rungs 98, 101) by anything but
    /// LENGTH. It is the only reading here that sees a FORWARD conditional
    /// branch, and every other one is blind to a `यदि` whose arms are short.
    conditional: usize,
    /// **WHICH CONDITIONAL, IN EMISSION ORDER — AND IT IS BESIDE THE COUNT AND
    /// NEVER IN PLACE OF IT.**
    ///
    /// The count says the object TESTS; this says WHAT IT TESTS FOR. Rungs 98
    /// and 101 have isomorphic control graphs — one call, one conditional, four
    /// jumps, no backward edge — and every other reading in [`structural`]
    /// answers the same for both. The one thing that differs is what the source
    /// asked: 98's `यदि` compares with `अधिकम्` and lowers to `न्यूनलङ्घनम्`
    /// — `अधिकम्` is `न्यून` WITH ITS OPERANDS EXCHANGED (`ir.t1:2191-2193`),
    /// measured, and an earlier version of this margin said `<` — and 101's with
    /// `समम्`, which lowers to `समलङ्घनम्`. Nothing else on this tree asserted
    /// that, so an emitter that lowered `समम्` through `न्यूनलङ्घनम्` — the
    /// wrong comparison, the right shape — passed every test in this file.
    ///
    /// **AND THIS COLUMN IS BLIND TO THE EXCHANGE**, which is the defect rung 98
    /// is the one rung that could carry: a lowering that dropped the exchange
    /// answers `न्यूनलङ्घनम्` here and computes `<` where the source wrote
    /// `>`. See
    /// [`each_comparison_the_front_end_has_lowers_to_its_own_mnemonic_and_adhikam_exchanges_its_operands`]
    /// for the whole five-operator map, and
    /// [`RungReading::conditional_operands`] — the column added beside this one
    /// for exactly that blindness — for the pair each rung really compares.
    ///
    /// **IT IS NOT A CLOSED SET AND IT IS NOT A SECOND CLASSIFIER.** It is a
    /// projection of the mnemonics [`transfers`] already carries, taken off the
    /// SAME `Kind::Conditional` arm the count is taken off — the complement of
    /// `लङ्घनम्`. A branch nobody taught this reader is still counted AND its
    /// spelling is reported, which is the case
    /// [`an_untaught_conditional_mnemonic_still_counts_and_an_unconditional_one_makes_rung_101_read_as_rung_97`]
    /// pins by hand.
    ///
    /// **THE COST, STATED.** This is the only reading in the tuple that is a
    /// claim about the ISA's SPELLING rather than about control, so a rename in
    /// the assembler reds the table for a reason that is not a defect. The
    /// count is kept beside it precisely because the count survives that
    /// rename.
    conditional_mnemonics: Vec<String>,
    /// **AND WHICH TWO OPERANDS IT COMPARES, RESOLVED TO SPILL SLOTS — THE ONE
    /// DEFECT THE MNEMONIC COLUMN IS STRUCTURALLY BLIND TO.**
    ///
    /// `(करण, अपादान)` per conditional, in the same emission order as
    /// [`RungReading::conditional_mnemonics`] and exactly as long as it, both
    /// being taken off [`label_transfer`]'s `Kind::Conditional` arm.
    ///
    /// **WHY IT IS HERE.** `अधिकम्` lowers to `न्यूनलङ्घनम्` WITH ITS OPERANDS
    /// EXCHANGED (`ir.t1:2191-2193`), so a lowering that dropped the exchange
    /// answers the SAME mnemonic and computes `<` where the source wrote `>`.
    /// Rung 98 is the ladder's one `अधिकम्` and was therefore the one rung that
    /// could carry that defect with the whole table green.
    ///
    /// **A SLOT AND NOT A REGISTER**, because the register number is an
    /// allocation artefact: `व न्यूनम् द` and `द अधिकम् व` are the same
    /// comparison and name opposite registers, since the operand loads are
    /// placed in SOURCE order. See [`conditional_operand_slots`].
    ///
    /// **`None` IS NOT `Some(0)`.** `०` is a real slot — it is the FIRST
    /// parameter — so an operand that is a materialised literal or the result of
    /// an expression answers `None`. A reader that defaulted to `०` would score
    /// rung 100's `स न्यूनम् २` as a comparison of the first parameter against
    /// itself.
    conditional_operands: Vec<(Option<u64>, Option<u64>)>,
}

/// The structural tuple — [`RungReading`] with the instruction count struck.
/// Named because it is eight wide now and a bare tuple that long in a signature
/// says nothing about which column is which.
type Shape = (
    usize,
    usize,
    Vec<Kind>,
    usize,
    Vec<Dir>,
    usize,
    Vec<String>,
    Vec<(Option<u64>, Option<u64>)>,
);

impl RungReading {
    /// The tuple with the instruction count STRUCK — the control shape alone.
    fn structural(&self) -> Shape {
        (
            self.backward,
            self.live_backward,
            self.live_backward_kind.clone(),
            self.calls,
            self.call_dirs.clone(),
            self.conditional,
            self.conditional_mnemonics.clone(),
            self.conditional_operands.clone(),
        )
    }
}

/// Every reading of one emitted text, taken through the instrument above and
/// nothing else, so the table cannot drift from the per-rung tests.
fn read_rung(text: &str) -> RungReading {
    let (live, _) = reachability(text);
    let ts = transfers(text);
    RungReading {
        instructions: text_lines(text).0.len(),
        backward: backward_targets(text).len(),
        live_backward: live_backward_targets(text).len(),
        live_backward_kind: ts
            .iter()
            .filter(|t| t.dir == Dir::Backward && live.contains(&t.from))
            .map(|t| t.kind)
            .collect(),
        calls: jump_targets(text, RA).len(),
        call_dirs: ts
            .iter()
            .filter(|t| t.kind == Kind::Call)
            .map(|t| t.dir)
            .collect(),
        conditional: ts.iter().filter(|t| t.kind == Kind::Conditional).count(),
        conditional_mnemonics: ts
            .iter()
            .filter(|t| t.kind == Kind::Conditional)
            .map(|t| t.mnemonic.clone())
            .collect(),
        conditional_operands: conditional_operand_slots(text)
            .into_iter()
            .map(|(_, karana, apadana)| (karana, apadana))
            .collect(),
    }
}

/// Rung 101's source — `shrinkhala.t1`'s `स्वपरीक्षैकशततमी`, `प१ ⧺ प२ ⧺ प३ ⧺ प४ ⧺ प५`.
///
/// A BRANCH WITH BOTH ARMS OVER AN ARITHMETIC TEST, and the seventh row on this
/// table. Rung 98 is also a `यदि` and this is not rung 98 twice: 98's branch has
/// NO `अन्यथा` and falls through, this one has both arms and EACH RETURNS, so
/// the emitter places two exits and a join no arm reaches. Its condition is the
/// first on the ladder that is an arithmetic EXPRESSION — `स शेषः २` — rather
/// than a name against a literal.
///
/// THE FIVE PIECES WERE REASSEMBLED FROM THE RUNG RATHER THAN RETYPED, the way
/// every fixture above it was: `shrinkhala.t1:2963-2967`'s five `उक्तम् … इति`
/// literals concatenated character for character give exactly the string below.
/// The rung has FOUR bare `इति` — one closing `घ`, one closing the TAKEN arm
/// before `अन्यथा`, one closing the `अन्यथा` arm, one closing `ङ` — which is
/// why it needs five pieces where rung 100 needs four; copying rung 100's split
/// here terminates the literal at `इति अन्यथा` and drops the rest.
///
/// **IT IS THE FIRST ROW THAT ENTERS THIS TABLE WITHOUT A PER-RUNG TEST OF ITS
/// OWN**, so the table is the only thing on the tree that states its shape.
const RUNG_101_SOURCE: &str = "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ ददाति न६४ आदि प्रत्यागमनम् ङ ७ । इति \
सार्वजनिक वृत्तिः ङ आदाय स ॱॱ न६४ ददाति न६४ आदि यदि आरभ्य स शेषः २ समाप्तम् समम् ० आदि \
प्रत्यागमनम् ० । इति अन्यथा आदि प्रत्यागमनम् १ । इति इति";

/// [`RUNG_101_SOURCE`] with the `इति` that closes the TAKEN ARM — the one
/// standing immediately before `अन्यथा` — DELETED. The case that must still be
/// REFUSED, and it is the refusal rung 98 cannot have: 98's `यदि` has no
/// `अन्यथा` at all, so this `इति` is a terminator only a BOTH-ARMED branch owns.
const RUNG_101_NO_TAKEN_ARM_END: &str = "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ ददाति न६४ आदि प्रत्यागमनम् ङ ७ । इति \
सार्वजनिक वृत्तिः ङ आदाय स ॱॱ न६४ ददाति न६४ आदि यदि आरभ्य स शेषः २ समाप्तम् समम् ० आदि \
प्रत्यागमनम् ० । अन्यथा आदि प्रत्यागमनम् १ । इति इति";

/// Rung 101's base — `स्वपरीक्षैकशततमी`'s own
/// `प्रत्यागमनम् ९००० योगः अष्टकाः ॱ दैर्घ्य` (`shrinkhala.t1:2983`).
const RUNG_101_BASE: u64 = 9_000;

/// The seven sources this table reads, in ladder order. Each but rung 101 is the
/// fixture the rung's own test compiles — not a copy — so a fixture corrected in
/// one place is corrected here.
fn ladder() -> [(&'static str, &'static str); 7] {
    [
        ("97", RUNG_97_SOURCE),
        ("98", RUNG_98_SOURCE),
        ("99", RUNG_99_SOURCE),
        ("100", RUNG_100_SOURCE),
        ("101", RUNG_101_SOURCE),
        ("102", RUNG_102_SOURCE),
        ("103", RUNG_103_SOURCE),
    ]
}

/// Compiles every rung of [`ladder`] and answers its reading, refusing any
/// source that does not reach the emitter — a refusal answers zero
/// instructions and would enter the table as a control shape rather than as
/// the absence of one.
fn ladder_readings() -> Vec<(&'static str, RungReading)> {
    let mut it = load();
    ladder()
        .into_iter()
        .map(|(name, src)| {
            let (text, exit) = compile(&mut it, src, "क");
            assert_eq!(
                exit, EMITTED,
                "rung {name}'s source must reach the emitter; exit {exit} is a \
                 refusal (shrinkhala.t1:118-154 numbers them) and a refusal \
                 enters this table as an all-zero shape\n{text}"
            );
            let (_, unknown) = text_lines(&text);
            assert_eq!(
                unknown,
                Vec::<String>::new(),
                "rung {name}: every non-blank line must be a directive, a label \
                 or an instruction\n{text}"
            );
            assert_eq!(
                repeats(&defined_labels(&text)),
                Vec::<(String, usize)>::new(),
                "rung {name}: a label defined twice gives `earlier in the text` \
                 two answers\n{text}"
            );
            (name, read_rung(&text))
        })
        .collect()
}

/// **THE SEVEN RUNGS' READINGS, PINNED IN ONE PLACE, AND NO TWO ARE THE SAME.**
///
/// Seven tuples, measured on this tree and not carried from a margin.
///
/// **`W-279` EMPTIED THE BACKWARD COLUMN FOR EVERY RUNG BUT 99 AND 100, AND
/// THE `conditional` COLUMN IS WHAT REPLACED IT.** Rung 98 answered ONE
/// backward edge and rung 101 answered TWO, all of them in blocks nothing
/// reached, and those DEAD counts were the only reading that kept 97, 98 and
/// 101 apart. With the dead code gone all three read `0 0 [] 1 [Forward]` and
/// were separated by the INSTRUCTION COUNT alone.
///
/// [`RungReading::conditional`] takes that back, and it takes it back on a LIVE
/// reading rather than a dead one: rung 97 tests nothing and answers 0, rungs
/// 98 and 101 each stand behind one `यदि` and answer 1.
///
/// **AND [`RungReading::conditional_mnemonics`] TOOK THE LAST PAIR.** 98 and
/// 101 stayed merged under the strike because their control graphs really are
/// isomorphic; the mnemonic is what tells `<` from `समम्`. See
/// [`striking_the_instruction_count_now_merges_no_pair_and_the_mnemonic_took_the_last_one`],
/// which is where the empty merge list is asserted rather than described — and
/// where the cost of reading a SPELLING is named.
///
/// **AND THE EIGHTH READING TAKES BACK NO PAIR AT ALL, WHICH IS WHY IT IS
/// HERE.** [`RungReading::conditional_operands`] resolves each conditional's two
/// operands to SPILL SLOTS. The table was already fully separated, so its strike
/// is VACUOUS and is recorded as vacuous — it was not added to separate a row.
/// It was added because `अधिकम्` lowers to `न्यूनलङ्घनम्` WITH ITS OPERANDS
/// EXCHANGED, and a lowering that dropped that exchange moves NOTHING in the
/// seven columns above: same mnemonic, same count, same control graph, same
/// length. Rung 98 is the ladder's one `अधिकम्` and was the one rung that could
/// carry that defect with this whole table green. The eight measured pairs:
///
/// ```text
///    97  []                 no conditional
///    98  [(८, ०)]           `व अधिकम् द` — EXCHANGED; व is slot ० and द slot ८
///    99  [(८, -)]           `र न्यूनम् ११` — locals स=० and र=८; ११ materialised
///   100  [(०, -)]           `स न्यूनम् २` — स is the one parameter
///   101  [(-, -)]           `आरभ्य स शेषः २ समाप्तम् समम् ०` — neither side in a slot
///   102  []                 no conditional
///   103  []                 no conditional
/// ```
///
/// All seven were registered before the run and all seven held. `-` is `None`
/// and it is NOT slot `०`: `०` is the first parameter, so a reader that
/// defaulted to zero would report rungs 99, 100 and 101 as comparing the first
/// parameter against itself.
#[test]
fn the_seven_answered_rungs_are_separated_by_their_readings_and_no_two_collide() {
    let readings = ladder_readings();

    for (name, r) in &readings {
        println!(
            // THE MNEMONICS GO OUT THROUGH `Display` AND NOT `Debug`: `{:?}`
            // on a `Vec<String>` escapes every Devanagari codepoint to
            // `\u{{94d}}`, so the one column a human would read the line FOR
            // comes out unreadable.
            "METRIC t1_rung{name}_reading {} {} {} {:?} {} {:?} {} [{}] {:?}",
            r.instructions,
            r.backward,
            r.live_backward,
            r.live_backward_kind,
            r.calls,
            r.call_dirs,
            r.conditional,
            r.conditional_mnemonics.join(","),
            r.conditional_operands
        );
        // THE TWO CONDITIONAL COLUMNS CANNOT BE DIFFERENT LENGTHS — both are
        // taken off `label_transfer`'s `Kind::Conditional` arm, so this is a
        // check that the shared predicate really is shared. A short operand
        // list beside a full mnemonic list is the reading an emitter would get
        // for free by placing a branch whose operands this file cannot parse.
        assert_eq!(
            r.conditional_operands.len(),
            r.conditional_mnemonics.len(),
            "rung {name}: one operand pair per conditional, and the two columns \
             are projections of the SAME arm"
        );
    }

    let expected: Vec<(&str, RungReading)> = vec![
        (
            "97",
            RungReading {
                instructions: 29,
                backward: 0,
                live_backward: 0,
                live_backward_kind: vec![],
                calls: 1,
                call_dirs: vec![Dir::Forward],
                conditional: 0,
                conditional_mnemonics: vec![],
                conditional_operands: vec![],
            },
        ),
        (
            "98",
            RungReading {
                instructions: 40,
                backward: 0,
                live_backward: 0,
                live_backward_kind: vec![],
                calls: 1,
                call_dirs: vec![Dir::Forward],
                conditional: 1,
                conditional_mnemonics: vec!["न्यूनलङ्घनम्".to_string()],
                conditional_operands: vec![(Some(8), Some(0))],
            },
        ),
        (
            "99",
            RungReading {
                instructions: 29,
                backward: 1,
                live_backward: 1,
                live_backward_kind: vec![Kind::Jump],
                calls: 0,
                call_dirs: vec![],
                conditional: 1,
                conditional_mnemonics: vec!["न्यूनलङ्घनम्".to_string()],
                conditional_operands: vec![(Some(8), None)],
            },
        ),
        (
            "100",
            RungReading {
                instructions: 43,
                backward: 1,
                live_backward: 1,
                live_backward_kind: vec![Kind::Call],
                calls: 2,
                call_dirs: vec![Dir::Forward, Dir::Backward],
                conditional: 1,
                conditional_mnemonics: vec!["न्यूनलङ्घनम्".to_string()],
                conditional_operands: vec![(Some(0), None)],
            },
        ),
        (
            "101",
            RungReading {
                instructions: 36,
                backward: 0,
                live_backward: 0,
                live_backward_kind: vec![],
                calls: 1,
                call_dirs: vec![Dir::Forward],
                conditional: 1,
                conditional_mnemonics: vec!["समलङ्घनम्".to_string()],
                conditional_operands: vec![(None, None)],
            },
        ),
        (
            "102",
            RungReading {
                instructions: 13,
                backward: 0,
                live_backward: 0,
                live_backward_kind: vec![],
                calls: 1,
                call_dirs: vec![Dir::Unplaced],
                conditional: 0,
                conditional_mnemonics: vec![],
                conditional_operands: vec![],
            },
        ),
        (
            "103",
            RungReading {
                instructions: 18,
                backward: 0,
                live_backward: 0,
                live_backward_kind: vec![],
                calls: 0,
                call_dirs: vec![],
                conditional: 0,
                conditional_mnemonics: vec![],
                conditional_operands: vec![],
            },
        ),
    ];

    // THE WHOLE TABLE AT ONCE AND NOT SIX ASSERTIONS, so a rung whose shape
    // moves is reported beside the five that did not.
    assert_eq!(
        readings, expected,
        "the ladder's seven readings must be exactly the table above"
    );

    // AND NO TWO ROWS ARE THE SAME — the claim the table exists to make.
    let mut collisions: Vec<(&str, &str)> = Vec::new();
    for (i, (a, ra)) in readings.iter().enumerate() {
        for (b, rb) in readings.iter().skip(i + 1) {
            if ra == rb {
                collisions.push((a, b));
            }
        }
    }
    assert_eq!(
        collisions,
        Vec::<(&str, &str)>::new(),
        "two rungs of the ladder answer the same reading, so neither's green \
         is a statement about its own program"
    );
}

/// **THE MERGE LIST IS EMPTY FOR THE FIRST TIME, AND THE READING THAT EMPTIED
/// IT IS THE CONDITIONAL'S MNEMONIC.**
///
/// THE LEDGER'S PREDICTION, REGISTERED IN `STATE.md` BEFORE THE COLUMN WAS
/// MEASURED (cycle 912's `Next:`, written at `27a85b4a`):
/// `97:[], 98:[न्यूनलङ्घनम्], 99:[न्यूनलङ्घनम्], 100:[न्यूनलङ्घनम्],
/// 101:[समलङ्घनम्], 102:[], 103:[]` — which separates 98 from 101, the last
/// merged pair, and changes no other row. That is exactly what the table
/// answers.
///
/// **AND THE PREDICTION THAT CAME WITH IT WAS WRONG ON ONE POINT, WHICH IS THE
/// FINDING.** The `Next:` said the mnemonic must be added BESIDE the count and
/// never in place of it, so that an untaught branch is still counted. It is —
/// but the count is now SUBSUMED: `conditional_mnemonics.len()` IS
/// `conditional`, so striking the count from the tuple below merges NOTHING.
/// The count is kept anyway and the reason is named where the strike is
/// asserted: it is the only one of the two that survives an assembler renaming
/// a mnemonic, and the refusal case rests on it.
///
/// **THE PREDICTION REGISTERED FOR THE NEXT CYCLE, BEFORE IT IS MEASURED.**
/// Strike `conditional_mnemonics` and the merge list returns to exactly
/// `[("98", "101")]` — the pair this column exists to break and no other. IT
/// HELD, and it is the assertion at the foot of this test.
///
/// **AND THE CYCLE AFTER IT MADE THAT STRIKE CONDITIONAL, WHICH IS THIS TEST'S
/// STANDING LESSON ABOUT A LOAD-BEARING STRIKE.**
/// [`RungReading::conditional_operands`] separates 98 from 101 TOO — 98 compares
/// slot ८ against slot ०, 101 compares two values that are in no slot — so the
/// mnemonic's strike merges the pair only in a tuple that lacks the operands as
/// well. Both projections are asserted below and BOTH columns stay: the mnemonic
/// survives an allocator that moved every slot, the operands survive an
/// assembler that renamed every branch. A strike is load-bearing against a
/// STATED tuple and never absolutely, and that was learnt here twice in three
/// cycles — the conditional COUNT was load-bearing for one cycle and subsumed
/// the next.
///
/// **THE THREE CORRECTIONS THIS ROW MAKES, KEPT ABOVE THEIR SUBJECTS.**
///
/// 1. The cycle-911 margin said the reading was to be taken "off `transfers()`
///    which already carries `Kind`". It did NOT. [`Kind`] had two states where
///    the truth has three: an unconditional jump and a conditional test were
///    both `Branch`, so a count of `Branch` answers FOUR for rung 99 and THREE
///    for rung 97 — a difference in exit count, not in whether either program
///    tests anything. The reading required splitting the enum first.
/// 2. Rung 99's LIVE BACKWARD EDGE IS NOT THE LOOP'S TEST. It is an
///    unconditional `लङ्घनम् शून्यःम् कघपर्व१य् ।`; the test is the FORWARD
///    `न्यूनलङ्घनम्` that LEAVES the loop. Six cycles of margin called that edge
///    a branch, which is true only of the folded enum.
/// 3. `backward` counting DEAD edges was the whole of the 97/98/101
///    separation, and the previous margin called the loss "a reading this file
///    does not have". It has it now, and on a LIVE reading.
///
/// **WHAT IS STILL NOT SEPARATED, STATED SO THE EMPTY LIST IS NOT MISTAKEN FOR
/// A STRONGER CLAIM THAN IT IS.** Rungs 98 and 101 still have ISOMORPHIC
/// CONTROL GRAPHS — one call, one conditional, four jumps, no backward edge —
/// and that remains a true statement about the objects. The table no longer
/// merges them, but it does not separate them on control: it separates them on
/// what the conditional COMPARES FOR. A defect that swapped the two objects'
/// control graphs wholesale and kept each mnemonic would still read as two
/// distinct rows.
///
/// **WHAT IS NOT LOST.** The full tuple still separates all seven — see
/// [`the_seven_answered_rungs_are_separated_by_their_readings_and_no_two_collide`]
/// — and the separation `live_backward` and `live_backward_kind` make between
/// rungs 99 and 100 is untouched: those are LIVE edges and neither `W-279` nor
/// the enum split reaches them.
#[test]
fn striking_the_instruction_count_now_merges_no_pair_and_the_mnemonic_took_the_last_one() {
    let readings = ladder_readings();

    // THE STRONG CLAIM FIRST: length struck, still six distinct shapes.
    let shapes: Vec<(&str, _)> = readings.iter().map(|(n, r)| (*n, r.structural())).collect();
    let mut merged: Vec<(&str, &str)> = Vec::new();
    for (i, (a, sa)) in shapes.iter().enumerate() {
        for (b, sb) in shapes.iter().skip(i + 1) {
            if sa == sb {
                merged.push((a, b));
            }
        }
    }
    assert_eq!(
        merged,
        Vec::<(&str, &str)>::new(),
        "with the instruction count struck, NO pair merges — the first time \
         that has been true on a LIVE reading. It was empty before `W-279` on \
         DEAD edges, a three-way merge after it, a one-way merge once \
         `conditional` took back the pairs that had rung 97 in them, and empty \
         now that the conditional's MNEMONIC tells 98's `<` from 101's `समम्`. \
         Every rung's green is a statement about its own program"
    );

    // THE PROJECTIONS. Each strikes one reading from the structural tuple and
    // answers the pairs that then merge.
    let merges = |f: &dyn Fn(&RungReading) -> String| -> Vec<(&'static str, &'static str)> {
        let keys: Vec<(&str, String)> = readings.iter().map(|(n, r)| (*n, f(r))).collect();
        let mut out = Vec::new();
        for (i, (a, ka)) in keys.iter().enumerate() {
            for (b, kb) in keys.iter().skip(i + 1) {
                if ka == kb {
                    out.push((*a, *b));
                }
            }
        }
        out
    };

    assert_eq!(
        merges(&|r: &RungReading| format!(
            "{} {:?} {} {:?} {} {:?}",
            r.live_backward,
            r.live_backward_kind,
            r.calls,
            r.call_dirs,
            r.conditional,
            r.conditional_mnemonics
        )),
        Vec::<(&str, &str)>::new(),
        "strike `backward` and NOTHING merges — the strike still costs nothing, \
         because every rung but 99 and 100 answers zero backward edges and \
         those two are already apart on `live_backward_kind`. This assertion \
         was the load-bearing one for `backward` and it stays vacuous: it would \
         read the same empty list with the reading deleted"
    );

    assert_eq!(
        merges(&|r: &RungReading| format!(
            "{} {} {:?} {} {} {:?}",
            r.backward,
            r.live_backward,
            r.live_backward_kind,
            r.calls,
            r.conditional,
            r.conditional_mnemonics
        )),
        vec![("97", "102")],
        "strike `call_dirs` and ONE pair merges, where two did before the \
         mnemonic column and six before `conditional`. `Dir::Unplaced` is the \
         relocation and it remains the ONLY reading that separates an in-object \
         call from a cross-module one — the 97/102 pair, both of which test \
         nothing, so neither the count nor the mnemonic list can reach them. \
         This is `call_dirs` load-bearing and alone in being so"
    );

    // AND THE READINGS THAT ARE NOT LOAD-BEARING FOR THE TABLE, named rather
    // than implied. Each was added to break a collision in ONE reading, and
    // that pairwise claim is asserted below where it is actually true.
    assert_eq!(
        merges(&|r: &RungReading| format!(
            "{} {} {} {:?} {} {:?}",
            r.backward,
            r.live_backward,
            r.calls,
            r.call_dirs,
            r.conditional,
            r.conditional_mnemonics
        )),
        Vec::<(&str, &str)>::new(),
        "striking `live_backward_kind` merges NOTHING — not rungs 99 and 100, \
         whose collision was in one reading and never in the tuple, and not the \
         98/101 pair the mnemonic now holds apart. The prediction that this \
         strike would red 99/100 is still FALSE, and the reading still makes \
         the sharper statement it gained in the enum split: 99 answers `Jump` \
         where 100 answers `Call`"
    );
    assert_eq!(
        merges(&|r: &RungReading| format!(
            "{} {:?} {} {:?} {} {:?}",
            r.backward,
            r.live_backward_kind,
            r.calls,
            r.call_dirs,
            r.conditional,
            r.conditional_mnemonics
        )),
        Vec::<(&str, &str)>::new(),
        "striking `live_backward` merges nothing either"
    );

    // ── THE STRIKE THIS CYCLE EXISTS FOR, AND IT IS THE ONLY ONE ON THIS
    // LIST THAT COSTS ANYTHING. Struck, the structural tuple is exactly the one
    // the PREVIOUS cycle pinned, and the pair it could not separate comes back.
    assert_eq!(
        merges(&|r: &RungReading| format!(
            "{} {} {:?} {} {:?} {}",
            r.backward, r.live_backward, r.live_backward_kind, r.calls, r.call_dirs, r.conditional
        )),
        vec![("98", "101")],
        "strike `conditional_mnemonics` and the 98/101 pair merges again — the \
         prediction registered in this test's own margin BEFORE the strike was \
         run, and the proof the column is not decoration. A reading whose \
         strike changes nothing is a reading the table does not need"
    );

    // ── AND THE COUNT, WHICH THIS CYCLE MADE REDUNDANT AND KEPT ANYWAY.
    // `conditional_mnemonics.len()` IS `conditional`, so striking the count
    // while the list stands merges NOTHING: the previous cycle's load-bearing
    // assertion here — a three-way merge — is now vacuous, and it is recorded
    // as vacuous rather than deleted, because that is the measured cost of
    // adding the list BESIDE the count the ledger's `Next:` insisted on.
    //
    // WHY THE COUNT STAYS. It is a claim about CONTROL and the list is a claim
    // about the ISA's SPELLING. An assembler that renamed `न्यूनलङ्घनम्` would
    // red the list on every rung that tests, and the count would still say each
    // object tests exactly once — and the refusal case
    // [`an_untaught_conditional_mnemonic_still_counts_and_an_unconditional_one_makes_rung_101_read_as_rung_97`]
    // is asserted on the COUNT, where a spelling cannot reach it.
    assert_eq!(
        merges(&|r: &RungReading| format!(
            "{} {} {:?} {} {:?} {:?}",
            r.backward,
            r.live_backward,
            r.live_backward_kind,
            r.calls,
            r.call_dirs,
            r.conditional_mnemonics
        )),
        Vec::<(&str, &str)>::new(),
        "strike the conditional COUNT and nothing merges, because the mnemonic \
         list carries the count in its length. The column that took back two \
         pairs one cycle ago is subsumed one cycle later — said out loud so it \
         is not read as load-bearing a second time"
    );

    // ── THIS CYCLE'S COLUMN, AND ITS STRIKE IS RECORDED AS VACUOUS.
    // `conditional_operands` was added to see a defect, not to separate a pair:
    // the table was ALREADY fully separated, so striking the operands can merge
    // nothing. That was the prediction registered before the run and it held.
    // It is asserted anyway — an assertion that would read the same list with
    // the reading deleted is worth keeping only when it is LABELLED vacuous,
    // which is the convention `backward`'s strike above established.
    assert_eq!(
        merges(&|r: &RungReading| format!(
            "{} {} {:?} {} {:?} {} {:?}",
            r.backward,
            r.live_backward,
            r.live_backward_kind,
            r.calls,
            r.call_dirs,
            r.conditional,
            r.conditional_mnemonics
        )),
        Vec::<(&str, &str)>::new(),
        "strike `conditional_operands` and NOTHING merges — VACUOUS, and \
         recorded as vacuous. The operand pair exists to catch a dropped \
         `अधिकम्` exchange, which is a defect no separation can see: the \
         mnemonic is IDENTICAL on both sides of it"
    );

    // ── AND THE CONSEQUENCE FOR THE PREVIOUS CYCLE'S LOAD-BEARING STRIKE,
    // STATED RATHER THAN LEFT TO BE INFERRED. The strike above asserts that
    // striking `conditional_mnemonics` merges the 98/101 pair — and that is now
    // a statement about a tuple carrying NEITHER conditional spelling NOR the
    // operands. With the operands standing, the mnemonic's strike costs nothing:
    // 98 compares slot ८ against slot ० and 101 compares two values that are in
    // no slot at all. So the mnemonic column is no longer the ONLY thing holding
    // the ladder's last merged pair apart.
    assert_eq!(
        merges(&|r: &RungReading| format!(
            "{} {} {:?} {} {:?} {} {:?}",
            r.backward,
            r.live_backward,
            r.live_backward_kind,
            r.calls,
            r.call_dirs,
            r.conditional,
            r.conditional_operands
        )),
        Vec::<(&str, &str)>::new(),
        "strike `conditional_mnemonics` while `conditional_operands` STANDS and \
         nothing merges — the operand pair carries the 98/101 separation on its \
         own, which the cycle that added the mnemonic could not know. Both \
         columns stay: the mnemonic survives an allocator that moved every \
         slot, and the operands survive an assembler that renamed every branch"
    );

    // THE PAIRWISE CLAIMS THE TWO NON-LOAD-BEARING READINGS DO MAKE, which is
    // where their green belongs. These are the collisions the ladder actually
    // hit, a cycle apart, in a SINGLE reading each.
    let at = |name: &str| -> RungReading {
        readings
            .iter()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("rung {name} is on the table"))
            .1
            .clone()
    };
    let (r98, r99, r100) = (at("98"), at("99"), at("100"));
    assert_eq!(
        (r98.backward, r99.backward),
        (0, 1),
        "rungs 98 and 99 no longer collide in the backward count — 98 answered \
         ONE until `W-279` and answers NONE now, so the collision \
         `live_backward` was added for has been dissolved by the emitter \
         rather than by the reading"
    );
    assert_ne!(
        r98.live_backward, r99.live_backward,
        "and reachability still separates them, on 99's live loop edge alone"
    );
    assert_eq!(
        (r99.live_backward, r100.live_backward),
        (1, 1),
        "rungs 99 and 100 answer the SAME live backward edge count — the \
         collision `live_backward_kind` was added for"
    );
    assert_ne!(
        r99.live_backward_kind, r100.live_backward_kind,
        "and the KIND is what tells them apart: 99's is an unconditional JUMP \
         closing a loop, 100's is a CALL to the routine's own entry. Neither is \
         `Conditional`, which is the point — a LOOP IS CLOSED BY A JUMP AND \
         LEFT BY A TEST, so no backward reading on this ladder ever sees the \
         conditional and `conditional` reads a FORWARD edge in all four rungs \
         that answer one"
    );

    // AND THE PAIRWISE CLAIM `conditional` ITSELF MAKES, which is the 97/98
    // pair: the two shortest routines that both end in a return, one of which
    // reaches it through a test. Every other reading in the tuple answers the
    // same for both.
    let r97 = at("97");
    assert_eq!(
        (r97.conditional, r98.conditional),
        (0, 1),
        "rung 97 returns straight and tests NOTHING; rung 98 stands behind a \
         one-armed `यदि` and tests ONCE. This is the separation `W-279` took \
         away and this column gives back"
    );
    assert_eq!(
        (
            r97.backward,
            r97.live_backward,
            r97.calls,
            r97.call_dirs.clone()
        ),
        (
            r98.backward,
            r98.live_backward,
            r98.calls,
            r98.call_dirs.clone()
        ),
        "and they agree in EVERY structural reading but the two conditional \
         ones, so the test rung 98 stands behind is the whole of the separation \
         and not one vote in it"
    );

    // ── AND THE PAIRWISE CLAIM THE MNEMONIC LIST ITSELF MAKES, which is the
    // 98/101 pair and ONLY that pair: the two `यदि` rungs whose control graphs
    // are isomorphic. Asserted here, where it is true, rather than implied by
    // the empty merge list above.
    let r101 = at("101");
    assert_eq!(
        (
            r98.backward,
            r98.live_backward,
            r98.live_backward_kind.clone(),
            r98.calls,
            r98.call_dirs.clone(),
            r98.conditional
        ),
        (
            r101.backward,
            r101.live_backward,
            r101.live_backward_kind.clone(),
            r101.calls,
            r101.call_dirs.clone(),
            r101.conditional
        ),
        "rungs 98 and 101 agree in EVERY reading of control — no backward \
         edges, one call forward, one conditional. Their CFGs are isomorphic \
         and no count can tell them apart"
    );
    assert_eq!(
        (
            r98.conditional_mnemonics.clone(),
            r101.conditional_mnemonics.clone()
        ),
        (vec!["न्यूनलङ्घनम्".to_string()], vec!["समलङ्घनम्".to_string()]),
        "and the MNEMONIC is the whole of the separation: 98's `यदि` compares \
         with `<` and lowers to `न्यूनलङ्घनम्`, 101's with `समम्` and lowers to \
         `समलङ्घनम्`. An emitter that lowered one comparison through the other \
         operator keeps every reading above and moves ONLY this one"
    );
}

/// **RUNG 98 IS THE LADDER'S ONE `अधिकम्`, AND THE TABLE NOW READS WHICH TWO
/// SLOTS IT COMPARES — WHICH IS THE ONE DEFECT EVERY OTHER COLUMN IS BLIND TO.**
///
/// `अधिकम्` lowers to `न्यूनलङ्घनम्` WITH ITS OPERANDS EXCHANGED, so a lowering
/// that dropped the exchange answers the SAME mnemonic, the same count, the same
/// control graph and the same instruction count, and computes `<` where rung 98's
/// source wrote `>`. The rung's own status is an octet LENGTH and cannot see it
/// either. Until this cycle nothing on this tree read rung 98's operands.
///
/// **THE PREDICTION, REGISTERED IN THE LEDGER BEFORE THE COLUMN WAS MEASURED:**
/// `97:[], 98:[(८,०)], 99:[(८,-)], 100:[(०,-)], 101:[(-,-)], 102:[], 103:[]` —
/// rung 98 EXCHANGED because its `यदि` is `व अधिकम् द` with `व` in slot `०`;
/// rungs 99 and 100 comparing a spilled name against a MATERIALISED literal, so
/// one side is in no slot; rung 101 comparing an EXPRESSION (`स शेषः २`) against
/// a literal, so neither side is. All seven held.
///
/// **THE THREE MUTATIONS BELOW ARE WHAT MAKE THAT GREEN MEAN ANYTHING**, taken
/// on the text the chain really emitted:
///
/// 1. EXCHANGE rung 98's two operands by hand — the dropped exchange, made by
///    hand — and the pair must MOVE while `conditional_mnemonics` does not. That
///    is the whole claim: the mnemonic column cannot see this and this one can.
/// 2. OVERWRITE the करण register between its spill load and the branch. Its slot
///    is GONE and the pair must answer `None` there: a register holds a slot
///    only until something writes it, and the reader that shipped in cycle 912
///    never invalidated, so it would have answered the stale `८`.
/// 3. THE CASE THAT MUST STILL BE REFUSED — [`RUNG_98_NO_YADI_END`] never
///    reaches the emitter, and a refused rung must answer an EMPTY pair list and
///    NOT `[(Some(0), Some(0))]`. `०` is a real slot here; it is the FIRST
///    parameter, so a reader that defaulted to it would score a refusal as a
///    comparison of `व` against itself and enter it on the table as a shape.
#[test]
fn rung_98s_exchange_is_read_off_its_operands_and_a_refused_rung_answers_no_pair() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_98_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 98 emits\n{text}");

    // THE SLOTS IN THE SOURCE'S OWN TERMS FIRST: `व` is the first parameter and
    // `द` the second, so the pair below is read as `द` AGAINST `व` — the
    // exchange — rather than as two numbers.
    assert_eq!(
        parameter_slots(&text),
        vec![0, 8],
        "rung 98's callee takes `व` then `द`, spilled to slots ० and ८\n{text}"
    );

    let base = read_rung(&text);
    assert_eq!(
        (
            base.conditional,
            base.conditional_mnemonics.clone(),
            base.conditional_operands.clone()
        ),
        (1, vec!["न्यूनलङ्घनम्".to_string()], vec![(Some(8), Some(0))]),
        "rung 98's `यदि व अधिकम् द` lowers to ONE `न्यूनलङ्घनम्` comparing slot \
         ८ — `द` — AGAINST slot ० — `व`. The operands are EXCHANGED, which is \
         what makes `अधिकम्` and `न्यूनम्` one instruction\n{text}"
    );

    // THE LINE ALL THREE MUTATIONS ARE ABOUT, asserted present exactly once: a
    // `replace` that matched nothing leaves the text alone and every assertion
    // below would then be about the UNMUTATED object.
    let branch = text
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("न्यूनलङ्घनम्"))
        .map(str::to_string)
        .collect::<Vec<_>>();
    assert_eq!(
        branch.len(),
        1,
        "rung 98 places exactly one conditional branch\n{text}"
    );
    let branch = &branch[0];
    let f: Vec<&str> = branch.split_whitespace().collect();
    assert_eq!(f.len(), 5, "`{branch}` is mnemonic, करण, अपादान, कर्म, `।`");

    // ── 1. THE EXCHANGE, DROPPED BY HAND ────────────────────────────────────
    //
    // The two operands carry their ROLE in their ending — `…न` the करण, `…त्`
    // the अपादान — so exchanging them means re-declining each register, not
    // moving two words. Built with `strip_suffix` and never `replace`, which
    // would rewrite an ending that occurred inside a register's NAME.
    let (l_reg, r_reg) = (
        f[1].strip_suffix('न').expect("the करण is a register"),
        f[2].strip_suffix("त्").expect("the अपादान is a register"),
    );
    let dropped = text.replace(branch, &format!("{} {r_reg}न {l_reg}त् {} ।", f[0], f[3]));
    assert_ne!(dropped, text, "the mutation must actually bite\n{text}");
    let read = read_rung(&dropped);
    assert_eq!(
        read.conditional_mnemonics, base.conditional_mnemonics,
        "the MNEMONIC does not move — which is the whole reason this column \
         exists\n{dropped}"
    );
    assert_eq!(
        read,
        RungReading {
            conditional_operands: vec![(Some(0), Some(8))],
            ..base.clone()
        },
        "and the reading is the base with the OPERAND PAIR exchanged and \
         nothing else — not the count, not the control shape, not the length. \
         An emitter that dropped `अधिकम्`'s exchange moves exactly this one \
         field, and before this cycle the table had no field to move\n{dropped}"
    );

    // ── 2. A WRITTEN REGISTER HOLDS NO SLOT ─────────────────────────────────
    //
    // `योगः <करण>म् शून्यःन ०न ।` inserted directly above the branch: the करण
    // register now holds zero, and whatever slot it was loaded from is no longer
    // in it. The reader must say `None` rather than the slot it used to hold.
    let clobbered = text.replace(branch, &format!("योगः {l_reg}म् {ZERO}न ०न ।\n{branch}"));
    assert_ne!(clobbered, text, "the mutation must actually bite\n{text}");
    assert_eq!(
        read_rung(&clobbered).conditional_operands,
        vec![(None, Some(0))],
        "the करण was overwritten between its spill load and the branch, so it \
         is in NO slot — `None`, never the stale ८. MEASURED, not argued: with \
         the invalidation taken back out of `conditional_operand_slots` this \
         assertion reads `[(Some(8), Some(0))]`, the slot the register had \
         stopped holding\n{clobbered}"
    );

    // ── 3. THE CASE THAT MUST STILL BE REFUSED ──────────────────────────────
    let (refused, exit) = compile(&mut it, RUNG_98_NO_YADI_END, "क");
    assert_eq!(
        exit, PARSE_REFUSED,
        "rung 98 without the `यदि`'s own `इति` is refused at parse\n{refused}"
    );
    let read = read_rung(&refused);
    assert_eq!(
        (
            read.instructions,
            read.conditional,
            read.conditional_mnemonics.clone(),
            read.conditional_operands.clone()
        ),
        (0, 0, vec![], vec![]),
        "a refusal emits no instruction, so there is no conditional and NO PAIR \
         — an EMPTY list and never `[(Some(0), Some(0))]`. `०` is the first \
         parameter's slot, so a reader that defaulted to it would report a \
         refused source as a comparison of `व` against itself\n{refused}"
    );
}

/// **THE TWO MUTATIONS THAT SAY WHAT `conditional` READS, AND ONE OF THEM MUST
/// NOT MOVE THE NUMBER.**
///
/// A count is the least a table can rest on, so what it counts is demonstrated
/// on rung 101's own emitted text rather than argued from [`transfers`]'s
/// margin. Both mutations rewrite ONE line — `समलङ्घनम् स्थिर१न स्थिर०त्
/// कङपर्व३य् ।`, the `यदि`'s test — and nothing else.
///
/// **THE CASE THAT MUST STILL BE REFUSED IS THE FIRST ONE**, and it is a
/// refusal of a READING rather than of a source: a mnemonic this file has never
/// been taught must STILL COUNT AS CONDITIONAL. `अधिकलङ्घनम्` is not in this
/// file, not in the ladder and not in any list here; the reader must answer ONE
/// for it because it is not `लङ्घनम्`. Key the classification the other way —
/// on a list of conditional mnemonics — and this mutation answers ZERO, which
/// is the reading an emitter gets for free by lowering a `यदि` through a branch
/// nobody wrote down. The whole tuple is asserted and not the column alone,
/// because a reader that answered one here by misreading the line's SHAPE would
/// move `instructions` too.
///
/// **AND [`RungReading::conditional_mnemonics`] DOES NOT WEAKEN THAT REFUSAL,
/// WHICH IS THE THING TO CHECK WHEN A SPELLING ENTERS A TABLE OF SHAPES.** The
/// list is a projection of the same `Kind::Conditional` arm, so the untaught
/// `अधिकलङ्घनम्` is REPORTED rather than dropped: the mutated reading is the
/// base with ONE field moved and every other field — the count included —
/// identical. That is asserted as a whole-struct equality against a base with
/// the one field substituted, so a reader that quietly dropped an untaught
/// mnemonic (answering `[]`, the closed-set failure) reds here.
///
/// **AND THE SECOND SAYS THE COLUMN IS NOT FREE.** Rewrite the test as a
/// well-formed unconditional jump to the same target and rung 101's structural
/// shape becomes RUNG 97'S, exactly — `0 0 [] 1 [Forward] 0`. That is the
/// merge this cycle's column exists to break, reproduced by hand on the object
/// rather than asserted about the emitter, and it is why striking `conditional`
/// from the tuple puts the 97/101 pair back.
#[test]
fn an_untaught_conditional_mnemonic_still_counts_and_an_unconditional_one_makes_rung_101_read_as_rung_97()
 {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_101_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 101 emits\n{text}");

    // THE LINE BOTH MUTATIONS REWRITE, ASSERTED TO BE PRESENT EXACTLY ONCE —
    // a `replace` that matched nothing would leave the text untouched and both
    // assertions below would then be about the UNMUTATED object.
    const TEST_LINE: &str = "समलङ्घनम् स्थिर१न स्थिर०त् कङपर्व३य् ।";
    assert_eq!(
        text.lines().filter(|l| l.trim() == TEST_LINE).count(),
        1,
        "rung 101's `यदि` lowers to exactly one `समलङ्घनम्`, and both mutations \
         below are statements about THAT line\n{text}"
    );

    let base = read_rung(&text);
    assert_eq!(
        base.conditional, 1,
        "the unmutated object tests once\n{text}"
    );

    // ── THE REFUSAL: AN UNTAUGHT MNEMONIC IS STILL A TEST ───────────────────
    let untaught = text.replace(TEST_LINE, "अधिकलङ्घनम् स्थिर१न स्थिर०त् कङपर्व३य् ।");
    assert_ne!(untaught, text, "the mutation must have changed the text");
    let read = read_rung(&untaught);
    assert_eq!(
        read.conditional, base.conditional,
        "a conditional mnemonic this reader was never taught must still COUNT. \
         It is not `लङ्घनम्`, so it is not unconditional, and that is the whole \
         of the classification\n{untaught}"
    );
    assert_eq!(
        read,
        RungReading {
            conditional_mnemonics: vec!["अधिकलङ्घनम्".to_string()],
            ..base.clone()
        },
        "and it must read as the base with the SPELLING moved and nothing \
         else — the mnemonic list REPORTS a branch nobody taught it rather \
         than dropping it. A reader keyed on a closed list would answer an \
         empty `conditional_mnemonics` here, which is the reading an emitter \
         gets for free by lowering a `यदि` through a branch nobody wrote \
         down\n{untaught}"
    );
    let (_, unknown) = text_lines(&untaught);
    assert_eq!(
        unknown,
        Vec::<String>::new(),
        "and the counter still recognises the line, so the reading above is \
         about the MNEMONIC and not about a shape that fell out of the text"
    );

    // ── AND THE COST: STRIKE THE TEST AND 101 IS 97 ─────────────────────────
    let straightened = text.replace(TEST_LINE, "लङ्घनम् शून्यःम् कङपर्व३य् ।");
    assert_ne!(
        straightened, text,
        "the mutation must have changed the text"
    );
    let flat = read_rung(&straightened);
    assert_eq!(
        flat.conditional, 0,
        "an unconditional jump to the same target is not a test\n{straightened}"
    );

    let (r97, _) = compile(&mut it, RUNG_97_SOURCE, "क");
    assert_eq!(
        flat.structural(),
        read_rung(&r97).structural(),
        "and with the test gone rung 101's CONTROL SHAPE IS RUNG 97'S — the \
         merge the column exists to break, made by hand. The two objects are \
         still different lengths, which is exactly the separation this table \
         refuses to rest on\n{straightened}"
    );
    // THE BLOCKS DO NOT MOVE. `reachability` pushes every target of every
    // instruction and takes fall-through off the LAST one, which is still the
    // jump to कङपर्व४ — so this mutation changes the KIND of one edge and
    // nothing else, and the equality above is not an artefact of a block
    // falling out of the live set.
    assert_eq!(
        reachability(&straightened),
        reachability(&text),
        "the same blocks are live and the same two are dead\n{straightened}"
    );
}

/// **RUNG 101'S ९१४४, DERIVED FROM THE TEXT INSTEAD OF FROM AN IMAGE.**
///
/// Thirty-six instructions, no line unaccounted for.
///
/// **THE TWO CHAINS NO LONGER AGREE HERE, AND THAT IS `W-279`.** The `39` is
/// the figure `OCTETS_PER_INSTRUCTION`'s margin carries for this rung —
/// `39→९१५६`, taken from the RUST chain's `tools/demo.sh` and confirmed by a
/// native build in cycle 878 — and this test re-derived it from what the `.t1`
/// emitter places until the emitter stopped placing three instructions nothing
/// could reach. Rung 101 is where the `यदि` lowering paid MOST for that defect,
/// because BOTH its arms return: two join jumps into blocks the returns had
/// walked away from, and the join itself orphaned behind them. The Rust chain
/// still places all three.
///
/// AND THE COUNT IS ASSERTED BEFORE THE STATUS, not after: a fold that dropped
/// an instruction would move the status too, and reading the status alone
/// reports the fold as "the wrong number" rather than as a missing line.
#[test]
fn the_rung_101_source_emits_the_thirty_six_instructions_its_status_counts() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_101_SOURCE, "क");
    assert_eq!(
        exit, EMITTED,
        "the front half must reach the emitter; exit {exit} is a refusal\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    // THE UNRECOGNISED LINES FIRST, for the reason [`text_lines`] gives.
    assert_eq!(
        unknown,
        Vec::<String>::new(),
        "every non-blank line of rung 101's text must be a directive, a label \
         or an instruction, or a low count is a reader's gap\n{text}"
    );
    assert_eq!(
        insts.len(),
        36,
        "rung 101 is a two-armed branch over `स शेषः २` and the `.t1` chain \
         places THIRTY-SIX machine instructions for it. The ladder's registered \
         figure is THIRTY-NINE and it was this file's green until `W-279`; the \
         three that went are the two join jumps and the orphaned join\n{text}"
    );
    assert_eq!(
        rung_status(RUNG_101_BASE, insts.len()),
        9_144,
        "so the rung answers ९१४४ — ९००० plus 144 octets. ९१५६ is what the \
         native image answered and what it will answer again only if the dead \
         code comes back, so a green at ९१५६ here is now a REGRESSION and not \
         a confirmation"
    );
}

/// **AND THE CASE THAT MUST STILL BE REFUSED, WHICH IS THE ONE THE TABLE'S OWN
/// GUARD EXISTS FOR.** A source that does not reach the emitter answers ZERO
/// instructions, and zero instructions read as a control shape of all zeros.
///
/// The reason [`ladder_readings`]'s `exit == EMITTED` assertion is load-bearing
/// and not decorative is measured here rather than argued: **the all-zero shape
/// IS RUNG 103'S STRUCTURAL SHAPE**, exactly — no backward edges, no live
/// backward edges, no calls. The only reading that separates a refusal from
/// rung 103 is the instruction COUNT, and the count is precisely the reading
/// [`RungReading::structural`] strikes. So a refusal admitted to the table
/// passes [`the_seven_answered_rungs_are_separated_by_their_readings_and_no_two_collide`]
/// on its length and reds
/// [`striking_the_instruction_count_now_merges_no_pair_and_the_mnemonic_took_the_last_one`]
/// a cycle later, reported as a collision between two rungs that both compiled.
#[test]
fn a_refused_rung_101_answers_the_base_alone_and_its_all_zero_shape_is_rung_103s() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_101_NO_TAKEN_ARM_END, "क");
    assert_eq!(
        exit, PARSE_REFUSED,
        "striking the `इति` that closes the taken arm must be REFUSED, and at \
         PARSE — `अन्यथा` cannot open where no arm is open\n{text}"
    );

    let (insts, unknown) = text_lines(&text);
    assert_eq!(
        unknown,
        Vec::<String>::new(),
        "a refusal answers text with no line shape this reader cannot read\n{text}"
    );
    assert_eq!(
        insts.len(),
        0,
        "a refused source emits NO instructions\n{text}"
    );
    assert_eq!(
        rung_status(RUNG_101_BASE, insts.len()),
        9_000,
        "so a refused rung 101 answers ९००० alone and can never be read as ९१५६"
    );

    // THE SHAPE A REFUSAL WOULD ENTER THE TABLE AS, and the row it collides
    // with. This is the assertion that makes the guard's absence visible.
    let refused = read_rung(&text);
    assert_eq!(
        refused,
        RungReading {
            instructions: 0,
            backward: 0,
            live_backward: 0,
            live_backward_kind: vec![],
            calls: 0,
            call_dirs: vec![],
            conditional: 0,
            conditional_mnemonics: vec![],
            conditional_operands: vec![],
        },
        "a refusal reads as all zeros, which is the ABSENCE of a control shape \
         and not a control shape. NEITHER conditional reading rescues this: \
         rung 101 tests once on `समलङ्घनम्` and its REFUSAL tests zero times on \
         nothing at all, the same empty pair rung 103 and rung 97 answer — and \
         the OPERAND list is empty rather than `[(Some(0), Some(0))]`, because \
         `०` is a real slot and a defaulted zero would read a refusal as a \
         comparison of the first parameter against itself\n{text}"
    );
    let r103 = ladder_readings()
        .into_iter()
        .find(|(n, _)| *n == "103")
        .expect("rung 103 is on the table")
        .1;
    assert_eq!(
        refused.structural(),
        r103.structural(),
        "and the all-zero shape IS rung 103's — so the structural projection \
         cannot tell a refusal from the ladder's global-read rung, and only \
         `ladder_readings`'s `exit == EMITTED` can"
    );
    assert_ne!(
        refused.instructions, r103.instructions,
        "the instruction count is the ONLY reading that separates them, and it \
         is the one the structural projection strikes"
    );
}

/// **THE TABLE'S WEAKEST SEPARATION, READ OUT — AND THE READING IT DOES NOT
/// EARN.**
///
/// Rungs 98 and 101 differed in [`RungReading::structural`] ONLY in
/// `backward`, one dead edge against two, so the claim "these are two different
/// programs" rested on a COUNT of code nothing reaches. Cycle 910 read those
/// edges out — rung 98's one was `कङपर्व५ → कङपर्व४`; rung 101's two were
/// `कङपर्व६ → कङपर्व५` and `कङपर्व७ → कङपर्व५`, two unreachable tails jumping
/// back into one unreachable join, all three BRANCHES as predicted — and
/// concluded that the kinds earned no place in [`RungReading`].
///
/// # AND THEN THE EDGES WENT — `W-279`
///
/// The reading held its shape and its subject was deleted. Both rungs now
/// answer the EMPTY list, so the separation that rested on the count is gone
/// with it. The test is kept, inverted, because the inversion is the
/// load-bearing claim — a rung 98 or 101 that answers a non-empty dead-edge
/// list again has had the defect put back.
///
/// # AND WHAT SEPARATES THEM NOW IS NOT CONTROL
///
/// Every reading of CONTROL still merges the pair, and that is asserted below
/// rather than left as prose: no backward edges, one forward call, one
/// conditional, each. `structural()` no longer merges them only because
/// [`RungReading::conditional_mnemonics`] entered it — 98 tests with
/// `न्यूनलङ्घनम्` and 101 with `समलङ्घनम्` — which is a difference in what the
/// `यदि` COMPARES FOR and not in the graph. Said in both directions so the
/// non-merge is not read as a claim the CFGs came apart.
#[test]
fn rungs_98_and_101_carry_no_dead_backward_edge_and_only_their_mnemonic_tells_them_apart() {
    let mut it = load();
    let (branch, exit) = compile(&mut it, RUNG_98_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 98 emits\n{branch}");
    let (both_arms, exit) = compile(&mut it, RUNG_101_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 101 emits\n{both_arms}");

    let e98 = dead_backward_edges(&branch);
    let e101 = dead_backward_edges(&both_arms);
    println!("METRIC t1_rung98_dead_backward_edges {}", e98.len());
    println!("METRIC t1_rung101_dead_backward_edges {}", e101.len());

    // ── THE GUARD FIRST, AND IT IS THE CASE THAT MUST STILL BE REFUSED ──────
    //
    // An edge whose block is not in the DEAD list is not a dead edge, whatever
    // the filter said. Taken on both rungs before either list is pinned, so a
    // reader that admitted a live block could never reach the pins below.
    //
    // MEASURED, NOT ASSUMED: this loop is a TRIPWIRE HERE AND NOT THE CONTROL.
    // Every backward edge rungs 98 and 101 carry is already dead, so striking
    // `dead_backward_edges`'s membership check leaves this test GREEN — it is
    // [`a_backward_edge_out_of_a_reachable_block_is_not_read_as_dead`], on rung
    // 99's live loop edge, that reds. Said out loud because a reader who took
    // this loop for the refusal case would delete the test that is one.
    for (name, text, edges) in [("98", &branch, &e98), ("101", &both_arms, &e101)] {
        let (live, dead) = reachability(text);
        for (from, target, kind) in edges {
            assert!(
                dead.contains(from),
                "rung {name}: the edge {from} → {target} ({kind:?}) is read as \
                 DEAD, so {from} must be one of the blocks the walk never \
                 visited\nlive: {live:?}\ndead: {dead:?}\n{text}"
            );
            assert!(
                !live.contains(from),
                "rung {name}: {from} cannot be both\nlive: {live:?}\ndead: {dead:?}"
            );
        }
    }

    // ── RUNG 98'S ONE IS GONE ───────────────────────────────────────────────
    assert_eq!(
        e98,
        Vec::<(String, String, Kind)>::new(),
        "rung 98's single dead edge was `कङपर्व५ → कङपर्व४`, the join its \
         returning arm never fell into jumping BACK to the else target. \
         `W-279` stopped writing it and कङपर्व५ is now empty\n{branch}"
    );

    // ── AND RUNG 101'S TWO ──────────────────────────────────────────────────
    assert_eq!(
        e101,
        Vec::<(String, String, Kind)>::new(),
        "rung 101's TWO were `कङपर्व६ → कङपर्व५` and `कङपर्व७ → कङपर्व५`, two \
         unreachable tails jumping back into one unreachable join. Both are \
         gone, and so are the blocks that held them: the `यदि` arm discards the \
         block a returning arm opened instead of terminating it\n{both_arms}"
    );
    // AND THE BLOCKS THEY LEFT ARE NOT MERELY UNREAD — they are absent from the
    // text. A reader that had stopped following transfers would answer the two
    // empty lists above just as well; the label list is what tells them apart.
    for (name, text, gone) in [
        ("98", &branch, vec!["कङपर्व६"]),
        ("101", &both_arms, vec!["कङपर्व६", "कङपर्व७"]),
    ] {
        let defined = defined_labels(text);
        for label in gone {
            assert!(
                !defined.contains(&label.to_string()),
                "rung {name}: {label} held one of the dead edges and must no \
                 longer be emitted at all\n{defined:?}\n{text}"
            );
        }
    }

    // ── THE READING, WEIGHED ────────────────────────────────────────────────
    //
    // The DISTINCT kinds, which is what a `dead_backward_kind` reading would
    // add over the count already in the tuple.
    let kinds = |edges: &[(String, String, Kind)]| -> Vec<Kind> {
        let mut k: Vec<Kind> = Vec::new();
        for (_, _, kind) in edges {
            if !k.contains(kind) {
                k.push(*kind);
            }
        }
        k
    };
    assert_eq!(
        (kinds(&e98), kinds(&e101)),
        (Vec::<Kind>::new(), Vec::<Kind>::new()),
        "cycle 910's prediction — rung 98's one dead edge is a branch and rung \
         101's two arms each RETURN, so both of its dead edges are branches too \
         — was confirmed and is now unmeasurable: there are no dead edges left \
         on either rung to have a kind"
    );

    // THE RESULT, AS AN ASSERTION AND NOT AS PROSE: with the dead code gone the
    // two rungs' control shapes are IDENTICAL, so neither the dead-edge count
    // nor its kinds can separate them and `dead_backward_kind` still earns no
    // place in `RungReading` — now because there is nothing left for it to read.
    let (b, a) = (read_rung(&branch), read_rung(&both_arms));
    assert_eq!(
        (
            b.backward,
            b.live_backward,
            b.live_backward_kind.clone(),
            b.calls,
            b.call_dirs.clone(),
            b.conditional
        ),
        (
            a.backward,
            a.live_backward,
            a.live_backward_kind.clone(),
            a.calls,
            a.call_dirs.clone(),
            a.conditional
        ),
        "rungs 98 and 101 MERGE in every reading of control. They were apart on \
         the dead-edge count — one against two — and that was the table's own \
         stated weakest joint; `W-279` took the joint out and nothing about \
         their GRAPHS has told them apart since"
    );
    assert_ne!(
        b.conditional_mnemonics, a.conditional_mnemonics,
        "and the only structural reading that does tell them apart is the \
         conditional's MNEMONIC — `न्यूनलङ्घनम्` against `समलङ्घनम्`, the `<` \
         against the `समम्`. This is a claim about what the test COMPARES FOR, \
         not about control"
    );
    assert_ne!(
        b.structural(),
        a.structural(),
        "so `structural()` does not merge them, and it is that one reading \
         doing it"
    );
    assert_ne!(
        b.instructions, a.instructions,
        "the instruction count, 40 against 36, also separates them — and it is \
         the reading [`RungReading::structural`] strikes, which is why the \
         mnemonic had to be in the tuple for \
         [`the_seven_answered_rungs_are_separated_by_their_readings_and_no_two_collide`] \
         to survive the strike"
    );
}

/// **A BACKWARD EDGE OUT OF A REACHABLE BLOCK IS NOT A DEAD EDGE, AND THE
/// STRUCK READER IS DRIVEN RATHER THAN ARGUED ABOUT.**
///
/// [`dead_backward_edges`]'s membership check is the whole instrument: without
/// it the reader answers EVERY backward edge, and rung 99 — the `यावत्`, whose
/// edge the object really takes every iteration — would be reported as carrying
/// a dead one.
///
/// **AND SINCE `W-279` THAT IS A READING NO RUNG ON THE LADDER ANSWERS.** Rung
/// 98 used to: its dead jump out of `कङपर्व५` gave the struck reader a real
/// collision to make, and the margin here said so. The emitter fix removed
/// every dead backward edge from all seven rungs, so the struck reader would
/// now INVENT the ladder's only one, out of the one block on it that must never
/// be read that way. Weaker as a collision, stronger as a claim.
#[test]
fn a_backward_edge_out_of_a_reachable_block_is_not_read_as_dead() {
    let mut it = load();
    let (loop_, exit) = compile(&mut it, RUNG_99_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 99 emits\n{loop_}");
    let (branch, exit) = compile(&mut it, RUNG_98_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 98 emits\n{branch}");

    // THE READER WITH THE MEMBERSHIP CHECK STRUCK.
    let struck = |text: &str| -> Vec<(String, String, Kind)> {
        transfers(text)
            .into_iter()
            .filter(|t| t.dir == Dir::Backward)
            .map(|t| (t.from, t.target, t.kind))
            .collect()
    };

    // The edge is THERE — the checked reader is not losing it, it is placing it.
    assert_eq!(
        struck(&loop_).len(),
        1,
        "rung 99 has exactly one backward edge\n{loop_}"
    );
    let (live, _) = reachability(&loop_);
    assert!(
        live.contains(&struck(&loop_)[0].0),
        "and the block it leaves is ON the walk — the object takes it\n{loop_}"
    );
    assert_eq!(
        dead_backward_edges(&loop_),
        Vec::<(String, String, Kind)>::new(),
        "so the checked reader answers NO dead edge for the ladder's loop\n{loop_}"
    );

    // AND WHAT STRIKING THE CHECK WOULD COST, stated against the whole ladder
    // rather than against rung 98 alone, because rung 98 no longer has a dead
    // edge to be confused with.
    assert_eq!(
        dead_backward_edges(&branch),
        Vec::<(String, String, Kind)>::new(),
        "rung 98 carries no dead backward edge since `W-279` — it carried the \
         only one this control could be read against\n{branch}"
    );
    assert_eq!(
        (struck(&loop_).len(), dead_backward_edges(&loop_).len()),
        (1, 0),
        "so with the check struck rung 99 answers ONE dead edge where the \
         checked reader answers none, and ONE is a reading no rung on this \
         ladder now gives: the mutation does not over-count an existing \
         population, it invents the whole of it out of the ladder's only LIVE \
         backward branch\nrung 99: {:?}",
        struck(&loop_)
    );
}

/// **WHAT EVERY RUNG ON THE LADDER PAYS FOR CODE NOTHING REACHES, IN ONE
/// TABLE — AND SINCE `W-279` THE ANSWER IS NOTHING, ON ALL SEVEN.**
///
/// Two rungs were already priced one at a time
/// (`t1_rung98_unreachable_instructions` and its rung-100 twin, ONE each), and
/// pricing them one at a time is what let the ledger write down that rung 100
/// "was never priced" — it was, at 2286 and at 2696 of this file. So the
/// reading is taken OVER [`ladder`], the same seven sources the separation
/// table reads, and a rung cannot be added to one without entering the other.
///
/// THE PREDICTION CYCLE 911 REGISTERED AND THIS TABLE CONFIRMED: 0, 1, 0, 1,
/// **3**, 0, 0 — rung 101's dead blocks were `कघपर्व१` (empty), `कङपर्व५`,
/// `कङपर्व६` and `कङपर्व७`, each of the last three holding ONE unconditional
/// `लङ्घनम्`, and the both-armed `यदि` was what placed them.
///
/// **THE FIX THE TABLE WAS BUILT TO JUDGE HAS LANDED AND THE TABLE IS ALL
/// ZEROS.** Rung 101 went 3→0, rungs 98 and 100 went 1→0, and the four rungs
/// that never paid are unmoved. THE TABLE IS KEPT, and this is the reason it
/// answers [`DeadWeight`] and not a bare `usize`: zero is now the expected
/// answer everywhere, which is exactly when a reader that has silently stopped
/// reading looks like a passing one. `Unread` is what separates the two, and
/// [`a_rung_with_no_dead_block_answers_zero_and_a_refusal_answers_unread`] is
/// the control that keeps the separation honest.
#[test]
fn the_dead_weight_of_every_rung_is_priced_in_one_table_and_the_both_armed_yadi_pays_most() {
    let mut it = load();
    let mut table: Vec<(&'static str, DeadWeight)> = Vec::new();
    for (name, src) in ladder() {
        let (text, exit) = compile(&mut it, src, "क");
        assert_eq!(
            exit, EMITTED,
            "rung {name}'s source must reach the emitter; exit {exit} is a \
             refusal and a refusal is NOT a zero — see \
             [`a_rung_with_no_dead_block_answers_zero_and_a_refusal_answers_unread`]\n{text}"
        );
        let w = dead_instructions(&text);
        match w {
            DeadWeight::Dead(n) => {
                println!("METRIC t1_rung{name}_unreachable_instructions {n}")
            }
            DeadWeight::Unread => {
                println!("METRIC t1_rung{name}_unreachable_instructions UNREAD")
            }
        }
        table.push((name, w));
    }

    assert_eq!(
        table,
        vec![
            ("97", DeadWeight::Dead(0)),
            ("98", DeadWeight::Dead(0)),
            ("99", DeadWeight::Dead(0)),
            ("100", DeadWeight::Dead(0)),
            ("101", DeadWeight::Dead(0)),
            ("102", DeadWeight::Dead(0)),
            ("103", DeadWeight::Dead(0)),
        ],
        "the ladder's dead weight, rung by rung"
    );
}

/// **RUNG 101'S CHAIN OF THREE TRAMPOLINES IS GONE, AND THE FOUR BLOCKS ARE
/// TWO — BOTH EMPTY.**
///
/// This test was cycle 911's reading of the defect and is now the falsifier for
/// its fix, on the same rung and with the same instrument. What it read:
///
/// ```text
///   कङपर्व६ ──▶ कङपर्व५ ──▶ कङनिर्गम        BEFORE `W-279`
///   कङपर्व७ ──▶ कङपर्व५
/// ```
///
/// Three unreachable blocks of one unconditional `लङ्घनम्` each — a join per
/// returning arm, written into the empty block the `प्रत्यागमनम्` had already
/// opened, and the join `कङपर्व५` orphaned behind them. Nothing reached any of
/// the three, because the two LIVE arms `कङपर्व३` and `कङपर्व४` each jump
/// straight to `कङनिर्गम`.
///
/// What it reads now: `कङपर्व६` and `कङपर्व७` are not emitted at all, and
/// `कङपर्व५` is EMPTY — its `लङ्घनम्` to the exit is elided because the exit is
/// the block that follows it. The two live arms still leave for `कङनिर्गम`
/// directly, which is the half of the old reading that was never the defect and
/// must not move.
///
/// **THE CASE THAT MUST STILL BE REFUSED, AND IT IS WHY THIS KEEPS THE EMPTY
/// JOIN RATHER THAN ASSERTING IT AWAY.** `कङपर्व५` is still ALLOCATED, opened
/// and closed — the arm discards only the block a return opened, never the
/// join — so the routine's block run stays dense and `रेखीकरणम्` finds no hole
/// in it. A fix that discarded the join too would answer a shorter text here
/// and a wrong `पर्वसंख्यान` nothing on this ladder would show.
#[test]
fn rung_101s_dead_trampolines_are_gone_and_the_join_it_orphaned_is_an_empty_block() {
    let mut it = load();
    let (text, exit) = compile(&mut it, RUNG_101_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 101 emits\n{text}");

    let (live, dead) = reachability(&text);
    assert_eq!(
        live.len() + dead.len(),
        blocks(&text).len(),
        "every block is one or the other"
    );
    assert_eq!(
        dead,
        vec!["कघपर्व१".to_string(), "कङपर्व५".to_string()],
        "TWO blocks are unreachable where there were four: the empty one `घ` \
         opens after its return, and the join `ङ`'s two returning arms never \
         fall into. `कङपर्व६` and `कङपर्व७` were the other two\n{:#?}",
        blocks(&text)
    );

    // ── AND BOTH CARRY NOTHING, which is the whole of the fix ───────────────
    assert_eq!(
        blocks(&text)
            .iter()
            .filter(|b| dead.contains(&b.label))
            .map(|b| (b.label.clone(), b.insts.len()))
            .collect::<Vec<_>>(),
        vec![("कघपर्व१".to_string(), 0), ("कङपर्व५".to_string(), 0)],
        "no instruction sits in a block nothing reaches. Before `W-279` the \
         three `ङ` blocks held one `लङ्घनम्` each\n{:#?}",
        blocks(&text)
    );

    // ── THE TWO BLOCKS THAT WENT, NAMED RATHER THAN COUNTED ─────────────────
    //
    // The count above would be satisfied by an emitter that kept them and
    // emptied them; it is the LABEL LIST that says they are not placed.
    let defined = defined_labels(&text);
    for label in ["कङपर्व६", "कङपर्व७"] {
        assert!(
            !defined.contains(&label.to_string()),
            "{label} held one of the trampolines and must not be emitted at \
             all — the `यदि` arm hands its id back to `अग्रिमपर्वाङ्क` instead \
             of terminating it\n{defined:?}\n{text}"
        );
    }
    assert!(
        defined.contains(&"कङपर्व५".to_string()),
        "but the JOIN is still placed — it is opened and closed like every \
         other id, which is what keeps the routine's block run dense\n{defined:?}"
    );

    // ── THE LIVE ARMS ARE UNTOUCHED, which is the half that was never wrong ─
    assert!(
        live.contains(&"कङपर्व३".to_string()) && live.contains(&"कङपर्व४".to_string()),
        "both arms of the `यदि` are reached\n{:#?}",
        blocks(&text)
    );
    assert_eq!(
        blocks(&text)
            .iter()
            .filter(|b| b.label == "कङपर्व३" || b.label == "कङपर्व४")
            .map(|b| {
                // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
                let f: Vec<&str> = b.insts[b.insts.len() - 1].split_whitespace().collect();
                f[f.len() - 2].trim_end_matches("य्").to_string()
            })
            .collect::<Vec<_>>(),
        vec!["कङनिर्गम".to_string(), "कङनिर्गम".to_string()],
        "and each still leaves for `कङनिर्गम` DIRECTLY — the arms return, they \
         do not fall into the join, and that was never the defect\n{:#?}",
        blocks(&text)
    );

    // ── AND THE TWO READINGS THAT PRICED IT, BOTH AT ZERO ───────────────────
    println!(
        "METRIC t1_rung101_dead_backward_edges {}",
        dead_backward_edges(&text).len()
    );
    assert_eq!(
        dead_backward_edges(&text).len(),
        0,
        "cycle 910's reading answered TWO — `कङपर्व६` and `कङपर्व७` jumping \
         back into `कङपर्व५` — and both edges are gone with their blocks\n{:?}",
        dead_backward_edges(&text)
    );
    assert_eq!(
        dead_instructions(&text),
        DeadWeight::Dead(0),
        "and the dead WEIGHT, which priced the third instruction the backward \
         reading could not see, is zero as well. `Dead(0)` and NOT `Unread`: \
         the text was read whole and 36 instructions were found in it"
    );
    assert_eq!(
        text_lines(&text).0.len(),
        36,
        "which is the same 36 \
         [`the_rung_101_source_emits_the_thirty_six_instructions_its_status_counts`] \
         counts, taken here so this test cannot go green on an empty text"
    );
}

// ── THE CASES `W-279`'S GUARD MUST STILL EMIT ───────────────────────────────
//
// The fix reads one bit — did the statement just built TERMINATE the block in
// hand — and drops the arm's join `लङ्घन` when it did. Every rung on the ladder
// whose `यदि` has an arm at all has a RETURNING arm, so all seven exercise the
// dropping half and NONE exercises the keeping half. A guard with its condition
// inverted, or one that never cleared its flag, would leave the whole ladder
// green and lower every branch in the corpus into a fall-through.
//
// These three sources are that half. They are hand-written rather than taken
// off `shrinkhala.t1`, and they are the only sources in this file that are: no
// rung compiles a `यदि` that falls through, which is the entire reason they
// exist. Each is asserted on the JOIN — placed, reached, and reached the way
// the lowering intends.

/// A `यदि` whose arms both ASSIGN. Neither terminates, so BOTH need the join.
const FALLTHROUGH_YADI: &str = "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ आदाय स ॱॱ न६४ ददाति न६४ आदि चरः र ॱॱ न६४ भवति ० । \
यदि स अधिकम् २ आदि र भवति १ । इति अन्यथा आदि र भवति २ । इति प्रत्यागमनम् र । इति";

/// One arm returns and one falls through — the guard read PER ARM, not per
/// `यदि`. Rungs 98 and 100 have this shape with no `अन्यथा` at all; here the
/// else body exists and must still be joined.
const ONE_ARM_RETURNS_YADI: &str = "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ आदाय स ॱॱ न६४ ददाति न६४ आदि चरः र ॱॱ न६४ भवति ० । \
यदि स अधिकम् २ आदि प्रत्यागमनम् ९ । इति अन्यथा आदि र भवति २ । इति प्रत्यागमनम् र । इति";

/// An outer arm whose LAST STATEMENT is a `यदि` with both arms returning. The
/// inner arm clears the flag before answering, so the outer arm still writes
/// its join — which it must, because the inner join is a block the outer `यदि`
/// did not build and cannot reason about.
const NESTED_YADI: &str = "मण्डलम् क ॥ सार्वजनिक वृत्तिः घ आदाय स ॱॱ न६४ ददाति न६४ आदि चरः र ॱॱ न६४ भवति ० । \
यदि स अधिकम् २ आदि यदि स अधिकम् ५ आदि प्रत्यागमनम् ७ । इति अन्यथा आदि प्रत्यागमनम् ८ । इति \
इति अन्यथा आदि र भवति ३ । इति प्रत्यागमनम् र । इति";

/// The `पर्व` numbers a routine's non-entry blocks carry, in emission order.
///
/// `कार्यक्रमरचना` records a routine's blocks as the run
/// `प्रवेश ..= अग्रिमपर्वाङ्क` and `रेखीकरणम्` walks it, so an id numbered and
/// never written is a HOLE the walk reads as a block. `W-279`'s discard hands
/// the id back rather than skipping the close, and this is what says it did:
/// the numbers must be CONTIGUOUS AND ASCENDING.
///
/// MEASURED, NOT ARGUED. With the guard's condition inverted — the arm dropping
/// its join when it did NOT return, and keeping it when it did — `मण्डलसङ्कलनम्`
/// does not merely emit a worse object: it dies inside `उत्सर्जनॱरेखीकरणम्` with
/// ``ॱ आज्ञारम्भ` read from Nil`, the walk reading a `पर्वकोश` slot the builder
/// never wrote. That is the hole, and it is what the discard is for.
fn block_numbers(text: &str, routine: &str) -> Vec<u64> {
    let prefix = format!("{routine}पर्व");
    defined_labels(text)
        .into_iter()
        .filter_map(|l| l.strip_prefix(&prefix).and_then(devanagari_int))
        .collect()
}

/// **A `यदि` WHOSE ARMS FALL THROUGH STILL GETS ITS JOIN, AND `W-279` IS READ
/// AGAINST THE HALF OF ITSELF NO RUNG COMPILES.**
///
/// Three shapes, one assertion each on the join, plus the block-run invariant
/// the discard could break silently. The `assign`/`fall-through` arms here are
/// what every real `.t1` module is full of — `ir.t1`'s own `यदि` arms assign
/// and fall through — so a regression would red the corpus census rather than
/// this file; it reds here FIRST, in seconds, and names the arm.
#[test]
fn an_arm_that_falls_through_still_takes_its_join_and_the_block_run_stays_dense() {
    let mut it = load();

    // ── BOTH ARMS ASSIGN: the taken arm JUMPS to the join, the else arm FALLS
    // into it because the join is the block emitted next. Two different ways of
    // reaching one block, and dropping either is a fall-through into the wrong
    // code rather than a shorter object.
    let (text, exit) = compile(&mut it, FALLTHROUGH_YADI, "क");
    assert_eq!(exit, EMITTED, "the fall-through source emits\n{text}");
    let (live, dead) = reachability(&text);
    assert!(
        live.contains(&"कघपर्व३".to_string()),
        "the join कघपर्व३ is REACHED\n{:#?}",
        blocks(&text)
    );
    assert_eq!(
        blocks(&text).iter().find(|b| b.label == "कघपर्व१").map(|b| {
            // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
            let f: Vec<&str> = b.insts[b.insts.len() - 1].split_whitespace().collect();
            f[f.len() - 2].trim_end_matches("य्").to_string()
        }),
        Some("कघपर्व३".to_string()),
        "and the taken arm ENDS with the join `लङ्घन` — this is the instruction \
         `W-279` drops when the arm returned, and here it must stand\n{:#?}",
        blocks(&text)
    );
    assert_eq!(
        dead,
        vec!["कघपर्व४".to_string()],
        "only the empty block the routine's own return opens is dead\n{:#?}",
        blocks(&text)
    );
    assert_eq!(dead_instructions(&text), DeadWeight::Dead(0));
    assert_eq!(block_numbers(&text, "कघ"), vec![1, 2, 3, 4], "dense run");

    // ── ONE ARM RETURNS, ONE DOES NOT: the guard is per ARM.
    let (text, exit) = compile(&mut it, ONE_ARM_RETURNS_YADI, "क");
    assert_eq!(exit, EMITTED, "the mixed source emits\n{text}");
    let (live, _) = reachability(&text);
    assert!(
        live.contains(&"कघपर्व३".to_string()),
        "the join is still reached — the else arm falls into it\n{:#?}",
        blocks(&text)
    );
    assert_eq!(
        blocks(&text)
            .iter()
            .find(|b| b.label == "कघपर्व१")
            .map(|b| b.insts.len()),
        Some(3),
        "the RETURNING arm is three instructions and ends at the exit: the \
         value, the result register, the jump out. No join `लङ्घन` follows \
         it\n{:#?}",
        blocks(&text)
    );
    assert_eq!(
        dead_instructions(&text),
        DeadWeight::Dead(0),
        "so the mixed shape places no dead code either\n{:#?}",
        blocks(&text)
    );
    assert_eq!(block_numbers(&text, "कघ"), vec![1, 2, 3, 4], "dense run");

    // ── A NESTED `यदि` WHOSE ARMS BOTH RETURN, AS THE OUTER ARM'S LAST
    // STATEMENT. The outer arm must STILL write its join, because what is in
    // hand when it looks is the INNER join — a live-by-construction block the
    // outer `यदि` did not build. Dropping it would fall the inner join through
    // into whatever the emitter placed next.
    let (text, exit) = compile(&mut it, NESTED_YADI, "क");
    assert_eq!(exit, EMITTED, "the nested source emits\n{text}");
    assert_eq!(
        blocks(&text)
            .iter()
            .find(|b| b.label == "कघपर्व६")
            .map(|b| b.insts.clone()),
        Some(vec![format!("लङ्घनम् {ZERO}म् कघपर्व३य् ।")]),
        "कघपर्व६ is the INNER join and it carries the OUTER `यदि`'s join \
         `लङ्घन` to कघपर्व३ — the instruction the flag's clearing keeps\n{:#?}",
        blocks(&text)
    );
    assert_eq!(
        block_numbers(&text, "कघ"),
        vec![1, 2, 3, 4, 5, 6, 7],
        "dense run"
    );

    // AND THE ONE DEAD INSTRUCTION THIS SHAPE STILL PLACES, ASSERTED RATHER
    // THAN LEFT FOR A LATER CYCLE TO DISCOVER. The guard reads ONE level: the
    // inner `यदि` answers "I did not terminate" because its join is in hand,
    // and that is true of the join as a block and false of this program, whose
    // inner arms both left. Propagating it is a second row and not this one —
    // the conservative answer is always CORRECT, only sometimes long.
    assert_eq!(
        dead_instructions(&text),
        DeadWeight::Dead(1),
        "one instruction nothing reaches — कघपर्व६'s जump — because both inner \
         arms return and the outer arm cannot see that through the inner \
         join\n{:#?}",
        blocks(&text)
    );
}

/// **A RUNG WITH NO DEAD BLOCK ANSWERS ZERO; A SOURCE THAT NEVER EMITTED
/// ANSWERS `Unread`. THE READER THAT CANNOT TELL THEM APART IS DRIVEN HERE.**
///
/// This is the case the reading above exists to keep honest. The number
/// `t1_rung101_unreachable_instructions` is a number a FIX to the branch
/// lowering would drive to zero — which makes zero the single most dangerous
/// answer this instrument can give, because a reader that has stopped reading
/// gives the same one. Rung 103 emits eighteen instructions and no unreachable
/// block: its honest answer is `Dead(0)`. [`RUNG_101_NO_TAKEN_ARM_END`] is
/// refused at parse and emits no `.text` at all: its honest answer is
/// `Unread`.
///
/// THE MUTATION, not the argument: `struck` is [`dead_instructions`] with the
/// three states collapsed to the bare `usize` rungs 98 and 100 are priced
/// with, and it hands back ZERO for both — so under it the day the emitter
/// stops emitting and the day the emitter is fixed are one reading.
#[test]
fn a_rung_with_no_dead_block_answers_zero_and_a_refusal_answers_unread() {
    let mut it = load();
    let (clean, exit) = compile(&mut it, RUNG_103_SOURCE, "क");
    assert_eq!(exit, EMITTED, "rung 103 emits\n{clean}");
    let (refused, exit) = compile(&mut it, RUNG_101_NO_TAKEN_ARM_END, "क");
    assert_eq!(
        exit, PARSE_REFUSED,
        "the taken arm's missing `इति` must still be refused at parse\n{refused}"
    );

    // THE READ RUNG. Eighteen instructions and none of them dead.
    assert_eq!(
        text_lines(&clean).0.len(),
        18,
        "rung 103's eighteen — the zero below is about its blocks and not \
         about an empty text\n{clean}"
    );
    assert_eq!(
        dead_instructions(&clean),
        DeadWeight::Dead(0),
        "rung 103 was READ and carries no unreachable instruction\n{:#?}",
        blocks(&clean)
    );

    // THE UNREAD ONE. No `.text`, so no blocks, so nothing to price.
    assert_eq!(
        blocks(&refused),
        Vec::<Block>::new(),
        "a source refused at parse emits no basic block\n{refused}"
    );
    assert_eq!(
        dead_instructions(&refused),
        DeadWeight::Unread,
        "so it is UNREAD — not a rung carrying zero dead instructions\n{refused}"
    );
    assert_ne!(
        dead_instructions(&clean),
        dead_instructions(&refused),
        "and the two are APART, which is the whole reason the reader has a \
         third state"
    );

    // ── WHAT COLLAPSING THE THREE STATES TO A COUNT WOULD COST ──────────────
    let struck = |text: &str| -> usize {
        let (_, dead) = reachability(text);
        blocks(text)
            .iter()
            .filter(|b| dead.contains(&b.label))
            .map(|b| b.insts.len())
            .sum()
    };
    assert_eq!(
        struck(&clean),
        0,
        "struck, the clean rung answers zero — correctly"
    );
    assert_eq!(
        struck(&refused),
        struck(&clean),
        "and struck, THE REFUSAL ANSWERS THE SAME ZERO. A fix to the branch \
         lowering is reported by this number going to zero, so under the \
         struck reader a chain that stopped emitting reports the fix as \
         landed\nrefused: {refused}"
    );
}

// ── W-279 — WHICH COMPARISON DOES A `यदि` ACTUALLY LOWER TO? ────────────────
//
// The ladder's `conditional_mnemonics` column reads seven rungs and sees TWO
// of `ir.t1:266-271`'s six `CmpOp`s — `न्यूनलङ्घनम्` and `समलङ्घनम्`. Nothing
// on this tree asked the chain for the other four, and nothing on this tree
// asked it for the OPERAND ORDER of any of them.
//
// **THE ORDER IS NOT OPTIONAL AND IT IS NOT A REFINEMENT.** `ir.t1:2191-2193`
// lowers `अधिकम्` as `न्यूनतुलनाभेद` WITH ITS OPERANDS EXCHANGED, so `व अधिकम् द`
// and `व न्यूनम् द` emit the SAME mnemonic and differ only in which register is
// the करण. A mnemonic-only reading — which is every reading in this file until
// here — scores a lowering that forgot the exchange as correct, and that
// lowering computes `<` where the source wrote `>`.

/// A one-armed `यदि` over TWO PARAMETERS, one comparison per instantiation.
///
/// **TWO PARAMETERS AND NOT A PARAMETER AGAINST A LITERAL**, because a literal
/// is materialised into a register of its own and the two operands would then
/// be told apart by which one was CONSTRUCTED rather than by which one the
/// source wrote first. Two parameters give two spill slots, `०` and `८`, and
/// the slot is the source position — see [`parameter_slots`].
///
/// The shape is [`RUNG_98_SOURCE`]'s second routine with the call removed:
/// one `यदि`, no `अन्यथा`, each arm returning a parameter. Rung 98 itself
/// compares with `अधिकम्`, which is exactly the operator whose lowering this
/// fixture exists to read, so the shape is the ladder's and not an invention.
fn comparison_source(left: &str, op: &str, right: &str) -> String {
    format!(
        "मण्डलम् क ॥ सार्वजनिक वृत्तिः ङ आदाय व ॱॱ न६४ ऽ द ॱॱ न६४ ददाति न६४ आदि \
यदि {left} {op} {right} आदि प्रत्यागमनम् व । इति प्रत्यागमनम् द । इति"
    )
}

/// Where each parameter LIVES — the spill slot the prologue writes `अर्थ<i>`
/// into, in argument order.
///
/// The prologue emits the pair `योगः <r>म् अर्थ<i>न ०न ।` then
/// `निधानम् <SP>य् <off>न <r>न ।`, so the register is a courier and the SLOT is
/// the parameter's identity. Read as a pair and not as two independent scans:
/// the same scratch register carries every argument in turn, so a reader that
/// collected stores alone would attribute all of them to one parameter.
fn parameter_slots(text: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let mut carried: Option<(String, u64)> = None;
    for l in text.lines() {
        // SPLIT ON SPACES, NEVER ON A WORD BOUNDARY.
        let f: Vec<&str> = l.split_whitespace().collect();
        match f.as_slice() {
            // `योगः <r>म् अर्थ<i>न ०न ।` — argument i is in flight in <r>.
            ["योगः", dst, src, "०न", "।"] if src.starts_with("अर्थ") && src.ends_with('न') =>
            {
                let Some(reg) = dst.strip_suffix("म्") else {
                    continue;
                };
                let Some(idx) = src
                    .strip_prefix("अर्थ")
                    .and_then(|s| s.strip_suffix('न'))
                    .and_then(devanagari_int)
                else {
                    continue;
                };
                carried = Some((reg.to_string(), idx));
            }
            // `निधानम् <SP>य् <off>न <r>न ।` — and it lands here.
            ["निधानम्", base, off, src, "।"] if *base == format!("{SP}य्") => {
                let (Some(reg), Some(slot)) = (
                    src.strip_suffix('न'),
                    off.strip_suffix('न').and_then(devanagari_int),
                ) else {
                    continue;
                };
                if let Some((carrier, idx)) = &carried
                    && carrier == reg
                {
                    // Positions arrive in order; a gap would be a defect and is
                    // left VISIBLE rather than padded over.
                    if *idx as usize == out.len() {
                        out.push(slot);
                    }
                    carried = None;
                }
            }
            _ => carried = None,
        }
    }
    out
}

/// **EVERY COMPARISON THE FRONT END HAS, LOWERED — AND `अधिकम्` IS `न्यून` WITH
/// ITS OPERANDS EXCHANGED.**
///
/// `spec/grammar-t1.ebnf:645` gives FIVE `compare_op`s and `ir.t1:266-271`
/// declares SIX `CmpOp`s, so the map is not a bijection in either direction and
/// both halves of that are asserted here:
///
/// ```text
///   समम्        समलङ्घनम्        (प१, प२)      predicted, measured
///   असमम्       विषमलङ्घनम्      (प१, प२)      predicted, measured
///   न्यूनम्      न्यूनलङ्घनम्      (प१, प२)      predicted, measured
///   बृहत्समम्    अन्यूनलङ्घनम्     (प१, प२)      predicted, measured
///   अधिकम्      न्यूनलङ्घनम्      (प२, प१)      predicted, measured — EXCHANGED
///   अचिह्नन्यून   — no spelling, refused at `अर्थ`, no `.text`
///   अचिह्नान्यून  — no spelling, refused at `अर्थ`, no `.text`
/// ```
///
/// **THE PREDICTIONS WERE REGISTERED BEFORE THE RUN**, off `ir.t1:2187-2193`
/// and `yantrotsarjana.t1:373-378`, and all five held. The one prediction that
/// did NOT hold was a corollary: "`व न्यूनम् द` and `द अधिकम् व` emit the
/// IDENTICAL line". They do not — they emit `न्यूनलङ्घनम् स्थिर०न स्थिर१त्` and
/// `न्यूनलङ्घनम् स्थिर१न स्थिर०त्`, because the operand LOADS are placed in source
/// order and the exchange then falls on different registers. Resolved to slots
/// the two agree exactly, which is why this reader resolves and why a reader
/// comparing branch lines as TEXT would have called that pair a defect.
///
/// **THE MEASURED NEGATIVE, RECORDED AND NOT INVENTED.** The two unsigned
/// `CmpOp`s are unreachable from the front end: `ir.t1:3148` builds
/// `अचिह्नन्यूनतुलनाभेद` from inside the chain and no `compare_op` spells either,
/// so this test does not synthesize a source for them — it asserts that the
/// spellings one would write are REFUSED.
#[test]
fn each_comparison_the_front_end_has_lowers_to_its_own_mnemonic_and_adhikam_exchanges_its_operands()
{
    let mut it = load();

    // The slot a parameter lives in, taken from the text rather than assumed,
    // so the pairs below are stated in the source's own terms: `०` is `व` and
    // `८` is `द`.
    let (baseline, exit) = compile(&mut it, &comparison_source("व", "न्यूनम्", "द"), "क");
    assert_eq!(exit, EMITTED, "the fixture emits\n{baseline}");
    assert_eq!(
        parameter_slots(&baseline),
        vec![0, 8],
        "two parameters, `व` in slot ० and `द` in slot ८\n{baseline}"
    );

    // ── THE FIVE THE GRAMMAR HAS ────────────────────────────────────────────
    for (op, mnemonic, karana, apadana) in [
        ("समम्", "समलङ्घनम्", 0, 8),
        ("असमम्", "विषमलङ्घनम्", 0, 8),
        ("न्यूनम्", "न्यूनलङ्घनम्", 0, 8),
        ("बृहत्समम्", "अन्यूनलङ्घनम्", 0, 8),
        ("अधिकम्", "न्यूनलङ्घनम्", 8, 0),
    ] {
        let (text, exit) = compile(&mut it, &comparison_source("व", op, "द"), "क");
        assert_eq!(exit, EMITTED, "`व {op} द` emits\n{text}");
        assert_eq!(
            read_rung(&text).conditional_mnemonics,
            vec![mnemonic.to_string()],
            "`व {op} द` lowers to exactly one conditional, and it is \
             `{mnemonic}`\n{text}"
        );
        assert_eq!(
            conditional_operand_slots(&text),
            vec![(mnemonic.to_string(), Some(karana), Some(apadana))],
            "`व {op} द` compares slot {karana} AGAINST slot {apadana} — ० is \
             `व` and ८ is `द`, so a pair the other way round is the exchange \
             done wrong\n{text}"
        );
    }

    // ── AND THE EXCHANGE IS WHAT MAKES THE TWO SPELLINGS ONE COMPARISON ─────
    //
    // `द अधिकम् व` is `व न्यूनम् द`. It is asserted through the SLOTS and not
    // through the branch line, because the lines differ: the loads are placed
    // in source order, so the same comparison lands on opposite registers.
    let (mirrored, exit) = compile(&mut it, &comparison_source("द", "अधिकम्", "व"), "क");
    assert_eq!(exit, EMITTED, "`द अधिकम् व` emits\n{mirrored}");
    assert_eq!(
        conditional_operand_slots(&mirrored),
        conditional_operand_slots(&baseline),
        "`द अधिकम् व` IS `व न्यूनम् द` once the registers are resolved\n\
         mirrored: {mirrored}"
    );
    assert_ne!(
        text_lines(&mirrored)
            .0
            .iter()
            .filter(|l| l.starts_with("न्यूनलङ्घनम्"))
            .collect::<Vec<_>>(),
        text_lines(&baseline)
            .0
            .iter()
            .filter(|l| l.starts_with("न्यूनलङ्घनम्"))
            .collect::<Vec<_>>(),
        "and the two branch LINES are apart, which is why this reading \
         resolves registers to slots instead of comparing text"
    );

    // ── THE CASE THAT MUST STILL BE REFUSED ─────────────────────────────────
    //
    // The two unsigned conditions have no spelling. A chain that lowered an
    // unknown comparison to a DEFAULT mnemonic — `यन्त्रशाखापदम्`'s last arm
    // (`yantrotsarjana.t1:378`) answers `अचिह्नान्यूनलङ्घनम्` for any kind it was
    // not taught — would emit a branch here, and the object would compute a
    // comparison nobody wrote.
    for op in ["अचिह्नन्यूनम्", "अचिह्नान्यूनम्"]
    {
        let (text, exit) = compile(&mut it, &comparison_source("व", op, "द"), "क");
        assert_eq!(
            exit, RESOLVE_REFUSED,
            "`{op}` is not a `compare_op`, so `अर्थ` refuses it\n{text}"
        );
        assert_eq!(
            text_lines(&text).0.len(),
            0,
            "and it emits NO instruction — never a branch on a default \
             mnemonic\n{text}"
        );
        assert_eq!(
            conditional_operand_slots(&text),
            Vec::new(),
            "so there is no conditional to read\n{text}"
        );
    }

    // ── AND THE READER IS DRIVEN, NOT TRUSTED ───────────────────────────────
    //
    // The mutation control this file uses everywhere else: take the text the
    // chain really emitted and exchange the two operands BY HAND. If the reader
    // reports the same pair for both, it is reading the mnemonic and calling it
    // an operand order, and every assertion above is vacuous.
    let swapped = baseline.replace("न्यूनलङ्घनम् स्थिर०न स्थिर१त्", "न्यूनलङ्घनम् स्थिर१न स्थिर०त्");
    assert_ne!(
        swapped, baseline,
        "the mutation must actually bite\n{baseline}"
    );
    assert_eq!(
        conditional_operand_slots(&swapped),
        vec![("न्यूनलङ्घनम्".to_string(), Some(8), Some(0))],
        "a hand-exchanged branch is reported EXCHANGED — the reading is about \
         operands and not about the mnemonic\n{swapped}"
    );
}
