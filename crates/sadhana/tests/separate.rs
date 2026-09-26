//! Separate compilation, end to end — task `B-069d2a`, doc 03 §3.3.
//!
//! Two files assembled **independently** into objects, neither knowing the
//! other's addresses, linked by `संयोजनम्` with no source in sight. That is
//! what `B-014` promised and what `बन्धकः` did not do: it re-encoded every unit
//! from its parse tree, which needs the source at link time. `B-096` routed
//! every build through this path and `B-096b` deleted the other one.
//!
//! The check is not that it links. It is that the call lands on the callee's
//! **final** address — which neither file could have known, because the callee
//! moved when the caller was placed in front of it.

use sadhana::encode::encode_object;
use sadhana::kosha::{LOAD_ADDRESS, object};
use sadhana::parse::assemble_program;
use sadhana::samyojana::link;
use sadhana::vastu::read;

/// Assemble one source to object bytes, as `sadhana --वस्तु` does.
fn compile(src: &str) -> Vec<u8> {
    let program = assemble_program(src).unwrap_or_else(|e| panic!("{src}: {e:?}"));
    let (text, pending) = encode_object(&program).unwrap_or_else(|e| panic!("{src}: {e:?}"));
    object(
        &text,
        &program,
        &pending,
        None,
        &sadhana::encode::layout_addresses(&program, sadhana::encode::Target::Uncompressed),
    )
}

/// Calls `कार्यम्`, which it does not define, then spins.
const CALLER: &str = "॥ वैश्विकम् मुख्यम् ॥
मुख्यम्ॱॱ
लङ्घनम् पुनःस्थानम्म् कार्यम्य् ।
चक्रःॱॱ
लङ्घनम् शून्यःम् चक्रःय् ।
";

/// Defines `कार्यम्` and returns. Two instructions, so the caller's own
/// addresses and the callee's differ by something a single file could not guess.
const CALLEE: &str = "॥ वैश्विकम् कार्यम् ॥
कार्यम्ॱॱ
योगः अर्थ०म् शून्यःन ७न ।
सापेक्षलङ्घनम् शून्यःम् पुनःस्थानम्त् ०न ।
";

#[test]
fn two_files_compiled_apart_link_into_one_program() {
    let a = compile(CALLER);
    let b = compile(CALLEE);

    // Round-tripped through the file format, exactly as a build would: written
    // to bytes, read back, linked. Nothing here holds a parse tree.
    let objects = [
        read(&a).expect("caller reads"),
        read(&b).expect("callee reads"),
    ];
    let linked = link(&objects).unwrap_or_else(|e| panic!("{e:?}"));

    assert_eq!(linked.text.len(), 8 + 8, "two instructions from each file");

    // Where the linker put the callee — it is after the caller, so its address
    // is one neither file could have known.
    let target = *linked
        .symbols
        .get("कार्यम्")
        .expect("the callee is in the symbol table");
    assert_eq!(target, LOAD_ADDRESS + 8, "the callee follows the caller");

    // And the call reaches it. Decoded rather than compared to a constant: a
    // `jal`'s displacement is scattered across four discontiguous runs, and a
    // hand-computed expectation is the mistake `B-042` and `B-055` both made.
    let word = u32::from_le_bytes(linked.text[..4].try_into().expect("4 bytes"));
    let d = sadhana::vishlesana::decode(word).expect("the call decodes");
    assert_eq!(d.insn, "jal");
    let disp = d
        .operands
        .iter()
        .find(|(k, _)| k == "disp")
        .map(|(_, v)| *v)
        .expect("a jal has a displacement");
    assert_eq!(
        LOAD_ADDRESS as i64 + disp,
        target as i64,
        "the call must land on the callee, not near it"
    );
}

#[test]
fn a_name_no_object_defines_is_named() {
    // The caller alone. Linking it must fail, and say which name is missing —
    // an unresolved symbol reported as a link that succeeded is a program that
    // jumps to zero.
    let a = compile(CALLER);
    let e = link(&[read(&a).expect("reads")]).expect_err("must not link");
    assert!(
        e.iter().any(|m| m.contains("कार्यम्")),
        "the missing name is not reported: {e:?}"
    );
}

#[test]
fn the_order_of_the_objects_changes_the_addresses_and_not_the_answer() {
    // Linking B then A puts the callee first. The call must still land on it,
    // which is the whole difference between resolving a symbol and remembering
    // an offset.
    let a = compile(CALLER);
    let b = compile(CALLEE);
    let swapped = [read(&b).expect("reads"), read(&a).expect("reads")];
    let linked = link(&swapped).unwrap_or_else(|e| panic!("{e:?}"));

    let target = *linked.symbols.get("कार्यम्").expect("callee");
    assert_eq!(target, LOAD_ADDRESS, "the callee is first now");

    // The caller's `jal` is now the third instruction, at offset 8.
    let word = u32::from_le_bytes(linked.text[8..12].try_into().expect("4 bytes"));
    let d = sadhana::vishlesana::decode(word).expect("decodes");
    let disp = d
        .operands
        .iter()
        .find(|(k, _)| k == "disp")
        .map(|(_, v)| *v)
        .expect("displacement");
    assert_eq!(
        LOAD_ADDRESS as i64 + 8 + disp,
        target as i64,
        "the call must follow the callee wherever it went"
    );
    assert!(disp < 0, "and here it points backwards");
}

#[test]
fn a_data_reference_survives_another_file_being_linked_in_front_of_it() {
    // `B-069d2b`, and the defect it fixed. `namaste-main.sas` loads the address
    // of `सन्देशः`, which sits in its OWN `ॱदत्त` — so the reference looks
    // resolvable while assembling, and is not: every object's `.text` is laid
    // end to end and `.data` follows all of it, so adding `lib-mudraka` in
    // front pushes the data 32 bytes further away.
    //
    // Resolved at assembly time it pointed 32 bytes short — a program that
    // links, runs, and prints nothing.
    let main = compile(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .expect("root")
                .join("spec/namaste-main.sas"),
        )
        .expect("read namaste-main.sas"),
    );
    let lib = compile(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .expect("root")
                .join("spec/lib-mudraka.sas"),
        )
        .expect("read lib-mudraka.sas"),
    );

    let objects = [read(&main).expect("main"), read(&lib).expect("lib")];
    let linked = link(&objects).unwrap_or_else(|e| panic!("{e:?}"));

    let message = *linked.symbols.get("सन्देशः").expect("the string is a symbol");

    // The `auipc`/`addi` pair at the top of main computes that address.
    let hi = u32::from_le_bytes(linked.text[..4].try_into().expect("4 bytes"));
    let lo = u32::from_le_bytes(linked.text[4..8].try_into().expect("4 bytes"));
    let hi = sadhana::vishlesana::decode(hi).expect("auipc decodes");
    let lo = sadhana::vishlesana::decode(lo).expect("addi decodes");
    assert_eq!(hi.insn, "auipc");
    assert_eq!(lo.insn, "addi");

    let upper = hi
        .operands
        .iter()
        .find(|(k, _)| k == "imm" || k == "simm")
        .map(|(_, v)| *v)
        .expect("auipc immediate");
    let lower = lo
        .operands
        .iter()
        .find(|(k, _)| k == "imm" || k == "simm")
        .map(|(_, v)| *v)
        .expect("addi immediate");

    // What the machine computes: pc of the auipc, plus the two halves.
    let computed = LOAD_ADDRESS as i64 + (upper << 12) + lower;
    assert_eq!(
        computed, message as i64,
        "the pair must reach the string after linking, not before"
    );
}

#[test]
fn a_label_that_is_not_first_keeps_its_offset() {
    // Every symbol in an object carried value 0 until `B-069d2b`, which put
    // each one at its object's base. That is right for a label at the start and
    // wrong for every other, and both test programs happened to have theirs
    // first — so nothing caught it.
    let b = compile(CALLEE);
    let o = read(&b).expect("reads");
    let entry = o
        .symbols
        .iter()
        .find(|s| s.name == "कार्यम्")
        .expect("the callee is named");
    assert_eq!(entry.value, 0, "this one really is first");

    // A file whose label is not first must say so.
    let two = compile("योगः अर्थ०म् शून्यःन १न ।\n॥ वैश्विकम् दूसरा ॥\nदूसराॱॱ\nयोगः अर्थ०म् शून्यःन २न ।\n");
    let o = read(&two).expect("reads");
    let second = o.symbols.iter().find(|s| s.name == "दूसरा").expect("named");
    assert_eq!(second.value, 4, "one instruction in, so four bytes in");
}

#[test]
fn a_linked_image_carries_the_names_it_was_built_from() {
    // `B-103`. `--संयोजय` wrote text and data and dropped every symbol, so `nm`
    // on an image linked from objects reported nothing while a single-file
    // build of the same program showed `मुख्यम्`. An object format nobody else
    // can read cannot be checked against anyone else's tools (`B-063`), and
    // that argument does not stop at the object.
    let a = compile(CALLER);
    let b = compile(CALLEE);
    let linked = link(&[read(&a).expect("caller"), read(&b).expect("callee")]).expect("links");

    assert_eq!(
        linked.table.len(),
        3,
        "मुख्यम् and चक्रः from the caller, कार्यम् from the callee: {:?}",
        linked.table.iter().map(|s| &s.name).collect::<Vec<_>>()
    );
    let named = |n: &str| linked.table.iter().find(|s| s.name == n).expect(n);
    // Each carries its own address and its own visibility, or the table is a
    // list of names rather than a symbol table.
    assert_eq!(named("मुख्यम्").value, *linked.symbols.get("मुख्यम्").unwrap());
    assert!(named("कार्यम्").global, "it is exported");
    assert!(!named("चक्रः").global, "it is not");
    assert_eq!(
        named("मुख्यम्").section,
        sadhana::kosha::SymSection::Text,
        "a label in ॱपाठ"
    );
}

#[test]
fn two_objects_may_each_have_their_own_local_name() {
    // `B-014` decided a label is local to its file unless exported, precisely
    // so two files may each have a `चक्रः`. `बन्धकः` linked such a pair without
    // complaint; the object linker refused it, because its duplicate check did
    // not ask whether the name was global (`B-103`).
    let one = compile("॥ वैश्विकम् एकः ॥\nएकःॱॱ\nचक्रःॱॱ\nलङ्घनम् शून्यःम् चक्रःय् ।\n");
    let two = compile("॥ वैश्विकम् द्वौ ॥\nद्वौॱॱ\nचक्रःॱॱ\nलङ्घनम् शून्यःम् चक्रःय् ।\n");
    let linked =
        link(&[read(&one).expect("one"), read(&two).expect("two")]).expect("two locals may repeat");

    // Both are in the table, at their own addresses.
    let loops: Vec<u64> = linked
        .table
        .iter()
        .filter(|s| s.name == "चक्रः")
        .map(|s| s.value)
        .collect();
    assert_eq!(loops.len(), 2, "one per file");
    assert_ne!(loops[0], loops[1], "and they are different places");

    // And each file's jump reaches ITS OWN loop, not the other file's — which
    // is the whole of what shadowing means.
    for (at, want) in [(0usize, loops[0]), (4, loops[1])] {
        let word = u32::from_le_bytes(linked.text[at..at + 4].try_into().expect("4 bytes"));
        let d = sadhana::vishlesana::decode(word).expect("decodes");
        let disp = d
            .operands
            .iter()
            .find(|(k, _)| k == "disp")
            .map(|(_, v)| *v)
            .expect("a jal has a displacement");
        assert_eq!(LOAD_ADDRESS as i64 + at as i64 + disp, want as i64);
    }

    // A GLOBAL defined twice is still refused: silently taking one makes which
    // file was listed first into program behaviour.
    let e = link(&[read(&one).expect("one"), read(&one).expect("again")])
        .expect_err("two exports of एकः");
    assert!(e.iter().any(|m| m.contains("एकः")), "{e:?}");
}

#[test]
fn a_line_table_survives_the_link() {
    // `B-104`. `-g` reached the object in `B-100b` and stopped there:
    // `--संयोजय` wrote text and data and dropped `.debug_line` with it, so a
    // program linked from objects had no line table at all.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("root")
            .join("spec/lib-mudraka.sas"),
    )
    .expect("read");
    let program = assemble_program(&source).expect("parses");
    let (text, pending) = encode_object(&program).expect("encodes");
    let bytes = sadhana::kosha::object(
        &text,
        &program,
        &pending,
        Some("spec/lib-mudraka.sas"),
        &sadhana::encode::layout_addresses(&program, sadhana::encode::Target::Uncompressed),
    );

    let linked = link(&[read(&bytes).expect("reads")]).expect("links");
    let line = linked
        .debug
        .iter()
        .find(|(n, _)| n == ".debug_line")
        .expect("the line table travelled");

    // And its address was fixed rather than carried through as the zero an
    // object holds. Reading the whole section for the load address is enough:
    // an unrelocated table has no eight bytes that spell it.
    assert!(
        line.1
            .windows(8)
            .any(|w| u64::from_le_bytes(w.try_into().expect("8")) == LOAD_ADDRESS),
        "the set_address operand was not relocated"
    );

    // And two of them merge (`B-104b`), with the second unit's line program
    // moved and its `DW_AT_stmt_list` moved with it.
    // A second file with debug of its own. Not the same object twice: that
    // exports `मुद्रकः` twice and the linker is right to refuse it.
    let other = assemble_program("दूसराॱॱ\nयोगः अर्थ०म् शून्यःन ७न ।\n").expect("parses");
    let (other_text, other_pending) = encode_object(&other).expect("encodes");
    let other = sadhana::kosha::object(
        &other_text,
        &other,
        &other_pending,
        Some("दूसरा.sas"),
        &sadhana::encode::layout_addresses(&other, sadhana::encode::Target::Uncompressed),
    );

    let two =
        link(&[read(&bytes).expect("one"), read(&other).expect("two")]).expect("two units merge");
    let merged = two
        .debug
        .iter()
        .find(|(n, _)| n == ".debug_line")
        .expect("a line table");
    assert!(
        merged.1.len() > line.1.len(),
        "the two programs are concatenated"
    );

    // The second unit's stmt_list is where its program actually starts. This
    // is the field the merge exists for: leave it at zero and every line in
    // the second file reports against the first file's table.
    let info = two
        .debug
        .iter()
        .find(|(n, _)| n == ".debug_info")
        .expect("units");
    // The first unit's length is its own `unit_length` plus the four bytes
    // that field occupies, which is where the second begins.
    let first_len = u32::from_le_bytes(info.1[..4].try_into().expect("4 bytes")) as usize + 4;
    let stmt_at = sadhana::dwarf::compile_unit_parts("दूसरा.sas", 0, 0).stmt_list_at;
    let second = u32::from_le_bytes(
        info.1[first_len + stmt_at..first_len + stmt_at + 4]
            .try_into()
            .expect("4 bytes"),
    );
    assert_eq!(
        second as usize,
        line.1.len(),
        "the second unit points at the first unit's line program"
    );

    // `.debug_abbrev` is taken once: every unit points at offset 0, so a
    // second copy would be bytes nobody reads.
    let abbrev = two
        .debug
        .iter()
        .find(|(n, _)| n == ".debug_abbrev")
        .expect("abbrev");
    assert_eq!(
        abbrev.1,
        linked
            .debug
            .iter()
            .find(|(n, _)| n == ".debug_abbrev")
            .expect("one")
            .1,
        "the shared table was appended rather than shared"
    );
}

#[test]
fn a_local_name_is_not_visible_to_another_object() {
    // `B-014` decided a label is local to its file unless exported, and the
    // half of that rule worth testing is the refusal: without it every name in
    // every file would collide with every other and many files would be
    // impossible. Ported from `बन्धकः`, which is gone (`B-096b`) — the rule is
    // the language's, not one linker's.
    let library = compile("गुप्तःॱॱ\nयोगः अर्थ०म् शून्यःन ७न ।\n");
    let caller = compile("॥ वैश्विकम् मुख्यम् ॥\nमुख्यम्ॱॱ\nलङ्घनम् पुनःस्थानम्म् गुप्तःय् ।\n");

    let e = link(&[
        read(&caller).expect("caller"),
        read(&library).expect("library"),
    ])
    .expect_err("a local is not reachable from another file");
    assert!(e.iter().any(|m| m.contains("गुप्तः")), "{e:?}");

    // Exported, the same program links — so what fails is the visibility and
    // not the reference.
    let exported = compile("॥ वैश्विकम् गुप्तः ॥\nगुप्तःॱॱ\nयोगः अर्थ०म् शून्यःन ७न ।\n");
    link(&[
        read(&caller).expect("caller"),
        read(&exported).expect("library"),
    ])
    .expect("now it is visible");
}

#[test]
fn a_cross_file_branch_stays_wide_at_the_compressed_target() {
    // `बन्धकः` iterated the relaxation fixpoint across every unit together, so
    // a branch in one file to a label in another could compress. Objects are
    // compressed one at a time and nothing relaxes afterwards (`B-069e`), so it
    // cannot — larger, never wrong, and the alternative is a linker that
    // relaxes.
    //
    // Pinned because it is a real difference `B-096a` introduced, and a
    // difference nobody wrote down is one the next reader has to rediscover.
    use sadhana::encode::{Target, encode_object_for};
    let program =
        assemble_program("॥ वैश्विकम् मुख्यम् ॥\nमुख्यम्ॱॱ\nलङ्घनम् शून्यःम् बाह्यम्य् ।\n").expect("parses");
    let (text, pending) = encode_object_for(&program, Target::Compressed).expect("encodes");
    assert_eq!(
        text.len(),
        4,
        "a jump whose target this file does not define cannot be chosen short"
    );
    assert_eq!(pending.len(), 1, "and it leaves a record naming four bytes");

    // And the file is compressing: an instruction beside it that CAN be chosen
    // short is, so the four bytes above are this instruction's and not the
    // assembler declining to compress at all.
    let mixed = assemble_program(
        "॥ वैश्विकम् मुख्यम् ॥\nमुख्यम्ॱॱ\nयोगः अर्थ०म् शून्यःन ५न ।\n\
         लङ्घनम् शून्यःम् बाह्यम्य् ।\n",
    )
    .expect("parses");
    let (both, _) = encode_object_for(&mixed, Target::Compressed).expect("encodes");
    assert_eq!(
        both.len(),
        2 + 4,
        "the `c.li` shrank and the instruction carrying a record did not"
    );
}
