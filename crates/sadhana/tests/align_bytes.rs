//! What `॥ संरेखः n ॥` actually EMITS — task `W-079`.
//!
//! `W-071`'s seven tests all read one surface: `encode::layout_addresses`. That
//! function is now very well tested and the bytes it is supposed to describe
//! were not tested at all, so `W-071`'s fix could be reverted with all seventy
//! green. a peer session found that cold, by mutation; an eighth test of the same
//! kind would not have found it, which is the point.
//!
//! Every test here reads a byte or a recorded value that a PRODUCER wrote —
//! the symbol table `kosha` emits, the `.text` bytes `encode` emits — rather
//! than the layout function both of them consult. Each names the mutation it
//! kills, so a later reader can tell whether it still does.

use sadhana::encode::{Target, layout_addresses};
use sadhana::parse::{Program, Section, assemble_program};

/// One 4-byte instruction, as doc 02 §2.5 writes it.
const ONE: &str = "    योगः  अर्थ०म्  शून्यःन  ०१०न ।\n";

fn one(src: &str) -> Program {
    assemble_program(src).unwrap_or_else(|e| panic!("{src}: {e:?}"))
}

/// A program whose label sits after alignment padding: one instruction, then a
/// 16-byte boundary, then the label. The label's real address is 16; its
/// instruction INDEX is 1, so the address `W-071` replaced — `index * 4` — is 4.
/// Every test below depends on those two numbers differing, which is what makes
/// the reverted implementation observable.
fn padded() -> Program {
    one(&format!(
        "॥ कोष्ठकम् ॱपाठ ॥\n॥ वैश्विकम् मुख्यम् ॥\nमुख्यम्ॱॱ\n{ONE}॥ संरेखः १६ ॥\nलक्ष्यम्ॱॱ\n{ONE}"
    ))
}

fn label_index(p: &Program, name: &str) -> usize {
    let l = p
        .labels
        .iter()
        .find(|l| l.name == name)
        .unwrap_or_else(|| panic!("no label {name}"));
    assert_eq!(l.section, Section::Text, "{name} must be in ॱपाठ");
    l.at
}

// ---------------------------------------------------------------------------
// TEST 3 — first, because without it `W-071`'s fix is unprotected.
//
// KILLS: reverting `kosha.rs` to `l.at as u64 * 4`.
//
// `W-071`'s own test for this called `layout_addresses` directly, so it
// exercised the function that was ALREADY RIGHT rather than the `kosha` call
// site that was wrong. This one goes through `kosha::object` and reads the
// value out of the emitted symbol table, which is the thing a linker and a
// debugger actually read.
// ---------------------------------------------------------------------------

#[test]
fn the_emitted_symbol_table_records_the_padded_address() {
    let p = padded();
    let at = layout_addresses(&p, Target::Uncompressed);
    let idx = label_index(&p, "लक्ष्यम्");

    // The fixture is only meaningful if the two disagree. Asserted rather than
    // assumed: if a later change made every instruction four bytes again, this
    // test would silently stop testing anything.
    assert_eq!(at[idx], 16, "the label sits after twelve bytes of padding");
    assert_ne!(
        at[idx] as u64,
        idx as u64 * 4,
        "fixture is inert: index*4 happens to equal the real address"
    );

    let (text, pending) = sadhana::encode::encode_object(&p).expect("encodes");
    let bytes = sadhana::kosha::object(&text, &p, &pending, None, &at);
    let object = sadhana::vastu::read(&bytes).expect("our own object is readable");

    let sym = object
        .symbols
        .iter()
        .find(|s| s.name == "लक्ष्यम्")
        .expect("लक्ष्यम् reaches the symbol table");

    assert_eq!(
        sym.value,
        u64::from(at[idx]),
        "the symbol table says {:#x}, layout says {:#x} — a linker and the \
         disassembly would disagree about where लक्ष्यम् is",
        sym.value,
        at[idx]
    );
}

// ---------------------------------------------------------------------------
// TEST 4 — KILLS: padding becoming `0x00000000`.
//
// `W-071`'s comment justifies `nop` by citing the differential oracle, and that
// oracle never assembles an aligned program, so the justification was unchecked.
// `0x00000000` is not an instruction on RISC-V — it traps — so this mutation
// puts three words of trap in `.text` between two live instructions.
// ---------------------------------------------------------------------------

#[test]
fn the_padding_bytes_are_nops_and_not_traps() {
    let p = padded();
    let text = sadhana::encode::encode_program(&p).expect("encodes");
    let words = sadhana::encode::words(&text);

    assert_eq!(
        words.len(),
        5,
        "one instruction, three nops, one instruction"
    );
    for (i, w) in words[1..4].iter().enumerate() {
        assert_eq!(
            *w,
            0x0000_0013,
            "padding word {} is {:#010x}; 0x00000000 traps and 0x00000013 is nop",
            i + 1,
            w
        );
    }
    assert_ne!(
        words[1], 0,
        "an all-zero word in ॱपाठ is an illegal instruction, not padding"
    );
}

// ---------------------------------------------------------------------------
// TEST 5 — KILLS: `align_pad_at` returning a constant.
//
// `W-071` checked two link addresses, 0x80000000 and 0x80200000. Both are ≡ 0
// (mod 32), so the assertion held for any padding whatsoever — an implementation
// that always emitted the same number of bytes passed it. What discriminates is
// that the padding must DEPEND on how far the section already is from the
// boundary, so two programs differing only in preceding length must pad by
// different amounts and still land on the same boundary.
// ---------------------------------------------------------------------------

#[test]
fn the_padding_depends_on_how_far_the_section_already_is() {
    let mut seen = Vec::new();
    for before in 1..=3 {
        let src = format!(
            "॥ कोष्ठकम् ॱपाठ ॥\nमुख्यम्ॱॱ\n{}॥ संरेखः १६ ॥\nलक्ष्यम्ॱॱ\n{ONE}",
            ONE.repeat(before)
        );
        let p = one(&src);
        let at = layout_addresses(&p, Target::Uncompressed);
        let idx = label_index(&p, "लक्ष्यम्");

        assert_eq!(
            at[idx] % 16,
            0,
            "with {before} instruction(s) before it, लक्ष्यम् landed at {:#x}",
            at[idx]
        );
        // The padding actually emitted, not the address it produced.
        let pad = at[idx] - (before as u32 * 4);
        seen.push(pad);
    }

    assert_eq!(
        seen,
        vec![12, 8, 4],
        "padding must shrink as the section approaches the boundary; a constant \
         would satisfy the alignment assertion and be wrong"
    );
}
