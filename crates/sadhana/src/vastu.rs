//! **वस्तु** — reading a relocatable object, task `B-069d1`, doc 03 §3.3.1.
//!
//! `बन्धकः` linked by re-encoding every unit from its parse tree. That satisfied
//! gate B10 and is not separate compilation: a file cannot be assembled once
//! and linked many times if linking needs the source again. `B-069b` taught
//! `kosha` to *write* an object; this reads one back.
//!
//! # Why it must read GNU's objects too
//!
//! A reader checked only against our own writer proves the two agree, which
//! they would even if both were wrong — the failure `B-058a` found in the
//! encoding table, where a mask and a field map were wrong together and no test
//! comparing them could see it.
//!
//! So the test reads an object `riscv64-elf-as` produced and compares what it
//! finds against `readelf`. That is the same argument the conformance corpus
//! rests on, applied to a container.
//!
//! # What it does not do
//!
//! Merge symbols across files, or apply a relocation. Those are the rest of
//! `B-069d` and they need this first: nothing can be merged that cannot be
//! read.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Which of an object's sections a symbol names a place in.
///
/// Resolved here rather than by the linker. `samyojana` matched `section == 1`
/// against `.text` on the reasoning that ours and GNU's both put it there,
/// which was true while `.bss` and `.rela.data` were absent and stops being
/// true the moment either is written (`B-099`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Placement {
    /// `.text`.
    Text,
    /// `.data`.
    Data,
    /// `.bss` — space that exists in memory and not in the file.
    Bss,
    /// `.debug_line` — a section symbol a `DW_AT_stmt_list` points through.
    DebugLine,
    /// `SHN_UNDEF`: the file asks for this name rather than defining it.
    #[default]
    Undefined,
    /// A section this reader does not model. Never placed, never guessed at.
    Other,
}

/// A symbol as the file records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectSymbol {
    /// The name, from `.strtab`.
    pub name: String,
    /// Its value — an offset within its section, for an object.
    pub value: u64,
    /// The section header index it belongs to; 0 is `SHN_UNDEF`.
    pub section: u16,
    /// Which section that index turned out to be.
    pub placement: Placement,
    /// True for `STB_GLOBAL`.
    pub global: bool,
}

impl ObjectSymbol {
    /// Whether this file asks for the name rather than defining it.
    #[must_use]
    pub fn is_undefined(&self) -> bool {
        self.section == 0
    }
}

/// One `.rela.text` entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectRelocation {
    /// Byte offset into `.text`.
    pub offset: u64,
    /// Index into [`Object::symbols`], as ELF counts it — 0 is the null symbol.
    pub symbol: u32,
    /// A type from `spec/relocations-riscv64.tsv`.
    pub kind: u32,
    /// Added to the symbol's value.
    pub addend: i64,
}

/// A relocatable object, as far as linking needs it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Object {
    /// `.text` contents.
    pub text: Vec<u8>,
    /// `.data` contents, empty when the section is absent.
    pub data: Vec<u8>,
    /// Every symbol, in file order, including the null one at index 0.
    pub symbols: Vec<ObjectSymbol>,
    /// Every `.rela.text` record.
    pub relocations: Vec<ObjectRelocation>,
    /// Bytes reserved in `.bss`: present in memory, absent from the file.
    pub bss: u64,
    /// The debug sections this object carries, by name, in file order.
    ///
    /// Read back so a linker can carry them into the image it writes: `-g`
    /// reached the object in `B-100b` and `--संयोजय` dropped it (`B-104`).
    pub debug: Vec<(String, Vec<u8>)>,
    /// Relocations against a debug section, by that section's name.
    ///
    /// Kept per section for the reason `.rela.data` is kept apart from
    /// `.rela.text`: an offset means nothing without the section it counts
    /// from (`B-099`).
    pub debug_relocations: Vec<(String, Vec<ObjectRelocation>)>,
    /// Every `.rela.data` record — a pointer written into `ॱदत्त` (`B-098`).
    ///
    /// Kept apart from [`relocations`](Self::relocations) because an offset
    /// means nothing without the section it counts from, and applying a data
    /// offset to `.text` patches a real instruction with a real address.
    pub data_relocations: Vec<ObjectRelocation>,
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}
fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}
fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// A nul-terminated name at `at` in a string table.
fn name_at(strtab: &[u8], at: usize) -> String {
    let end = strtab[at.min(strtab.len())..]
        .iter()
        .position(|c| *c == 0)
        .map_or(strtab.len(), |n| at + n);
    String::from_utf8_lossy(&strtab[at.min(strtab.len())..end]).into_owned()
}

/// One section header, as read.
struct Section {
    name: u32,
    kind: u32,
    offset: u64,
    size: u64,
    link: u32,
    /// For a `SHT_RELA`, the index of the section it patches.
    info: u32,
    entsize: u64,
}

/// Read a relocatable object.
///
/// `None` when the bytes are not an ELF64 little-endian RISC-V `ET_REL` — a
/// refusal rather than a guess, because a linker that half-reads a file
/// produces a program rather than an error.
#[must_use]
pub fn read(bytes: &[u8]) -> Option<Object> {
    if bytes.get(..4)? != b"\x7fELF" {
        return None;
    }
    // ELFCLASS64, ELFDATA2LSB. A 32-bit or big-endian file has a different
    // header layout entirely, so every offset below would be wrong.
    if *bytes.get(4)? != 2 || *bytes.get(5)? != 1 {
        return None;
    }
    if u16_at(bytes, 16)? != 1 {
        return None; // not ET_REL
    }

    let shoff = u64_at(bytes, 40)? as usize;
    let shentsize = u16_at(bytes, 58)? as usize;
    let shnum = u16_at(bytes, 60)? as usize;
    let shstrndx = u16_at(bytes, 62)? as usize;

    let mut sections = Vec::with_capacity(shnum);
    for i in 0..shnum {
        let at = shoff + i * shentsize;
        sections.push(Section {
            name: u32_at(bytes, at)?,
            kind: u32_at(bytes, at + 4)?,
            offset: u64_at(bytes, at + 24)?,
            size: u64_at(bytes, at + 32)?,
            link: u32_at(bytes, at + 40)?,
            info: u32_at(bytes, at + 44)?,
            entsize: u64_at(bytes, at + 56)?,
        });
    }

    let shstr = sections.get(shstrndx)?;
    let shstrtab = bytes.get(shstr.offset as usize..(shstr.offset + shstr.size) as usize)?;
    let named = |s: &Section| name_at(shstrtab, s.name as usize);
    let contents = |s: &Section| {
        bytes
            .get(s.offset as usize..(s.offset + s.size) as usize)
            .map(<[u8]>::to_vec)
            .unwrap_or_default()
    };

    let mut out = Object::default();
    // Their INDICES too, because `sh_info` on a relocation section names the
    // section it patches by index, and that is the only thing that says whether
    // a record's offset counts from `.text` or from `.data`.
    let (mut i_text, mut i_data, mut i_bss) = (None, None, None);
    let mut debug_index: Vec<(u32, String)> = Vec::new();
    for (i, s) in sections.iter().enumerate() {
        match named(s).as_str() {
            ".text" => {
                out.text = contents(s);
                i_text = Some(i as u32);
            }
            ".data" => {
                out.data = contents(s);
                i_data = Some(i as u32);
            }
            // SHT_NOBITS is 8, and its size is the whole of it: there are no
            // bytes to read, which is what makes reserved space free on disk.
            ".bss" if s.kind == 8 => {
                out.bss = s.size;
                i_bss = Some(i as u32);
            }
            // `SHT_PROGBITS` and a `.debug_` name. Matched on both, because a
            // `.rela.debug_line` is also named `.debug`-something and is not a
            // debug section — it is the records that patch one.
            name if s.kind == 1 && name.starts_with(".debug_") => {
                debug_index.push((i as u32, name.to_string()));
                out.debug.push((name.to_string(), contents(s)));
            }
            _ => {}
        }
    }

    // SHT_SYMTAB is 2. Found by TYPE rather than by name: the name is
    // conventional and the type is what makes it a symbol table.
    if let Some(symtab) = sections.iter().find(|s| s.kind == 2) {
        let strtab_section = sections.get(symtab.link as usize)?;
        let strtab = contents(strtab_section);
        let entsize = if symtab.entsize == 0 {
            24
        } else {
            symtab.entsize as usize
        };
        let count = (symtab.size as usize) / entsize;
        for i in 0..count {
            let at = symtab.offset as usize + i * entsize;
            let info = *bytes.get(at + 4)?;
            let section = u16_at(bytes, at + 6)?;
            out.symbols.push(ObjectSymbol {
                name: name_at(&strtab, u32_at(bytes, at)? as usize),
                value: u64_at(bytes, at + 8)?,
                section,
                placement: match u32::from(section) {
                    0 => Placement::Undefined,
                    n if Some(n) == i_text => Placement::Text,
                    n if Some(n) == i_data => Placement::Data,
                    n if Some(n) == i_bss => Placement::Bss,
                    n if debug_index
                        .iter()
                        .any(|(i, name)| *i == n && name == ".debug_line") =>
                    {
                        Placement::DebugLine
                    }
                    _ => Placement::Other,
                },
                global: info >> 4 == 1,
            });
        }
    }

    // SHT_RELA is 4, and `sh_info` names the section it patches. That used to
    // be read past on the grounds that there was only one relocatable section;
    // `B-098` added `.rela.data`, so taking every record into one list would
    // apply a data offset to `.text` — patching a real instruction with a real
    // address, which assembles, links and runs.
    for rela in sections.iter().filter(|s| s.kind == 4) {
        let entsize = if rela.entsize == 0 {
            24
        } else {
            rela.entsize as usize
        };
        let count = (rela.size as usize) / entsize;
        // `sh_info` is the section index this one patches. A record whose
        // target is neither `.text` nor `.data` is skipped rather than guessed
        // at: applying it to whichever list came first is how a plausible wrong
        // patch gets written.
        let target = if Some(rela.info) == i_data {
            &mut out.data_relocations
        } else if Some(rela.info) == i_text {
            &mut out.relocations
        } else if let Some((_, name)) = debug_index.iter().find(|(i, _)| *i == rela.info) {
            let name = name.clone();
            if !out.debug_relocations.iter().any(|(n, _)| *n == name) {
                out.debug_relocations.push((name.clone(), Vec::new()));
            }
            &mut out
                .debug_relocations
                .iter_mut()
                .find(|(n, _)| *n == name)
                .expect("just inserted")
                .1
        } else {
            continue;
        };
        for i in 0..count {
            let at = rela.offset as usize + i * entsize;
            let info = u64_at(bytes, at + 8)?;
            target.push(ObjectRelocation {
                offset: u64_at(bytes, at)?,
                // The symbol index is the HIGH half and the type the low. A
                // reader that swaps them gets a plausible pair of small
                // numbers and patches the wrong field with the wrong rule.
                symbol: (info >> 32) as u32,
                kind: (info & 0xffff_ffff) as u32,
                addend: u64_at(bytes, at + 16)? as i64,
            });
        }
    }

    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_that_is_not_an_object_is_refused() {
        // A linker that half-reads a file produces a program rather than an
        // error, so every one of these is a refusal.
        assert!(read(b"").is_none(), "empty");
        assert!(read(b"not an elf at all").is_none(), "no magic");
        // Our own executable is ELF and is not ET_REL.
        let exe = crate::kosha::write(&0x0000_0013u32.to_le_bytes());
        assert!(read(&exe).is_none(), "an executable is not an object");
    }

    #[test]
    fn our_own_object_reads_back() {
        use crate::kosha::{Relocation, SymSection, Symbol, write_relocatable};
        let text = 0x0000_00efu32.to_le_bytes().to_vec();
        let symbols = vec![
            Symbol {
                name: "मुख्यम्".into(),
                value: 0,
                section: SymSection::Text,
                global: true,
            },
            Symbol {
                name: "बाह्यम्".into(),
                value: 0,
                section: SymSection::Undefined,
                global: true,
            },
        ];
        let relocations = vec![Relocation {
            section: crate::kosha::RelSection::Text,
            offset: 0,
            symbol: 2,
            kind: 17,
            addend: 0,
        }];
        let bytes = write_relocatable(&text, &[], 0, &[], &symbols, &relocations, 4);

        let o = read(&bytes).expect("reads");
        assert_eq!(o.text, text);
        assert_eq!(o.relocations.len(), 1);
        assert_eq!(o.relocations[0].kind, 17, "R_RISCV_JAL");
        assert_eq!(o.relocations[0].offset, 0);

        let target = &o.symbols[o.relocations[0].symbol as usize];
        assert_eq!(target.name, "बाह्यम्");
        assert!(
            target.is_undefined(),
            "the record must point at the name the file does not define"
        );
        // And the name it DOES define is not undefined, or the two are swapped.
        assert!(
            o.symbols
                .iter()
                .any(|s| s.name == "मुख्यम्" && !s.is_undefined()),
            "our own label came back as undefined"
        );
    }
}
