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

use crate::t1::ast::SymbolId;
use crate::t1::ir::*;
use crate::t1::regalloc::{AllocationMap, Location, allocate_registers};
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

/// The stack the image carries (§2.6): 64 KiB reserved in `ॱदत्त`.
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
        let who = format!("{m} ॱ {n}");
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
/// slots (`Load`/`Store`), then the saved `स्थिर`s the routine uses, then
/// `पुनःस्थानम्` at the top; rounded up to 16.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Total bytes; what the prologue subtracts from `स्तूपसूचकः`.
    pub bytes: i64,
    /// The `स्थिर` registers saved, in allocator-number order, each with its offset.
    pub saved: Vec<(u8, i64)>,
    /// `पुनःस्थानम्`'s offset: `bytes - 8`.
    pub ra_offset: i64,
    /// Spill slot `k` lives at `8k`.
    pub num_spills: usize,
    /// Local slot `k` lives at `8(num_spills + k)` — `W-245`.
    pub num_locals: usize,
}

/// How many local slots a routine addresses: one past the highest `Load`/`Store`
/// slot, or none (`W-245`).
#[must_use]
pub fn count_locals(func: &Function) -> usize {
    func.blocks
        .values()
        .flat_map(|b| b.insts.iter())
        .filter_map(|(_, i)| match i {
            Instruction::Load(k) | Instruction::Store(k, _) => Some(*k + 1),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

/// Lay out the frame from the allocation (§2.4) and the routine's local count.
#[must_use]
pub fn frame_layout(alloc: &AllocationMap, num_locals: usize) -> Frame {
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
    let slot_bytes = 8 * (alloc.num_spills + num_locals) as i64;
    let saved: Vec<(u8, i64)> = used
        .iter()
        .enumerate()
        .map(|(i, r)| (*r, slot_bytes + 8 * i as i64))
        .collect();
    let raw = slot_bytes + 8 * saved.len() as i64 + 8;
    let bytes = (raw + 15) / 16 * 16;
    Frame {
        bytes,
        saved,
        ra_offset: bytes - 8,
        num_spills: alloc.num_spills,
        num_locals,
    }
}

// --- the emitter ---------------------------------------------------------------------

/// One routine's emission state.
struct Routine<'a> {
    label: String,
    func: &'a Function,
    alloc: &'a AllocationMap,
    frame: Frame,
    out: String,
}

impl Routine<'_> {
    fn line(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn location(&self, v: ValueId) -> Location {
        *self
            .alloc
            .locations
            .get(&v)
            .unwrap_or_else(|| panic!("{}: {v:?} has no location", self.label))
    }

    /// The register a value can be READ from, loading a spilled one into
    /// `क्षणिक<scratch>` first. `extra` is what the stack pointer has been moved
    /// by since the prologue (the stack-argument push of a `Call`).
    fn read(&mut self, v: ValueId, scratch: u8, extra: i64) -> String {
        match self.location(v) {
            Location::Register(r) => register_name(r),
            Location::Spill(k) => {
                let t = temp(scratch);
                let off = devanagari(8 * k as i64 + extra);
                self.line(&format!("आहारः {t}म् {SP}त् {off}न ।"));
                t
            }
        }
    }

    /// The register a value is WRITTEN into, and the store that follows for a
    /// spilled one. The caller emits the defining instruction between the two.
    fn write(&self, v: ValueId, scratch: u8) -> (String, Option<String>) {
        match self.location(v) {
            Location::Register(r) => (register_name(r), None),
            Location::Spill(k) => {
                let t = temp(scratch);
                let off = devanagari(8 * k as i64);
                (t.clone(), Some(format!("निधानम् {SP}य् {off}न {t}न ।")))
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
        for (r, off) in self.frame.saved.clone() {
            self.line(&format!(
                "निधानम् {SP}य् {}न {}न ।",
                devanagari(off),
                register_name(r)
            ));
        }
    }

    fn epilogue(&mut self) {
        self.line(&format!("{}ॱॱ", exit_label(&self.label)));
        self.line(&format!(
            "आहारः {RA}म् {SP}त् {}न ।",
            devanagari(self.frame.ra_offset)
        ));
        for (r, off) in self.frame.saved.clone() {
            self.line(&format!(
                "आहारः {}म् {SP}त् {}न ।",
                register_name(r),
                devanagari(off)
            ));
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
) -> Result<(), Refusal> {
    let target = routine_label(names, callee)?;
    let on_stack = args.len().saturating_sub(8);
    let adjust = 16 * ((on_stack as i64 + 1) / 2);
    if adjust > 0 {
        rt.line(&format!("योगः {SP}म् {SP}न {}न ।", devanagari(-adjust)));
        for (j, a) in args.iter().enumerate().skip(8) {
            let src = rt.read(*a, 0, adjust);
            let off = devanagari(8 * (j as i64 - 8));
            rt.line(&format!("निधानम् {SP}य् {off}न {src}न ।"));
        }
    }
    for (j, a) in args.iter().enumerate().take(8) {
        let src = rt.read(*a, 0, adjust);
        rt.line(&format!("योगः {}म् {src}न ०न ।", arg(j)));
    }
    rt.line(&format!("लङ्घनम् {RA}म् {target}य् ।"));
    if adjust > 0 {
        rt.line(&format!("योगः {SP}म् {SP}न {}न ।", devanagari(adjust)));
    }
    let (rd, store) = rt.write(v, 2);
    rt.line(&format!("योगः {rd}म् {}न ०न ।", arg(0)));
    if let Some(s) = store {
        rt.line(&s);
    }
    Ok(())
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

/// The verb of each binary kind, as the ISA has it. `Add`/`Sub` were the two
/// the emitter began with; `W-245` added the eight ADR-0032 froze.
fn binary_verb(inst: &Instruction) -> Option<&'static str> {
    Some(match inst {
        Instruction::Add(..) => "योगः",
        Instruction::Sub(..) => "वियोगः",
        Instruction::Mul(..) => "गुणनम्",
        Instruction::Div(..) => "भागः",
        Instruction::Rem(..) => "शेषः",
        Instruction::Shl(..) => "वामसरणम्",
        // arithmetic since 2026-09-14: the source operator `दक्षिणसृ` is the
        // interpreter's sign-preserving shift and the corpus's hi/lo splits rely on it
        // (see yantrotsarjana.t1's margin at the same line).
        Instruction::Shr(..) => "सचिह्नदक्षिणसरणम्",
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
fn lower_cond_branch(
    rt: &mut Routine<'_>,
    c: ValueId,
    then: BlockId,
    els: BlockId,
    next: Option<BlockId>,
    fused: Option<(CmpOp, ValueId, ValueId)>,
) {
    let label = rt.label.clone();
    match fused {
        Some((op, a, b)) => {
            let ra = rt.read(a, 0, 0);
            let rb = rt.read(b, 1, 0);
            rt.line(&format!(
                "{} {ra}न {rb}त् {}य् ।",
                branch_word(op),
                block_label(&label, then)
            ));
        }
        None => {
            let cond = rt.read(c, 0, 0);
            rt.line(&format!(
                "विषमलङ्घनम् {cond}न {ZERO}त् {}य् ।",
                block_label(&label, then)
            ));
        }
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
                let src = rt.read(*x, 0, 0);
                rt.line(&format!("योगः {}म् {src}न ०न ।", arg(0)));
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
        Some(Terminator::CondBranch(c, t, e)) => lower_cond_branch(rt, *c, *t, *e, next, fused),
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
                Instruction::Call(..) => called = true,
                Instruction::Param(i) if id != func.entry_block => {
                    return Err(Refusal::ParamOutsideEntry {
                        function: label.to_string(),
                        block: id,
                        param: *i,
                    });
                }
                Instruction::Param(i) if called => {
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
    Ok(())
}

/// One routine as T0 text: prologue, blocks in id order with fallthrough, one
/// epilogue (§2.4, §2.5, §2.7). Appends to `pool` the constants it could not
/// materialise inline.
///
/// # Errors
/// A [`Refusal`], by name, before any text of the routine is returned.
pub fn emit_function(
    func: &Function,
    names: &Names,
    alloc: &AllocationMap,
    pool: &mut Vec<i64>,
    strings: &mut Vec<Vec<u8>>,
) -> Result<String, Refusal> {
    let label = routine_label(names, func.name)?;
    verify(func, &label)?;
    let frame = frame_layout(alloc, count_locals(func));
    let mut order: Vec<BlockId> = func.blocks.keys().copied().collect();
    order.sort_by_key(|b| b.0);
    let max_param = func
        .blocks
        .values()
        .flat_map(|b| b.insts.iter())
        .filter_map(|(_, i)| match i {
            Instruction::Param(i) => Some(*i),
            _ => None,
        })
        .max();
    let highest_offset = match max_param {
        Some(i) if i >= 8 => frame.bytes + 8 * (i as i64 - 8),
        _ => frame.bytes,
    };
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

    let mut rt = Routine {
        label: label.clone(),
        func,
        alloc,
        frame,
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
                | Instruction::Rem(a, b)
                | Instruction::Shl(a, b)
                | Instruction::Shr(a, b)
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
                    let off = devanagari(8 * (rt.frame.num_spills + k) as i64);
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
                    let off = devanagari(i64::try_from(*off).unwrap_or(i64::MAX));
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
                Instruction::Store(k, x) => {
                    let src = rt.read(*x, 0, 0);
                    let off = devanagari(8 * (rt.frame.num_spills + k) as i64);
                    rt.line(&format!("निधानम् {SP}य् {off}न {src}न ।"));
                }
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
                // `य्` — अधिकरण, the place written TO, and NOT the load's `त्`.
                // Writing `{addr}त्` here assembles to "`स्थिर०` and `स्थिर२`
                // both claim अपादान", the same refusal `LoadIndex` above records
                // for its add. The ISA assigns operands by role, so a store is
                // not a load with the arrow reversed.
                //
                // NO RESULT VALUE, hence no `rt.write` — which is exactly why
                // this kind is invisible to DCE unless `is_side_effecting` names
                // it. See `a_store_at_survives_dead_code_elimination`.
                Instruction::StoreAt(a, x) => {
                    let addr = rt.read(*a, 0, 0);
                    let src = rt.read(*x, 1, 0);
                    rt.line(&format!("निधानम् {addr}य् ०न {src}न ।"));
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
                Instruction::Param(i) => {
                    let (rd, store) = rt.write(*v, 2);
                    if *i < 8 {
                        rt.line(&format!("योगः {rd}म् {}न ०न ।", arg(*i)));
                    } else {
                        // The caller left it just above this frame (§2.5).
                        let off = devanagari(rt.frame.bytes + 8 * (*i as i64 - 8));
                        rt.line(&format!("आहारः {rd}म् {SP}त् {off}न ।"));
                    }
                    if let Some(s) = store {
                        rt.line(&s);
                    }
                }
                Instruction::Call(callee, args) => emit_call(&mut rt, *v, *callee, args, names)?,
            }
        }
        emit_terminator(&mut rt, block, next, fused)?;
    }
    rt.epilogue();
    check_branch_ranges(func, &label, &rt.out)?;
    Ok(rt.out)
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
fn check_branch_ranges(func: &Function, label: &str, text: &str) -> Result<(), Refusal> {
    let lines: Vec<&str> = text.lines().collect();
    let address_of = |line: usize| -> i64 {
        // Neither a label nor a directive (`॥ … ॥`, the exported-label line) is an
        // instruction word.
        4 * lines[..line]
            .iter()
            .filter(|l| !l.ends_with("ॱॱ") && !l.starts_with('॥'))
            .count() as i64
    };
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
/// with no arguments, and the entry's result as the finisher status.
#[must_use]
pub fn emit_startup(entry: Option<&str>) -> String {
    emit_startup_with_records(entry, false)
}

/// [`emit_startup`], with the record region emitted only when the image needs
/// it. `W-284`.
///
/// THE REGION IS NOT UNCONDITIONAL, AND THE FIXTURES ARE WHY: a program that
/// allocates nothing must produce THE SAME OCTETS it produced before this row.
/// `॥ स्थानम् ॥` in `ॱदत्त` materialises bytes — the stack above it is how an
/// image comes to 65536 — so an unconditional cursor moves every image by one
/// word and an unconditional region by four megabytes of address space against
/// `yantra::DEFAULT_RAM`'s twenty. A FEATURE THAT COSTS A PROGRAM THAT DOES NOT
/// USE IT IS CHARGED TO THE WRONG ACCOUNT.
///
/// The `records` flag comes from [`module_allocates`], and its `.t1` twin
/// `यन्त्ररचनामस्ति` must answer the same question the same way: the two halves
/// diverge here on a SECTION DIRECTIVE — `॥ कोष्ठकम् ॱरिक्त ॥` against
/// `॥ कोष्ठकम् ॱदत्त ॥` — which is a whole-image difference from one bool.
#[must_use]
pub fn emit_startup_with_records(entry: Option<&str>, records: bool) -> String {
    let mut s = String::new();
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
    let _ = writeln!(s, "॥ वैश्विकम् यन्त्रारम्भ ॥");
    let _ = writeln!(s, "यन्त्रारम्भॱॱ");
    let _ = writeln!(s, "स्थानसापेक्षयोगः {SP}म् स्तूपान्तःॱउपरिन ।");
    let _ = writeln!(s, "योगः {SP}म् {SP}न स्तूपान्तःॱअधःन ।");
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
    let _ = writeln!(s, "यन्त्रसमाप्तिॱॱ");
    let _ = writeln!(s, "उपरिभारः {}म् {FINISHER_HI}न ।", temp(4));
    let _ = writeln!(s, "निधानम्ॱअ३२ {}य् ०न {}न ।", temp(4), arg(0));
    let _ = writeln!(s, "यन्त्रचक्रॱॱ");
    let _ = writeln!(s, "लङ्घनम् {ZERO}म् यन्त्रचक्रय् ।");
    s
}

/// THE STARTUP OBJECT (`W-243`, research/25 §5 R6): the stub and the stack it
/// addresses, as one T0 text of its own — one per image, linked FIRST so that
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
    let mut s = emit_startup_with_records(entry, records);
    let _ = writeln!(s, "॥ कोष्ठकम् ॱदत्त ॥");
    let _ = writeln!(s, "॥ संरेखः १६ ॥");
    let _ = writeln!(s, "स्तूपःॱॱ");
    let _ = writeln!(s, "॥ स्थानम् {} ॥", devanagari(STACK_BYTES as i64));
    let _ = writeln!(s, "स्तूपान्तःॱॱ");
    s
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
            // no escaping and say what is there.
            //
            // AN EMPTY LITERAL GETS ITS LABEL AND NO DIRECTIVE. `अष्टकाः` takes
            // `1+` operands (spec/directives.tsv), so `॥ अष्टकाः ॥` is not a
            // thing that can be written; the label alone is the right answer
            // anyway, since the address of zero octets is where the next thing
            // starts and an empty slice has nothing to point at.
            if b.is_empty() {
                continue;
            }
            let octets: Vec<String> = b.iter().map(|o| devanagari(i64::from(*o))).collect();
            let _ = writeln!(s, "॥ अष्टकाः {} ॥", octets.join(" "));
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
    check_labels(module)?;
    let entry_label = match module.entry {
        Some(sym) => {
            let label = routine_label(&module.names, sym)?;
            if let Some(f) = module.functions.iter().find(|f| f.name == sym) {
                let params = f
                    .blocks
                    .values()
                    .flat_map(|b| b.insts.iter())
                    .filter(|(_, i)| matches!(i, Instruction::Param(_)))
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
    let mut routines = String::new();
    for func in &module.functions {
        let alloc = allocate_registers(func, ALLOCATABLE);
        routines.push_str(&emit_function(
            func,
            &module.names,
            &alloc,
            &mut pool,
            &mut strings,
        )?);
    }
    // The entry's label is the startup object's business (`emit_startup_object`);
    // it was computed above for the parameter check.
    let _ = entry_label;
    let mut out = routines;
    out.push_str(&emit_data(&pool, &strings, &module.globals));
    Ok(out)
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
    use crate::vishlesana::{decode_at, reassemble};

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
        assert_eq!(
            image.data.len() as u64,
            STACK_BYTES,
            "the stack is in the image; no pool"
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
        let frame = frame_layout(&alloc, count_locals(&f));
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
}
