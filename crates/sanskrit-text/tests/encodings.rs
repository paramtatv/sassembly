//! `spec/encodings-riscv64.tsv` must be derived, complete and consistent — B-008.
//!
//! Every pattern in that file came out of `riscv64-elf-as`, never out of anyone's
//! memory. These tests check the properties that would reveal a hand edit or a
//! stale regeneration, because a wrong encoding does not fail loudly — it
//! assembles to a different instruction and runs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sanskrit-text has a grandparent")
        .to_path_buf()
}

struct Row {
    insn: String,
    family: String,
    mnemonic: String,
    pattern: u32,
    mask: u32,
    bits: u32,
}

fn rows() -> Vec<Row> {
    let text = std::fs::read_to_string(root().join("spec/encodings-riscv64.tsv"))
        .expect("read spec/encodings-riscv64.tsv");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("insn\t") && !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            assert!(f.len() >= 6, "malformed row: {l:?}");
            Row {
                insn: f[0].into(),
                family: f[1].into(),
                mnemonic: f[2].into(),
                pattern: u32::from_str_radix(f[3].trim_start_matches("0x"), 16).expect("pattern"),
                mask: u32::from_str_radix(f[4].trim_start_matches("0x"), 16).expect("mask"),
                bits: f[5].parse().expect("bits"),
            }
        })
        .collect()
}

#[test]
fn every_instruction_the_registry_names_has_an_encoding() {
    // The registry is the ISA surface (R-02-1). An instruction named there with
    // no encoding is one the assembler cannot emit, which makes the name a
    // promise the toolchain does not keep.
    let reg = std::fs::read_to_string(root().join("spec/mnemonics-riscv64.src.tsv"))
        .expect("read registry");
    let have: Vec<String> = rows().into_iter().map(|r| r.insn).collect();

    let mut missing = Vec::new();
    for line in reg.lines() {
        if line.starts_with('#') || line.starts_with("family\t") || line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 5 || f[4].trim().starts_with('(') {
            continue;
        }
        for insn in f[4].split(',').map(str::trim) {
            if insn.is_empty() || insn.contains('(') {
                continue;
            }
            if !have.contains(&insn.to_string()) {
                missing.push(format!("{insn} ({})", f[0]));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "named in the registry but not encoded — rerun tools/gen-encodings.py:\n  {}",
        missing.join("\n  ")
    );
}

#[test]
fn a_branch_target_is_a_displacement_with_real_bits() {
    // THIS TEST USED TO ASSERT THE OPPOSITE, and the assertion was the bug.
    //
    // It required every branch and jump target to be marked `label`, on the
    // reasoning that PC-relative targets "occupy no operand field the encoder
    // can fill" because they are resolved by relocation. That is true of a
    // SYMBOL and false of a DISPLACEMENT: `beq x0, x0, .+8` is a legal
    // instruction and the assembler places the distance in the field at once.
    //
    // Believing it cost two things. No branch could be encoded, since the slot
    // had no bits to put anything in. And because no probe ever moved those
    // bits, they read as fixed opcode — `beq`'s mask came out 0xfe007dff and
    // `jal`'s 0xffbff07f, each swallowing its own displacement, which would
    // make a mask-based disassembler reject every real branch (`B-013`).
    //
    // The B-type and J-type layouts are now derived by probing `.+N` and `.-N`,
    // and appear nowhere in this repository as a written-down table (`B-055`).
    let text = std::fs::read_to_string(root().join("spec/encodings-riscv64.tsv")).expect("read");
    let mut checked = 0;
    for l in text.lines() {
        if l.starts_with('#') || l.starts_with("insn\t") {
            continue;
        }
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() < 7 {
            continue;
        }
        if !matches!(
            f[0],
            "beq" | "bne" | "blt" | "bge" | "bltu" | "bgeu" | "jal"
        ) {
            continue;
        }
        let disp = f[6]
            .split('|')
            .find(|s| s.starts_with("disp:"))
            .unwrap_or_else(|| panic!("{}: no displacement slot in {}", f[0], f[6]));
        let parts: Vec<&str> = disp.split(':').collect();
        let mask = u32::from_str_radix(parts[1].trim_start_matches("0x"), 16).expect("hex");
        assert_ne!(mask, 0, "{}: displacement has no bits", f[0]);
        let map = parts.get(2).copied().unwrap_or("");
        assert!(!map.is_empty(), "{}: displacement has no bit map", f[0]);
        // Two-byte alignment: value bit 0 is never representable, and that is
        // observed rather than assumed — it is probed like every other bit and
        // simply never moves anything.
        assert!(
            !map.split(';').any(|p| p.starts_with("0>")),
            "{}: value bit 0 should not map anywhere",
            f[0]
        );
        // The sign bit is the one a positive-only probe cannot reach.
        let top = map
            .split(';')
            .filter_map(|p| p.split_once('>'))
            .map(|(v, _)| v.parse::<u32>().unwrap_or(0))
            .max()
            .expect("non-empty");
        assert!(top >= 12, "{}: displacement stops at value bit {top}", f[0]);
        checked += 1;
    }
    assert_eq!(checked, 7, "expected six branches and jal");
}

#[test]
fn every_non_label_slot_has_a_derived_field() {
    // A slot with no bits and no label marking means the probe never moved it —
    // the field is unknown, and an encoder would silently place nothing there.
    let text = std::fs::read_to_string(root().join("spec/encodings-riscv64.tsv")).expect("read");
    let mut bad = Vec::new();
    for l in text.lines() {
        if l.starts_with('#') || l.starts_with("insn\t") || l.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = l.split('\t').collect();
        for slot in f[6].split('|') {
            // `fixed` is a hardwired operand — c.addi16sp always takes sp — so
            // it correctly occupies no field.
            if slot == "(none)" || slot.starts_with("label") || slot.starts_with("fixed") {
                continue;
            }
            if slot.ends_with(":0x00000000") {
                bad.push(format!("{} {}", f[0], slot));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "slots with no derived field:\n  {}",
        bad.join("\n  ")
    );
}

/// One operand slot: what kind it is, which bits it owns, and where each value
/// bit lands.
struct Slot {
    kind: String,
    mask: u32,
    /// (value bit, encoding bit) pairs.
    map: Vec<(u32, u32)>,
}

/// Slots of one row.
fn slots_of(row: &str) -> Vec<Slot> {
    let f: Vec<&str> = row.split('\t').collect();
    if f[6] == "(none)" {
        return Vec::new();
    }
    f[6].split('|')
        .map(|s| {
            let p: Vec<&str> = s.split(':').collect();
            let mask = u32::from_str_radix(p[1].trim_start_matches("0x"), 16).expect("mask");
            let map = p
                .get(2)
                .map(|m| {
                    m.split(';')
                        .filter(|x| !x.is_empty())
                        .map(|x| {
                            let (a, b) = x.split_once('>').expect("bit map entry");
                            (a.parse().expect("from"), b.parse().expect("to"))
                        })
                        .collect()
                })
                .unwrap_or_default();
            Slot {
                kind: p[0].to_string(),
                mask,
                map,
            }
        })
        .collect()
}

fn data_rows() -> Vec<String> {
    std::fs::read_to_string(root().join("spec/encodings-riscv64.tsv"))
        .expect("read")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("insn\t") && !l.trim().is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn every_mapped_bit_lands_inside_its_own_field() {
    // The map says where a value bit goes; the mask says which bits the slot
    // owns. A target outside the mask would have the encoder writing into
    // another operand's field — or into the opcode.
    for row in data_rows() {
        let insn = row.split('\t').next().unwrap_or("?");
        for s in slots_of(&row) {
            for (from, to) in &s.map {
                assert!(
                    s.mask & (1 << to) != 0,
                    "{insn}: {} value bit {from} maps to encoding bit {to}, \
                     outside its mask {:#010x}",
                    s.kind,
                    s.mask
                );
            }
        }
    }
}

#[test]
fn no_two_value_bits_share_an_encoding_bit() {
    // Two value bits landing on one encoding bit means one of them was
    // mis-observed; the encoder would silently lose information.
    for row in data_rows() {
        let insn = row.split('\t').next().unwrap_or("?");
        for s in slots_of(&row) {
            let mut seen = Vec::new();
            for (from, to) in &s.map {
                assert!(
                    !seen.contains(to),
                    "{insn}: {} maps two value bits onto encoding bit {to} (bit {from})",
                    s.kind
                );
                seen.push(*to);
            }
        }
    }
}

#[test]
fn every_register_slot_maps_five_bits() {
    // A RISC-V register field is five bits. Fewer means the probe failed to
    // move some of them and the encoder would truncate register numbers —
    // silently turning x24 into x8.
    for row in data_rows() {
        let insn = row.split('\t').next().unwrap_or("?");
        if insn.starts_with("c.") {
            continue; // compressed fields are three bits, or two, by design
        }
        for s in slots_of(&row) {
            if s.kind == "reg" || s.kind == "freg" {
                assert_eq!(
                    s.map.len(),
                    5,
                    "{insn}: {} maps {} bits, expected 5 — {:?}",
                    s.kind,
                    s.map.len(),
                    s.map
                );
            }
        }
    }
}

#[test]
fn the_s_type_split_immediate_is_derived_correctly() {
    // Hand-derived from the specification: a store immediate places imm[4:0]
    // into bits 11..7 and imm[11:5] into bits 31..25. Nothing wrote that layout
    // down — this asserts the probe observed it.
    let row = data_rows()
        .into_iter()
        .find(|r| r.starts_with("sd\t"))
        .expect("sd row");
    // `simm` since `B-061`: the field accepts -1, which the assembler was
    // asked rather than told, so a store offset is signed.
    let s = slots_of(&row)
        .into_iter()
        .find(|s| s.kind == "imm" || s.kind == "simm")
        .expect("sd has an immediate");
    assert_eq!(
        s.mask, 0xfe00_0f80,
        "S-type immediate occupies 31..25 and 11..7"
    );
    // (11, 31) is the sign bit and was NOT in this list before `B-061`: the
    // probe only tried positive values, and no positive store offset reaches
    // imm[11]. Its absence meant a negative offset could not be encoded at all,
    // and the mask claimed bit 31 while the map could not fill it.
    for pair in [(0u32, 7u32), (4, 11), (5, 25), (10, 30), (11, 31)] {
        assert!(
            s.map.contains(&pair),
            "sd: value bit {} should map to encoding bit {}; got {:?}",
            pair.0,
            pair.1,
            s.map
        );
    }
}

#[test]
fn a_pattern_never_sets_a_bit_its_mask_does_not_cover() {
    // The pattern is the invariant bits; the mask says which those are. A bit
    // set outside the mask would mean an operand field leaked into the pattern,
    // which is exactly what the differencing method exists to prevent.
    for r in rows() {
        assert_eq!(
            r.pattern & !r.mask,
            0,
            "{}: pattern {:#010x} sets bits outside mask {:#010x}",
            r.insn,
            r.pattern,
            r.mask
        );
    }
}

#[test]
fn compressed_encodings_fit_in_sixteen_bits() {
    for r in rows() {
        if r.bits == 16 {
            assert!(
                r.pattern <= 0xFFFF && r.mask <= 0xFFFF,
                "{} is 16-bit but pattern/mask exceed it",
                r.insn
            );
            assert!(r.insn.starts_with("c."), "{} is 16-bit but not c.*", r.insn);
        } else {
            assert_eq!(r.bits, 32, "{}: unexpected width {}", r.insn, r.bits);
        }
    }
}

#[test]
fn the_opcode_field_is_always_constrained() {
    // Bits 0..=6 are the RISC-V opcode. If differencing left any of them free,
    // the candidate shapes did not actually vary the operands and the "pattern"
    // would be one arbitrary encoding rather than the family's fixed bits.
    for r in rows() {
        let opcode_bits = if r.bits == 16 { 0b11 } else { 0b111_1111 };
        assert_eq!(
            r.mask & opcode_bits,
            opcode_bits,
            "{}: opcode field is not fully constrained (mask {:#010x})",
            r.insn,
            r.mask
        );
    }
}

#[test]
fn no_two_instructions_share_a_pattern_and_mask() {
    // Two entries with identical fixed bits would be indistinguishable to a
    // decoder, which means one of them is wrong.
    let mut seen: BTreeMap<(u32, u32), String> = BTreeMap::new();
    for r in rows() {
        if let Some(prev) = seen.insert((r.pattern, r.mask), r.insn.clone()) {
            panic!(
                "{} and {} have identical pattern {:#010x} / mask {:#010x}",
                prev, r.insn, r.pattern, r.mask
            );
        }
    }
}

#[test]
fn known_encodings_match_the_specification() {
    // Three values derived by hand from the pinned RISC-V PDF, the same way
    // tools/check-toolchain.sh derives `add`. If the generator ever produced
    // something else, this is what would catch it — the generator's output is
    // not permitted to be its own oracle.
    let by: BTreeMap<String, Row> = rows().into_iter().map(|r| (r.insn.clone(), r)).collect();

    // Each derived by hand from the pinned PDF's opcode map. Every one of these
    // caught a real generator defect while B-008 was being written:
    for (insn, want, why) in [
        (
            "add",
            0x0000_0033u32,
            "funct7 0000000, funct3 000, opcode 0110011",
        ),
        ("sub", 0x4000_0033, "as add, funct7 0100000"),
        (
            "addi",
            0x0000_0013,
            "funct3 000, opcode 0010011 — caught auto-compression",
        ),
        (
            "beq",
            0x0000_0063,
            "funct3 000, opcode 1100011 — caught the I-shape mismatch",
        ),
        ("bne", 0x0000_1063, "funct3 001"),
        ("blt", 0x0000_4063, "funct3 100"),
        ("bge", 0x0000_5063, "funct3 101"),
        (
            "jal",
            0x0000_006f,
            "opcode 1101111 — caught rd=ra baked in by the pseudo-form",
        ),
        ("jalr", 0x0000_0067, "opcode 1100111"),
        ("lui", 0x0000_0037, "opcode 0110111"),
        ("ecall", 0x0000_0073, "fully fixed"),
        ("ebreak", 0x0010_0073, "as ecall with imm 1"),
    ] {
        assert_eq!(
            by[insn].pattern, want,
            "{insn} ({why}): got {:#010x}",
            by[insn].pattern
        );
    }
}

#[test]
fn every_row_names_a_family_and_a_mnemonic_that_exist() {
    let reg = std::fs::read_to_string(root().join("spec/mnemonics-riscv64.src.tsv"))
        .expect("read registry");
    for r in rows() {
        assert!(
            reg.contains(&format!("\n{}\t", r.family)),
            "{}: family `{}` is not in the registry",
            r.insn,
            r.family
        );
        assert!(
            reg.contains(&r.mnemonic),
            "{}: mnemonic {} is not in the registry",
            r.insn,
            r.mnemonic
        );
    }
    println!("METRIC rv64gc_encodings_derived {}", rows().len());
}
