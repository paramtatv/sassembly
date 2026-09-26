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

/// Data shares the one `PT_LOAD`, eight-aligned after the text, and a `.bss`
/// reservation shows up in `p_memsz` without lengthening the file.
#[test]
fn data_is_eight_aligned_after_the_text_and_bss_costs_no_file_bytes() {
    let text = [0x13u8, 0x00, 0x00, 0x00, 0x13]; // five octets — deliberately unaligned
    let data = [0xAAu8, 0xBB];
    let with_bss = write_image(&text, &data, 4096);
    let without = write_image(&text, &data, 0);

    assert_eq!(
        with_bss.len(),
        without.len(),
        "a bss reservation is space the file does not carry"
    );
    // text begins at 120; five octets end at 125; data must begin at 128.
    assert_eq!(
        with_bss.len(),
        128 + data.len(),
        "data begins eight-aligned after the text:\n{:02x?}",
        with_bss
    );
    let filesz = u64::from_le_bytes(with_bss[96..104].try_into().unwrap());
    let memsz = u64::from_le_bytes(with_bss[104..112].try_into().unwrap());
    assert_eq!(filesz, 10, "p_filesz spans text, padding and data");
    assert_eq!(
        memsz,
        16 + 4096,
        "p_memsz is the span rounded up, plus the bss the loader zeroes"
    );
    assert!(Machine::load_elf(&with_bss, RAM).is_ok(), "yantra loads it");
}
