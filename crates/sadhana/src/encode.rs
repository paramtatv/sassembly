//! RV64GC encoder — instruction records to machine code.
//!
//! Every table it reads was derived from the assembler rather than written
//! down: `spec/encodings-riscv64.tsv` gives a pattern, a mask and a
//! value-bit-to-encoding-bit map per operand (`B-037`, `B-038`), and
//! `spec/registers-riscv64.tsv` gives register numbers read back out of real
//! encodings (`B-005`). **No RISC-V format table exists in this crate.**
//!
//! # How a kāraka finds its field
//!
//! The parser produces operands tagged by role, not by position, so the encoder
//! must decide which machine field each role belongs in. It does that from the
//! masks, because the three register fields sit at fixed positions:
//!
//! | mask | field |
//! |---|---|
//! | `0x00000f80` | `rd`, bits 11..7 |
//! | `0x000f8000` | `rs1`, bits 19..15 |
//! | `0x01f00000` | `rs2`, bits 24..20 |
//!
//! That is enough to separate a load from a store without being told which is
//! which. `lb`'s first slot carries `rd`'s mask and `sb`'s carries `rs2`'s — so
//! a slot holding `rd` is a destination and one holding `rs2` is a source
//! value, and the कर्म / करण distinction lands correctly by construction.
//!
//! # Selecting among a family's encodings
//!
//! `योगः` covers nine encodings. The candidate is chosen by matching the
//! *shape* of what was written — how many register operands, whether an
//! immediate is present — against each candidate's slots, then by width. Nothing
//! is hardcoded per family.

use crate::lex::Karaka;
use crate::nidana::{Diagnostic, Language};
use crate::parse::{Instruction, Program};
use std::collections::BTreeMap;

const ENCODINGS: &str = include_str!("../../../spec/encodings-riscv64.tsv");
const REGISTERS: &str = include_str!("../../../spec/registers-riscv64.tsv");
/// What `riscv64-elf-as` compresses, and the coincidence each form needs.
const COMPRESSION: &str = include_str!("../../../spec/compression-choices.tsv");
const FENCE_DOMAINS: &str = include_str!("../../../spec/fence-domains-riscv64.tsv");

/// The three register fields, identified by where they sit.
const RD: u32 = 0x0000_0f80;
const RS1: u32 = 0x000f_8000;
const RS2: u32 = 0x01f0_0000;
/// The fourth register of an R4-type, which only the fused multiply-adds have.
const RS3: u32 = 0xf800_0000;

/// One operand slot of one encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    /// `reg`, `freg`, `imm`, `label` or `fixed`.
    pub kind: String,
    /// Which encoding bits this slot occupies.
    pub mask: u32,
    /// `(value bit, encoding bit)` pairs.
    pub map: Vec<(u32, u32)>,
    /// The lowest value the field accepts.
    ///
    /// Zero for every 32-bit field. The compressed three-bit register fields
    /// reach only `x8`..`x15`, so theirs is 8, and `c.add`'s is 1 because `x0`
    /// is not encodable there. Without it the probe could find no baseline that
    /// assembled and 40 compressed slots had a mask with no map at all — a
    /// field nothing could place a value into (`B-058b2b1`).
    pub bias: u32,
}

/// Whether a slot kind names a register.
fn is_register(kind: &str) -> bool {
    kind == "reg" || kind == "freg"
}

/// Whether two slot kinds can hold the same thing.
///
/// A register and an immediate are both numbers and are not interchangeable:
/// putting `s1` where `c.addi` wants its immediate produces a real instruction
/// that adds nine.
fn same_kind(a: &str, b: &str) -> bool {
    is_register(a) == is_register(b) && (a == "disp") == (b == "disp")
}

/// Whether a slot kind holds an immediate, signed or not.
///
/// `simm` is derived rather than declared: `tools/gen-encodings.py` asks the
/// assembler whether the field accepts `-1`. `addi` and every store offset do;
/// a shift amount and `lui` do not, because a count is not a number.
#[must_use]
fn is_immediate(kind: &str) -> bool {
    kind == "imm" || kind == "simm"
}

impl Slot {
    /// Place a value into this slot's bits.
    ///
    /// Walking the derived map is the whole of field placement: no shifting by
    /// a remembered offset, no special case for the scattered S-type or
    /// compressed immediates. A value bit with no mapping is dropped, which is
    /// how an out-of-range immediate loses its high bits — [`fits`] is what
    /// catches that before it happens.
    #[must_use]
    pub fn place(&self, value: u64) -> u32 {
        let mut word = 0u32;
        for (from, to) in &self.map {
            if value >> from & 1 == 1 {
                word |= 1 << to;
            }
        }
        word
    }

    /// Whether every set bit of `value` has somewhere to go.
    ///
    /// Silently truncating is the failure this guards: an immediate too wide
    /// for its field would otherwise encode as a different, valid number.
    #[must_use]
    pub fn fits(&self, value: u64) -> bool {
        let width = self.map.iter().map(|(f, _)| *f).max().map_or(0, |m| m + 1);

        // A biased field's range runs from its lowest accepted value, not from
        // zero. `c.and`'s registers are `x8`..`x15` in three bits: checking
        // `value < 8` rejected `x8` itself, so every register-register
        // compressed form was refused as out of range.
        if self.bias != 0 && (value as i64) >= 0 {
            let span = 1u64 << width;
            return value >= u64::from(self.bias) && value < u64::from(self.bias) + span;
        }
        // Two's complement: a negative value is in range if its sign-extension
        // is consistent, which for the widths here means the discarded bits are
        // all copies of the top mapped bit.
        let signed = value as i64;

        // A DISPLACEMENT is signed, and its top mapped bit is the sign. Judging
        // a positive one by the unsigned range accepts values that then encode
        // as negative: a branch 4096 bytes forward became 0x800000e3, a branch
        // 4096 bytes BACKWARD, and the corpus could not catch it because a
        // generator that emitted an out-of-range branch would be measuring GNU
        // `as`'s long-branch rewrite instead (`B-012`).
        //
        // Immediates are not treated this way here because their sign bit is
        // not mapped at all — the positive-only probe never reached it, exactly
        // as it did not reach imm[12] before `B-055`. That makes `fits` reject
        // 2048 for `addi`, which is the right answer for the wrong reason, and
        // it is `B-061`.
        // A displacement and a signed immediate are both two's complement, so
        // their top mapped bit is a sign and a positive value must leave it
        // clear. Judging one by the unsigned range is how a branch 4096 bytes
        // forward came to encode as one 4096 bytes backward (`B-012`), and it
        // would now do the same to `addi` — whose sign bit exists only since
        // the probe learned to go negative (`B-061`).
        if self.kind == "disp" || self.kind == "simm" {
            let limit = 1i64 << (width - 1);
            return signed >= -limit && signed < limit;
        }

        if signed < 0 {
            let limit = 1i64 << (width - 1);
            signed >= -limit
        } else {
            width >= 64 || value < (1u64 << width)
        }
    }
}

/// A reference the assembler could not resolve, for the linker to finish.
///
/// `encode_at` errors on an undefined name because an executable has nowhere to
/// put the question. An **object** does: the field is written as zero and this
/// says which name and which relocation type will fill it (`B-069b2`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// The name that was not defined here.
    pub name: String,
    /// A type from `spec/relocations-riscv64.tsv`, by name.
    pub kind: &'static str,
    /// Byte offset into the section of the field to patch.
    pub at: u32,
    /// Which section that offset counts from.
    pub section: crate::kosha::RelSection,
}

/// One candidate encoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encoding {
    /// RISC-V mnemonic, e.g. `addi`.
    pub insn: String,
    /// Sassembly family key, e.g. `add`.
    pub family: String,
    /// Fixed bits.
    pub pattern: u32,
    /// Which bits the pattern constrains.
    ///
    /// The encoder does not use this — it starts from the pattern and ORs
    /// operands in — which is exactly how `beq` spent four cycles with a mask
    /// that swallowed its own displacement. `विश्लेषणम्` decodes by it, so the
    /// round trip in `tests/roundtrip.rs` is the first thing that reads it.
    pub mask: u32,
    /// 16 or 32.
    pub bits: u32,
    /// Operand slots in written order.
    pub slots: Vec<Slot>,
    /// Operand size in bits, when the instruction has one.
    ///
    /// `None` for anything whose name does not state a width — `add` and
    /// `ecall` alike. This is the one column of the encoding table that is a
    /// naming convention rather than a measurement; see `tools/gen-encodings.py`.
    pub width: Option<u32>,
    /// The pair of types a conversion moves between, written then read.
    ///
    /// `None` for everything that is not an `fcvt`. This is the one thing no
    /// operand can say: `fcvt.w.s` and `fcvt.w.d` both write an integer
    /// register from a float one, and only what they READ differs (`B-075`).
    pub converts: Option<(String, String)>,
    /// Whether this encoding's operands are ordering-domain SETS (`B-107`).
    ///
    /// True for `fence` and nothing else, and read from the table's
    /// `operand_shape` column rather than written here: that column records
    /// what `riscv64-elf-as` was probed with, and `iorw, iorw` is the one shape
    /// that is a set of names instead of a register or a number.
    ///
    /// Without it a domain name is a domain name everywhere, so a label called
    /// `पठनम्` — "reading", an ordinary word for an ordinary routine — could be
    /// defined and never referenced. The reading is chosen by what the family
    /// can take, which is the same principle as every other selection here.
    pub takes_domains: bool,
}

impl Encoding {
    fn slot_with(&self, mask: u32) -> Option<&Slot> {
        self.slots.iter().find(|s| s.mask == mask)
    }
    fn immediate(&self) -> Option<&Slot> {
        self.slots.iter().find(|s| is_immediate(&s.kind))
    }
    /// The `n`-th immediate slot in written order.
    ///
    /// `fence` has two and they are not interchangeable: the first is the
    /// predecessor set and the second the successor, in the order the shape
    /// `iorw, iorw` was probed.
    fn nth_immediate(&self, n: usize) -> Option<&Slot> {
        self.slots.iter().filter(|s| is_immediate(&s.kind)).nth(n)
    }
    fn register_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| s.kind == "reg" || s.kind == "freg")
            .count()
    }
}

/// The derived encoding table, parsed once.
///
/// Every decode, reassemble and `compressed_at` asks for the whole table, and
/// each call used to parse `spec/encodings-riscv64.tsv` again from its text —
/// which is how `W-211`'s round trip over 8,097 instructions became a
/// 29-minute census (`W-233`). The table is a compile-time constant, so it is
/// parsed on first use and kept.
#[must_use]
pub fn encodings() -> &'static [Encoding] {
    static TABLE: std::sync::OnceLock<Vec<Encoding>> = std::sync::OnceLock::new();
    TABLE.get_or_init(parse_encodings)
}

/// Parse the derived encoding table from its text.
fn parse_encodings() -> Vec<Encoding> {
    ENCODINGS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("insn\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() < 7 {
                return None;
            }
            let slots = if f[6] == "(none)" {
                Vec::new()
            } else {
                f[6].split('|')
                    .filter_map(|s| {
                        let p: Vec<&str> = s.split(':').collect();
                        let mask =
                            u32::from_str_radix(p.get(1)?.trim_start_matches("0x"), 16).ok()?;
                        let map = p
                            .get(2)
                            .map(|m| {
                                m.split(';')
                                    .filter(|x| !x.is_empty())
                                    .filter_map(|x| {
                                        let (a, b) = x.split_once('>')?;
                                        Some((a.parse().ok()?, b.parse().ok()?))
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        Some(Slot {
                            kind: p[0].to_string(),
                            mask,
                            map,
                            bias: p.get(3).and_then(|b| b.parse().ok()).unwrap_or(0),
                        })
                    })
                    .collect()
            };
            Some(Encoding {
                insn: f[0].into(),
                family: f[1].into(),
                pattern: u32::from_str_radix(f[3].trim_start_matches("0x"), 16).ok()?,
                mask: u32::from_str_radix(f[4].trim_start_matches("0x"), 16).ok()?,
                bits: f[5].parse().ok()?,
                slots,
                width: f.get(7).and_then(|w| w.parse().ok()),
                converts: f
                    .get(8)
                    .and_then(|c| c.split_once(','))
                    .map(|(a, b)| (a.to_string(), b.to_string())),
                takes_domains: f
                    .get(9)
                    .is_some_and(|s| s.split(',').any(|o| o.trim() == "iorw")),
            })
        })
        .collect()
}

/// Resolve a Sassembly register name to its number and class.
#[must_use]
pub fn register(name: &str) -> Option<(u32, bool)> {
    REGISTERS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("devanagari\t"))
        .find_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 4 && f[0] == name)
                .then(|| Some((f[2].parse().ok()?, f[3] == "float")))
                .flatten()
        })
}

/// Resolve a fence ordering-domain set — task `B-045`.
///
/// `fence`'s two operands are each a SET drawn from four domains: device input,
/// device output, memory read, memory write. A set is written as names joined
/// by `ऽ`, the separator ADR-0003 ratified and nothing had used:
///
/// ```text
/// स्मृतिबन्धः पठनम्ऽलेखनम्त् पठनम्ऽलेखनम्य् ।     fence rw, rw
/// ```
///
/// The bit each domain occupies is read out of the assembler by
/// `tools/gen-fence-domains.py`, never recalled — a wrong bit here produces a
/// valid fence that orders the wrong operations, and that does not fail. It
/// races, sometimes, on another machine.
///
/// Naming a domain twice is rejected rather than silently absorbed: `पठनम्ऽपठनम्`
/// is a mistake, and OR-ing it away would hide it.
#[must_use]
fn domain_set(base: &str) -> Option<u64> {
    let mut bits = 0u64;
    for part in base.split('ऽ') {
        let b = FENCE_DOMAINS
            .lines()
            .filter(|l| !l.starts_with('#') && !l.starts_with("devanagari\t"))
            .find_map(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                (f.len() >= 3 && f[0] == part)
                    .then(|| f[2].parse::<u64>().ok())
                    .flatten()
            })?;
        if bits & b != 0 {
            return None; // the same domain named twice
        }
        bits |= b;
    }
    (bits != 0).then_some(bits)
}

/// Why encoding failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodeError {
    /// 1-based line.
    pub line: usize,
    /// Which diagnostic this is, from `spec/diagnostics.tsv`.
    ///
    /// Empty while a message is still formatted by hand (`B-078c`). A caller
    /// asking *what went wrong* should read this rather than the prose:
    /// `B-015` put diagnostics in a table so the wording could change, and a
    /// test matching on wording is exactly what that would break.
    pub code: &'static str,
    /// What varies, in the order the template names it.
    pub args: Vec<String>,
    /// The message, in the language the encoder was built to speak.
    ///
    /// [`message`](Self::message) renders it in any language, which a library
    /// cannot choose for itself — reading `SANSOS_LANG` here would make these
    /// tests depend on the shell that ran them (`B-015`).
    pub reason: String,
}

impl EncodeError {
    /// The message in a chosen language.
    ///
    /// A diagnostic with no code yet falls back to its rendered text, so a
    /// caller need not know which have been migrated.
    #[must_use]
    pub fn message(&self, lang: Language) -> String {
        if self.code.is_empty() {
            return self.reason.clone();
        }
        let args: Vec<&str> = self.args.iter().map(String::as_str).collect();
        Diagnostic::new(self.code, &args).render(lang)
    }
}

impl core::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "line {}: {}", self.line, self.reason)
    }
}

/// Numeric value of an operand base: a register number or a numeral.
/// Split `लक्ष्यॱउपरि` into its label and which half of the address it names.
///
/// Returns `None` when the operand carries no address modifier at all.
fn split_address_part(base: &str) -> Option<(&str, bool)> {
    if let Some(l) = base.strip_suffix("ॱउपरि") {
        return Some((l, true));
    }
    base.strip_suffix("ॱअधः").map(|l| (l, false))
}

/// The first immediate slot nothing has claimed.
///
/// `csrrwi` takes two — the control register number and the value to write —
/// and every other instruction takes one. Handing both to the first slot made
/// them collide, which the corpus found the moment the CSR families became
/// nameable (`B-060`).
fn next_immediate<'e>(enc: &'e Encoding, filled: &[(u32, &str)]) -> Option<&'e Slot> {
    enc.slots
        .iter()
        .filter(|s| is_immediate(&s.kind))
        .find(|s| !filled.iter().any(|(m, _)| *m == s.mask))
}

/// Numeric value of an operand base, for an encoding that either does or does
/// not take ordering-domain sets.
///
/// `domains_fit` is not a convenience. Without it `पठनम्` is the number 2
/// wherever it is written, so a misspelled label in an instruction taking an
/// immediate would assemble to a plausible constant rather than being named
/// (`B-107`). The reading has to follow the family, here as in the shape
/// classification above.
fn value_of(base: &str, domains_fit: bool) -> Option<u64> {
    if let Some((n, _)) = register(base) {
        return Some(u64::from(n));
    }
    if domains_fit && let Some(bits) = domain_set(base) {
        return Some(bits);
    }
    // `bits` rather than `value`: `ऋण` makes a literal negative and the field
    // wants two's complement. `value` refuses a negative outright so a caller
    // that has not thought about sign cannot get a very large number by
    // accident (`B-062`).
    //
    // This read `signed_value` and then cast to `u64`, which is what said the
    // reader was the wrong shape (`W-075`): the cast was the real answer, and
    // the `i64` in the middle was where a literal past `i64::MAX` quietly became
    // `i64::MAX`. Whether the bits fit is decided below against the field's own
    // width, which is where it belongs — a 12-bit immediate has a range no
    // general numeral reader could know.
    sanskrit_text::numeral::bits(base).ok()
}

/// Encode one instruction.
///
/// # Errors
/// Returns a diagnostic naming what was written whenever no encoding of the
/// family matches, an operand is unknown, or a value will not fit its field.
pub fn encode(inst: &Instruction) -> Result<u32, EncodeError> {
    encode_at(inst, 0, &BTreeMap::new())
}

/// Assemble a whole program — task `B-007`.
///
/// Two passes, because a branch may name a label defined later.
///
/// **Pass one** assigns an address to every instruction. **Pass two** encodes
/// each one, resolving a label to the distance from the instruction that names
/// it — which is why pass one must finish first, and why the encoder rather
/// than the parser owns addresses: only it knows how many bytes an instruction
/// takes.
///
/// # The relaxation fixpoint, and why it converges immediately
///
/// `B-007` asks for a fixpoint because choosing a compressed form shortens an
/// instruction, which moves everything after it, which can bring a distant
/// branch into range of a shorter encoding — so the answer feeds back into the
/// question and must be iterated to a fixed point.
///
/// This encoder emits only 32-bit forms (`bits == 32`), so every instruction is
/// four bytes and pass one's answer cannot change. The loop below is therefore
/// written as a fixpoint and converges on the first iteration. That is not the
/// same as being implemented: when compressed selection lands (`B-056`) the
/// iteration will start doing work, and the shape is here so that it is one
/// change rather than two.
///
/// # Errors
/// Every instruction that fails to encode, not only the first.
pub fn encode_program(program: &Program) -> Result<Vec<u8>, Vec<EncodeError>> {
    encode_program_for(program, Target::Uncompressed)
}

/// Which instruction set the image is for.
///
/// GNU models this with `-march`: `rv64gc` auto-compresses and `rv64imafd` does
/// not. Both are real targets and this project has a corpus for each —
/// `spec/conformance-t0.tsv` and `spec/conformance-rvc-t0.tsv` — so the choice
/// is named rather than assumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Target {
    /// Every instruction is 32 bits. The default, and what every corpus except
    /// the rvc one expects.
    #[default]
    Uncompressed,
    /// Choose a 16-bit form wherever the assembler would (`B-058b2b5`).
    Compressed,
}

/// Assemble for a named target.
///
/// # Errors
/// As [`encode_program`].
pub fn encode_program_for(program: &Program, target: Target) -> Result<Vec<u8>, Vec<EncodeError>> {
    // Pass one: addresses, from each instruction's OWN width.
    //
    // This used to be `pc += 4` with a comment saying every instruction is four
    // bytes today. That is still true — nothing selects a compressed form yet
    // (`B-058b2b`) — but the loop no longer assumes it, so the fixpoint that
    // `B-007` built for exactly this reason can finally iterate when selection
    // arrives rather than needing to be rewritten around it.
    let mut addresses: Vec<u32> = Vec::with_capacity(program.instructions.len());
    let mut symbols: BTreeMap<String, u32> = BTreeMap::new();
    let mut previous: Vec<u32> = Vec::new();

    for _ in 0..MAX_RELAXATION_ROUNDS {
        addresses.clear();
        let mut pc = 0u32;
        for (i, inst) in program.instructions.iter().enumerate() {
            pc += align_pad_at(program, i, pc);
            addresses.push(pc);
            pc += width_of_for(inst, pc, &symbols, target);
        }
        // Text addresses stay relative to the image base: a branch is a
        // difference, so the base cancels, and keeping it out of `addresses`
        // means one place converts to absolute — the data base below.
        symbols.clear();
        // `.data` follows `.text`, eight-aligned, exactly as `kosha` lays it
        // out. Kept RELATIVE to the image base like every text address: a
        // pc-relative offset is a difference and the base cancels, so mixing
        // one absolute address into the table put 0x80000000 into an auipc.
        let data_base = pc.next_multiple_of(8);
        // `ॱरिक्त` follows `ॱदत्त` in memory and nowhere in the file.
        let data_len: u32 = program
            .data
            .iter()
            .map(|d| u32::try_from(d.bytes.len()).unwrap_or(0))
            .sum();
        let bss_base = (data_base + data_len).next_multiple_of(8);
        for l in &program.labels {
            let addr = match l.section {
                crate::parse::Section::Text => addresses.get(l.at).copied().unwrap_or(pc),
                crate::parse::Section::Data => {
                    data_base + u32::try_from(l.data_offset).unwrap_or(u32::MAX)
                }
                crate::parse::Section::Bss => {
                    bss_base + u32::try_from(l.data_offset).unwrap_or(u32::MAX)
                }
            };
            symbols.insert(l.name.clone(), addr);
        }
        if addresses == previous {
            break;
        }
        previous.clone_from(&addresses);
    }

    // Say so when layout did NOT settle.
    //
    // `MAX_RELAXATION_ROUNDS`'s own comment says a program that has not settled
    // after this many rounds "is a bug in the encoder rather than a large
    // program" — and nothing checked it. The loop simply fell out and encoded
    // with whatever addresses the last round happened to hold, so every branch
    // and every symbol reference was computed against a layout the encoder had
    // already superseded. That is an image which assembles clean, exits 0,
    // carries no diagnostic, and does not run.
    //
    // `B-058b2b7` measured what that costs: under `--संक्षिप्त`, 19 of 36
    // property checks failed and 15 of them failed in SILENCE. `trap-seam`
    // printed the right number of lines with a sentinel `0xc3` replaced by the
    // firmware address `0x80044e34`, in a program that still shut down cleanly.
    //
    // This does not make relaxation converge — that is a separate row. It makes
    // not converging sayable, which is the difference between a wrong image and
    // a refusal.
    if addresses != previous {
        return Err(vec![EncodeError {
            line: 0,
            code: "E22",
            args: vec![MAX_RELAXATION_ROUNDS.to_string()],
            reason: format!(
                "layout did not settle after {MAX_RELAXATION_ROUNDS} rounds, so every \
                 branch and symbol reference would be computed against addresses the \
                 encoder has already superseded"
            ),
        }]);
    }

    let mut text: Vec<u8> = Vec::with_capacity(program.instructions.len() * 4);
    let mut errors = Vec::new();
    for (i, inst) in program.instructions.iter().enumerate() {
        // The padding the layout above already accounted for. Emitting it here
        // and counting it there are the same number by construction: both ask
        // `align_pad_at` the same question about the same `pc` (`W-071`).
        let pad = align_pad_at(program, i, u32::try_from(text.len()).unwrap_or(0));
        if pad != 0 {
            let fill = align_fill(u32::try_from(text.len()).unwrap_or(0), pad);
            text.extend_from_slice(&fill);
        }
        // Layout and emit must agree about where this instruction IS.
        //
        // Every displacement in the program is computed from `addresses[i]`,
        // but the bytes land at `text.len()`. Nothing required those to be the
        // same number, and when they drift the program still assembles: each
        // instruction is individually well formed and every branch is off by
        // the drift. `B-058b2b7` traced `boot-sbi.sas` under `--संक्षिप्त` to
        // exactly this — a `jal` emitted at 0x16 encoded its displacement from
        // 0x18 and jumped to 0x06, the second half of the `addi` at 0x04.
        // Executing from inside an instruction is how an image that assembles
        // clean produces a machine that prints nothing.
        //
        // This is an equality the encoder can check for free, so the class of
        // defect where the two passes disagree cannot be silent again.
        if u32::try_from(text.len()).unwrap_or(u32::MAX) != addresses[i] {
            return Err(vec![EncodeError {
                line: 0,
                code: "E23",
                args: vec![
                    i.to_string(),
                    format!("{:#x}", addresses[i]),
                    format!("{:#x}", text.len()),
                    (text.len() as i64 - i64::from(addresses[i])).to_string(),
                ],
                reason: format!(
                    "instruction {i} was laid out at {:#x} and emitted at {:#x}; every \
                     displacement in this program is computed from the former and would \
                     be wrong by {} octets",
                    addresses[i],
                    text.len(),
                    i64::from(addresses[i]) - text.len() as i64,
                ),
            }]);
        }
        let chosen = if target == Target::Compressed {
            compressed_at(inst, addresses[i], &symbols)
                .map(|h| Ok(u32::from(h)))
                .unwrap_or_else(|| encode_at(inst, addresses[i], &symbols))
        } else {
            encode_at(inst, addresses[i], &symbols)
        };
        match chosen {
            Ok(w) => {
                let n = width_of_for(inst, addresses[i], &symbols, target) as usize;
                // Little-endian, and only the bytes this instruction occupies:
                // a compressed form is the low halfword and writing four would
                // put two zero bytes where the next instruction goes.
                text.extend_from_slice(&w.to_le_bytes()[..n]);
            }
            Err(e) => errors.push(e),
        }
    }
    if errors.is_empty() {
        Ok(text)
    } else {
        Err(errors)
    }
}

/// How many bytes an instruction will occupy.
///
/// Always 4 until `B-058b2b` teaches the encoder to choose a compressed form.
/// It exists now because the address pass has to ask *something*: a loop that
/// adds a constant cannot be made to iterate later without being rewritten,
/// and rewriting the fixpoint is how relaxation bugs get introduced.
#[must_use]
pub(crate) fn width_of_for(
    inst: &Instruction,
    pc: u32,
    symbols: &BTreeMap<String, u32>,
    target: Target,
) -> u32 {
    // Asked twice — once to lay out and once to emit — and it must answer the
    // same both times or an instruction is written at an address the layout did
    // not reserve. Both calls pass the same pc and symbols, so they do.
    //
    // In the first relaxation round a forward label is unknown, so a branch
    // fails to encode and this answers 4. A later round knows the address and
    // may answer 2, which moves everything after it — which is exactly what the
    // fixpoint is for, and the first thing since `B-007` to make it iterate.
    match target {
        Target::Compressed if compressed_at(inst, pc, symbols).is_some() => 2,
        _ => 4,
    }
}

/// Read uniform-width text back as 32-bit words.
///
/// For callers that analyse text known to hold no compressed instruction —
/// today that is every caller, because nothing emits one. `विश्लेषणम्`'s
/// `decode_at` is what reads a stream whose widths vary, and the census and the
/// duplicate detector move to it in `B-058b2b`.
///
/// # Panics
/// Never; a trailing partial word is dropped rather than guessed at.
#[must_use]
pub fn words(text: &[u8]) -> Vec<u32> {
    text.chunks_exact(4)
        .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// How many times layout may be recomputed before it is called divergent.
///
/// Relaxation converges monotonically — instructions only shrink — so a program
/// that has not settled after this many rounds is a bug in the encoder rather
/// than a large program.
pub(crate) const MAX_RELAXATION_ROUNDS: usize = 8;

/// Encode one instruction at a known address, resolving labels against
/// `symbols`.
///
/// # Errors
/// A diagnostic naming what was written whenever no encoding matches, an
/// operand is unknown, a label is undefined, or a value will not fit.
pub fn encode_at(
    inst: &Instruction,
    pc: u32,
    symbols: &BTreeMap<String, u32>,
) -> Result<u32, EncodeError> {
    encode_in(inst, pc, symbols, 32)
}

/// The 16-bit form of an instruction, when one exists and the operands fit.
///
/// `None` means no compressed encoding of this family accepts these operands —
/// a register outside `x8`..`x15` for a three-bit field, an immediate that is
/// not the right multiple, a displacement out of reach. The wide form is always
/// correct, so a refusal here is never an error.
///
/// # Roles are decided once, in the wide encoding
///
/// The obvious approach — assign kāraka rôles to compressed fields the way
/// [`encode_at`] does — cannot work: that assignment reads field POSITIONS, and
/// `RS1` is `0x000f_8000` while `c.add`'s source sits at `0x0000_007c`.
/// Generalising the constants per encoding would mean deriving, for each of
/// thirty-seven compressed forms, which field is which rôle.
///
/// None of that is necessary. A compressed instruction *is* a wide one with a
/// redundant operand removed, so the wide encoding already decided the rôles.
/// This encodes wide, reads the operands back out with `विश्लेषणम्` — in slot
/// order, which is the order `spec/compression-choices.tsv` names its relations
/// by — drops the redundant one, and places what is left into the compressed
/// slots in the same order.
///
/// The decoder was written to check the encoder (`B-013`). Using it here means
/// rôle assignment exists in exactly one place and the compressed path cannot
/// drift from the wide one.
#[must_use]
pub fn compressed_at(inst: &Instruction, pc: u32, symbols: &BTreeMap<String, u32>) -> Option<u16> {
    // The wide form settles the rôles and proves the operands are legal.
    let wide = encode_at(inst, pc, symbols).ok()?;
    let decoded = crate::vishlesana::decode(wide)?;
    // KINDS travel with the values. Dropping them let `add s0, s0, s1` reduce
    // to two operands, match `c.addi`'s [reg, imm] shape, and encode the
    // register `s1` as the immediate 9 — a valid instruction that is a
    // different one.
    let values: Vec<(&str, i64)> = decoded
        .operands
        .iter()
        .map(|(k, v)| (k.as_str(), *v))
        .collect();

    // The derivation is keyed on the WIDE mnemonic, which the decoder just
    // gave us. Searching the family instead matched `c.add` for an `addw`,
    // because no compressed row carries a व्याप्ति — the same defect that made
    // every load an `lb` (`B-043`), avoided here by asking a more exact
    // question rather than by adding another column.
    let all = encodings();

    // A result written to `x0` is discarded, so a compressed form naming it is
    // a hint rather than the operation and the assembler will not produce one.
    // Three of 5949 rvc conformance cases turned on this — `add zero,zero,a0`,
    // `addi zero,zero,0` and `slli zero,zero,0` — and every one of them would
    // have been a *different instruction*, not merely a larger one.
    //
    // The probe cannot see this: GNU ACCEPTS `c.add x0, x9` as a hint, so the
    // baseline search finds `x0` legal and records no bias. What it will not do
    // is *choose* it. Acceptance and selection are different questions.
    if let Some(wide) = all.iter().find(|e| e.insn == decoded.insn)
        && let Some(rd) = wide.slots.iter().position(|s| s.mask == RD)
        && values.get(rd).is_some_and(|(_, v)| *v == 0)
    {
        return None;
    }

    for (insn, relation) in forms_of(&decoded.insn) {
        let Some(e) = all.iter().find(|e| e.insn == insn && e.bits == 16) else {
            continue;
        };
        let Some(kept) = reduce(&values, relation) else {
            continue;
        };
        if kept.len() != e.slots.len() {
            continue;
        }
        let mut half = e.pattern;
        let mut ok = true;
        for (slot, (kind, value)) in e.slots.iter().zip(&kept) {
            if !same_kind(&slot.kind, kind) || !slot.fits(*value as u64) {
                ok = false;
                break;
            }
            half |= slot.place(*value as u64);
        }
        if ok {
            return Some(half as u16);
        }
    }
    None
}

/// The compressed forms a wide instruction is known to become, with the
/// coincidence each needs.
///
/// Read from `spec/compression-choices.tsv`: `addw` becomes `c.addw` when its
/// second operand repeats its first, and `add` becomes `c.add` under the same
/// condition. Keying on the wide mnemonic is what keeps those apart.
fn forms_of(wide: &str) -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    for l in COMPRESSION.lines() {
        if l.starts_with('#') || l.starts_with("assembly\t") || l.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() < 4 || f[1] == "-" {
            continue;
        }
        if f[0].split_whitespace().next() == Some(wide) && !out.contains(&(f[2], f[3])) {
            out.push((f[2], f[3]));
        }
    }
    out
}

/// The operand values with the redundant one removed.
///
/// Reduction is the whole of compression: `योगः कम् कन खन` names three
/// registers and `c.add` holds two, because the first source *is* the
/// destination and saying so twice is what the wide form is for.
///
/// Positions are the wide encoding's slot order, which is the order the
/// derivation named its relations by.
fn reduce<'a>(values: &[(&'a str, i64)], relation: &str) -> Option<Vec<(&'a str, i64)>> {
    let at = |i: usize| values.get(i).map(|(_, v)| *v);
    let drop = match relation {
        // Equal VALUES of the same kind: two registers, not a register and an
        // immediate that happen to share a number.
        "arg1=arg0" => (at(1)? == at(0)? && values[1].0 == values[0].0).then_some(1),
        "arg1=zero" => (at(1)? == 0 && is_register(values[1].0)).then_some(1),
        "arg2=zero" => (at(2)? == 0 && is_register(values[2].0)).then_some(2),
        "arg0=zero" => (at(0)? == 0 && is_register(values[0].0)).then_some(0),
        "none" => return Some(values.to_vec()),
        _ => None,
    }?;
    let mut kept = values.to_vec();
    kept.remove(drop);
    Some(kept)
}

/// Assemble one file as a **relocatable object** — task `B-069b2`.
///
/// Returns the text, the names it defines, and the references it could not
/// resolve. A name this file does not define is not an error here: it is the
/// question an object exists to ask.
///
/// The layout is the same fixpoint `encode_program_for` runs, because an
/// object's internal addresses are still its own — only the cross-file ones are
/// left open.
///
/// # Errors
/// As [`encode_program`], minus undefined names.
pub fn encode_object(program: &Program) -> Result<(Vec<u8>, Vec<Pending>), Vec<EncodeError>> {
    encode_object_for(program, Target::Uncompressed)
}

/// As [`encode_object`], at a chosen target — task `B-069e`.
///
/// # The row said this needed `R_RISCV_RELAX` and it does not
///
/// The reasoning was that relaxation moves every offset a record names. True of
/// a linker that relaxes; ours does not, and GNU's only does where an object
/// asks it to. `R_RISCV_RELAX` is a MARKER a producer attaches to a relocation
/// to say *this may be shortened*; emitting none says *leave it alone*, which
/// is what a file whose widths are already chosen means.
///
/// So the assembler runs the same relaxation fixpoint here that `encode_program`
/// runs, and writes the offsets it converges on. What the row feared is a thing
/// this toolchain has to opt into.
///
/// # Errors
/// As [`encode_program`], minus undefined names.
pub fn encode_object_for(
    program: &Program,
    target: Target,
) -> Result<(Vec<u8>, Vec<Pending>), Vec<EncodeError>> {
    let mut addresses: Vec<u32> = Vec::with_capacity(program.instructions.len());
    let mut symbols: BTreeMap<String, u32> = BTreeMap::new();
    let mut pc = 0u32;
    // The widths, iterated to a fixpoint: a compressed branch moves every
    // instruction after it, which moves the branch's own target, which may
    // change whether it still fits. `B-007` built this loop and `B-058b2b5`
    // first made it iterate.
    let mut previous: Vec<u32> = Vec::new();
    for _ in 0..MAX_RELAXATION_ROUNDS {
        addresses.clear();
        pc = 0;
        for (i, inst) in program.instructions.iter().enumerate() {
            pc += align_pad_at(program, i, pc);
            addresses.push(pc);
            pc += width_of_for(inst, pc, &symbols, target);
        }
        // Rebuilt AFTER the pass, from this round's addresses. Clearing it
        // before the pass — which this loop did until `W-241`, as
        // `layout_addresses` did — meant the pass never saw a label at all, so
        // every branch was scored wide and the emit pass below, which does see
        // the labels, wrote it narrow: the E23 refusal further down fired on
        // every spec program with a compressible branch (15 of 82 under
        // `--संक्षिप्त`, measured by `tests/paradigm_t0.rs`), which is why the
        // flag refused what `encode_program_for` assembled. Only text labels,
        // for the reason stated below.
        symbols.clear();
        for l in &program.labels {
            if l.section == crate::parse::Section::Text {
                symbols.insert(l.name.clone(), addresses.get(l.at).copied().unwrap_or(pc));
            }
        }
        if addresses == previous {
            break;
        }
        previous.clone_from(&addresses);
    }

    // Say so when layout did NOT settle.
    //
    // `MAX_RELAXATION_ROUNDS`'s own comment says a program that has not settled
    // after this many rounds "is a bug in the encoder rather than a large
    // program" — and nothing checked it. The loop simply fell out and encoded
    // with whatever addresses the last round happened to hold, so every branch
    // and every symbol reference was computed against a layout the encoder had
    // already superseded. That is an image which assembles clean, exits 0,
    // carries no diagnostic, and does not run.
    //
    // `B-058b2b7` measured what that costs: under `--संक्षिप्त`, 19 of 36
    // property checks failed and 15 of them failed in SILENCE. `trap-seam`
    // printed the right number of lines with a sentinel `0xc3` replaced by the
    // firmware address `0x80044e34`, in a program that still shut down cleanly.
    //
    // This does not make relaxation converge — that is a separate row. It makes
    // not converging sayable, which is the difference between a wrong image and
    // a refusal.
    if addresses != previous {
        return Err(vec![EncodeError {
            line: 0,
            code: "E22",
            args: vec![MAX_RELAXATION_ROUNDS.to_string()],
            reason: format!(
                "layout did not settle after {MAX_RELAXATION_ROUNDS} rounds, so every \
                 branch and symbol reference would be computed against addresses the \
                 encoder has already superseded"
            ),
        }]);
    }
    let data_base = pc.next_multiple_of(8);
    let data_len: u32 = program
        .data
        .iter()
        .map(|d| u32::try_from(d.bytes.len()).unwrap_or(0))
        .sum();
    let bss_base = (data_base + data_len).next_multiple_of(8);
    // ONLY text labels. A pc-relative reference can be resolved while
    // assembling exactly when both ends sit in the same section, because only
    // then does linking move them together: every object's `.text` is laid end
    // to end and `.data` follows all of it, so the distance from an instruction
    // to a datum in the same file changes the moment another file is added.
    //
    // Resolving one anyway is what this did, and it produced an object whose
    // `auipc`/`addi` pair pointed 32 bytes short once `lib-mudraka` was linked
    // in front of the data — a program that runs and prints nothing.
    let _ = (data_base, bss_base);
    for l in &program.labels {
        if l.section != crate::parse::Section::Text {
            continue;
        }
        symbols.insert(l.name.clone(), addresses.get(l.at).copied().unwrap_or(pc));
    }

    let mut text: Vec<u8> = Vec::with_capacity(program.instructions.len() * 4);
    // What an object cannot yet carry is REFUSED here, never dropped. Measuring
    // `B-096` found three differences between this path and `बन्धकः` and all
    // three were silent: `ॱरिक्त` vanished from the file, and `--संक्षिप्त` and
    // `-g` did nothing. A gap that says nothing is indistinguishable from a
    // feature that works, which is how `B-058b2b3` shipped a flag that was inert
    // and `B-069d2b` shipped an address resolved too early. `ॱरिक्त` is a real
    // `.bss` now (`B-099`); the two flags are still refused.
    //
    // An address in `ॱदत्त` is a record in `.rela.data` (`B-098`, ADR-0013).
    // The eight bytes the parser reserved stay zero here and the linker writes
    // them: an object has no load address, so the value does not exist yet.
    //
    // `R_RISCV_64` is the absolute 64-bit relocation, derived in `B-069a` and
    // applied by nothing until now — it had a number, a place to apply it, and
    // no producer in the language until a directive could name an address.
    let mut data_pending = Vec::new();
    let mut data_at = 0u32;
    for d in &program.data {
        for (offset, name) in &d.addresses {
            data_pending.push(Pending {
                name: name.clone(),
                kind: "R_RISCV_64",
                at: data_at + u32::try_from(*offset).unwrap_or(0),
                section: crate::kosha::RelSection::Data,
            });
        }
        data_at += u32::try_from(d.bytes.len()).unwrap_or(0);
    }

    let mut pending = data_pending;
    let mut errors = Vec::new();
    for (i, inst) in program.instructions.iter().enumerate() {
        // Same padding as the layout pass counted, for the same reason as the
        // uncompressed path above (`W-071`).
        let pad = align_pad_at(program, i, u32::try_from(text.len()).unwrap_or(0));
        if pad != 0 {
            let fill = align_fill(u32::try_from(text.len()).unwrap_or(0), pad);
            text.extend_from_slice(&fill);
        }
        // Layout and emit must agree about where this instruction IS.
        //
        // Every displacement in this object, and every symbol the object
        // exports, is computed from `addresses[i]`; the bytes land at
        // `text.len()`. Nothing required those to be the same number.
        //
        // They diverged under `--संक्षिप्त` because the address pass asked
        // `compressed_at` with an EMPTY symbol map — the loop above cleared it
        // before the pass until `W-241` — while this emit pass asks it with the
        // final one, so an instruction the layout scored as four octets was
        // emitted as two. The fixpoint cannot see that: it compares addresses
        // between rounds, not addresses against output. With the table carried
        // forward, a settled fixpoint means the final table IS the one the last
        // pass used, and the two answers are the same by construction; this
        // check stays because "by construction" is a claim and this is the
        // measurement.
        //
        // Measured on `spec/boot-sbi.sas` at `B-058b2b7`:
        //
        //     laid out   [0, 4, 8, c, 10, 12, 16, 18, 1c, 1e, 22]
        //     emitted    [0, 4, 8, c,  e, 10, 14, 16, 1a, 1c, 20]
        //
        // The image assembles, exits 0, and every branch past the first
        // divergence is wrong: the `jal` emitted at 0x16 took its displacement
        // from 0x18 and jumped to 0x06, the second half of the `addi` at 0x04.
        // That is a machine executing from inside an instruction, and it prints
        // nothing — which is 15 of the 19 checks `B-058b2b7` measured.
        //
        // Refusing does not make relaxation agree with itself; that is a
        // separate row. It makes the disagreement sayable.
        if u32::try_from(text.len()).unwrap_or(u32::MAX) != addresses[i] {
            return Err(vec![EncodeError {
                line: 0,
                code: "E23",
                args: vec![
                    i.to_string(),
                    format!("{:#x}", addresses[i]),
                    format!("{:#x}", text.len()),
                    (text.len() as i64 - i64::from(addresses[i])).to_string(),
                ],
                reason: format!(
                    "instruction {i} was laid out at {:#x} and emitted at {:#x}; every \
                     displacement and every exported symbol is computed from the former, \
                     so this image would be wrong by {} octets from here on",
                    addresses[i],
                    text.len(),
                    text.len() as i64 - i64::from(addresses[i]),
                ),
            }]);
        }
        // A compressed form carries no relocation: `c.j`'s eleven bits cannot
        // hold a link-time address, and `compressed_at` needs the target to
        // choose one at all. So an instruction naming a name this file does not
        // define stays wide by construction — `compressed_at` returns nothing
        // for it — and the record it leaves behind names four bytes that are
        // still there.
        let n = width_of_for(inst, addresses[i], &symbols, target) as usize;
        let chosen = if n == 2 {
            compressed_at(inst, addresses[i], &symbols)
                .map(|h| Ok(u32::from(h)))
                .unwrap_or_else(|| encode_at(inst, addresses[i], &symbols))
        } else {
            encode_collecting(inst, addresses[i], &symbols, 32, Some(&mut pending))
        };
        match chosen {
            Ok(w) => text.extend_from_slice(&w.to_le_bytes()[..n]),
            Err(e) => {
                // Keep the layout: an instruction that cannot be encoded still
                // occupies its width, or the NEXT instruction's E23 fires and
                // returns alone, hiding the cause (a far `jal` read as a
                // layout drift on 2026-09-14).
                text.extend_from_slice(&[0u8; 4][..n]);
                errors.push(e);
            }
        }
    }
    if errors.is_empty() {
        Ok((text, pending))
    } else {
        Err(errors)
    }
}

/// Encode against the encodings of one width.
///
/// # Errors
/// As [`encode_at`].
fn encode_in(
    inst: &Instruction,
    pc: u32,
    symbols: &BTreeMap<String, u32>,
    bits: u32,
) -> Result<u32, EncodeError> {
    encode_collecting(inst, pc, symbols, bits, None)
}

/// Encode, and optionally collect the references that could not be resolved.
///
/// With `pending` as `None` an undefined name is an error, which is what an
/// executable needs. With `Some`, the field is left zero and the name is
/// recorded — which is what an object needs, and the only difference between
/// the two.
///
/// # Errors
/// As [`encode_at`].
fn encode_collecting(
    inst: &Instruction,
    pc: u32,
    symbols: &BTreeMap<String, u32>,
    bits: u32,
    mut pending: Option<&mut Vec<Pending>>,
) -> Result<u32, EncodeError> {
    let all = encodings();
    let candidates: Vec<&Encoding> = all
        .iter()
        .filter(|e| e.family == inst.family.key)
        .filter(|e| e.bits == bits)
        .collect();

    if candidates.is_empty() {
        return Err(EncodeError {
            line: inst.line,
            code: "E01",
            args: Diagnostic::new("E01", &[&inst.family.name]).args,
            reason: Diagnostic::new("E01", &[&inst.family.name]).render(Language::default()),
        });
    }

    // A domain set is neither a register nor a numeral: it is a set of names
    // that lands in an immediate field. Counting it as a register is what made
    // `स्मृतिबन्धः` report "no encoding taking 2 register(s)".
    //
    // But only where a domain set can go (`B-107`). `पठनम्` is "reading" as
    // well as the memory-read domain, and reading it as a domain everywhere
    // made a label of that name definable and unreferenceable: `लङ्घनम्
    // शून्यःम् पठनम्य्` counted its target as a domain, so `jal` — one
    // register and one displacement — matched nothing, and the diagnostic
    // blamed the shape without ever saying the name meant something else. The
    // table already knows which encodings take one, so the family decides.
    let domains_fit = candidates.iter().any(|e| e.takes_domains);
    let is_domain =
        |o: &crate::parse::Operand| !o.is_numeral && domains_fit && domain_set(&o.base).is_some();
    let domains = inst.operands.iter().filter(|o| is_domain(o)).count();
    // A label is not a register either. Counting it as one made `beq` look
    // like a three-register instruction and match nothing.
    // An operand carrying an address modifier fills an IMMEDIATE, not a branch
    // target, so it must not be counted as a label or `auipc` matches nothing.
    let labels = inst
        .operands
        .iter()
        .filter(|o| {
            !o.is_numeral
                && !is_domain(o)
                && register(&o.base).is_none()
                && split_address_part(&o.base).is_none()
        })
        .count();
    let regs = inst
        .operands
        .iter()
        .filter(|o| !o.is_numeral && register(&o.base).is_some())
        .count();
    let imms = inst
        .operands
        .iter()
        .filter(|o| o.is_numeral || split_address_part(&o.base).is_some())
        .count();

    // Match the written shape, then the width. The `w` suffix is RV64's
    // convention for a 32-bit operation on 64-bit registers; it is the one
    // naming rule this encoder relies on, and it is checked by the differential
    // test rather than assumed.
    // If no encoding of this family takes a target, an operand that resolved to
    // neither a register nor a numeral is not a label — it is a name nobody
    // knows, and saying "no encoding takes 2 registers" would blame the shape
    // for a typo. Name it instead.
    if labels > 0
        && !candidates
            .iter()
            .any(|e| e.slots.iter().any(|s| s.kind == "disp"))
        && let Some(op) = inst
            .operands
            .iter()
            .find(|o| !o.is_numeral && !is_domain(o) && register(&o.base).is_none())
    {
        return Err(EncodeError {
            line: inst.line,
            code: "E02",
            args: Diagnostic::new("E02", &[&op.base]).args,
            reason: Diagnostic::new("E02", &[&op.base]).render(Language::default()),
        });
    }

    // A type suffix names the VALUES, not the instruction (ADR-0014). Doc 02
    // §2.5's own line says so: `अ३२` is `i32` and `न३२` is `u32`, a type name
    // borrowed by T0 for its width suffix.
    //
    // So it is refused only where something can contradict it, and the family
    // name is one such thing — ADR-0006 put signedness there. `अचिह्नाहारः` is
    // `lbu` and loads a `u32`, so `ॱअ` on it is false; `सचिह्नदक्षिणसरणम्` is
    // `sra` and shifts an `i64`, so `ॱन` on it is false.
    //
    // Everything else is accepted. `योगःॱन३२` is *add two u32s*, which is a
    // true sentence about the values even though `addw` is the same instruction
    // either way. `B-097` refused it by reasoning from ADR-0006, which decides
    // how an INSTRUCTION is named and says nothing about what its OPERANDS are.
    //
    // A conversion is exempt: it writes a PAIR of types (ADR-0010) whose
    // classes name what it moves between.
    if inst.types.len() == 1 {
        let class = inst.types[0].chars().next();
        let unsigned_family = inst.family.name.starts_with("अचिह्न");
        let signed_family = inst.family.name.starts_with("सचिह्न");
        let contradiction = match class {
            Some('न') if signed_family => Some("E20"),
            Some('अ') if unsigned_family => Some("E21"),
            _ => None,
        };
        if let Some(code) = contradiction {
            return Err(EncodeError {
                line: inst.line,
                code,
                args: vec![inst.family.name.clone()],
                reason: Diagnostic::new(code, &[&inst.family.name]).render(Language::default()),
            });
        }
    }

    let want = inst.width.bits();
    // NOTE: 30 families are ambiguous here — several encodings fit and file
    // order decides. `crates/sadhana/tests/encodings_sound.rs` pins the set and
    // `B-056` is the naming project that fixes it. Refusing instead of choosing
    // is the right behaviour and cannot land until the names exist.
    let mut best: Option<&Encoding> = None;
    for e in &candidates {
        if e.register_count() != regs {
            continue;
        }
        let disp_slots = e.slots.iter().filter(|s| s.kind == "disp").count();
        if disp_slots != labels {
            continue;
        }
        // The destination's register CLASS chooses among encodings that differ
        // only in direction. `प्लवसंचारः` covers four: two write a float
        // register from an integer one and two do the reverse, and the operands
        // already say which — so the direction needs no name (`B-074`).
        if let Some(dest) = inst.by_role(Karaka::Destination)
            && let Some((_, dest_is_float)) = register(&dest.base)
            && let Some(rd) = e.slot_with(RD)
            && (rd.kind == "freg") != dest_is_float
        {
            continue;
        }

        // A conversion is chosen by the PAIR of types written after its name,
        // and by nothing else: every one of the eighteen writes one register
        // from one other, so shape, width and register class between them
        // distinguish only three of the four ways they differ (`B-075`).
        match &e.converts {
            Some((to, from)) => {
                if inst.types.len() != 2 || &inst.types[0] != to || &inst.types[1] != from {
                    continue;
                }
            }
            // And only a conversion takes two: nothing else reads one type and
            // writes another, so a second one on `योगः` is a mistake, not a
            // request the encoder should quietly drop.
            None if inst.types.len() > 1 => continue,
            None => {}
        }

        // Most instructions have at most one immediate, and several operands
        // can share it (a load's offset). `fence` has two, and each domain set
        // needs one of its own, so the count is exact there and capped here.
        let imm_slots = e.slots.iter().filter(|s| is_immediate(&s.kind)).count();
        let ok = if domains > 0 {
            imm_slots == imms + domains
        } else {
            usize::from(e.immediate().is_some()) == imms.min(1)
        };
        if !ok {
            continue;
        }
        // An encoding that states a width must match the one asked for. The
        // `load` family covers lb, lh, lw, ld and their unsigned forms, and
        // every one of them takes rd, rs1 and an immediate — identical shapes.
        // Selecting on shape alone therefore took whichever came first in the
        // file, so `आहारः` was `lb` at every width and `ld` was unreachable.
        // The type's CLASS must match what the encoding operates on. `ॱप` is a
        // float and `ॱअ`/`ॱन` are integers, and until `B-077` the letter was
        // validated and then thrown away — so `योगःॱप३२`, "add, 32-bit float",
        // assembled as an integer `addw`, and every float family in the corpus
        // was written with the integer type that happens to share its width.
        //
        // Which class an encoding wants is derivable and needs no column: an
        // encoding with a `freg` slot operates on floats. Twenty-two families
        // have one, forty-nine do not, and none has both.
        //
        // A conversion is exempt: ADR-0010 gives it an explicit PAIR of types,
        // and its written half names the destination, which for `fcvt.w.s` is
        // an integer even though the encoding reads a float.
        if e.converts.is_none()
            && let Some(class) = inst.types.first().and_then(|s| s.chars().next())
            && matches!(class, 'अ' | 'न' | 'प')
        {
            let wants_float = e.slots.iter().any(|s| s.kind == "freg");
            if wants_float != (class == 'प') {
                continue;
            }
        }

        match e.width {
            Some(w) if w != want => continue,
            _ => {}
        }
        if e.width.is_some() {
            best = Some(e);
            break;
        }
        if best.is_none() {
            best = Some(e);
        }
    }

    let Some(enc) = best else {
        // A conversion that found nothing failed on its types, not its shape,
        // and every one of them has the same shape — so counting registers at
        // the reader would name the wrong thing entirely.
        if candidates.iter().all(|e| e.converts.is_some()) {
            let mut pairs: Vec<String> = candidates
                .iter()
                .filter_map(|e| e.converts.as_ref())
                .map(|(a, b)| format!("ॱ{a}ॱ{b}"))
                .collect();
            pairs.sort();
            pairs.dedup();
            return Err(EncodeError {
                line: inst.line,
                code: "E10",
                args: vec![
                    inst.family.name.clone(),
                    inst.types.join("ॱ"),
                    pairs.join(" "),
                ],
                reason: Diagnostic::new(
                    "E10",
                    &[&inst.family.name, &inst.types.join("ॱ"), &pairs.join(" ")],
                )
                .render(Language::default()),
            });
        }
        // The class is the likeliest thing to have failed when the shape is
        // fine, and blaming the operand count would send the reader to check
        // registers that were correct (`B-077`).
        if let Some(class) = inst.types.first().and_then(|s| s.chars().next())
            && matches!(class, 'अ' | 'न' | 'प')
        {
            let float = candidates
                .iter()
                .any(|e| e.slots.iter().any(|s| s.kind == "freg"));
            if float != (class == 'प') {
                return Err(EncodeError {
                    line: inst.line,
                    // Two codes, not one with a word for an argument: an
                    // argument is interpolated verbatim, so passing "floats"
                    // put an English word inside the Sanskrit sentence. What
                    // differs between the two messages is the message.
                    code: if float { "E09" } else { "E03" },
                    args: vec![inst.family.name.clone(), class.to_string()],
                    reason: Diagnostic::new(
                        if float { "E09" } else { "E03" },
                        &[&inst.family.name, &class.to_string()],
                    )
                    .render(Language::default()),
                });
            }
        }
        return Err(EncodeError {
            line: inst.line,
            code: "E04",
            args: Diagnostic::new(
                "E04",
                &[&inst.family.name, &regs.to_string(), &imms.to_string()],
            )
            .args,
            reason: Diagnostic::new(
                "E04",
                &[&inst.family.name, &regs.to_string(), &imms.to_string()],
            )
            .render(Language::default()),
        });
    };

    // Does some operand claim rs1 as an ADDRESS? If so, करण cannot have it,
    // wherever the two are written.
    //
    // `परमाणुयोगः` (amoadd) is the shape that exposed this: it has a
    // destination, a value and an address, so the करण arm's rule — take rs1
    // when there is a destination — collided with the address in both operand
    // orders. Deciding by role rather than by arrival is what D-02-C is for,
    // and this is the first instruction with enough roles to tell the
    // difference.
    //
    // A branch is excluded because there अपादान is the standard of comparison
    // and takes rs2 (ADR-0008), not an address at all.
    let has_disp = enc.slots.iter().any(|s| s.kind == "disp");
    let address_claims_rs1 = !has_disp
        && inst.operands.iter().any(|o| {
            matches!(o.karaka, Karaka::SourceAddress | Karaka::DestAddress)
                && !o.is_numeral
                && register(&o.base).is_some()
        });

    let mut word = enc.pattern;
    let mut used_rs1 = false;
    // Which field each operand claimed, so a second claim on the same field is
    // an error rather than a bitwise OR.
    //
    // This is the general form of the bug `B-044` was raised for. `sd` has no
    // rd, so two operands that both resolve to "some source" both fell through
    // to rs2: `निधानम् क्षणिक०न स्तूपसूचकःए ८न ।` OR-ed t0 (5) and sp (2) into
    // register 7, and emitted a valid store of the wrong register from the
    // wrong base. Nothing rejected it, because every individual step succeeded.
    let mut filled: Vec<(u32, &str)> = Vec::new();

    for op in &inst.operands {
        // An address modifier — `B-064`. `लक्ष्यॱउपरि` is the high 20 bits of the
        // distance to लक्ष्य and `लक्ष्यॱअधः` the low 12, which is how a
        // 32-bit address is built out of two instructions that each hold less.
        //
        // Both are PC-RELATIVE, and that is forced rather than chosen: the
        // image loads at 0x80000000, `lui` sign-extends its immediate, and
        // 0x80000000 sign-extends to 0xffffffff80000000. Absolute addressing
        // cannot reach our own load address — GNU `ld` says "relocation
        // truncated to fit" and refuses.
        //
        // `ॱअधः` measures from the PRECEDING instruction, because that is the
        // `auipc` whose register it is completing. GNU spells the same thing
        // `%pcrel_lo(1b)`, naming the auipc by a back-reference; here the
        // convention is positional and the two must be adjacent.
        if let Some((label, upper)) = split_address_part(&op.base)
            && let Some(imm) = enc.slots.iter().find(|s| is_immediate(&s.kind))
        {
            let Some(target) = symbols.get(label) else {
                if let Some(out) = pending.as_deref_mut() {
                    // Written as zero, exactly as the assembler leaves it: the
                    // linker computes the whole value from the symbol and the
                    // addend, so anything we put here would be overwritten or,
                    // worse, added to.
                    out.push(Pending {
                        section: crate::kosha::RelSection::Text,
                        name: label.into(),
                        kind: if upper {
                            "R_RISCV_PCREL_HI20"
                        } else {
                            "R_RISCV_PCREL_LO12_I"
                        },
                        at: pc,
                    });
                    continue;
                }
                return Err(EncodeError {
                    line: inst.line,
                    code: "E05",
                    args: Diagnostic::new("E05", &[label]).args,
                    reason: Diagnostic::new("E05", &[label]).render(Language::default()),
                });
            };
            let base = if upper { pc } else { pc.wrapping_sub(4) };
            let offset = i64::from(*target) - i64::from(base);
            let hi = (offset + 0x800) >> 12;
            let value = if upper { hi } else { offset - (hi << 12) };
            if !imm.fits(value as u64) {
                return Err(EncodeError {
                    line: inst.line,
                    code: "E06",
                    args: Diagnostic::new("E06", &[&op.base, &enc.insn]).args,
                    reason: Diagnostic::new("E06", &[&op.base, &enc.insn])
                        .render(Language::default()),
                });
            }
            if filled.iter().any(|(m, _)| *m == imm.mask) {
                return Err(EncodeError {
                    line: inst.line,
                    code: "E11",
                    args: vec![op.base.clone()],
                    reason: Diagnostic::new("E11", &[&op.base]).render(Language::default()),
                });
            }
            filled.push((imm.mask, &op.base));
            word |= imm.place(value as u64);
            continue;
        }

        // A label names a place; the field holds the DISTANCE to it. `B-055`
        // derived those bits and this is what fills them.
        //
        // सम्प्रदान marks the target, because that is where control goes — the
        // same reading `fence` uses for its successor set. A branch written
        // with any other kāraka is refused rather than guessed at: the sigil is
        // the whole of D-02-C, and a target that could be marked करण would make
        // "jump to" and "jump from" spellable the same way.
        if let Some(disp) = enc.slots.iter().find(|s| s.kind == "disp")
            && !op.is_numeral
            && register(&op.base).is_none()
        {
            if op.karaka != Karaka::DestAddress {
                return Err(EncodeError {
                    line: inst.line,
                    code: "E12",
                    args: vec![op.base.clone()],
                    reason: Diagnostic::new("E12", &[&op.base]).render(Language::default()),
                });
            }
            let Some(target) = symbols.get(&op.base) else {
                if let Some(out) = pending.as_deref_mut() {
                    out.push(Pending {
                        section: crate::kosha::RelSection::Text,
                        name: op.base.clone(),
                        kind: "R_RISCV_JAL",
                        at: pc,
                    });
                    continue;
                }
                return Err(EncodeError {
                    line: inst.line,
                    code: "E05",
                    args: Diagnostic::new("E05", &[&op.base]).args,
                    reason: Diagnostic::new("E05", &[&op.base]).render(Language::default()),
                });
            };
            let delta = i64::from(*target) - i64::from(pc);
            if !disp.fits(delta as u64) {
                return Err(EncodeError {
                    line: inst.line,
                    code: "E07",
                    args: Diagnostic::new(
                        "E07",
                        &[
                            &op.base,
                            &delta.to_string(),
                            &(disp.map.len() + 1).to_string(),
                            &enc.insn,
                        ],
                    )
                    .args,
                    reason: Diagnostic::new(
                        "E07",
                        &[
                            &op.base,
                            &delta.to_string(),
                            &(disp.map.len() + 1).to_string(),
                            &enc.insn,
                        ],
                    )
                    .render(Language::default()),
                });
            }
            if filled.iter().any(|(m, _)| *m == disp.mask) {
                return Err(EncodeError {
                    line: inst.line,
                    code: "E13",
                    args: vec![op.base.clone()],
                    reason: Diagnostic::new("E13", &[&op.base]).render(Language::default()),
                });
            }
            filled.push((disp.mask, &op.base));
            word |= disp.place(delta as u64);
            continue;
        }

        let Some(value) = value_of(&op.base, domains_fit) else {
            return Err(EncodeError {
                line: inst.line,
                code: "E02",
                args: Diagnostic::new("E02", &[&op.base]).args,
                reason: Diagnostic::new("E02", &[&op.base]).render(Language::default()),
            });
        };

        // The kāraka chooses the field. A destination is rd; an address base is
        // rs1 whether it is being read from (अपादान) or written to (सम्प्रदान);
        // a plain source takes rs1 if free, else rs2. Because a store's value
        // operand lands in a slot carrying rs2's mask, this separates load from
        // store without either being named.
        let slot = match op.karaka {
            Karaka::Destination => enc.slot_with(RD),
            // अपादान on a BRANCH is the standard of comparison, not an
            // address. Sanskrit puts the thing compared against in the
            // ablative — खात् क न्यूनः, "क is less THAN ख" — so `न्यूनलङ्घनम्
            // कन खत् लक्ष्यय्` reads "jump if क is less than ख".
            //
            // Without it a comparison cannot be written at all: `blt` has two
            // sources and no destination, so both करण operands fell to rs2 and
            // B-044's guard refused the instruction. करण alone cannot separate
            // a subject from a standard, and giving the second one position
            // instead of a role would be the one thing D-02-C exists to
            // prevent.
            Karaka::SourceAddress if enc.slots.iter().any(|s| s.kind == "disp") => {
                enc.slot_with(RS2)
            }
            Karaka::SourceAddress | Karaka::DestAddress => {
                if domain_set(&op.base).is_some() {
                    // अपादान is what the fence orders FROM, सम्प्रदान what it
                    // orders TO — which is exactly predecessor and successor.
                    // The sigils were already carrying that meaning; `fence` is
                    // the first instruction to need it away from an address.
                    enc.nth_immediate(usize::from(op.karaka == Karaka::DestAddress))
                } else if op.is_numeral {
                    next_immediate(enc, &filled)
                } else {
                    enc.slot_with(RS1)
                }
            }
            // अधिकरण is in D-02-C's table of five and in no example, and no
            // encoding assigns it a field. It used to be treated as करण, which
            // happened to give the right answer for a load — रd and rs1 are
            // both free there, so it landed in rs1 — and silently corrupted a
            // store, where it fell through to rs2 and was OR-ed on top of the
            // value operand.
            //
            // Making it an alias for त् / य् is the tempting fix and it is
            // wrong: two spellings for one role is what D-02-C exists to
            // prevent, and for a store `खए` genuinely does not say whether ख
            // is the address or the value. What अधिकरण should mean is an open
            // question (`O-02-2`), so it is rejected rather than guessed at.
            Karaka::Locus => {
                return Err(EncodeError {
                    line: inst.line,
                    code: "E14",
                    args: vec![op.base.clone()],
                    reason: Diagnostic::new("E14", &[&op.base]).render(Language::default()),
                });
            }
            Karaka::Source => {
                if op.is_numeral {
                    next_immediate(enc, &filled)
                } else if !used_rs1
                    && !address_claims_rs1
                    && enc.slot_with(RS1).is_some()
                    // A destination means करण is being read INTO something, so
                    // it takes rs1. A displacement means the same: a branch has
                    // no destination but its करण is still the subject being
                    // compared, not a value being stored. Without the second
                    // test the rule written for stores — value goes to rs2 —
                    // fired on branches too and collided with the standard.
                    && (enc.slot_with(RD).is_some()
                        || enc.slots.iter().any(|s| s.kind == "disp"))
                {
                    used_rs1 = true;
                    enc.slot_with(RS1)
                } else {
                    // rs2, then rs3. A fused multiply-add reads three
                    // registers — `प्लवगुणयोगः` is the only shape with more
                    // sources than the two the encoder knew about, and it
                    // collided on the third rather than placing it.
                    //
                    // When rs2 exists and is taken, the answer is rs3 OR
                    // NOTHING — never rs1. Falling back to rs1 here let
                    // `निधानम् कन खन ८न` assemble, putting a second value into
                    // a store's address base: a valid instruction writing
                    // somewhere nobody named. Returning rs2 again keeps the
                    // collision that refuses it.
                    match enc.slot_with(RS2) {
                        Some(rs2) if filled.iter().any(|(m, _)| *m == RS2) => {
                            enc.slot_with(RS3).or(Some(rs2))
                        }
                        Some(rs2) => Some(rs2),
                        None => {
                            used_rs1 = true;
                            enc.slot_with(RS1)
                        }
                    }
                }
            }
        };

        let Some(slot) = slot else {
            return Err(EncodeError {
                line: inst.line,
                code: "E15",
                args: vec![op.base.clone(), op.karaka.name().into(), enc.insn.clone()],
                reason: Diagnostic::new("E15", &[&op.base, op.karaka.name(), &enc.insn])
                    .render(Language::default()),
            });
        };

        if let Some((_, prev)) = filled.iter().find(|(m, _)| *m == slot.mask) {
            return Err(EncodeError {
                line: inst.line,
                code: "E16",
                args: vec![prev.to_string(), op.base.clone(), enc.insn.clone()],
                reason: Diagnostic::new("E16", &[prev, &op.base, &enc.insn])
                    .render(Language::default()),
            });
        }
        filled.push((slot.mask, &op.base));

        if !slot.fits(value) {
            return Err(EncodeError {
                line: inst.line,
                code: "E17",
                args: vec![
                    op.base.clone(),
                    slot.map.len().to_string(),
                    enc.insn.clone(),
                ],
                reason: Diagnostic::new("E17", &[&op.base, &slot.map.len().to_string(), &enc.insn])
                    .render(Language::default()),
            });
        }
        word |= slot.place(value);
    }

    Ok(word)
}

/// Bytes of padding needed before instruction `i` so that `pc` lands on every
/// boundary `॥ संरेखः n ॥` asked for there (`W-071`).
///
/// The padding is `nop`, not zero. `0x00000000` is not an instruction on RISC-V
/// — it traps — and GNU `as` pads `.text` with `c.nop`/`nop` for the same
/// reason, which the differential oracle (doc 03 §4.4) compares against. Two
/// bytes of `c.nop` first when the address is odd-halfword, then four-byte
/// `nop`s, exactly as `as` does it.
pub(crate) fn align_pad_at(program: &crate::parse::Program, i: usize, pc: u32) -> u32 {
    let mut at = pc;
    for (idx, n) in &program.text_aligns {
        if *idx == i {
            let n = *n as u32;
            at = at.next_multiple_of(n);
        }
    }
    at - pc
}

/// The `nop` bytes that fill `pad` bytes from `pc`.
pub(crate) fn align_fill(pc: u32, pad: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(pad as usize);
    let mut at = pc;
    let end = pc + pad;
    // `c.nop` (0x0001) only when it is needed to reach a four-byte boundary;
    // a run of `c.nop` where `nop` fits would differ from `as` byte for byte.
    if !at.is_multiple_of(4) && end - at >= 2 {
        out.extend_from_slice(&0x0001u16.to_le_bytes());
        at += 2;
    }
    while end - at >= 4 {
        out.extend_from_slice(&0x0000_0013u32.to_le_bytes());
        at += 4;
    }
    if end - at == 2 {
        out.extend_from_slice(&0x0001u16.to_le_bytes());
    }
    out
}

/// The byte offset of every instruction in `ॱपाठ`, after relaxation and after
/// any `॥ संरेखः n ॥` padding.
///
/// `kosha` needs this to give a label its address. It used `l.at * 4`, which
/// assumes every instruction is four bytes — false for a compressed one, and
/// false for any label standing after alignment padding (`W-071`). A label
/// after a two-byte instruction was reported four bytes further along than it
/// is, so a debugger and a linker disagreed with the disassembly.
///
/// # The table is rebuilt AFTER the width pass, never before it
///
/// This loop cleared `symbols` at the top of every round (`W-241`). The width
/// pass then ran against an empty table every time, so no branch was ever seen
/// to reach a label and every one was laid out wide — under [`Target::Compressed`]
/// the six-instruction program in `tests/compressed.rs` was laid out at
/// `0 2 4 6 10 12 14` while [`encode_program_for`] emitted it at `0 2 4 6 8 10`.
/// Fifteen of the 82 spec programs were placed wrongly this way, by up to twelve
/// octets (`spec/schedule.sas`). Two loops in one crate disagreed about where an
/// instruction is, and the disagreement was found from the other language:
/// `encode.t1`'s `स्थानविन्यासः` rebuilds the table each round and iterates to
/// the fixpoint, and its test named the Rust answer as the wrong one.
///
/// The pass now asks its widths with the PREVIOUS round's table, exactly as
/// `encode_program_for` does, and the table is rebuilt from this round's
/// addresses once the pass is done. A forward branch is unknown on round ० and
/// wide; on round १ it is known and may shrink; the fixpoint settles when a
/// round changes nothing. `tests/paradigm_t0.rs` holds this answer against the
/// emitted bytes for every spec program at both targets.
#[must_use]
pub fn layout_addresses(program: &Program, target: Target) -> Vec<u32> {
    let mut symbols: BTreeMap<String, u32> = BTreeMap::new();
    let mut addresses: Vec<u32> = Vec::new();
    let mut previous: Vec<u32> = Vec::new();
    let mut pc = 0u32;
    for _ in 0..MAX_RELAXATION_ROUNDS {
        addresses.clear();
        pc = 0;
        for (i, inst) in program.instructions.iter().enumerate() {
            pc += align_pad_at(program, i, pc);
            addresses.push(pc);
            pc += width_of_for(inst, pc, &symbols, target);
        }
        // Rebuilt from THIS round's addresses, after the pass has used the
        // previous round's — see the doc comment above.
        symbols.clear();
        for l in &program.labels {
            if l.section == crate::parse::Section::Text {
                symbols.insert(l.name.clone(), addresses.get(l.at).copied().unwrap_or(pc));
            }
        }
        if addresses == previous {
            break;
        }
        previous.clone_from(&addresses);
    }
    // A label may sit at the very end of the program, where `at` equals the
    // instruction count and there is no instruction to borrow an address from.
    addresses.push(pc);
    addresses
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(src: &str) -> u32 {
        let p = crate::parse::assemble_program(src).expect("parses");
        words(&encode_program(&p).expect("encodes"))[0]
    }

    #[test]
    fn the_source_type_is_what_tells_two_conversions_apart() {
        // `B-075`, and the whole reason the second suffix exists. These two
        // write the same register from the same register: same shape, same
        // destination class, same result width. Only what they READ differs,
        // and before this the assembler had no way to be told which was meant
        // — it took whichever the encoding table listed first.
        let from_single = word("प्लवरूपान्तरम्ॱअ३२ॱप३२ अर्थ०म् प्लव१न ।");
        let from_double = word("प्लवरूपान्तरम्ॱअ३२ॱप६४ अर्थ०म् प्लव१न ।");
        assert_ne!(
            from_single, from_double,
            "reading a float32 and a float64 are different instructions"
        );
        // And each is the one asked for, not merely a different one. The exact
        // words are checked against `riscv64-elf-as` by the conformance corpus,
        // which now spells all eighteen.
        let name = |w| crate::vishlesana::decode(w).expect("decodes").insn;
        assert_eq!(name(from_single), "fcvt.w.s");
        assert_eq!(name(from_double), "fcvt.w.d");
    }

    #[test]
    fn a_conversion_without_its_pair_is_refused_and_told_what_to_write() {
        // Every one of the eighteen has the same shape, so counting registers
        // would say "no encoding taking 2 registers" — true of nothing, and
        // pointing the reader at the operands, which were fine.
        let p = crate::parse::assemble_program("प्लवरूपान्तरम्ॱअ३२ अर्थ०म् प्लव१न ।").expect("parses");
        let e = encode_program(&p).expect_err("must not encode");
        // The family and the list of pairs, both of which survive translation.
        // Asserting the PROSE would pin whichever language the compiler happens
        // to speak, which is what `B-015` moved diagnostics into a table to
        // stop — and asserting the CODE needs `EncodeError` to carry one, which
        // is `B-078b`.
        assert!(
            e[0].reason.contains("प्लवरूपान्तरम्"),
            "the family at fault: {}",
            e[0].reason
        );
        assert!(
            e[0].reason.contains("ॱअ३२ॱप६४"),
            "it lists the pairs: {}",
            e[0].reason
        );
    }

    #[test]
    fn only_a_conversion_takes_a_second_type() {
        // `योगः` reads and writes one type. A second suffix on it is a
        // misunderstanding, and silently ignoring it would encode an addition
        // the reader did not ask for.
        let p = crate::parse::assemble_program("योगःॱअ३२ॱप६४ अर्थ०म् अर्थ१न अर्थ२न ।").expect("parses");
        assert!(
            encode_program(&p).is_err(),
            "a second type on an add is refused"
        );
    }

    #[test]
    fn the_derived_displacement_map_reconstructs_what_the_assembler_emits() {
        // `B-055`. Branch and jump targets were recorded as `label:0x00000000`
        // — a slot with no bits — so no branch could be encoded and `jal`'s
        // mask claimed the whole 20-bit displacement as fixed opcode.
        //
        // The expected words below are from `riscv64-elf-as`, not from this
        // table: `beq x0,x0,.+8` is 0x00000463 because the assembler says so.
        // Rebuilding them from the derived pattern and bit map is what shows
        // the derivation captured the real B-type and J-type layouts, both of
        // which scatter the immediate and neither of which is written down
        // anywhere in this crate.
        let all = encodings();
        let find = |name: &str| {
            all.iter()
                .find(|e| e.insn == name)
                .unwrap_or_else(|| panic!("{name} missing from the table"))
        };

        for (name, disp, want) in [
            ("beq", 0u64, 0x0000_0063u32),
            ("beq", 4, 0x0000_0263),
            ("beq", 8, 0x0000_0463),
            ("beq", 16, 0x0000_0863),
            ("jal", 0, 0x0000_006f),
            ("jal", 4, 0x0040_006f),
        ] {
            let e = find(name);
            let slot = e
                .slots
                .iter()
                .find(|s| s.kind == "disp")
                .unwrap_or_else(|| panic!("{name} has no displacement slot"));
            assert!(!slot.map.is_empty(), "{name}'s displacement map is empty");
            let got = e.pattern | slot.place(disp);
            assert_eq!(
                got, want,
                "{name} .+{disp}: got {got:#010x}, as says {want:#010x}"
            );
        }
    }

    #[test]
    fn a_displacement_field_is_not_claimed_as_fixed_opcode() {
        // beq's mask was 0xfe007dff and jal's 0xffbff07f — both covering their
        // own displacement. Harmless while nothing decoded, fatal for B-013.
        for name in ["beq", "bne", "blt", "bge", "jal"] {
            let all = encodings();
            let e = all.iter().find(|e| e.insn == name).expect("present");
            let disp = e
                .slots
                .iter()
                .find(|s| s.kind == "disp")
                .expect("has a displacement");
            assert_ne!(disp.mask, 0, "{name}'s displacement has no bits");
        }
    }
    use crate::parse::assemble_source;

    fn one(src: &str) -> u32 {
        let is = assemble_source(src).expect("parses");
        encode(&is[0]).expect("encodes")
    }

    #[test]
    fn the_tables_load() {
        let e = encodings();
        assert!(e.len() > 150, "only {} encodings loaded", e.len());
        assert_eq!(register("शून्यः"), Some((0, false)));
        assert_eq!(register("क्षणिक३"), Some((28, false)), "t3 is x28");
        assert_eq!(register("प्लव३१"), Some((31, true)));
        assert_eq!(register("नास्ति"), None);
    }

    #[test]
    fn an_r_type_encodes_exactly() {
        // add x1, x2, x3 = 0x003100b3, the value tools/check-toolchain.sh
        // derives by hand from the pinned specification.
        // add t0, t1, t2 — x5, x6, x7. Registers ending in a numeral, because
        // a name ending in a virama consonant cannot take the म् sigil at all
        // (B-039); पुनःस्थानम्म् conjoins to one akṣara.
        let w = one("योगः क्षणिक०म् क्षणिक१न क्षणिक२न ।");
        assert_eq!(w, 0x0073_02b3, "got {w:#010x}");
    }

    #[test]
    fn the_destination_kara_ka_selects_rd_wherever_it_is_written() {
        // Free operand order is only real if the role, not the position,
        // decides the field.
        let a = one("योगः क्षणिक०म् क्षणिक१न क्षणिक२न ।");
        let b = one("योगः क्षणिक१न क्षणिक२न क्षणिक०म् ।");
        assert_eq!(a, b, "operand order changed the encoding");
    }

    #[test]
    fn an_immediate_form_is_selected_by_shape() {
        // addi x1, x2, 100 = 0x06410093
        // addi t0, t1, 100
        let w = one("योगः क्षणिक०म् क्षणिक१न १००न ।");
        assert_eq!(w, 0x0643_0293, "got {w:#010x}");
    }

    #[test]
    fn a_virama_final_register_encodes_as_a_destination() {
        // ADR-0005, end to end. पुनःस्थानम् is `ra`, and before that decision it
        // could not be written as a destination at all — its own final म्
        // conjoined with the कर्म sigil. add ra,t1,t2 = 0x007300b3.
        let w = one("योगः पुनःस्थानम्म् क्षणिक१न क्षणिक२न ।");
        assert_eq!(w, 0x0073_00b3, "got {w:#010x}");
    }

    #[test]
    fn a_value_too_wide_for_its_field_is_refused() {
        // Silent truncation would encode a different, valid number.
        let is = assemble_source("योगः क्षणिक०म् क्षणिक१न ९९९९९९९न ।").expect("parses");
        let e = encode(&is[0]).expect_err("must refuse");
        // The code, not the sentence: `B-078` put these in a table so the
        // wording can change, and this compiler speaks Sanskrit by default.
        assert_eq!(e.code, "E17");
        assert!(
            e.args.iter().any(|a| a == "९९९९९९९"),
            "the value at fault travels with the error: {:?}",
            e.args
        );
    }

    #[test]
    fn an_unknown_register_is_named() {
        let is = assemble_source("योगः गजःम् क्षणिक१न क्षणिक२न ।").expect("parses");
        let e = encode(&is[0]).expect_err("must refuse");
        assert!(e.reason.contains("गजः"), "{}", e.reason);
    }

    #[test]
    fn placing_walks_the_derived_map_with_no_shift_arithmetic() {
        // The S-type split: value bits 0..4 go to 7..11 and 5..10 to 25..30.
        let store = encodings()
            .iter()
            .find(|e| e.insn == "sd")
            .expect("sd encoding");
        let imm = store.immediate().expect("sd has an immediate");
        assert_eq!(imm.place(1), 1 << 7, "bit 0 -> bit 7");
        assert_eq!(imm.place(1 << 5), 1 << 25, "bit 5 -> bit 25");
        assert_eq!(imm.place(0b11_1111), (1 << 25) | (0b1_1111 << 7));
    }

    #[test]
    fn fits_accepts_the_full_signed_range_and_rejects_past_it() {
        let addi = encodings().iter().find(|e| e.insn == "addi").expect("addi");
        let imm = addi.immediate().expect("addi has an immediate");
        assert!(imm.fits(0));
        assert!(imm.fits(2047), "2047 is the largest 12-bit signed value");
        assert!(!imm.fits(1 << 20));
    }
}
