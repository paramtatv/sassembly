//! An image can be built somewhere other than `0x8000_0000` — task `C-001a1`.
//!
//! Every image this toolchain has ever written was placed at the QEMU `virt`
//! reset vector, and the address was a `const`. Stage C boots under **OpenSBI**
//! (doc 11 §7.1.1), and OpenSBI is itself loaded at `0x8000_0000` — so a kernel
//! that boots under firmware cannot be at the address every image used, and
//! nothing could be built for one until the address became a parameter.
//!
//! The property worth asserting is not that the headers change. It is that the
//! **text does not**: `ॱउपरि`/`ॱअधः` measure from the instruction carrying them
//! (`B-064`), so a program's bytes are the same wherever it is placed. If that
//! were false, moving an image would silently produce one that runs and reads
//! the wrong memory — the failure `B-064` describes, which does not crash.

use sadhana::encode::encode_object;
use sadhana::kosha::{LOAD_ADDRESS, object, write_debuggable, write_debuggable_at};
use sadhana::parse::assemble_program;
use sadhana::samyojana::{Linked, link, link_at};
use sadhana::vastu::read;

/// Where OpenSBI hands control to an S-mode payload on the `virt` machine.
/// 2 MiB above the firmware, which is where the firmware itself is.
const SBI_PAYLOAD: u64 = 0x8020_0000;

/// The speaking program: a label in `.text`, a string in `ॱदत्त`, and its
/// address taken pc-relatively. Everything that could depend on the load
/// address is in it.
const NAMASTE: &str = include_str!("../../../spec/namaste.sas");

fn compile(src: &str) -> Vec<u8> {
    let program = assemble_program(src).unwrap_or_else(|e| panic!("{e:?}"));
    let (text, pending) = encode_object(&program).unwrap_or_else(|e| panic!("{e:?}"));
    object(
        &text,
        &program,
        &pending,
        None,
        &sadhana::encode::layout_addresses(&program, sadhana::encode::Target::Uncompressed),
    )
}

fn linked_at(src: &str, load: u64) -> Linked {
    let bytes = compile(src);
    let objects = [read(&bytes).expect("the object reads back")];
    link_at(&objects, load).unwrap_or_else(|e| panic!("{e:?}"))
}

/// `e_entry`, and the segment's `p_vaddr` and `p_paddr`, read out of the file.
fn addresses(elf: &[u8]) -> (u64, u64, u64) {
    let word = |at: usize| u64::from_le_bytes(elf[at..at + 8].try_into().expect("8 bytes"));
    let phoff = word(32) as usize;
    (word(24), word(phoff + 16), word(phoff + 24))
}

#[test]
fn an_image_carries_the_address_it_was_built_for() {
    let image = linked_at(NAMASTE, SBI_PAYLOAD);
    let elf = write_debuggable_at(
        &image.text,
        &image.data,
        &image.table,
        image.bss,
        &[],
        SBI_PAYLOAD,
    );
    assert_eq!(
        addresses(&elf),
        (SBI_PAYLOAD, SBI_PAYLOAD, SBI_PAYLOAD),
        "e_entry, p_vaddr and p_paddr all name where the image was placed"
    );

    // And the section headers agree, which is what `readelf` and `objdump`
    // read. A file whose program header says one thing and whose sections say
    // another disassembles at the wrong addresses without failing.
    assert!(
        elf.windows(8).any(|w| w == SBI_PAYLOAD.to_le_bytes()),
        "the address appears in the file"
    );
    assert!(
        !elf.windows(8).any(|w| w == LOAD_ADDRESS.to_le_bytes()),
        "and the default does not appear anywhere in it"
    );
}

#[test]
fn moving_an_image_changes_no_instruction() {
    let here = linked_at(NAMASTE, LOAD_ADDRESS);
    let there = linked_at(NAMASTE, SBI_PAYLOAD);

    assert!(!here.text.is_empty(), "there is a program to compare");
    assert_eq!(
        here.text, there.text,
        "the encoding is pc-relative, so placing it elsewhere re-encodes nothing"
    );
    assert_eq!(here.data, there.data, "and the data is just bytes");
}

#[test]
fn every_name_moves_by_exactly_the_distance() {
    let here = linked_at(NAMASTE, LOAD_ADDRESS);
    let there = linked_at(NAMASTE, SBI_PAYLOAD);
    let delta = SBI_PAYLOAD - LOAD_ADDRESS;

    assert!(here.symbols.len() >= 3, "there are names to check");
    for (name, at) in &here.symbols {
        let moved = there.symbols.get(name).expect("the same names, both times");
        assert_eq!(
            *moved,
            at + delta,
            "`{name}` is {at:#x} at the default address and {moved:#x} at {SBI_PAYLOAD:#x}"
        );
    }
}

/// The default is still the default, byte for byte.
///
/// A parameter with a default is where a refactor silently changes every
/// existing artifact: `tools/check-reproducible.sh` compares one build against
/// another, not against the previous commit, so it would not have noticed.
#[test]
fn the_old_entry_points_still_build_the_same_file() {
    let image =
        link(&[read(&compile(NAMASTE)).expect("reads")]).unwrap_or_else(|e| panic!("{e:?}"));
    let by_default = write_debuggable(&image.text, &image.data, &image.table, image.bss, &[]);
    let by_hand = write_debuggable_at(
        &image.text,
        &image.data,
        &image.table,
        image.bss,
        &[],
        LOAD_ADDRESS,
    );
    assert_eq!(by_default, by_hand);
    assert_eq!(addresses(&by_default).0, LOAD_ADDRESS);
}
