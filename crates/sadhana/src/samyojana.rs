//! **संयोजनम्** — linking objects, task `B-069d2a`, doc 03 §3.3.
//!
//! `बन्धकः` links by re-encoding every unit from its parse tree, which needs
//! the source. This links from **objects**: bytes, a symbol table, and a list
//! of holes. That is what separate compilation means and it is the last of
//! `B-014`'s promise.
//!
//! # Patching uses the encoding table, not a rule per relocation
//!
//! A relocation says *put this value in the field of the instruction at this
//! offset*. Which bits that field occupies is already derived —
//! `spec/encodings-riscv64.tsv` has a bit map per slot, probed from the
//! assembler (`B-037`, `B-055`).
//!
//! So applying a relocation is: decode the instruction, find its displacement
//! or immediate slot, and `place` the value with the same code the encoder
//! uses. Nothing here knows that a `jal` scatters its displacement across four
//! discontiguous runs — the map knows, and the map was measured.
//!
//! Writing a patcher per relocation type would be transcribing the J-type
//! layout a second time, and the first copy took four cycles to get right.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::encode::encodings;
use crate::kosha::{LOAD_ADDRESS, SymSection, Symbol};
use crate::vastu::{Object, Placement};

/// A linked image: text, data, and where every name ended up.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Linked {
    /// Instruction bytes, all objects end to end.
    pub text: Vec<u8>,
    /// Data bytes, likewise.
    pub data: Vec<u8>,
    /// Bytes reserved after the data: in memory, not in the file (`B-099`).
    pub bss: u64,
    /// Every defined name and its absolute address, for lookups.
    ///
    /// A convenience, and lossy by construction: two objects may each define a
    /// local `चक्रः` and a map holds one. [`table`](Self::table) is the truth.
    pub symbols: BTreeMap<String, u64>,
    /// The debug sections, carried through with their addresses fixed.
    ///
    /// `-g` reached the object in `B-100b` and stopped there: `--संयोजय` wrote
    /// text and data and dropped `.debug_line` with it, so a program linked
    /// from objects had no line table at all (`B-104`).
    pub debug: Vec<(String, Vec<u8>)>,
    /// Every symbol, in object order, for the image's `.symtab` (`B-103`).
    ///
    /// `--संयोजय` wrote text and data and dropped every name, so `nm` on an
    /// image linked from objects reported nothing while a single-file build of
    /// the same program showed `मुख्यम्`. An object format nobody else can read
    /// cannot be checked against anyone else's tools, and that argument does
    /// not stop at the object.
    pub table: Vec<Symbol>,
}

/// The relocation types this can apply, by number, from the derived table.
fn kind_named(name: &str) -> Option<u32> {
    const TABLE: &str = include_str!("../../../spec/relocations-riscv64.tsv");
    TABLE
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("name\t"))
        .filter_map(|l| l.split_once('\t'))
        .find(|(n, _)| *n == name)
        .and_then(|(_, v)| v.trim().parse().ok())
}

/// Link objects into one image.
///
/// # Errors
/// Every unresolved name and every relocation this cannot apply, rather than
/// the first — one run should report one link's problems.
pub fn link(objects: &[Object]) -> Result<Linked, Vec<String>> {
    link_at(objects, LOAD_ADDRESS)
}

/// As [`link`], placing the image at a chosen address — task `C-001a1`.
///
/// A symbol's value is absolute, so the address the image will be written at
/// has to be known here and not only in [`crate::kosha::write_debuggable_at`].
/// The relocations do not care: every one this applies is pc-relative and the
/// base cancels — which is exactly why an image can be moved at all.
///
/// # Errors
/// As [`link`].
pub fn link_at(objects: &[Object], load: u64) -> Result<Linked, Vec<String>> {
    let mut errors = Vec::new();

    // Where each object's text begins. Data follows all of it, at the next page,
    // exactly as `kosha` lays an image out (`kosha::data_base`, `W-363`).
    let mut text_at = Vec::with_capacity(objects.len());
    let mut data_at = Vec::with_capacity(objects.len());
    let mut bss_at = Vec::with_capacity(objects.len());
    let (mut text_len, mut data_len, mut bss_len) = (0u64, 0u64, 0u64);
    for o in objects {
        text_at.push(text_len);
        data_at.push(data_len);
        bss_at.push(bss_len);
        text_len += o.text.len() as u64;
        data_len += o.data.len() as u64;
        bss_len += o.bss;
    }
    let data_base = crate::kosha::data_base(text_len);
    // `.bss` follows all of the data, exactly as `kosha` lays an image out: a
    // reservation that overlapped another object's data would be a buffer
    // writing over initialised bytes, which runs.
    //
    // ON SIXTEEN, NOT EIGHT (`V-009` part (i-b2)): the startup's stack is the
    // first thing in `.bss`, and `sp` is its top, which the RISC-V ABI wants
    // 16-aligned. In `.data` it was — the data starts on a page — but the data
    // ends wherever its objects end. `kosha` places `.bss` at the data rounded
    // to EIGHT, so the up-to-eight octets between that and this start are
    // reported as `.bss` too ([`Linked::bss`] below): `p_memsz` still ends
    // exactly at the last reservation. Twin: `samyojana.t1` `बीजाधारः`.
    let bss_kosha = (data_base + data_len).next_multiple_of(8);
    let bss_base = (data_base + data_len).next_multiple_of(16);

    // Every definition, with its final address.
    //
    // A GLOBAL defined twice is a collision the linker must refuse: silently
    // taking one makes which file was listed first into program behaviour. A
    // LOCAL defined twice is ordinary — `B-014` decided that a label is local
    // to its file unless exported, precisely so two files may each have a
    // `चक्रः` — and this refused it, while `बन्धकः` linked the same two
    // programs without complaint (`B-103`).
    let mut symbols: BTreeMap<String, u64> = BTreeMap::new();
    let mut exported: BTreeMap<String, u64> = BTreeMap::new();
    let mut scopes: Vec<BTreeMap<String, u64>> = alloc::vec![BTreeMap::new(); objects.len()];
    let mut table: Vec<Symbol> = Vec::new();
    for (n, o) in objects.iter().enumerate() {
        for s in &o.symbols {
            if s.name.is_empty() || s.is_undefined() {
                continue;
            }
            // What the reader resolved, not a section number. This matched
            // `s.section == 1` against `.text` on the reasoning that ours and
            // GNU's both put it there — true while `.bss` and `.rela.data`
            // were absent, and false the moment either is written, because the
            // indices after `.data` all shift (`B-099`).
            let base = match s.placement {
                Placement::Text => load + text_at[n],
                Placement::Data => load + data_base + data_at[n],
                Placement::Bss => load + bss_base + bss_at[n],
                // A section this reader does not model is not placed. Guessing
                // `.data` would give the name a real address in the wrong
                // place, which is worse than refusing to define it.
                // A section symbol names a section, not a place a program can
                // jump to, so it is not put in the image's table.
                Placement::DebugLine | Placement::Undefined | Placement::Other => continue,
            };
            let address = base + s.value;
            if s.global && exported.insert(s.name.clone(), address).is_some() {
                // `V-009` (ii): a name a module's matrix kernel also defines is
                // RESERVED, and the refusal says so — but ONLY when the kernel is
                // one of the definitions. The kernel is keyed on ITS OWN entry,
                // not on the pair of names (a user object can export both): an
                // object defining `<module><member>` in `.text` for BOTH kernel
                // members WITHOUT their epilogue labels. Every compiled routine
                // places `<label>निर्गम` (`riscv64::exit_label`, §2.2); the
                // kernel is fixed text with no epilogue and places none.
                let kernel = |module: &str| {
                    objects.iter().any(|o| {
                        // ANY definition in `.text`: a global of the same name
                        // in the same object is a second, `.data`, definition.
                        let in_text = |name: &str| {
                            o.symbols.iter().any(|d| {
                                !d.is_undefined()
                                    && d.placement == Placement::Text
                                    && d.name == name
                            })
                        };
                        let placed = |name: &str| o.symbols.iter().any(|d| d.name == name);
                        crate::t1::nirvahana::MATRIX_KERNEL_MEMBERS
                            .iter()
                            .all(|member| {
                                let entry = alloc::format!("{module}{member}");
                                in_text(&entry) && !placed(&crate::t1::riscv64::exit_label(&entry))
                            })
                    })
                };
                match crate::t1::nirvahana::kernel_name_duplicate(&s.name, kernel) {
                    Some(why) => errors.push(alloc::format!(
                        "`{}` is defined by more than one object: {why}",
                        s.name
                    )),
                    None => errors.push(alloc::format!(
                        "`{}` is defined by more than one object",
                        s.name
                    )),
                }
            }
            scopes[n].insert(s.name.clone(), address);
            symbols.insert(s.name.clone(), address);
            table.push(Symbol {
                name: s.name.clone(),
                value: address,
                section: match s.placement {
                    Placement::Text => SymSection::Text,
                    Placement::Bss => SymSection::Bss,
                    _ => SymSection::Data,
                },
                global: s.global,
            });
        }
    }

    let mut text: Vec<u8> = Vec::with_capacity(text_len as usize);
    for o in objects {
        text.extend_from_slice(&o.text);
    }
    let mut data: Vec<u8> = Vec::with_capacity(data_len as usize);
    for o in objects {
        data.extend_from_slice(&o.data);
    }

    // `.rela.data` — an absolute 64-bit pointer, not a displacement (`B-098`).
    // It is applied before the text records because the data bytes are already
    // laid out and nothing about it depends on relaxation.
    let abs64 = kind_named("R_RISCV_64");
    for (n, o) in objects.iter().enumerate() {
        for r in &o.data_relocations {
            let Some(sym) = o.symbols.get(r.symbol as usize) else {
                errors.push(alloc::format!(
                    "a relocation names symbol {}, which does not exist",
                    r.symbol
                ));
                continue;
            };
            // The object's own names first, then what any object exported.
            let Some(target) = scopes[n].get(&sym.name).or_else(|| exported.get(&sym.name)) else {
                errors.push(alloc::format!(
                    "`{}` is not defined by any object",
                    sym.name
                ));
                continue;
            };
            if Some(r.kind) != abs64 {
                // Refused, not approximated. `.rela.data` carries one kind
                // today and a second arriving unhandled would be written by
                // whichever rule happened to be first.
                errors.push(alloc::format!(
                    "relocation type {} in .data is not applied yet",
                    r.kind
                ));
                continue;
            }
            let at = (data_at[n] + r.offset) as usize;
            let Some(slot) = data.get_mut(at..at + 8) else {
                errors.push(alloc::format!(
                    "a data relocation points past .data, at {at:#x}"
                ));
                continue;
            };
            // Absolute: the symbol's own address, not a distance from here.
            // A pointer in a table is dereferenced, never added to a pc.
            slot.copy_from_slice(&((*target as i64 + r.addend) as u64).to_le_bytes());
        }
    }

    let jal = kind_named("R_RISCV_JAL");
    let hi20 = kind_named("R_RISCV_PCREL_HI20");
    let lo12 = kind_named("R_RISCV_PCREL_LO12_I");
    for (n, o) in objects.iter().enumerate() {
        for r in &o.relocations {
            let Some(sym) = o.symbols.get(r.symbol as usize) else {
                errors.push(alloc::format!(
                    "a relocation names symbol {}, which does not exist",
                    r.symbol
                ));
                continue;
            };
            // A `%pcrel_lo` names WHERE ITS `%pcrel_hi` IS, not what the pair
            // reaches (`B-105`). So the target is the hi20's, found by the
            // address this record gives — which is the ABI's rule and replaces
            // reading `pc - 4` on the assembler's promise that the two are
            // adjacent. Following the record makes the pair's own claim the
            // authority, and it is what lets GNU `ld` read these objects.
            let resolved = if Some(r.kind) == lo12 {
                let hi_at = sym.value as i64 + r.addend;
                match o
                    .relocations
                    .iter()
                    .find(|h| Some(h.kind) == hi20 && h.offset as i64 == hi_at)
                    .and_then(|h| o.symbols.get(h.symbol as usize))
                {
                    Some(hi_sym) => scopes[n]
                        .get(&hi_sym.name)
                        .or_else(|| exported.get(&hi_sym.name))
                        .copied()
                        .ok_or_else(|| hi_sym.name.clone()),
                    None => Err(alloc::format!(
                        "a %pcrel_lo at {:#x} names {hi_at:#x}, where no %pcrel_hi is",
                        r.offset
                    )),
                }
            } else {
                // The object's own names first, then what any object exported.
                // A file's own `चक्रः` is the one it means, and shadowing is
                // what makes local names local — `B-014`'s rule.
                scopes[n]
                    .get(&sym.name)
                    .or_else(|| exported.get(&sym.name))
                    .copied()
                    .ok_or_else(|| sym.name.clone())
            };
            let target = match resolved {
                Ok(v) => v,
                Err(name) => {
                    errors.push(alloc::format!("`{name}` is not defined by any object"));
                    continue;
                }
            };
            let at = (text_at[n] + r.offset) as usize;
            let pc = load + at as u64;
            // A `%pcrel_lo`'s addend belongs to the address it names, not to
            // the value it writes, so it is not added again here.
            let addend = if Some(r.kind) == lo12 { 0 } else { r.addend };
            let value = target as i64 + addend - pc as i64;

            let outcome = if Some(r.kind) == jal {
                patch(&mut text, at, "disp", value)
            } else if Some(r.kind) == hi20 {
                // The upper twenty bits, rounded so that adding a SIGNED lower
                // twelve reaches the target: `addi` sign-extends, so a low half
                // above 0x7ff must be borrowed against.
                patch(&mut text, at, "imm", (value + 0x800) >> 12)
            } else if Some(r.kind) == lo12 {
                // Measured from the `auipc`, not from here. `ॱअधः` completes
                // the register the preceding instruction set, so the pair is
                // positional and the low half is computed against that pc —
                // computing it against its own would be wrong by four.
                let hi_pc = load as i64 + text_at[n] as i64 + sym.value as i64 + r.addend;
                let whole = target as i64 - hi_pc;
                patch(&mut text, at, "imm", whole - ((whole + 0x800) >> 12 << 12))
            } else {
                // Refused, not approximated. A relocation applied by the wrong
                // rule produces a program that runs and goes somewhere else.
                Err(alloc::format!(
                    "relocation type {} at {at:#x} is not applied yet",
                    r.kind
                ))
            };
            if let Err(e) = outcome {
                errors.push(e);
            }
        }
    }

    // The debug sections, with their addresses fixed. One object only: two
    // units concatenate cleanly in `.debug_info` and `.debug_line`, but each
    // CU's `DW_AT_stmt_list` would have to be moved to its own line program,
    // and nothing records where that field sits. `B-104b` emits a record for
    // it; refusing is what keeps a two-object build from quietly reporting
    // every line against the first file's table.
    // Concatenated, unit by unit. DWARF puts one complete unit after another
    // in both sections and a reader walks them, so merging is appending — with
    // one field moved per unit: `DW_AT_stmt_list` is the offset of that unit's
    // line program, and the second unit's program is no longer at zero.
    //
    // `.debug_abbrev` is taken once. Every unit this toolchain writes uses the
    // same table and points at offset 0, so appending a second copy would be
    // bytes nobody reads.
    let mut debug: Vec<(String, Vec<u8>)> = Vec::new();
    let mut info: Vec<u8> = Vec::new();
    let mut lines: Vec<u8> = Vec::new();
    let mut abbrev: Vec<u8> = Vec::new();
    for (n, o) in objects.iter().enumerate() {
        if o.debug.is_empty() {
            continue;
        }
        let base = load + text_at[n];
        let line_base = lines.len() as u64;
        let section = |name: &str| {
            o.debug
                .iter()
                .find(|(s, _)| s == name)
                .map(|(_, b)| b.clone())
                .unwrap_or_default()
        };
        let mut this_info = section(".debug_info");
        let this_line = section(".debug_line");
        let this_abbrev = section(".debug_abbrev");

        for (name, records) in &o.debug_relocations {
            let target: &mut Vec<u8> = match name.as_str() {
                ".debug_info" => &mut this_info,
                // Nothing patches `.debug_line` but its own set_address, which
                // is a text address and handled below by the same loop.
                _ => continue,
            };
            for r in records {
                let Some(sym) = o.symbols.get(r.symbol as usize) else {
                    errors.push(alloc::format!(
                        "a relocation names symbol {}, which does not exist",
                        r.symbol
                    ));
                    continue;
                };
                let at = r.offset as usize;
                match sym.placement {
                    // `DW_AT_stmt_list`: four bytes, an OFFSET into the merged
                    // `.debug_line`, not an address.
                    Placement::DebugLine => {
                        let Some(slot) = target.get_mut(at..at + 4) else {
                            errors.push(alloc::format!("a record points past {name}, at {at:#x}"));
                            continue;
                        };
                        let value = (line_base as i64 + r.addend) as u32;
                        slot.copy_from_slice(&value.to_le_bytes());
                    }
                    _ => {
                        let Some(slot) = target.get_mut(at..at + 8) else {
                            errors.push(alloc::format!("a record points past {name}, at {at:#x}"));
                            continue;
                        };
                        slot.copy_from_slice(&(base as i64 + r.addend).to_le_bytes());
                    }
                }
            }
        }

        // `.debug_line`'s own record is the `DW_LNE_set_address`, a text
        // address, and is applied against this object's text base.
        let mut this_line = this_line;
        for (name, records) in &o.debug_relocations {
            if name != ".debug_line" {
                continue;
            }
            for r in records {
                let at = r.offset as usize;
                let Some(slot) = this_line.get_mut(at..at + 8) else {
                    errors.push(alloc::format!("a record points past {name}, at {at:#x}"));
                    continue;
                };
                slot.copy_from_slice(&(base as i64 + r.addend).to_le_bytes());
            }
        }

        if abbrev.is_empty() {
            abbrev = this_abbrev;
        } else if abbrev != this_abbrev {
            // Every unit points at abbrev offset 0, so two different tables
            // cannot both be reachable. Refused rather than silently using the
            // first, which would decode the second unit's DIEs by the wrong
            // shape.
            errors.push(
                "objects disagree about .debug_abbrev; a merged unit would decode by the wrong table"
                    .to_string(),
            );
        }
        info.extend_from_slice(&this_info);
        lines.extend_from_slice(&this_line);
    }
    if !abbrev.is_empty() {
        debug.push((".debug_abbrev".into(), abbrev));
        debug.push((".debug_info".into(), info));
        debug.push((".debug_line".into(), lines));
    }

    if errors.is_empty() {
        Ok(Linked {
            text,
            data,
            // The pad to the 16-aligned start counts only when there IS a
            // `.bss`: an image with none keeps none, and its one header.
            bss: if bss_len > 0 {
                bss_len + (bss_base - bss_kosha)
            } else {
                0
            },
            symbols,
            table,
            debug,
        })
    } else {
        Err(errors)
    }
}

/// Write `value` into the named field of the instruction at `at`.
///
/// Decodes, replaces the `disp` slot and re-encodes. The bit map does the
/// scattering, so this knows nothing about J-type layout — which is the point,
/// because the encoder's copy of that knowledge was measured and this one would
/// have been typed.
///
/// # Why four octets may be read unconditionally
///
/// This reads `at..at + 4` and demands a 32-bit encoding, while every other
/// instruction walker in this crate asks [`crate::vishlesana::width_of`] first
/// (`B-058b1`). That is safe here, and by construction rather than by luck: a
/// relocation is only ever recorded for an instruction naming a symbol the file
/// does not define, and [`crate::encode::compressed_at`] begins by encoding the
/// wide form, which fails for exactly those. So an instruction carrying a
/// relocation cannot have been compressed, and the four octets a relocation
/// names are still four octets — the property `encode_object_for` states where
/// it emits them.
///
/// `B-058b2b7` tested this rather than assuming it: a `width_of` guard added
/// here fired on **zero** of 36 `spec/*.sas` assembled with `--संक्षिप्त`, and
/// was removed as armour that cannot be wrong. **If a future linker relaxes
/// across objects** — which `B-069e` names as the alternative to keeping
/// cross-file branches wide — that property dies, a relocation can land on a
/// compressed instruction, and this function will silently re-encode over two
/// instructions. Restore the guard with that change, not after it.
fn patch(text: &mut [u8], at: usize, kind: &str, value: i64) -> Result<(), String> {
    let bytes = text
        .get(at..at + 4)
        .ok_or_else(|| alloc::format!("a relocation points past the text, at {at:#x}"))?;
    let word = u32::from_le_bytes(bytes.try_into().map_err(|_| "short read".to_string())?);
    let decoded = crate::vishlesana::decode(word)
        .ok_or_else(|| alloc::format!("nothing decodes at {at:#x}, so nothing can be patched"))?;

    let all = encodings();
    let e = all
        .iter()
        .find(|e| e.insn == decoded.insn && e.bits == 32)
        .ok_or_else(|| alloc::format!("`{}` has no 32-bit encoding", decoded.insn))?;
    // `imm` and `simm` are both immediates; the derivation splits them by
    // whether the field accepts a negative, which is not a distinction a
    // relocation makes.
    let slot = e
        .slots
        .iter()
        .find(|s| s.kind == kind || (kind == "imm" && s.kind == "simm"))
        .ok_or_else(|| alloc::format!("`{}` has no {kind} to patch", decoded.insn))?;
    if !slot.fits(value as u64) {
        return Err(alloc::format!(
            "`{}` at {at:#x} cannot reach {value} bytes",
            decoded.insn
        ));
    }

    // Rebuild from the pattern rather than OR-ing into the existing word: the
    // field is zero in an object, but a linker that assumed so and met a
    // non-zero one would add to it silently.
    let mut out = e.pattern;
    for (s, (_, v)) in e.slots.iter().zip(&decoded.operands) {
        let v = if core::ptr::eq(s, slot) { value } else { *v };
        out |= s.place(v as u64);
    }
    text[at..at + 4].copy_from_slice(&out.to_le_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_relocation_table_is_the_authority_here_too() {
        // If the derived table ever loses `R_RISCV_JAL`, this linker silently
        // refuses every call rather than mispatching one — but it should be
        // loud about why.
        assert_eq!(kind_named("R_RISCV_JAL"), Some(17));
        assert_eq!(kind_named("R_RISCV_NOT_A_THING"), None);
    }

    #[test]
    fn a_name_defined_twice_is_refused() {
        use crate::vastu::{Object, ObjectSymbol};
        let one = || Object {
            text: Vec::new(),
            data: Vec::new(),
            bss: 0,
            debug: Vec::new(),
            debug_relocations: Vec::new(),
            symbols: alloc::vec![ObjectSymbol {
                name: "क".into(),
                value: 0,
                section: 1,
                placement: Placement::Text,
                global: true,
            }],
            relocations: Vec::new(),
            data_relocations: Vec::new(),
        };
        let e = link(&[one(), one()]).expect_err("must refuse");
        assert!(e[0].contains("more than one object"), "{}", e[0]);
    }
}
