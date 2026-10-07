//! Directives and sections — task `B-057`.
//!
//! Doc 02 §2.5 named the directive set when the language was designed;
//! `spec/directives.tsv` is that decision made machine-readable. A directive is
//! wrapped in `॥` where an instruction ends in `।`, so the two are told apart
//! **without knowing the names** — doc 15 §3.1 calls that a genuinely nice
//! property of a script that already had two terminators.

use sadhana::parse::{Program, Section, assemble_program, directives};
use sadhana::samyojana::{Linked, link};

/// Parse one source, as a single-file build does.
fn one(src: &str) -> Program {
    assemble_program(src).unwrap_or_else(|e| panic!("{src}: {e:?}"))
}

/// Assemble one program to an object and link it, as every build now does.
///
/// `बन्धकः` linked from parse trees and is gone (`B-096b`); this is the path a
/// build takes, so a test that used the other one was testing something no
/// program does.
fn link_one(program: &Program) -> Result<Linked, Vec<String>> {
    let (text, pending) =
        sadhana::encode::encode_object(program).map_err(|e| vec![format!("{e:?}")])?;
    let bytes = sadhana::kosha::object(
        &text,
        program,
        &pending,
        None,
        &sadhana::encode::layout_addresses(program, sadhana::encode::Target::Uncompressed),
    );
    let object = sadhana::vastu::read(&bytes).ok_or_else(|| vec!["unreadable".to_string()])?;
    link(&[object])
}

#[test]
fn the_registry_holds_what_doc_02_named() {
    let d = directives();
    assert_eq!(d.len(), 18, "doc 02 §2.5 lists sixteen, plus आस्की and जाल");
    for name in ["कोष्ठकम्", "वैश्विकम्", "संरेखः", "अष्टकाः", "कच्चा", "आस्की", "जाल"]
    {
        assert!(d.iter().any(|k| k.name == name), "{name} is missing");
    }
    let live = d.iter().filter(|k| k.status == "live").count();
    assert_eq!(live, 11, "eleven implemented, seven named and not");
}

#[test]
fn a_section_switch_moves_what_follows() {
    let p = assemble_program("योगः अर्थ०म् शून्यःन १न ।\n॥ कोष्ठकम् ॱदत्त ॥\n॥ चतुरष्टकाः ७ ॥\n")
        .expect("parses");
    assert_eq!(p.instructions.len(), 1);
    assert_eq!(p.data.len(), 1);
    assert_eq!(p.data[0].section, Section::Data);
    assert_eq!(p.data[0].bytes, vec![7, 0, 0, 0], "little-endian word");
}

#[test]
fn each_width_emits_its_own_number_of_bytes() {
    for (src, want) in [
        ("॥ अष्टकाः २५५ ॥", vec![255u8]),
        ("॥ द्वयष्टकाः ०षोड्बएएफ ॥", vec![]),
        ("॥ चतुरष्टकाः १ २ ॥", vec![1, 0, 0, 0, 2, 0, 0, 0]),
        ("॥ अष्टाष्टकाः १ ॥", vec![1, 0, 0, 0, 0, 0, 0, 0]),
        ("॥ स्थानम् ३ ॥", vec![0, 0, 0]),
    ] {
        if want.is_empty() {
            continue; // that hex is not writable; covered by the numeral tests
        }
        let p = assemble_program(&format!("॥ कोष्ठकम् ॱदत्त ॥\n{src}\n")).expect("parses");
        assert_eq!(p.data[0].bytes, want, "{src}");
    }
}

#[test]
fn a_negative_value_is_stored_two_s_complement() {
    // ADR-0009 made `ऋण` writable; a data directive is where it lands in bytes.
    let p = assemble_program("॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टकाः ऋण१ ॥\n").expect("parses");
    assert_eq!(p.data[0].bytes, vec![0xff]);
}

#[test]
fn a_named_but_unimplemented_directive_is_refused() {
    // The whole point of recording `planned` in the registry. An assembler that
    // quietly drops `॥ संरेखः ४ ॥` emits a program wrong by four bytes and says
    // nothing — so a directive doc 02 named and nothing implements must fail
    // loudly rather than be ignored.
    let e = assemble_program("॥ समम् क ५ ॥\n").expect_err("must not parse");
    // The directive at fault, not the English for "unimplemented": `B-078`
    // moved this into the table and the parser speaks Sanskrit.
    assert!(e[0].contains("समम्"), "got: {}", e[0]);
    assert!(
        e[0].contains(".equ"),
        "the message should name the gas equivalent"
    );
}

#[test]
fn an_unknown_directive_names_the_registry() {
    let e = assemble_program("॥ अज्ञातः ५ ॥\n").expect_err("must not parse");
    assert!(e[0].contains("directives.tsv"), "got: {}", e[0]);
}

#[test]
fn an_instruction_in_the_data_section_is_refused() {
    let e =
        assemble_program("॥ कोष्ठकम् ॱदत्त ॥\nयोगः अर्थ०म् शून्यःन १न ।\n").expect_err("must not parse");
    // The instruction at fault and the section that refused it, both of which
    // survive translation — `B-078` moved this message into the table.
    assert!(e[0].contains("योगः"), "got: {}", e[0]);
    assert!(e[0].contains("ॱदत्त"), "got: {}", e[0]);
}

#[test]
fn a_global_is_recorded() {
    // With the label too: a name exported and not defined is refused where the
    // unit is read (`B-096`), so a program cannot declare one without it.
    let p = assemble_program("॥ वैश्विकम् मुख्यम् ॥\nमुख्यम्ॱॱ\nयोगः अर्थ०म् शून्यःन १न ।\n").expect("parses");
    assert_eq!(p.globals, vec!["मुख्यम्".to_string()]);

    let e = assemble_program("॥ वैश्विकम् मुख्यम् ॥\n").expect_err("nothing defines it");
    assert!(e[0].contains("मुख्यम्"), "{}", e[0]);
}

#[test]
fn the_data_reaches_the_elf() {
    let p = assemble_program("योगः अर्थ०म् शून्यःन १न ।\n॥ कोष्ठकम् ॱदत्त ॥\n॥ चतुरष्टकाः ०षोड्४१ ॥\n")
        .expect("parses");
    let text = sadhana::encode::encode_program(&p).expect("encodes");
    let bytes: Vec<u8> = p.data.iter().flat_map(|d| d.bytes.clone()).collect();
    let elf = sadhana::kosha::write_with_data(&text, &bytes);
    // 0x41 is somewhere in the image, and the section count says .data exists.
    // Four: null, .text, .data, .shstrtab. No `.bss` because nothing was
    // reserved and no `.symtab` because there are no labels — `B-071` emits
    // only the sections that exist, since an empty header is 64 bytes that
    // every image would carry.
    assert_eq!(u16::from_le_bytes([elf[60], elf[61]]), 4, "four sections");
    assert!(
        elf.windows(4).any(|w| w == [0x41, 0, 0, 0]),
        "the datum is in the file"
    );
}

#[test]
fn a_string_literal_emits_its_utf8() {
    // ADR-0003: `꣹ … ꣹` measured 0 of 27 faces, so the delimiters are words.
    let p = assemble_program("॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टकाः उक्तम् नमस्ते इति ॥\n").expect("parses");
    assert_eq!(
        p.data[0].bytes,
        "नमस्ते".as_bytes(),
        "the text's UTF-8, exactly"
    );
    assert_eq!(
        p.data[0].bytes.len(),
        18,
        "six code points, three bytes each"
    );
}

#[test]
fn a_string_is_not_terminated_for_you() {
    // Doc 02 §2.5 maps this to `.ascii`, not `.asciz`. A program that wants a
    // terminator writes one and can see it — a hidden byte in a data section is
    // mass nobody asked for, and doc 18 counts every byte.
    let p = assemble_program("॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टकाः उक्तम् क इति ॥\n").expect("parses");
    assert_eq!(p.data[0].bytes, "क".as_bytes());
}

#[test]
fn a_doubled_iti_is_the_word_itself() {
    // `B-067`, ADR-0011. Before this, the closing delimiter could not appear in
    // a string at all, so the commonest word in classical Sanskrit quotation —
    // `इति` — was the one word unwritable inside a quotation.
    let p =
        assemble_program("॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टकाः उक्तम् सः इति इति अवदत् इति ॥\n").expect("parses");
    assert_eq!(
        p.data[0].bytes,
        "सः इति अवदत्".as_bytes(),
        "the doubled pair is one literal इति and the string runs on"
    );
}

#[test]
fn a_lone_doubled_iti_does_not_close_the_string() {
    // `उक्तम् क इति इति` is a string containing `क इति` and nothing closing it.
    // Reading the pair as a close-plus-stray would accept a program whose text
    // ends where the author did not say it does.
    let e = assemble_program("॥ अष्टकाः उक्तम् क इति इति ॥\n").expect_err("must not parse");
    assert!(e[0].contains("इति"), "got: {}", e[0]);
}

#[test]
fn nothing_may_follow_the_closing_iti() {
    // Silently dropping it would let this assemble as `क` — a program that does
    // not say what it does.
    let e = assemble_program("॥ अष्टकाः उक्तम् क इति ख ॥\n").expect_err("must not parse");
    assert!(e[0].contains('ख'), "the stray word is named: {}", e[0]);
}

#[test]
fn an_unclosed_string_is_refused() {
    let e = assemble_program("॥ अष्टकाः उक्तम् नमस्ते ॥\n").expect_err("must not parse");
    assert!(e[0].contains("इति"), "got: {}", e[0]);
}

#[test]
fn only_the_byte_directive_takes_a_string() {
    // A string is a sequence of bytes; asking a 32-bit directive to hold one
    // would silently pad or truncate it.
    let e = assemble_program("॥ चतुरष्टकाः उक्तम् क इति ॥\n").expect_err("must not parse");
    assert!(e[0].contains("अष्टकाः"), "got: {}", e[0]);
}

#[test]
fn reserved_space_costs_memory_and_not_file() {
    // `B-068`. A zeroed buffer in `ॱदत्त` costs its own size on disk; in
    // `ॱरिक्त` it costs only address space. With the hello-world budget
    // enforced (`B-022`) that difference is the point of the section.
    let p = assemble_program("॥ कोष्ठकम् ॱरिक्त ॥\nबफरःॱॱ\n॥ स्थानम् ४०९६ ॥\n").expect("parses");
    assert_eq!(p.bss, 4096);
    assert!(p.data.is_empty(), "nothing was written to the file");

    let text = sadhana::encode::encode_program(&p).expect("encodes");
    let elf = sadhana::kosha::write_full(&text, &[], &[], p.bss as u64);
    assert!(
        elf.len() < 1024,
        "4 KiB reserved, {} bytes on disk",
        elf.len()
    );

    // p_memsz must exceed p_filesz by the reservation, or the loader gives the
    // program nothing and every write into the buffer lands somewhere else. The
    // reservation is in the SECOND program header, the writable one (`W-363`):
    // the first is the text, `R E`, and reserves nothing.
    let ph = 64usize + 56;
    assert_eq!(
        u32::from_le_bytes(elf[ph + 4..ph + 8].try_into().expect("4")),
        0b110,
        "the segment holding `.bss` is PF_R | PF_W"
    );
    let filesz = u64::from_le_bytes(elf[ph + 32..ph + 40].try_into().expect("8"));
    let memsz = u64::from_le_bytes(elf[ph + 40..ph + 48].try_into().expect("8"));
    assert_eq!(memsz - filesz.next_multiple_of(8), 4096);
}

#[test]
fn a_value_cannot_be_written_into_reserved_space() {
    // Nothing in the file backs it, so it would be dropped at load.
    let e = assemble_program("॥ कोष्ठकम् ॱरिक्त ॥\n॥ चतुरष्टकाः ७ ॥\n").expect_err("must not parse");
    assert!(e[0].contains("ॱरिक्त"), "got: {}", e[0]);
}

#[test]
fn the_grammar_the_table_and_the_parser_agree_that_a_directive_is_wrapped() {
    // `B-094` and ADR-0012. Three places describe this construct and no two
    // agreed: `directives.tsv` said *wrapped*, the frozen EBNF opened nothing,
    // and the parser accepted either. The gap was found by trying to write a
    // fourth description for `B-034`, which is not a way anyone should have to
    // find it — so all three are asserted here against each other.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("root");

    // 1. The grammar. The production must open AND close with the mark.
    let ebnf = std::fs::read_to_string(root.join("spec/grammar-t0.ebnf")).expect("read");
    // Read to the `;`, not to the newline: ADR-0013 wrapped this production
    // across two lines and a line-based reading then saw one mark instead of
    // two. A production ends where EBNF says it does.
    let at = ebnf
        .find("\ndirective  ")
        .expect("the directive production is defined");
    let production = &ebnf[at + 1..][..ebnf[at + 1..].find(';').expect("a production ends")];
    let (_, rhs) = production.split_once('=').expect("a production has a rhs");
    assert_eq!(
        rhs.matches("double_danda").count(),
        2,
        "the directive production is not wrapped: {production}"
    );
    assert!(
        rhs.trim_start().starts_with("double_danda"),
        "no opening mark, which is the whole of ADR-0012: {production}"
    );
    // And what sits between the marks is a directive's own kind of argument,
    // not an instruction's kāraka-marked operand — ADR-0013. This said
    // `{ operand }` for 159 cycles and described no directive ever written.
    assert!(
        rhs.contains("directive_argument"),
        "a directive takes arguments, not operands: {production}"
    );

    // 2. The table, which claims the property the opening mark provides.
    let tsv = std::fs::read_to_string(root.join("spec/directives.tsv")).expect("read");
    assert!(
        tsv.contains("wrapped in `॥`"),
        "spec/directives.tsv no longer states the property it is asserting"
    );

    // 3. The parser. Wrapped assembles; the form the frozen grammar used to
    //    describe does not, and says which mark is missing.
    assemble_program("॥ वैश्विकम् क ॥\nकॱॱ\nयोगः अर्थ०म् शून्यःन १न ।\n").expect("wrapped parses");
    let e = assemble_program("वैश्विकम् क ॥\n").expect_err("an unopened directive is refused");
    assert_eq!(e.len(), 1, "one omission, one diagnostic: {e:?}");
    assert!(e[0].contains("ADR-0012"), "{}", e[0]);

    // A lone mark was silently an empty statement for 150 cycles.
    assert!(assemble_program("॥\n").is_err(), "a lone ॥ assembled");
    assert!(
        assemble_program("॥ ॥\n").is_err(),
        "an empty directive assembled"
    );
}

#[test]
fn an_unclosed_directive_does_not_swallow_the_one_after_it() {
    // `॥` opens and closes, so it is a paired delimiter, and a missing mark
    // could make the NEXT directive's opener read as this one's closer. That
    // would turn one omission into a program that assembles and is wrong.
    let e = assemble_program("॥ वैश्विकम् क\n॥ कोष्ठकम् ॱदत्त ॥\n")
        .expect_err("an unclosed directive is refused");
    // The section switch must not have been consumed as this directive's tail.
    assert!(
        e.iter().any(|m| m.contains("वैश्विकम्") || m.contains("॥")),
        "{e:?}"
    );
}

#[test]
fn a_name_in_data_is_the_address_of_what_it_names() {
    // `B-095`, ADR-0013. Until this, `ॱदत्त` held numbers and text and nothing
    // else, so a jump table, a vtable and a table of string pointers were all
    // unwritable — and `R_RISCV_64`, derived in `B-069a`, had no producer.
    let program = one(
        "मुख्यम्ॱॱ\nयोगः अर्थ०म् शून्यःन ७न ।\nदूसराॱॱ\nयोगः अर्थ०म् शून्यःन ९न ।\n\
         ॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टाष्टकाः मुख्यम् दूसरा ॥\n",
    );
    let image = link_one(&program).expect("links");

    assert_eq!(image.data.len(), 16, "two addresses, eight bytes each");
    let entry = |n: usize| u64::from_le_bytes(image.data[n * 8..n * 8 + 8].try_into().unwrap());
    // Against the symbol table, not against a constant: a hand-computed
    // expectation is derived from the same layout it is meant to check, which
    // is the mistake `B-042` and `B-055` both made.
    let address = |name: &str| *image.symbols.get(name).expect("named");
    assert_eq!(entry(0), address("मुख्यम्"));
    assert_eq!(entry(1), address("दूसरा"));
    // And they are different, or the test would pass on a table of one value
    // written twice.
    assert_ne!(entry(0), entry(1));
}

#[test]
fn an_address_is_sixty_four_bits_and_the_narrow_directives_refuse_one() {
    // Writing an address into `चतुरष्टकाः` would keep the low half and discard
    // the rest — a program that assembles and is wrong, which is the failure
    // mode this project keeps finding.
    for narrow in ["अष्टकाः", "द्वयष्टकाः", "चतुरष्टकाः", "स्थानम्"]
    {
        let src = format!("कॱॱ\n॥ कोष्ठकम् ॱदत्त ॥\n॥ {narrow} क ॥\n");
        let e = sadhana::parse::assemble_program(&src).expect_err("must refuse");
        assert!(e[0].contains("ADR-0013"), "{narrow}: {}", e[0]);
    }
    // The wide one takes it.
    sadhana::parse::assemble_program("कॱॱ\n॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टाष्टकाः क ॥\n")
        .expect("अष्टाष्टकाः holds an address");
}

#[test]
fn a_name_nothing_defines_is_refused_rather_than_written_as_zero() {
    // A zero here is a table entry that points at the reset vector.
    let p = sadhana::parse::assemble_program(
        "कॱॱ\nयोगः अर्थ०म् शून्यःन ७न ।\n॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टाष्टकाः अनुपस्थितः ॥\n",
    )
    .expect("parses — the name is a link-time question");
    let e = link_one(&p).expect_err("must not link");
    assert!(e.iter().any(|m| m.contains("अनुपस्थितः")), "{e:?}");
}

#[test]
fn an_object_records_an_address_as_a_relocation_rather_than_a_value() {
    // `B-098`. This was refused (`E18`) for two cycles, because `.rela.data` is
    // a section `kosha` did not write and the zeros the parser reserved would
    // have reached the file unrelocated — an object that links and points at
    // nothing, which is the defect `B-069d2b` shipped.
    //
    // An object has no load address, so the value does not exist while
    // assembling. The eight bytes stay zero and the record says who fills them.
    let p = one("कॱॱ\nयोगः अर्थ०म् शून्यःन ७न ।\n॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टाष्टकाः क ॥\n");
    let (text, pending) = sadhana::encode::encode_object(&p).expect("an object records it");

    let data: Vec<&sadhana::encode::Pending> = pending
        .iter()
        .filter(|p| p.section == sadhana::kosha::RelSection::Data)
        .collect();
    assert_eq!(data.len(), 1, "one address, one record: {pending:?}");
    assert_eq!(data[0].kind, "R_RISCV_64", "an absolute pointer");
    assert_eq!(data[0].at, 0, "at the start of ॱदत्त");
    assert_eq!(data[0].name, "क");

    // And it round-trips through the file, into the list that says which
    // section it patches. Sharing one list with `.rela.text` would apply a data
    // offset to an instruction, which assembles, links and runs.
    let bytes = sadhana::kosha::object(
        &text,
        &p,
        &pending,
        None,
        &sadhana::encode::layout_addresses(&p, sadhana::encode::Target::Uncompressed),
    );
    let o = sadhana::vastu::read(&bytes).expect("reads");
    assert_eq!(o.data_relocations.len(), 1, "{:?}", o.data_relocations);
    assert!(o.relocations.is_empty(), "nothing patches .text here");
    assert_eq!(o.data, [0u8; 8], "the value is not written yet");

    // The linker writes it, and to the symbol's own address rather than to a
    // distance: a pointer in a table is dereferenced, never added to a pc.
    let linked = sadhana::samyojana::link(&[o]).expect("links");
    let written = u64::from_le_bytes(linked.data[..8].try_into().expect("8 bytes"));
    assert_eq!(written, *linked.symbols.get("क").expect("क is defined"));
    assert_ne!(
        written, 0,
        "an unrelocated zero is the defect this replaced"
    );
}

#[test]
fn exporting_a_name_nothing_defines_is_refused_where_the_unit_is_read() {
    // `वैश्विकम् X` says *this file defines X and shows it to others* — a claim
    // about one translation unit. The check lived in `बन्धकः` and the object
    // path did not have it, so `--वस्तु` wrote a file whose export was a name
    // nothing in it defined and said nothing (`B-096a`).
    //
    // It is the parser's now, where every path that reads a unit meets it, and
    // it outlived the linker it came from (`B-096b`).
    let e = assemble_program("॥ वैश्विकम् अनुपस्थितः ॥\n").expect_err("must not parse");
    assert!(e[0].contains("अनुपस्थितः"), "the name at fault: {}", e[0]);
    assert!(
        e[0].contains("वैश्विकम्"),
        "and the declaration that promised it: {}",
        e[0]
    );
}

// ---------------------------------------------------------------------------
// `॥ संरेखः n ॥` — task `W-071`.
//
// The directive had NO test at all, which is how it shipped emitting `n` bytes
// into `ॱदत्त` while `ॱपाठ`, the section the author had open, was left alone.
// Every assertion below distinguishes "a number changed" from "the section
// actually moved", because the first is what the broken version also did.
// ---------------------------------------------------------------------------

/// Text offsets of every instruction, as the layout settles them.
fn text_at(p: &Program) -> Vec<u32> {
    sadhana::encode::layout_addresses(p, sadhana::encode::Target::Uncompressed)
}

/// Bytes in `ॱदत्त`, concatenated.
fn data_bytes(p: &Program) -> Vec<u8> {
    p.data.iter().flat_map(|d| d.bytes.clone()).collect()
}

#[test]
fn a_64_bit_constant_with_the_top_bit_set_is_the_number_written() {
    // `W-075`, and the literal is the one that found it: the FNV-1a-64 basis,
    // 0xcbf29ce484222325. It used to assemble as 0x7fffffffffffffff — every
    // 64-bit constant with the top bit set did — with no diagnostic. `E-004`
    // caught it only because that agent printed the value back; had it been
    // trusted, the digest would have been self-consistent and not FNV.
    let src = "॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टाष्टकाः ०षोड्इआऊ२९इउ४८४२२२३२५ ॥\n";
    let p = one(src);
    assert_eq!(
        data_bytes(&p),
        0xcbf2_9ce4_8422_2325u64.to_le_bytes(),
        "the octets emitted must be the number written"
    );
}

#[test]
fn ऋण_reaches_the_most_negative_64_bit_value_exactly() {
    // It used to come back one short: the magnitude was clamped to i64::MAX
    // BEFORE being negated, so the most negative number was unwritable and
    // quietly became i64::MIN + 1.
    let src = "॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टाष्टकाः ऋण९२२३३७२०३६८५४७७५८०८ ॥\n";
    let p = one(src);
    assert_eq!(data_bytes(&p), i64::MIN.to_le_bytes());
}

#[test]
fn a_numeral_too_large_is_refused_rather_than_saturated() {
    // Saturation is not the safe alternative to wrapping. Both emit a number
    // other than the one written; saturation is the more convincing of the two
    // because the result looks deliberate.
    let big = "९".repeat(30);
    let src = format!("॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टाष्टकाः {big} ॥\n");
    let e = assemble_program(&src).expect_err("past u64 must not assemble");
    let joined = e.join("\n");
    // The refusal must not say "is not a numeral" — it is one, spelled
    // correctly, and a reader sent to look for a typo will not find one.
    assert!(
        !joined.contains("is not a numeral") && !joined.contains("संख्या नास्ति"),
        "must not blame the spelling: {joined}"
    );
    assert!(
        joined.contains("१८४४६७४४०७३७०९५५१६१५"),
        "must name the bound it exceeded: {joined}"
    );
}

#[test]
fn a_count_and_a_boundary_carry_no_sign() {
    // Read as a bit pattern `ऋण५` is 0xffff_ffff_ffff_fffb, so `स्थानम्` would
    // ask to reserve the address space and `संरेखः` would see something that is
    // not a power of two. Both are refused, and for the rule actually broken:
    // a count and a boundary are magnitudes.
    for src in [
        "॥ कोष्ठकम् ॱरिक्त ॥\nखम्ॱॱ\n॥ स्थानम् ऋण५ ॥\n",
        "॥ कोष्ठकम् ॱपाठ ॥\n॥ संरेखः ऋण१६ ॥\n",
    ] {
        let e = assemble_program(src).expect_err("a signed count must not assemble");
        let joined = e.join("\n");
        assert!(
            joined.contains("ऋण"),
            "must name the sign as the fault: {joined}"
        );
        assert!(
            !joined.contains("is not a numeral") && !joined.contains("संख्या नास्ति"),
            "must not blame the spelling: {joined}"
        );
    }
}

/// Bytes in `ॱदत्त`.
fn data_len(p: &Program) -> usize {
    p.data.iter().map(|d| d.bytes.len()).sum()
}

const ONE: &str = "    योगः  अर्थ०म्  शून्यःन  ०१०न ।\n";

/// `sh_addralign` must be the boundary `संरेखः` asked for, because that is the
/// only part of the request the LINKER can see (`W-080`).
///
/// Padding inside the object is not enough. A link may place `.text` at any
/// multiple of `sh_addralign`, so an object that pads to 16 while claiming 4 has
/// its alignment silently undone the moment it is linked behind anything whose
/// size is not a multiple of 16.
///
/// Measured before this was fixed: `॥ संरेखः १६ ॥` linked after a four-byte
/// object put the label at `0x…14`, while gas's `.balign 16` put its own at
/// `0x…20` — because gas raises the section and we did not. The alignment held
/// inside the object and did not survive linking, which is why `C-001e3a` had to
/// round its arena base at runtime rather than trust the directive.
/// A value must FIT the destination, not be trimmed to it (`W-082`).
///
/// `॥ अष्टकाः ३०० ॥` used to emit `0x2c` — 300 masked to its low octet — with no
/// diagnostic. GNU `as` warns and truncates; this toolchain has no warning
/// channel, and R-02-1's stance is emit-what-was-written-or-refuse. So the rule
/// is `W-075`'s with 64 replaced by N: writable iff it fits N bits unsigned, or
/// with `ऋण` as signed.
///
/// Both directions matter. `२५५` and `ऋण१२८` are the largest values an octet
/// holds either way and must still pass; a check that only refused would be as
/// wrong as one that only truncated.
#[test]
fn a_datum_must_fit_its_width_and_is_refused_rather_than_trimmed() {
    for (operand, ok) in [
        ("२५५", true),
        ("ऋण१२८", true),
        ("०", true),
        ("३००", false),
        ("ऋण२००", false),
    ] {
        let src = format!("॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टकाः {operand} ॥\n");
        let r = assemble_program(&src);
        assert_eq!(
            r.is_ok(),
            ok,
            "`॥ अष्टकाः {operand} ॥` should {} — an octet holds 0..=255 unsigned \
             and -128..=127 with ऋण, and anything else must be refused rather \
             than masked to its low byte",
            if ok { "assemble" } else { "be refused" }
        );
        if !ok {
            let e = format!("{:?}", r.expect_err("must be refused"));
            // The rendered message carries no code — no diagnostic in this
            // crate does — so assert what P31 PROMISES instead: the width it
            // was asked to write, and the largest value that fits, quoted in
            // DECIMAL because `०षोड्` ends in a virama and SLP1 has no form for
            // a virama before a vowel (`W-075`).
            assert!(
                e.contains("W-082") && e.contains("255"),
                "must be refused by naming the width and the bound in decimal, \
                 not by some other rule: {e}"
            );
        }
    }
}

#[test]
fn संरेखः_raises_the_section_alignment_the_linker_sees() {
    let src = format!("॥ कोष्ठकम् ॱपाठ ॥\n{ONE}॥ संरेखः १६ ॥\nलक्ष्यम्ॱॱ\n{ONE}");
    let p = one(&src);
    let (text, pending) = sadhana::encode::encode_object(&p).expect("encode");
    let obj = sadhana::kosha::object(
        &text,
        &p,
        &pending,
        None,
        &sadhana::encode::layout_addresses(&p, sadhana::encode::Target::Uncompressed),
    );

    // Find `.text` BY NAME rather than by position. Header 1 is `.text` today —
    // `kosha.rs` builds its plan with `.text` first — but a test that asserts a
    // value from a section it never identifies would read a different section's
    // alignment if that plan ever gained an entry, and would then pass or fail
    // for a reason unconnected to `संरेखः` (a peer session).
    let shoff = u64::from_le_bytes(obj[0x28..0x30].try_into().unwrap()) as usize;
    let shnum = u16::from_le_bytes(obj[0x3c..0x3e].try_into().unwrap()) as usize;
    let shstrndx = u16::from_le_bytes(obj[0x3e..0x40].try_into().unwrap()) as usize;
    let strtab = u64::from_le_bytes(
        obj[shoff + shstrndx * 64 + 24..shoff + shstrndx * 64 + 32]
            .try_into()
            .unwrap(),
    ) as usize;
    let name_of = |e: usize| {
        let off = u32::from_le_bytes(obj[e..e + 4].try_into().unwrap()) as usize;
        let start = strtab + off;
        let end = obj[start..].iter().position(|b| *b == 0).unwrap() + start;
        core::str::from_utf8(&obj[start..end]).unwrap().to_owned()
    };
    let entry = (0..shnum)
        .map(|i| shoff + i * 64)
        .find(|e| name_of(*e) == ".text")
        .expect("no .text section header");
    let align = u64::from_le_bytes(obj[entry + 48..entry + 56].try_into().unwrap());
    assert_eq!(
        align, 16,
        "`॥ संरेखः १६ ॥` must raise .text's sh_addralign to 16; it is {align}, so a \
         linker may place this section anywhere on a {align}-byte boundary and the \
         padding inside the object proves nothing"
    );
}

#[test]
fn संरेखः_moves_the_text_section_and_not_the_data_one() {
    let src = format!("॥ कोष्ठकम् ॱपाठ ॥\n{ONE}॥ संरेखः १६ ॥\nलक्ष्यम्ॱॱ\n{ONE}");
    let p = one(&src);

    // The label names the instruction AFTER the padding, and that address is
    // what alignment was asked for. Reading `p.labels` rather than an offset
    // computed here is the point: a label that still named the pre-padding
    // address is exactly the bug, and it looked correct in the byte count.
    let at = text_at(&p);
    let label = p.labels.iter().find(|l| l.name == "लक्ष्यम्").expect("label");
    assert_eq!(label.section, Section::Text);
    let addr = at[label.at];
    assert_eq!(
        addr % 16,
        0,
        "लक्ष्यम् at {addr:#x} is not on a 16-byte boundary"
    );
    assert_eq!(addr, 16, "one 4-byte instruction, then padding to 16");

    // And `ॱदत्त` did NOT grow. This is the assertion that fails on the
    // version this task fixed: it put sixteen bytes here.
    assert_eq!(data_len(&p), 0, "संरेखः must not write to ॱदत्त");
}

#[test]
fn संरेखः_at_a_boundary_emits_nothing() {
    // `n` is a BOUNDARY, not a count. `स्थानम् १६` always writes sixteen
    // bytes; `संरेखः १६` at an address already divisible by sixteen writes
    // none. Sharing one arm made the two indistinguishable.
    let aligned = one(&format!("॥ कोष्ठकम् ॱपाठ ॥\n॥ संरेखः १६ ॥\n{ONE}"));
    assert_eq!(text_at(&aligned)[0], 0, "address 0 already divides by 16");
    assert_eq!(data_len(&aligned), 0);
}

#[test]
fn संरेखः_holds_at_two_different_link_addresses() {
    // Alignment is a property of the address, so it has to survive the image
    // being placed somewhere else. A padding count that was merely right once
    // would pass the first assertion above and fail here.
    let src = format!("॥ कोष्ठकम् ॱपाठ ॥\n{ONE}॥ संरेखः ३२ ॥\nलक्ष्यम्ॱॱ\n{ONE}");
    let p = one(&src);
    let at = text_at(&p);
    let label = p.labels.iter().find(|l| l.name == "लक्ष्यम्").expect("label");
    for base in [0x8000_0000u64, 0x8020_0000u64] {
        let absolute = base + u64::from(at[label.at]);
        assert_eq!(
            absolute % 32,
            0,
            "at base {base:#x} the label is {absolute:#x}"
        );
    }
}

#[test]
fn a_data_directive_inside_ॱपाठ_is_refused() {
    // It used to place the bytes in `ॱदत्त` and leave both surrounding labels
    // naming the next INSTRUCTION, so they compared equal and reading through
    // one returned an opcode byte. Refused is not the same as supported, and
    // the diagnostic says which section to use.
    let src = format!("॥ कोष्ठकम् ॱपाठ ॥\n{ONE}॥ अष्टकाः २०१ २०२ ॥\n");
    let e = assemble_program(&src).expect_err("data in ॱपाठ must not assemble");
    // The diagnostic has to name BOTH the section that cannot hold it and the
    // one that can. "refused" without a destination sends the reader looking.
    let joined = e.join("\n");
    assert!(
        joined.contains("अष्टकाः"),
        "must name the directive: {joined}"
    );
    assert!(
        joined.contains("ॱपाठ"),
        "must name the section refused: {joined}"
    );
    assert!(
        joined.contains("ॱदत्त"),
        "must name where it belongs: {joined}"
    );
}

#[test]
fn संरेखः_aligns_ॱदत्त_too() {
    // The directive acts on whichever section is open, which is the whole
    // correction. Two octets, then a boundary of eight, is six of padding.
    let src = "॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टकाः २०१ २०२ ॥\n॥ संरेखः ८ ॥\n॥ अष्टकाः २०३ ॥\n";
    let p = one(src);
    assert_eq!(data_len(&p), 9, "2 + 6 padding + 1");
}

#[test]
fn संरेखः_refuses_what_is_not_a_boundary() {
    // `१` is satisfied by every address, so it reads like a statement that
    // does something and does nothing. `०` asks for a multiple of nothing.
    // A non-power-of-two is not an alignment either.
    for n in ["०", "१", "६"] {
        let src = format!("॥ कोष्ठकम् ॱपाठ ॥\n॥ संरेखः {n} ॥\n{ONE}");
        assert!(
            assemble_program(&src).is_err(),
            "संरेखः {n} should be refused"
        );
    }
}

#[test]
fn a_label_after_a_compressed_instruction_has_its_real_address() {
    // Not alignment, but the same defect: `kosha` computed a text label as
    // `index * 4`, so every label standing after a two-byte instruction was
    // reported four bytes further along than it is. Independent of `संरेखः`
    // and true before this task touched anything.
    let src = format!("॥ कोष्ठकम् ॱपाठ ॥\n{ONE}लक्ष्यम्ॱॱ\n{ONE}");
    let p = one(&src);
    let at = sadhana::encode::layout_addresses(&p, sadhana::encode::Target::Compressed);
    let label = p.labels.iter().find(|l| l.name == "लक्ष्यम्").expect("label");
    assert_eq!(
        at[label.at], at[1],
        "the label must name instruction 1 wherever layout put it"
    );
}

// ---------------------------------------------------------------------------
// `॥ संरेखः n ॥` inside `ॱरिक्त` — task `B-108`.
//
// Refused until now by `B-068`'s rule that only `स्थानम्` may appear in a
// section holding no octets. The refusal was inherited, not reasoned: `संरेखः`
// shared an arm with the directives that emit values and was caught by a rule
// about emitting values. It emits none — it advances the location counter,
// which is the only thing `स्थानम्` does there either.
// ---------------------------------------------------------------------------

#[test]
fn संरेखः_advances_the_counter_in_ॱरिक्त() {
    // One octet reserved, then a 4096 boundary: the region grows to the
    // boundary and a label after it names an aligned address. This is
    // `C-001e3a`'s arena, which had to round its base up at runtime instead.
    let p = one("॥ कोष्ठकम् ॱरिक्त ॥\n॥ स्थानम् १ ॥\n॥ संरेखः ४०९६ ॥\nक्षेत्रम्ॱॱ\n॥ स्थानम् ८ ॥\n");
    let label = p.labels.iter().find(|l| l.name == "क्षेत्रम्").expect("label");
    assert_eq!(label.section, Section::Bss);
    assert_eq!(
        label.data_offset % 4096,
        0,
        "क्षेत्रम् at {} is not on a 4096 boundary",
        label.data_offset
    );
    assert_eq!(label.data_offset, 4096, "one octet, then padding to 4096");
    assert_eq!(p.bss, 4096 + 8, "the boundary, then the eight reserved");
}

#[test]
fn aligning_an_empty_ॱरिक्त_reserves_nothing() {
    // Zero is already a multiple of every boundary. A no-op by arithmetic
    // rather than by a special case, and it must not invent a region.
    let p = one("॥ कोष्ठकम् ॱरिक्त ॥\n॥ संरेखः ४०९६ ॥\n");
    assert_eq!(p.bss, 0, "nothing was reserved, so nothing is aligned");
    assert_eq!(p.bss_line, 0, "and no line is blamed for a reservation");
}

#[test]
fn ॱरिक्त_still_refuses_directives_that_write_values() {
    // B-108 does not widen the section. The rule P20 states is about EMITTING,
    // and it still holds for everything that emits — what changed is that
    // `संरेखः` was never one of those.
    for d in ["॥ अष्टकाः २०१ ॥", "॥ चतुरष्टकाः ७ ॥", "॥ अष्टाष्टकाः १ ॥"]
    {
        let src = format!("॥ कोष्ठकम् ॱरिक्त ॥\n{d}\n");
        let e = assemble_program(&src).expect_err("{d} writes values into ॱरिक्त");
        assert!(e[0].contains("ॱरिक्त"), "must name the section: {}", e[0]);
    }
}

#[test]
fn ॱरिक्त_reserves_nothing_in_the_file() {
    // The point of the section, and the reason alignment here is a counter and
    // not padding: `ॱरिक्त` costs address space, never file bytes. If aligning
    // it had emitted octets, they would have to live somewhere.
    let p = one("॥ कोष्ठकम् ॱरिक्त ॥\n॥ स्थानम् १ ॥\n॥ संरेखः ६४ ॥\n");
    assert_eq!(p.bss, 64);
    assert!(p.data.is_empty(), "alignment in ॱरिक्त emits no data");
}
