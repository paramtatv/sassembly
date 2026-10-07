//! **कोशः** — the ELF64 writer, task `B-010`.
//!
//! Doc 17 names the object format Kosha, "sheath, treasury": the sheath
//! compiled substance travels in.
//!
//! This writes a complete, bootable ELF64 for RISC-V from nothing but a list of
//! encoded bytes. **No GNU `ld` is involved**, which is the point — `BUILD.md`
//! B10 requires the linker to work without it, and an ELF writer that leaned on
//! `ld` to produce the headers would be deferring the whole question.
//!
//! # What is deliberately minimal
//!
//! One `PT_LOAD` segment for the text, `R E`, mapped at `0x8000_0000`, and — when the
//! image has data or `.bss` — a second, `RW`, at the next page ([`data_base`], `W-363`).
//!
//! The headers are deliberately not mapped, and that is not tidiness. With
//! `-bios none` the QEMU `virt` machine's reset vector jumps to `0x8000_0000`
//! unconditionally — it does not read `e_entry` — so whatever sits at that
//! address is what executes. A first version mapped the whole file there and
//! the machine spent its life executing `\x7fELF` as instructions, which hangs
//! rather than faults and looks exactly like a program with a bad loop.
//!
//! `p_align` is 4 rather than a page for the same reason: page alignment would
//! require `p_offset ≡ p_vaddr (mod 0x1000)`, which for a 120-byte header means
//! 4 KiB of padding to satisfy a constraint a bare-metal loader does not have.
//!
//! Section headers are written even though execution does not need them,
//! because they are what let `readelf` and `objdump` read the result — and an
//! object format nobody else can read cannot be checked against anyone else's
//! tools, which is this project's whole method.
//!
//! DWARF (`B-011`) and relocations (`B-014`) are absent.

extern crate alloc;

use alloc::vec::Vec;

/// Where the QEMU `virt` machine begins executing a `-kernel` image with
/// `-bios none`, and the default for every image this toolchain writes.
///
/// It is a default rather than the only answer (`C-001a1`). Booting under
/// OpenSBI means the firmware is already at this address, so a kernel has to be
/// somewhere else — 0x80200000 by the convention every RISC-V S-mode payload
/// uses. [`write_debuggable_at`] takes the address; this is what the callers
/// that do not care pass.
pub const LOAD_ADDRESS: u64 = 0x8000_0000;

const EHDR_SIZE: u64 = 64;
const PHDR_SIZE: u64 = 56;
const SHDR_SIZE: u64 = 64;

/// `EM_RISCV`.
const EM_RISCV: u16 = 243;

/// A page: where the writable segment begins, relative to the load address (`W-363`).
pub const PAGE: u64 = 4096;

/// Where `.data` begins relative to the load address, for `text_len` octets of text —
/// task `W-363`.
///
/// THE TEXT ROUNDED UP TO A PAGE, so the data and `.bss` can be their own `PT_LOAD`,
/// `PF_R | PF_W`, while the text stays `PF_R | PF_X`. Until `W-363` it was the text
/// rounded to eight and the one segment covering both was labelled `R E`: a loader that
/// installs protections from `p_flags` (`yantra::loader::load_application` does) faulted
/// on the first store to `.data`. A page and not eight because protections are per page,
/// and a data page that shared a frame with text would have to be one or the other.
///
/// ONE STATEMENT, read by the writer below and by [`crate::samyojana::link_at`]: the
/// linker gives every data name its address from this and the writer puts the segment
/// there, and two copies of the rule are how the two would come apart. The `.t1` twin is
/// `कोशॱदत्तपृष्ठाधारः`.
///
/// LATENT, RECORDED RATHER THAN REFUSED: the page is RELATIVE TO THE LOAD ADDRESS, so it
/// is a page boundary only when the load address is one. Every address this tree links
/// at is (`0x8000_0000`, `0x8020_0000`, `0x8040_0000`, `0x2000_0000`, the loader tests'
/// `0x1000_0000`), and both engines give the same answer at any address, so nothing
/// diverges — but an image linked at an unaligned address would carry a data segment
/// `load_application` refuses by name ("not a page boundary").
#[must_use]
pub const fn data_base(text_len: u64) -> u64 {
    text_len.next_multiple_of(PAGE)
}

// The section-name table is built from the sections that exist, so there is
// no fixed string blob and no fixed offsets into one (`B-071`).

/// One `Elf64_Sym` is twenty-four bytes.
const SYM_SIZE: u64 = 24;

/// The derived ABI flags, `spec/elf-abi-riscv64.tsv`.
const ABI: &str = include_str!("../../../spec/elf-abi-riscv64.tsv");

/// Whether the text holds a compressed instruction, read from the text.
///
/// `e_flags` should describe **this file**, not the switch that produced it.
/// Taking the target as a parameter would let the two drift: a build that asked
/// for compression and compressed nothing would still claim RVC, and — worse —
/// text assembled elsewhere and handed to this writer would be described by
/// whatever the caller happened to pass.
///
/// The length is in the low two bits of each instruction, which is what lets a
/// mixed-width stream be walked at all (`B-058b1`). So the file answers for
/// itself.
#[must_use]
fn text_has_compressed(text: &[u8]) -> bool {
    let mut at = 0usize;
    while at + 1 < text.len() {
        let half = u16::from_le_bytes([text[at], text[at + 1]]);
        let width = crate::vishlesana::width_of(half);
        if width == 2 {
            return true;
        }
        at += width;
    }
    false
}

/// `e_flags` for a target, derived rather than chosen.
///
/// `ld` refuses to merge objects whose float ABI disagrees — *"can't link
/// double-float modules with soft-float modules"*. Every image this writer has
/// ever produced said `0x0`, which claims soft float, so none of them could
/// have been linked against anything GNU built. Nothing noticed until one was
/// offered to a linker (`B-069b`).
///
/// `norvc` is `0x4` (double float) and `rvc` is `0x5` (double float plus
/// compressed). The uncompressed value is used here because that is the
/// default target; wiring the compressed one through is part of `B-069b2`.
#[must_use]
fn abi_flags(compressed: bool) -> u32 {
    let want = if compressed { "rvc" } else { "norvc" };
    ABI.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("target\t"))
        .filter_map(|l| l.split_once('\t'))
        .find(|(t, _)| *t == want)
        .and_then(|(_, v)| u32::from_str_radix(v.trim().trim_start_matches("0x"), 16).ok())
        .unwrap_or(0)
}

/// Where a symbol lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymSection {
    /// `.text` — code.
    Text,
    /// `.data` — initialised bytes.
    Data,
    /// `.bss` — reserved space the file does not carry.
    Bss,
    /// `SHN_UNDEF` — named here, defined elsewhere (`B-069b`).
    Undefined,
    /// The section symbol for `.debug_line` — where this unit's lines begin.
    ///
    /// A second section symbol, because `DW_AT_stmt_list` is an offset into
    /// `.debug_line` and `DW_AT_low_pc` is an address in `.text`. One symbol
    /// for both would give the linker the wrong base for one of them
    /// (`B-104b`).
    DebugLineSection,
    /// The section symbol for `.text` — `STT_SECTION`, no name of its own.
    ///
    /// A debug relocation names the SECTION rather than a label in it: the
    /// address `DW_LNE_set_address` carries is where the text begins, and a
    /// program with no label at offset zero has nothing else to point at
    /// (`B-100b`).
    TextSection,
}

/// A name the image should carry — task `B-063`.
///
/// Without these `nm` reports "no symbols" and a debugger has nothing to say
/// about a running program. An object format nobody else can read cannot be
/// checked against anyone else's tools, which is this project's method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    /// The label, as written.
    pub name: alloc::string::String,
    /// Its address.
    pub value: u64,
    /// Which section the name lives in.
    ///
    /// A bool was enough while there were two sections and became a lie when
    /// `ॱरिक्त` arrived: a buffer reported as `.data` makes `nm` print `d` for
    /// something the file does not contain.
    pub section: SymSection,
    /// True when `॥ वैश्विकम् … ॥` declared it visible outside this object.
    pub global: bool,
}

/// A name this object references and does not define.
///
/// `SymSection::Undefined` is `SHN_UNDEF`, which is what makes a symbol a
/// request rather than a definition — the linker's whole job is to answer it.
/// An executable has none; a relocatable object is mostly made of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relocation {
    /// Which section the offset is measured in.
    ///
    /// A record says *put this value at this offset*, and an offset means
    /// nothing without the section it counts from. `.rela.text` and
    /// `.rela.data` are two sections precisely because ELF makes a relocation
    /// section name the one it patches, in `sh_info` (`B-098`).
    pub section: RelSection,
    /// Byte offset into that section of the field to patch.
    pub offset: u64,
    /// Index into the symbol table written alongside, 1-based as ELF counts
    /// (index 0 is the null symbol).
    pub symbol: u32,
    /// A type from `spec/relocations-riscv64.tsv`, derived in `B-069a`.
    pub kind: u32,
    /// Added to the symbol's value before the field is written.
    pub addend: i64,
}

/// Which section a relocation patches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RelSection {
    /// `.text` — a call, or half of a pc-relative address.
    #[default]
    Text,
    /// `.data` — a pointer written into `ॱदत्त` (ADR-0013).
    Data,
    /// A debug section, by name: `.debug_line` or `.debug_info` (`B-100b`).
    Debug(&'static str),
}

/// Assemble the pieces of an object: what the file defines, what it asks for.
///
/// The symbol table is the file's own labels plus one `SHN_UNDEF` entry per
/// distinct unresolved name, and the relocations index into it. Building the
/// two together is what keeps the indices right — a record pointing at the
/// wrong symbol is a program that links and calls the wrong function.
#[must_use]
pub fn object(
    text: &[u8],
    program: &crate::parse::Program,
    pending: &[crate::encode::Pending],
    debug_for: Option<&str>,
    text_addresses: &[u32],
) -> Vec<u8> {
    let mut symbols: Vec<Symbol> = program
        .labels
        .iter()
        .map(|l| Symbol {
            name: l.name.clone(),
            // Its offset within its own section. This was zero for every
            // symbol, which put each one at its object's base — right only for
            // a label that happens to be first, which is what every test
            // program had (`B-069d2b`).
            // Its offset within its own section, taken from the layout rather
            // than assumed. `l.at * 4` was right only while every instruction
            // was four bytes and nothing but instructions sat in `ॱपाठ`;
            // a compressed instruction is two, and `॥ संरेखः n ॥` puts padding
            // between them (`W-071`).
            value: match l.section {
                crate::parse::Section::Text => {
                    u64::from(text_addresses.get(l.at).copied().unwrap_or(0))
                }
                _ => l.data_offset as u64,
            },
            section: match l.section {
                crate::parse::Section::Text => SymSection::Text,
                crate::parse::Section::Data => SymSection::Data,
                crate::parse::Section::Bss => SymSection::Bss,
            },
            global: program.globals.contains(&l.name),
        })
        .collect();

    let mut undefined: Vec<&str> = pending.iter().map(|p| p.name.as_str()).collect();
    undefined.sort_unstable();
    undefined.dedup();
    undefined.retain(|n| !symbols.iter().any(|s| s.name == *n));
    for name in &undefined {
        symbols.push(Symbol {
            name: (*name).to_string(),
            value: 0,
            section: SymSection::Undefined,
            global: true,
        });
    }

    // The section symbol goes in FIRST, before any index is computed.
    //
    // `B-100b` inserted it after `index_of` had been built and every text
    // relocation then named the symbol one place below the one it meant —
    // which is `B-069b2`'s lesson, *build the symbol table and the relocations
    // together*, broken again by adding a symbol later in the same function.
    // The `.text` section symbol is always present now (`B-105`). GNU's ABI
    // says an `R_RISCV_PCREL_LO12_I` names the address of its matching
    // `%pcrel_hi` rather than the target, and `ld` refuses an object whose
    // lo12 names the target — *dangerous relocation: %pcrel_lo missing
    // matching %pcrel_hi*. A section symbol plus the auipc's offset is that
    // address, and coins no label name.
    symbols.insert(
        0,
        Symbol {
            name: String::new(),
            value: 0,
            section: SymSection::TextSection,
            global: false,
        },
    );
    if debug_for.is_some() {
        // One more, because a unit points into two sections: `DW_AT_low_pc` is an
        // address in `.text` and `DW_AT_stmt_list` an offset into
        // `.debug_line`. They are told apart by their section, since a section
        // symbol has no name of its own.
        symbols.insert(
            0,
            Symbol {
                name: String::new(),
                value: 0,
                section: SymSection::DebugLineSection,
                global: false,
            },
        );
    }

    // The writer reorders locals before globals, so the index a relocation
    // needs is that order, not the order above.
    let mut ordered: Vec<&Symbol> = symbols.iter().filter(|s| !s.global).collect();
    ordered.extend(symbols.iter().filter(|s| s.global));
    // One map, not a scan per relocation: the fixpoint instrument's object has
    // ~72k data labels and ~36k relocations, and `position` over `ordered` for
    // each was an hour of the probe's Rust half (2026-09-14).
    let index_by_name: alloc::collections::BTreeMap<&str, u32> = ordered
        .iter()
        .enumerate()
        .map(|(i, s)| (s.name.as_str(), i as u32 + 1))
        .collect();
    let index_of = |name: &str| index_by_name.get(name).copied().unwrap_or(0);

    let text_section_symbol = ordered
        .iter()
        .position(|s| s.section == SymSection::TextSection)
        .map_or(0, |i| i as u32 + 1);

    let kinds = relocation_kinds();
    let relocations: Vec<Relocation> = pending
        .iter()
        .filter_map(|p| {
            // A `%pcrel_lo` names WHERE ITS `%pcrel_hi` IS, not what the pair
            // reaches: the linker follows the address to the hi20 record and
            // takes the target from there. Ours named the target, which is the
            // one thing that reads as a plain address and is not one, and GNU
            // `ld` refused every object containing a pair (`B-105`).
            //
            // The `auipc` is the instruction before, because `ॱअधः` completes
            // the register the preceding instruction set (`B-064`). That is
            // still our rule; this only writes it down where the ABI expects.
            let lo12 = p.kind == "R_RISCV_PCREL_LO12_I";
            Some(Relocation {
                section: p.section,
                offset: u64::from(p.at),
                symbol: if lo12 {
                    text_section_symbol
                } else {
                    index_of(&p.name)
                },
                kind: *kinds.get(p.kind)?,
                addend: if lo12 { i64::from(p.at) - 4 } else { 0 },
            })
        })
        .collect();

    let data: Vec<u8> = program.data.iter().flat_map(|d| d.bytes.clone()).collect();

    // `-g` (`B-100b`). The addresses are relative to `.text`, which is what an
    // object's addresses always are, and each is fixed by an `R_RISCV_64`
    // against the section symbol. Writing them without the records would read
    // correctly with `readelf` on the object and be silently wrong in every
    // program linked from it.
    let mut debug: Vec<(&'static str, Vec<u8>)> = Vec::new();
    let mut relocations = relocations;
    if let Some(file) = debug_for {
        let rows: Vec<crate::dwarf::Row> = program
            .instructions
            .iter()
            .enumerate()
            .map(|(i, inst)| crate::dwarf::Row {
                address: i as u64 * 4,
                line: inst.line as u32,
                file: 0,
            })
            .collect();
        let line = crate::dwarf::line_program_for(&[file], &rows, text.len() as u64, true);
        let unit = crate::dwarf::compile_unit_parts(file, 0, text.len() as u64);

        // Their indices. `index_of` matches on NAME and both are nameless, so
        // they are found by section instead — the same reason `vastu` resolves
        // a symbol's placement rather than trusting an index (`B-099`).
        let index_by_section = |want: SymSection| {
            ordered
                .iter()
                .position(|s| s.section == want)
                .map_or(0, |i| i as u32 + 1)
        };
        let section_symbol = index_by_section(SymSection::TextSection);
        let line_symbol = index_by_section(SymSection::DebugLineSection);
        let kinds = relocation_kinds();
        if let Some(kind) = kinds.get("R_RISCV_64").copied() {
            if let Some(at) = line.set_address_at {
                relocations.push(Relocation {
                    section: RelSection::Debug(".debug_line"),
                    offset: at as u64,
                    symbol: section_symbol,
                    kind,
                    addend: 0,
                });
            }
            relocations.push(Relocation {
                section: RelSection::Debug(".debug_info"),
                offset: unit.low_pc_at as u64,
                symbol: section_symbol,
                kind,
                addend: 0,
            });
        }
        // `DW_AT_stmt_list` is four bytes and an offset, not an address, so it
        // takes `R_RISCV_32` against the line section rather than the text.
        if let Some(kind) = kinds.get("R_RISCV_32").copied() {
            relocations.push(Relocation {
                section: RelSection::Debug(".debug_info"),
                offset: unit.stmt_list_at as u64,
                symbol: line_symbol,
                kind,
                addend: 0,
            });
        }
        debug.push((".debug_abbrev", unit.abbrev));
        debug.push((".debug_info", unit.info));
        debug.push((".debug_line", line.bytes));
    }

    // The largest boundary any `संरेखः` in `ॱपाठ` asked for, floored at the
    // instruction width. `W-080`: this is what survives linking.
    let text_align = program
        .text_aligns
        .iter()
        .map(|(_, n)| *n as u64)
        .chain(core::iter::once(4))
        .max()
        .unwrap_or(4);
    write_relocatable(
        text,
        &data,
        program.bss as u64,
        &debug,
        &symbols,
        &relocations,
        text_align,
    )
}

/// The derived relocation numbers, by name.
fn relocation_kinds() -> alloc::collections::BTreeMap<&'static str, u32> {
    const TABLE: &str = include_str!("../../../spec/relocations-riscv64.tsv");
    TABLE
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("name\t"))
        .filter_map(|l| {
            let (n, v) = l.split_once('\t')?;
            Some((n, v.trim().parse().ok()?))
        })
        .collect()
}

/// Build a **relocatable** object — task `B-069b`, doc 03 §3.3.
///
/// `ET_REL`, not `ET_EXEC`: no program header, no load address, and a
/// `.rela.text` saying which fields a linker must still fill in. This is what
/// lets a file be assembled once and linked many times, which `B-014` promised
/// and `बन्धकः` did not do — it re-encoded every unit at link time (`B-096b`).
///
/// # Why it is a separate writer
///
/// An executable and an object differ in nearly everything above the section
/// contents: one is mapped and has an entry point, the other is a bag of
/// sections with holes in it. Sharing the layout code would mean a function
/// where half the parameters are ignored depending on a flag, and the parts
/// they do share — the section-name table, the symbol table — are the parts
/// that were already extracted.
///
/// The addresses in `text` are relative to zero, because an object has no load
/// address; the linker chooses one.
#[must_use]
/// `text_align` is `sh_addralign` for `.text`, and it must be the largest
/// boundary `संरेखः` asked for rather than the instruction width (`W-080`).
///
/// It is what the LINKER is told, and a link may place `.text` at any multiple
/// of it — so an object claiming 4 while containing a 16-boundary has its
/// alignment silently undone the moment it is linked behind anything whose size
/// is not a multiple of 16. Measured: `॥ संरेखः १६ ॥` linked after a four-byte
/// object put the label at `0x...14`, while gas's `.balign 16`, which raises the
/// section to 16, put its own at `0x...20`. The alignment held inside the object
/// and did not survive linking — which is why `C-001e3a` had to round its arena
/// base at runtime rather than rely on the directive.
pub fn write_relocatable(
    text: &[u8],
    data: &[u8],
    bss: u64,
    debug: &[(&'static str, Vec<u8>)],
    symbols: &[Symbol],
    relocations: &[Relocation],
    text_align: u64,
) -> Vec<u8> {
    // Locals before globals, as `sh_info` promises.
    let mut ordered: Vec<&Symbol> = symbols.iter().filter(|s| !s.global).collect();
    let locals = ordered.len() + 1;
    ordered.extend(symbols.iter().filter(|s| s.global));

    let mut strtab = alloc::vec![0u8];
    let mut name_offsets = Vec::with_capacity(ordered.len());
    for s in &ordered {
        name_offsets.push(strtab.len() as u32);
        strtab.extend_from_slice(s.name.as_bytes());
        strtab.push(0);
    }

    let mut names: Vec<u8> = alloc::vec![0u8];
    let name_of = |names: &mut Vec<u8>, s: &str| -> u32 {
        let at = names.len() as u32;
        names.extend_from_slice(s.as_bytes());
        names.push(0);
        at
    };
    // Split by what each record patches. Two sections, because that is how ELF
    // says which: `sh_info` on a `SHT_RELA` names the section it applies to,
    // and one list would leave a data offset indistinguishable from a text one
    // (`B-098`).
    // Three ways, not two. `partition` on "is it Data" swept every
    // `Debug(..)` record into `.rela.text` as well as its own section, so an
    // object built with `-g` carried two `R_RISCV_64` records pointing tens of
    // bytes past the end of its text — written by `B-100b`, found by `B-104`
    // trying to link one. A two-way split over a three-way enum has no
    // compiler to catch it, which is the argument for matching.
    let mut text_rela: Vec<&Relocation> = Vec::new();
    let mut data_rela: Vec<&Relocation> = Vec::new();
    for r in relocations {
        match r.section {
            RelSection::Text => text_rela.push(r),
            RelSection::Data => data_rela.push(r),
            RelSection::Debug(_) => {}
        }
    }

    // The sections this object will carry, in the order they are written. One
    // list rather than a run of `extra`/`after` offsets: every index below is
    // its position here, so adding a conditional section cannot leave one of
    // them pointing at its neighbour. `B-071` shipped that bug in the
    // executable writer — a name table whose position was derived from a
    // section that had not been emitted — and the arithmetic here had grown two
    // conditionals with a third (`.debug_line`, `B-100b`) coming.
    let mut plan: Vec<&str> = alloc::vec![".text", ".rela.text", ".data"];
    if !data_rela.is_empty() {
        plan.push(".rela.data");
    }
    if bss > 0 {
        plan.push(".bss");
    }
    // Each debug section, and a `.rela.` for it when it has records. The names
    // are borrowed from `debug`, which outlives this call.
    let debug_rela: Vec<&str> = debug
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| {
            relocations
                .iter()
                .any(|r| r.section == RelSection::Debug(name))
        })
        .collect();
    let rela_names: Vec<alloc::string::String> = debug_rela
        .iter()
        .map(|n| alloc::format!(".rela{n}"))
        .collect();
    for (i, (name, _)) in debug.iter().enumerate() {
        plan.push(name);
        let _ = i;
    }
    for n in &rela_names {
        plan.push(n);
    }
    plan.extend([".symtab", ".strtab", ".shstrtab"]);

    // Index 0 is the null header, so a section's index is its place plus one.
    let index_of = |name: &str| -> u16 {
        plan.iter()
            .position(|s| *s == name)
            .map_or(0, |i| i as u16 + 1)
    };
    let (i_text, i_data) = (index_of(".text"), index_of(".data"));
    let i_bss = (bss > 0).then(|| index_of(".bss"));
    let (i_symtab, i_strtab, i_shstrtab) = (
        index_of(".symtab"),
        index_of(".strtab"),
        index_of(".shstrtab"),
    );
    let shnum = plan.len() as u16 + 1;

    let mut n_of: alloc::collections::BTreeMap<&str, u32> = alloc::collections::BTreeMap::new();
    for s in &plan {
        n_of.insert(s, name_of(&mut names, s));
    }
    let named = |s: &str| n_of.get(s).copied().unwrap_or(0);
    let (n_text, n_rela, n_data) = (named(".text"), named(".rela.text"), named(".data"));
    let n_rela_data = (!data_rela.is_empty()).then(|| named(".rela.data"));
    let n_bss = (bss > 0).then(|| named(".bss"));
    let (n_symtab, n_strtab, n_shstrtab) = (named(".symtab"), named(".strtab"), named(".shstrtab"));

    // Where each section's bytes land. Written in plan order, so a section
    // added to the list above is added here and nowhere else.
    const RELA_SIZE: u64 = 24;
    let text_offset = EHDR_SIZE;
    let text_size = text.len() as u64;
    let rela_offset = (text_offset + text_size).next_multiple_of(8);
    let rela_size = text_rela.len() as u64 * RELA_SIZE;
    let data_offset = rela_offset + rela_size;
    let data_size = data.len() as u64;
    let rela_data_offset = (data_offset + data_size).next_multiple_of(8);
    let rela_data_size = data_rela.len() as u64 * RELA_SIZE;
    let symtab_offset = (rela_data_offset + rela_data_size).next_multiple_of(8);
    let symtab_size = (ordered.len() as u64 + 1) * SYM_SIZE;
    let strtab_offset = symtab_offset + symtab_size;
    let strtab_size = strtab.len() as u64;
    let shstrtab_offset = strtab_offset + strtab_size;
    let shstrtab_size = names.len() as u64;

    // The debug sections go LAST in the file, whatever their place among the
    // headers. A section header says where its bytes are, so file order and
    // header order are independent — and laying them here leaves every offset
    // above exactly where it was, which is what makes an object without `-g`
    // byte-identical to the one before this task.
    let mut debug_at: Vec<(u64, u64)> = Vec::with_capacity(debug.len());
    let mut at = shstrtab_offset + shstrtab_size;
    for (_, bytes) in debug {
        at = at.next_multiple_of(8);
        debug_at.push((at, bytes.len() as u64));
        at += bytes.len() as u64;
    }
    let mut debug_rela_at: Vec<(u64, u64)> = Vec::with_capacity(debug_rela.len());
    for name in &debug_rela {
        let n = relocations
            .iter()
            .filter(|r| r.section == RelSection::Debug(name))
            .count() as u64;
        at = at.next_multiple_of(8);
        debug_rela_at.push((at, n * RELA_SIZE));
        at += n * RELA_SIZE;
    }
    let shoff = at.next_multiple_of(8);

    let mut out = Vec::with_capacity(shoff as usize + shnum as usize * SHDR_SIZE as usize);

    out.extend_from_slice(&[0x7f, b'E', b'L', b'F']);
    out.push(2); // ELFCLASS64
    out.push(1); // ELFDATA2LSB
    out.push(1); // EV_CURRENT
    out.extend_from_slice(&[0u8; 9]);
    push_u16(&mut out, 1); // ET_REL — the whole point
    push_u16(&mut out, EM_RISCV);
    push_u32(&mut out, 1);
    push_u64(&mut out, 0); // no entry point; an object is not started
    push_u64(&mut out, 0); // no program headers
    push_u64(&mut out, shoff);
    // An object is emitted uncompressed today, and says so rather than
    // claiming a target it was not built for.
    push_u32(&mut out, abi_flags(text_has_compressed(text)));
    push_u16(&mut out, EHDR_SIZE as u16);
    push_u16(&mut out, 0); // e_phentsize
    push_u16(&mut out, 0); // e_phnum
    push_u16(&mut out, SHDR_SIZE as u16);
    push_u16(&mut out, shnum);
    push_u16(&mut out, i_shstrtab);

    debug_assert_eq!(out.len() as u64, text_offset);
    out.extend_from_slice(text);
    while (out.len() as u64) < rela_offset {
        out.push(0);
    }
    // `r_info` packs the symbol index into the high 32 bits and the type into
    // the low 32. Getting the halves the wrong way round produces a file that
    // reads as relocation type 9 against symbol 17 — plausible, and wrong.
    let rela = |out: &mut Vec<u8>, r: &Relocation| {
        push_u64(out, r.offset);
        push_u64(out, u64::from(r.symbol) << 32 | u64::from(r.kind));
        push_u64(out, r.addend as u64);
    };
    for r in &text_rela {
        rela(&mut out, r);
    }
    out.extend_from_slice(data);
    while (out.len() as u64) < rela_data_offset {
        out.push(0);
    }
    for r in &data_rela {
        rela(&mut out, r);
    }
    while (out.len() as u64) < symtab_offset {
        out.push(0);
    }
    out.extend_from_slice(&[0u8; SYM_SIZE as usize]);
    for (s, name) in ordered.iter().zip(&name_offsets) {
        push_u32(&mut out, *name);
        let bind = u8::from(s.global);
        let kind = match s.section {
            SymSection::Undefined => 0u8, // STT_NOTYPE: nothing is known of it
            SymSection::Text => 2,
            SymSection::TextSection | SymSection::DebugLineSection => 3, // STT_SECTION
            _ => 1,
        };
        out.push(bind << 4 | kind);
        out.push(0);
        push_u16(
            &mut out,
            match s.section {
                SymSection::Text | SymSection::TextSection => i_text,
                SymSection::DebugLineSection => index_of(".debug_line"),
                SymSection::Data => i_data,
                // Its own section at last. A buffer reported as `.data` is a
                // claim the file does not support, and `nm` printed `d` for a
                // name that belongs in `.bss`.
                SymSection::Bss => i_bss.unwrap_or(i_data),
                SymSection::Undefined => 0,
            },
        );
        push_u64(&mut out, s.value);
        push_u64(&mut out, 0);
    }
    out.extend_from_slice(&strtab);
    out.extend_from_slice(&names);
    for ((offset, _), (_, bytes)) in debug_at.iter().zip(debug) {
        while (out.len() as u64) < *offset {
            out.push(0);
        }
        out.extend_from_slice(bytes);
    }
    for ((offset, _), name) in debug_rela_at.iter().zip(&debug_rela) {
        while (out.len() as u64) < *offset {
            out.push(0);
        }
        for r in relocations
            .iter()
            .filter(|r| r.section == RelSection::Debug(name))
        {
            rela(&mut out, r);
        }
    }
    while !(out.len() as u64).is_multiple_of(8) {
        out.push(0);
    }

    debug_assert_eq!(out.len() as u64, shoff);
    let mut shdr = |name, kind, flags, offset, size, link, info, align, entsize| {
        push_u32(&mut out, name);
        push_u32(&mut out, kind);
        push_u64(&mut out, flags);
        push_u64(&mut out, 0); // sh_addr — an object is not placed anywhere
        push_u64(&mut out, offset);
        push_u64(&mut out, size);
        push_u32(&mut out, link);
        push_u32(&mut out, info);
        push_u64(&mut out, align);
        push_u64(&mut out, entsize);
    };

    shdr(0, 0, 0, 0, 0, 0, 0, 0, 0);
    shdr(
        n_text,
        1,
        0b110,
        text_offset,
        text_size,
        0,
        0,
        text_align,
        0,
    );
    // SHT_RELA. `sh_link` is the symbol table it indexes and `sh_info` the
    // section it patches; a reader that trusted either and found it wrong would
    // apply the right relocation to the wrong bytes.
    shdr(
        n_rela,
        4,
        0,
        rela_offset,
        rela_size,
        u32::from(i_symtab),
        u32::from(i_text),
        8,
        RELA_SIZE,
    );
    shdr(n_data, 1, 0b011, data_offset, data_size, 0, 0, 8, 0);
    // `.rela.data` — the pointers ADR-0013 lets `ॱदत्त` hold. Same shape as
    // `.rela.text`; only `sh_info` differs, and that is the whole reason there
    // are two of them.
    if let Some(name) = n_rela_data {
        shdr(
            name,
            4,
            0,
            rela_data_offset,
            rela_data_size,
            u32::from(i_symtab),
            u32::from(i_data),
            8,
            RELA_SIZE,
        );
    }
    // SHT_NOBITS occupies no file space, so its offset is only where it would
    // have been. `sh_addr` is zero here: an object is not placed anywhere.
    if let Some(name) = n_bss {
        shdr(name, 8, 0b011, symtab_offset, bss, 0, 0, 8, 0);
    }
    // The debug sections, in plan order. `SHF_NONE`: they are not loaded.
    for ((offset, size), (name, _)) in debug_at.iter().zip(debug) {
        shdr(named(name), 1, 0, *offset, *size, 0, 0, 1, 0);
    }
    for (((offset, size), name), _) in debug_rela_at.iter().zip(&debug_rela).zip(0..) {
        shdr(
            named(&alloc::format!(".rela{name}")),
            4,
            0,
            *offset,
            *size,
            u32::from(i_symtab),
            u32::from(index_of(name)),
            8,
            RELA_SIZE,
        );
    }
    shdr(
        n_symtab,
        2,
        0,
        symtab_offset,
        symtab_size,
        u32::from(i_strtab),
        locals as u32,
        8,
        SYM_SIZE,
    );
    shdr(n_strtab, 3, 0, strtab_offset, strtab_size, 0, 0, 1, 0);
    shdr(n_shstrtab, 3, 0, shstrtab_offset, shstrtab_size, 0, 0, 1, 0);
    out
}

fn push_u16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn push_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn push_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Build a bootable ELF64 image from encoded instruction bytes.
///
/// The entry point is the first instruction, which sits immediately after the
/// headers.
#[must_use]
pub fn write(text: &[u8]) -> Vec<u8> {
    write_with_data(text, &[])
}

/// Build a bootable image with an initialised data section — task `B-057`.
///
/// `.data` is its own `PT_LOAD`, `PF_R | PF_W`, at the page after the text
/// ([`data_base`], `W-363`). It shared the text's `R E` segment until a loader that
/// honours `p_flags` existed (`yantra::loader::load_application`) and faulted on the
/// first store; the address moves to the page, the file stays packed.
#[must_use]
pub fn write_with_data(text: &[u8], data: &[u8]) -> Vec<u8> {
    write_image(text, data, &[])
}

/// Build a bootable image carrying a symbol table — task `B-063`.
///
/// Locals are written before globals because the format requires it: a section
/// header's `sh_info` is the index of the first non-local symbol, and a reader
/// that trusts it will mis-classify every symbol if the order is wrong.
#[must_use]
fn write_image(text: &[u8], data: &[u8], symbols: &[Symbol]) -> Vec<u8> {
    write_full(text, data, symbols, 0)
}

/// Build an image reserving `bss` bytes that exist in memory and not in the
/// file — task `B-068`.
///
/// This is the whole of what `.bss` is: `p_memsz` larger than `p_filesz`, so
/// the loader zeroes the difference. A zeroed buffer in `.data` costs its own
/// size on disk, and with the hello-world budget enforced (`B-022`) that is a
/// cost the format is designed to avoid paying.
#[must_use]
pub fn write_full(text: &[u8], data: &[u8], symbols: &[Symbol], bss: u64) -> Vec<u8> {
    write_debuggable(text, data, symbols, bss, &[])
}

/// As [`write_full`], plus any number of debug sections — tasks `B-011a`,
/// `B-011b`.
///
/// The bytes come from [`crate::dwarf`]. It is a separate entry point rather
/// than another parameter on `write_full` because debug info is exactly the
/// kind of mass doc 18 §0.2 says must be removable: a build that does not ask
/// for it pays nothing, not even an empty section header.
///
/// Sections are written in the order given, and each is `SHT_PROGBITS` with no
/// flags — in the file, never in memory.
#[must_use]
pub fn write_debuggable(
    text: &[u8],
    data: &[u8],
    symbols: &[Symbol],
    bss: u64,
    debug: &[(&str, Vec<u8>)],
) -> Vec<u8> {
    write_debuggable_at(text, data, symbols, bss, debug, LOAD_ADDRESS)
}

/// As [`write_debuggable`], at a chosen load address — task `C-001a1`.
///
/// Every address in the file is derived from this one: `e_entry`, the segment's
/// `p_vaddr` and `p_paddr`, and each section header's `sh_addr`. The TEXT is
/// not: `ॱउपरि`/`ॱअधः` measure from the instruction carrying them, so the bytes
/// a program assembles to are the same wherever it is placed, and a test says
/// so. That is what makes moving an image a header change rather than a
/// re-encode.
///
/// The symbol values passed in are already absolute, so they must have been
/// computed against the same address — [`crate::samyojana::link_at`] takes it
/// for that reason.
#[must_use]
pub fn write_debuggable_at(
    text: &[u8],
    data: &[u8],
    symbols: &[Symbol],
    bss: u64,
    debug: &[(&str, Vec<u8>)],
    load: u64,
) -> Vec<u8> {
    // TWO SEGMENTS WHEN THERE IS ANYTHING TO WRITE (`W-363`): the text, `PF_R | PF_X`,
    // and the data with `.bss`, `PF_R | PF_W`, at the next page. An image with neither
    // data nor `.bss` has nothing writable and keeps the one header.
    let writable = !data.is_empty() || bss > 0;
    let phnum: u16 = if writable { 2 } else { 1 };
    let text_offset = EHDR_SIZE + PHDR_SIZE * u64::from(phnum);
    let text_size = text.len() as u64;
    // In the FILE the data stays packed behind the text, eight-aligned: a page of
    // padding would buy nothing a bare-metal loader reads. Only its ADDRESS moves to the
    // page, which is why `p_align` below is eight, the alignment both agree on.
    let data_offset = if writable {
        (text_offset + text_size).next_multiple_of(8)
    } else {
        text_offset + text_size
    };
    let data_size = data.len() as u64;
    let data_addr = load + data_base(text_size);

    // Locals first: `sh_info` is the index of the first non-local symbol and a
    // reader that trusts it mis-classifies everything if the order is wrong.
    let mut ordered: Vec<&Symbol> = symbols.iter().filter(|s| !s.global).collect();
    let locals = ordered.len() + 1; // +1 for the null symbol at index 0
    ordered.extend(symbols.iter().filter(|s| s.global));

    let mut strtab = alloc::vec![0u8];
    let mut name_offsets = Vec::with_capacity(ordered.len());
    for s in &ordered {
        name_offsets.push(strtab.len() as u32);
        strtab.extend_from_slice(s.name.as_bytes());
        strtab.push(0);
    }

    let symtab_offset = (data_offset + data_size).next_multiple_of(8);
    let symtab_size = if symbols.is_empty() {
        0
    } else {
        (ordered.len() as u64 + 1) * SYM_SIZE
    };
    let strtab_offset = symtab_offset + symtab_size;
    let strtab_size = if symbols.is_empty() {
        0
    } else {
        strtab.len() as u64
    };

    // --- which sections exist ----------------------------------------------
    //
    // Only the ones with something in them. An empty section header is 64
    // bytes plus its name, and `B-068` shipped a `.bss` header on every image
    // whether or not it reserved anything — 13% of a 560-byte hello world.
    // Doc 18 §0.2's rule is that removable mass is removed rather than
    // budgeted, so the table is built from what is actually present.
    let has_data = !data.is_empty();
    let has_bss = bss > 0;
    let has_syms = !symbols.is_empty();
    let debug: Vec<&(&str, Vec<u8>)> = debug.iter().filter(|(_, b)| !b.is_empty()).collect();

    let mut names: Vec<u8> = alloc::vec![0u8];
    let name_of = |names: &mut Vec<u8>, s: &str| -> u32 {
        let at = names.len() as u32;
        names.extend_from_slice(s.as_bytes());
        names.push(0);
        at
    };
    let n_text = name_of(&mut names, ".text");
    let n_data = if has_data {
        name_of(&mut names, ".data")
    } else {
        0
    };
    let n_bss = if has_bss {
        name_of(&mut names, ".bss")
    } else {
        0
    };
    let (n_symtab, n_strtab) = if has_syms {
        (
            name_of(&mut names, ".symtab"),
            name_of(&mut names, ".strtab"),
        )
    } else {
        (0, 0)
    };
    let n_debug: Vec<u32> = debug
        .iter()
        .map(|(name, _)| name_of(&mut names, name))
        .collect();
    let n_shstrtab = name_of(&mut names, ".shstrtab");

    // Indices, assigned in the order the headers are written.
    let mut next = 1u16;
    let i_text = next;
    next += 1;
    let i_data = if has_data {
        let i = next;
        next += 1;
        i
    } else {
        0
    };
    let i_bss = if has_bss {
        let i = next;
        next += 1;
        i
    } else {
        0
    };
    let (_i_symtab, i_strtab) = if has_syms {
        let a = next;
        next += 2;
        (a, a + 1)
    } else {
        (0, 0)
    };
    next += debug.len() as u16;
    let i_shstrtab = next;
    let shnum = next + 1;

    // Where the contents actually end. Without symbols nothing pads to
    // `symtab_offset`, so deriving this from it put the name table 0-7 bytes
    // past the end of what was written.
    let content_end = if has_syms {
        strtab_offset + strtab_size
    } else {
        data_offset + data_size
    };
    // Debug sections are byte streams with no alignment requirement, so each
    // goes straight after the last.
    let mut debug_at = Vec::with_capacity(debug.len());
    let mut at = content_end;
    for (_, bytes) in &debug {
        debug_at.push(at);
        at += bytes.len() as u64;
    }
    let shstrtab_offset = at;
    let shstrtab_size = names.len() as u64;
    let shoff = (shstrtab_offset + shstrtab_size).next_multiple_of(8);

    let mut out = Vec::with_capacity(shoff as usize + shnum as usize * SHDR_SIZE as usize);

    // --- ELF header --------------------------------------------------------
    out.extend_from_slice(&[0x7f, b'E', b'L', b'F']);
    out.push(2); // ELFCLASS64
    out.push(1); // ELFDATA2LSB
    out.push(1); // EV_CURRENT
    out.push(0); // ELFOSABI_NONE
    out.extend_from_slice(&[0u8; 8]);
    push_u16(&mut out, 2); // ET_EXEC
    push_u16(&mut out, EM_RISCV);
    push_u32(&mut out, 1);
    push_u64(&mut out, load); // e_entry — the reset vector goes here
    push_u64(&mut out, EHDR_SIZE); // e_phoff
    push_u64(&mut out, shoff);
    push_u32(&mut out, abi_flags(text_has_compressed(text))); // e_flags
    push_u16(&mut out, EHDR_SIZE as u16);
    push_u16(&mut out, PHDR_SIZE as u16);
    push_u16(&mut out, phnum); // e_phnum
    push_u16(&mut out, SHDR_SIZE as u16);
    push_u16(&mut out, shnum);
    push_u16(&mut out, i_shstrtab);

    // --- program headers ---------------------------------------------------
    // The text: read and execute, never write.
    push_u32(&mut out, 1); // PT_LOAD
    push_u32(&mut out, 0b101); // PF_R | PF_X
    push_u64(&mut out, text_offset);
    push_u64(&mut out, load);
    push_u64(&mut out, load);
    push_u64(&mut out, text_size); // p_filesz
    push_u64(&mut out, text_size); // p_memsz
    push_u64(&mut out, 4); // p_align
    if writable {
        // The data and `.bss`: read and write, never execute (`W-363`).
        push_u32(&mut out, 1); // PT_LOAD
        push_u32(&mut out, 0b110); // PF_R | PF_W
        push_u64(&mut out, data_offset);
        push_u64(&mut out, data_addr);
        push_u64(&mut out, data_addr);
        push_u64(&mut out, data_size); // p_filesz
        // p_memsz — the loader zeroes the difference, which is `ॱरिक्त`.
        push_u64(&mut out, data_size.next_multiple_of(8) + bss);
        push_u64(&mut out, 8); // p_align
    }

    // --- contents ----------------------------------------------------------
    debug_assert_eq!(out.len() as u64, text_offset);
    out.extend_from_slice(text);
    while (out.len() as u64) < data_offset {
        out.push(0);
    }
    out.extend_from_slice(data);

    if has_syms {
        while (out.len() as u64) < symtab_offset {
            out.push(0);
        }
        out.extend_from_slice(&[0u8; SYM_SIZE as usize]); // STN_UNDEF
        for (s, name) in ordered.iter().zip(&name_offsets) {
            push_u32(&mut out, *name);
            let bind = u8::from(s.global);
            let kind = if s.section == SymSection::Text {
                2u8
            } else {
                1
            };
            out.push(bind << 4 | kind);
            out.push(0);
            push_u16(
                &mut out,
                match s.section {
                    // An executable carries no section symbols; one arriving
                    // here would be an object's symbol table in the wrong
                    // writer, so it is placed in .text rather than dropped.
                    SymSection::Text | SymSection::TextSection => i_text,
                    // An executable carries no section symbols at all.
                    SymSection::DebugLineSection => i_text,
                    SymSection::Data => i_data,
                    SymSection::Bss => i_bss,
                    SymSection::Undefined => 0,
                },
            );
            push_u64(&mut out, s.value);
            push_u64(&mut out, 0); // st_size — needs the linker's layout
        }
        out.extend_from_slice(&strtab);
    }

    for (_, bytes) in &debug {
        out.extend_from_slice(bytes);
    }

    out.extend_from_slice(&names);
    while !(out.len() as u64).is_multiple_of(8) {
        out.push(0);
    }

    // --- section headers ---------------------------------------------------
    debug_assert_eq!(out.len() as u64, shoff);
    let mut shdr = |name, kind, flags, addr, offset, size, link, info, align, entsize| {
        push_u32(&mut out, name);
        push_u32(&mut out, kind);
        push_u64(&mut out, flags);
        push_u64(&mut out, addr);
        push_u64(&mut out, offset);
        push_u64(&mut out, size);
        push_u32(&mut out, link);
        push_u32(&mut out, info);
        push_u64(&mut out, align);
        push_u64(&mut out, entsize);
    };

    shdr(0, 0, 0, 0, 0, 0, 0, 0, 0, 0); // SHN_UNDEF
    shdr(n_text, 1, 0b110, load, text_offset, text_size, 0, 0, 4, 0);
    if has_data {
        shdr(
            n_data,
            1,
            0b011,
            data_addr,
            data_offset,
            data_size,
            0,
            0,
            8,
            0,
        );
    }
    if has_bss {
        let addr = data_addr + data_size.next_multiple_of(8);
        // SHT_NOBITS: it occupies no file space, so its offset is only where
        // it would have been.
        shdr(
            n_bss,
            8,
            0b011,
            addr,
            data_offset + data_size,
            bss,
            0,
            0,
            8,
            0,
        );
    }
    if has_syms {
        shdr(
            n_symtab,
            2,
            0,
            0,
            symtab_offset,
            symtab_size,
            u32::from(i_strtab),
            locals as u32,
            8,
            SYM_SIZE,
        );
        shdr(n_strtab, 3, 0, 0, strtab_offset, strtab_size, 0, 0, 1, 0);
    }
    for (n, ((_, bytes), name)) in debug.iter().zip(&n_debug).enumerate() {
        // SHT_PROGBITS, no flags: in the file and never in memory.
        shdr(*name, 1, 0, 0, debug_at[n], bytes.len() as u64, 0, 0, 1, 0);
    }
    shdr(
        n_shstrtab,
        3,
        0,
        0,
        shstrtab_offset,
        shstrtab_size,
        0,
        0,
        1,
        0,
    );

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Find a section header by name, because indices move when a section is
    /// omitted (`B-071`) and a test that hardcodes one breaks for the wrong
    /// reason.
    fn section(elf: &[u8], want: &str) -> Option<usize> {
        let shoff = u64::from_le_bytes(elf[40..48].try_into().ok()?) as usize;
        let shnum = u16::from_le_bytes(elf[60..62].try_into().ok()?) as usize;
        let shstrndx = u16::from_le_bytes(elf[62..64].try_into().ok()?) as usize;
        let strs = shoff + shstrndx * SHDR_SIZE as usize;
        let strs_off = u64::from_le_bytes(elf[strs + 24..strs + 32].try_into().ok()?) as usize;
        (0..shnum)
            .find(|i| {
                let sh = shoff + i * SHDR_SIZE as usize;
                let n = u32::from_le_bytes(elf[sh..sh + 4].try_into().unwrap_or_default()) as usize;
                elf[strs_off + n..]
                    .split(|b| *b == 0)
                    .next()
                    .is_some_and(|s| s == want.as_bytes())
            })
            .map(|i| shoff + i * SHDR_SIZE as usize)
    }

    #[test]
    fn the_header_says_what_it_is() {
        let elf = write(&[0x0000_0013]);
        assert_eq!(&elf[..4], b"\x7fELF");
        assert_eq!(elf[4], 2, "ELFCLASS64");
        assert_eq!(elf[5], 1, "little endian");
        assert_eq!(u16::from_le_bytes([elf[16], elf[17]]), 2, "ET_EXEC");
        assert_eq!(u16::from_le_bytes([elf[18], elf[19]]), EM_RISCV);
    }

    /// `e_flags` out of a written image.
    fn flags_of(elf: &[u8]) -> u32 {
        u32::from_le_bytes(elf[48..52].try_into().expect("4 bytes"))
    }

    #[test]
    fn the_header_describes_the_file_and_not_the_switch() {
        // `B-069c`. `e_flags` said 0 — soft float — on every image until
        // `B-069b1`, and `ld` refused to merge one. It is derived now, and it
        // is derived from the TEXT rather than from a target parameter, so a
        // build that asked for compression and compressed nothing cannot
        // claim RVC.
        let wide: Vec<u8> = core::iter::repeat_n(0x0000_0013u32.to_le_bytes(), 3)
            .flatten()
            .collect();
        // `c.addi s0, 1`, per riscv64-elf-as with `.option rvc`.
        let mut mixed = wide.clone();
        mixed.extend_from_slice(&0x0405u16.to_le_bytes());

        assert!(!text_has_compressed(&wide));
        assert!(text_has_compressed(&mixed));
        assert_eq!(flags_of(&write(&wide)), abi_flags(false), "double float");
        assert_eq!(
            flags_of(&write(&mixed)),
            abi_flags(true),
            "double float plus RVC"
        );
        assert_ne!(
            abi_flags(false),
            abi_flags(true),
            "the two targets must be distinguishable, or the field says nothing"
        );
    }

    #[test]
    fn the_entry_point_is_the_first_instruction() {
        // `addi zero, zero, 0` twice, as the BYTES the encoder now produces.
        let nop = 0x0000_0013u32.to_le_bytes();
        let text: Vec<u8> = nop.iter().chain(nop.iter()).copied().collect();
        let elf = write(&text);
        let entry = u64::from_le_bytes(elf[24..32].try_into().expect("8 bytes"));
        assert_eq!(
            entry, LOAD_ADDRESS,
            "the reset vector goes here, not to e_entry"
        );
        // And the word at that offset is the first one given.
        let at = (EHDR_SIZE + PHDR_SIZE) as usize;
        assert_eq!(
            u32::from_le_bytes(elf[at..at + 4].try_into().expect("4 bytes")),
            0x0000_0013
        );
    }

    #[test]
    fn the_segment_covers_every_byte_it_claims() {
        // A p_filesz longer than the file is how an image loads garbage; one
        // shorter silently truncates the program.
        let text: Vec<u8> = core::iter::repeat_n(0x0000_0013u32.to_le_bytes(), 5)
            .flatten()
            .collect();
        let elf = write(&text);
        let ph = EHDR_SIZE as usize;
        let offset = u64::from_le_bytes(elf[ph + 8..ph + 16].try_into().expect("8 bytes"));
        let filesz = u64::from_le_bytes(elf[ph + 32..ph + 40].try_into().expect("8 bytes"));
        assert_eq!(filesz, 20, "five words of text and no headers");
        assert_eq!(
            offset,
            EHDR_SIZE + PHDR_SIZE,
            "the segment starts at the text"
        );
        assert!(
            (offset + filesz) as usize <= elf.len(),
            "the segment runs past the end of the file"
        );
    }

    #[test]
    fn a_symbol_table_puts_locals_before_globals() {
        // `sh_info` is the index of the first non-local symbol, and a reader
        // that trusts it mis-classifies everything if the order is wrong. So
        // the order is not incidental — it is the format's contract.
        use alloc::string::ToString;
        let syms = [
            Symbol {
                name: "वैश्विक".to_string(),
                value: 0x8000_0000,
                section: SymSection::Text,
                global: true,
            },
            Symbol {
                name: "स्थानीय".to_string(),
                value: 0x8000_0004,
                section: SymSection::Text,
                global: false,
            },
        ];
        let elf = write_image(&[0x0000_0013, 0x0000_0013], &[], &syms);

        let sh = section(&elf, ".symtab").expect(".symtab exists");
        let sh_info = u32::from_le_bytes(elf[sh + 44..sh + 48].try_into().expect("4"));
        assert_eq!(sh_info, 2, "null symbol and one local come first");

        let off = u64::from_le_bytes(elf[sh + 24..sh + 32].try_into().expect("8")) as usize;
        // Symbol 1 is the local; its binding nibble must be zero.
        let info = elf[off + SYM_SIZE as usize + 4];
        assert_eq!(info >> 4, 0, "the first real symbol is local");
        assert_eq!(info & 0xf, 2, "and it is a function");
    }

    #[test]
    fn a_data_symbol_is_an_object_and_not_a_function() {
        use alloc::string::ToString;
        let syms = [Symbol {
            name: "सन्देशः".to_string(),
            value: 0x8000_0008,
            section: SymSection::Data,
            global: false,
        }];
        let elf = write_image(&[0x0000_0013], &[1, 2, 3, 4], &syms);
        let sh = section(&elf, ".symtab").expect(".symtab exists");
        let off = u64::from_le_bytes(elf[sh + 24..sh + 32].try_into().expect("8")) as usize;
        let sym = off + SYM_SIZE as usize;
        assert_eq!(elf[sym + 4] & 0xf, 1, "STT_OBJECT");
        assert_eq!(
            u16::from_le_bytes(elf[sym + 6..sym + 8].try_into().expect("2")),
            2,
            "it lives in section 2, which is .data"
        );
    }

    #[test]
    fn an_empty_program_still_produces_a_valid_header() {
        let elf = write(&[]);
        assert_eq!(&elf[..4], b"\x7fELF");
        assert!(elf.len() > (EHDR_SIZE + PHDR_SIZE) as usize);
    }
}
