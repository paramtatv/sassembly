//! `W-235` — EMITTER LANE R1: the T1 IR onto the machine, as T0 ASSEMBLY TEXT.
//!
//! The design is `research/25-t1-to-riscv-emitter-design.md` §2, and every
//! decision below cites its section. The one that shapes the whole file is
//! §2.1: this emitter writes **T0 source** — the text a person writes in
//! `spec/*.sas` — and `सङ्केतन` (`crate::assemble_object`) is the encoder. It
//! writes no bytes. So a mnemonic here that is not in
//! `spec/mnemonics-riscv64.src.tsv` does not assemble, and the unit tests call
//! the assembler on every emitted routine rather than comparing golden strings:
//! "it assembles" is a fact the assembler states about this file's output, with
//! the line named on refusal.
//!
//! What is lowered (§2.5): `ConstInt` in three forms (`addi`, `lui`+`addi`, the
//! constant pool), `Add`, `Sub`, `Param` (in `अर्थ०–७`, past eight on the
//! stack), `Call` as `लङ्घनम् पुनःस्थानम्म्` with a frame on EVERY routine
//! (§2.4), `Return`, `Branch`, `CondBranch` as a `bne`-against-zero. Spilled
//! values travel through `आहारः`/`निधानम्` at `स्तूपसूचकः + 8k`. The program
//! begins with a startup stub the emitter owns (§2.6) whose `sp` comes from
//! `auipc`, not `lui`, and whose finisher store turns the entry's result into
//! the exit status `yantra` reads.
//!
//! The convention (§2.3), stated once: doc 02 §2.4's role table — arguments and
//! result in `अर्थ०–अर्थ७`, every allocated IR value in a callee-saved
//! `स्थिर०–स्थिर११`, the emitter's own scratch in `क्षणिक०–क्षणिक६`,
//! `पुनःस्थानम्`/`स्तूपसूचकः` for the frame — with `अर्थ७` the ecall number.
//!
//! `W-245` — THE FORMS THAT MAKE A STATUS MORE THAN ZERO. The IR gained `Cmp`,
//! `Load`/`Store` (a routine's locals as FRAME SLOTS, one region above the spill
//! slots — the same `स्तूपसूचकः + 8k` traffic `Location::Spill` already writes),
//! and the eight operators ADR-0032 froze. [`lower_cond_branch`] is no longer a
//! seam: a `CondBranch` whose condition is a `Cmp` that ends its block and is
//! read by nothing else FUSES to the condition's own branch instruction —
//! `<op>लङ्घनम् R(a)न R(b)त् <then>य् ।`, करण the subject, अपादान the standard
//! (ADR-0008) — and the `Cmp` emits no line of its own. A `Cmp` read as a value
//! is `न्यूनम्`/`अचिह्नन्यूनम्` (`slt`/`sltu`), or a `वियोगः` followed by
//! `अचिह्नन्यूनम् … १न` (`seqz`) / `… शून्यःन …` (`snez`), or `न्यूनम्` followed by
//! `वैषम्यम् … १न` (`xori`) for the not-less pair.

use crate::t1::abi::{AbiLocation, AbiState, FloatRole};
use crate::t1::ast::SymbolId;
use crate::t1::ir::*;
use crate::t1::regalloc::{AllocationMap, Location, RegClass, allocate_registers_for};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

// --- the words the emitter writes -------------------------------------------------------

/// The registers the allocator hands out: `स्थिर०–स्थिर११` (§2.3). Twelve, so that
/// every IR value is callee-saved and lives across a `Call` without the emitter
/// reasoning about clobbers.
pub const ALLOCATABLE: u8 = 12;

/// Names the emitter owns in every module (§2.2). The collision check covers them.
const OWNED_LABELS: &[&str] = &[
    "यन्त्रारम्भ",
    "यन्त्रसफल",
    "यन्त्रसमाप्ति",
    "यन्त्रचक्र",
    "स्तूपः",
    "स्तूपान्तः",
    "ध्रुवकोशः",
];

/// The stack the image carries (§2.6): 64 KiB reserved in `ॱरिक्त` (`.bss`,
/// `V-009` part (i-b2)) — in memory, not in the file.
const STACK_BYTES: u64 = 65_536;

/// Where the SiFive test finisher lives (`yantra::FINISHER`), as the `lui` half.
const FINISHER_HI: &str = "०षोड्१००";

const ZERO: &str = "शून्यः";
const RA: &str = "पुनःस्थानम्";
const SP: &str = "स्तूपसूचकः";

/// The allocator's number to the register it names.
#[must_use]
pub fn register_name(n: u8) -> String {
    assert!(n < ALLOCATABLE, "allocator number {n} is not a स्थिर");
    format!("स्थिर{}", devanagari(i64::from(n)))
}

/// `V-004` PART 3 — THE ALLOCATOR'S NUMBER IS A ROLE INDEX, AND WHICH ROLE IT
/// INDEXES IS A PROPERTY OF THE FILE, NOT OF THE NUMBER.
///
/// [`register_name`] above is the integer file's answer and it is the only one
/// the emitter had: `Location::Register(2)` became `स्थिर२` whatever map it came
/// out of, so a float allocation map had NO spelling here at all. This is the
/// seam. The float answer is [`FloatRole::register_name`], which spells
/// `प्लव<hardware>` — the lexicon's only float spelling
/// (`spec/registers-riscv64.tsv`) — and NOT `FloatRole::role_name`, which would
/// not assemble.
///
/// **THE FLOAT ROLE IS `Saved` AND THE BASIS IS §2.3, NOT A PREFERENCE.** The
/// integer scan fills `स्थिर०–स्थिर११` so that "every allocated IR value is
/// callee-saved and lives across a `Call` without the emitter reasoning about
/// clobbers" ([`ALLOCATABLE`]); `fs0-fs11` is the same property in the other
/// file, and it is also twelve wide, so the two scans are the same scan.
/// `Temp` is caller-saved and `Arg` is the ABI's, so neither can hold a value
/// across a call.
///
/// REFUSES past the end rather than naming a register: `None`, because
/// saturating would hand out `f27` for a thirteenth float value and wrapping
/// would hand out `f8` — and both of those assemble.
#[must_use]
pub fn class_register_name(class: RegClass, n: u8) -> Option<String> {
    match class {
        RegClass::Int => (n < ALLOCATABLE).then(|| register_name(n)),
        RegClass::Float => ALLOCATABLE_FLOAT_ROLE.register_name(n),
    }
}

/// The role whose twelve registers [`class_register_name`] hands out for
/// `RegClass::Float`. Named so the choice has one site and the test can state it.
pub const ALLOCATABLE_FLOAT_ROLE: FloatRole = FloatRole::Saved;

/// How many registers a file's scan may hand out — what `num_registers` must be
/// in [`crate::t1::regalloc::allocate_registers_for`] for the map it returns to
/// be spellable by [`class_register_name`].
///
/// They are both twelve today. The function exists anyway because the two counts
/// have DIFFERENT CAUSES — `ALLOCATABLE` is doc 02 §2.4's `स्थिर` row and the
/// float count is `FLOAT_SAVED`'s length — and a caller that hard-codes twelve
/// would be right by coincidence.
#[must_use]
pub fn allocatable(class: RegClass) -> u8 {
    match class {
        RegClass::Int => ALLOCATABLE,
        RegClass::Float => ALLOCATABLE_FLOAT_ROLE.count(),
    }
}

/// The record region's label, its cursor's label, and the region's size.
///
/// `W-279`. THE SIZE IS A CHOICE WITH A STATED BASIS, NOT A DERIVATION: no
/// static count predicts what an executed path allocates — measured today, where
/// `encode.t1` declares more struct-typed locals than either faulting source and
/// faults in neither. So this is generous against `yantra::DEFAULT_RAM` (20 MB)
/// and the OVERFLOW REFUSES rather than wraps. A bump past the end that wrapped
/// would corrupt records silently, which is the failure shape this project keeps
/// meeting; a refusal names itself.
///
/// 512 MiB since 2026-09-14 — MEASURED, not modelled: the first whole-corpus
/// native compile to a halt (fixpoint4, 1.79 MB of sources) reached a high water
/// of 353,242,600 octets, 18 MB over the 320 MiB this was; the interpreter's bump
/// model gives 274 MB of runs and the rest is records (~79 MB), which the model
/// now prices too. 512 MiB is 1.5× that measurement.
/// (320 MiB since 2026-09-13: a full native self-compile of the twenty sources was
/// MEASURED at 254 MB of record allocation under the interpreter's bump model
/// (`T1_BUMP=1`, first block 128 elements, growth need + 2×old), and the region
/// is counted in the layout, not materialised in the image (`स्थानम्` in
/// `ॱरिक्त`), so the cost is address space: the loader sizes RAM from the memory
/// extent plus headroom. The `.t1` emitter's `यन्त्ररचनाष्टकाः` and its emitted
/// literal carry the SAME number — the twin check is the only thing that keeps
/// the two in step.
pub const RECORD_REGION: &str = "रचनाक्षेत्रम्";
pub const RECORD_CURSOR: &str = "रचनासूचकः";
const RECORD_REGION_OCTETS: i64 = 512 * (1 << 20);

fn temp(n: u8) -> String {
    format!("क्षणिक{}", devanagari(i64::from(n)))
}

fn arg(n: usize) -> String {
    format!("अर्थ{}", devanagari(n as i64))
}

/// An integer as a T0 numeral: Devanagari digits, `ऋण` for a negative
/// (`spec/grammar-t0.ebnf` `numeral`, ADR-0009). Never a Latin minus.
#[must_use]
pub fn devanagari(n: i64) -> String {
    let mut s = String::new();
    if n < 0 {
        s.push_str("ऋण");
    }
    for c in n.unsigned_abs().to_string().chars() {
        s.push(match c {
            '0' => '०',
            '1' => '१',
            '2' => '२',
            '3' => '३',
            '4' => '४',
            '5' => '५',
            '6' => '६',
            '7' => '७',
            '8' => '८',
            '9' => '९',
            other => other,
        });
    }
    s
}

/// A 64-bit pattern as a T0 hexadecimal numeral — `०षोड्` and the digits
/// `०–९ अ आ इ ई उ ऊ` (`grammar-t0.ebnf` `hexadecimal`). The constant pool is
/// written this way because a datum is a BIT PATTERN (`parse.rs`, `W-075`).
#[must_use]
pub fn hex64(bits: u64) -> String {
    const DIGITS: [&str; 16] = [
        "०", "१", "२", "३", "४", "५", "६", "७", "८", "९", "अ", "आ", "इ", "ई", "उ", "ऊ",
    ];
    let mut s = String::from("०षोड्");
    let mut started = false;
    for shift in (0..16).rev() {
        let d = ((bits >> (shift * 4)) & 0xf) as usize;
        if d != 0 || started || shift == 0 {
            started = true;
            s.push_str(DIGITS[d]);
        }
    }
    s
}

// --- the module the emitter reads --------------------------------------------------------

/// The symbol → `(module, routine)` map the resolver holds (§3.1). On the corpus
/// it is read from `artha.t1`'s arenas; in a test it is written by hand.
pub type Names = HashMap<SymbolId, (String, String)>;

/// One module's worth of IR, named, with the routine the startup stub calls.
pub struct Module {
    /// `W-278` — THE GLOBALS THIS MODULE DECLARES: `(symbol, initial value)`.
    ///
    /// The DECLARING module emits the storage, with the label exported, and
    /// every other module reaches it by name through the linker. One object per
    /// global across the whole image — the alternative, a copy per module, would
    /// make a cross-module write land in one copy and the read take another, so
    /// every shared global would answer its initialiser forever.
    /// `(label, initial value)`. The LABEL and not the symbol: the T1 twin's
    /// data block writes the same text, and holding the label on both sides is
    /// what makes the two emitters agree by construction rather than by both
    /// deriving it correctly. A `LoadGlobal` still names a SYMBOL and resolves
    /// it through `names`, so the two must concatenate to the same string —
    /// `routine_label`'s `{module}{name}`, no joiner.
    /// `W-293` adds the third element: the OCTETS this global reserves, carried
    /// from `मध्यरूप`'s `वैश्विकसामर्थ्यकोश` and not derived here — `८` for a
    /// scalar, `खण्डसामर्थ्यम् गुणनम् ८` for a run.
    pub globals: Vec<(String, i64, i64)>,
    /// The module's name — the first half of every routine label (§2.2).
    pub name: String,
    /// Its routines.
    pub functions: Vec<Function>,
    /// Every symbol a routine or a `Call` names.
    pub names: Names,
    /// The zero-parameter routine whose result becomes the exit status (§2.6).
    /// `None` emits a stub that calls nothing and halts `0x5555`.
    pub entry: Option<SymbolId>,
}

/// What the emitter refuses, by name (§2.9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A block ends in `Unreachable` — "declared and built by nothing" (`ir.t1:99`).
    Unreachable { function: String, block: BlockId },
    /// A block has no terminator at all.
    NoTerminator { function: String, block: BlockId },
    /// A `Branch`/`CondBranch` names a block the function does not hold (Rule X3).
    TargetNotInFunction {
        function: String,
        from: BlockId,
        target: BlockId,
    },
    /// A symbol no entry of [`Names`] names.
    UnnamedSymbol { symbol: SymbolId },
    /// Two `(module, name)` pairs — or a pair and an emitter-owned name —
    /// concatenate to one word (§2.2).
    LabelCollision {
        label: String,
        first: String,
        second: String,
    },
    /// A `Param` read after a `Call` in its block would read a clobbered `अर्थ` (§2.5).
    ParamAfterCall {
        function: String,
        block: BlockId,
        param: usize,
    },
    /// A `Param` outside the entry block — the arguments are only in `अर्थ` on entry.
    ParamOutsideEntry {
        function: String,
        block: BlockId,
        param: usize,
    },
    /// A frame one `addi` cannot address (§2.4).
    FrameTooLarge { function: String, bytes: i64 },
    /// The entry routine takes parameters; the stub passes none (§2.6).
    EntryTakesParameters { function: String, params: usize },
    /// A conditional's target is further than ±4 KiB of text (§2.5).
    BranchOutOfRange {
        function: String,
        from: BlockId,
        target: BlockId,
        bytes: i64,
    },
    /// An unconditional `लङ्घनम्` whose target label is further than
    /// ±1 MiB of text — the J-type's own reach (§2.5). `W-306`'s remaining
    /// hole: the relaxation REPLACES a far conditional with one of these, so
    /// leaving the J-type unmeasured moved the wrap one instruction along
    /// instead of refusing it. Named by LABEL and not by block, because the
    /// exit label is a target no `BlockId` can spell.
    JumpOutOfRange {
        function: String,
        target: String,
        bytes: i64,
    },
    /// `W-306c` — A `StoreAt` whose element width is not १, २, ४ or ८ octets.
    /// THE STORE SIDE REFUSES WHERE THE LOAD SIDE GUESSES, and that asymmetry
    /// is ruled rather than inherited: `LoadIndex`'s `_ =>` arm answers the
    /// bare `आहारः` for every unnamed width, which reads a WORD where the IR
    /// asked for something else — wrong, but only in the register. The same
    /// fall-through on a store writes eight octets where the IR asked for
    /// three, so it CORRUPTS the five beyond the field. `सङ्कीर्णनिधानरचना`'s
    /// own margin already ruled it: an unnamed width emits nothing.
    StoreWidthUnnamed { function: String, bytes: u64 },
    /// `V-005` — A VALUE READ FROM THE WRONG REGISTER FILE: a float where an
    /// integer goes (a call's argument, a return, an address, an integer
    /// operator, a store through memory, a branch condition) or an integer
    /// where a float op reads a float. Refused because the assembler does NOT
    /// check an operand's file: `योगः अर्थ०म् प्लव८न ०न` assembles, as
    /// `addi a0, x8, 0`, and passes `स्थिर०`'s word where the float was meant —
    /// a silent wrong value on the native side of a program the interpreter
    /// runs. Arguments and results now travel in `fa` registers (`abi_slots`),
    /// so what is left to refuse is an integer operator, an address and a
    /// branch condition — and, since `V-008` stores a float to memory
    /// (`StoreAt` with `प्लवनिधानम्`), a float stored at a width that is not a
    /// whole word. `value` is the value's `ValueId`.
    ///
    /// **THE ASSEMBLER'S OWN REGISTER-FILE CHECK IS OUT OF THIS ROW'S SCOPE**
    /// (coordinator ruling, 2026-10-04): an operand-file check in the two
    /// assemblers is wider than V-005, so this refusal, on BOTH emitters, is
    /// the guard until that check exists — an emitter that writes a float
    /// register where an integer one goes is refused here, not assembled.
    FileMismatch {
        function: String,
        block: BlockId,
        value: usize,
    },
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::Unreachable { function, block } => {
                write!(f, "{function}: block {block:?} ends in Unreachable")
            }
            Refusal::NoTerminator { function, block } => {
                write!(f, "{function}: block {block:?} has no terminator")
            }
            Refusal::TargetNotInFunction {
                function,
                from,
                target,
            } => write!(
                f,
                "{function}: block {from:?} targets {target:?}, which is not a block of this function"
            ),
            Refusal::UnnamedSymbol { symbol } => write!(f, "symbol {symbol:?} has no name"),
            Refusal::LabelCollision {
                label,
                first,
                second,
            } => write!(f, "label {label} is both {first} and {second}"),
            Refusal::ParamAfterCall {
                function,
                block,
                param,
            } => write!(
                f,
                "{function}: block {block:?} reads Param({param}) after a Call clobbered अर्थ"
            ),
            Refusal::ParamOutsideEntry {
                function,
                block,
                param,
            } => write!(
                f,
                "{function}: block {block:?} reads Param({param}) outside the entry block"
            ),
            Refusal::FrameTooLarge { function, bytes } => {
                write!(
                    f,
                    "{function}: a frame of {bytes} bytes exceeds one addi (2047)"
                )
            }
            Refusal::EntryTakesParameters { function, params } => {
                write!(
                    f,
                    "{function}: the entry takes {params} parameters; the stub passes none"
                )
            }
            Refusal::BranchOutOfRange {
                function,
                from,
                target,
                bytes,
            } => write!(
                f,
                "{function}: the conditional in {from:?} is {bytes} bytes from {target:?}, past ±4 KiB"
            ),
            Refusal::JumpOutOfRange {
                function,
                target,
                bytes,
            } => write!(
                f,
                "{function}: the लङ्घनम् to {target} is {bytes} bytes away, past ±1 MiB"
            ),
            Refusal::StoreWidthUnnamed { function, bytes } => write!(
                f,
                "{function}: a StoreAt of {bytes} octets has no निधानम् form (१, २, ४ and ८ do)"
            ),
            Refusal::FileMismatch {
                function,
                block,
                value,
            } => write!(
                f,
                "{function}: block {block:?} reads value {value} from the wrong register file \
                 (a float is stored to memory only as a whole word, and never read by an \
                 integer operator, an address or a branch condition)"
            ),
        }
    }
}

impl std::error::Error for Refusal {}

// --- names ---------------------------------------------------------------------------

/// Routine label = module name ⧺ routine name, no joiner (§2.2).
///
/// # Errors
/// The symbol is not in `names`.
pub fn routine_label(names: &Names, symbol: SymbolId) -> Result<String, Refusal> {
    names
        .get(&symbol)
        .map(|(module, name)| format!("{module}{name}"))
        .ok_or(Refusal::UnnamedSymbol { symbol })
}

/// Label for a global's STORAGE, including the two the emitter owns.
///
/// `ir.t1` mints the record cursor and the record region in a band of its own
/// (`रचनासूचकसंज्ञा` = 10_000_003, `रचनाक्षेत्रसंज्ञा` = 10_000_004) and states at
/// ir.t1:543 that **"the emitters own the label"** — module-less, so the pair
/// `("", "रचनासूचकः")` spells it. Nothing ever puts them in `names`, because
/// `names` is built from declarations and these have none: the startup object
/// defines the storage and every module merely refers to it.
///
/// So resolving them through [`routine_label`] could only ever refuse, and did:
/// every one of `storage-probe.py --compare`'s seven fixtures came back
/// `BUILD REFUSED — STOPPED AT emit: symbol SymbolId(10000003) has no name`,
/// which is why the storage gate had been red since 2026-09-14 while looking
/// like three separate array-indexing defects. It was one missing label.
///
/// SEPARATE FROM [`routine_label`] ON PURPOSE. A CALL whose target is in this
/// band is a defect, and folding the builtins into the shared resolver would
/// hand it a label and link it to storage instead of refusing. Only the two
/// arms that address a global — `LoadGlobal` and `AddrOfGlobal` — call this.
///
/// # Errors
/// The symbol is neither a builtin nor in `names`.
pub fn global_label(names: &Names, symbol: SymbolId) -> Result<String, Refusal> {
    if symbol == RECORD_CURSOR_SYMBOL {
        return Ok(RECORD_CURSOR.to_string());
    }
    if symbol == RECORD_REGION_SYMBOL {
        return Ok(RECORD_REGION.to_string());
    }
    routine_label(names, symbol)
}

/// Block label = routine label ⧺ `पर्व` ⧺ the block id (§2.2).
#[must_use]
pub fn block_label(routine: &str, block: BlockId) -> String {
    format!("{routine}पर्व{}", devanagari(block.0 as i64))
}

/// Exit label = routine label ⧺ `निर्गम` — one epilogue per routine (§2.2).
#[must_use]
pub fn exit_label(routine: &str) -> String {
    format!("{routine}निर्गम")
}

/// Every label a module defines, refusing a collision by pair (§2.2).
fn check_labels(module: &Module) -> Result<(), Refusal> {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for owned in OWNED_LABELS {
        seen.insert((*owned).to_string(), "the emitter".to_string());
    }
    for func in &module.functions {
        let (m, n) = module
            .names
            .get(&func.name)
            .ok_or(Refusal::UnnamedSymbol { symbol: func.name })?;
        let label = format!("{m}{n}");
        // `V-009` (ii): a module's matrix kernel routine is named by a matrix
        // member, so a user routine of that name collides with it — and the
        // refusal says the name is reserved (the interpreter's words exactly).
        let who = if func.name == MATRIX_PRODUCT_SYMBOL || func.name == MATRIX_TRANSPOSE_SYMBOL {
            format!(
                "the matrix kernel routine: {}",
                crate::t1::nirvahana::kernel_name_refusal(m, n)
            )
        } else {
            format!("{m} ॱ {n}")
        };
        if let Some(first) = seen.insert(label.clone(), who.clone()) {
            return Err(Refusal::LabelCollision {
                label,
                first,
                second: who,
            });
        }
    }
    Ok(())
}

// --- frames --------------------------------------------------------------------------

/// The frame of one routine (§2.4): spill slots, then — `W-245` — the LOCAL
/// slots (`Load`/`Store`), then the saved callee-saved registers the routine
/// uses, then `पुनःस्थानम्` at the top; rounded up to 16.
///
/// `V-004` PART 4: the frame serves **TWO REGISTER FILES**, because
/// `regalloc::allocate_registers_for` is run once per class and "the spill
/// numbering in each returned map starts at zero and counts only its own"
/// (`regalloc.rs:54`). Two maps therefore both call their first spill `0`, and
/// a frame that kept one `num_spills` would put an `f` value and an `x` value
/// in the same eight bytes. So each file gets its own spill region and its own
/// saved registers, and [`Frame::spill_offset`] is the ONE site that knows
/// which region a class's slot `k` lands in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Total bytes; what the prologue subtracts from `स्तूपसूचकः`.
    pub bytes: i64,
    /// The callee-saved registers saved, grouped by file — every `Int` first in
    /// allocator-number order, then every `Float` — each with its class, its
    /// allocator number and its offset. The class is carried because the number
    /// alone does not say which file it indexes (PART 3) and the prologue needs
    /// both the right spelling AND the right store mnemonic.
    pub saved: Vec<(RegClass, u8, i64)>,
    /// `पुनःस्थानम्`'s offset: `bytes - 8`.
    pub ra_offset: i64,
    /// The INTEGER file's spill count. Integer spill slot `k` lives at `8k`.
    pub num_spills: usize,
    /// The FLOAT file's spill count. Float spill slot `k` lives at
    /// `8(num_spills + k)` — a second region, not a continuation of the first,
    /// because the float scan numbers its own slots from zero.
    ///
    /// `t1_sources`'s pairing guard hand-lists struct fields in `RISCV_FIELDS`,
    /// and this one is listed since `V-005` gave the T1 IR a float kind: the
    /// port's `यन्त्रचौकट` carries it as `प्लवनिक्षेपसंख्यान`.
    pub num_float_spills: usize,
    /// Local slot `k` lives at `8(num_spills + num_float_spills + k)` — `W-245`.
    pub num_locals: usize,
}

impl Frame {
    /// Where spill slot `k` of `class` lives. The integer region is first, so
    /// `spill_offset(Int, k)` is `8k` — byte-for-byte what every caller computed
    /// before this file served two of them.
    #[must_use]
    pub fn spill_offset(&self, class: RegClass, k: usize) -> i64 {
        match class {
            RegClass::Int => 8 * k as i64,
            RegClass::Float => 8 * (self.num_spills + k) as i64,
        }
    }

    /// Where local slot `k` lives: above BOTH spill regions (`W-245`).
    #[must_use]
    pub fn local_offset(&self, k: usize) -> i64 {
        8 * (self.num_spills + self.num_float_spills + k) as i64
    }
}

/// How many local slots a routine addresses: one past the highest `Load`/`Store`
/// slot, or none (`W-245`).
#[must_use]
pub fn count_locals(func: &Function) -> usize {
    func.blocks
        .values()
        .flat_map(|b| b.insts.iter())
        .filter_map(|(_, i)| match i {
            Instruction::Load(k) | Instruction::LoadFloat(k) | Instruction::Store(k, _) => {
                Some(*k + 1)
            }
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

/// The allocator numbers one map holds in registers, ascending and without
/// repeats — the registers that file's prologue must save.
fn registers_held(alloc: &AllocationMap) -> Vec<u8> {
    let mut used: Vec<u8> = alloc
        .locations
        .values()
        .filter_map(|l| match l {
            Location::Register(r) => Some(*r),
            Location::Spill(_) => None,
        })
        .collect::<HashSet<u8>>()
        .into_iter()
        .collect();
    used.sort_unstable();
    used
}

/// Lay out the frame from the allocation(s) (§2.4) and the routine's local count.
///
/// `float` is the SECOND scan's map, `None` (or empty) when the routine has no
/// float values — every routine of the compiler corpus, which writes no float
/// built-in (`V-005`). With `None` the result is byte-for-byte the one-file
/// frame: the float spill region is empty, no float register is saved, and
/// nothing above the integer region moves.
#[must_use]
pub fn frame_layout(
    alloc: &AllocationMap,
    float: Option<&AllocationMap>,
    num_locals: usize,
) -> Frame {
    let num_float_spills = float.map_or(0, |f| f.num_spills);
    let slot_bytes = 8 * (alloc.num_spills + num_float_spills + num_locals) as i64;
    // Integer saved registers first, then float: one contiguous run per file, so
    // a reader of the prologue sees the two files in the order the frame names.
    let held: Vec<(RegClass, u8)> = registers_held(alloc)
        .into_iter()
        .map(|r| (RegClass::Int, r))
        .chain(
            float
                .map(registers_held)
                .unwrap_or_default()
                .into_iter()
                .map(|r| (RegClass::Float, r)),
        )
        .collect();
    let saved: Vec<(RegClass, u8, i64)> = held
        .into_iter()
        .enumerate()
        .map(|(i, (c, r))| (c, r, slot_bytes + 8 * i as i64))
        .collect();
    let raw = slot_bytes + 8 * saved.len() as i64 + 8;
    let bytes = (raw + 15) / 16 * 16;
    Frame {
        bytes,
        saved,
        ra_offset: bytes - 8,
        num_spills: alloc.num_spills,
        num_float_spills,
        num_locals,
    }
}

/// The store mnemonic for a file (`spec/mnemonics-riscv64.src.tsv:81-82`):
/// `निधानम्` is `sd`, `प्लवनिधानम्` is `fsd`. A float saved in a frame slot with
/// `निधानम्` would assemble — `sd` takes an `x` register — and store the WRONG
/// REGISTER, because `स्थिर३` and `प्लवस्थिर३` are different hardware.
#[must_use]
fn store_mnemonic(class: RegClass) -> &'static str {
    match class {
        RegClass::Int => "निधानम्",
        RegClass::Float => "प्लवनिधानम्",
    }
}

/// The load mnemonic for a file: `आहारः` is `ld`, `प्लवाहारः` is `fld`.
#[must_use]
fn load_mnemonic(class: RegClass) -> &'static str {
    match class {
        RegClass::Int => "आहारः",
        RegClass::Float => "प्लवाहारः",
    }
}

// --- the emitter ---------------------------------------------------------------------

/// One routine's emission state.
struct Routine<'a> {
    label: String,
    func: &'a Function,
    /// The INTEGER file's map: its numbers index `स्थिर`.
    alloc: &'a AllocationMap,
    /// `V-004` PART 5: the FLOAT file's map — the second scan,
    /// `allocate_registers_for(.., RegClass::Float, classify)` — whose numbers
    /// index `fs0-fs11` ([`ALLOCATABLE_FLOAT_ROLE`]). Empty in every routine
    /// with no float value — `V-005`'s `value_class` answers `Float` for a
    /// float op's float result and a `प६४` local's read. Part 3 carried ONE map and a
    /// `class` field naming its file; with two maps the file is a property of
    /// WHICH MAP holds the value, so [`Routine::location`] answers both.
    float: &'a AllocationMap,
    frame: Frame,
    out: String,
}

impl Routine<'_> {
    fn line(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }

    /// `V-004` PARTS 3-5: the register an allocator number names in `class`'s
    /// file. The panic is the old `register_name` assertion moved behind the
    /// class: a number the scan could not have produced is a bug in the scan,
    /// and `स्थिर१२`/`प्लव`-nothing is not a diagnosis. The prologue saves
    /// registers from BOTH files and the frame's own entry says which file
    /// each number indexes; a value's file is the map [`Routine::location`]
    /// found it in.
    fn class_reg_name(&self, class: RegClass, n: u8) -> String {
        class_register_name(class, n).unwrap_or_else(|| {
            panic!(
                "{}: allocator number {n} is not in the {class:?} file",
                self.label
            )
        })
    }

    /// Where a value lives AND WHICH FILE it lives in. The two scans are
    /// disjoint by construction (`regalloc.rs` leaves an out-of-class value out
    /// of the interval set), so a value is in exactly one map; the integer map
    /// is asked first only because it is the one every routine fills today.
    fn location(&self, v: ValueId) -> (RegClass, Location) {
        if let Some(l) = self.alloc.locations.get(&v) {
            return (RegClass::Int, *l);
        }
        if let Some(l) = self.float.locations.get(&v) {
            return (RegClass::Float, *l);
        }
        panic!("{}: {v:?} has no location in either file", self.label)
    }

    /// `V-004` PART 5 — THE SCRATCH REGISTER A SPILLED VALUE PASSES THROUGH, PER
    /// FILE. Integer scratch `n` is `क्षणिक<n>` (`t<n>`), as it always was.
    /// Float scratch `n` is `ft<n>` — [`FloatRole::Temp`]'s register `n`,
    /// spelled `प्लव<hardware>` by [`FloatRole::register_name`], so scratch 0,
    /// 1, 2 and 3 are `प्लव०`..`प्लव३` (`f0`-`f3`).
    ///
    /// `Temp` and not `Saved` or `Arg`, for the same reason the integer side
    /// uses `क्षणिक`: a scratch lives for ONE instruction and must not be a
    /// register the allocator hands out (`Saved` is [`ALLOCATABLE_FLOAT_ROLE`])
    /// or one a call's arguments travel in (`Arg`). The scratch numbers the
    /// lowerings pass (0-3) are the SAME numbers in both files, so the
    /// discipline that keeps an instruction's two operands and its result in
    /// different integer scratches keeps them apart in the float file too.
    ///
    /// A method and not a module-level `fn`/`const`, as `read` and `write`
    /// are; its `.t1` twin is the float band of the port's register code
    /// (`यन्त्रप्लवक्षणिकाधारः`, spelled by `यन्त्रप्लवक्षणिकसङ्ख्या`).
    fn scratch(&self, class: RegClass, n: u8) -> String {
        match class {
            RegClass::Int => temp(n),
            RegClass::Float => FloatRole::Temp
                .register_name(n)
                .unwrap_or_else(|| panic!("{}: there is no float scratch {n}", self.label)),
        }
    }

    /// The register a value can be READ from, loading a spilled one into the
    /// scratch register `scratch` of ITS file first. `extra` is what the stack
    /// pointer has been moved by since the prologue (the stack-argument push
    /// of a `Call`).
    ///
    /// `V-004` PART 5: the load is the FILE'S load — `आहारः` (`ld`) for an
    /// integer slot, `प्लवाहारः` (`fld`) for a float one — and the slot is the
    /// file's own region ([`Frame::spill_offset`]). `आहारः` into a float value
    /// would assemble and load the word into an `x` register the float
    /// instruction never reads.
    fn read(&mut self, v: ValueId, scratch: u8, extra: i64) -> String {
        let (class, loc) = self.location(v);
        match loc {
            Location::Register(r) => self.class_reg_name(class, r),
            Location::Spill(k) => {
                let t = self.scratch(class, scratch);
                let off = devanagari(self.frame.spill_offset(class, k) + extra);
                let load = load_mnemonic(class);
                self.line(&format!("{load} {t}म् {SP}त् {off}न ।"));
                t
            }
        }
    }

    /// The register a value is WRITTEN into, and the store that follows for a
    /// spilled one. The caller emits the defining instruction between the two.
    /// The store is the file's store (`निधानम्`/`प्लवनिधानम्`), as in `read`.
    fn write(&self, v: ValueId, scratch: u8) -> (String, Option<String>) {
        let (class, loc) = self.location(v);
        match loc {
            Location::Register(r) => (self.class_reg_name(class, r), None),
            Location::Spill(k) => {
                let t = self.scratch(class, scratch);
                let off = devanagari(self.frame.spill_offset(class, k));
                let store = store_mnemonic(class);
                (t.clone(), Some(format!("{store} {SP}य् {off}न {t}न ।")))
            }
        }
    }

    fn prologue(&mut self) {
        let f = self.frame.bytes;
        // `W-243`: EVERY routine label is exported (`॥ वैश्विकम् ॥`, `directives.tsv`).
        // A label is local to its object (B-014), and each module is its own
        // object, so a call into another module resolves only through the
        // linker's export table (`samyojana.rs`: the object's own names first,
        // then what any object exported). The IR carries no publicness
        // (`ir.rs` `Function { name, blocks, entry_block }`) and §2.2's
        // collision check makes module ⧺ routine unique across the corpus, so
        // exporting all of them cannot define a name twice. research/25 §5 R6.
        self.line(&format!("॥ वैश्विकम् {} ॥", self.label));
        self.line(&format!("{}ॱॱ", self.label));
        self.line(&format!("योगः {SP}म् {SP}न {}न ।", devanagari(-f)));
        self.line(&format!(
            "निधानम् {SP}य् {}न {RA}न ।",
            devanagari(self.frame.ra_offset)
        ));
        for (c, r, off) in self.frame.saved.clone() {
            let name = self.class_reg_name(c, r);
            let store = store_mnemonic(c);
            self.line(&format!("{store} {SP}य् {}न {name}न ।", devanagari(off)));
        }
    }

    fn epilogue(&mut self) {
        self.line(&format!("{}ॱॱ", exit_label(&self.label)));
        self.line(&format!(
            "आहारः {RA}म् {SP}त् {}न ।",
            devanagari(self.frame.ra_offset)
        ));
        for (c, r, off) in self.frame.saved.clone() {
            let name = self.class_reg_name(c, r);
            let load = load_mnemonic(c);
            self.line(&format!("{load} {name}म् {SP}त् {}न ।", devanagari(off)));
        }
        self.line(&format!(
            "योगः {SP}म् {SP}न {}न ।",
            devanagari(self.frame.bytes)
        ));
        self.line(&format!("सापेक्षलङ्घनम् {ZERO}म् {RA}त् ०न ।"));
    }
}

/// `ConstInt` in its three forms (§2.5). `pool` collects the constants that fit
/// neither an `addi` nor a `lui`+`addi`; the pool index is the label `ध्रुव<k>`.
fn lower_constant(rt: &mut Routine<'_>, v: ValueId, c: i64, pool: &mut Vec<i64>) {
    let (rd, store) = rt.write(v, 2);
    if (-2048..=2047).contains(&c) {
        rt.line(&format!("योगः {rd}म् {ZERO}न {}न ।", devanagari(c)));
    } else if let Some((hi, lo)) = split_hi_lo(c) {
        // `lui` sign-extends bit 31 on RV64, so this form is only for a value
        // that IS a sign-extended 32-bit number; `hi` carries the +1 when `lo`
        // is negative (`namaste.sas:22–24`'s pair).
        rt.line(&format!("उपरिभारः {rd}म् {}न ।", devanagari(hi)));
        if lo != 0 {
            rt.line(&format!("योगः {rd}म् {rd}न {}न ।", devanagari(lo)));
        }
    } else {
        let k = pool.iter().position(|p| *p == c).unwrap_or_else(|| {
            pool.push(c);
            pool.len() - 1
        });
        let k = devanagari(k as i64);
        let t = temp(0);
        rt.line(&format!("स्थानसापेक्षयोगः {t}म् ध्रुव{k}ॱउपरिन ।"));
        rt.line(&format!("आहारः {rd}म् {t}त् ध्रुव{k}ॱअधःन ।"));
    }
    if let Some(s) = store {
        rt.line(&s);
    }
}

/// `W-254` — a string literal's octets, and the value is their ADDRESS.
///
/// [`lower_constant`]'s pooled arm WITH THE LOAD DROPPED, and that one
/// substitution is the whole difference. A pooled integer wants the eight
/// octets at the label, so its second instruction is `आहारः` (a load); a string
/// wants the label itself, so its second instruction is `योगः` (an addi) and
/// what lands in `rd` is where the octets are.
///
/// Identical octets share a pool entry, as identical integers do. Two `उक्तम्`s
/// spelling the same text are the same address, which is observable — but they
/// are also immutable, there is no write path to a literal's storage, and the
/// alternative is paying for every duplicate in an image whose RAM is the
/// binding constraint (`W-262`).
///
/// THE LENGTH IS NOT IN THE VALUE, and that is ADR-0026 rather than an omission
/// here: a slice is ONE address-sized word, so a consumer takes the length from
/// the type. Nothing in this unit can hand a length to a routine that wants one
/// at runtime, and the first caller that needs it is where that question gets
/// answered.
fn lower_string(rt: &mut Routine<'_>, v: ValueId, bytes: &[u8], strings: &mut Vec<Vec<u8>>) {
    let (rd, store) = rt.write(v, 2);
    let k = strings
        .iter()
        .position(|s| s.as_slice() == bytes)
        .unwrap_or_else(|| {
            strings.push(bytes.to_vec());
            strings.len() - 1
        });
    let k = devanagari(k as i64);
    let t = temp(0);
    rt.line(&format!("स्थानसापेक्षयोगः {t}म् पाठ{k}ॱउपरिन ।"));
    rt.line(&format!("योगः {rd}म् {t}न पाठ{k}ॱअधःन ।"));
    if let Some(s) = store {
        rt.line(&s);
    }
}

/// `(hi, lo)` with `c == (hi << 12) + lo`, `lo` in the 12-bit signed range and
/// `hi` in `lui`'s 20-bit field written unsigned — or `None` when `c` is not a
/// sign-extended 32-bit value.
#[must_use]
pub fn split_hi_lo(c: i64) -> Option<(i64, i64)> {
    if i32::try_from(c).is_err() {
        return None;
    }
    let hi = (c + 0x800) >> 12;
    let lo = c - (hi << 12);
    // `0x7FFF_F800..=0x7FFF_FFFF` would need hi = 0x80000, which `lui`'s field
    // cannot hold as a positive 20-bit value; those go to the pool.
    if !(-(1 << 19)..(1 << 19)).contains(&hi) {
        return None;
    }
    Some((hi & 0xFFFFF, lo))
}

/// `Call(sym, args)` (§2.5): र the arguments out — `अर्थ<j>` for the first eight,
/// the stack above `स्तूपसूचकः` for the rest — ट the `लङ्घनम्`, व the result.
fn emit_call(
    rt: &mut Routine<'_>,
    v: ValueId,
    callee: SymbolId,
    args: &[ValueId],
    names: &Names,
    float_result: bool,
) -> Result<(), Refusal> {
    let target = routine_label(names, callee)?;
    // `V-005`: each argument's place by `abi.rs`'s allocation over the
    // arguments' FILES — integers in `अर्थ०-७`, floats in `fa0-fa7`, each file
    // counted on its own, and the rest on ONE shared stack in argument order.
    // An all-integer call is the eight-then-stack layout it always was.
    let classes: Vec<RegClass> = args.iter().map(|a| rt.location(*a).0).collect();
    let slots = abi_slots(&classes);
    let on_stack = slots
        .iter()
        .filter(|l| matches!(l, AbiLocation::Stack(_)))
        .count();
    let adjust = 16 * ((on_stack as i64 + 1) / 2);
    if adjust > 0 {
        rt.line(&format!("योगः {SP}म् {SP}न {}न ।", devanagari(-adjust)));
        for ((a, slot), class) in args.iter().zip(&slots).zip(&classes) {
            if let AbiLocation::Stack(off) = slot {
                let src = rt.read(*a, 0, adjust);
                let off = devanagari(*off as i64);
                let store = store_mnemonic(*class);
                rt.line(&format!("{store} {SP}य् {off}न {src}न ।"));
            }
        }
    }
    for (a, slot) in args.iter().zip(&slots) {
        match slot {
            AbiLocation::IntReg(n) => {
                let src = rt.read(*a, 0, adjust);
                rt.line(&format!("योगः {}म् {src}न ०न ।", arg(usize::from(*n))));
            }
            AbiLocation::FloatReg(f) => {
                let src = rt.read(*a, 0, adjust);
                rt.line(&format!("{FLOAT_MOVE} {}म् {src}न {src}न ।", float_arg(*f)));
            }
            AbiLocation::Stack(_) => {}
        }
    }
    rt.line(&format!("लङ्घनम् {RA}म् {target}य् ।"));
    if adjust > 0 {
        rt.line(&format!("योगः {SP}म् {SP}न {}न ।", devanagari(adjust)));
    }
    let (rd, store) = rt.write(v, 2);
    if float_result {
        rt.line(&format!(
            "{FLOAT_MOVE} {rd}म् {}न {}न ।",
            float_arg(0),
            float_arg(0)
        ));
    } else {
        rt.line(&format!("योगः {rd}म् {}न ०न ।", arg(0)));
    }
    if let Some(s) = store {
        rt.line(&s);
    }
    Ok(())
}

/// `V-005` — THE FLOAT REGISTER COPY, `fsgnj.d rd, rs, rs` (`fmv.d`'s own
/// expansion): the sign of `rs` injected into `rs` is `rs`. The tree's word for
/// `fsgnj` is `प्लवचिह्नारोपणम्` (`spec/encodings-riscv64.tsv`), bare for `.d`.
const FLOAT_MOVE: &str = "प्लवचिह्नारोपणम्";

/// `fa<n>` — float argument register `n`, spelled `प्लव<hardware>`.
fn float_arg(n: u8) -> String {
    FloatRole::Arg
        .register_name(n)
        .unwrap_or_else(|| panic!("there is no float argument register {n}"))
}

/// `V-005` — WHERE EACH ARGUMENT TRAVELS: `abi.rs`'s `AbiState` over the
/// values' files in order — an integer takes the next of `अर्थ०-७`, a float
/// the next of `fa0-fa7`, and whichever runs out goes to the SHARED stack at
/// the next eight-octet slot. The caller and the callee both ask this, the
/// caller over its arguments and the callee over its parameters, so the two
/// sides of one call cannot disagree.
fn abi_slots(classes: &[RegClass]) -> Vec<AbiLocation> {
    let mut state = AbiState::new();
    classes
        .iter()
        .map(|c| {
            let reg = match c {
                RegClass::Int => state.allocate_int().map(AbiLocation::IntReg),
                RegClass::Float => state.allocate_float().map(AbiLocation::FloatReg),
            };
            reg.unwrap_or_else(|| AbiLocation::Stack(state.allocate_stack(8)))
        })
        .collect()
}

/// `V-005` — every parameter's place, by its index: [`abi_slots`] over the
/// routine's `Param`/`ParamFloat` in parameter order.
fn param_locations(func: &Function) -> HashMap<usize, AbiLocation> {
    let mut params: Vec<(usize, RegClass)> = func
        .blocks
        .values()
        .flat_map(|b| b.insts.iter())
        .filter_map(|(_, i)| match i {
            Instruction::Param(k) => Some((*k, RegClass::Int)),
            Instruction::ParamFloat(k) => Some((*k, RegClass::Float)),
            _ => None,
        })
        .collect();
    params.sort_by_key(|(k, _)| *k);
    params.dedup_by_key(|(k, _)| *k);
    let classes: Vec<RegClass> = params.iter().map(|(_, c)| *c).collect();
    params
        .iter()
        .map(|(k, _)| *k)
        .zip(abi_slots(&classes))
        .collect()
}

/// `V-005` — THE FILE A VALUE LIVES IN, from the instruction that defines it:
/// `Float` for a float op whose result is a float ([`FloatOp::defines_float`])
/// and for a `प६४` local's read, `Int` for everything else. The classifier both
/// scans of [`emit_module_and_relaxations`] share; `yantrotsarjana.t1`'s
/// `यन्त्रप्लववर्गः` is its twin.
#[must_use]
pub fn value_class(inst: &Instruction) -> RegClass {
    match inst {
        Instruction::Float(op, _) if op.defines_float() => RegClass::Float,
        Instruction::LoadFloat(_)
        | Instruction::ParamFloat(_)
        | Instruction::CallFloat(..)
        | Instruction::LoadAtFloat(_) => RegClass::Float,
        _ => RegClass::Int,
    }
}

/// `V-005` — the word of each float op (`spec/encodings-riscv64.tsv`), bare for
/// the 64-bit form as `निधानम्` is bare for `sd` — the assembler's default
/// width is sixty-four — except the CONVERSIONS, which the assembler chooses by
/// their written PAIR of types and nothing else (`B-075`). A bit move needs no
/// pair: the destination's register file says which way it goes (`B-074`).
#[must_use]
pub fn float_verb(op: FloatOp) -> &'static str {
    match op {
        FloatOp::Add => "प्लवयोगः",
        FloatOp::Sub => "प्लववियोगः",
        FloatOp::Mul => "प्लवगुणनम्",
        FloatOp::Div => "प्लवभागः",
        FloatOp::Sqrt => "प्लववर्गमूलम्",
        FloatOp::MulAdd => "प्लवगुणयोगः",
        FloatOp::Eq => "प्लवसमम्",
        FloatOp::Lt => "प्लवन्यूनम्",
        FloatOp::Le => "प्लवानधिकम्",
        FloatOp::FromInt => "प्लवरूपान्तरम्ॱप६४ॱअ६४",
        FloatOp::ToInt => "प्लवरूपान्तरम्ॱअ६४ॱप६४",
        FloatOp::FromBits | FloatOp::ToBits => "प्लवसंचारः",
    }
}

/// `V-005` — `Float(op, args)`: ONE line, `<verb> R(v)म् R(a)न [R(b)न [R(c)न]] ।`.
/// Operand `i` is read through scratch `[0, 1, 3][i]` of ITS file — the third
/// is `fmadd`'s and must not share the result's scratch २ — and the result is
/// written through scratch २, as every binary kind does. A read and the write
/// may be in different files (a compare reads floats and writes an integer):
/// [`Routine::read`] and [`Routine::write`] each take the file from the value.
fn lower_float(rt: &mut Routine<'_>, v: ValueId, op: FloatOp, args: &[ValueId]) {
    const SCRATCH: [u8; 3] = [0, 1, 3];
    let srcs: Vec<String> = args
        .iter()
        .zip(SCRATCH)
        .map(|(a, t)| rt.read(*a, t, 0))
        .collect();
    let (rd, store) = rt.write(v, 2);
    let mut line = format!("{} {rd}म्", float_verb(op));
    for s in &srcs {
        line.push_str(&format!(" {s}न"));
    }
    line.push_str(" ।");
    rt.line(&line);
    if let Some(s) = store {
        rt.line(&s);
    }
}

/// `V-008` part 2 — `vsetvli`'s word, `spec/encodings-riscv64.tsv`'s
/// (owner-confirmed 2026-10-05); `the_vector_words_are_the_tables` pins it.
pub const VECTOR_SET_LENGTH: &str = "व्यूहदैर्घ्यम्";

/// The stem of the vector registers `व्यूह०`..`व्यूह३१` (owner ruling,
/// `spec/registers-riscv64.tsv`), pinned by the same test.
pub const VECTOR_REGISTER_STEM: &str = "व्यूह";

/// `vtype` for SEW 64, LMUL m8, tail- and mask-agnostic: vsew 3 at bits 5..3,
/// vlmul 3 at bits 2..0, vta bit 6, vma bit 7 — `0xdb`, written as the plain
/// numeral the owner ruled (design D4, D5). VLMAX at VLEN 128 is 16.
pub const VECTOR_TYPE_E64_M8: i64 = 219;

/// The infix of the labels a vector expansion defines INSIDE one instruction —
/// `<routine>खण्ड<value>…` (ADR-0043 D2: `खण्ड`, the strip, is internal).
/// [`check_branch_ranges`] skips the conditionals that target them: they are
/// the expansion's own, and a block's terminator is the first conditional
/// that is NOT one of them.
pub const VECTOR_LABEL_INFIX: &str = "खण्ड";

fn vector_register(n: i64) -> String {
    format!("{VECTOR_REGISTER_STEM}{}", devanagari(n))
}

/// `V-008` part 2 — `Vector(op, [dst, a, b])` EXPANDED IN PLACE into the
/// strip-mined loop of the design (D3), the twin of `yantrotsarjana.t1`'s
/// `यन्त्रव्यूहावतरणम्` line for line:
///
/// 1. the three run bases are copied into `क्षणिक०`..`क्षणिक२` — the allocated
///    registers are never written, and a spilled base is read through those
///    same scratches;
/// 2. the LENGTH of each run is its header word at base−8, or ० for the nil
///    base (a fresh run): the result's into `क्षणिक३`, each operand's into
///    `क्षणिक४` and compared — UNEQUAL LENGTHS REFUSE (owner ruling Q2): the
///    code `0x35a` (`ir.t1`'s `व्यूहदैर्घ्यनिषेधः`) in FAIL form, the finisher
///    word [`fail_word`]`(0x35a)` = `0x035a_3333`, then a spin, never a
///    truncated loop;
/// 3. the count is the instruction's value, written before the loop consumes it;
/// 4. the loop: `vsetvli` at e64/m8 takes `vl` from the remaining count, loads a
///    strip of each operand into `v8`/`v16`, computes into `v24`, stores it, and
///    advances all three bases by `vl × 8` and the count by `vl`. `v0` is never
///    touched; `v1`–`v7` stay free (D6).
///
/// The `.vv` operands are written `v8` then `v16`: the first source is GNU's
/// `vs2`, so `vfsub`/`vfdiv` compute `क − ख` and `क ÷ ख` as the scalar ops do.
fn lower_vector(rt: &mut Routine<'_>, v: ValueId, op: FloatOp, args: &[ValueId]) {
    let label = format!("{}{VECTOR_LABEL_INFIX}{}", rt.label, devanagari(v.0 as i64));
    let refuse = format!("{label}निषेध");
    let done = format!("{label}अतिक्रम");
    for (k, a) in args.iter().enumerate() {
        let t = u8::try_from(k).unwrap_or(0);
        let r = rt.read(*a, t, 0);
        rt.line(&format!("योगः {}म् {r}न ०न ।", temp(t)));
    }
    // The lengths: the result run's into क्षणिक३, each operand's into क्षणिक४.
    for (k, into) in [(0u8, 3u8), (1, 4), (2, 4)] {
        let skip = format!("{label}दैर्घ्य{}", devanagari(i64::from(k)));
        rt.line(&format!("योगः {}म् {ZERO}न ०न ।", temp(into)));
        rt.line(&format!("समलङ्घनम् {}न {ZERO}त् {skip}य् ।", temp(k)));
        rt.line(&format!("आहारः {}म् {}त् ऋण८न ।", temp(into), temp(k)));
        rt.line(&format!("{skip}ॱॱ"));
        if k > 0 {
            rt.line(&format!("विषमलङ्घनम् {}न {}त् {refuse}य् ।", temp(3), temp(4)));
        }
    }
    let (rd, store) = rt.write(v, 5);
    rt.line(&format!("योगः {rd}म् {}न ०न ।", temp(3)));
    if let Some(s) = store {
        rt.line(&s);
    }
    rt.line(&format!("समलङ्घनम् {}न {ZERO}त् {done}य् ।", temp(3)));
    rt.line(&format!("{label}ॱॱ"));
    rt.line(&format!(
        "{VECTOR_SET_LENGTH} {}म् {}न {}न ।",
        temp(4),
        temp(3),
        devanagari(VECTOR_TYPE_E64_M8)
    ));
    rt.line(&format!("आहारः {}म् {}त् ।", vector_register(8), temp(1)));
    rt.line(&format!("आहारः {}म् {}त् ।", vector_register(16), temp(2)));
    rt.line(&format!(
        "{} {}म् {}न {}न ।",
        float_verb(op),
        vector_register(24),
        vector_register(8),
        vector_register(16)
    ));
    rt.line(&format!("निधानम् {}य् {}न ।", temp(0), vector_register(24)));
    rt.line(&format!("वामसरणम् {}म् {}न ३न ।", temp(5), temp(4)));
    for k in 0..3u8 {
        rt.line(&format!("योगः {}म् {}न {}न ।", temp(k), temp(k), temp(5)));
    }
    rt.line(&format!("वियोगः {}म् {}न {}न ।", temp(3), temp(3), temp(4)));
    rt.line(&format!("विषमलङ्घनम् {}न {ZERO}त् {label}य् ।", temp(3)));
    rt.line(&format!("लङ्घनम् {ZERO}म् {done}य् ।"));
    rt.line(&format!("{refuse}ॱॱ"));
    rt.line(&format!("उपरिभारः {}म् {FINISHER_HI}न ।", temp(5)));
    // `W-381`: the FAIL-form word holds no `addi` immediate, so `lui` its high
    // twenty bits and `addi` the low twelve (`0x333`, below `0x800`, so no
    // borrow) — `yantrotsarjana.t1`'s two lines, number for number.
    let word = fail_word(crate::t1::nirvahana::VECTOR_LENGTH_REFUSAL);
    rt.line(&format!(
        "उपरिभारः {}म् {}न ।",
        temp(6),
        devanagari((word / 4096) as i64)
    ));
    rt.line(&format!(
        "योगः {}म् {}न {}न ।",
        temp(6),
        temp(6),
        devanagari((word % 4096) as i64)
    ));
    rt.line(&format!("निधानम्ॱअ३२ {}य् ०न {}न ।", temp(5), temp(6)));
    rt.line(&format!("लङ्घनम् {ZERO}म् {refuse}य् ।"));
    rt.line(&format!("{done}ॱॱ"));
}

/// `W-381` — A REFUSAL CODE'S FINISHER WORD: sifive-test's FAIL form,
/// `0x3333 | code << 16`, the twin of `ir.t1`'s `समापकविफलशब्दः`. yantra halts
/// on any finisher store, but QEMU (and hardware) acts only on PASS and FAIL
/// words and runs past any other; in this form both stop, and both report
/// `code` as the status. `ir.t1`'s own two refusals reach this emitter as IR
/// constants already in this form; the vector expansion below calls it.
#[must_use]
pub const fn fail_word(code: u64) -> u64 {
    (code << 16) | 0x3333
}

// ── `V-009` part (ii): THE MODULE'S MATRIX KERNEL ────────────────────────

/// `ir.t1`'s `आव्यूहवृत्तिसंज्ञा`: the symbol of a module's synthesised PRODUCT
/// kernel routine (`व्यूहॱआव्यूहगुणनम्` and `व्यूहॱसमासः` call it), named
/// (module, `आव्यूहगुणनम्`) by both drivers.
pub const MATRIX_PRODUCT_SYMBOL: SymbolId = SymbolId(10_000_006);
/// `ir.t1`'s `व्युत्क्रमवृत्तिसंज्ञा`: the TRANSPOSE kernel routine's symbol
/// (`व्यूहॱव्युत्क्रमः` calls it), named (module, `व्युत्क्रमः`).
pub const MATRIX_TRANSPOSE_SYMBOL: SymbolId = SymbolId(10_000_007);

/// The kernel's mnemonics by code − 1, the words of `spec/encodings-riscv64.tsv`
/// for the RISC-V forms named beside them (`the_matrix_kernel_words_are_the_tables`
/// pins them). `yantrotsarjana.t1`'s `यन्त्राव्यूहक्रियापदम्` is the twin.
const MATRIX_KERNEL_VERBS: [&str; 15] = [
    "योगः",       // add
    "गुणनम्",       // mul
    "विकल्पः",     // or
    "दक्षिणसरणम्",  // srli
    "वामसरणम्",    // slli
    "वियोगः",     // sub
    "आहारः",      // ld
    "निधानम्",     // sd
    "समलङ्घनम्",    // beq
    "विषमलङ्घनम्",  // bne
    "लङ्घनम्",      // jal
    "सापेक्षलङ्घनम्", // jalr
    "व्यूहदैर्घ्यम्",   // vsetvli
    "प्लवगुणनम्",    // vfmul.vv
    "प्लवयोगः",    // vfadd.vv
];

/// THE KERNEL, READABLE, one row per line — what [`MATRIX_KERNEL_TABLE`]
/// encodes, and `the_compact_kernel_table_is_the_readable_one` decodes the
/// table and compares it with these rows (all three generated from one listing).
/// A row is `[verb, op, op, op]`; verb ० is not an instruction: `[0, e, 0, 0]`
/// opens entry `e` (१ the product, २ the transpose), `[0, 0, n, 0]` places the
/// kernel's label `n`, and `[0, 0, n, c]` places label `n` with a refusal block
/// for code `c` (१ `0x35a`, २ `0x35b`). An operand is `role × 256 + register`
/// (role १ `म्`, २ `न`, ३ `त्`, ४ `य्`; register ०–६ `t0`–`t6`, १०–१७ `a0`–`a7`,
/// २० zero, २१ sp, २२ ra, ३२+n `v<n>`), an immediate `1408 + value`, or a
/// label `1792 + n` — [`matrix_kernel_operand`] reads them.
///
/// The product (`a0` C, `a1` A, `a2` B, `a3` batch, `a4` M, `a5` K, `a6` N,
/// all row-major — `ir.t1` permuted a column-major call already — and `a7`
/// non-zero for `परिवर्तितम्`, A stored K×M and read with its row and `k`
/// strides swapped, `8` and `M·8` for `K·8` and `8`): the three
/// lengths (० for a nil base); every dimension and each product of two below
/// 2^32, so nothing wraps; the lengths against the shape, else `0x35a`; an
/// empty result answers ०; a result run that is an operand run is `0x35b`.
/// Then per batch, row and strip of the row (`vsetvli` e64/m8, `vl` ≤ 16): the
/// accumulator `v24` = +0.0 (a stride-0 `vlse64` of a zero word on the stack),
/// and for each `k` in order `v8` = A[i,k] in every lane (a stride-0 `vlse64`),
/// `v16` = B[k, j..j+vl] (`vle64`), `v8 = v8 · v16`, `v24 = v24 + v8` — per
/// lane exactly the interpreter's `s = s + a·b` from `s = +0.0`, two roundings,
/// the same order — and the strip stored (`vse64`). The transpose (`a0` C, `a1`
/// A, `a2` M, `a3` N) reads column j with a stride-N·8 `vlse64` and stores it as
/// row j. Both answer the element count. `v0` is never named.
#[cfg(test)]
#[rustfmt::skip]
const MATRIX_KERNEL: &[[u16; 4]] = &[
    [0, 1, 0, 0], // the product entry
    [1, 256, 532, 1408], // add t0, zero, 0
    [9, 522, 788, 1793], // beq a0, zero, L1
    [7, 256, 778, 1400], // ld t0, a0, -8
    [0, 0, 1, 0], // L1:
    [1, 257, 532, 1408], // add t1, zero, 0
    [9, 523, 788, 1794], // beq a1, zero, L2
    [7, 257, 779, 1400], // ld t1, a1, -8
    [0, 0, 2, 0], // L2:
    [1, 258, 532, 1408], // add t2, zero, 0
    [9, 524, 788, 1795], // beq a2, zero, L3
    [7, 258, 780, 1400], // ld t2, a2, -8
    [0, 0, 3, 0], // L3:
    [3, 259, 525, 526], // or t3, a3, a4
    [3, 259, 515, 527], // or t3, t3, a5
    [3, 259, 515, 528], // or t3, t3, a6
    [2, 260, 526, 527], // mul t4, a4, a5
    [2, 261, 527, 528], // mul t5, a5, a6
    [2, 262, 526, 528], // mul t6, a4, a6
    [3, 259, 515, 516], // or t3, t3, t4
    [3, 259, 515, 517], // or t3, t3, t5
    [3, 259, 515, 518], // or t3, t3, t6
    [4, 259, 515, 1440], // srli t3, t3, 32
    [10, 515, 788, 1803], // bne t3, zero, L11
    [2, 260, 516, 525], // mul t4, t4, a3
    [10, 516, 769, 1803], // bne t4, t1, L11
    [2, 261, 517, 525], // mul t5, t5, a3
    [10, 517, 770, 1803], // bne t5, t2, L11
    [2, 262, 518, 525], // mul t6, t6, a3
    [10, 518, 768, 1803], // bne t6, t0, L11
    [9, 518, 788, 1802], // beq t6, zero, L10
    [9, 522, 779, 1804], // beq a0, a1, L12
    [9, 522, 780, 1804], // beq a0, a2, L12
    [1, 277, 533, 1360], // add sp, sp, -48
    [8, 1045, 1416, 518], // sd sp, 8, t6
    [8, 1045, 1408, 532], // sd sp, 0, zero
    [8, 1045, 1424, 525], // sd sp, 16, a3
    [5, 260, 527, 1411], // slli t4, a5, 3
    [1, 269, 532, 1416], // add a3, zero, 8
    [1, 261, 532, 1408], // add t5, zero, 0
    [9, 529, 788, 1809], // beq a7, zero, L17
    [5, 269, 526, 1411], // slli a3, a4, 3
    [2, 261, 525, 527], // mul t5, a3, a5
    [6, 261, 517, 525], // sub t5, t5, a3
    [1, 260, 532, 1416], // add t4, zero, 8
    [0, 0, 17, 0], // L17:
    [8, 1045, 1432, 516], // sd sp, 24, t4
    [8, 1045, 1440, 517], // sd sp, 32, t5
    [5, 273, 528, 1411], // slli a7, a6, 3
    [0, 0, 4, 0], // L4:
    [1, 256, 526, 1408], // add t0, a4, 0
    [0, 0, 5, 0], // L5:
    [1, 257, 528, 1408], // add t1, a6, 0
    [1, 258, 524, 1408], // add t2, a2, 0
    [0, 0, 6, 0], // L6:
    [13, 259, 513, 1627], // vsetvli t3, t1, 219
    [7, 312, 789, 532], // ld v24, sp, zero
    [1, 260, 523, 1408], // add t4, a1, 0
    [1, 261, 514, 1408], // add t5, t2, 0
    [1, 262, 527, 1408], // add t6, a5, 0
    [9, 518, 788, 1800], // beq t6, zero, L8
    [0, 0, 7, 0], // L7:
    [7, 296, 772, 532], // ld v8, t4, zero
    [7, 304, 773, 0], // ld v16, t5
    [14, 296, 552, 560], // vfmul.vv v8, v8, v16
    [15, 312, 568, 552], // vfadd.vv v24, v24, v8
    [1, 260, 516, 525], // add t4, t4, a3
    [1, 261, 517, 529], // add t5, t5, a7
    [1, 262, 518, 1407], // add t6, t6, -1
    [10, 518, 788, 1799], // bne t6, zero, L7
    [0, 0, 8, 0], // L8:
    [8, 1034, 568, 0], // sd a0, v24
    [5, 260, 515, 1411], // slli t4, t3, 3
    [1, 266, 522, 516], // add a0, a0, t4
    [1, 258, 514, 516], // add t2, t2, t4
    [6, 257, 513, 515], // sub t1, t1, t3
    [10, 513, 788, 1798], // bne t1, zero, L6
    [7, 260, 789, 1432], // ld t4, sp, 24
    [1, 267, 523, 516], // add a1, a1, t4
    [1, 256, 512, 1407], // add t0, t0, -1
    [10, 512, 788, 1797], // bne t0, zero, L5
    [7, 260, 789, 1440], // ld t4, sp, 32
    [1, 267, 523, 516], // add a1, a1, t4
    [2, 260, 527, 529], // mul t4, a5, a7
    [1, 268, 524, 516], // add a2, a2, t4
    [7, 260, 789, 1424], // ld t4, sp, 16
    [1, 260, 516, 1407], // add t4, t4, -1
    [8, 1045, 1424, 516], // sd sp, 16, t4
    [10, 516, 788, 1796], // bne t4, zero, L4
    [0, 0, 9, 0], // L9:
    [7, 266, 789, 1416], // ld a0, sp, 8
    [1, 277, 533, 1456], // add sp, sp, 48
    [12, 276, 790, 1408], // jalr zero, ra, 0
    [0, 0, 10, 0], // L10:
    [1, 266, 532, 1408], // add a0, zero, 0
    [12, 276, 790, 1408], // jalr zero, ra, 0
    [0, 0, 11, 1], // L11: the shape refusal
    [0, 0, 12, 2], // L12: the alias refusal
    [0, 2, 0, 0], // the transpose entry
    [1, 256, 532, 1408], // add t0, zero, 0
    [9, 522, 788, 1805], // beq a0, zero, L13
    [7, 256, 778, 1400], // ld t0, a0, -8
    [0, 0, 13, 0], // L13:
    [1, 257, 532, 1408], // add t1, zero, 0
    [9, 523, 788, 1806], // beq a1, zero, L14
    [7, 257, 779, 1400], // ld t1, a1, -8
    [0, 0, 14, 0], // L14:
    [3, 259, 524, 525], // or t3, a2, a3
    [4, 259, 515, 1440], // srli t3, t3, 32
    [10, 515, 788, 1803], // bne t3, zero, L11
    [2, 262, 524, 525], // mul t6, a2, a3
    [10, 518, 768, 1803], // bne t6, t0, L11
    [10, 518, 769, 1803], // bne t6, t1, L11
    [9, 518, 788, 1802], // beq t6, zero, L10
    [9, 522, 779, 1804], // beq a0, a1, L12
    [5, 273, 525, 1411], // slli a7, a3, 3
    [1, 277, 533, 1360], // add sp, sp, -48
    [8, 1045, 1416, 518], // sd sp, 8, t6
    [0, 0, 15, 0], // L15:
    [1, 256, 524, 1408], // add t0, a2, 0
    [1, 257, 523, 1408], // add t1, a1, 0
    [0, 0, 16, 0], // L16:
    [13, 259, 512, 1627], // vsetvli t3, t0, 219
    [7, 296, 769, 529], // ld v8, t1, a7
    [8, 1034, 552, 0], // sd a0, v8
    [5, 260, 515, 1411], // slli t4, t3, 3
    [1, 266, 522, 516], // add a0, a0, t4
    [2, 260, 515, 529], // mul t4, t3, a7
    [1, 257, 513, 516], // add t1, t1, t4
    [6, 256, 512, 515], // sub t0, t0, t3
    [10, 512, 788, 1808], // bne t0, zero, L16
    [1, 267, 523, 1416], // add a1, a1, 8
    [1, 269, 525, 1407], // add a3, a3, -1
    [10, 525, 788, 1807], // bne a3, zero, L15
    [11, 276, 1801, 0], // jal zero, L9
];

/// THE KERNEL AS ONE COMPACT TABLE (owner ruling 2026-10-06, option (A)): four
/// LETTERS per row — the verb, then three operands — letter for letter the
/// literal `yantrotsarjana.t1`'s `यन्त्राव्यूहकायोत्सर्जनम्` loops over. A letter
/// is a number below 64: `अ`..`ह` (U+0905 on) are 0..52 and `ॲ`..`ॼ` (U+0972 on)
/// 53..63, independent letters only. Verb 0 is a control row, its numbers as
/// written; otherwise an operand letter is 0 for none, the label number in a
/// branch's label slot, or an index from 1 into [`MATRIX_KERNEL_OPERANDS`].
const MATRIX_KERNEL_TABLE: &str = "अआअअआआदॳऎञळआऌआरहअअआअआइदॳऎटळइऌइऱहअअइअआईदॳऎठळईऌईलहअअईअईउडढईउचणईउचतइऊढणइऋणतइऌढतईउचछईउचजईउचझउउचॸएचळऐइऊछडएछबऐइऋजडएजभऐइऌझडएझफऐऎझळएऎञऱऑऎञलऑआओधसऍषॵझऍषॳदऍषॶडऊऊणॴआऐदॵआऋदॳऎथळखऊऐढॴइऋडणऋऋजडआऊदॵअअखअऍषॷछऍषॸजऊऑतॴअअउअआआढॳअअऊअआइतॳआईठॳअअऋअऒउघॺऌखऴदआऊटॳआऋङॳआऌणॳऎझळऍअअऌअऌऔमदऌकयअओऔनऩऔखपनआऊछडआऋजथआऌझॲएझळऌअअऍअऍशपअऊऊचॴआऍञछआईङछऋइघचएघळऋऌऊऴॷआऎटछआआगॲएगळऊऌऊऴॸआऎटछइऊणथआएठछऌऊऴॶआऊछॲऍषॶछएछळउअअऎअऌऍऴॵआओधॹऑऒवॳअअएअआऍदॳऑऒवॳअअऐआअअऑइअइअअआआदॳऎञळऒऌआरहअअऒअआइदॳऎटळओऌइऱहअअओअईउठडउउचॸएचळऐइऌठडएझफऐएझबऐऎझळएऎञऱऑऊऑडॴआओधसऍषॵझअअऔअआआठॳआइटॳअअकअऒउगॺऌऔबथऍशनअऊऊचॴआऍञछइऊचथआइघछऋआगचएगळकआऎटॵआऐडॲएडळऔऐऒऎअ";

/// The operand codes the table indexes, two letters each, high six bits first.
const MATRIX_KERNEL_OPERANDS: &str = "उअउआउइउईउउउऊउऋउएउऐउऑउऒउखउङउचउभउवउॵऍअऍआऍइऍईऍउऍऊऍऋऍएऍऐऍऑऍऒऍओऍऔऍकऍखऍङऍचऍभऍवऍॵऑअऑआऑइऑउऑऊऑएऑऐऑऑऑङऑचऑछकएकचचकचॵचॼछअछईछऍछकछझछथछवञठ";

/// The number a table letter stands for — `यन्त्राव्यूहाक्षरमूल्यम्`.
fn matrix_kernel_letter(c: char) -> u16 {
    let cp = c as u32;
    let v = if cp >= 0x972 {
        cp - 0x972 + 53
    } else {
        cp - 0x905
    };
    u16::try_from(v).unwrap_or(0)
}

/// Operand letter `v` of a row of verb `verb` in slot `slot` (1..3), as the
/// code [`matrix_kernel_operand`] reads — `यन्त्राव्यूहपदसङ्केतः`.
fn matrix_kernel_operand_code(verb: u16, slot: usize, v: u16) -> u16 {
    if v == 0 {
        return 0;
    }
    let label_slot = match verb {
        9 | 10 => 3,
        11 => 2,
        _ => 0,
    };
    if slot == label_slot {
        return v + 1792;
    }
    let at = usize::from(v - 1) * 2;
    let letters: Vec<char> = MATRIX_KERNEL_OPERANDS.chars().skip(at).take(2).collect();
    matrix_kernel_letter(letters[0]) * 64 + matrix_kernel_letter(letters[1])
}

/// The table's rows, decoded.
fn matrix_kernel_rows() -> Vec<[u16; 4]> {
    let letters: Vec<u16> = MATRIX_KERNEL_TABLE
        .chars()
        .map(matrix_kernel_letter)
        .collect();
    letters
        .chunks(4)
        .map(|r| {
            let verb = r[0];
            if verb == 0 {
                return [0, r[1], r[2], r[3]];
            }
            [
                verb,
                matrix_kernel_operand_code(verb, 1, r[1]),
                matrix_kernel_operand_code(verb, 2, r[2]),
                matrix_kernel_operand_code(verb, 3, r[3]),
            ]
        })
        .collect()
}

/// `matrix_kernel`'s operand `code` as text with its leading space: a
/// register with its role's suffix, an immediate, or a kernel label of `p`.
/// The twin of `yantrotsarjana.t1`'s `यन्त्राव्यूहपदम्`.
fn matrix_kernel_operand(code: u16, p: &str) -> String {
    if code == 0 {
        return String::new();
    }
    if code >= 1792 {
        return format!(
            " {p}{VECTOR_LABEL_INFIX}{}य्",
            devanagari(i64::from(code - 1792))
        );
    }
    if code >= 1280 {
        return format!(" {}न", devanagari(i64::from(code) - 1408));
    }
    let r = code % 256;
    let name = match r {
        0..=6 => temp(u8::try_from(r).unwrap_or(0)),
        10..=17 => arg(usize::from(r - 10)),
        20 => ZERO.to_string(),
        21 => SP.to_string(),
        22 => RA.to_string(),
        _ => vector_register(i64::from(r) - 32),
    };
    let suffix = match code / 256 {
        1 => "म्",
        2 => "न",
        3 => "त्",
        _ => "य्",
    };
    format!(" {name}{suffix}")
}

/// `V-009` part (ii) — a module's synthesised kernel routine as fixed text, or
/// `None` for any other routine. The PRODUCT routine carries the whole kernel,
/// both entries; the TRANSPOSE routine's text is empty, its label being the
/// kernel's second entry. Nothing of either routine's IR is lowered.
///
/// # Errors
/// [`Refusal::UnnamedSymbol`] when the module's names lack either kernel.
pub fn matrix_kernel(func: &Function, names: &Names) -> Result<Option<String>, Refusal> {
    if func.name == MATRIX_TRANSPOSE_SYMBOL {
        return Ok(Some(String::new()));
    }
    if func.name != MATRIX_PRODUCT_SYMBOL {
        return Ok(None);
    }
    let p = routine_label(names, MATRIX_PRODUCT_SYMBOL)?;
    let t = routine_label(names, MATRIX_TRANSPOSE_SYMBOL)?;
    let mut out = String::new();
    for [verb, a, b, c] in matrix_kernel_rows() {
        if verb > 0 {
            out.push_str(MATRIX_KERNEL_VERBS[usize::from(verb - 1)]);
            for o in [a, b, c] {
                out.push_str(&matrix_kernel_operand(o, &p));
            }
            out.push_str(" ।\n");
            continue;
        }
        if a > 0 {
            let entry = if a == 1 { &p } else { &t };
            out.push_str(&format!("॥ वैश्विकम् {entry} ॥\n{entry}ॱॱ\n"));
            continue;
        }
        let label = format!("{p}{VECTOR_LABEL_INFIX}{}", devanagari(i64::from(b)));
        out.push_str(&format!("{label}ॱॱ\n"));
        if c > 0 {
            out.push_str(&matrix_kernel_refusal(&label, c));
        }
    }
    Ok(Some(out))
}

/// The refusal block at `label` for code `c` (१ `0x35a`, २ `0x35b`): the
/// FAIL-form word to the finisher, then a jump to itself — the vector
/// expansion's block (`lower_vector`), number for number. The twin of
/// `yantrotsarjana.t1`'s `यन्त्राव्यूहनिषेधः`.
fn matrix_kernel_refusal(label: &str, c: u16) -> String {
    let code = if c == 1 {
        crate::t1::nirvahana::VECTOR_LENGTH_REFUSAL
    } else {
        crate::t1::nirvahana::REFUSAL_ADHYASA
    };
    let word = fail_word(code);
    format!(
        "उपरिभारः {t5}म् {FINISHER_HI}न ।\nउपरिभारः {t6}म् {hi}न ।\nयोगः {t6}म् {t6}न {lo}न ।\n\
         निधानम्ॱअ३२ {t5}य् ०न {t6}न ।\nलङ्घनम् {ZERO}म् {label}य् ।\n",
        t5 = temp(5),
        t6 = temp(6),
        hi = devanagari((word / 4096) as i64),
        lo = devanagari((word % 4096) as i64),
    )
}

/// The branch instruction of each ADR-0008 condition — the comparison word
/// compounded with `लङ्घनम्` (`spec/mnemonics-riscv64.src.tsv:45–50`).
#[must_use]
pub fn branch_word(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "समलङ्घनम्",
        CmpOp::Ne => "विषमलङ्घनम्",
        CmpOp::Lt => "न्यूनलङ्घनम्",
        CmpOp::Ge => "अन्यूनलङ्घनम्",
        CmpOp::Ltu => "अचिह्नन्यूनलङ्घनम्",
        CmpOp::Geu => "अचिह्नान्यूनलङ्घनम्",
    }
}

/// The condition that branches exactly when `op` does NOT — the half of the
/// far-conditional relaxation that moves no address (`W-306`'s `Next`).
///
/// A B-type conditional reaches ±4 KiB; the J-type `लङ्घनम्` reaches ±1 MiB.
/// So a conditional too far from its target is relaxed by INVERTING it over a
/// jump that carries the far target: `<op>लङ्घनम् … <t>य्` becomes
/// `<inverse(op)>लङ्घनम् … <skip>य्` / `लङ्घनम् शून्यःम् <t>य्` / `<skip>ॱॱ`.
/// This routine is that inversion and nothing else; the addresses it enables
/// moving are [`check_branch_ranges`]'s business.
///
/// **THE SIX ARE THREE PAIRS AND THE TWO FAMILIES DO NOT MIX.** `Ge` is the
/// inverse of `Lt` and `Geu` of `Ltu`, never `Geu` of `Lt`: `blt`/`bge`
/// compare sign-extended words and `bltu`/`bgeu` do not, so a crossed pair
/// would invert the comparison AND its signedness at once and be wrong on
/// exactly the operands whose top bit differs — `-1 < 1` signed is
/// `0xffff…f >= 1` unsigned, and both readings would take the same arm. That
/// is the case the T1 twin's agreement test names, because the crossed table
/// is the one a reader writes by pairing the words in the order they are
/// declared.
#[must_use]
pub fn inverse_condition(op: CmpOp) -> CmpOp {
    match op {
        CmpOp::Eq => CmpOp::Ne,
        CmpOp::Ne => CmpOp::Eq,
        CmpOp::Lt => CmpOp::Ge,
        CmpOp::Ge => CmpOp::Lt,
        CmpOp::Ltu => CmpOp::Geu,
        CmpOp::Geu => CmpOp::Ltu,
    }
}

/// The verb of each binary kind, as the ISA has it. `Add`/`Sub` were the two
/// the emitter began with; `W-245` added the eight ADR-0032 froze.
fn binary_verb(inst: &Instruction) -> Option<&'static str> {
    Some(match inst {
        Instruction::Add(..) => "योगः",
        Instruction::Sub(..) => "वियोगः",
        Instruction::Mul(..) => "गुणनम्",
        Instruction::Div(..) => "भागः",
        Instruction::Rem(..) => "शेषः",
        // `W-381` stage 3 (ruling (b)): over a name declared unsigned.
        Instruction::DivU(..) => "अचिह्नभागः",
        Instruction::RemU(..) => "अचिह्नशेषः",
        Instruction::Shl(..) => "वामसरणम्",
        // arithmetic since 2026-09-14: the source operator `दक्षिणसृ` is the
        // interpreter's sign-preserving shift and the corpus's hi/lo splits rely on it
        // (see yantrotsarjana.t1's margin at the same line).
        Instruction::Shr(..) => "सचिह्नदक्षिणसरणम्",
        // `W-333`: the same source operator over a NAME declared unsigned. `sra`
        // on a `न६४` with bit 63 set sign-extends — `क दक्षिणसृ ६३` over
        // `क ॱॱ न६४ भवति ऋण१` answered all-ones — so that case is `srl`. The
        // five unmasked corpus shifts the margin above protects are all signed
        // and stay `Shr`.
        Instruction::ShrL(..) => "दक्षिणसरणम्",
        Instruction::And(..) => "युक्तम्",
        Instruction::Or(..) => "विकल्पः",
        Instruction::Xor(..) => "वैषम्यम्",
        _ => return None,
    })
}

/// `Cmp(op, a, b)` READ AS A VALUE (`W-245`): १ or ० into `R(v)`. `Lt`/`Ltu`
/// are the ISA's own `slt`/`sltu`; `Ge`/`Geu` are those with the bit flipped
/// (`xori … १`); `Eq`/`Ne` are a `sub` and then `sltiu … १` (`seqz`) or
/// `sltu … शून्यः …` (`snez`).
fn lower_compare_value(rt: &mut Routine<'_>, v: ValueId, op: CmpOp, a: ValueId, b: ValueId) {
    let ra = rt.read(a, 0, 0);
    let rb = rt.read(b, 1, 0);
    let (rd, store) = rt.write(v, 2);
    match op {
        CmpOp::Lt => rt.line(&format!("न्यूनम् {rd}म् {ra}न {rb}न ।")),
        CmpOp::Ltu => rt.line(&format!("अचिह्नन्यूनम् {rd}म् {ra}न {rb}न ।")),
        CmpOp::Ge => {
            rt.line(&format!("न्यूनम् {rd}म् {ra}न {rb}न ।"));
            rt.line(&format!("वैषम्यम् {rd}म् {rd}न १न ।"));
        }
        CmpOp::Geu => {
            rt.line(&format!("अचिह्नन्यूनम् {rd}म् {ra}न {rb}न ।"));
            rt.line(&format!("वैषम्यम् {rd}म् {rd}न १न ।"));
        }
        CmpOp::Eq => {
            rt.line(&format!("वियोगः {rd}म् {ra}न {rb}न ।"));
            rt.line(&format!("अचिह्नन्यूनम् {rd}म् {rd}न १न ।"));
        }
        CmpOp::Ne => {
            rt.line(&format!("वियोगः {rd}म् {ra}न {rb}न ।"));
            rt.line(&format!("अचिह्नन्यूनम् {rd}म् {ZERO}न {rd}न ।"));
        }
    }
    if let Some(s) = store {
        rt.line(&s);
    }
}

/// The `Cmp` a block's `CondBranch` FUSES with (`W-245`): the block's LAST
/// instruction, defining the condition, and read by nothing else in the
/// function — no instruction, no other terminator. Last, so that no instruction
/// between the compare and the branch can reuse its operands' registers; read
/// once, so that dropping its value loses nothing.
#[must_use]
pub fn fusable_compare(func: &Function, block: &Block) -> Option<(CmpOp, ValueId, ValueId)> {
    let Some(Terminator::CondBranch(c, _, _)) = block.terminator else {
        return None;
    };
    let Some((v, Instruction::Cmp(op, a, b))) = block.insts.last() else {
        return None;
    };
    if *v != c {
        return None;
    }
    // Read elsewhere: by any instruction, or by any OTHER block's terminator
    // (this block's is the branch that fuses).
    let read_elsewhere = func.blocks.values().any(|b| {
        b.insts.iter().any(|(_, i)| i.operands().contains(&c))
            || (b.id != block.id
                && matches!(
                    b.terminator,
                    Some(Terminator::CondBranch(x, _, _)) | Some(Terminator::Return(Some(x))) if x == c
                ))
    });
    (!read_elsewhere).then_some((*op, *a, *b))
}

/// `CondBranch(c, t, e)` (§2.5, §2.7): FUSED with the compare that ends its
/// block — `<op>लङ्घनम् R(a)न R(b)त् <t>य् ।`, the six ADR-0008 conditions, करण
/// the subject and अपादान the standard — or, for a condition that is a VALUE
/// (a call's result, a loaded बूल), the synthesized `विषमलङ्घनम् R(c)न शून्यःत्`;
/// then a jump to `e` unless `e` falls through. Until `W-245` the IR had no
/// `Cmp` and the second form was the whole of this arm.
///
/// **`relax` IS `W-306`'s SECOND PASS AND IT MOVES NO OCTET WHEN IT IS FALSE.**
/// A B-type conditional reaches ±4 KiB and the J-type `लङ्घनम्` reaches ±1 MiB,
/// so a conditional too far from its target is relaxed by INVERTING it
/// ([`inverse_condition`]) over a jump that carries the far target:
///
/// ```text
/// <inverse(op)>लङ्घनम् R(a)न R(b)त् <skip>य् ।
/// लङ्घनम् शून्यःम् <t>य् ।
/// <skip>ॱॱ
/// ```
///
/// The inverted conditional branches over EXACTLY ONE instruction — eight
/// bytes — so the relaxed form is in range by construction and the relaxed set
/// can never have to grow a second time. That is why [`emit_function`] needs
/// two passes and not a fixpoint.
///
/// THE SKIP LABEL IS PER BLOCK, `…पर्व<n>अतिक्रम`, and it is why `block` is a
/// parameter. Keying it by the TARGET instead would collide the moment two
/// blocks relax a branch to one join — the ordinary `यदि` inside `यावत्` shape
/// that `W-306`'s first half found the range guard blind on — and the second
/// definition would silently win.
// Eight: the branch's own five, plus the block that WROTE it (the skip label is
// keyed by it) and the pass flag. Splitting them into a struct would name the
// same seven fields one indirection away and leave the `.t1` twin — which takes
// them as seven parameters, T1 having no such struct — harder to read against.
#[allow(clippy::too_many_arguments)]
fn lower_cond_branch(
    rt: &mut Routine<'_>,
    c: ValueId,
    block: BlockId,
    then: BlockId,
    els: BlockId,
    next: Option<BlockId>,
    fused: Option<(CmpOp, ValueId, ValueId)>,
    relax: bool,
) {
    let label = rt.label.clone();
    let skip = format!("{}अतिक्रम", block_label(&label, block));
    // Relaxed, the conditional carries the SKIP and the jump carries the target;
    // near, the conditional carries the target and there is no jump at all.
    let taken = if relax {
        skip.clone()
    } else {
        block_label(&label, then)
    };
    match fused {
        Some((op, a, b)) => {
            let ra = rt.read(a, 0, 0);
            let rb = rt.read(b, 1, 0);
            let op = if relax { inverse_condition(op) } else { op };
            rt.line(&format!("{} {ra}न {rb}त् {taken}य् ।", branch_word(op)));
        }
        None => {
            let cond = rt.read(c, 0, 0);
            // `विषमलङ्घनम्` is `Ne` against शून्यः; its inverse is `Eq`, and it is
            // spelled through `inverse_condition`/`branch_word` rather than
            // written out so the two halves cannot drift apart.
            let word = if relax {
                branch_word(inverse_condition(CmpOp::Ne))
            } else {
                "विषमलङ्घनम्"
            };
            rt.line(&format!("{word} {cond}न {ZERO}त् {taken}य् ।"));
        }
    }
    if relax {
        rt.line(&format!("लङ्घनम् {ZERO}म् {}य् ।", block_label(&label, then)));
        rt.line(&format!("{skip}ॱॱ"));
    }
    if next != Some(els) {
        rt.line(&format!("लङ्घनम् {ZERO}म् {}य् ।", block_label(&label, els)));
    }
}

fn emit_terminator(
    rt: &mut Routine<'_>,
    block: &Block,
    next: Option<BlockId>,
    fused: Option<(CmpOp, ValueId, ValueId)>,
    relax: bool,
) -> Result<(), Refusal> {
    let label = rt.label.clone();
    match &block.terminator {
        None => {
            return Err(Refusal::NoTerminator {
                function: label,
                block: block.id,
            });
        }
        Some(Terminator::Unreachable) => {
            return Err(Refusal::Unreachable {
                function: label,
                block: block.id,
            });
        }
        Some(Terminator::Return(value)) => {
            if let Some(x) = value {
                // `V-005`: a float result goes out in `fa0` (`abi.rs`), copied
                // with the sign-injection move `fsgnj.d fa0, v, v`; `ir.t1` has
                // already put the value in the RETURN TYPE's file.
                let (class, _) = rt.location(*x);
                let src = rt.read(*x, 0, 0);
                match class {
                    RegClass::Int => rt.line(&format!("योगः {}म् {src}न ०न ।", arg(0))),
                    RegClass::Float => {
                        rt.line(&format!("{FLOAT_MOVE} {}म् {src}न {src}न ।", float_arg(0)))
                    }
                }
            }
            if next.is_some() {
                rt.line(&format!("लङ्घनम् {ZERO}म् {}य् ।", exit_label(&label)));
            }
        }
        Some(Terminator::Branch(b)) => {
            if next != Some(*b) {
                rt.line(&format!("लङ्घनम् {ZERO}म् {}य् ।", block_label(&label, *b)));
            }
        }
        Some(Terminator::CondBranch(c, t, e)) => {
            lower_cond_branch(rt, *c, block.id, *t, *e, next, fused, relax);
        }
    }
    Ok(())
}

/// Check what the emitter cannot lower before writing a line of it (§2.9).
fn verify(func: &Function, label: &str) -> Result<(), Refusal> {
    let mut ids: Vec<BlockId> = func.blocks.keys().copied().collect();
    ids.sort_by_key(|b| b.0);
    for id in ids {
        let block = &func.blocks[&id];
        let targets: Vec<BlockId> = match block.terminator {
            Some(Terminator::CondBranch(_, t, e)) => vec![t, e],
            Some(Terminator::Branch(t)) => vec![t],
            _ => Vec::new(),
        };
        for target in targets {
            if !func.blocks.contains_key(&target) {
                return Err(Refusal::TargetNotInFunction {
                    function: label.to_string(),
                    from: id,
                    target,
                });
            }
        }
        let mut called = false;
        for (_, inst) in &block.insts {
            match inst {
                Instruction::Call(..) | Instruction::CallFloat(..) => called = true,
                Instruction::Param(i) | Instruction::ParamFloat(i) if id != func.entry_block => {
                    return Err(Refusal::ParamOutsideEntry {
                        function: label.to_string(),
                        block: id,
                        param: *i,
                    });
                }
                Instruction::Param(i) | Instruction::ParamFloat(i) if called => {
                    return Err(Refusal::ParamAfterCall {
                        function: label.to_string(),
                        block: id,
                        param: *i,
                    });
                }
                _ => {}
            }
        }
    }
    // `V-005` — EVERY OPERAND IN ITS OWN FILE, as a second pass over the blocks
    // in id order after the checks above (`यन्त्रपरीक्षा` keeps the same order).
    // A `Store` may take either file — it stores the value's own bits; a float
    // op reads the files `FloatOp::reads_float` names; every other operand and
    // a Return's or CondBranch's value must be an integer. See
    // [`Refusal::FileMismatch`] for why a mismatch is refused and not lowered.
    let class: HashMap<ValueId, RegClass> = func
        .blocks
        .values()
        .flat_map(|b| b.insts.iter())
        .map(|(v, i)| (*v, value_class(i)))
        .collect();
    let is_float = |v: &ValueId| class.get(v) == Some(&RegClass::Float);
    let mut ids: Vec<BlockId> = func.blocks.keys().copied().collect();
    ids.sort_by_key(|b| b.0);
    for id in ids {
        let block = &func.blocks[&id];
        for (_, inst) in &block.insts {
            let wrong = match inst {
                // `V-005`: a call's arguments travel in EITHER file — `a` or `fa`
                // registers by `abi.rs`'s count, `ir.t1` having moved each into
                // its parameter's file — and a Store takes either.
                Instruction::Store(..) | Instruction::Call(..) | Instruction::CallFloat(..) => None,
                // `V-008`: a word stored AT an address takes either file — a
                // float goes out with `प्लवनिधानम्` — but only as a WHOLE WORD:
                // there is no narrow float store, so a float at any width but ८
                // is refused here rather than truncated. The address is an
                // integer, always.
                Instruction::StoreAt(a, x, w) => {
                    if is_float(a) {
                        Some(*a)
                    } else if is_float(x) && *w != 8 {
                        Some(*x)
                    } else {
                        None
                    }
                }
                Instruction::Float(op, args) => args
                    .iter()
                    .enumerate()
                    .find(|(i, a)| is_float(a) != op.reads_float(*i))
                    .map(|(_, a)| *a),
                other => other.operands().into_iter().find(|a| is_float(a)),
            };
            if let Some(v) = wrong {
                return Err(Refusal::FileMismatch {
                    function: label.to_string(),
                    block: id,
                    value: v.0,
                });
            }
        }
        // A Return's value may be a float (it goes out in `fa0`); a branch
        // condition may not.
        let read = match block.terminator {
            Some(Terminator::CondBranch(v, _, _)) => Some(v),
            _ => None,
        };
        if let Some(v) = read.filter(|v| is_float(v)) {
            return Err(Refusal::FileMismatch {
                function: label.to_string(),
                block: id,
                value: v.0,
            });
        }
    }
    Ok(())
}

/// One routine as T0 text: prologue, blocks in id order with fallthrough, one
/// epilogue (§2.4, §2.5, §2.7). Appends to `pool` the constants it could not
/// materialise inline.
///
/// **`relaxed` IS `W-332`'s CENSUS AND IT CARRIES LABELS, NOT A TALLY.** The
/// routine's label is pushed when — and only when — the second pass fires, so
/// `relaxed.len()` is the count and `relaxed` itself says WHICH routines it
/// counted. A bare `usize` would answer *how many* and leave *which* to a
/// byte-diff, which cannot answer it: relaxation adds text, and any module's
/// size change re-lays-out everything downstream of it, so a moved image is
/// evidence that SOMETHING moved and never evidence of what.
///
/// It is an out-parameter beside `pool` and `strings` rather than a third
/// member of the return tuple because those two are already the shape this
/// signature uses for *what the routine produced besides its text*, and because
/// `emit_module` accumulates all three across the routines of a module.
///
/// # Errors
/// A [`Refusal`], by name, before any text of the routine is returned.
pub fn emit_function(
    func: &Function,
    names: &Names,
    alloc: &AllocationMap,
    float: &AllocationMap,
    pool: &mut Vec<i64>,
    strings: &mut Vec<Vec<u8>>,
    relaxed: &mut Vec<String>,
) -> Result<String, Refusal> {
    let label = routine_label(names, func.name)?;
    verify(func, &label)?;
    // `V-004` PART 5: BOTH maps lay out the frame. An empty float map — every
    // routine with no float value — gives the one-file frame byte for byte
    // (`frame_layout`'s `None` case: no float spill region, no float register
    // saved), which is what keeps the corpus emission unchanged.
    let frame = frame_layout(alloc, Some(float), count_locals(func));
    let mut order: Vec<BlockId> = func.blocks.keys().copied().collect();
    order.sort_by_key(|b| b.0);
    // `V-005`: where each parameter arrives, by `abi.rs`'s two counts over the
    // parameters in order; the highest offset is the deepest STACK parameter's.
    let param_slots = param_locations(func);
    let highest_offset = param_slots
        .values()
        .filter_map(|l| match l {
            AbiLocation::Stack(off) => Some(frame.bytes + *off as i64),
            _ => None,
        })
        .max()
        .unwrap_or(frame.bytes);
    if highest_offset > 2047 {
        return Err(Refusal::FrameTooLarge {
            function: label,
            bytes: frame.bytes,
        });
    }
    let targeted: HashSet<BlockId> = func
        .blocks
        .values()
        .flat_map(|b| match b.terminator {
            Some(Terminator::CondBranch(_, t, e)) => vec![t, e],
            Some(Terminator::Branch(t)) => vec![t],
            _ => Vec::new(),
        })
        .collect();

    // `W-306` — THE TWO PASSES, AND WHY THE POOLS ARE REWOUND BETWEEN THEM.
    // The body below is emitted with every conditional NEAR; if
    // `check_branch_ranges` then finds ANY of them out of range, the WHOLE
    // routine is re-emitted with every conditional RELAXED. It is two passes
    // and not a fixpoint because a relaxed conditional branches over exactly
    // one instruction, so relaxing can never put a third conditional out of
    // range — the loop below is bounded by `relax` going false → true once.
    //
    // `pool` and `strings` are the caller's and are APPENDED TO as the body is
    // written, so a second pass over the same body would push a second copy of
    // every constant the first pass minted. They are rewound to their marks at
    // the top of each pass, which also keeps the indices identical: the same
    // body in the same order mints the same entries.
    //
    // IT IS A `loop` AND NOT A SPLIT-OUT ROUTINE. A second function here would
    // be a module-level symbol with no twin in `yantrotsarjana.t1`, which
    // `every_symbol_the_rust_emitter_declares_is_paired_or_recorded` refuses by
    // name; the `.t1` side takes the same shape — one pass written twice around
    // a flag — so the two halves stay readable against each other.
    let pool_mark = pool.len();
    let strings_mark = strings.len();
    let mut relax = false;
    let out = loop {
        pool.truncate(pool_mark);
        strings.truncate(strings_mark);

        let mut rt = Routine {
            label: label.clone(),
            func,
            alloc,
            float,
            // Cloned per pass: the frame is the same layout both times — the
            // relaxation adds text, never a spill — but `Routine` owns it.
            frame: frame.clone(),
            out: String::new(),
        };
        rt.prologue();
        for (n, id) in order.iter().enumerate() {
            let block = &rt.func.blocks[id];
            let next = order.get(n + 1).copied();
            // The entry block carries no label of its own unless a back-edge
            // targets it (§2.2); every other block is labelled.
            if *id != func.entry_block || targeted.contains(id) {
                rt.line(&format!("{}ॱॱ", block_label(&label, *id)));
            }
            // `W-245`: the compare the branch fuses with emits no line of its own.
            let fused = fusable_compare(func, block);
            let last = block.insts.len().saturating_sub(1);
            for (n, (v, inst)) in block.insts.iter().enumerate() {
                if fused.is_some() && n == last {
                    break;
                }
                match inst {
                    Instruction::ConstInt(c) => lower_constant(&mut rt, *v, *c, pool),
                    Instruction::ConstStr(b) => lower_string(&mut rt, *v, b, strings),
                    Instruction::Add(a, b)
                    | Instruction::Sub(a, b)
                    | Instruction::Mul(a, b)
                    | Instruction::Div(a, b)
                    | Instruction::DivU(a, b)
                    | Instruction::Rem(a, b)
                    | Instruction::RemU(a, b)
                    | Instruction::Shl(a, b)
                    | Instruction::Shr(a, b)
                    | Instruction::ShrL(a, b)
                    | Instruction::And(a, b)
                    | Instruction::Or(a, b)
                    | Instruction::Xor(a, b) => {
                        let ra = rt.read(*a, 0, 0);
                        let rb = rt.read(*b, 1, 0);
                        let (rd, store) = rt.write(*v, 2);
                        let verb = binary_verb(inst).expect("a binary kind has a verb");
                        rt.line(&format!("{verb} {rd}म् {ra}न {rb}न ।"));
                        if let Some(s) = store {
                            rt.line(&s);
                        }
                    }
                    Instruction::Cmp(op, a, b) => lower_compare_value(&mut rt, *v, *op, *a, *b),
                    Instruction::Load(k) => {
                        // A local's slot is one region above the spill slots (§2.4, W-245).
                        let (rd, store) = rt.write(*v, 2);
                        let off = devanagari(rt.frame.local_offset(*k));
                        rt.line(&format!("आहारः {rd}म् {SP}त् {off}न ।"));
                        if let Some(s) = store {
                            rt.line(&s);
                        }
                    }
                    // A module-level global read. The address comes from the
                    // global's own exported label — PC-relative, the same two-line
                    // shape `lower_string` uses for the string pool — and then one
                    // load from it. Two steps and not one because the label names
                    // STORAGE, not a value: a global is a word in memory that
                    // another module may have written.
                    Instruction::LoadGlobal(g) => {
                        let label = global_label(names, *g)?;
                        let (rd, store) = rt.write(*v, 2);
                        let t = temp(0);
                        rt.line(&format!("स्थानसापेक्षयोगः {t}म् {label}ॱउपरिन ।"));
                        rt.line(&format!("योगः {t}म् {t}न {label}ॱअधःन ।"));
                        rt.line(&format!("आहारः {rd}म् {t}त् ०न ।"));
                        if let Some(s) = store {
                            rt.line(&s);
                        }
                    }
                    // A record field read: one word from a RUNTIME base.
                    // `आहारः` already takes a base register — the ISA side needed
                    // nothing new — so what was missing was above the emitter: an
                    // instruction whose base is a value rather than the stack
                    // pointer, and a layout to take the offset from.
                    Instruction::LoadField(b, off) => {
                        let base = rt.read(*b, 0, 0);
                        let (rd, store) = rt.write(*v, 2);
                        // `W-381` stage 4: the offset is an i64's bits (the length
                        // word at base − 8 is read at −8), signed in the line.
                        let off = devanagari(off.cast_signed());
                        rt.line(&format!("आहारः {rd}म् {base}त् {off}न ।"));
                        if let Some(s) = store {
                            rt.line(&s);
                        }
                    }
                    // A run's element: TWO lines where the field read needs one.
                    // `आहारः` takes a base register and a CONSTANT displacement,
                    // and an index's displacement is a value, so the addition
                    // cannot be folded into the load. `ir.t1` has already scaled
                    // the index by the element width, so `o` arrives as a byte
                    // count and nothing is multiplied here.
                    //
                    // THE RESULT REGISTER IS ITS OWN SCRATCH, and safe rather than
                    // lucky: both operands are READ before it is written, so the
                    // add's destination cannot clobber an operand still to be read.
                    // `W-294` — THE LOAD IS PICKED BY THE ELEMENT'S WIDTH, in octets.
                    //
                    // EIGHT EMITS THE BARE `आहारः`, CHARACTER FOR CHARACTER AS
                    // BEFORE. `आहारःॱअ६४` would assemble to the same instruction and
                    // to a DIFFERENT source line, and `measure_corpus_twin_emit`
                    // compares TEXT — so spelling the common case explicitly would
                    // rewrite every image in the corpus to prove a point about
                    // symmetry. Only the widths this lowering used to refuse take a
                    // suffix, which is what keeps the blast radius equal to the work.
                    Instruction::LoadIndex(b, o, w) => {
                        let base = rt.read(*b, 0, 0);
                        let off = rt.read(*o, 1, 0);
                        let (rd, store) = rt.write(*v, 2);
                        let load = match w {
                            1 => "आहारःॱअ८",
                            2 => "आहारःॱअ१६",
                            4 => "आहारःॱअ३२",
                            // `W-381` stage 3: an UNSIGNED two- or four-octet
                            // element (`ir.t1` adds 16 to its width), `lhu`/`lwu`.
                            18 => "अचिह्नाहारःॱन१६",
                            20 => "अचिह्नाहारःॱन३२",
                            _ => "आहारः",
                        };
                        // करण TWICE — `न` and not `त्`. A register-register add
                        // takes two करण; अपादान is the LOAD's base role and is
                        // filled exactly once, so `{base}त् {off}त्` assembles to
                        // "`स्थिर०` and `स्थिर२` both claim अपादान". Measured from
                        // the assembler's own refusal, not read off the grammar.
                        rt.line(&format!("योगः {rd}म् {base}न {off}न ।"));
                        rt.line(&format!("{load} {rd}म् {rd}त् ०न ।"));
                        // An octet is unsigned: `आहारःॱअ८` is the signed byte load and the
                        // assembler has no unsigned form, so mask to the low eight bits — the
                        // .t1 emitter writes the same line (2026-09-13, the self-image's entry).
                        if *w == 1 {
                            rt.line(&format!("युक्तम् {rd}म् {rd}न २५५न ।"));
                        }
                        if let Some(s) = store {
                            rt.line(&s);
                        }
                    }
                    // `V-005`: THE STORE IS THE VALUE'S FILE'S STORE. A float
                    // value goes out with `प्लवनिधानम्` (`fsd`); an integer —
                    // including the ० a `भवति ०` writes into a `प६४` local — with
                    // `निधानम्`, so its bits are what the next `LoadFloat` reads.
                    // For an integer value this is the old line character for
                    // character.
                    Instruction::Store(k, x) => {
                        let (class, _) = rt.location(*x);
                        let src = rt.read(*x, 0, 0);
                        let off = devanagari(rt.frame.local_offset(*k));
                        let store = store_mnemonic(class);
                        rt.line(&format!("{store} {SP}य् {off}न {src}न ।"));
                    }
                    // `V-005`: a `प६४` local's read — `Load` with the float load.
                    Instruction::LoadFloat(k) => {
                        let (rd, store) = rt.write(*v, 2);
                        let off = devanagari(rt.frame.local_offset(*k));
                        let load = load_mnemonic(RegClass::Float);
                        rt.line(&format!("{load} {rd}म् {SP}त् {off}न ।"));
                        if let Some(s) = store {
                            rt.line(&s);
                        }
                    }
                    Instruction::Float(op, args) => lower_float(&mut rt, *v, *op, args),
                    // `V-008` part 2 — the strip-mined loop, in place.
                    Instruction::Vector(op, args) => lower_vector(&mut rt, *v, *op, args),
                    // `W-283`, the ruled storage model. This is `LoadGlobal`'s first
                    // two lines with the third removed — the address is the RESULT
                    // here rather than a step on the way to one.
                    //
                    // IT LANDS IN `rd`, NOT IN THE SCRATCH `temp(0)`. That is the
                    // whole difference and the reason the kind exists: a value the
                    // register allocator knows about outlives the instruction that
                    // formed it, so one address can serve a load and a store, or two
                    // stores, instead of being rebuilt per use.
                    Instruction::AddrOfGlobal(g) => {
                        let label = global_label(names, *g)?;
                        let (rd, store) = rt.write(*v, 2);
                        rt.line(&format!("स्थानसापेक्षयोगः {rd}म् {label}ॱउपरिन ।"));
                        rt.line(&format!("योगः {rd}म् {rd}न {label}ॱअधःन ।"));
                        if let Some(s) = store {
                            rt.line(&s);
                        }
                    }
                    // ONE line, because the address arrived built. `त्` — अपादान,
                    // the place read FROM.
                    Instruction::LoadAt(a) => {
                        let addr = rt.read(*a, 0, 0);
                        let (rd, store) = rt.write(*v, 2);
                        rt.line(&format!("आहारः {rd}म् {addr}त् ०न ।"));
                        if let Some(s) = store {
                            rt.line(&s);
                        }
                    }
                    // `V-008`: a `प६४` slot's read — the same line with the float
                    // load into the value's float register, from the same word.
                    Instruction::LoadAtFloat(a) => {
                        let addr = rt.read(*a, 0, 0);
                        let (rd, store) = rt.write(*v, 2);
                        let load = load_mnemonic(RegClass::Float);
                        rt.line(&format!("{load} {rd}म् {addr}त् ०न ।"));
                        if let Some(s) = store {
                            rt.line(&s);
                        }
                    }
                    // `य्` — अधिकरण, the place written TO, and NOT the load's `त्`.
                    // Writing `{addr}त्` here assembles to "`स्थिर०` and `स्थिर२`
                    // both claim अपादान", the same refusal `LoadIndex` above records
                    // for its add. The ISA assigns operands by role, so a store is
                    // not a load with the arrow reversed.
                    //
                    // NO RESULT VALUE, hence no `rt.write` — which is exactly why
                    // this kind is invisible to DCE unless `is_side_effecting` names
                    // it. See `a_store_at_survives_dead_code_elimination`.
                    //
                    // `W-306c` — THE STORE IS PICKED BY THE ELEMENT'S WIDTH, in
                    // octets, exactly as `LoadIndex` above picks its load.
                    //
                    // EIGHT EMITS THE BARE `निधानम्`, CHARACTER FOR CHARACTER AS
                    // BEFORE, for the same reason the load side keeps its bare
                    // `आहारः`: `निधानम्ॱअ६४` would assemble to the same
                    // instruction and to a DIFFERENT source line, and
                    // `measure_corpus_twin_emit` compares TEXT, so spelling the
                    // common case would rewrite every image in the corpus to make
                    // a point about symmetry.
                    //
                    // AND THE ONE PLACE THIS DIFFERS FROM THE LOAD: there is no
                    // `_ =>` arm. A width the table does not name REFUSES. The
                    // load's fall-through answers the bare `आहारः` and reads a
                    // word into a register — wrong in one register; the same
                    // fall-through here writes EIGHT octets where the IR asked for
                    // three and corrupts the five past the field. No mask follows
                    // a narrow store either, and none is wanted: `निधानम्ॱअ८`
                    // writes the low eight bits of the source and leaves the rest
                    // of the register alone, which is the asymmetry with
                    // `आहारःॱअ८`'s sign extension — a store has no destination
                    // register to extend into.
                    Instruction::StoreAt(a, x, w) => {
                        // `V-008`: A FLOAT IS STORED WITH THE FLOAT STORE, as a
                        // whole word (`verify` refused any other width). The
                        // integer arms below are untouched, character for
                        // character.
                        let float = rt.location(*x).0 == RegClass::Float;
                        let store_op = match w {
                            8 if float => store_mnemonic(RegClass::Float),
                            1 => "निधानम्ॱअ८",
                            2 => "निधानम्ॱअ१६",
                            4 => "निधानम्ॱअ३२",
                            8 => "निधानम्",
                            _ => {
                                return Err(Refusal::StoreWidthUnnamed {
                                    function: label.clone(),
                                    bytes: *w,
                                });
                            }
                        };
                        let addr = rt.read(*a, 0, 0);
                        let src = rt.read(*x, 1, 0);
                        rt.line(&format!("{store_op} {addr}य् ०न {src}न ।"));
                    }
                    // `W-284` — A RECORD ALLOCATION: bump the cursor, answer the
                    // OLD value. The storage every other kind in this group
                    // addresses.
                    //
                    // EIGHT LINES BECAUSE THE CURSOR HOLDS AN OFFSET, NOT AN
                    // ADDRESS — a data word cannot carry a label's address without
                    // a relocation the assembler has not got, so the region's base
                    // is computed PC-relatively at each allocation and added. Three
                    // of the eight are that recomputation and would vanish the day
                    // the assembler grows a relocation.
                    //
                    // NO BOUNDS CHECK, AND THAT IS A STATED GAP RATHER THAN AN
                    // OVERSIGHT: a bump past the region's end must REFUSE, and
                    // refusing needs a comparison and a trap this arm does not
                    // emit. Until it does, an overflow runs off the end of `ॱरिक्त`
                    // and the failure is a BAD ACCESS rather than a wrap — loud,
                    // but not the named refusal it should be. The wrap is the one
                    // that would corrupt records in silence, and it is the one that
                    // cannot happen here.
                    Instruction::AllocRecord(size) => {
                        let (rd, store) = rt.write(*v, 3);
                        let c = temp(0);
                        let o = temp(1);
                        let n = temp(2);
                        let sz = devanagari(i64::try_from(*size).unwrap_or(i64::MAX));
                        rt.line(&format!("स्थानसापेक्षयोगः {c}म् {RECORD_CURSOR}ॱउपरिन ।"));
                        rt.line(&format!("योगः {c}म् {c}न {RECORD_CURSOR}ॱअधःन ।"));
                        rt.line(&format!("आहारः {o}म् {c}त् ०न ।"));
                        rt.line(&format!("योगः {n}म् {o}न {sz}न ।"));
                        rt.line(&format!("निधानम् {c}य् ०न {n}न ।"));
                        rt.line(&format!("स्थानसापेक्षयोगः {c}म् {RECORD_REGION}ॱउपरिन ।"));
                        rt.line(&format!("योगः {c}म् {c}न {RECORD_REGION}ॱअधःन ।"));
                        rt.line(&format!("योगः {rd}म् {c}न {o}न ।"));
                        if let Some(s) = store {
                            rt.line(&s);
                        }
                    }
                    // ONE line — `LoadField` above with the load removed, exactly as
                    // `AddrOfGlobal` is `LoadGlobal` with its load removed. The
                    // offset is a constant, so `योगः` takes it as an immediate and
                    // no scratch register is needed.
                    //
                    // `न` AND NOT `त्`: this is an addition, not a load, so the base
                    // is करण. The `त्` here would assemble — an add takes two करण
                    // and would report both operands claiming अपादान — which is the
                    // refusal `LoadIndex` records two arms up.
                    Instruction::AddrOfField(b, off) => {
                        let base = rt.read(*b, 0, 0);
                        let (rd, store) = rt.write(*v, 2);
                        let off = devanagari(i64::try_from(*off).unwrap_or(i64::MAX));
                        rt.line(&format!("योगः {rd}म् {base}न {off}न ।"));
                        if let Some(s) = store {
                            rt.line(&s);
                        }
                    }
                    // ONE line for the same reason, and the index arrives ALREADY
                    // SCALED — `ir.t1` multiplies by the element width before
                    // building this, exactly as it does for `LoadIndex`. Scaling
                    // here as well would multiply twice and address past the end of
                    // every record but the first; the two arms must agree about
                    // which side scales, and the answer is the builder's.
                    Instruction::AddrOfIndex(b, i) => {
                        let base = rt.read(*b, 0, 0);
                        let idx = rt.read(*i, 1, 0);
                        let (rd, store) = rt.write(*v, 2);
                        rt.line(&format!("योगः {rd}म् {base}न {idx}न ।"));
                        if let Some(s) = store {
                            rt.line(&s);
                        }
                    }
                    Instruction::Param(i) | Instruction::ParamFloat(i) => {
                        let (rd, store) = rt.write(*v, 2);
                        match param_slots.get(i) {
                            Some(AbiLocation::FloatReg(f)) => rt.line(&format!(
                                "{FLOAT_MOVE} {rd}म् {}न {}न ।",
                                float_arg(*f),
                                float_arg(*f)
                            )),
                            Some(AbiLocation::Stack(off)) => {
                                // The caller left it just above this frame (§2.5).
                                let load = if matches!(inst, Instruction::ParamFloat(_)) {
                                    load_mnemonic(RegClass::Float)
                                } else {
                                    load_mnemonic(RegClass::Int)
                                };
                                let off = devanagari(rt.frame.bytes + *off as i64);
                                rt.line(&format!("{load} {rd}म् {SP}त् {off}न ।"));
                            }
                            Some(AbiLocation::IntReg(n)) => {
                                rt.line(&format!("योगः {rd}म् {}न ०न ।", arg(usize::from(*n))));
                            }
                            None => rt.line(&format!("योगः {rd}म् {}न ०न ।", arg(*i))),
                        }
                        if let Some(s) = store {
                            rt.line(&s);
                        }
                    }
                    Instruction::Call(callee, args) => {
                        emit_call(&mut rt, *v, *callee, args, names, false)?
                    }
                    Instruction::CallFloat(callee, args) => {
                        emit_call(&mut rt, *v, *callee, args, names, true)?
                    }
                }
            }
            emit_terminator(&mut rt, block, next, fused, relax)?;
        }
        rt.epilogue();

        match check_branch_ranges(func, &label, &rt.out, relax) {
            Ok(()) => break rt.out,
            // The only refusal a second pass can answer, and it is answered
            // ONCE: `check_branch_ranges` does not measure conditionals in a
            // relaxed routine, because a relaxed one branches eight bytes.
            Err(Refusal::BranchOutOfRange { .. }) if !relax => {
                // `W-332` — THE CENSUS IS TAKEN HERE AND NOWHERE ELSE, because
                // this arm IS the relaxation: it is the one edge on which a
                // routine stops being near-emitted, it is reached at most once
                // per routine (the guard is `!relax`), and the pass it starts
                // cannot fail back to `false`. Counting the PASSES instead
                // would count two for every relaxed routine and one for every
                // other, which is a count of routines wearing a count of
                // relaxations. Nor at the `relax` READ below, which happens on
                // both passes of a relaxed routine and would double it the
                // other way.
                relaxed.push(label.clone());
                relax = true;
            }
            Err(other) => return Err(other),
        }
    };
    Ok(out)
}

/// A conditional reaches ±4 KiB; the emitter refuses a routine whose text puts
/// a `CondBranch` further than that from its target rather than trusting the
/// routine to be short (§2.5). Measured on the emitted lines, four bytes each.
///
/// **EACH BRANCH IS MEASURED AT ITS OWN ADDRESS, AND `W-306` IS WHY IT WAS NOT.**
/// The line was found by `lines.iter().position(|l| is_conditional(l) && l.ends_with(&operand))`
/// — the FIRST conditional in the WHOLE routine naming that target — while the
/// refusal it raised named `from: id`. So when TWO blocks branch conditionally to
/// one target, the second was measured at the first's address: a near branch at
/// the top of a routine made every later branch to the same join read as near,
/// and a far one passed the guard to be refused by the assembler (or, worse, to
/// encode a wrapped B-type immediate). Two blocks branching to one join is the
/// ordinary shape of `यदि` inside `यावत्`, not a corner: the guard was blind
/// on the most common control flow the language has.
///
/// The branch line is now attributed to the block that WROTE it, by walking the
/// text once and tracking which block each line belongs to — a label line opens
/// its block, and the conditional seen while that block is current is the one its
/// `CondBranch` emitted. Exactly one conditional line per block can exist:
/// [`lower_cond_branch`] writes one, and a `तुलना` read as a VALUE lowers to
/// `न्यूनम्`/`अविषमम्`-class lines that are not branch words. The `.t1` twin
/// records the same two addresses as it writes them — `यन्त्रशाखास्थानकोश` now
/// keyed by the BRANCHING block and no longer by its target — so the twins agree
/// on which address a refusal is measured from and not merely on its kind.
///
/// **`relax` TURNS THE MEASUREMENT OFF, AND THAT IS NOT THE GUARD GOING BLIND.**
/// A relaxed routine's conditionals do not carry block labels at all — each one
/// carries its own block's `…अतिक्रम`, two words below it — so measuring them
/// against `block_at` would measure a distance neither branch has. Eight bytes
/// is the relaxed distance, it is a constant of the form [`lower_cond_branch`]
/// writes, and it is inside ±4 KiB for every routine that can exist. What is
/// NOT measured in the relaxed mode is the conditional; the J-type
/// `लङ्घनम्` the relaxation leans on IS, in BOTH modes, and that is the
/// second half of this routine. It reaches ±1 MiB, and until `W-306` closed it
/// a routine past that was accepted here and wrapped downstream — the
/// relaxation had merely moved the unmeasured wrap one instruction along.
///
/// **THE J-TYPE IS MEASURED BY LABEL, NOT BY BLOCK, AND THAT IS NOT LAZINESS.**
/// Three sites write `लङ्घनम् शून्यःम् <t>य्`: the `Branch` terminator, the
/// `Return` that must reach the shared epilogue, and the relaxation's own far
/// jump. The second targets [`exit_label`], which no `BlockId` spells — so a
/// block-keyed reading would silently skip the one jump a long routine is most
/// likely to stretch. A target this text declares no label line for is SKIPPED
/// and not refused: `लङ्घनम् पुनःस्थानम्म् <routine>य्` is a CALL, its distance is the
/// linker's, and `emit_call` writes it with `पुनःस्थानम्` and never with `शून्यः` — which is
/// why the prefix names the register and not just the word.
fn check_branch_ranges(
    func: &Function,
    label: &str,
    text: &str,
    relax: bool,
) -> Result<(), Refusal> {
    let lines: Vec<&str> = text.lines().collect();
    let address_of = |line: usize| -> i64 {
        // Neither a label nor a directive (`॥ … ॥`, the exported-label line) is an
        // instruction word.
        4 * lines[..line]
            .iter()
            .filter(|l| !l.ends_with("ॱॱ") && !l.starts_with('॥'))
            .count() as i64
    };

    // ── THE J-TYPE, MEASURED IN BOTH MODES (`W-306`) ──
    //
    // A label line is its own whole text (`<name>ॱॱ`), which is also how
    // `address_of` knows not to count one as a word. Built from the TEXT and not
    // from `func.blocks`, so `exit_label`'s line — a target with no `BlockId` —
    // is in the map like any other.
    let line_of_label: HashMap<&str, usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(n, l)| l.strip_suffix("ॱॱ").map(|name| (name, n)))
        .collect();
    let jump_prefix = format!("लङ्घनम् {ZERO}म् ");
    for (n, l) in lines.iter().enumerate() {
        let Some(target) = l
            .strip_prefix(&jump_prefix)
            .and_then(|r| r.strip_suffix("य् ।"))
        else {
            continue;
        };
        // A target this text declares no label for is a CALL or the startup's
        // own spelling — not this routine's distance to measure.
        let Some(&target_line) = line_of_label.get(target) else {
            continue;
        };
        let bytes = address_of(target_line) - address_of(n);
        if !(-(1 << 20)..(1 << 20)).contains(&bytes) {
            return Err(Refusal::JumpOutOfRange {
                function: label.to_string(),
                target: target.to_string(),
                bytes,
            });
        }
    }

    // ── THE B-TYPE, WHICH THE RELAXED FORM DOES NOT CARRY ──
    if relax {
        return Ok(());
    }
    // Any of the six conditions, or the synthesized `विषमलङ्घनम्` (W-245).
    let is_conditional = |l: &str| {
        l.starts_with("विषमलङ्घनम् ")
            || [CmpOp::Eq, CmpOp::Lt, CmpOp::Ge, CmpOp::Ltu, CmpOp::Geu]
                .iter()
                .any(|op| l.starts_with(&format!("{} ", branch_word(*op))))
    };
    let mut ids: Vec<BlockId> = func.blocks.keys().copied().collect();
    ids.sort_by_key(|b| b.0);
    // The label line each block opens with, matched as WHOLE text and never as a
    // substring: `…पर्व१` is a prefix of `…पर्व१०`, and a `contains` would open the
    // wrong block in a routine with ten labelled blocks (found by the T1 twin,
    // `W-236`, which records the address as it writes the line).
    let label_of: HashMap<String, BlockId> = ids
        .iter()
        .map(|b| (format!("{}ॱॱ", block_label(label, *b)), *b))
        .collect();
    let mut block_at: HashMap<BlockId, usize> = HashMap::new();
    let mut branch_at: HashMap<BlockId, usize> = HashMap::new();
    // The entry block carries no label of its own unless a back-edge targets it,
    // so the prologue's lines already belong to it (§2.2).
    let mut current = Some(func.entry_block);
    for (n, l) in lines.iter().enumerate() {
        if let Some(b) = label_of.get(*l) {
            block_at.insert(*b, n);
            current = Some(*b);
        } else if is_conditional(l)
            && !l.contains(&format!("{label}{VECTOR_LABEL_INFIX}"))
            && let Some(b) = current
        {
            branch_at.entry(b).or_insert(n);
        }
    }
    for id in ids {
        let Some(Terminator::CondBranch(_, t, _)) = func.blocks[&id].terminator else {
            continue;
        };
        let (Some(&from_line), Some(&target_line)) = (branch_at.get(&id), block_at.get(&t)) else {
            continue;
        };
        let bytes = address_of(target_line) - address_of(from_line);
        if !(-4096..4096).contains(&bytes) {
            return Err(Refusal::BranchOutOfRange {
                function: label.to_string(),
                from: id,
                target: t,
                bytes,
            });
        }
    }
    Ok(())
}

/// The startup stub (§2.6): `sp` from `auipc` — not `lui`, whose sign-extension
/// of bit 31 would put `0x8000_0000+` in the upper half — the call to the entry
/// with no arguments, and the entry's result as the finisher status; then the
/// 64 KiB stack it addresses, reserved in `.bss` (`V-009` part (i-b2)), as its
/// `.t1` twin `यन्त्रारम्भोत्सर्जनम्` has always carried it.
#[must_use]
pub fn emit_startup(entry: Option<&str>) -> String {
    emit_startup_with_records(entry, false)
}

/// [`emit_startup`], with the record region emitted only when the image needs
/// it. `W-284`.
///
/// THE REGION IS NOT UNCONDITIONAL, AND THE FIXTURES ARE WHY: a program that
/// allocates nothing must produce THE SAME OCTETS it produced before this row.
/// `॥ स्थानम् ॥` in `ॱदत्त` materialises bytes — the stack was how an image
/// came to 65536 until `V-009` (i-b2) moved it to `.bss` — so an unconditional
/// cursor moves every image by one word and an unconditional region by
/// 512 MiB of address space. A FEATURE THAT COSTS A PROGRAM THAT DOES NOT
/// USE IT IS CHARGED TO THE WRONG ACCOUNT.
///
/// The `records` flag comes from [`module_allocates`], and its `.t1` twin
/// `यन्त्ररचनामस्ति` must answer the same question the same way: the two halves
/// diverge here on a SECTION DIRECTIVE — `॥ कोष्ठकम् ॱरिक्त ॥` against
/// `॥ कोष्ठकम् ॱदत्त ॥` — which is a whole-image difference from one bool.
#[must_use]
pub fn emit_startup_with_records(entry: Option<&str>, records: bool) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "॥ वैश्विकम् यन्त्रारम्भ ॥");
    let _ = writeln!(s, "यन्त्रारम्भॱॱ");
    let _ = writeln!(s, "स्थानसापेक्षयोगः {SP}म् स्तूपान्तःॱउपरिन ।");
    let _ = writeln!(s, "योगः {SP}म् {SP}न स्तूपान्तःॱअधःन ।");
    // `V-009` part (i-b): THE FLOAT AND VECTOR UNITS ON, before the entry runs.
    // `sstatus.FS` (bits 14:13) and `sstatus.VS` (bits 10:9) ← Initial (01):
    // `lui` 0x2000, `addi` 0x200, `csrrs x0, sstatus, t5`. A real hart resets
    // both `Off`, and every F, D or V instruction under `Off` is illegal — so
    // until this, no float or vector image ran on QEMU (true since `V-005`).
    //
    // `sstatus` (0x100), NOT `mstatus`: its FS and VS are views of the same
    // bits, it is writable from M-mode (QEMU `-bios none`) and from S-mode
    // (yantra, which has no `mstatus` at all; an SBI firmware), so ONE word
    // runs on every venue. INITIAL, NOT DIRTY: Initial already enables the unit,
    // and the hart moves it to Dirty on the first write; Dirty would claim state
    // nobody has written. AFTER the two `sp` words, never before: the W-376
    // thread host decodes exactly those two at `e_entry` and starts thread
    // k ≥ 1 at `e_entry + 8`, so every thread passes through these three.
    let _ = writeln!(s, "उपरिभारः {}म् ०षोड्२न ।", temp(5));
    let _ = writeln!(s, "योगः {}म् {}न ०षोड्२००न ।", temp(5), temp(5));
    let _ = writeln!(s, "नियन्त्रकविकल्पः {ZERO}म् ०षोड्१००त् {}न ।", temp(5));
    // `V-009` (i-b2), the coordinator's finding 6: THE CANARY. The stack's
    // bottom word holds its own address; `यन्त्रसमाप्ति` checks it before the
    // finisher store. DETECTION AT EXIT, not prevention. ARMED ONLY WHEN THE
    // SLOT IS ZERO, branchless (round 2): `.bss` is zeroed at load, so thread 0
    // writes `t5`; a W-376 thread k ≥ 1 entering at `e_entry + 8` stores the
    // slot back UNCHANGED, intact or smashed, so an earlier overflow survives.
    // slot += (slot == 0) × t5.
    let _ = writeln!(s, "स्थानसापेक्षयोगः {}म् स्तूपःॱउपरिन ।", temp(5));
    let _ = writeln!(s, "योगः {}म् {}न स्तूपःॱअधःन ।", temp(5), temp(5));
    let _ = writeln!(s, "आहारः {}म् {}त् ०न ।", temp(4), temp(5));
    let _ = writeln!(s, "अचिह्नन्यूनम् {}म् {}न १न ।", temp(3), temp(4));
    let _ = writeln!(s, "गुणनम् {}म् {}न {}न ।", temp(3), temp(3), temp(5));
    let _ = writeln!(s, "योगः {}म् {}न {}न ।", temp(4), temp(4), temp(3));
    let _ = writeln!(s, "निधानम् {}य् ०न {}न ।", temp(5), temp(4));
    if let Some(label) = entry {
        let _ = writeln!(s, "लङ्घनम् {RA}म् {label}य् ।");
        let _ = writeln!(s, "समलङ्घनम् {}न {ZERO}त् यन्त्रसफलय् ।", arg(0));
        let _ = writeln!(s, "वामसरणम् {}म् {}न १६न ।", arg(0), arg(0));
        let _ = writeln!(s, "उपरिभारः {}म् ०षोड्३न ।", temp(5));
        let _ = writeln!(s, "योगः {}म् {}न ०षोड्३३३न ।", temp(5), temp(5));
        let _ = writeln!(s, "विकल्पः {}म् {}न {}न ।", arg(0), arg(0), temp(5));
        let _ = writeln!(s, "लङ्घनम् {ZERO}म् यन्त्रसमाप्तिय् ।");
    }
    let _ = writeln!(s, "यन्त्रसफलॱॱ");
    let _ = writeln!(s, "उपरिभारः {}म् ०षोड्५न ।", arg(0));
    let _ = writeln!(s, "योगः {}म् {}न ०षोड्५५५न ।", arg(0), arg(0));
    // The canary intact: on to the store. Changed: the status is `0x353B`
    // (finisher word `0x353B_3333`, QEMU exit 59) instead of the program's.
    // The store sits under `यन्त्रचक्र` so no new label is needed; repeating
    // it is harmless, the first one halts the machine.
    let _ = writeln!(s, "यन्त्रसमाप्तिॱॱ");
    let _ = writeln!(s, "स्थानसापेक्षयोगः {}म् स्तूपःॱउपरिन ।", temp(5));
    let _ = writeln!(s, "योगः {}म् {}न स्तूपःॱअधःन ।", temp(5), temp(5));
    let _ = writeln!(s, "आहारः {}म् {}त् ०न ।", temp(4), temp(5));
    let _ = writeln!(s, "समलङ्घनम् {}न {}त् यन्त्रचक्रय् ।", temp(4), temp(5));
    let _ = writeln!(s, "उपरिभारः {}म् ०षोड्३५३आ३न ।", arg(0));
    let _ = writeln!(s, "योगः {}म् {}न ०षोड्३३३न ।", arg(0), arg(0));
    let _ = writeln!(s, "यन्त्रचक्रॱॱ");
    let _ = writeln!(s, "उपरिभारः {}म् {FINISHER_HI}न ।", temp(4));
    let _ = writeln!(s, "निधानम्ॱअ३२ {}य् ०न {}न ।", temp(4), arg(0));
    let _ = writeln!(s, "लङ्घनम् {ZERO}म् यन्त्रचक्रय् ।");
    // `V-009` part (i-b2): THE STACK IS RESERVED IN `.bss`, NOT STORED IN THE
    // FILE (owner ruling 2026-10-05). `॥ स्थानम् ॥` in `ॱदत्त` materialised
    // 65,536 zero octets in every image file; in `ॱरिक्त` it is `p_memsz` the
    // file does not carry, which every loader zero-fills.
    //
    // FIRST IN THE STARTUP'S `.bss`, AND SO FIRST IN THE IMAGE'S: the startup is
    // linked first, and the record region below comes AFTER the stack. So the
    // stack is the 64 KiB right after the file-backed data — inside any RAM
    // budget that holds the file (`W-363`'s `Span::FileBacked`) — and the heap
    // begins at `स्तूपान्तः`, growing up while the stack grows down. After the
    // 512 MiB region it would sit 512 MiB up, and a file-backed budget would
    // halt on the first push. The linker starts `.bss` on sixteen
    // (`samyojana`), so `स्तूपान्तः` — the `sp` the two words above load — keeps
    // the ABI's 16-octet alignment it had at the start of the page-aligned data.
    let _ = writeln!(s, "॥ कोष्ठकम् ॱरिक्त ॥");
    let _ = writeln!(s, "॥ संरेखः १६ ॥");
    // A 4 KiB GUARD below the stack (finding 6): an overflow lands here, not
    // in the last module's data, and is reported by the canary at exit.
    let _ = writeln!(s, "॥ स्थानम् ४०९६ ॥");
    let _ = writeln!(s, "स्तूपःॱॱ");
    let _ = writeln!(s, "॥ स्थानम् {} ॥", devanagari(STACK_BYTES as i64));
    let _ = writeln!(s, "स्तूपान्तःॱॱ");
    // `W-284` — THE RECORD REGION AND ITS CURSOR, EMITTED EXACTLY ONCE, AND
    // ONLY WHEN THE IMAGE ALLOCATES.
    //
    // THEY LIVE HERE AND NOT IN `emit_data` BECAUSE THAT RUNS PER OBJECT: an
    // image is one startup plus every module, so a `॥ वैश्विकम् ॥` region in the
    // module data section is defined N times and the linker refuses —
    // "`रचनासूचकः` is defined by more than one object", measured on `artha.t1`
    // and `ir.t1`, which moved from stopping at RUN to stopping at LINK.
    //
    // MAKING THEM LOCAL INSTEAD WOULD ALSO LINK AND IS WRONG: each module would
    // take its own 4 MB, and twenty modules is 80 MB against `DEFAULT_RAM`'s
    // 20 MB. ONE SHARED BUMP IS THE POINT — a per-module region is not a
    // smaller version of this feature, it is a different and broken one, since
    // a record allocated in one module and read in another would be addressed
    // through two different bases.
    if records {
        let _ = writeln!(s, "॥ कोष्ठकम् ॱरिक्त ॥");
        let _ = writeln!(s, "॥ संरेखः १६ ॥");
        let _ = writeln!(s, "॥ वैश्विकम् {RECORD_REGION} ॥");
        let _ = writeln!(s, "{RECORD_REGION}ॱॱ");
        let _ = writeln!(s, "॥ स्थानम् {} ॥", devanagari(RECORD_REGION_OCTETS));
        let _ = writeln!(s, "॥ कोष्ठकम् ॱदत्त ॥");
        let _ = writeln!(s, "॥ संरेखः १६ ॥");
        let _ = writeln!(s, "॥ वैश्विकम् {RECORD_CURSOR} ॥");
        let _ = writeln!(s, "{RECORD_CURSOR}ॱॱ");
        let _ = writeln!(s, "॥ अष्टाष्टकाः {} ॥", hex64(0));
        let _ = writeln!(s, "॥ कोष्ठकम् ॱपाठ ॥");
    }
    s
}

/// THE STARTUP OBJECT (`W-243`, research/25 §5 R6): the stub and the stack it
/// addresses (in `.bss`, first, then the record region when there is one), as
/// one T0 text of its own — one per image, linked FIRST so that
/// `e_entry`, the image's first instruction, is `यन्त्रारम्भ`. A module's text
/// carries no startup any more: an image is one startup object plus every
/// module object, and the entry's label reaches the module through its export.
#[must_use]
pub fn emit_startup_object(entry: Option<&str>) -> String {
    emit_startup_object_with_records(entry, false)
}

/// Does this module allocate a record? `W-284`.
///
/// The startup carries the region, but only an image that needs one should pay
/// for it — so the caller that has the module asks this and passes the answer.
/// Its `.t1` twin is `यन्त्ररचनामस्ति`, which the emitter sets as it writes an
/// allocation rather than scanning for one; the two arrive at the same bool by
/// different routes, which is the only reason both are worth having.
/// ir.t1's `रचनासूचकसंज्ञा`: the record cursor addressed from the IR by run
/// growth and slices. An address-of it needs the region as much as an
/// allocation does.
pub const RECORD_CURSOR_SYMBOL: SymbolId = SymbolId(10_000_003);
/// ir.t1's `रचनाक्षेत्रसंज्ञा`: the region itself (the cursor is an offset into it).
pub const RECORD_REGION_SYMBOL: SymbolId = SymbolId(10_000_004);

#[must_use]
pub fn module_allocates(m: &Module) -> bool {
    m.functions.iter().any(|f| {
        f.blocks.iter().any(|(_, b)| {
            b.insts.iter().any(|(_, i)| {
                matches!(i, Instruction::AllocRecord(_))
                    || matches!(i, Instruction::AddrOfGlobal(g) if *g == RECORD_CURSOR_SYMBOL || *g == RECORD_REGION_SYMBOL)
            })
        })
    })
}

/// [`emit_startup_object`], with the record region when the image allocates.
#[must_use]
pub fn emit_startup_object_with_records(entry: Option<&str>, records: bool) -> String {
    // The stack moved into [`emit_startup_with_records`] with `V-009` (i-b2),
    // AHEAD of the record region in `.bss`, as `यन्त्रारम्भोत्सर्जनम्` always
    // carried it; the object is that text.
    emit_startup_with_records(entry, records)
}

/// The data a module carries (§2.6): its constant pool. The stack reservation
/// went with the startup into [`emit_startup_object`] (`W-243`), because the
/// stub is what addresses it and an image has one stack, not one per module.
#[must_use]
pub fn emit_data(pool: &[i64], strings: &[Vec<u8>], globals: &[(String, i64, i64)]) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "॥ कोष्ठकम् ॱदत्त ॥");
    let _ = writeln!(s, "॥ संरेखः १६ ॥");
    let _ = writeln!(s, "ध्रुवकोशःॱॱ");
    for (k, c) in pool.iter().enumerate() {
        let _ = writeln!(s, "ध्रुव{}ॱॱ", devanagari(k as i64));
        let _ = writeln!(s, "॥ अष्टाष्टकाः {} ॥", hex64(*c as u64));
    }
    // `W-278` — THE GLOBALS, WITH THE QUADS AND BEFORE THE OCTETS, for the
    // alignment reason the next comment gives: each is one `अष्टाष्टकाः`, so it
    // must sit in the region that keeps `संरेखः १६`. Each carries its own
    // EXPORTED label, because another module's read resolves to it by name.
    // `W-293` — A RUN'S GLOBAL GETS A POINTER AND ITS STORAGE.
    //
    // **THIS BRANCH IS A MIRROR AND DECIDES NOTHING.** `यन्त्रदत्तोत्सर्जनम्`
    // (`yantrotsarjana.t1:2104`) emits exactly these lines from the same arena, and
    // the twin octet test is what holds the two identical. The condition is the
    // arena's answer — `मध्यरूप` computed it at `ir.t1:2793` — so a reader looking
    // for where a global's storage is DECIDED should look there and not here.
    //
    // The pointer word is a NAME, which `ADR-0013` makes the ADDRESS of what it
    // names (`parse.rs:903-915` pushes it onto `Datum.addresses` as a relocation).
    // So the base is set at LINK time and needs no code to run — which is why a
    // module whose own entry never runs still gets a usable global, and why the
    // three placements that put an allocation in a routine's entry block are
    // recorded dead in `tools/sassembly-model.py --placements`.
    //
    // The storage sits in `ॱरिक्त` at `संरेखः १६`, the section and alignment the
    // record region takes: space in memory, absent from the file, so 163 corpus
    // globals at 1 KiB each cost no image size.
    for (label, init, octets) in globals {
        let _ = writeln!(s, "॥ वैश्विकम् {label} ॥");
        let _ = writeln!(s, "{label}ॱॱ");
        if *octets > 8 {
            let _ = writeln!(s, "॥ अष्टाष्टकाः {label}भण्डार ॥");
            let _ = writeln!(s, "॥ कोष्ठकम् ॱरिक्त ॥");
            let _ = writeln!(s, "॥ संरेखः १६ ॥");
            // `W-len` — THE RUN'S LENGTH HEADER, ONE WORD AT `भण्डार वियोगः ८`.
            // The storage is reached through a POINTER held in the symbol's own
            // data word, so the header goes in front of the STORAGE and the
            // pointer does not move: every existing access is byte-identical
            // and no relocation gains an addend.
            //
            // ZERO FOR FREE — `ॱरिक्त` is space in memory, absent from the
            // file, so the header reads ० at load with no store. That is what a
            // high-water mark starts at. The LOCAL allocation must store its ०
            // because a bump allocator hands out whatever the cursor left.
            //
            // The eight octets of padding keep `भण्डार` १६-aligned; the header
            // alone would leave the storage at १६k+८. Nothing in the language
            // needs more than ८ — widths are १, २, ४, ८ — but this section's
            // alignment is the record region's and is not this unit's to move.
            let _ = writeln!(s, "॥ स्थानम् ८ ॥");
            let _ = writeln!(s, "{label}भण्डारशीर्षॱॱ");
            let _ = writeln!(s, "॥ स्थानम् ८ ॥");
            let _ = writeln!(s, "{label}भण्डारॱॱ");
            let _ = writeln!(s, "॥ स्थानम् {} ॥", devanagari(*octets));
            let _ = writeln!(s, "॥ कोष्ठकम् ॱदत्त ॥");
            let _ = writeln!(s, "॥ संरेखः १६ ॥");
        } else {
            let _ = writeln!(s, "॥ अष्टाष्टकाः {} ॥", hex64(*init as u64));
        }
    }
    // NO RECORD REGION HERE, AND THE FIRST VERSION OF `W-284` PUT ONE HERE.
    // This routine runs PER OBJECT — an image is one startup plus every module
    // — so a `॥ वैश्विकम् ॥` region written here is defined N times and the
    // linker refuses it by name. The region and its cursor live in
    // `emit_startup_with_records`, the one object every image has exactly one
    // of, and only when the image allocates.
    //
    // IT ALSO DIVERGED FROM THE `.t1` TWIN SILENTLY, and the twin test is what
    // said so: an unconditional copy here made the two halves disagree on a
    // SECTION DIRECTIVE for EVERY program in the corpus, allocating or not.

    // `W-254` — THE STRING POOL, AFTER THE QUADS AND NOT AMONG THEM. A blob is
    // any number of octets, so placing one between two `अष्टाष्टकाः` would put
    // every later quad at an address that is no longer a multiple of eight. The
    // quads keep the region's `संरेखः १६`; the octets take what follows it.
    if !strings.is_empty() {
        let _ = writeln!(s, "पाठकोशःॱॱ");
        for (k, b) in strings.iter().enumerate() {
            // `W-len` — A LITERAL'S LENGTH HEADER, ONE WORD AT `पाठक वियोगः ८`,
            // and it is the literal's TRUE length rather than ०: a literal is
            // never written, so its high-water mark is its octet count and is
            // known here at compile time.
            //
            // `संरेखः ८` FIRST, AND THE MARGIN ABOVE IS WHY IT IS SAFE HERE. The
            // pool packs blobs of arbitrary length end to end, so a quad placed
            // among them would be UNALIGNED and `दैर्घ्य`'s load would fault.
            // Padding between literals costs nothing — it is padding among the
            // QUADS that the pool was moved after them to avoid.
            //
            // OUTSIDE THE EMPTY-LITERAL `continue` BELOW, DELIBERATELY. An empty
            // literal still takes a label and still answers `दैर्घ्य`, and the
            // answer must be ०; a header emitted only for non-empty literals
            // would leave `पाठक वियोगः ८` reading whatever precedes it.
            let _ = writeln!(s, "॥ संरेखः ८ ॥");
            // capacity ० (2026-09-13): a literal's first indexed store copies it.
            let _ = writeln!(s, "॥ अष्टाष्टकाः ० ॥");
            let _ = writeln!(s, "पाठ{}शीर्षॱॱ", devanagari(k as i64));
            let _ = writeln!(s, "॥ अष्टाष्टकाः {} ॥", devanagari(b.len() as i64));
            let _ = writeln!(s, "पाठ{}ॱॱ", devanagari(k as i64));
            // `अष्टकाः` and not `आस्की`, though both are live: the octets are
            // arbitrary — `artha.t1:679` types a literal as unsigned octets,
            // not as characters — and a directive taking a quoted payload would
            // need an escape for every octet that ends the quote. Numbers need
            // no escaping and say what is there; the text form below is used
            // only where the octets need none either.
            //
            // AN EMPTY LITERAL GETS ITS LABEL AND NO DIRECTIVE. `अष्टकाः` takes
            // `1+` operands (spec/directives.tsv), so `॥ अष्टकाः ॥` is not a
            // thing that can be written; the label alone is the right answer
            // anyway, since the address of zero octets is where the next thing
            // starts and an empty slice has nothing to point at.
            if b.is_empty() {
                continue;
            }
            // `SAS-011` (c): ONE WORD THAT READS BACK AS ITSELF IS WRITTEN AS
            // TEXT — `parse::string_payload`, the owner's narrow rule — and
            // every other run as numerals. Measured on the corpus: 97.55% of
            // the data octets take the text form, and the assembler's numeral
            // path cost ~4,300 interpreter steps per octet. The `.t1` twin's
            // octet walk (`यन्त्रपाठरूपयोग्यम्`) must answer the same, and
            // `sas011_text_literal_twin.rs` holds the two to the same pool.
            //
            // ADR-0044 D3: a run of Devanagari-8 octets that reads back as
            // itself travels as its LETTERS, `वर्णाष्टकम् … इति`, and the
            // assembler packs each one back into its octet. `उक्तम्` is asked
            // first and `devanagari8::payload` refuses anything UTF-8-shaped,
            // so a run that took the text or numeral form before still does
            // (D6). The `.t1` twin is `यन्त्रवर्णाष्टकयोग्यम्`.
            let operands = match crate::parse::string_payload(b) {
                Some(text) => format!("उक्तम् {text} इति"),
                None => match crate::devanagari8::payload(b) {
                    Some(letters) => {
                        format!("{} {letters} इति", crate::devanagari8::OPEN)
                    }
                    None => b
                        .iter()
                        .map(|o| devanagari(i64::from(*o)))
                        .collect::<Vec<_>>()
                        .join(" "),
                },
            };
            let _ = writeln!(s, "॥ अष्टकाः {operands} ॥");
        }
    }
    s
}

/// The whole module as one T0 text: every routine, each under its exported
/// label, then the data (§2.6). Since `W-243` the startup is NOT here — it is
/// [`emit_startup_object`], one per image — so a module object holds nothing
/// but its own routines and pool, and every module of an image can be linked
/// behind one startup. The entry is still checked here (a routine with
/// parameters cannot be called by the stub) because this is where the IR is.
/// Registers are allocated here with [`ALLOCATABLE`] `स्थिर`s per routine.
///
/// # Errors
/// The first [`Refusal`], by name; no text is returned for a module the emitter
/// cannot lower in full.
pub fn emit_module(module: &Module) -> Result<String, Refusal> {
    emit_module_and_relaxations(module).map(|(text, _)| text)
}

/// **WHY A CENSUS AND NOT A BYTE-DIFF, MEASURED.** On `b8ba2717` the two images
/// differ first at offset 96 — `p_filesz` — and their common suffix is 4,725 of
/// 1,400,602 octets. A byte-diff CANNOT answer how many routines relaxed,
/// because relaxing one shifts every address after it and the comparison stops
/// being about relaxation three words in.
///
/// **A RELAXATION IS A SILENT EVENT AND WAS THE ONLY ONE THE EMITTER HAD.**
/// Every other decision it takes shows up in the text it writes and is thus
/// visible to the twin comparison and to the fixpoint. Relaxation shows up too —
/// three words where one stood — but nothing DISTINGUISHES those three from
/// three the routine would have written anyway, so the image is a lower bound on
/// the question and not an answer to it. `W-330` spent two days on that gap: the
/// inline-capacity test refused at 20 sources and passed at 1, and the mechanism
/// was load-dependent relaxation, which nothing counted.
///
/// [`emit_module`] with `W-332`'s relaxation census: the labels of the routines
/// this module emitted in their RELAXED form, in emission order, so
/// `.len()` is the module's relaxation count.
///
/// **IT IS A SECOND ENTRY POINT AND NOT A CHANGED SIGNATURE.** `emit_module` is
/// called from 53 sites across the test crates and every one of them wants the
/// text alone; widening the return type would have edited all of them to
/// discard a value, and the census would then be paid for in churn rather than
/// in the one place that reads it.
///
/// **WHAT AN EMPTY VEC MEANS, AND WHAT IT DOES NOT.** Empty is *no routine of
/// this module needed relaxing* — every conditional reached its target within
/// ±4 KiB. It is NOT *this emitter cannot relax*: the two are indistinguishable
/// in the emitted octets, which is the whole reason `W-332` exists, and they
/// are told apart by a test that relaxes something on purpose next to one that
/// relaxes nothing.
///
/// # Errors
/// The first [`Refusal`], by name; no text and no census for a module the
/// emitter cannot lower in full.
pub fn emit_module_and_relaxations(module: &Module) -> Result<(String, Vec<String>), Refusal> {
    check_labels(module)?;
    let entry_label = match module.entry {
        Some(sym) => {
            let label = routine_label(&module.names, sym)?;
            if let Some(f) = module.functions.iter().find(|f| f.name == sym) {
                let params = f
                    .blocks
                    .values()
                    .flat_map(|b| b.insts.iter())
                    .filter(|(_, i)| {
                        matches!(i, Instruction::Param(_) | Instruction::ParamFloat(_))
                    })
                    .count();
                if params > 0 {
                    return Err(Refusal::EntryTakesParameters {
                        function: label,
                        params,
                    });
                }
            }
            Some(label)
        }
        None => None,
    };
    let mut pool = Vec::new();
    let mut strings: Vec<Vec<u8>> = Vec::new();
    let mut relaxed: Vec<String> = Vec::new();
    let mut routines = String::new();
    // `V-004` PART 5 — TWO SCANS, ONE PER REGISTER FILE, SHARING ONE CLASSIFIER.
    // The classifier answers `Int` for every value because the T1 IR has no
    // float producer: `Instruction` has no float kind, and the classifier that
    // answers `Float` is `V-005`'s. So the integer scan is `allocate_registers`
    // exactly (`regalloc.rs`'s control test) and the float scan returns an
    // EMPTY map — no value, no spill, no saved register — and the emitted text
    // is the one-file text. The second scan is run anyway, so that the day a
    // classifier answers `Float` the only edit is this closure.
    //
    // `V-005`: THE CLASSIFIER ANSWERS `Float` NOW, and it is the only edit the
    // margin above promised: a float op whose result is a float, and a `प६४`
    // local's read. The port's twin is `यन्त्रप्लववर्गः` (the rule itself is
    // `utsarjana.t1`'s `आज्ञावर्गः`, which the port's float scan calls), by the
    // same two rules, the first through `ir.t1`'s `प्लवफलम्`. The port skips
    // the float scan for a routine with no float instruction: its map is
    // empty either way.
    let classify = |_: ValueId, inst: &Instruction| value_class(inst);
    for func in &module.functions {
        // `V-009` (ii): a synthesised matrix kernel routine is fixed text.
        if let Some(text) = matrix_kernel(func, &module.names)? {
            routines.push_str(&text);
            continue;
        }
        let alloc = allocate_registers_for(func, ALLOCATABLE, RegClass::Int, &classify);
        let float = allocate_registers_for(
            func,
            allocatable(RegClass::Float),
            RegClass::Float,
            &classify,
        );
        routines.push_str(&emit_function(
            func,
            &module.names,
            &alloc,
            &float,
            &mut pool,
            &mut strings,
            &mut relaxed,
        )?);
    }
    // The entry's label is the startup object's business (`emit_startup_object`);
    // it was computed above for the parameter check.
    let _ = entry_label;
    let mut out = routines;
    out.push_str(&emit_data(&pool, &strings, &module.globals));
    Ok((out, relaxed))
}

// --- the fixture R1 is accepted on, shared with R2's agreement test and R3 ---------------

/// `f(n) = if n then f(n−1) + n else 0` as hand IR — `Param`, `CondBranch`,
/// `Sub`, `Call`, `Add`, `Return` — with an entry `मुख्य` that calls `f(5)`;
/// research/25 §5 R1's acceptance is that this runs on `yantra` to status 15.
#[must_use]
pub fn fixture_recursive_sum() -> Module {
    let f = SymbolId(1);
    let main = SymbolId(2);
    let mut names = Names::new();
    names.insert(f, ("परीक्षा".into(), "योगफलम्".into()));
    names.insert(main, ("परीक्षा".into(), "मुख्य".into()));

    // f: entry(0) p = Param(0); CondBranch(p, then(1), else(2))
    //    then(1): one = 1; m = p - one; r = f(m); s = r + p; Return s
    //    else(2): z = 0; Return z
    let (p, one, m, r, s, z) = (
        ValueId(0),
        ValueId(1),
        ValueId(2),
        ValueId(3),
        ValueId(4),
        ValueId(5),
    );
    let (entry, then, els) = (BlockId(0), BlockId(1), BlockId(2));
    let mut blocks = HashMap::new();
    blocks.insert(
        entry,
        Block {
            id: entry,
            insts: vec![(p, Instruction::Param(0))],
            terminator: Some(Terminator::CondBranch(p, then, els)),
        },
    );
    blocks.insert(
        then,
        Block {
            id: then,
            insts: vec![
                (one, Instruction::ConstInt(1)),
                (m, Instruction::Sub(p, one)),
                (r, Instruction::Call(f, vec![m])),
                (s, Instruction::Add(r, p)),
            ],
            terminator: Some(Terminator::Return(Some(s))),
        },
    );
    blocks.insert(
        els,
        Block {
            id: els,
            insts: vec![(z, Instruction::ConstInt(0))],
            terminator: Some(Terminator::Return(Some(z))),
        },
    );
    let recursive = Function {
        name: f,
        blocks,
        entry_block: entry,
    };

    // main: five = 5; result = f(five); Return result
    let (five, result) = (ValueId(10), ValueId(11));
    let mut blocks = HashMap::new();
    blocks.insert(
        entry,
        Block {
            id: entry,
            insts: vec![
                (five, Instruction::ConstInt(5)),
                (result, Instruction::Call(f, vec![five])),
            ],
            terminator: Some(Terminator::Return(Some(result))),
        },
    );
    let entry_fn = Function {
        name: main,
        blocks,
        entry_block: entry,
    };

    Module {
        globals: Vec::new(),
        name: "परीक्षा".into(),
        functions: vec![recursive, entry_fn],
        names,
        entry: Some(main),
    }
}

/// `W-245`'s fixture, shared with the T1 twin's `यन्त्ररूपदृष्टान्तः`: every
/// form the row added, in one routine `रूपाणि(n)`, and an entry `मुख्य` that
/// calls it with 5.
///
/// ```text
/// रूपाणि(n):
///   entry(0): p = Param(0); Store(0, p); Branch(cond)
///   cond(1):  i = Load(0); lim = 3; c = Cmp(Lt, i, lim); CondBranch(c, body, exit)   — FUSED
///   body(2):  one = 1; j = Load(0); k = j + one; Store(0, k); Branch(cond)
///   exit(3):  x = Load(0); two = 2;
///             the eight operators over (x, two), each read once by a chain of adds;
///             the six compares AS VALUES over (x, two), summed in;
///             Return(sum)
/// ```
///
/// It assembles and its twins agree octet for octet; it is not run on
/// `yantra` because it reaches `गुणनम्` (no M extension there) — the corpus
/// routine of `crates/yantra/tests/paradigm_encode.rs` is the one that runs.
#[must_use]
pub fn fixture_forms() -> Module {
    let forms = SymbolId(1);
    let main = SymbolId(2);
    let mut names = Names::new();
    names.insert(forms, ("परीक्षा".into(), "रूपाणि".into()));
    names.insert(main, ("परीक्षा".into(), "मुख्य".into()));
    let (entry, cond, body, exit) = (BlockId(0), BlockId(1), BlockId(2), BlockId(3));
    let mut next = 0usize;
    let mut v = || {
        next += 1;
        ValueId(next - 1)
    };
    let mut blocks = HashMap::new();
    let p = v();
    let s0 = v();
    blocks.insert(
        entry,
        Block {
            id: entry,
            insts: vec![(p, Instruction::Param(0)), (s0, Instruction::Store(0, p))],
            terminator: Some(Terminator::Branch(cond)),
        },
    );
    let (i, lim, c) = (v(), v(), v());
    blocks.insert(
        cond,
        Block {
            id: cond,
            insts: vec![
                (i, Instruction::Load(0)),
                (lim, Instruction::ConstInt(3)),
                (c, Instruction::Cmp(CmpOp::Lt, i, lim)),
            ],
            terminator: Some(Terminator::CondBranch(c, body, exit)),
        },
    );
    let (one, j, k, s1) = (v(), v(), v(), v());
    blocks.insert(
        body,
        Block {
            id: body,
            insts: vec![
                (one, Instruction::ConstInt(1)),
                (j, Instruction::Load(0)),
                (k, Instruction::Add(j, one)),
                (s1, Instruction::Store(0, k)),
            ],
            terminator: Some(Terminator::Branch(cond)),
        },
    );
    let (x, two) = (v(), v());
    let mut insts = vec![(x, Instruction::Load(0)), (two, Instruction::ConstInt(2))];
    let mut acc = x;
    let ops: Vec<Instruction> = vec![
        Instruction::Mul(x, two),
        Instruction::Div(x, two),
        Instruction::Rem(x, two),
        Instruction::Shl(x, two),
        Instruction::Shr(x, two),
        Instruction::And(x, two),
        Instruction::Or(x, two),
        Instruction::Xor(x, two),
        Instruction::Cmp(CmpOp::Eq, x, two),
        Instruction::Cmp(CmpOp::Ne, x, two),
        Instruction::Cmp(CmpOp::Lt, x, two),
        Instruction::Cmp(CmpOp::Ge, x, two),
        Instruction::Cmp(CmpOp::Ltu, x, two),
        Instruction::Cmp(CmpOp::Geu, x, two),
    ];
    for op in ops {
        let r = v();
        insts.push((r, op));
        let sum = v();
        insts.push((sum, Instruction::Add(acc, r)));
        acc = sum;
    }
    // The all-ones constant, −1 here and `१८४४६७४४०७३७०९५५१६१५` (2⁶⁴−1) in the
    // T1 twin's fixture — the interpreter's number is wider than a register, and
    // both twins must write `ऋण१` (found by the twin census on the corpus).
    let (mask, masked) = (v(), v());
    insts.push((mask, Instruction::ConstInt(-1)));
    insts.push((masked, Instruction::And(acc, mask)));
    blocks.insert(
        exit,
        Block {
            id: exit,
            insts,
            terminator: Some(Terminator::Return(Some(masked))),
        },
    );
    let routine = Function {
        name: forms,
        blocks,
        entry_block: entry,
    };
    let (five, result) = (v(), v());
    let mut blocks = HashMap::new();
    blocks.insert(
        entry,
        Block {
            id: entry,
            insts: vec![
                (five, Instruction::ConstInt(5)),
                (result, Instruction::Call(forms, vec![five])),
            ],
            terminator: Some(Terminator::Return(Some(result))),
        },
    );
    let entry_fn = Function {
        name: main,
        blocks,
        entry_block: entry,
    };
    Module {
        globals: Vec::new(),
        name: "परीक्षा".into(),
        functions: vec![routine, entry_fn],
        names,
        entry: Some(main),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::Target;
    use crate::nidana::Language;
    use crate::t1::regalloc::allocate_registers;
    use crate::vishlesana::{decode_at, reassemble};

    /// `V-008` part 2 — THE VECTOR WORDS ARE THE TABLES'. `vsetvli`'s mnemonic
    /// is `spec/encodings-riscv64.tsv`'s, every vector register `lower_vector`
    /// names is `spec/registers-riscv64.tsv`'s at the hardware number it means,
    /// and `vtype` 219 is e64/m8/ta/ma by its fields.
    #[test]
    fn the_vector_words_are_the_tables() {
        let enc = include_str!("../../../../spec/encodings-riscv64.tsv");
        let row = enc
            .lines()
            .map(|l| l.split('\t').collect::<Vec<_>>())
            .find(|f| f.first() == Some(&"vsetvli"))
            .expect("the table has vsetvli");
        assert_eq!(row[2], VECTOR_SET_LENGTH);
        for n in [0u32, 8, 16, 24, 31] {
            let name = vector_register(i64::from(n));
            assert_eq!(
                crate::encode::register(&name),
                Some((n, false)),
                "{name} is v{n}"
            );
            assert!(
                crate::encode::is_vector_register(&name),
                "{name} is a vector register"
            );
        }
        let (vlmul, vsew, vta, vma) = (3, 3 << 3, 1 << 6, 1 << 7);
        assert_eq!(VECTOR_TYPE_E64_M8, vlmul | vsew | vta | vma);
    }

    /// `V-009` (ii): THE COMPACT TABLE DECODES TO THE READABLE ROWS, and uses
    /// only the alphabet's letters.
    #[test]
    fn the_compact_kernel_table_is_the_readable_one() {
        assert_eq!(matrix_kernel_rows(), MATRIX_KERNEL.to_vec());
        for c in MATRIX_KERNEL_TABLE
            .chars()
            .chain(MATRIX_KERNEL_OPERANDS.chars())
        {
            let cp = c as u32;
            assert!(
                (0x905..=0x939).contains(&cp) || (0x972..=0x97c).contains(&cp),
                "{c:?} is outside the alphabet"
            );
        }
    }

    /// `V-009` (ii): THE KERNEL'S MNEMONICS ARE THE TABLE'S WORDS for the
    /// RISC-V forms its rows mean, code by code, and `vlse64.v` — the strided
    /// load the kernel writes as `आहारः` with a stride register — is in the
    /// table under the same word.
    #[test]
    fn the_matrix_kernel_words_are_the_tables() {
        let enc = include_str!("../../../../spec/encodings-riscv64.tsv");
        let word = |insn: &str| {
            enc.lines()
                .map(|l| l.split('\t').collect::<Vec<_>>())
                .find(|f| f.first() == Some(&insn))
                .map(|f| f[2].to_string())
                .unwrap_or_else(|| panic!("the table has {insn}"))
        };
        let forms = [
            "add", "mul", "or", "srli", "slli", "sub", "ld", "sd", "beq", "bne", "jal", "jalr",
            "vsetvli", "vfmul.vv", "vfadd.vv",
        ];
        for (k, insn) in forms.iter().enumerate() {
            assert_eq!(
                MATRIX_KERNEL_VERBS[k],
                word(insn),
                "code {} is {insn}",
                k + 1
            );
        }
        assert_eq!(word("vlse64.v"), word("ld"));
        assert_eq!(word("vle64.v"), word("ld"));
        assert_eq!(word("vse64.v"), word("sd"));
        // every row's verb is a known code, and no row names v0
        for row in MATRIX_KERNEL {
            assert!(usize::from(row[0]) <= forms.len());
            if row[0] > 0 {
                for o in &row[1..] {
                    assert!(
                        *o == 0 || *o >= 1280 || o % 256 != 32,
                        "v0 is named: {row:?}"
                    );
                }
            }
        }
    }

    /// `V-004` PART 3 — THE INTEGER ARM IS THE OLD SPELLING, UNCHANGED.
    ///
    /// The seam is only safe if adding it moved nothing: every emitted
    /// `स्थिर<n>` in the corpus comes through `class_register_name` now, so the
    /// `Int` arm must agree with `register_name` on every number the scan can
    /// produce, not merely on a sample.
    ///
    /// AND IT MUST REFUSE WHERE `register_name` PANICS. The old function
    /// asserted `n < ALLOCATABLE`; the new one cannot assert, because it is
    /// also the float path, so the refusal became a `None` and the panic moved
    /// to `Routine::reg_name`. Twelve and 255 are both past the end.
    #[test]
    fn v004_the_int_arm_is_byte_for_byte_the_old_spelling() {
        assert_eq!(allocatable(RegClass::Int), ALLOCATABLE);
        for n in 0..ALLOCATABLE {
            assert_eq!(
                class_register_name(RegClass::Int, n).unwrap(),
                register_name(n),
                "the Int arm moved at {n}"
            );
        }
        assert_eq!(class_register_name(RegClass::Int, ALLOCATABLE), None);
        assert_eq!(class_register_name(RegClass::Int, 255), None);
    }

    /// `V-004` PART 3 — THE FLOAT ARM HANDS OUT THE CALLEE-SAVED FILE.
    ///
    /// The three mappings are from the RISC-V calling convention and are NOT
    /// derivable from the table under test: `fs0` is `f8`, `fs2` is `f18`,
    /// `fs11` is `f27`. They are the reason the file is not contiguous and the
    /// reason a role index used raw would be a wrong register rather than a
    /// failure.
    ///
    /// REFUSED: a thirteenth. Saturating would give `प्लव२७` and wrapping
    /// `प्लव८`, and both of those assemble — which is why `None` is the only
    /// answer that is not a lie.
    #[test]
    fn v004_the_float_arm_spells_the_callee_saved_file() {
        assert_eq!(
            ALLOCATABLE_FLOAT_ROLE,
            FloatRole::Saved,
            "§2.3: across a Call"
        );
        assert_eq!(allocatable(RegClass::Float), 12, "fs0-fs11");
        assert_eq!(class_register_name(RegClass::Float, 0).unwrap(), "प्लव८");
        assert_eq!(class_register_name(RegClass::Float, 2).unwrap(), "प्लव१८");
        assert_eq!(class_register_name(RegClass::Float, 11).unwrap(), "प्लव२७");
        assert_eq!(
            class_register_name(RegClass::Float, 12),
            None,
            "there is no fs12"
        );
        assert_eq!(class_register_name(RegClass::Float, 255), None);
    }

    /// A map in one file: `regs` held in registers, `spills` slots of its own.
    /// Built by hand rather than by a scan, because the point under test is what
    /// the FRAME does with two of them and a scan would only produce one.
    fn one_file_map(regs: &[u8], spills: usize) -> AllocationMap {
        AllocationMap {
            locations: regs
                .iter()
                .enumerate()
                .map(|(i, r)| (ValueId(i), Location::Register(*r)))
                .collect(),
            num_spills: spills,
        }
    }

    /// `V-004` PART 4 — A FLOAT FILE NOTHING LANDED IN COSTS NOTHING.
    ///
    /// Every routine the tree can build is this case: `Instruction` has no float
    /// kind, so the second scan's map is `None` or empty. The two must give the
    /// SAME frame as each other and the same one the single-file layout gave, or
    /// the seam has moved 870 corpus routines' stack offsets.
    #[test]
    fn v004_a_frame_with_no_float_file_is_the_frame_it_was() {
        let int = one_file_map(&[0, 1], 2);
        let none = frame_layout(&int, None, 3);
        // 2 spills + 3 locals = ४० bytes of slots, two saved स्थिरs at ४० and ४८,
        // पुनःस्थानम् above them: ५६ + ८ = ६४, already a multiple of 16.
        assert_eq!(none.num_spills, 2);
        assert_eq!(none.num_float_spills, 0);
        assert_eq!(none.num_locals, 3);
        assert_eq!(
            none.saved,
            vec![(RegClass::Int, 0, 40), (RegClass::Int, 1, 48)]
        );
        assert_eq!(none.ra_offset, 56);
        assert_eq!(none.bytes, 64);
        assert_eq!(none.spill_offset(RegClass::Int, 0), 0);
        assert_eq!(
            none.local_offset(0),
            16,
            "locals sit above the spills, W-245"
        );

        let empty_float = one_file_map(&[], 0);
        assert_eq!(
            frame_layout(&int, Some(&empty_float), 3),
            none,
            "AN EMPTY SECOND FILE IS NOT A SECOND REGION. A layout that reserved \
             a word, or a saved entry, for a file that holds nothing would shift \
             every local and every saved offset in the corpus by eight"
        );
    }

    /// `V-004` PART 4 — THE TWO FILES' SAVED SLOTS ARE DISJOINT, AND THE SAME
    /// ALLOCATOR NUMBER IN THE TWO FILES IS TWO REGISTERS.
    ///
    /// This is the case the old `Vec<(u8, i64)>` could not represent: `0` means
    /// `स्थिर०` in the integer file and `प्लव८` in the float file (PART 3), so a
    /// frame keyed on the number alone would either save one of them twice or
    /// hand both the same eight bytes. Here both files hold `0` and `2`.
    #[test]
    fn v004_the_two_files_saved_slots_are_disjoint_and_the_int_file_comes_first() {
        let int = one_file_map(&[0, 2], 0);
        let float = one_file_map(&[0, 2], 0);
        let f = frame_layout(&int, Some(&float), 0);
        assert_eq!(
            f.saved,
            vec![
                (RegClass::Int, 0, 0),
                (RegClass::Int, 2, 8),
                (RegClass::Float, 0, 16),
                (RegClass::Float, 2, 24),
            ],
            "four registers, four slots, integers first"
        );
        let offsets: HashSet<i64> = f.saved.iter().map(|(_, _, o)| *o).collect();
        assert_eq!(offsets.len(), 4, "no two saved registers share a slot");
        // ३२ raw bytes of saved registers + ८ for पुनःस्थानम् = ४०, rounded up to
        // ४८, so पुनःस्थानम् is at ४० and ३२ is the pad.
        assert_eq!(f.ra_offset, 40);
        assert_eq!(f.bytes, 48);
    }

    /// `V-004` PART 4 — THE FLOAT SPILL REGION IS A SECOND REGION AND NOT A
    /// CONTINUATION OF THE FIRST.
    ///
    /// `regalloc.rs:54` rules that "the spill numbering in each returned map
    /// starts at zero and counts only its own", so BOTH maps call their first
    /// spill `0`. The answer this test refuses is the one a single `num_spills`
    /// gives: `spill_offset(Int, 0) == spill_offset(Float, 0) == 0`, which
    /// assembles and silently stores an `f` value over an `x` value.
    #[test]
    fn v004_the_float_spill_region_does_not_alias_the_int_spill_region() {
        let int = one_file_map(&[0], 2);
        let float = one_file_map(&[0], 3);
        let f = frame_layout(&int, Some(&float), 1);
        assert_eq!((f.num_spills, f.num_float_spills, f.num_locals), (2, 3, 1));

        assert_eq!(f.spill_offset(RegClass::Int, 0), 0);
        assert_eq!(f.spill_offset(RegClass::Int, 1), 8);
        assert_eq!(f.spill_offset(RegClass::Float, 0), 16);
        assert_eq!(f.spill_offset(RegClass::Float, 2), 32);
        assert_ne!(
            f.spill_offset(RegClass::Int, 0),
            f.spill_offset(RegClass::Float, 0),
            "SLOT ० OF THE TWO FILES IS NOT ONE WORD"
        );
        assert_eq!(f.local_offset(0), 40, "the local lies above BOTH regions");

        // Every addressed word of the frame, once: 2 + 3 spills, 1 local, 2
        // saved, पुनःस्थानम्. A region that overlapped another would show up here
        // as a short set and nowhere else.
        let mut words: Vec<i64> = (0..2)
            .map(|k| f.spill_offset(RegClass::Int, k))
            .chain((0..3).map(|k| f.spill_offset(RegClass::Float, k)))
            .chain(std::iter::once(f.local_offset(0)))
            .chain(f.saved.iter().map(|(_, _, o)| *o))
            .chain(std::iter::once(f.ra_offset))
            .collect();
        words.sort_unstable();
        assert_eq!(words, vec![0, 8, 16, 24, 32, 40, 48, 56, 72]);
        assert!(words.iter().all(|w| *w < f.bytes), "all inside the frame");
        assert_eq!(f.bytes, 80, "७२ raw rounded up to ८०");
        assert!(
            !words.contains(&64),
            "६४ is the rounding pad and nobody's slot"
        );
    }

    /// `V-004` PART 4 — THE PROLOGUE AND EPILOGUE SPELL EACH FILE WITH ITS OWN
    /// MNEMONIC.
    ///
    /// `निधानम्` is `sd` and takes an `x` register; `प्लवनिधानम्` is `fsd`
    /// (`spec/mnemonics-riscv64.src.tsv:81-82`). Saving `प्लव१८` with `निधानम्`
    /// would not fail to assemble — it would assemble as `sd s2` and store the
    /// WRONG REGISTER, which is why this is checked on the emitted text and not
    /// on the frame.
    ///
    /// THE CHECK IS PER LINE AND NOT `contains`, because `प्लवनिधानम्` CONTAINS
    /// `निधानम्`: a `text.contains("निधानम्")` is satisfied by the float line and
    /// would pass a prologue that had no integer store in it at all.
    #[test]
    fn v004_the_prologue_and_epilogue_spell_each_file_with_its_own_mnemonic() {
        let s = SymbolId(1);
        let mut blocks = HashMap::new();
        blocks.extend([block(
            0,
            vec![(ValueId(0), Instruction::ConstInt(1))],
            Terminator::Return(Some(ValueId(0))),
        )]);
        let func = Function {
            name: s,
            blocks,
            entry_block: BlockId(0),
        };
        let alloc = one_file_map(&[0], 0);
        let float = one_file_map(&[2], 0);
        let frame = frame_layout(&alloc, Some(&float), 0);
        let mut rt = Routine {
            label: "परीक्षा".to_string(),
            func: &func,
            alloc: &alloc,
            float: &float,
            frame,
            out: String::new(),
        };
        rt.prologue();
        rt.epilogue();
        let text = rt.out;

        // The register each file's save names: स्थिर० is x8+0, प्लव१८ is fs2.
        assert_eq!(class_register_name(RegClass::Int, 0).unwrap(), "स्थिर०");
        assert_eq!(class_register_name(RegClass::Float, 2).unwrap(), "प्लव१८");

        // One store and one load per saved register, each with its file's verb
        // and its file's register — matched as whole lines.
        for want in [
            "निधानम् स्तूपसूचकःय् ०न स्थिर०न ।",
            "प्लवनिधानम् स्तूपसूचकःय् ८न प्लव१८न ।",
            "आहारः स्थिर०म् स्तूपसूचकःत् ०न ।",
            "प्लवाहारः प्लव१८म् स्तूपसूचकःत् ८न ।",
        ] {
            assert_eq!(
                text.lines().filter(|l| *l == want).count(),
                1,
                "exactly one `{want}`:\n{text}"
            );
        }

        // AND THE WRONG PAIRINGS ARE ABSENT. Each of these is a line a frame
        // that had forgotten the class would have emitted instead.
        for forbidden in [
            "निधानम् स्तूपसूचकःय् ८न प्लव१८न ।",
            "आहारः प्लव१८म् स्तूपसूचकःत् ८न ।",
            "प्लवनिधानम् स्तूपसूचकःय् ०न स्थिर०न ।",
            "प्लवाहारः स्थिर०म् स्तूपसूचकःत् ०न ।",
        ] {
            assert!(
                !text.lines().any(|l| l == forbidden),
                "`{forbidden}` names one file's verb and the other's register:\n{text}"
            );
        }

        // पुनःस्थानम् is an `x` register and keeps the integer verb whatever the
        // float file holds.
        assert!(
            text.lines().any(|l| l == "निधानम् स्तूपसूचकःय् २४न पुनःस्थानम्न ।"),
            "{text}"
        );
    }

    /// `V-004` PART 5 — A SPILLED FLOAT VALUE TRAVELS THROUGH THE FLOAT LOAD AND
    /// STORE, AT ITS FLOAT SPILL OFFSET, THROUGH A FLOAT SCRATCH.
    ///
    /// THE OWNER'S UNIT-LEVEL FALSIFIER (ruling 2026-10-04, option 1). Thirteen
    /// integer and thirteen float values are all live across a `Call`; each file
    /// has twelve callee-saved registers, so THE FLOAT FILE SPILLS EXACTLY ONE
    /// (the integer file spills two — the `Call`'s result is a fourteenth
    /// integer value). Integer slot ० is offset ०; the float spill is slot ० of
    /// the float region, which starts ABOVE the integer region
    /// (`Frame::spill_offset`), so the two slots ० are two words and the test
    /// can tell which file's verb wrote which word.
    ///
    /// THE CLASSIFIER IS THE TEST'S OWN, as in `regalloc.rs`'s tests: nothing in
    /// the tree answers `Float` yet, so this is the only way to reach the path.
    /// The text is not ASSEMBLED, and on purpose: the arithmetic kinds that read
    /// these values are integer instructions, and `योगः` over `प्लव` registers is
    /// not a program. What is under test is the SPILL TRAFFIC, which is lines.
    ///
    /// MEASURED, 2026-10-04 (a Linux x86-64 host, `--release`). The float slot is at
    /// १६ (two integer spills). RED FIRST: `read`/`write` as they were before
    /// PART 5 (integer verb, integer scratch) emit `निधानम् स्तूपसूचकःय् १६न
    /// क्षणिक२न ।` and fail the fsd-count assertion. MUTANT: `read` alone
    /// emitting `आहारः` for a float slot emits `आहारः प्लव०म् स्तूपसूचकःत् १६न ।`
    /// and fails the fld-count assertion — before the every-line check, which
    /// would also refuse that line.
    #[test]
    fn v004_a_spilled_float_value_uses_the_float_load_and_store_at_its_float_offset() {
        let f_sym = SymbolId(1);
        let callee = SymbolId(2);
        let ints: Vec<ValueId> = (0..13).map(ValueId).collect();
        let floats: Vec<ValueId> = (20..33).map(ValueId).collect();
        let mut insts: Vec<(ValueId, Instruction)> = Vec::new();
        for (i, v) in ints.iter().chain(floats.iter()).enumerate() {
            insts.push((*v, Instruction::ConstInt(i as i64 + 1)));
        }
        let called = ValueId(50);
        insts.push((called, Instruction::Call(callee, vec![])));
        // Every value is read AFTER the call, so all twenty-six are live across
        // it. A float operand's add is classified float too (below), so each
        // file's reads stay in that file.
        for (n, v) in ints.iter().chain(floats.iter()).enumerate() {
            insts.push((ValueId(60 + n), Instruction::Add(*v, *v)));
        }
        let mut blocks = HashMap::new();
        blocks.extend([block(0, insts, Terminator::Return(None))]);
        let func = Function {
            name: f_sym,
            blocks,
            entry_block: BlockId(0),
        };
        let float_set: HashSet<ValueId> = floats
            .iter()
            .copied()
            .chain((13..26).map(|n| ValueId(60 + n)))
            .collect();
        let classify = |v: ValueId, _: &Instruction| {
            if float_set.contains(&v) {
                RegClass::Float
            } else {
                RegClass::Int
            }
        };
        let int_map = allocate_registers_for(&func, ALLOCATABLE, RegClass::Int, &classify);
        let float_map = allocate_registers_for(
            &func,
            allocatable(RegClass::Float),
            RegClass::Float,
            &classify,
        );
        // The integer file spills TWO, not one, and that is the scan and not
        // the emitter: the `Call`'s own result is a fourteenth integer value
        // defined while twelve are held, and linear scan gives it a register by
        // spilling the longest-lived. What matters here is only that the
        // integer region is non-empty, so the float region starts ABOVE it.
        assert!(int_map.num_spills >= 1, "the integer file spills too");
        assert_eq!(float_map.num_spills, 1, "13 float values, 12 fs");
        let spilled: Vec<ValueId> = float_map
            .locations
            .iter()
            .filter(|(_, l)| matches!(l, Location::Spill(_)))
            .map(|(v, _)| *v)
            .collect();
        assert_eq!(spilled.len(), 1, "exactly one float value spills");
        assert!(
            floats.contains(&spilled[0]),
            "and it is one of the thirteen"
        );

        let mut names = Names::new();
        names.insert(f_sym, ("परीक्षा".into(), "प्लवपरीक्षा".into()));
        names.insert(callee, ("परीक्षा".into(), "योगफलम्".into()));
        let (mut pool, mut strings, mut relaxed) = (Vec::new(), Vec::new(), Vec::new());
        let text = emit_function(
            &func,
            &names,
            &int_map,
            &float_map,
            &mut pool,
            &mut strings,
            &mut relaxed,
        )
        .expect("emits");
        let frame = frame_layout(&int_map, Some(&float_map), 0);
        assert_eq!(frame.spill_offset(RegClass::Int, 0), 0);
        let fo_bytes = frame.spill_offset(RegClass::Float, 0);
        assert_eq!(
            fo_bytes,
            8 * int_map.num_spills as i64,
            "above the integer region"
        );
        let fo = devanagari(fo_bytes);
        let lines: Vec<&str> = text.lines().collect();

        // THE FLOAT SLOT: one store, from the float scratch `ft2` (`प्लव२` —
        // `ConstInt` writes through scratch 2), with `प्लवनिधानम्`.
        assert_eq!(
            lines
                .iter()
                .filter(|l| **l == format!("प्लवनिधानम् स्तूपसूचकःय् {fo}न प्लव२न ।"))
                .count(),
            1,
            "the spilled float value's definition stores it with fsd at {fo}:\n{text}"
        );
        // ...and at least one read of it, into a float scratch (`ft0`/`ft1`).
        let float_loads = lines
            .iter()
            .filter(|l| {
                **l == format!("प्लवाहारः प्लव०म् स्तूपसूचकःत् {fo}न ।")
                    || **l == format!("प्लवाहारः प्लव१म् स्तूपसूचकःत् {fo}न ।")
            })
            .count();
        assert!(float_loads >= 1, "the float slot is read with fld:\n{text}");

        // EVERY line that touches the float slot is float traffic, and NONE is the
        // integer verb. This is the check the mutant fails.
        for l in &lines {
            if l.contains(&format!("स्तूपसूचकःत् {fo}न ")) || l.contains(&format!("स्तूपसूचकःय् {fo}न "))
            {
                assert!(
                    l.starts_with("प्लवाहारः प्लव")
                        || l.starts_with(&format!("प्लवनिधानम् स्तूपसूचकःय् {fo}न प्लव")),
                    "offset {fo} is the float spill slot and `{l}` is not float traffic:\n{text}"
                );
            }
        }

        // THAT SLOT ONLY: the float verbs appear at the spill slot and at the
        // twelve saved `fs` registers' slots (prologue + epilogue), nowhere else.
        let saved_float: HashSet<String> = frame
            .saved
            .iter()
            .filter(|(c, _, _)| *c == RegClass::Float)
            .map(|(_, _, o)| devanagari(*o))
            .collect();
        assert_eq!(saved_float.len(), 12, "all twelve fs are used and saved");
        for l in lines
            .iter()
            .filter(|l| l.starts_with("प्लवाहारः") || l.starts_with("प्लवनिधानम्"))
        {
            let at_spill =
                l.contains(&format!("स्तूपसूचकःत् {fo}न ")) || l.contains(&format!("स्तूपसूचकःय् {fo}न "));
            let at_saved = saved_float.iter().any(|o| {
                l.contains(&format!("स्तूपसूचकःत् {o}न ")) || l.contains(&format!("स्तूपसूचकःय् {o}न "))
            });
            assert!(
                at_spill || at_saved,
                "`{l}` is a float load/store outside the float spill slot and the saved slots:\n{text}"
            );
        }
        assert_eq!(
            lines.iter().filter(|l| l.starts_with("प्लवनिधानम्")).count(),
            1 + 12,
            "one spill store and twelve saves:\n{text}"
        );

        // THE INTEGER SLOT is untouched by any of this: `निधानम्` at ० through
        // `क्षणिक२`, read back with `आहारः`.
        assert_eq!(
            lines
                .iter()
                .filter(|l| **l == "निधानम् स्तूपसूचकःय् ०न क्षणिक२न ।")
                .count(),
            1,
            "{text}"
        );
        assert!(
            lines.iter().any(|l| *l == "आहारः क्षणिक०म् स्तूपसूचकःत् ०न ।"
                || *l == "आहारः क्षणिक१म् स्तूपसूचकःत् ०न ।"),
            "{text}"
        );
    }

    /// `V-004` PART 3 — THE ASSEMBLER IS THE ORACLE, AND IT ANSWERS FOR BOTH
    /// FILES. Equality against a literal cannot catch a name that is spelled
    /// consistently and is not in the lexicon, which is exactly the failure
    /// `role_name` would be: `प्लवस्थिर२` is a correct diagnostic and does not
    /// assemble. So every name the seam can produce is resolved by
    /// `encode::register`, its CLASS BIT is checked (a float name that resolved
    /// to an integer register would pass a string test), and its hardware
    /// number is checked against the role table.
    ///
    /// REFUSED, and it is the point of the test: no `role_name` resolves.
    #[test]
    fn v004_every_name_the_seam_produces_assembles_and_no_role_name_does() {
        for n in 0..allocatable(RegClass::Int) {
            let name = class_register_name(RegClass::Int, n).unwrap();
            let (_, is_float) = crate::encode::register(&name)
                .unwrap_or_else(|| panic!("{name} is not in the lexicon"));
            assert!(!is_float, "{name} resolved to a float register");
        }
        for n in 0..allocatable(RegClass::Float) {
            let name = class_register_name(RegClass::Float, n).unwrap();
            let (num, is_float) = crate::encode::register(&name)
                .unwrap_or_else(|| panic!("{name} is not in the lexicon"));
            assert!(is_float, "{name} resolved to an integer register");
            assert_eq!(
                num,
                u32::from(ALLOCATABLE_FLOAT_ROLE.hardware(n).unwrap()),
                "{name} is not the hardware number the role names"
            );
        }
        for role in FloatRole::ALL {
            for n in 0..role.count() {
                let diagnostic = role.role_name(n).unwrap();
                assert_eq!(
                    crate::encode::register(&diagnostic),
                    None,
                    "{diagnostic} is a role name and must not assemble"
                );
            }
        }
    }

    /// `V-004` PART 3 — THE TWO FILES SHARE NO SPELLING, so a map handed to the
    /// wrong arm cannot emit text that happens to assemble. Twelve distinct
    /// names each, and no name in both.
    #[test]
    fn v004_the_two_files_share_no_spelling() {
        let ints: HashSet<String> = (0..allocatable(RegClass::Int))
            .map(|n| class_register_name(RegClass::Int, n).unwrap())
            .collect();
        let floats: HashSet<String> = (0..allocatable(RegClass::Float))
            .map(|n| class_register_name(RegClass::Float, n).unwrap())
            .collect();
        assert_eq!(ints.len(), 12, "twelve distinct स्थिर");
        assert_eq!(floats.len(), 12, "twelve distinct प्लव");
        assert!(ints.is_disjoint(&floats), "a name in both files");
    }

    /// A GLOBAL DECLARED IN ONE MODULE AND READ IN ANOTHER RESOLVES TO ONE
    /// OBJECT — the label the declarer EXPORTS is character-for-character the
    /// label the reader loads from.
    ///
    /// `W-278`. THIS IS THE PROPERTY THE WHOLE DESIGN RESTS ON, and the one
    /// that fails silently if it is wrong. `parse.t1` writes
    /// `वास्तुॱअभिव्यञ्जककोश` and `vastu.t1` reads it; if each module emitted its
    /// own copy of the storage, the write would land in one word and the read
    /// take another, and every shared global would answer its initialiser for
    /// ever. No crash, no red test elsewhere: just a number that never changes.
    ///
    /// The WRITE half is not lowered yet (`assign_name` is its own cause), so
    /// this pins what can be pinned today: the two labels agree, the declarer
    /// exports exactly one definition, and the reader defines none.
    #[test]
    fn a_global_read_in_another_module_names_the_declarers_one_object() {
        let g = SymbolId(7);
        let reader = SymbolId(8);
        let mut names = Names::new();
        names.insert(g, ("वास्तु".into(), "अभिव्यञ्जककोश".into()));
        names.insert(reader, ("व्याकर".into(), "पठिता".into()));
        let label = routine_label(&names, g).expect("the global is named");

        let declarer = Module {
            globals: vec![(label.clone(), 0, 8)],
            name: "वास्तु".into(),
            functions: Vec::new(),
            names: names.clone(),
            entry: None,
        };
        let declarer_text = emit_module(&declarer).expect("the declarer emits");
        assert!(
            declarer_text.contains(&format!("॥ वैश्विकम् {label} ॥")),
            "the declaring module EXPORTS the label, or no other module could \
             reach it:\n{declarer_text}"
        );
        assert_eq!(
            declarer_text.matches(&format!("\n{label}ॱॱ")).count(),
            1,
            "exactly ONE definition of the storage:\n{declarer_text}"
        );

        let v = ValueId(0);
        let mut blocks = HashMap::new();
        blocks.insert(
            BlockId(0),
            Block {
                id: BlockId(0),
                insts: vec![(v, Instruction::LoadGlobal(g))],
                terminator: Some(Terminator::Return(Some(v))),
            },
        );
        let readers = Module {
            globals: Vec::new(),
            name: "व्याकर".into(),
            functions: vec![Function {
                name: reader,
                blocks,
                entry_block: BlockId(0),
            }],
            names,
            entry: Some(reader),
        };
        let reader_text = emit_module(&readers).expect("the reader emits");
        assert!(
            reader_text.contains(&format!("{label}ॱउपरिन")),
            "the reader materialises the DECLARER's label:\n{reader_text}"
        );
        assert!(
            !reader_text.contains(&format!("\n{label}ॱॱ")),
            "the reader must NOT define the storage — one object per global:\n{reader_text}"
        );
        assembles(&reader_text);
        assembles(&declarer_text);
    }

    /// A record field read emits a load from a RUNTIME base, and the assembler
    /// accepts it.
    ///
    /// The point of the test is the BASE REGISTER. `Load(k)` emits
    /// `आहारः rd, स्तूपसूचकः, …` with the stack pointer wired in; this must emit
    /// the register holding the record's address instead, or a field read would
    /// silently read the frame. So the assertion is that the emitted line does
    /// NOT name the stack pointer, and that `सङ्केतन` encodes what comes out —
    /// an emitted line the assembler refuses is a fault in this emitter.
    #[test]
    fn a_field_read_loads_from_the_records_address_and_not_the_frame() {
        let f = SymbolId(1);
        let mut names = Names::new();
        names.insert(f, ("परीक्षा".into(), "क्षेत्रपठनम्".into()));
        let (base, field) = (ValueId(0), ValueId(1));
        let mut blocks = HashMap::new();
        blocks.insert(
            BlockId(0),
            Block {
                id: BlockId(0),
                insts: vec![
                    // The record's address arrives as the parameter.
                    (base, Instruction::Param(0)),
                    // Its THIRD field, so `WORD * 2` by the owner's rule.
                    (field, Instruction::LoadField(base, 16)),
                ],
                terminator: Some(Terminator::Return(Some(field))),
            },
        );
        let module = Module {
            globals: Vec::new(),
            name: "परीक्षा".into(),
            functions: vec![Function {
                name: f,
                blocks,
                entry_block: BlockId(0),
            }],
            names,
            // NOT the entry: the startup stub calls the entry with no
            // arguments, so a parameter-taking routine is refused there
            // (`EntryTakesParameters`) — which is the emitter being right, and
            // is why this fixture leaves the entry unset.
            entry: None,
        };
        let text = emit_module(&module).expect("the module emits");
        let load = text
            .lines()
            .find(|l| l.trim_start().starts_with("आहारः"))
            .expect("a field read emits a load");
        assert!(
            load.contains("१६न"),
            "the offset is the one the layout gave, in Devanagari: {load}"
        );
        assert!(
            !load.contains(SP),
            "a field read must load from the RECORD's address, not the frame —              this is the whole difference from `Load(k)`, which wires the stack              pointer in: {load}"
        );
        assembles(&text);
    }

    /// `सङ्केतन` on an emitted text. On refusal the test fails NAMING the line —
    /// its number, its text, and what the assembler said — which is the REFUSED
    /// case of the row: an emitted line the assembler refuses is a fault in
    /// this file, and the message has to say which line.
    fn assemble(text: &str) -> Result<Vec<u8>, String> {
        crate::assemble_object(
            text,
            Some("परीक्षा"),
            Target::Uncompressed,
            false,
            Language::English,
        )
        .map_err(|ds| {
            ds.iter()
                .map(|d| {
                    let line = text.lines().nth(d.line.saturating_sub(1)).unwrap_or("");
                    format!("line {}: `{line}` — {}", d.line, d.reason)
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    fn assembles(text: &str) -> Vec<u8> {
        match assemble(text) {
            Ok(bytes) => bytes,
            Err(named) => panic!("सङ्केतन refused the emitted text:\n{named}\n\n{text}"),
        }
    }

    fn block(id: usize, insts: Vec<(ValueId, Instruction)>, t: Terminator) -> (BlockId, Block) {
        (
            BlockId(id),
            Block {
                id: BlockId(id),
                insts,
                terminator: Some(t),
            },
        )
    }

    fn module(functions: Vec<(SymbolId, &str, Function)>, entry: Option<SymbolId>) -> Module {
        let mut names = Names::new();
        for (sym, name, _) in &functions {
            names.insert(*sym, ("परीक्षा".into(), (*name).into()));
        }
        Module {
            globals: Vec::new(),
            name: "परीक्षा".into(),
            functions: functions.into_iter().map(|(_, _, f)| f).collect(),
            names,
            entry,
        }
    }

    #[test]
    fn numerals_are_devanagari_with_rna_for_a_negative() {
        assert_eq!(devanagari(0), "०");
        assert_eq!(devanagari(2047), "२०४७");
        assert_eq!(devanagari(-32), "ऋण३२");
        assert_eq!(hex64(0x1_0000_002A), "०षोड्१००००००२अ");
        assert_eq!(hex64(u64::MAX), "०षोड्ऊऊऊऊऊऊऊऊऊऊऊऊऊऊऊऊ");
        assert_eq!(hex64(0), "०षोड्०");
    }

    #[test]
    fn the_lui_split_carries_the_plus_one_when_lo_is_negative() {
        assert_eq!(split_hi_lo(4095), Some((1, -1)));
        assert_eq!(split_hi_lo(0x1234_5678), Some((0x12345, 0x678)));
        // −300000 = (−73 << 12) − 992: the +0x800 bias rounds hi to the nearest
        // page so that lo stays inside ±2048.
        assert_eq!(split_hi_lo(-300_000), Some(((-73i64) & 0xFFFFF, -992)));
        assert_eq!(split_hi_lo(0x7FFF_F800), None, "hi would be 0x80000");
        assert_eq!(split_hi_lo(1 << 32), None);
    }

    /// Every form of §2.5 in one module, and the assembler accepts every line:
    /// the three constant forms, `Add`, `Sub`, `Param` below and above eight,
    /// a nine-argument `Call`, `Branch`, `CondBranch`, `Return` with and
    /// without a value — plus the startup stub, the frames and the pool.
    #[test]
    fn every_emitted_form_assembles_under_sanketana() {
        let (nine, main) = (SymbolId(1), SymbolId(2));
        // nine(a0..a8): returns a8 + a0 — the ninth argument travels on the stack.
        let mut blocks = HashMap::new();
        let insts: Vec<(ValueId, Instruction)> = (0..9)
            .map(|i| (ValueId(i), Instruction::Param(i)))
            .collect();
        let mut insts = insts;
        insts.push((ValueId(9), Instruction::Add(ValueId(8), ValueId(0))));
        blocks.extend([block(0, insts, Terminator::Return(Some(ValueId(9))))]);
        let f_nine = Function {
            name: nine,
            blocks,
            entry_block: BlockId(0),
        };
        // main: constants of all three forms, a branch, a cond branch, a bare return.
        let mut blocks = HashMap::new();
        let args: Vec<ValueId> = (20..29).map(ValueId).collect();
        let mut insts: Vec<(ValueId, Instruction)> = args
            .iter()
            .enumerate()
            .map(|(i, v)| (*v, Instruction::ConstInt(i as i64 + 1)))
            .collect();
        insts.push((ValueId(30), Instruction::ConstInt(4095)));
        insts.push((ValueId(31), Instruction::ConstInt(-300_000)));
        insts.push((ValueId(32), Instruction::ConstInt(1 << 40)));
        insts.push((ValueId(33), Instruction::ConstInt(-1 - (1 << 40))));
        insts.push((ValueId(34), Instruction::Call(nine, args.clone())));
        insts.push((ValueId(35), Instruction::Sub(ValueId(30), ValueId(31))));
        blocks.extend([
            block(0, insts, Terminator::Branch(BlockId(1))),
            block(
                1,
                vec![],
                Terminator::CondBranch(ValueId(34), BlockId(3), BlockId(2)),
            ),
            block(2, vec![], Terminator::Return(Some(ValueId(35)))),
            block(3, vec![], Terminator::Return(None)),
        ]);
        let f_main = Function {
            name: main,
            blocks,
            entry_block: BlockId(0),
        };
        let m = module(
            vec![(nine, "नवार्थम्", f_nine), (main, "मुख्य", f_main)],
            Some(main),
        );
        let text = emit_module(&m).expect("emits");
        assembles(&text);
        for needle in [
            "आहारः स्थिर",                    // Param(8) from the stack
            "निधानम् स्तूपसूचकःय् ०न",            // the ninth argument stored
            "उपरिभारः स्थिर",                 // lui form
            "ध्रुव०ॱउपरि",                     // the pool
            "॥ अष्टाष्टकाः ०षोड्१०००००००००० ॥", // 1 << 40 in the pool
            "विषमलङ्घनम् स्थिर",                // the synthesized compare
            "वियोगः स्थिर",
        ] {
            assert!(text.contains(needle), "the text lacks `{needle}`:\n{text}");
        }
        println!("{text}");
    }

    /// R1's acceptance in this crate: the fixture assembles, links at the load
    /// address, and every word of its text decodes back to itself (W-211's
    /// `reassemble`, 100%). Running it to status 15 is `crates/yantra/tests/
    /// riscv64_emitter.rs` — `yantra` depends on this crate, not the reverse.
    #[test]
    fn the_recursive_fixture_assembles_links_and_its_bytes_decode_back() {
        let text = emit_module(&fixture_recursive_sum()).expect("emits");
        let bytes = assembles(&text);
        let obj = crate::vastu::read(&bytes).expect("the object reads back");
        // `W-243`: the image is the startup object first, then the module's.
        let startup = assembles(&emit_startup_object(Some("परीक्षामुख्य")));
        let startup = crate::vastu::read(&startup).expect("the startup object reads back");
        let image = crate::samyojana::link_at(&[startup, obj], crate::kosha::LOAD_ADDRESS)
            .unwrap_or_else(|e| panic!("link: {}", e.join("; ")));
        let mut words = 0;
        let mut at = 0;
        while at < image.text.len() {
            let (decoded, width) = decode_at(&image.text, at)
                .unwrap_or_else(|| panic!("word at {at} does not decode"));
            let word = u32::from_le_bytes(image.text[at..at + 4].try_into().unwrap());
            assert_eq!(
                reassemble(&decoded),
                Some(word),
                "word at {at} ({}) does not come back",
                decoded.insn
            );
            at += width;
            words += 1;
        }
        let instructions = text.lines().filter(|l| l.ends_with(" ।")).count()
            + emit_startup_object(Some("परीक्षामुख्य"))
                .lines()
                .filter(|l| l.ends_with(" ।"))
                .count();
        assert_eq!(words, instructions, "one word per instruction line");
        println!("METRIC riscv64_fixture_words {words}");
        println!("METRIC riscv64_fixture_roundtrip 100%");
        // `V-009` part (i-b2): the stack is RESERVED, not stored — no `.data`
        // octet at all (no pool either), and `.bss` is the 64 KiB stack
        // plus the 4 KiB guard below it (the data is empty, so no pad).
        assert_eq!(image.data.len(), 0, "the stack is not in the file; no pool");
        assert_eq!(
            image.bss,
            4096 + STACK_BYTES,
            "the 4 KiB guard and the stack are the image's .bss"
        );
    }

    /// Every routine has a frame (§2.4): `ra` saved at F−8, restored, and the
    /// return is `jalr zero, ra`. The fixture's `f` saves the two `स्थिर`s it uses.
    #[test]
    fn every_routine_has_a_frame_that_saves_ra_and_the_sthiras_it_uses() {
        let text = emit_module(&fixture_recursive_sum()).expect("emits");
        // f uses स्थिर० and स्थिर१: 8·(2 + 1) rounded to 16 = 32, ra at 24.
        assert!(text.contains("परीक्षायोगफलम्ॱॱ\nयोगः स्तूपसूचकःम् स्तूपसूचकःन ऋण३२न ।\nनिधानम् स्तूपसूचकःय् २४न पुनःस्थानम्न ।\nनिधानम् स्तूपसूचकःय् ०न स्थिर०न ।\nनिधानम् स्तूपसूचकःय् ८न स्थिर१न ।"), "{text}");
        // main's two values share स्थिर०: 8·(1 + 1) = 16, ra at 8 — a frame all the same.
        assert!(text.contains("परीक्षामुख्यॱॱ\nयोगः स्तूपसूचकःम् स्तूपसूचकःन ऋण१६न ।\nनिधानम् स्तूपसूचकःय् ८न पुनःस्थानम्न ।\nनिधानम् स्तूपसूचकःय् ०न स्थिर०न ।"), "{text}");
        assert_eq!(
            text.matches("सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।").count(),
            2,
            "two routines, two returns"
        );
        assert_eq!(
            text.matches("आहारः पुनःस्थानम्म् स्तूपसूचकःत् ").count(),
            2,
            "two routines restore ra"
        );
        assert!(text.contains("लङ्घनम् पुनःस्थानम्म् परीक्षायोगफलम्य् ।"));
        // `W-243`: every routine label is exported, and the module text begins
        // with its first routine — the startup is an object of its own.
        assert!(
            text.starts_with("॥ वैश्विकम् परीक्षायोगफलम् ॥\nपरीक्षायोगफलम्ॱॱ\n"),
            "{text}"
        );
        assert!(text.contains("॥ वैश्विकम् परीक्षामुख्य ॥\nपरीक्षामुख्यॱॱ\n"));
        assert!(!text.contains("यन्त्रारम्भ"), "no startup in a module's text");
        // sp via auipc, never lui (§2.6), in the startup object.
        let startup = emit_startup_object(Some("परीक्षामुख्य"));
        assert!(startup.starts_with(
            "॥ वैश्विकम् यन्त्रारम्भ ॥\nयन्त्रारम्भॱॱ\nस्थानसापेक्षयोगः स्तूपसूचकःम् स्तूपान्तःॱउपरिन ।"
        ));
        assert!(
            startup.contains("स्तूपःॱॱ\n॥ स्थानम् ६५५३६ ॥\nस्तूपान्तःॱॱ\n"),
            "{startup}"
        );
        assert!(!startup.contains("उपरिभारः स्तूपसूचकःम्"));
    }

    /// THE REFUSED CASE: an emitted line `सङ्केतन` refuses fails the test naming
    /// the line. `निधेहि` is the word `emit.rs` writes for a move and it appears
    /// 0 times in `spec/mnemonics-riscv64.src.tsv` (research/25 §1.2) — so it is
    /// the deliberately wrong mnemonic, injected into one line of an otherwise
    /// good text; the assembler's diagnostic must point at that line and no other.
    #[test]
    fn a_line_sanketana_refuses_is_named_by_number_and_text() {
        let text = emit_module(&fixture_recursive_sum()).expect("emits");
        let lines: Vec<&str> = text.lines().collect();
        let victim = lines
            .iter()
            .position(|l| l.starts_with("योगः अर्थ०म् "))
            .expect("the move into अर्थ०");
        let mut wrong = lines.clone();
        let bad = lines[victim].replacen("योगः", "निधेहि", 1);
        wrong[victim] = &bad;
        let wrong = wrong.join("\n") + "\n";
        let named = assemble(&wrong).expect_err("निधेहि is not a mnemonic and must be refused");
        let expected = format!("line {}: `{bad}`", victim + 1);
        assert!(
            named.starts_with(&expected),
            "the refusal names the wrong line:\n{named}"
        );
        assert!(
            named.contains("निधेहि"),
            "the refusal names the word:\n{named}"
        );
        println!("REFUSED {named}");
    }

    /// `W-306c` — THE NARROW STORE'S SUFFIX, AND THE WIDTH THAT MUST REFUSE.
    ///
    /// THREE STATES, NOT TWO: a width the table names emits its own mnemonic; ८
    /// emits the BARE `निधानम्` character for character, which is what keeps
    /// every corpus image octet-identical; and a width the table does NOT name
    /// is REFUSED. A two-state instrument here — "it emits something" — would
    /// pass on the defect this arm was written to stop, because the old code
    /// emitted the bare store for every width including three.
    ///
    /// EACH EMITTED TEXT IS PUT THROUGH `सङ्केतन`. A suffix this emitter
    /// invented and the assembler has never heard of would otherwise read as
    /// green here and refuse at assembly — `निधेहि`'s lesson, paid once.
    #[test]
    fn a_narrow_store_takes_its_width_s_mnemonic_and_an_unnamed_width_refuses() {
        // `AddrOfGlobal` of the record cursor is the one address this fixture
        // can form without a declaration, and `StoreAt` is the kind under test.
        let s = SymbolId(1);
        let store = |w: u64| {
            let mut blocks = HashMap::new();
            blocks.extend([block(
                0,
                vec![
                    (ValueId(0), Instruction::AddrOfGlobal(RECORD_CURSOR_SYMBOL)),
                    (ValueId(1), Instruction::ConstInt(7)),
                    (ValueId(2), Instruction::StoreAt(ValueId(0), ValueId(1), w)),
                ],
                Terminator::Return(None),
            )]);
            emit_module(&module(
                vec![(
                    s,
                    "निधानम्",
                    Function {
                        name: s,
                        blocks,
                        entry_block: BlockId(0),
                    },
                )],
                None,
            ))
        };

        // STATE 1 — a named narrow width emits its own mnemonic, and assembles.
        for (w, mnemonic) in [(1u64, "निधानम्ॱअ८"), (2, "निधानम्ॱअ१६"), (4, "निधानम्ॱअ३२")]
        {
            let text = store(w).unwrap_or_else(|e| panic!("width {w} emits: {e}"));
            let line = text
                .lines()
                .find(|l| l.contains("य् ०न ") && !l.contains(SP))
                .unwrap_or_else(|| panic!("width {w} emits a store line:\n{text}"));
            assert!(
                line.starts_with(&format!("{mnemonic} ")),
                "width {w} must emit `{mnemonic}`, not:\n{line}"
            );
            assemble(&text).unwrap_or_else(|e| panic!("सङ्केतन refused width {w}: {e}\n{text}"));
        }

        // STATE 2 — ८ emits the BARE word, with no `ॱअ` suffix anywhere on the
        // line. `starts_with("निधानम् ")` alone would also accept
        // `निधानम्ॱअ६४`, so the suffix is excluded by name.
        let text = store(8).expect("a whole word emits");
        let line = text
            .lines()
            .find(|l| l.contains("य् ०न ") && !l.contains(SP))
            .unwrap_or_else(|| panic!("width ८ emits a store line:\n{text}"));
        assert!(line.starts_with("निधानम् "), "width ८ must be bare:\n{line}");
        assert!(
            !line.contains("निधानम्ॱअ"),
            "width ८ must carry NO suffix:\n{line}"
        );
        assemble(&text).expect("सङ्केतन takes the bare store");

        // STATE 3 — THE REFUSED CASE, RUN. ३ is not १, २, ४ or ८; the old arm
        // emitted the bare eight-octet store for it and corrupted the five
        // octets past the field with no diagnostic anywhere.
        assert_eq!(
            store(3),
            Err(Refusal::StoreWidthUnnamed {
                function: "परीक्षानिधानम्".into(),
                bytes: 3
            }),
            "a width the table does not name must REFUSE, not fall through"
        );
        // And the refusal SAYS the width — `बूल`-shaped "it refused" would not
        // tell the next reader which store it was.
        let named = store(3).expect_err("३ is refused").to_string();
        assert!(named.contains('3'), "the refusal names the width: {named}");
        println!("REFUSED {named}");
    }

    #[test]
    fn unreachable_is_refused_by_block() {
        let s = SymbolId(1);
        let mut blocks = HashMap::new();
        blocks.extend([
            block(
                0,
                vec![(ValueId(0), Instruction::ConstInt(1))],
                Terminator::Branch(BlockId(1)),
            ),
            block(1, vec![], Terminator::Unreachable),
        ]);
        let f = Function {
            name: s,
            blocks,
            entry_block: BlockId(0),
        };
        let m = module(vec![(s, "अगम्यम्", f)], None);
        assert_eq!(
            emit_module(&m),
            Err(Refusal::Unreachable {
                function: "परीक्षाअगम्यम्".into(),
                block: BlockId(1)
            })
        );
    }

    #[test]
    fn a_label_collision_is_refused_by_pair() {
        let (a, b) = (SymbolId(1), SymbolId(2));
        let leaf = |sym| {
            let mut blocks = HashMap::new();
            blocks.extend([block(0, vec![], Terminator::Return(None))]);
            Function {
                name: sym,
                blocks,
                entry_block: BlockId(0),
            }
        };
        let mut names = Names::new();
        names.insert(a, ("पद".into(), "विभागपाठः".into()));
        names.insert(b, ("पदविभाग".into(), "पाठः".into()));
        let m = Module {
            globals: Vec::new(),
            name: "पद".into(),
            functions: vec![leaf(a), leaf(b)],
            names,
            entry: None,
        };
        assert_eq!(
            emit_module(&m),
            Err(Refusal::LabelCollision {
                label: "पदविभागपाठः".into(),
                first: "पद ॱ विभागपाठः".into(),
                second: "पदविभाग ॱ पाठः".into(),
            })
        );
        // And against a name the emitter owns. Concatenation is by code point,
        // not by sandhi — `यन्त्र` ⧺ `आरम्भ` is NOT `यन्त्रारम्भ` — so the pair
        // that collides is the one whose letters do.
        let mut names = Names::new();
        names.insert(a, ("यन्त्रा".into(), "रम्भ".into()));
        let m = Module {
            globals: Vec::new(),
            name: "यन्त्रा".into(),
            functions: vec![leaf(a)],
            names,
            entry: None,
        };
        assert!(matches!(
            emit_module(&m),
            Err(Refusal::LabelCollision { label, first, .. }) if label == "यन्त्रारम्भ" && first == "the emitter"
        ));
    }

    #[test]
    fn a_param_after_a_call_is_refused() {
        let (s, callee) = (SymbolId(1), SymbolId(2));
        let mut blocks = HashMap::new();
        blocks.extend([block(
            0,
            vec![
                (ValueId(0), Instruction::Call(callee, vec![])),
                (ValueId(1), Instruction::Param(0)),
            ],
            Terminator::Return(Some(ValueId(1))),
        )]);
        let f = Function {
            name: s,
            blocks,
            entry_block: BlockId(0),
        };
        let mut m = module(vec![(s, "क्रमः", f)], None);
        m.names.insert(callee, ("परीक्षा".into(), "अन्यः".into()));
        assert_eq!(
            emit_module(&m),
            Err(Refusal::ParamAfterCall {
                function: "परीक्षाक्रमः".into(),
                block: BlockId(0),
                param: 0
            })
        );
    }

    #[test]
    fn a_frame_one_addi_cannot_address_is_refused() {
        // 260 constants all live until a final chain of adds: 12 in registers,
        // 248 spilled, frame = 8·(248 + 12 + 1) rounded = 2096 > 2047.
        let s = SymbolId(1);
        let n = 260;
        let mut insts: Vec<(ValueId, Instruction)> = (0..n)
            .map(|i| (ValueId(i), Instruction::ConstInt(i as i64)))
            .collect();
        let mut acc = ValueId(0);
        for i in 1..n {
            let v = ValueId(n + i);
            insts.push((v, Instruction::Add(acc, ValueId(i))));
            acc = v;
        }
        let mut blocks = HashMap::new();
        blocks.extend([block(0, insts, Terminator::Return(Some(acc)))]);
        let f = Function {
            name: s,
            blocks,
            entry_block: BlockId(0),
        };
        let m = module(vec![(s, "बृहत्", f)], None);
        match emit_module(&m) {
            Err(Refusal::FrameTooLarge { function, bytes }) => {
                assert_eq!(function, "परीक्षाबृहत्");
                assert!(bytes > 2047, "{bytes}");
            }
            other => panic!("expected FrameTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn an_entry_with_parameters_is_refused() {
        let s = SymbolId(1);
        let mut blocks = HashMap::new();
        blocks.extend([block(
            0,
            vec![(ValueId(0), Instruction::Param(0))],
            Terminator::Return(Some(ValueId(0))),
        )]);
        let f = Function {
            name: s,
            blocks,
            entry_block: BlockId(0),
        };
        let m = module(vec![(s, "मुख्य", f)], Some(s));
        assert_eq!(
            emit_module(&m),
            Err(Refusal::EntryTakesParameters {
                function: "परीक्षामुख्य".into(),
                params: 1
            })
        );
    }

    /// Spilled values go through `क्षणिक` and the frame's slots, and the text
    /// still assembles: twenty live constants against twelve registers.
    #[test]
    fn spills_travel_through_the_frame_and_assemble() {
        let s = SymbolId(1);
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
        let mut blocks = HashMap::new();
        blocks.extend([block(0, insts, Terminator::Return(Some(acc)))]);
        let f = Function {
            name: s,
            blocks,
            entry_block: BlockId(0),
        };
        let m = module(vec![(s, "मुख्य", f)], Some(s));
        let text = emit_module(&m).expect("emits");
        // The spilled constant is the SECOND operand of its add, so it is read
        // into the second scratch register.
        assert!(
            text.contains("आहारः क्षणिक१म् स्तूपसूचकःत् "),
            "a spill is read:\n{text}"
        );
        assert!(
            text.contains("निधानम् स्तूपसूचकःय् ०न क्षणिक२न ।"),
            "a spill is written:\n{text}"
        );
        assembles(&text);
    }

    /// `W-245`: every form the row added assembles, and the text says which
    /// form each is — the FUSED compare (`न्यूनलङ्घनम् … त्`, no `न्यूनम्` line
    /// for it), the local's slot at `8·num_spills` (here ०), the eight verbs,
    /// and the six compares as values. Every word of the linked image decodes
    /// back, M extension included.
    #[test]
    fn every_form_of_w245_assembles_and_its_bytes_decode_back() {
        let text = emit_module(&fixture_forms()).expect("emits");
        let bytes = assembles(&text);
        for needle in [
            // the loop's compare fused with its branch: subject करण, standard अपादान
            "न्यूनलङ्घनम् स्थिर०न स्थिर१त् परीक्षारूपाणिपर्व२य् ।",
            // the local, stored at entry and loaded in the condition block
            "निधानम् स्तूपसूचकःय् ०न स्थिर०न ।",
            "आहारः स्थिर०म् स्तूपसूचकःत् ०न ।",
            "गुणनम् स्थिर",
            "भागः स्थिर",
            "शेषः स्थिर",
            "वामसरणम् स्थिर",
            "दक्षिणसरणम् स्थिर",
            "युक्तम् स्थिर",
            "विकल्पः स्थिर",
            "वैषम्यम् स्थिर",
            // the compares as values (the registers are the allocator's)
            "न्यूनम् स्थिर२म् स्थिर०न स्थिर१न ।",      // Lt as a value
            "अचिह्नन्यूनम् स्थिर२म् स्थिर०न स्थिर१न ।", // Ltu as a value
        ] {
            assert!(text.contains(needle), "the text lacks `{needle}`:\n{text}");
        }
        let has_line = |head: &str, tail: &str| {
            text.lines()
                .any(|l| l.starts_with(head) && l.ends_with(tail))
        };
        assert!(
            has_line("वियोगः स्थिर", "न ।"),
            "Eq/Ne begin with a sub:\n{text}"
        );
        assert!(
            has_line("अचिह्नन्यूनम् स्थिर", "न १न ।"),
            "Eq: seqz after the sub:\n{text}"
        );
        assert!(
            text.lines()
                .any(|l| l.starts_with("अचिह्नन्यूनम् स्थिर") && l.contains("म् शून्यःन स्थिर")),
            "Ne: snez after the sub:\n{text}"
        );
        assert_eq!(
            text.lines()
                .filter(|l| l.starts_with("वैषम्यम् स्थिर") && l.ends_with("न १न ।"))
                .count(),
            2,
            "Ge and Geu flip the bit with xori:\n{text}"
        );
        // The fused compare emitted no value line: exactly two `न्यूनम्` lines,
        // the Lt-as-value and the Ge-as-value.
        assert_eq!(
            text.lines().filter(|l| l.starts_with("न्यूनम् ")).count(),
            2,
            "{text}"
        );
        let obj = crate::vastu::read(&bytes).expect("the object reads back");
        let image = crate::samyojana::link_at(&[obj], crate::kosha::LOAD_ADDRESS)
            .unwrap_or_else(|e| panic!("link: {}", e.join("; ")));
        let mut at = 0;
        let mut words = 0;
        while at < image.text.len() {
            let (decoded, width) = decode_at(&image.text, at)
                .unwrap_or_else(|| panic!("word at {at} does not decode"));
            let word = u32::from_le_bytes(image.text[at..at + 4].try_into().unwrap());
            assert_eq!(reassemble(&decoded), Some(word), "{}", decoded.insn);
            at += width;
            words += 1;
        }
        assert_eq!(words, text.lines().filter(|l| l.ends_with(" ।")).count());
        println!("METRIC riscv64_forms_fixture_words {words}");
        println!("{text}");
    }

    /// REFUSED TO FUSE: a compare read TWICE — by the branch and by an
    /// instruction in another block — keeps its value form, and the branch is
    /// the synthesized `विषमलङ्घनम् … शून्यःत्` on that value. And a compare that
    /// is not the block's LAST instruction is not fused either.
    #[test]
    fn a_compare_read_elsewhere_or_not_last_is_not_fused() {
        let s = SymbolId(1);
        let (a, b, c, one, keep) = (ValueId(0), ValueId(1), ValueId(2), ValueId(3), ValueId(4));
        let mut blocks = HashMap::new();
        blocks.extend([
            block(
                0,
                vec![
                    (a, Instruction::ConstInt(1)),
                    (b, Instruction::ConstInt(2)),
                    (c, Instruction::Cmp(CmpOp::Eq, a, b)),
                ],
                Terminator::CondBranch(c, BlockId(1), BlockId(2)),
            ),
            block(
                1,
                vec![
                    (one, Instruction::ConstInt(1)),
                    (keep, Instruction::Add(c, one)),
                ],
                Terminator::Return(Some(keep)),
            ),
            block(2, vec![], Terminator::Return(Some(c))),
        ]);
        let f = Function {
            name: s,
            blocks,
            entry_block: BlockId(0),
        };
        let m = module(vec![(s, "द्विः", f)], None);
        let text = emit_module(&m).expect("emits");
        assert!(
            text.contains("वियोगः स्थिर") && text.contains("अचिह्नन्यूनम् "),
            "the compare keeps its value form:\n{text}"
        );
        assert!(
            text.contains("विषमलङ्घनम् स्थिर") && !text.contains("समलङ्घनम् स्थिर०न"),
            "the branch is on the value, not fused:\n{text}"
        );
        assembles(&text);

        // Not last: a constant after the compare.
        let mut blocks = HashMap::new();
        blocks.extend([
            block(
                0,
                vec![
                    (a, Instruction::ConstInt(1)),
                    (b, Instruction::ConstInt(2)),
                    (c, Instruction::Cmp(CmpOp::Eq, a, b)),
                    (one, Instruction::ConstInt(1)),
                ],
                Terminator::CondBranch(c, BlockId(1), BlockId(2)),
            ),
            block(1, vec![], Terminator::Return(Some(one))),
            block(2, vec![], Terminator::Return(None)),
        ]);
        let f = Function {
            name: s,
            blocks,
            entry_block: BlockId(0),
        };
        let m = module(vec![(s, "अनन्तिमः", f)], None);
        let text = emit_module(&m).expect("emits");
        assert!(text.contains("विषमलङ्घनम् "), "{text}");
        assert!(!text.contains("समलङ्घनम् "), "{text}");
    }

    /// A local's slot lies ABOVE the spill slots: with spills, `Load(0)` reads
    /// `8·num_spills`, and the frame counts both regions.
    #[test]
    fn a_local_slot_lies_above_the_spill_slots_in_the_frame() {
        let s = SymbolId(1);
        let n = 20;
        let mut insts: Vec<(ValueId, Instruction)> = (0..n)
            .map(|i| (ValueId(i), Instruction::ConstInt(i as i64 + 1)))
            .collect();
        insts.push((ValueId(n), Instruction::Store(1, ValueId(0))));
        let mut acc = ValueId(0);
        for i in 1..n {
            let v = ValueId(n + i);
            insts.push((v, Instruction::Add(acc, ValueId(i))));
            acc = v;
        }
        insts.push((ValueId(2 * n), Instruction::Load(1)));
        let mut blocks = HashMap::new();
        blocks.extend([block(0, insts, Terminator::Return(Some(ValueId(2 * n))))]);
        let f = Function {
            name: s,
            blocks,
            entry_block: BlockId(0),
        };
        let alloc = allocate_registers(&f, ALLOCATABLE);
        assert!(alloc.num_spills > 0, "the fixture spills");
        let frame = frame_layout(&alloc, None, count_locals(&f));
        assert_eq!(frame.num_locals, 2, "slots ० and १ — one past the highest");
        let m = module(vec![(s, "स्थानीयः", f)], Some(s));
        let text = emit_module(&m).expect("emits");
        let off = devanagari(8 * (alloc.num_spills as i64 + 1));
        assert!(
            text.contains(&format!("आहारः स्थिर०म् स्तूपसूचकःत् {off}न ।"))
                || text.contains(&format!("स्तूपसूचकःत् {off}न ।")),
            "Load(1) reads slot 8·(spills+1) = {off}:\n{text}"
        );
        assert!(text.contains(&format!("निधानम् स्तूपसूचकःय् {off}न ")), "{text}");
        assert!(frame.bytes >= 8 * (alloc.num_spills as i64 + 2) + 8);
        assembles(&text);
    }

    /// An entryless module (§2.6) is a stub that calls nothing and halts `0x5555`.
    #[test]
    fn an_entryless_module_emits_a_stub_that_halts_success() {
        let m = module(vec![], None);
        let text = emit_module(&m).expect("emits");
        assert!(
            !text.contains("यन्त्रारम्भ"),
            "a module carries no startup:\n{text}"
        );
        assembles(&text);
        // The stub for an image with no entry (`W-243`: the startup object).
        let startup = emit_startup_object(None);
        assert!(!startup.contains("लङ्घनम् पुनःस्थानम्म्"), "no call:\n{startup}");
        assert!(startup.contains("०षोड्५५५"));
        assembles(&startup);
    }

    /// ॥ A ROUTINE WHOSE CONDITIONAL CANNOT REACH ITS TARGET IS COUNTED ॥ `W-332`.
    ///
    /// **THE COUNTER IS THE CLAIM, SO THE TEST IS ABOUT THE COUNTER AND NOT
    /// ABOUT THE TEXT.** That the far conditional gets relaxed rather than
    /// refused is already pinned elsewhere; what had no witness is that the
    /// event is OBSERVABLE. A relaxation writes three words where one stood and
    /// nothing distinguishes those three from three the routine would have
    /// written anyway, so before this the only evidence a relaxation happened
    /// was that the emitter did not refuse — an absence.
    ///
    /// **BOTH DIRECTIONS ARE ASSERTED, AND AN EARLIER VERSION OF THIS TEST SAID
    /// THE SECOND ONE COULD NOT BE.** That version read a process-global
    /// `AtomicUsize`. `cargo test` runs this binary's tests on several threads,
    /// so a concurrent emit could only ADD to the delta: `>= 1` was sound under
    /// any interleaving and the converse — that a routine which FITS leaves the
    /// census alone — was unassertable, because a delta of 0 was not a property
    /// of this thread. The margin said so and treated it as the price.
    ///
    /// It was not the price; it was the design. `emit_module_and_relaxations`
    /// returns the labels relaxed BY THIS CALL, so both directions are now
    /// properties of this call alone and neither can flake when the suite is
    /// busy. The negative control below is the half that was impossible before,
    /// and it is the half that catches a census which counts every routine.
    ///
    /// **AND IT NAMES, WHICH A TALLY CANNOT.** The assertion is on the LABEL, so
    /// a census that fires on the wrong routine fails here instead of passing
    /// with a plausible 1.
    #[test]
    fn a_conditional_that_cannot_reach_its_target_is_counted_as_relaxed() {
        // ~2,000 lines between the branch and its target: a conditional reaches
        // ±4 KiB, the emitter writes four bytes per line, so 1,024 lines is the
        // edge and this clears it twice over even if a line is folded away.
        const FILLER: usize = 2_000;

        let c = ValueId(0);
        let mut blocks = HashMap::new();
        blocks.insert(
            BlockId(0),
            Block {
                id: BlockId(0),
                insts: vec![(c, Instruction::ConstInt(1))],
                // The FAR arm is `BlockId(2)`, past the filler. Blocks lay out
                // in id order with fallthrough, so this is the whole distance.
                terminator: Some(Terminator::CondBranch(c, BlockId(2), BlockId(1))),
            },
        );
        let mut insts = Vec::with_capacity(FILLER);
        for i in 0..FILLER {
            insts.push((ValueId(i + 1), Instruction::Add(c, c)));
        }
        blocks.insert(
            BlockId(1),
            Block {
                id: BlockId(1),
                insts,
                terminator: Some(Terminator::Branch(BlockId(2))),
            },
        );
        blocks.insert(
            BlockId(2),
            Block {
                id: BlockId(2),
                insts: Vec::new(),
                terminator: Some(Terminator::Return(Some(c))),
            },
        );

        let func = Function {
            name: SymbolId(1),
            blocks,
            entry_block: BlockId(0),
        };
        let m = module(vec![(SymbolId(1), "दूरशाखा", func)], None);

        let (text, relaxed) = emit_module_and_relaxations(&m).expect(
            "a far conditional is RELAXED, not refused — if this refuses, the \
             relaxation pass is gone and the census is the lesser loss",
        );

        assert_eq!(
            relaxed,
            vec![routine_label(&m.names, SymbolId(1)).expect("the routine has a label")],
            "the emitter relaxed this routine — it had to, the conditional is \
             ~{FILLER} lines from its target — so the census must hold exactly \
             its label and holds {relaxed:?}"
        );

        // ── THE NEGATIVE CONTROL, which the process-global version could not
        // state: the SAME shape with the filler removed fits its own branch, so
        // the census must come back EMPTY. Without this, a census that pushed
        // every routine it emitted would pass the assertion above.
        let near = module(vec![(SymbolId(1), "निकटशाखा", fitting_conditional())], None);
        let (near_text, near_relaxed) =
            emit_module_and_relaxations(&near).expect("a conditional in reach emits");
        assert!(
            near_relaxed.is_empty(),
            "a conditional that REACHES its target was counted as relaxed, so \
             the census is counting routines and not relaxations: {near_relaxed:?}"
        );

        assembles(&text);
        assembles(&near_text);
        println!("METRIC riscv_relaxed_routines_far {}", relaxed.len());
        println!("METRIC riscv_relaxed_routines_near {}", near_relaxed.len());
    }

    /// The far routine's shape with the filler removed: one conditional whose
    /// target is a handful of instructions away, well inside the B-type's ±4 KiB.
    /// It exists so the census has a NEGATIVE control built the same way as its
    /// positive one — a different shape would confound "fits" with "differs".
    fn fitting_conditional() -> Function {
        let c = ValueId(0);
        let mut blocks = HashMap::new();
        blocks.insert(
            BlockId(0),
            Block {
                id: BlockId(0),
                insts: vec![(c, Instruction::ConstInt(1))],
                terminator: Some(Terminator::CondBranch(c, BlockId(2), BlockId(1))),
            },
        );
        blocks.insert(
            BlockId(1),
            Block {
                id: BlockId(1),
                insts: vec![(ValueId(1), Instruction::Add(c, c))],
                terminator: Some(Terminator::Branch(BlockId(2))),
            },
        );
        blocks.insert(
            BlockId(2),
            Block {
                id: BlockId(2),
                insts: Vec::new(),
                terminator: Some(Terminator::Return(Some(c))),
            },
        );
        Function {
            name: SymbolId(1),
            blocks,
            entry_block: BlockId(0),
        }
    }

    // ── `W-306`, THE J-TYPE'S OWN REACH ────────────────────────────────────

    /// A routine `check_branch_ranges` can read but that carries no conditional,
    /// so the B-type half is a no-op and only the J-type half speaks.
    fn jumping_function() -> Function {
        let entry = BlockId(0);
        let mut blocks = HashMap::new();
        blocks.insert(
            entry,
            Block {
                id: entry,
                insts: Vec::new(),
                terminator: Some(Terminator::Return(None)),
            },
        );
        Function {
            name: SymbolId(1),
            blocks,
            entry_block: entry,
        }
    }

    /// `<jump>` then `words` filler instructions then the label: the jump sits at
    /// address ०, the label at `4 * (1 + words)`.
    fn jump_text(jump: &str, words: usize) -> String {
        let mut t = String::with_capacity(32 * words + 64);
        t.push_str(jump);
        t.push('\n');
        for _ in 0..words {
            t.push_str("योगः स्थिर०म् शून्यःन ०न ।\n");
        }
        t.push_str("परीक्षानिर्गमॱॱ");
        t
    }

    /// **THE J-TYPE IS MEASURED IN BOTH MODES, AND THE BOUNDARY IS EXACT.**
    ///
    /// `W-306` relaxed the far CONDITIONAL by replacing it with one of these, so
    /// a J-type left unmeasured did not remove the wrap — it moved it one
    /// instruction along and out of the only guard that was looking. The
    /// relaxation cannot be the answer here: it is already the relaxed form.
    ///
    /// ±1 MiB is 1_048_576, and `4 * (1 + 262_142) = 1_048_572` is the last
    /// distance that stands. One more filler word is the first that does not,
    /// and BOTH are asserted: a guard tested only on the refusal would pass with
    /// the bound off by any amount in the accepting direction.
    #[test]
    fn a_jump_past_one_mib_is_refused_in_both_modes() {
        let func = jumping_function();
        let jump = format!("लङ्घनम् {ZERO}म् परीक्षानिर्गमय् ।");

        for relax in [false, true] {
            let near = jump_text(&jump, 262_142);
            assert!(
                check_branch_ranges(&func, "परीक्षा", &near, relax).is_ok(),
                "1_048_572 bytes is inside ±1 MiB and must stand (relax={relax})"
            );

            let far = jump_text(&jump, 262_143);
            match check_branch_ranges(&func, "परीक्षा", &far, relax) {
                Err(Refusal::JumpOutOfRange {
                    function,
                    target,
                    bytes,
                }) => {
                    assert_eq!(function, "परीक्षा");
                    assert_eq!(target, "परीक्षानिर्गम", "named by LABEL, not by block");
                    assert_eq!(bytes, 1_048_576);
                }
                other => panic!("relax={relax}: expected JumpOutOfRange, got {other:?}"),
            }
        }
    }

    /// The mirror of [`jump_text`]: `परीक्षानिर्गमॱॱ` FIRST, then `words` filler
    /// instructions, then the jump. The label sits at address ०, the jump at
    /// `4 * words`, so the distance the reader measures is `-4 * words` — the
    /// only direction [`jump_text`] cannot build, because it puts the jump first
    /// and every distance it can reach is forward.
    fn backward_jump_text(jump: &str, words: usize) -> String {
        let mut t = String::with_capacity(32 * words + 64);
        t.push_str("परीक्षानिर्गमॱॱ\n");
        for _ in 0..words {
            t.push_str("योगः स्थिर०म् शून्यःन ०न ।\n");
        }
        t.push_str(jump);
        t
    }

    /// **THE NEGATIVE ENDPOINT, AND IT IS NOT THE POSITIVE ONE MIRRORED.**
    ///
    /// [`a_jump_past_one_mib_is_refused_in_both_modes`] measures the forward
    /// side only, and a bound asserted on one side alone would pass with the
    /// range spelled `-(1 << 20)..=(1 << 20)`, `(-(1 << 20) + 4)..(1 << 20)`, or
    /// symmetric at either endpoint — three different guards the forward corners
    /// cannot tell apart. The range is HALF-OPEN: `-(1 << 20)..(1 << 20)`, which
    /// is RISC-V's 21-bit signed J-type reach exactly, so the two sides do NOT
    /// stop at the same magnitude.
    ///
    /// Forward, the last distance that stands is `+1_048_572` — `+1_048_576` is
    /// already out. Backward, `-1_048_576` STANDS and the first that does not is
    /// `-1_048_580`, one word further. Both corners are asserted, and the
    /// accepting one IS the control the refusal needs: a guard tested only on
    /// the refusal would pass with the negative bound short by any amount.
    #[test]
    fn a_backward_jump_reaches_one_mib_exactly_and_no_word_further() {
        let func = jumping_function();
        let jump = format!("लङ्घनम् {ZERO}म् परीक्षानिर्गमय् ।");

        for relax in [false, true] {
            // -1_048_576 = -(1 << 20), the FIRST value the half-open range
            // admits, and the one a symmetric `-1_048_572` bound would refuse.
            let near = backward_jump_text(&jump, 262_144);
            assert!(
                check_branch_ranges(&func, "परीक्षा", &near, relax).is_ok(),
                "-1_048_576 is the negative endpoint and it STANDS (relax={relax})"
            );

            // One word further back, and the sign is carried into the refusal.
            let far = backward_jump_text(&jump, 262_145);
            match check_branch_ranges(&func, "परीक्षा", &far, relax) {
                Err(Refusal::JumpOutOfRange {
                    function,
                    target,
                    bytes,
                }) => {
                    assert_eq!(function, "परीक्षा");
                    assert_eq!(target, "परीक्षानिर्गम", "named by LABEL, not by block");
                    assert_eq!(bytes, -1_048_580, "the refusal reports a BACKWARD distance");
                }
                other => panic!("relax={relax}: expected JumpOutOfRange, got {other:?}"),
            }
        }
    }

    /// **THE TWO CASES THAT MUST STILL BE ACCEPTED, AND THEY ARE NOT THE SAME
    /// CASE.** A `लङ्घनम्` linked through `पुनःस्थानम्` is a CALL — `emit_call`
    /// writes it, its distance belongs to the linker, and refusing it would red
    /// every routine that calls anything from far enough away. A jump whose
    /// target this text declares NO label line for is the second: the startup
    /// stub spells `यन्त्रसमाप्ति` and `यन्त्रचक्र` itself, and a guard that
    /// refused an unknown target would refuse the program's own entry.
    ///
    /// Both are set at a distance that WOULD be refused were they measured, so
    /// the test cannot pass by the distance being short.
    #[test]
    fn a_call_and_an_unlabelled_target_are_not_this_routine_s_distance() {
        let func = jumping_function();

        let call = jump_text(&format!("लङ्घनम् {RA}म् परीक्षानिर्गमय् ।"), 262_143);
        assert!(
            check_branch_ranges(&func, "परीक्षा", &call, false).is_ok(),
            "a पुनःस्थानम्-linked लङ्घनम् is a call, not a jump inside this routine"
        );

        let unknown = jump_text(&format!("लङ्घनम् {ZERO}म् यन्त्रचक्रय् ।"), 262_143);
        assert!(
            check_branch_ranges(&func, "परीक्षा", &unknown, false).is_ok(),
            "a target this text declares no label line for is not measurable here"
        );

        // AND THE CONTROL: the same two texts with the jump spelled the way the
        // emitter spells an in-routine one ARE refused, so the acceptances above
        // are the FORM's and not the reader having stopped reaching the text.
        let real = jump_text(&format!("लङ्घनम् {ZERO}म् परीक्षानिर्गमय् ।"), 262_143);
        assert!(matches!(
            check_branch_ranges(&func, "परीक्षा", &real, false),
            Err(Refusal::JumpOutOfRange { .. })
        ));
    }

    /// A routine whose `Branch` terminator aims at a REAL block: entry falls to
    /// `पर्व२`, which returns. [`jumping_function`]'s one block ends in `Return`,
    /// so every J-type corner above targets [`exit_label`] — a name no `BlockId`
    /// spells — and the block-label case was taken by nothing on either side.
    fn block_jumping_function() -> Function {
        let entry = BlockId(0);
        let target = BlockId(2);
        let mut blocks = HashMap::new();
        blocks.insert(
            entry,
            Block {
                id: entry,
                insts: Vec::new(),
                terminator: Some(Terminator::Branch(target)),
            },
        );
        blocks.insert(
            target,
            Block {
                id: target,
                insts: Vec::new(),
                terminator: Some(Terminator::Return(None)),
            },
        );
        Function {
            name: SymbolId(1),
            blocks,
            entry_block: entry,
        }
    }

    /// `<exit label> <jump> <words fillers> <block २'s label>`: the jump sits at
    /// address ०, block २'s label at `4 * (1 + words)`, and `परीक्षानिर्गम` — the
    /// DECOY — sits at address ० as well, distance `०` from the jump.
    ///
    /// The decoy is the point of the fixture. A reader that resolved every J-type
    /// to [`exit_label`] instead of to the name the line carries would measure `०`
    /// here and accept, and a text with only ONE label line could not tell that
    /// reading from the right one.
    fn block_jump_text(words: usize) -> String {
        let mut t = String::with_capacity(32 * words + 64);
        t.push_str("परीक्षानिर्गमॱॱ\n");
        t.push_str(&format!("लङ्घनम् {ZERO}म् परीक्षापर्व२य् ।\n"));
        for _ in 0..words {
            t.push_str("योगः स्थिर०म् शून्यःन ०न ।\n");
        }
        t.push_str("परीक्षापर्व२ॱॱ");
        t
    }

    /// The mirror: block २'s label FIRST at address ०, then `words` fillers, then
    /// the decoy exit label, then the jump — so the jump and the decoy share an
    /// address and the distance the reader must measure is `-4 * words`.
    fn backward_block_jump_text(words: usize) -> String {
        let mut t = String::with_capacity(32 * words + 64);
        t.push_str("परीक्षापर्व२ॱॱ\n");
        for _ in 0..words {
            t.push_str("योगः स्थिर०म् शून्यःन ०न ।\n");
        }
        t.push_str("परीक्षानिर्गमॱॱ\n");
        t.push_str(&format!("लङ्घनम् {ZERO}म् परीक्षापर्व२य् ।"));
        t
    }

    /// **A J-TYPE AIMED AT A BLOCK, WHICH NEITHER TWIN'S CORNERS TOOK (`W-306`).**
    ///
    /// Three sites write `लङ्घनम् शून्यःम् <t>य्`, and only one of them targets
    /// [`exit_label`]; the `Branch` terminator and the relaxation's far jump both
    /// name a BLOCK. Every corner above nevertheless targets the exit label, and
    /// the `.t1` twin was blind the same way — `t1_jump_range_bound.rs` seeds
    /// target `०` for all seven of its cases, so `यन्त्रोत्सर्जनॱयन्त्रशाखादूरपरीक्षा`'s
    /// `लक्ष्यस्थानम् भवति यन्त्रपर्वस्थानकोश अङ्कः लङ्घनलक्ष्यम् अन्तः` had never been
    /// evaluated by a test. There the read is a TABLE INDEXED BY TARGET, which is
    /// exactly the keying class of defect `W-306` found in `यन्त्रशाखास्थानकोश`:
    /// index it by the jump's own ordinal and a far jump reads another row's
    /// address and is certified near. `t1_jump_block_target.rs` pins that side with
    /// a decoy at the wrong index; this is its Rust half, with the decoy spelled as
    /// a second label line in the text.
    ///
    /// All four corners go through the block label, and the bound is the same
    /// half-open `[-(1 << 20), 1 << 20)`: the point is the PATH, not a new bound.
    /// The refusal names `परीक्षापर्व२` and not `परीक्षानिर्गम`, so a reader that
    /// collapsed the two is caught by the message as well as by the verdict.
    #[test]
    fn a_jump_to_a_block_label_is_measured_at_the_block_s_address() {
        let func = block_jumping_function();

        for relax in [false, true] {
            // Forward: 4 * (1 + 262_142) = +1_048_572 stands, one word more does not.
            assert!(
                check_branch_ranges(&func, "परीक्षा", &block_jump_text(262_142), relax).is_ok(),
                "+1_048_572 to a block label is inside ±1 MiB (relax={relax})"
            );
            match check_branch_ranges(&func, "परीक्षा", &block_jump_text(262_143), relax)
            {
                Err(Refusal::JumpOutOfRange {
                    function,
                    target,
                    bytes,
                }) => {
                    assert_eq!(function, "परीक्षा");
                    assert_eq!(
                        target, "परीक्षापर्व२",
                        "the BLOCK label, not the decoy परीक्षानिर्गम sitting at distance ०"
                    );
                    assert_eq!(bytes, 1_048_576);
                }
                other => panic!("relax={relax}: expected JumpOutOfRange, got {other:?}"),
            }

            // Backward: -1_048_576 is the inclusive endpoint, -1_048_580 is not.
            assert!(
                check_branch_ranges(&func, "परीक्षा", &backward_block_jump_text(262_144), relax)
                    .is_ok(),
                "-1_048_576 to a block label STANDS (relax={relax})"
            );
            match check_branch_ranges(&func, "परीक्षा", &backward_block_jump_text(262_145), relax)
            {
                Err(Refusal::JumpOutOfRange { target, bytes, .. }) => {
                    assert_eq!(target, "परीक्षापर्व२", "the BLOCK label, not the decoy");
                    assert_eq!(bytes, -1_048_580, "and the sign is carried");
                }
                other => panic!("relax={relax}: expected JumpOutOfRange, got {other:?}"),
            }
        }
    }

    // ── `W-306`, THE B-TYPE'S OWN REACH ────────────────────────────────────

    /// A `यावत्` with a `यदि` in it, reduced to the three blocks the B-type half
    /// reads: entry branches conditionally into the loop head, and the loop head
    /// branches conditionally BACK to itself. [`jumping_function`] cannot reach
    /// this half at all — its one block ends in `Return`, so it carries no
    /// `CondBranch` for the second loop to measure.
    ///
    /// ONE function serves both directions because the two directions are read
    /// through DIFFERENT blocks, and that is the asymmetry the fixtures below
    /// assert. Forward, the conditional is the first line and the label comes
    /// after it, so the branch belongs to the entry block — which carries no
    /// label of its own (§2.2) and therefore cannot be a target. Backward, the
    /// label is the first line, so `current` has already moved to `पर्व१` by the
    /// time the conditional is read and the branch belongs to `पर्व१`. A
    /// self-edge is the only backward shape a text can hold with one label, and
    /// it is also the commonest one a loop emits.
    fn branching_function() -> Function {
        let mut blocks = HashMap::new();
        blocks.insert(
            BlockId(0),
            Block {
                id: BlockId(0),
                insts: Vec::new(),
                terminator: Some(Terminator::CondBranch(ValueId(0), BlockId(1), BlockId(2))),
            },
        );
        blocks.insert(
            BlockId(1),
            Block {
                id: BlockId(1),
                insts: Vec::new(),
                terminator: Some(Terminator::CondBranch(ValueId(0), BlockId(1), BlockId(2))),
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
        Function {
            name: SymbolId(1),
            blocks,
            entry_block: BlockId(0),
        }
    }

    /// A conditional as the emitter spells one (`{word} {ra}न {rb}त् {t}य् ।`).
    /// The reader does NOT parse the conditional's target — it pairs the branch
    /// with the block the TERMINATOR names — so the operands are the cheapest
    /// well-formed ones and nothing here is ever executed.
    fn cond_line(target: &str) -> String {
        format!("{} {ZERO}न {ZERO}त् {target}य् ।", branch_word(CmpOp::Lt))
    }

    /// `<conditional>` then `words` filler instructions then `पर्व१`'s label: the
    /// branch sits at address ०, the label at `4 * (1 + words)`. The mirror of
    /// [`jump_text`], for the B-type.
    fn branch_text_forward(words: usize) -> String {
        let target = block_label("परीक्षा", BlockId(1));
        let mut t = String::with_capacity(32 * words + 64);
        t.push_str(&cond_line(&target));
        t.push('\n');
        for _ in 0..words {
            t.push_str("योगः स्थिर०म् शून्यःन ०न ।\n");
        }
        t.push_str(&format!("{target}ॱॱ"));
        t
    }

    /// `पर्व१`'s label FIRST, then `words` filler instructions, then the
    /// conditional: the label sits at address ०, the branch at `4 * words`, so
    /// the distance is `-4 * words`. The mirror of [`backward_jump_text`].
    fn branch_text_backward(words: usize) -> String {
        let target = block_label("परीक्षा", BlockId(1));
        let mut t = String::with_capacity(32 * words + 64);
        t.push_str(&format!("{target}ॱॱ\n"));
        for _ in 0..words {
            t.push_str("योगः स्थिर०म् शून्यःन ०न ।\n");
        }
        t.push_str(&cond_line(&target));
        t
    }

    /// **THE B-TYPE'S FORWARD ENDPOINT, AND NOTHING IN `crates/` HAD EVER NAMED
    /// IT.** `check_branch_ranges` spells the conditional's reach `-4096..4096`
    /// — half-open, exactly like the J-type — yet every `BranchOutOfRange` match
    /// in the tree was `{ .. }`, so `bytes`, `from` and `target` were all
    /// unread and both endpoints were free to be wrong by any amount.
    ///
    /// ±4 KiB is 4096, and `4 * (1 + 1022) = 4092` is the last forward distance
    /// that stands; one more filler word is `4096`, the first that does not.
    /// BOTH are asserted: a guard tested only on the refusal would pass with the
    /// bound short by any amount, and the whole corpus is near, so a bound that
    /// is short relaxes routines that need no relaxing and rewrites every image.
    #[test]
    fn a_conditional_past_four_kib_forward_is_refused_and_4092_stands() {
        let func = branching_function();

        let near = branch_text_forward(1022);
        assert!(
            check_branch_ranges(&func, "परीक्षा", &near, false).is_ok(),
            "4092 bytes is inside ±4 KiB and must stand"
        );

        let far = branch_text_forward(1023);
        match check_branch_ranges(&func, "परीक्षा", &far, false) {
            Err(Refusal::BranchOutOfRange {
                function,
                from,
                target,
                bytes,
            }) => {
                assert_eq!(function, "परीक्षा");
                // The branch is the entry block's, and `पर्व१` is the block the
                // TERMINATOR names — not whatever the branch line spelled.
                assert_eq!(from, BlockId(0));
                assert_eq!(target, BlockId(1));
                assert_eq!(bytes, 4096);
            }
            other => panic!("expected BranchOutOfRange, got {other:?}"),
        }
    }

    /// **THE B-TYPE'S NEGATIVE ENDPOINT IS NOT THE POSITIVE ONE MIRRORED**, for
    /// the same reason the J-type's is not: `-4096..4096` is HALF-OPEN, so the
    /// two sides stop at different magnitudes. Backward, `-4096` STANDS and the
    /// first refused is `-4100`, one word further than forward's `4096`.
    ///
    /// A back-edge is where this matters: the forward corners alone cannot tell
    /// `-4096..4096` from `-4092..4096` or from `-4096..=4096`, and a loop whose
    /// body is just under 4 KiB is the shape that lands on the boundary.
    #[test]
    fn a_conditional_reaches_four_kib_backward_exactly_and_no_word_further() {
        let func = branching_function();

        // -4096 = -(1 << 12), the FIRST value the half-open range admits, and
        // the one a symmetric `-4092` bound would refuse.
        let near = branch_text_backward(1024);
        assert!(
            check_branch_ranges(&func, "परीक्षा", &near, false).is_ok(),
            "-4096 is the negative endpoint and it STANDS"
        );

        let far = branch_text_backward(1025);
        match check_branch_ranges(&func, "परीक्षा", &far, false) {
            Err(Refusal::BranchOutOfRange {
                function,
                from,
                target,
                bytes,
            }) => {
                assert_eq!(function, "परीक्षा");
                // Read through `पर्व१` and not through the entry block: the label
                // line precedes the conditional, so `current` had already moved.
                assert_eq!(from, BlockId(1), "a back-edge is measured at ITS block");
                assert_eq!(target, BlockId(1));
                assert_eq!(bytes, -4100, "the refusal reports a BACKWARD distance");
            }
            other => panic!("expected BranchOutOfRange, got {other:?}"),
        }
    }

    /// **THE CASE THAT MUST STILL BE ACCEPTED: `relax = true` DOES NOT MEASURE
    /// THE B-TYPE AT ALL.** `check_branch_ranges` returns `Ok` before the
    /// conditional half whenever `relax` is set, because the relaxed text
    /// carries no far conditional — the far one has already been inverted over a
    /// `लङ्घनम्`, and that jump is what the J-type half measures in BOTH modes.
    ///
    /// This is why the two tests above are `relax = false` only and cannot use
    /// the `for relax in [false, true]` loop the J-type pair uses, and why a
    /// fixture must call the routine DIRECTLY: `emit_module`'s second pass sets
    /// `relax` the moment it sees a `BranchOutOfRange`, so through the public
    /// entry point these four corners are unreachable. Both distances that are
    /// refused above are used here, so the acceptance is the MODE's and not the
    /// distance having quietly become near.
    #[test]
    fn the_relaxed_mode_measures_no_conditional_in_either_direction() {
        let func = branching_function();
        for text in [branch_text_forward(1023), branch_text_backward(1025)] {
            assert!(
                check_branch_ranges(&func, "परीक्षा", &text, false).is_err(),
                "the control: this distance IS refused unrelaxed"
            );
            assert!(
                check_branch_ranges(&func, "परीक्षा", &text, true).is_ok(),
                "relaxed, the B-type half is never reached"
            );
        }
    }
    // ── `W-306`, THE RELAXATION'S OWN FAR JUMP ─────────────────────────────

    /// The text [`lower_cond_branch`] writes when `relax` is true, which no
    /// fixture above builds: the INVERTED conditional over the skip label, the
    /// far `लङ्घनम्` carrying the real target, then the skip label line. The three
    /// sites that write `लङ्घनम् शून्यःम् <t>य्` are the `Branch` terminator, the
    /// `Return` that reaches [`exit_label`], and THIS one — and it is the one a
    /// long routine is most likely to stretch, because the relaxation exists
    /// precisely because the routine was too long for a B-type.
    ///
    /// TWO DECOYS, both at an address the far jump could reach from:
    /// `परीक्षानिर्गम` (the target of the `Return` writer) and `परीक्षापर्व०` (the
    /// BRANCHING block's own label) sit together above the conditional, so a
    /// reader that resolved the far jump to either would measure `ऋण४` — in range —
    /// and accept every corner below.
    ///
    /// `aim` is what the far jump spells. The text always declares
    /// `परीक्षापर्व१ॱॱ` last, so aiming elsewhere makes an UNDECLARED target
    /// without moving a single word.
    fn relaxed_branch_text(words: usize, aim: &str) -> String {
        let block = block_label("परीक्षा", BlockId(0));
        let mut t = String::with_capacity(32 * words + 128);
        t.push_str(&format!("{}ॱॱ\n", exit_label("परीक्षा")));
        t.push_str(&format!("{block}ॱॱ\n"));
        // Inverted: `Lt` relaxed is `Ge`, and it carries the SKIP, not the target.
        t.push_str(&format!(
            "{} {ZERO}न {ZERO}त् {block}अतिक्रमय् ।\n",
            branch_word(inverse_condition(CmpOp::Lt))
        ));
        t.push_str(&format!("लङ्घनम् {ZERO}म् {aim}य् ।\n"));
        t.push_str(&format!("{block}अतिक्रमॱॱ\n"));
        for _ in 0..words {
            t.push_str("योगः स्थिर०म् शून्यःन ०न ।\n");
        }
        t.push_str(&format!("{}ॱॱ", block_label("परीक्षा", BlockId(1))));
        t
    }

    /// The mirror: `परीक्षापर्व१`'s label FIRST at address ०, then `words` fillers,
    /// then both decoys, then the relaxed triple — so the far jump measures
    /// `ऋण४ * (words + १)` and the two decoys sit one word above it, at `ऋण४`.
    fn backward_relaxed_branch_text(words: usize, aim: &str) -> String {
        let block = block_label("परीक्षा", BlockId(0));
        let mut t = String::with_capacity(32 * words + 128);
        t.push_str(&format!("{}ॱॱ\n", block_label("परीक्षा", BlockId(1))));
        for _ in 0..words {
            t.push_str("योगः स्थिर०म् शून्यःन ०न ।\n");
        }
        t.push_str(&format!("{}ॱॱ\n", exit_label("परीक्षा")));
        t.push_str(&format!("{block}ॱॱ\n"));
        t.push_str(&format!(
            "{} {ZERO}न {ZERO}त् {block}अतिक्रमय् ।\n",
            branch_word(inverse_condition(CmpOp::Lt))
        ));
        t.push_str(&format!("लङ्घनम् {ZERO}म् {aim}य् ।\n"));
        t.push_str(&format!("{block}अतिक्रमॱॱ"));
        t
    }

    /// **THE RELAXATION'S FAR JUMP IS MEASURED IN THE MODE THAT EMITS IT, AND
    /// NOTHING WALKED IT ON EITHER SIDE (`W-306`).**
    ///
    /// `relax = true` is exactly the mode in which the B-type half returns early,
    /// so this jump is the ONLY thing left to measure in a relaxed routine — and
    /// it is the jump the relaxation introduced. A J-type half that ran only
    /// unrelaxed would leave the relaxed routine with no guard at all, which is
    /// the wrap `W-306` closed: the relaxation had merely moved the unmeasured
    /// instruction one line along.
    ///
    /// Each corner carries ITS OWN control. The near pair is refused when the
    /// same text is read with `relax = false` — `BranchOutOfRange`, because the
    /// conditional is now measured and it is far — so the acceptance under
    /// `relax = true` is the MODE's, and not the reader having stopped reaching
    /// the text. The bound is the same half-open `[ऋण१ MiB, १ MiB)` the other two
    /// writers get; the point of the fixture is the PATH and the decoys.
    #[test]
    fn the_relaxation_s_far_jump_is_measured_in_the_relaxed_mode() {
        let func = branching_function();
        let aim = block_label("परीक्षा", BlockId(1));

        // Forward: the far jump sits at address ४ (the conditional is word ०) and
        // `परीक्षापर्व१` at `४ * (words + २)`, so the distance is `४ * (words + १)`.
        // 4 * 262_143 = 1_048_572, the last that stands.
        let near = relaxed_branch_text(262_142, &aim);
        assert!(
            check_branch_ranges(&func, "परीक्षा", &near, true).is_ok(),
            "+1_048_572 from the relaxation's far jump is inside ±1 MiB"
        );
        assert!(
            matches!(
                check_branch_ranges(&func, "परीक्षा", &near, false),
                Err(Refusal::BranchOutOfRange { .. })
            ),
            "the control: unrelaxed, this text's conditional IS measured and IS far"
        );

        let far = relaxed_branch_text(262_143, &aim);
        match check_branch_ranges(&func, "परीक्षा", &far, true) {
            Err(Refusal::JumpOutOfRange {
                function,
                target,
                bytes,
            }) => {
                assert_eq!(function, "परीक्षा");
                assert_eq!(
                    target, aim,
                    "the far target, not the decoy परीक्षानिर्गम nor the branching block's own label"
                );
                assert_eq!(bytes, 1_048_576);
            }
            other => panic!("expected JumpOutOfRange, got {other:?}"),
        }

        // Backward: `ऋण४ * (words + १)`, so -1_048_576 is `words = 262_143` — the
        // inclusive endpoint — and -1_048_580 is one word further.
        let near_back = backward_relaxed_branch_text(262_143, &aim);
        assert!(
            check_branch_ranges(&func, "परीक्षा", &near_back, true).is_ok(),
            "-1_048_576 is the negative endpoint and it STANDS"
        );
        assert!(
            matches!(
                check_branch_ranges(&func, "परीक्षा", &near_back, false),
                Err(Refusal::BranchOutOfRange { .. })
            ),
            "the control: unrelaxed, the back-edge conditional IS measured and IS far"
        );

        let far_back = backward_relaxed_branch_text(262_144, &aim);
        match check_branch_ranges(&func, "परीक्षा", &far_back, true) {
            Err(Refusal::JumpOutOfRange { target, bytes, .. }) => {
                assert_eq!(target, aim, "the far target, not either decoy at ऋण४");
                assert_eq!(bytes, -1_048_580, "and the sign is carried");
            }
            other => panic!("expected JumpOutOfRange, got {other:?}"),
        }
    }

    /// **THE SKIP LABEL LINE IS NOT AN INSTRUCTION WORD, AND IT SITS INSIDE THE
    /// DISTANCE IT WOULD CORRUPT.** `…अतिक्रम` is written two lines below the
    /// conditional, BETWEEN the far jump and its target, so a reader that counted
    /// a label line as a word would read every relaxed routine's far distance ४
    /// bytes long — enough to refuse a routine that is exactly in range, and
    /// `emit_function` has no third pass to fall back to. The `.t1` twin states
    /// the same invariant as a comment on `यन्त्रचिह्नान्तः` and never evaluated it.
    ///
    /// Asserted by IDENTITY and not by a verdict: the same text with the skip
    /// label line deleted must report the SAME `bytes`. A verdict-only assertion
    /// would pass with the label counted and the bound off to match.
    #[test]
    fn the_skip_label_line_does_not_lengthen_the_far_distance() {
        let func = branching_function();
        let aim = block_label("परीक्षा", BlockId(1));
        let with = relaxed_branch_text(262_143, &aim);
        let skip_line = format!("{}अतिक्रमॱॱ\n", block_label("परीक्षा", BlockId(0)));
        assert!(
            with.contains(&skip_line),
            "the fixture does carry the label"
        );
        let without = with.replace(&skip_line, "");

        let bytes_of = |t: &str| match check_branch_ranges(&func, "परीक्षा", t, true)
        {
            Err(Refusal::JumpOutOfRange { bytes, .. }) => bytes,
            other => panic!("expected JumpOutOfRange, got {other:?}"),
        };
        assert_eq!(bytes_of(&with), 1_048_576);
        assert_eq!(
            bytes_of(&with),
            bytes_of(&without),
            "deleting the skip label line moves no address"
        );

        // AND THE CONTROL that this text can move at all: one filler word fewer
        // is `+1_048_572`, which STANDS — so the identity above is not two reads
        // of a distance that was pinned by something else.
        assert!(
            check_branch_ranges(&func, "परीक्षा", &relaxed_branch_text(262_142, &aim), true).is_ok(),
            "one word shorter and the same shape is accepted"
        );
    }

    /// **THE CASE THAT MUST STILL BE REFUSED TO REFUSE — i.e. SKIPPED.** A
    /// relaxed far jump whose target this text declares no label line for is not
    /// this routine's distance to measure: `emit_call` writes a CALL through
    /// `पुनःस्थानम्`, and the startup spells names no `पर्व` owns. The aim here is a
    /// block label the text never declares, at the distance that IS refused when
    /// the aim is declared — so the acceptance cannot be the distance being short.
    #[test]
    fn a_relaxed_far_jump_at_an_undeclared_target_is_skipped() {
        let func = branching_function();
        let undeclared = block_label("परीक्षा", BlockId(7));
        for text in [
            relaxed_branch_text(262_143, &undeclared),
            backward_relaxed_branch_text(262_144, &undeclared),
        ] {
            assert!(
                !text.contains(&format!("{undeclared}ॱॱ")),
                "the fixture declares no label line for {undeclared}"
            );
            assert!(
                check_branch_ranges(&func, "परीक्षा", &text, true).is_ok(),
                "an undeclared target is skipped, not refused"
            );
        }

        // AND THE CONTROL: the same two texts aiming at the label the fixture DOES
        // declare are refused, so the acceptances are the TARGET's and not the
        // relaxed mode having quietly stopped measuring the J-type.
        let aim = block_label("परीक्षा", BlockId(1));
        for text in [
            relaxed_branch_text(262_143, &aim),
            backward_relaxed_branch_text(262_144, &aim),
        ] {
            assert!(matches!(
                check_branch_ranges(&func, "परीक्षा", &text, true),
                Err(Refusal::JumpOutOfRange { .. })
            ));
        }
    }
}
