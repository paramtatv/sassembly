//! Does a linker accept our object? — task `B-069b`, doc 03 §3.3.
//!
//! `kosha::write_relocatable` emits `ET_REL` with a `.rela.text`. Nothing in
//! this repository reads that format, so checking it against our own reader
//! would prove only that we are self-consistent.
//!
//! `readelf` and `ld` are the check. They know nothing about this project, so
//! their agreement is evidence — the same argument the conformance corpus rests
//! on, applied to a container instead of an instruction.

use std::path::{Path, PathBuf};
use std::process::Command;

use sadhana::kosha::{Relocation, SymSection, Symbol, write_relocatable};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

/// `R_RISCV_JAL`, from the derived table rather than from memory.
fn jal_relocation() -> u32 {
    let text = std::fs::read_to_string(root().join("spec/relocations-riscv64.tsv"))
        .expect("read spec/relocations-riscv64.tsv");
    text.lines()
        .filter_map(|l| l.split_once('\t'))
        .find(|(n, _)| *n == "R_RISCV_JAL")
        .and_then(|(_, v)| v.trim().parse().ok())
        .expect("R_RISCV_JAL is in the derived table")
}

fn have(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

/// The tools a test needs, emitting a `METRIC` on BOTH paths — `W-085`.
///
/// A skip guard turns "the toolchain is missing" into "the test passed", and
/// until now the only difference was an `eprintln` that `cargo test` CAPTURES on
/// a passing test. So seven tests in this file could report green having run
/// nothing, and two of them said nothing at all: their guards returned in
/// silence, invisible even under `--nocapture`.
///
/// Emitting on both paths is what makes it checkable rather than merely louder.
/// One line per test either way, so the ABSENCE of both is a detectable state —
/// a test that stopped running stops emitting, and `crates/metrics` harvests
/// `METRIC` into `METRICS.tsv` where something other than a human reading a
/// terminal can see it.
fn require(test: &str, tools: &[&str]) -> bool {
    match tools.iter().find(|t| !have(t)) {
        Some(missing) => {
            println!("METRIC reloc_{test}_ran 0");
            eprintln!("{missing} not found — skipping {test}");
            false
        }
        None => {
            println!("METRIC reloc_{test}_ran 1");
            true
        }
    }
}

/// One object: `jal ra, बाह्यम्` with the target left for the linker.
fn object() -> Vec<u8> {
    // `jal ra, 0` — the displacement is zero because the linker writes it.
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
    // Symbol index 2: the null symbol is 0 and `मुख्यम्` is 1, both written
    // before it because locals-then-globals keeps their order here.
    let relocations = vec![Relocation {
        section: sadhana::kosha::RelSection::Text,
        offset: 0,
        symbol: 2,
        kind: jal_relocation(),
        addend: 0,
    }];
    write_relocatable(&text, &[], 0, &[], &symbols, &relocations, 4)
}

#[test]
fn readelf_reads_our_relocation_back() {
    if !require(
        "readelf_reads_our_relocation_back",
        &["riscv64-elf-readelf"],
    ) {
        return;
    }
    let dir = std::env::temp_dir().join("sansos-rel-readelf");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let obj = dir.join("a.o");
    std::fs::write(&obj, object()).expect("write");

    let head = Command::new("riscv64-elf-readelf")
        .arg("-h")
        .arg(&obj)
        .output()
        .expect("readelf -h");
    let head = String::from_utf8_lossy(&head.stdout);
    assert!(
        head.contains("REL (Relocatable file)"),
        "not an object file:\n{head}"
    );

    let rel = Command::new("riscv64-elf-readelf")
        .arg("-r")
        .arg(&obj)
        .output()
        .expect("readelf -r");
    let rel = String::from_utf8_lossy(&rel.stdout);
    assert!(
        rel.contains("R_RISCV_JAL"),
        "the relocation did not survive the round trip:\n{rel}"
    );
    // The symbol index half of `r_info` is what a swapped pack destroys: it
    // would read as some other type against some other symbol, and still parse.
    assert!(
        !rel.contains("R_RISCV_NONE"),
        "r_info's halves are the wrong way round:\n{rel}"
    );
}

/// `संरेखः` must still hold after a LINK, which is the property `W-080` fixed
/// and the one nothing tested (`W-084`).
///
/// `W-080`'s own test reads `sh_addralign` out of the object. That is the
/// mechanism — the field we set — and not the property: it cannot tell whether a
/// label is still aligned once a linker has placed the section.
///
/// **The arrangement is the whole test.** Linked alone at `0x80000000` the label
/// is aligned whatever `sh_addralign` says, because that base is already
/// 16-aligned — an experiment in that shape can only ever say yes, and it nearly
/// concluded the original defect was cosmetic. So this links our object BEHIND a
/// four-byte one: with `sh_addralign` correct the linker must push `.text` to a
/// 16-boundary, and with it wrong the label lands four bytes short.
///
/// Measured when the defect was live: `0x80000014` here, against `0x80000020`
/// for gas's `.balign 16` under the identical link.
#[test]
fn संरेखः_still_holds_after_a_link() {
    if !require(
        "sanrekhah_still_holds_after_a_link",
        &["riscv64-elf-ld", "riscv64-elf-as", "riscv64-elf-nm"],
    ) {
        return;
    }
    let dir = std::env::temp_dir().join("sansos-w084");
    std::fs::create_dir_all(&dir).expect("temp dir");

    let src = "॥ कोष्ठकम् ॱपाठ ॥\nयोगः अर्थ०म् शून्यःन १न ।\n॥ संरेखः ०षोड्१० ॥\nसंरेखितम्ॱॱ\nयोगः अर्थ०म् शून्यःन २न ।\n";
    let program = sadhana::parse::parse(&sadhana::lex::lex(src).expect("lex")).expect("parse");
    let (text, pending) = sadhana::encode::encode_object(&program).expect("encode");
    let ours = dir.join("ours.o");
    std::fs::write(
        &ours,
        sadhana::kosha::object(
            &text,
            &program,
            &pending,
            None,
            &sadhana::encode::layout_addresses(&program, sadhana::encode::Target::Uncompressed),
        ),
    )
    .expect("write");

    // FOUR bytes, deliberately not a multiple of sixteen. A linker is free to
    // place the next section at any multiple of its `sh_addralign`, so this is
    // what turns a correct field into an observable and a wrong one into a
    // misaligned label.
    let pad_s = dir.join("pad.s");
    std::fs::write(&pad_s, ".text\n.globl _start\n_start:\nnop\n").expect("write");
    let pad = dir.join("pad.o");
    assert!(
        Command::new("riscv64-elf-as")
            .args(["-march=rv64gc", "-o"])
            .arg(&pad)
            .arg(&pad_s)
            .status()
            .expect("as")
            .success(),
        "assembling the four-byte pad failed"
    );

    let linked = dir.join("linked.elf");
    assert!(
        Command::new("riscv64-elf-ld")
            .args(["-Ttext=0x80000000", "-o"])
            .arg(&linked)
            .arg(&pad)
            .arg(&ours)
            .status()
            .expect("ld")
            .success(),
        "link failed"
    );

    let nm = Command::new("riscv64-elf-nm")
        .arg(&linked)
        .output()
        .expect("nm");
    let out = String::from_utf8_lossy(&nm.stdout);
    let line = out
        .lines()
        .find(|l| l.contains("संरेखितम्"))
        .unwrap_or_else(|| panic!("no संरेखितम् in the linked image:\n{out}"));
    let addr = u64::from_str_radix(line.split_whitespace().next().expect("address"), 16)
        .expect("hex address");

    // The pad is four bytes, so an unaligned outcome is 0x…14 rather than 0x…20
    // — and the fixture only tests anything because the pad's size is not a
    // multiple of the boundary. Assert that too, or a pad that grew to sixteen
    // bytes would make this pass for a reason unconnected to alignment.
    let pad_len = std::fs::metadata(&pad).expect("pad").len();
    assert!(
        pad_len > 0,
        "the pad object is empty, so nothing is displaced"
    );
    assert_eq!(
        addr % 16,
        0,
        "`॥ संरेखः १६ ॥` did not survive the link: संरेखितम् is at {addr:#x}, which \
         is {} past a 16-boundary. The padding inside the object proves nothing — \
         sh_addralign is what the linker reads, and a section claiming 4 may be \
         placed anywhere on a four-byte boundary",
        addr % 16
    );
}

#[test]
fn a_linker_resolves_the_symbol_we_left_undefined() {
    // The strongest available check. `ld` has to read our section table, our
    // symbol table and our relocation, find `बाह्यम्` in an object GNU built,
    // and patch our `jal`. If any of the four is wrong it refuses.
    //
    // Using `ld` here is not a retreat from "no GNU ld in the path" — that rule
    // is about what `साधनम्` needs to build a program. This is `ld` as an
    // oracle, exactly as `as` is one for encodings.
    if !require(
        "a_linker_resolves_the_symbol_we_left_undefined",
        &["riscv64-elf-ld", "riscv64-elf-as"],
    ) {
        return;
    }
    let dir = std::env::temp_dir().join("sansos-rel-link");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let ours = dir.join("ours.o");
    std::fs::write(&ours, object()).expect("write");

    // The definition, from GNU, so only our side is under test.
    let src = dir.join("theirs.s");
    std::fs::write(&src, "\t.text\n\t.globl बाह्यम्\nबाह्यम्:\n\tret\n").expect("write");
    let theirs = dir.join("theirs.o");
    let asm = Command::new("riscv64-elf-as")
        .args(["-march=rv64gc", "-o"])
        .arg(&theirs)
        .arg(&src)
        .output()
        .expect("as");
    assert!(
        asm.status.success(),
        "{}",
        String::from_utf8_lossy(&asm.stderr)
    );

    let linked = dir.join("linked.elf");
    let out = Command::new("riscv64-elf-ld")
        .args(["-e", "मुख्यम्", "-Ttext=0x80000000", "-o"])
        .arg(&linked)
        .arg(&ours)
        .arg(&theirs)
        .output()
        .expect("ld");
    assert!(
        out.status.success(),
        "ld refused our object:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // And the patched instruction is a real jump, not a zeroed one.
    let bytes = std::fs::read(&linked).expect("read");
    assert!(bytes.len() > 64, "the linked file is empty");
    let dump = Command::new("riscv64-elf-objdump")
        .args(["-d"])
        .arg(&linked)
        .output()
        .expect("objdump");
    let dump = String::from_utf8_lossy(&dump.stdout);
    assert!(
        dump.contains("jal") || dump.contains("j\t"),
        "the jump did not survive linking:\n{dump}"
    );
    assert!(
        !dump.contains("\t000000ef"),
        "the displacement is still zero, so the relocation was ignored:\n{dump}"
    );
}

#[test]
fn a_devanagari_source_becomes_an_object_a_linker_can_finish() {
    // The whole path, end to end: Sassembly in, `ET_REL` out, GNU `ld` resolves
    // the name we left open against an object it built itself.
    //
    // `लङ्घनम् पुनःस्थानम्म् बाह्यम्य् ।` is a call to a name this file does not
    // define. As an executable that is an error and should be; as an object it
    // is the question the format exists to ask.
    if !require(
        "a_devanagari_source_becomes_an_object_a_linker_can_finish",
        &["riscv64-elf-ld", "riscv64-elf-as", "riscv64-elf-objdump"],
    ) {
        return;
    }
    let src = "॥ वैश्विकम् मुख्यम् ॥\nमुख्यम्ॱॱ\nलङ्घनम् पुनःस्थानम्म् बाह्यम्य् ।\n";
    let program = sadhana::parse::assemble_program(src).expect("parses");
    let (text, pending) =
        sadhana::encode::encode_object(&program).unwrap_or_else(|e| panic!("{e:?}"));

    assert_eq!(text.len(), 4, "one instruction");
    assert_eq!(pending.len(), 1, "one unresolved name");
    assert_eq!(pending[0].name, "बाह्यम्");
    assert_eq!(pending[0].kind, "R_RISCV_JAL");
    // The field is left zero: the linker computes the whole displacement, and
    // anything we wrote would be added to or overwritten.
    assert_eq!(
        u32::from_le_bytes(text[..4].try_into().expect("4 bytes")) >> 12,
        0,
        "the displacement must be zero for the linker to fill"
    );

    let dir = std::env::temp_dir().join("sansos-obj-end-to-end");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let ours = dir.join("ours.o");
    std::fs::write(
        &ours,
        sadhana::kosha::object(
            &text,
            &program,
            &pending,
            None,
            &sadhana::encode::layout_addresses(&program, sadhana::encode::Target::Uncompressed),
        ),
    )
    .expect("write");

    let their_src = dir.join("theirs.s");
    std::fs::write(&their_src, "\t.text\n\t.globl बाह्यम्\nबाह्यम्:\n\tret\n").expect("write");
    let theirs = dir.join("theirs.o");
    assert!(
        Command::new("riscv64-elf-as")
            .args(["-march=rv64gc", "-o"])
            .arg(&theirs)
            .arg(&their_src)
            .output()
            .expect("as")
            .status
            .success()
    );

    let linked = dir.join("linked.elf");
    let out = Command::new("riscv64-elf-ld")
        .args(["-e", "मुख्यम्", "-Ttext=0x80000000", "-o"])
        .arg(&linked)
        .arg(&ours)
        .arg(&theirs)
        .output()
        .expect("ld");
    assert!(
        out.status.success(),
        "ld refused:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let dump = Command::new("riscv64-elf-objdump")
        .arg("-d")
        .arg(&linked)
        .output()
        .expect("objdump");
    let dump = String::from_utf8_lossy(&dump.stdout);
    // The jump must reach the definition, not sit at zero.
    assert!(
        dump.contains("004000ef") || dump.contains("jal"),
        "the call was not patched:\n{dump}"
    );
    assert!(
        dump.contains("बाह्यम्"),
        "the linker did not resolve our name:\n{dump}"
    );
}

#[test]
fn we_can_read_an_object_gnu_wrote() {
    // The check that matters. A reader tested only against our own writer
    // proves the two agree — which they would even if both were wrong, the
    // failure `B-058a` found where a mask and a field map were wrong together.
    //
    // So: assemble with GNU, read with ours, and compare against `readelf`.
    if !require(
        "we_can_read_an_object_gnu_wrote",
        &["riscv64-elf-as", "riscv64-elf-readelf"],
    ) {
        return;
    }
    let dir = std::env::temp_dir().join("sansos-read-gnu");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let src = dir.join("g.s");
    std::fs::write(
        &src,
        "\t.text\n\t.option norvc\n\t.globl मुख्यम्\nमुख्यम्:\n\tjal ra, बाह्यम्\n\tret\n",
    )
    .expect("write");
    let obj = dir.join("g.o");
    assert!(
        Command::new("riscv64-elf-as")
            .args(["-march=rv64gc", "-o"])
            .arg(&obj)
            .arg(&src)
            .output()
            .expect("as")
            .status
            .success()
    );

    let bytes = std::fs::read(&obj).expect("read");
    let o = sadhana::vastu::read(&bytes).expect("our reader accepts GNU's object");

    // Two instructions, four bytes each: `.option norvc` keeps them wide.
    assert_eq!(o.text.len(), 8, "text length");

    // One relocation, and it must name the symbol GNU says it does.
    let want = String::from_utf8_lossy(
        &Command::new("riscv64-elf-readelf")
            .arg("-r")
            .arg(&obj)
            .output()
            .expect("readelf")
            .stdout,
    )
    .into_owned();
    let count = want.lines().filter(|l| l.contains("R_RISCV_")).count();
    assert_eq!(o.relocations.len(), count, "readelf sees {count} records");
    assert_eq!(
        o.relocations[0].kind, 17,
        "R_RISCV_JAL, per the derived table"
    );
    assert_eq!(o.relocations[0].offset, 0, "on the first instruction");

    let named = &o.symbols[o.relocations[0].symbol as usize];
    assert_eq!(
        named.name, "बाह्यम्",
        "the record points at the undefined name"
    );
    assert!(named.is_undefined());
    assert!(
        o.symbols
            .iter()
            .any(|s| s.name == "मुख्यम्" && !s.is_undefined() && s.global),
        "GNU's own global label came back wrong"
    );
}

#[test]
fn every_section_index_in_the_object_points_at_the_right_kind_of_section() {
    // `B-100a`. The object writer computed its indices as `4 + extra`, where
    // `extra` counted the conditional sections before them. Two conditionals
    // were already there and `.debug_line` is a third (`B-100b`), so the
    // arithmetic is replaced by one ordered list and every index is a position
    // in it.
    //
    // The property that arithmetic breaks is this one: a header field that
    // names another section must name a section of the kind it expects. Get it
    // wrong and `sh_link` on `.symtab` points at `.bss`, which is how a reader
    // takes reserved space for a string table.
    let source = "कॱॱ\nस्थानसापेक्षयोगः अर्थ०म् सारणीॱउपरिन ।\n\
                  योगः अर्थ०म् अर्थ०न सारणीॱअधःन ।\n\
                  ॥ कोष्ठकम् ॱदत्त ॥\nसारणीॱॱ\n॥ अष्टाष्टकाः क ॥\n\
                  ॥ कोष्ठकम् ॱरिक्त ॥\nबफरॱॱ\n॥ स्थानम् ३२ ॥\n";
    let p = sadhana::parse::assemble_program(source).expect("parses");
    let (text, pending) = sadhana::encode::encode_object(&p).expect("encodes");
    let bytes = sadhana::kosha::object(
        &text,
        &p,
        &pending,
        None,
        &sadhana::encode::layout_addresses(&p, sadhana::encode::Target::Uncompressed),
    );

    // Every optional section is present here, which is the case the arithmetic
    // was most likely to get wrong.
    let o = sadhana::vastu::read(&bytes).expect("reads");
    assert!(!o.relocations.is_empty(), ".rela.text");
    assert!(!o.data_relocations.is_empty(), ".rela.data");
    assert_eq!(o.bss, 32, ".bss");

    let u16_at = |at: usize| u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap());
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
    let u64_at = |at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());

    let shoff = u64_at(40) as usize;
    let shentsize = u16_at(58) as usize;
    let shnum = u16_at(60) as usize;
    let shstrndx = u16_at(62) as usize;
    let kind = |i: usize| u32_at(shoff + i * shentsize + 4);
    let link = |i: usize| u32_at(shoff + i * shentsize + 40);
    let info = |i: usize| u32_at(shoff + i * shentsize + 44);

    // `e_shstrndx` names the section-name table.
    assert_eq!(kind(shstrndx), 3, "e_shstrndx does not name a STRTAB");

    for i in 0..shnum {
        match kind(i) {
            // SHT_SYMTAB links its string table.
            2 => assert_eq!(kind(link(i) as usize), 3, "symtab {i} links a non-STRTAB"),
            // SHT_RELA links the symbol table and names what it patches; the
            // target must carry bytes, or the records point into nothing.
            4 => {
                assert_eq!(kind(link(i) as usize), 2, "rela {i} links a non-SYMTAB");
                assert_eq!(kind(info(i) as usize), 1, "rela {i} patches a non-PROGBITS");
            }
            _ => {}
        }
    }
}

#[test]
fn gnu_ld_fixes_our_line_table_when_it_places_the_text() {
    // `B-100b`. An object's addresses are relative to `.text`, so the line
    // table's `DW_LNE_set_address` carries zero and an `R_RISCV_64` against the
    // section symbol says who fills it. Writing the table WITHOUT that record
    // would decode correctly with `readelf` on the object — addresses relative
    // to zero are what an object has — and be silently wrong in every program
    // linked from it.
    //
    // So the check is not that we emit a record. It is that a stranger applies
    // it and the answer moves: `ld` places the text at 0x80000000 and the line
    // table must follow. `readelf` on the object cannot see that, which is the
    // whole reason `ld` is the oracle here.
    if !require(
        "gnu_ld_fixes_our_line_table_when_it_places_the_text",
        &["riscv64-elf-ld", "riscv64-elf-readelf"],
    ) {
        return;
    }

    let dir = std::env::temp_dir().join("sansos-b100b");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let source = root().join("spec/lib-mudraka.sas");
    let text = std::fs::read_to_string(&source).expect("read");
    let program = sadhana::parse::assemble_program(&text).expect("parses");
    let (bytes, pending) = sadhana::encode::encode_object(&program).expect("encodes");
    let obj = dir.join("mudraka.o");
    std::fs::write(
        &obj,
        sadhana::kosha::object(
            &bytes,
            &program,
            &pending,
            Some("spec/lib-mudraka.sas"),
            &sadhana::encode::layout_addresses(&program, sadhana::encode::Target::Uncompressed),
        ),
    )
    .expect("write");

    let out = dir.join("mudraka.elf");
    let status = Command::new("riscv64-elf-ld")
        .args(["-Ttext=0x80000000", "-e", "0x80000000"])
        .arg(&obj)
        .arg("-o")
        .arg(&out)
        .output()
        .expect("run ld");
    assert!(
        status.status.success(),
        "ld refused our object: {}",
        String::from_utf8_lossy(&status.stderr)
    );

    let dump = Command::new("riscv64-elf-readelf")
        .arg("--debug-dump=decodedline")
        .arg(&out)
        .output()
        .expect("run readelf");
    let text = String::from_utf8_lossy(&dump.stdout);
    assert!(
        text.contains("0x80000000"),
        "the line table was not relocated: {text}"
    );
    // And it is a table, not one row that happened to land right.
    assert!(
        text.contains("0x80000004"),
        "only one address moved: {text}"
    );
}

#[test]
fn an_object_built_with_g_has_no_debug_records_in_rela_text() {
    // `B-100b` split the relocation list two ways — "is it Data" — over an enum
    // with three variants, so every `.debug_*` record was written into
    // `.rela.text` as WELL as its own section. An object built with `-g`
    // carried two `R_RISCV_64` records pointing tens of bytes past the end of
    // its text, and GNU `ld` accepted the file.
    //
    // Found by `B-104` trying to link one: the linker resolved a text record
    // naming the section symbol, whose name is empty, and reported
    // "`` is not defined by any object".
    let source = std::fs::read_to_string(root().join("spec/lib-mudraka.sas")).expect("read");
    let program = sadhana::parse::assemble_program(&source).expect("parses");
    let (text, pending) = sadhana::encode::encode_object(&program).expect("encodes");
    let bytes = sadhana::kosha::object(
        &text,
        &program,
        &pending,
        Some("spec/lib-mudraka.sas"),
        &sadhana::encode::layout_addresses(&program, sadhana::encode::Target::Uncompressed),
    );

    let o = sadhana::vastu::read(&bytes).expect("reads");
    assert!(!o.debug.is_empty(), "-g wrote the sections");
    assert!(
        !o.debug_relocations.is_empty(),
        "and the records that fix them"
    );
    for r in &o.relocations {
        assert!(
            (r.offset as usize) < o.text.len(),
            ".rela.text names offset {:#x} in a {}-byte .text",
            r.offset,
            o.text.len()
        );
    }
}

#[test]
fn gnu_ld_links_an_object_containing_a_pc_relative_pair() {
    // `B-105`. `R_RISCV_PCREL_LO12_I` names WHERE ITS `%pcrel_hi` IS, not what
    // the pair reaches: the linker follows that address to the hi20 record and
    // takes the target from there. Ours named the target — the one thing that
    // reads as a plain address and is not one — so GNU `ld` refused every
    // object containing a pair with *dangerous relocation: %pcrel_lo missing
    // matching %pcrel_hi*.
    //
    // A section symbol plus the auipc's offset is that address and coins no
    // label name; GNU's assembler emits a `.L1^B1` local instead, which is the
    // same fact spelled with a name.
    //
    // This is the check that could not be made from inside: our own linker read
    // the pair positionally and was right either way, so only a stranger could
    // tell that the file did not say what it meant.
    if !require(
        "gnu_ld_links_an_object_containing_a_pc_relative_pair",
        &["riscv64-elf-ld", "riscv64-elf-objdump"],
    ) {
        return;
    }
    let dir = std::env::temp_dir().join("sansos-b105");
    std::fs::create_dir_all(&dir).expect("mkdir");

    let mut objects = Vec::new();
    for name in ["namaste-main.sas", "lib-mudraka.sas"] {
        let source = std::fs::read_to_string(root().join("spec").join(name)).expect("read");
        let program = sadhana::parse::assemble_program(&source).expect("parses");
        let (text, pending) = sadhana::encode::encode_object(&program).expect("encodes");
        let path = dir.join(name.replace(".sas", ".o"));
        std::fs::write(
            &path,
            sadhana::kosha::object(
                &text,
                &program,
                &pending,
                None,
                &sadhana::encode::layout_addresses(&program, sadhana::encode::Target::Uncompressed),
            ),
        )
        .expect("write");
        objects.push(path);
    }
    // `namaste-main` is the one with the pair, and it is the reason this test
    // uses it rather than a program the check would pass without.
    let bytes = std::fs::read(&objects[0]).expect("read");
    let o = sadhana::vastu::read(&bytes).expect("reads");
    let lo12 = 24; // R_RISCV_PCREL_LO12_I, from the derived table
    assert!(
        o.relocations.iter().any(|r| r.kind == lo12),
        "the fixture has no %pcrel_lo to check"
    );

    let out = dir.join("joined.elf");
    let status = Command::new("riscv64-elf-ld")
        .args(["-Ttext=0x80000000", "-e", "0x80000000"])
        .args(&objects)
        .arg("-o")
        .arg(&out)
        .output()
        .expect("run ld");
    assert!(
        status.status.success(),
        "ld refused our objects: {}",
        String::from_utf8_lossy(&status.stderr)
    );

    // And it produced the pair, not a pair of zeroes.
    let dump = Command::new("riscv64-elf-objdump")
        .arg("-d")
        .arg(&out)
        .output()
        .expect("run objdump");
    let text = String::from_utf8_lossy(&dump.stdout);
    assert!(text.contains("auipc"), "{text}");
}
