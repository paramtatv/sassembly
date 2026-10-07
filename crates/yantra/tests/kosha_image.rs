//! **`kosha.t1` WRITES AN IMAGE `yantra` LOADS.** Phase 2 of the ELF writer's
//! port, and the only phase whose failure cannot be masked by the rest of the
//! chain working: the segment list here is HAND-BUILT, so nothing upstream of
//! the writer is involved and nothing downstream is required.
//!
//! WHY THIS IS THE WHOLE RISK. `crates/sadhana-t1/src/kosha.t1` had ZERO
//! routines — the module was the symbol model and nothing else — so no `.t1`
//! could produce an image at all, however much of the chain was ported. The
//! census line "three declare no routine — there is nothing in any of them to
//! build" was true and covered this.
//!
//! WHAT THIS DOES NOT ASSERT: that the image RUNS. Running needs machine code,
//! and the code here is a placeholder word — the claim is that the header and
//! the program header are what a loader accepts, which is what phase 2 owes.
//! Phase 5 runs a real program through the whole path.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::Machine;

const LOAD: u64 = 0x8000_0000;
/// The limits `yantra-run` gives a program, imported rather than restated.
const RAM: usize = yantra::DEFAULT_RAM;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sadhana-t1/src")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

/// `कोश` and the octet arena it writes into — the whole of what the writer needs.
fn interpreter() -> Interpreter {
    let names = ["ashtaka.t1", "kosha.t1"];
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("{names:?} load: {e:?}"))
}

/// Write an image through `कोशॱप्रतिबिम्बलेखनम्` and hand back its octets.
fn write_image(text: &[u8], data: &[u8], bss: u64) -> Vec<u8> {
    let mut it = interpreter();
    let v = it
        .call(
            "कोशॱप्रतिबिम्बलेखनम्",
            vec![
                octets(text),
                octets(data),
                Value::Int(i128::from(bss)),
                Value::Int(i128::from(LOAD)),
            ],
            200_000_000,
        )
        .unwrap_or_else(|e| panic!("प्रतिबिम्बलेखनम् runs: {e}"));
    match v.octets() {
        Some(o) => o.as_slice().to_vec(),
        None => panic!("the writer returns a run of octets, not {v:?}"),
    }
}

/// **THE ACCEPTANCE.** `.t1` writes octets that `Machine::load_elf` accepts.
#[test]
fn kosha_t1_writes_an_image_yantra_loads() {
    // Four octets standing in for code. The writer does not read them and the
    // loader does not execute them; using a real program here would make this
    // test depend on the assembler, which is the thing phase 2 must not need.
    let text = [0x13u8, 0x00, 0x00, 0x00];
    let image = write_image(&text, &[], 0);

    assert_eq!(
        &image[0..4],
        b"\x7fELF",
        "the magic:\n{:02x?}",
        &image[..16]
    );
    assert_eq!(image[4], 2, "ELFCLASS64");
    assert_eq!(image[5], 1, "ELFDATA2LSB");
    assert_eq!(
        u16::from_le_bytes([image[18], image[19]]),
        243,
        "e_machine must be RISC-V or the loader refuses by name"
    );
    assert_eq!(
        image.len(),
        64 + 56 + text.len(),
        "header, one program header, then the text:\n{:02x?}",
        image
    );

    // THE ASSERTION THAT MATTERS: the loader itself, not a re-reading of the
    // fields by this test. A header this test agrees with and the loader
    // refuses would be two transcriptions agreeing with each other.
    let m = Machine::load_elf(&image, RAM);
    assert!(
        m.is_ok(),
        "yantra refused an image kosha.t1 wrote: {:?}\n{:02x?}",
        m.err(),
        image
    );
    println!("METRIC kosha_t1_image_octets {}", image.len());
}

/// Data is its own `PT_LOAD` (`W-363`): packed eight-aligned after the text IN
/// THE FILE, at the page after the text IN MEMORY, `PF_R | PF_W`; and a `.bss`
/// reservation shows up in its `p_memsz` without lengthening the file.
#[test]
fn data_is_its_own_writable_segment_and_bss_costs_no_file_bytes() {
    let text = [0x13u8, 0x00, 0x00, 0x00, 0x13]; // five octets — deliberately unaligned
    let data = [0xAAu8, 0xBB];
    let with_bss = write_image(&text, &data, 4096);
    let without = write_image(&text, &data, 0);

    assert_eq!(
        with_bss.len(),
        without.len(),
        "a bss reservation is space the file does not carry"
    );
    let u64_at = |img: &[u8], at: usize| u64::from_le_bytes(img[at..at + 8].try_into().unwrap());
    let u32_at = |img: &[u8], at: usize| u32::from_le_bytes(img[at..at + 4].try_into().unwrap());
    assert_eq!(
        u16::from_le_bytes([with_bss[56], with_bss[57]]),
        2,
        "e_phnum: the text and the data"
    );
    // text begins at 64 + 2 x 56 = 176; five octets end at 181; data at 184.
    assert_eq!(
        with_bss.len(),
        184 + data.len(),
        "data begins eight-aligned after the text:\n{:02x?}",
        with_bss
    );
    // The text's header: R E, the text only.
    let (t, d) = (64usize, 64usize + 56);
    assert_eq!(u32_at(&with_bss, t + 4), 0b101, "the text is PF_R | PF_X");
    assert_eq!(u64_at(&with_bss, t + 8), 176, "text p_offset");
    assert_eq!(u64_at(&with_bss, t + 16), LOAD, "text p_vaddr");
    assert_eq!(u64_at(&with_bss, t + 32), 5, "text p_filesz");
    assert_eq!(u64_at(&with_bss, t + 40), 5, "text p_memsz");
    // The data's header: RW, at the next page.
    assert_eq!(u32_at(&with_bss, d + 4), 0b110, "the data is PF_R | PF_W");
    assert_eq!(u64_at(&with_bss, d + 8), 184, "data p_offset");
    assert_eq!(
        u64_at(&with_bss, d + 16),
        LOAD + 4096,
        "data p_vaddr is the next page"
    );
    assert_eq!(u64_at(&with_bss, d + 32), 2, "data p_filesz");
    assert_eq!(
        u64_at(&with_bss, d + 40),
        8 + 4096,
        "p_memsz is the data rounded up, plus the bss the loader zeroes"
    );
    assert!(Machine::load_elf(&with_bss, RAM).is_ok(), "yantra loads it");

    // The twin: the Rust writer labels the same image the same way.
    let rust = sadhana::kosha::write_full(&text, &data, &[], 4096);
    assert_eq!(
        rust[64..64 + 2 * 56],
        with_bss[64..64 + 2 * 56],
        "both writers' program headers"
    );
}

/// **W-363's SIZE HALF: A DECLARED `.bss` TAIL IS A DECLARATION, NOT A RAM
/// REQUIREMENT** — and the same image loads under a budget 25x smaller when the
/// loader is asked to read it that way.
///
/// The subject is the compiler's own heap: `yantrotsarjana.t1:2070` declares
/// 536,870,912 octets = 512.0 MiB of `.bss`, which costs no file octets and,
/// through `ram_for`, demanded the RAM anyway. A walker's high water over a whole
/// 154-second recording is 6.3 MiB, so the ask is 82x what the program touches,
/// and 540 MB of linear memory is the one thing standing between that decoder and
/// a phone browser.
///
/// WHY THE REFUSAL IS ASSERTED FIRST AND BY ITS TEXT. A test that only showed
/// `FileBacked` loading would pass just as well with the bound deleted outright —
/// the `Declared` arm is the removal control, and it has to refuse, name the
/// shortfall, and name THIS segment. Then the same octets, the same RAM, one
/// different span, and the answer changes. Both asks are printed rather than
/// hard-coded: the ratio is the figure, and a `METRIC` line survives into the log
/// where a constant in a source file does not.
#[test]
fn a_declared_bss_tail_is_not_a_ram_requirement_under_file_backed() {
    let text = [0x13u8, 0x00, 0x00, 0x00];
    // The compiler's own figure, not a round number invented here.
    const HEAP: u64 = 536_870_912;
    let image = write_image(&text, &[], HEAP);

    // The file carries none of it — the precondition the rest of this rests on.
    // Two program headers: the text's, and the writable one the heap is in (W-363).
    assert_eq!(
        image.len(),
        64 + 2 * 56 + text.len().next_multiple_of(8),
        "a 512 MiB reservation must cost zero file octets"
    );

    let declared = yantra::ram_for(&image);
    let file_backed = yantra::ram_for_span(&image, yantra::Span::FileBacked);
    assert!(
        declared > HEAP as usize,
        "`ram_for` reads the declared span, so it must exceed the heap itself: {declared}"
    );
    assert_eq!(
        file_backed,
        yantra::DEFAULT_RAM,
        "with the tail not counted, a four-octet program falls to the floor"
    );

    // THE REMOVAL CONTROL. At the small budget the default span must refuse, or
    // nothing below is evidence of anything.
    let refused = Machine::load_elf(&image, file_backed);
    let message = match refused {
        Ok(_) => panic!(
            "the declared span accepted {file_backed} octets of RAM for an image \
             declaring {HEAP} — the bound is not being read"
        ),
        Err(e) => e,
    };
    // WHAT THE REFUSAL NAMES IS THE SEGMENT'S `memsz`, NOT THE BARE HEAP. For a
    // four-octet text that was 536,870,920 — the heap plus the text and its
    // eight-alignment — until W-363 gave the heap a segment of its own, where it
    // is the heap alone; and an earlier version of this assertion pinned the
    // literal 536,870,912 and failed on exactly that difference. The code was
    // right and the expectation was eight octets off. So this reads the figure
    // out of the message and asserts the RELATION, which is what the test
    // actually knows: whatever the segment needed, it was at least the heap, and
    // it was refused against the budget this test passed in.
    let named: u64 = message
        .split("needs ")
        .nth(1)
        .and_then(|t| t.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("the refusal must name the octets it needed: {message}"));
    assert!(
        named >= HEAP,
        "the shortfall named ({named}) must be at least the declared heap ({HEAP}): {message}"
    );
    assert!(
        message.contains(&format!("RAM is {file_backed}")),
        "the refusal must name the RAM it refused against: {message}"
    );

    // AND THE CLAIM. Same octets, same RAM, one different span.
    let loaded = Machine::load_elf_spanning(&image, file_backed, yantra::Span::FileBacked);
    assert!(
        loaded.is_ok(),
        "the file-backed span must accept an image whose only excess is .bss: {:?}",
        loaded.err()
    );

    println!("METRIC kosha_image_ram_declared {declared}");
    println!("METRIC kosha_image_ram_file_backed {file_backed}");
    println!(
        "METRIC kosha_image_ram_over_ask_x100 {}",
        (declared * 100) / file_backed
    );
}

/// W-363: AN IMAGE WITH NOTHING TO WRITE KEEPS ONE HEADER — text only, `R E`,
/// and its `p_memsz` is the text's own length, NOT rounded to eight (the
/// rounding belonged to the data that used to follow it). Both writers.
#[test]
fn a_text_only_image_is_one_r_e_segment_of_exactly_the_text() {
    let text = [0x13u8, 0x00, 0x00, 0x00, 0x13]; // five octets — deliberately unaligned
    let t1 = write_image(&text, &[], 0);
    let rust = sadhana::kosha::write_full(&text, &[], &[], 0);
    for (who, img) in [("kosha.t1", &t1), ("kosha.rs", &rust)] {
        let u64_at = |at: usize| u64::from_le_bytes(img[at..at + 8].try_into().unwrap());
        assert_eq!(u16::from_le_bytes([img[56], img[57]]), 1, "{who}: e_phnum");
        assert_eq!(
            u32::from_le_bytes(img[68..72].try_into().unwrap()),
            0b101,
            "{who}: PF_R | PF_X"
        );
        assert_eq!(u64_at(64 + 8), 120, "{who}: p_offset, behind one header");
        assert_eq!(u64_at(64 + 32), 5, "{who}: p_filesz is the text");
        assert_eq!(
            u64_at(64 + 40),
            5,
            "{who}: p_memsz is the text, not rounded to 8"
        );
    }
    assert_eq!(t1.len(), 120 + text.len(), "nothing follows the text");
    assert_eq!(rust[64..120], t1[64..120], "both writers' program header");
}

/// W-363: the page boundary. Text of EXACTLY a page puts the data at the very
/// next page (load + 4096, not + 8192); one octet more pushes it a page further.
#[test]
fn text_of_exactly_a_page_puts_the_data_at_the_next_page_and_no_further() {
    let data = [0xAAu8; 3];
    for (len, want) in [(4096usize, 4096u64), (4097, 8192), (4095, 4096)] {
        let text = vec![0x13u8; len];
        let t1 = write_image(&text, &data, 0);
        let rust = sadhana::kosha::write_full(&text, &data, &[], 0);
        let d = 64 + 56;
        for (who, img) in [("kosha.t1", &t1), ("kosha.rs", &rust)] {
            let vaddr = u64::from_le_bytes(img[d + 16..d + 24].try_into().unwrap());
            assert_eq!(vaddr, LOAD + want, "{who}: {len} octets of text");
        }
        assert_eq!(
            rust[64..64 + 2 * 56],
            t1[64..64 + 2 * 56],
            "{len}: both writers"
        );
        assert!(
            Machine::load_elf(&t1, RAM).is_ok(),
            "{len}: yantra loads it"
        );
    }
}

/// W-363: `data_base` stated twice — `kosha::data_base` and
/// `कोशॱदत्तपृष्ठाधारः` — and asked of both, across the page boundary.
#[test]
fn both_engines_place_the_data_base_at_the_same_page() {
    let mut it = interpreter();
    for len in [0u64, 1, 4, 8, 4095, 4096, 4097, 8191, 8192, 12_345, 1 << 20] {
        let t1 = it
            .call(
                "कोशॱदत्तपृष्ठाधारः",
                vec![Value::Int(i128::from(len))],
                1_000_000,
            )
            .unwrap_or_else(|e| panic!("दत्तपृष्ठाधारः runs: {e}"))
            .as_int();
        let rust = sadhana::kosha::data_base(len);
        assert_eq!(t1, Some(i128::from(rust)), "text of {len} octets");
        assert_eq!(rust % 4096, 0, "{len}: on a page");
        assert!(rust >= len && rust - len < 4096, "{len}: the NEXT page");
    }
}
