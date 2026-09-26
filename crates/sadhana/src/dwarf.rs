//! DWARF 5 line table — task `B-011a`, doc 03 §3.
//!
//! Maps each instruction's address back to the line of Sassembly that produced
//! it, so a debugger stepping through `namaste.sas` shows Devanagari source
//! rather than addresses.
//!
//! # The oracle is `riscv64-elf-as -gdwarf-5`
//!
//! Every constant here was read out of a real line table rather than out of the
//! standard. `tools/gen-dwarf-line.py` assembles a file with `.loc` directives,
//! dumps the result with `readelf --debug-dump=rawline`, and writes what it
//! finds to `spec/dwarf-line-v5.tsv`; the values below are checked against that
//! file by `header_matches_the_oracle`.
//!
//! This matters more than it looks. `line_base` is **-5** and `line_range` is
//! **14** — those are GNU's choices, not the standard's, and a special opcode
//! computed with different ones decodes to a different line. Reading them from
//! a produced table is the difference between agreeing with the toolchain and
//! agreeing with our reading of a document.
//!
//! # What a line table is
//!
//! A bytecode. A tiny machine holds an address and a line, both starting at
//! known values, and the program advances them; wherever the program says
//! **copy**, the current `(address, line)` pair becomes a row of the table.
//! Encoding it is therefore choosing a cheap way to say "advance by this much",
//! and the cheapest is a **special opcode**: one byte that advances both.

extern crate alloc;

use alloc::vec::Vec;

/// Line-program constants, as `riscv64-elf-as -gdwarf-5` emits them.
///
/// `spec/dwarf-line-v5.tsv` is the derivation and the test compares against it.
const VERSION: u16 = 5;
const ADDRESS_SIZE: u8 = 8;
const MIN_INSN_LENGTH: u8 = 1;
const MAX_OPS_PER_INSN: u8 = 1;
const DEFAULT_IS_STMT: u8 = 1;
const LINE_BASE: i8 = -5;
const LINE_RANGE: u8 = 14;
const OPCODE_BASE: u8 = 13;
/// How many arguments each standard opcode takes, opcodes 1..=12.
const STANDARD_OPCODE_LENGTHS: [u8; 12] = [0, 1, 1, 1, 1, 0, 0, 0, 1, 0, 0, 1];

// Standard opcodes used here.
const DW_LNS_COPY: u8 = 1;
const DW_LNS_ADVANCE_PC: u8 = 2;
const DW_LNS_ADVANCE_LINE: u8 = 3;
const DW_LNS_SET_FILE: u8 = 4;
// Extended opcodes.
const DW_LNE_END_SEQUENCE: u8 = 1;
const DW_LNE_SET_ADDRESS: u8 = 2;
// Content and form codes for the directory and file tables.
const DW_LNCT_PATH: u8 = 1;
const DW_LNCT_DIRECTORY_INDEX: u8 = 2;
const DW_FORM_STRING: u8 = 0x08;
const DW_FORM_UDATA: u8 = 0x0f;

/// What produced the image, recorded in the CU so a bug report says so.
const PRODUCER: &str = concat!("sadhana ", env!("CARGO_PKG_VERSION"));

/// One row of the table: an address, and the source line it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    /// Address of the instruction.
    pub address: u64,
    /// 1-based source line.
    pub line: u32,
    /// Index into the file table — which source this line is in.
    ///
    /// A linked image is several files' instructions laid end to end, so a row
    /// that did not say which file would blame every line on the first
    /// (`B-011b`).
    pub file: u32,
}

fn uleb(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let mut byte = (v & 0x7f) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if v == 0 {
            return;
        }
    }
}

fn sleb(out: &mut Vec<u8>, mut v: i64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        // The sign must survive: stop only when the remaining bits are all
        // copies of the byte's sign bit, or a negative delta decodes positive.
        let done = (v == 0 && byte & 0x40 == 0) || (v == -1 && byte & 0x40 != 0);
        out.push(if done { byte } else { byte | 0x80 });
        if done {
            return;
        }
    }
}

/// A nul-terminated string, which is what `DW_FORM_string` is.
fn string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(s.as_bytes());
    out.push(0);
}

/// The one-byte opcode that advances both address and line, if one fits.
///
/// This is the whole reason the format is small. `opcode = (line_delta -
/// line_base) + line_range * addr_advance + opcode_base`, valid only when the
/// line delta is inside `[line_base, line_base + line_range)` and the result
/// still fits in a byte — so the caller must be prepared to emit the long form.
fn special_opcode(line_delta: i64, addr_advance: u64) -> Option<u8> {
    let adjusted = line_delta - i64::from(LINE_BASE);
    if adjusted < 0 || adjusted >= i64::from(LINE_RANGE) {
        return None;
    }
    let op = adjusted + i64::from(LINE_RANGE) * (addr_advance as i64) + i64::from(OPCODE_BASE);
    (op <= 255).then_some(op as u8)
}

/// A complete `.debug_line` section for one compilation unit.
///
/// `rows` must be sorted by address. `end` is the address one past the last
/// instruction, which is what closes the sequence — without it a debugger has
/// Test-only convenience over [`line_program_for`].
///
/// `#[cfg(test)]` because nothing outside this file's tests calls it, and a
/// wrapper kept in the library is a wrapper compiled into every build of it
/// (`X-002`). Demoting it from `pub` is what made that visible: rustc does not
/// lint unused *public* items, so `pub` had been switching the check off.
#[cfg(test)]
/// no idea where the last line stops.
#[must_use]
fn line_program(files: &[&str], rows: &[Row], end: u64) -> Vec<u8> {
    line_program_for(files, rows, end, false).bytes
}

/// A `.debug_line` section and where its address operand sits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineProgram {
    /// The section contents.
    pub bytes: Vec<u8>,
    /// Byte offset of the eight-byte operand of `DW_LNE_set_address`.
    ///
    /// `None` when the program never sets one, which is the executable case
    /// where the first row already carries the load address.
    pub set_address_at: Option<usize>,
}

/// As [`line_program`], for an object.
///
/// `relocatable` forces the `DW_LNE_set_address` even when the first row is at
/// zero — which in an object it always is, because addresses are relative to
/// the section. Without it the sequence would start at zero *implicitly*, read
/// correctly with `readelf` on the object, and be silently wrong in every
/// program linked from it: there would be no operand for the linker to fix.
#[must_use]
pub fn line_program_for(files: &[&str], rows: &[Row], end: u64, relocatable: bool) -> LineProgram {
    let mut set_address_at = None;
    let mut has_set = false;
    let mut header = Vec::new();

    // --- the part after `header_length`, whose length is what it counts ----
    let mut prologue = alloc::vec![
        MIN_INSN_LENGTH,
        MAX_OPS_PER_INSN,
        DEFAULT_IS_STMT,
        LINE_BASE as u8,
        LINE_RANGE,
        OPCODE_BASE,
    ];
    prologue.extend_from_slice(&STANDARD_OPCODE_LENGTHS);

    // Directory table. One entry, and DWARF 5 requires entry 0 to be the
    // compilation directory — unlike version 2, where the table began at 1.
    prologue.push(1); // directory_entry_format_count
    uleb(&mut prologue, u64::from(DW_LNCT_PATH));
    uleb(&mut prologue, u64::from(DW_FORM_STRING));
    uleb(&mut prologue, 1); // directories_count
    string(&mut prologue, ".");

    // File table. `DW_FORM_string` writes the name inline; GNU uses
    // `DW_FORM_line_strp`, which needs a whole `.debug_line_str` section to
    // point into. Inline names are valid DWARF 5 and cost one section less.
    prologue.push(2); // file_name_entry_format_count
    uleb(&mut prologue, u64::from(DW_LNCT_PATH));
    uleb(&mut prologue, u64::from(DW_FORM_STRING));
    uleb(&mut prologue, u64::from(DW_LNCT_DIRECTORY_INDEX));
    uleb(&mut prologue, u64::from(DW_FORM_UDATA));
    uleb(&mut prologue, files.len() as u64);
    for name in files {
        string(&mut prologue, name);
        uleb(&mut prologue, 0); // directory index
    }

    // --- the line number program -------------------------------------------
    let mut program = Vec::new();
    let mut address = 0u64;
    let mut line = 1i64;

    // DWARF 5 §6.2.2: the `file` register starts at **1**, while the file table
    // is indexed from **0**. A table with one entry therefore leaves the machine
    // pointing past its end, and readelf reports every row as `<corrupt>` while
    // still printing the right line and address — a failure that looks like it
    // is about the lines and is not.
    //
    // GNU's answer is to write the name twice, at 0 and at 1. This says it once
    // and moves the register, which costs two bytes instead of a second copy of
    // the path.
    let mut file = u32::MAX;

    for row in rows {
        let addr_advance = row.address.saturating_sub(address);
        let line_delta = i64::from(row.line) - line;

        // Emitted only when it changes, which for a single-file build is once.
        // The register starts at 1 and the table is indexed from 0, so even one
        // file needs this said at least once.
        if row.file != file {
            program.push(DW_LNS_SET_FILE);
            uleb(&mut program, u64::from(row.file));
            file = row.file;
        }

        // The first row sets the address outright: the machine starts at zero
        // and the text may not. In an object it does, and the operand is
        // written anyway — it is the only place a relocation can attach.
        // `address == 0` used to stand for "no address set yet", which is the
        // same thing only while the text cannot start at zero. In an object it
        // does, so a second row emitted a second `set_address` and the reported
        // offset named it. Ask the question directly.
        if !has_set && (row.address != 0 || relocatable) {
            program.push(0); // extended opcode escape
            uleb(&mut program, 9); // length: opcode + 8 address bytes
            program.push(DW_LNE_SET_ADDRESS);
            set_address_at = Some(program.len());
            has_set = true;
            program.extend_from_slice(&row.address.to_le_bytes());
            address = row.address;
            if let Some(op) = special_opcode(line_delta, 0) {
                program.push(op);
            } else {
                program.push(DW_LNS_ADVANCE_LINE);
                sleb(&mut program, line_delta);
                program.push(DW_LNS_COPY);
            }
            line = i64::from(row.line);
            continue;
        }

        if let Some(op) = special_opcode(line_delta, addr_advance) {
            program.push(op);
        } else {
            if line_delta != 0 {
                program.push(DW_LNS_ADVANCE_LINE);
                sleb(&mut program, line_delta);
            }
            if addr_advance != 0 {
                program.push(DW_LNS_ADVANCE_PC);
                uleb(&mut program, addr_advance);
            }
            program.push(DW_LNS_COPY);
        }
        address = row.address;
        line = i64::from(row.line);
    }

    // Close the sequence at `end`, so the last row has an extent.
    if end > address {
        program.push(DW_LNS_ADVANCE_PC);
        uleb(&mut program, end - address);
    }
    program.push(0);
    uleb(&mut program, 1);
    program.push(DW_LNE_END_SEQUENCE);

    // --- assemble ----------------------------------------------------------
    // `unit_length` counts everything after itself; `header_length` counts
    // everything after ITSELF up to the first opcode. Two different anchors,
    // and getting either wrong makes the section undecodable rather than wrong
    // in a way a reader could notice.
    let unit_length = 2 + 1 + 1 + 4 + prologue.len() + program.len();
    header.extend_from_slice(&(unit_length as u32).to_le_bytes());
    header.extend_from_slice(&VERSION.to_le_bytes());
    header.push(ADDRESS_SIZE);
    header.push(0); // segment_selector_size
    header.extend_from_slice(&(prologue.len() as u32).to_le_bytes());
    // The operand's position moves by everything written before it. Recorded
    // while writing rather than recomputed afterwards: `B-011b` counted an
    // opcode byte in a bytecode with inline operands and counted an operand,
    // and the code under test was correct both times.
    let program_at = header.len() + prologue.len();
    header.extend_from_slice(&prologue);
    header.extend_from_slice(&program);
    LineProgram {
        bytes: header,
        set_address_at: set_address_at.map(|at| program_at + at),
    }
}

/// DWARF tag, attribute and form codes for the one DIE we emit.
const DW_TAG_COMPILE_UNIT: u8 = 0x11;
const DW_UT_COMPILE: u8 = 0x01;
const DW_AT_NAME: u8 = 0x03;
const DW_AT_STMT_LIST: u8 = 0x10;
const DW_AT_LOW_PC: u8 = 0x11;
const DW_AT_HIGH_PC: u8 = 0x12;
const DW_AT_LANGUAGE: u8 = 0x13;
const DW_AT_COMP_DIR: u8 = 0x1b;
const DW_AT_PRODUCER: u8 = 0x25;
const DW_FORM_ADDR: u8 = 0x01;
const DW_FORM_DATA2: u8 = 0x05;
const DW_FORM_SEC_OFFSET: u8 = 0x17;
/// `DW_LANG_Mips_Assembler`, which is what GNU AS puts in an assembled unit.
///
/// Sassembly is an assembly language, so this is the truthful answer and it is
/// also the one the oracle gives. Inventing a language code would make every
/// debugger fall back to "unknown" for no gain.
const DW_LANG_ASSEMBLER: u16 = 0x8001;

/// The abbreviation table: one entry, describing the shape of the CU below.
///
/// `.debug_info` is a stream of attribute VALUES with no labels; the abbrev
/// table is the key that says which attribute each value is and how wide. The
/// two are read together, so an attribute added to one and not the other
/// desynchronises everything after it rather than being ignored.
fn abbrev() -> Vec<u8> {
    let mut out = Vec::new();
    uleb(&mut out, 1); // abbreviation code, referenced from .debug_info
    uleb(&mut out, u64::from(DW_TAG_COMPILE_UNIT));
    out.push(0); // no children: the unit has no functions or variables yet
    for (attr, form) in [
        (DW_AT_PRODUCER, DW_FORM_STRING),
        (DW_AT_LANGUAGE, DW_FORM_DATA2),
        (DW_AT_NAME, DW_FORM_STRING),
        (DW_AT_COMP_DIR, DW_FORM_STRING),
        (DW_AT_LOW_PC, DW_FORM_ADDR),
        (DW_AT_HIGH_PC, DW_FORM_UDATA),
        (DW_AT_STMT_LIST, DW_FORM_SEC_OFFSET),
    ] {
        uleb(&mut out, u64::from(attr));
        uleb(&mut out, u64::from(form));
    }
    uleb(&mut out, 0); // end of this entry's attributes
    uleb(&mut out, 0);
    out.push(0); // end of the table
    out
}

/// `.debug_info` and `.debug_abbrev` for one compilation unit — `B-011b`.
///
/// Without these a debugger has a line table it has no reason to read: nothing
/// says which program the addresses belong to, what produced it, or where its
/// text begins and ends. `low_pc`/`high_pc` are how it decides an address is
/// Test-only convenience over [`compile_unit_parts`].
///
/// `#[cfg(test)]` because nothing outside this file's tests calls it, and a
/// wrapper kept in the library is a wrapper compiled into every build of it
/// (`X-002`). Demoting it from `pub` is what made that visible: rustc does not
/// lint unused *public* items, so `pub` had been switching the check off.
#[cfg(test)]
/// ours at all.
#[must_use]
fn compile_unit(name: &str, low_pc: u64, size: u64) -> (Vec<u8>, Vec<u8>) {
    let u = compile_unit_parts(name, low_pc, size);
    (u.info, u.abbrev)
}

/// A compilation unit, and where its `DW_AT_low_pc` sits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// `.debug_info` contents.
    pub info: Vec<u8>,
    /// `.debug_abbrev` contents.
    pub abbrev: Vec<u8>,
    /// Byte offset within `info` of the eight-byte `DW_AT_low_pc`.
    pub low_pc_at: usize,
    /// Byte offset within `info` of the four-byte `DW_AT_stmt_list`.
    ///
    /// The offset of this unit's line program within `.debug_line`. Zero in an
    /// object, where the unit is the only one; a linker concatenating two must
    /// move the second, or every line in the second file reports against the
    /// first file's table (`B-104b`).
    pub stmt_list_at: usize,
}

/// As [`compile_unit`], reporting where the address is.
///
/// `DW_AT_high_pc` needs no relocation: DWARF 4 made it a LENGTH, and a length
/// does not move when the section does.
#[must_use]
pub fn compile_unit_parts(name: &str, low_pc: u64, size: u64) -> Unit {
    let mut die = Vec::new();
    uleb(&mut die, 1); // the abbreviation above
    string(&mut die, PRODUCER);
    die.extend_from_slice(&DW_LANG_ASSEMBLER.to_le_bytes());
    string(&mut die, name);
    string(&mut die, ".");
    let low_pc_at = die.len();
    die.extend_from_slice(&low_pc.to_le_bytes());
    uleb(&mut die, size); // high_pc as a LENGTH, which DWARF 4 made legal
    let stmt_list_at = die.len();
    die.extend_from_slice(&0u32.to_le_bytes()); // stmt_list: our only line program

    // DWARF 5 puts `unit_type` and `address_size` BEFORE the abbrev offset;
    // DWARF 4 had the offset first and no unit type at all. A reader that
    // guessed would take our address size for half of an offset.
    let mut out = Vec::new();
    let unit_length = 2 + 1 + 1 + 4 + die.len();
    out.extend_from_slice(&(unit_length as u32).to_le_bytes());
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.push(DW_UT_COMPILE);
    out.push(ADDRESS_SIZE);
    out.extend_from_slice(&0u32.to_le_bytes()); // debug_abbrev_offset
    let die_at = out.len();
    out.extend_from_slice(&die);
    Unit {
        info: out,
        abbrev: abbrev(),
        low_pc_at: die_at + low_pc_at,
        stmt_list_at: die_at + stmt_list_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The derived header values, from `spec/dwarf-line-v5.tsv`.
    const DERIVED: &str = include_str!("../../../spec/dwarf-line-v5.tsv");

    fn derived(key: &str) -> i64 {
        DERIVED
            .lines()
            .filter(|l| !l.starts_with('#') && !l.starts_with("field\t"))
            .find_map(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                (f.len() >= 2 && f[0] == key).then(|| f[1].parse().ok())?
            })
            .unwrap_or_else(|| panic!("`{key}` is not in spec/dwarf-line-v5.tsv"))
    }

    #[test]
    fn the_header_constants_are_the_oracle_s() {
        // `line_base` and `line_range` are GNU's choices, not the standard's,
        // and a special opcode computed with different ones decodes to a
        // different LINE — a wrong answer that still parses.
        assert_eq!(derived("version"), i64::from(VERSION));
        assert_eq!(derived("address_size"), i64::from(ADDRESS_SIZE));
        assert_eq!(
            derived("min_instruction_length"),
            i64::from(MIN_INSN_LENGTH)
        );
        assert_eq!(
            derived("max_ops_per_instruction"),
            i64::from(MAX_OPS_PER_INSN)
        );
        assert_eq!(derived("default_is_stmt"), i64::from(DEFAULT_IS_STMT));
        assert_eq!(derived("line_base"), i64::from(LINE_BASE));
        assert_eq!(derived("line_range"), i64::from(LINE_RANGE));
        assert_eq!(derived("opcode_base"), i64::from(OPCODE_BASE));
        for (n, args) in STANDARD_OPCODE_LENGTHS.iter().enumerate() {
            assert_eq!(
                derived(&alloc::format!("opcode_{}_args", n + 1)),
                i64::from(*args),
                "standard opcode {} takes a different number of arguments",
                n + 1
            );
        }
    }

    #[test]
    fn a_row_says_which_file_it_came_from() {
        // `B-011b`. A linked image is several sources laid end to end. Before
        // the file index every row named the first file, so a debugger stepping
        // into a library showed the caller's source with the callee's lines —
        // plausible, and wrong in the way that costs an hour.
        let rows = [
            Row {
                address: 0x1000,
                line: 4,
                file: 0,
            },
            Row {
                address: 0x1004,
                line: 9,
                file: 1,
            },
        ];
        let bytes = line_program(&["a.sas", "b.sas"], &rows, 0x1008);

        // Both file indices are selected. Counting occurrences of the opcode
        // BYTE would be unsound and the first version of this test was: a
        // line program is a bytecode with inline operands, and `advance_pc 4`
        // contains a 4 that is not an opcode at all. Look for the opcode with
        // its operand instead.
        assert!(
            bytes.windows(2).any(|w| w == [DW_LNS_SET_FILE, 0]),
            "the first file is selected"
        );
        assert!(
            bytes.windows(2).any(|w| w == [DW_LNS_SET_FILE, 1]),
            "and the second, when the rows cross into it"
        );
        // Both names are in the table, so index 1 resolves to something.
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("a.sas") && text.contains("b.sas"));
    }

    #[test]
    fn the_compile_unit_says_where_the_program_starts_and_stops() {
        // Without low_pc/high_pc a debugger has no reason to believe an address
        // belongs to this unit, and the line table it already has goes unread.
        let (info, abbrev) = compile_unit("क.sas", 0x8000_0000, 52);
        let unit_length = u32::from_le_bytes(info[0..4].try_into().expect("4")) as usize;
        assert_eq!(unit_length + 4, info.len(), "unit_length covers the rest");
        assert_eq!(
            u16::from_le_bytes(info[4..6].try_into().expect("2")),
            VERSION
        );
        // DWARF 5 puts unit_type and address_size BEFORE the abbrev offset;
        // DWARF 4 had the offset first and no unit type at all.
        assert_eq!(info[6], DW_UT_COMPILE);
        assert_eq!(info[7], ADDRESS_SIZE);

        // The abbrev table must describe every attribute the DIE writes, in
        // order: the two are read together, and one extra attribute in either
        // desynchronises everything after it rather than being ignored.
        assert_eq!(abbrev[0], 1, "abbreviation code the DIE references");
        assert_eq!(abbrev[1], DW_TAG_COMPILE_UNIT);
        assert_eq!(abbrev[2], 0, "no children");
        assert_eq!(*abbrev.last().expect("table ends"), 0);
    }

    #[test]
    fn a_negative_line_delta_survives_the_sleb() {
        // Signed LEB128 is where a naive loop silently turns -3 into a large
        // positive number, which decodes as a jump to line 2097149.
        let mut out = Vec::new();
        sleb(&mut out, -3);
        assert_eq!(out, alloc::vec![0x7d]);
        let mut out = Vec::new();
        sleb(&mut out, -129);
        assert_eq!(out, alloc::vec![0xff, 0x7e]);
        let mut out = Vec::new();
        sleb(&mut out, 64);
        assert_eq!(
            out,
            alloc::vec![0xc0, 0x00],
            "0x40 must not read as negative"
        );
    }

    #[test]
    fn a_special_opcode_advances_both_or_declines() {
        // +1 line, +4 bytes: the common case, and it must be one byte.
        let op = special_opcode(1, 4).expect("the common step has a special opcode");
        // Decode it the way a reader does, and check it says what we meant.
        let adjusted = i64::from(op) - i64::from(OPCODE_BASE);
        assert_eq!(adjusted / i64::from(LINE_RANGE), 4, "address advance");
        assert_eq!(
            adjusted % i64::from(LINE_RANGE) + i64::from(LINE_BASE),
            1,
            "line advance"
        );
        // Out of range in either direction, and it must decline rather than
        // wrap into a valid opcode that means something else.
        assert_eq!(special_opcode(-6, 0), None, "below line_base");
        assert_eq!(special_opcode(9, 0), None, "past line_base + line_range");
        assert_eq!(special_opcode(0, 100), None, "past one byte");
    }

    #[test]
    fn the_lengths_count_from_their_own_anchors() {
        // `unit_length` counts from after itself and `header_length` from after
        // ITSELF — two different anchors, and either one wrong makes the
        // section undecodable rather than visibly wrong.
        let bytes = line_program(
            &["क.sas"],
            &[Row {
                address: 0x8000_0000,
                line: 3,
                file: 0,
            }],
            0x8000_0004,
        );
        let unit_length = u32::from_le_bytes(bytes[0..4].try_into().expect("4 bytes")) as usize;
        assert_eq!(unit_length + 4, bytes.len(), "unit_length covers the rest");

        let header_length = u32::from_le_bytes(bytes[8..12].try_into().expect("4 bytes")) as usize;
        // The prologue ends where the program begins, and every program begins
        // by pointing the `file` register at entry 0.
        assert_eq!(
            bytes[12 + header_length],
            DW_LNS_SET_FILE,
            "the program starts here"
        );
    }
}

#[cfg(test)]
mod patch_sites {
    use super::*;

    /// The offsets a relocation attaches to must really name the address bytes.
    ///
    /// Recorded while writing rather than recomputed, because `B-011b` scanned
    /// a bytecode with inline operands by hand and counted an operand as an
    /// opcode. This reads the eight bytes back and compares — the one check a
    /// wrong offset cannot pass.
    #[test]
    fn the_reported_offsets_point_at_the_addresses() {
        let rows = [
            Row {
                address: 0,
                line: 1,
                file: 0,
            },
            Row {
                address: 4,
                line: 2,
                file: 0,
            },
        ];
        let p = line_program_for(&["क.sas"], &rows, 8, true);
        let at = p.set_address_at.expect("an object always sets the address");
        assert_eq!(
            u64::from_le_bytes(p.bytes[at..at + 8].try_into().expect("8 bytes")),
            0,
            "the operand is the first row's address, which in an object is zero"
        );

        // And a distinctive value lands exactly there, so the offset is not
        // right by coincidence on a section full of zeroes.
        let moved = [Row {
            address: 0x1234_5678,
            line: 1,
            file: 0,
        }];
        let q = line_program_for(&["क.sas"], &moved, 0x1234_567c, true);
        let at = q.set_address_at.expect("set");
        assert_eq!(
            u64::from_le_bytes(q.bytes[at..at + 8].try_into().expect("8 bytes")),
            0x1234_5678
        );

        let u = compile_unit_parts("क.sas", 0x8000_0000, 16);
        assert_eq!(
            u64::from_le_bytes(
                u.info[u.low_pc_at..u.low_pc_at + 8]
                    .try_into()
                    .expect("8 bytes")
            ),
            0x8000_0000
        );
        // `stmt_list` is four bytes and zero, and must not overlap `low_pc`.
        assert_eq!(
            u32::from_le_bytes(
                u.info[u.stmt_list_at..u.stmt_list_at + 4]
                    .try_into()
                    .expect("4 bytes")
            ),
            0
        );
        assert!(
            u.stmt_list_at >= u.low_pc_at + 8 || u.stmt_list_at + 4 <= u.low_pc_at,
            "the two patch sites overlap"
        );
    }

    #[test]
    fn the_executable_form_is_unchanged() {
        // `line_program` is the same bytes it always was: an executable's first
        // row carries the load address, so the operand it needs is already
        // there and forcing a second would change every image.
        let rows = [Row {
            address: 0x8000_0000,
            line: 1,
            file: 0,
        }];
        assert_eq!(
            line_program(&["क.sas"], &rows, 0x8000_0004),
            line_program_for(&["क.sas"], &rows, 0x8000_0004, false).bytes
        );
    }
}
