//! Structural invariants on `spec/encodings-riscv64.tsv`.
//!
//! The conformance suite (B-003) checks what the encoder EMITS. This checks
//! what the encoder is emitting FROM, and it exists because a table can be
//! internally consistent, pass every test that was written, and still be wrong.
//!
//! `slli` shipped as pattern `0x00811413`, mask `0xffffffff`. Its field maps
//! were correct. Its mnemonic was correct. Nothing in the encoder disagreed
//! with anything else in the encoder — the probe's own operands, `rd=x8` and
//! `shamt=8`, had simply been fused into what the file called the opcode,
//! because `slli x8, sp, 8` assembles and so matched a single-variant candidate
//! shape meant for `c.addi4spn`. With one variant there is nothing to difference
//! and the mask stays all-ones.
//!
//! The result encoded `slli t0, t1, 4` as `0x00c31693`, which is not a wrong
//! instruction in the sense of being rejected — it is `slli a3, t1, 12`, a
//! perfectly valid instruction that the programmer did not write.
//!
//! Both invariants below are cheap, and either alone would have caught it.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

struct Row {
    insn: String,
    pattern: u32,
    mask: u32,
    bits: u32,
    fields: u32,
    shape: String,
}

fn rows() -> Vec<Row> {
    let text = std::fs::read_to_string(root().join("spec/encodings-riscv64.tsv"))
        .expect("read spec/encodings-riscv64.tsv");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("insn\t") && !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            assert!(f.len() >= 10, "malformed encoding row: {l:?}");
            let hex = |s: &str| u32::from_str_radix(s.trim_start_matches("0x"), 16).expect("hex");
            // Each field entry is `kind:0xMASK[:bitmap]`; the union of the
            // masks is every bit any operand occupies.
            let fields = f[6]
                .split('|')
                .filter(|e| *e != "(none)")
                .filter_map(|e| e.split(':').nth(1))
                .map(hex)
                .fold(0, |a, b| a | b);
            Row {
                insn: f[0].into(),
                pattern: hex(f[3]),
                mask: hex(f[4]),
                bits: f[5].parse().expect("bit width"),
                fields,
                shape: f[9].into(),
            }
        })
        .collect()
}

#[test]
fn every_field_has_somewhere_to_put_a_value() {
    // A slot with a mask and no bit map is a field the encoder cannot write
    // into: `place` walks the map, so an empty one emits zero however large the
    // value. Forty of the compressed slots were in that state (`B-058b2b1`) and
    // nothing noticed, because the bit-accounting invariant is satisfied by the
    // MASK alone — the field's bits were accounted for as the field's, and the
    // field still could not be filled.
    //
    // The cause was one line: `probe_bitmap` used `x0` as its baseline, and a
    // compressed three-bit register field reaches only `x8`..`x15`, so the
    // baseline never assembled and the map came back empty. The probe now
    // searches for a baseline the field accepts and records the offset as the
    // slot's bias.
    let text = std::fs::read_to_string(root().join("spec/encodings-riscv64.tsv")).expect("read");
    let mut blind = Vec::new();
    for l in text.lines() {
        if l.starts_with('#') || l.starts_with("insn\t") || l.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = l.split('\t').collect();
        for entry in f[6].split('|') {
            let p: Vec<&str> = entry.split(':').collect();
            if p.len() < 2 || p[0] == "fixed" {
                continue;
            }
            let mask = u32::from_str_radix(p[1].trim_start_matches("0x"), 16).unwrap_or(0);
            let mapped = p.get(2).is_some_and(|m| !m.trim().is_empty());
            if mask != 0 && !mapped {
                blind.push(format!("{} {}", f[0], entry));
            }
        }
    }
    assert!(
        blind.is_empty(),
        "{} field(s) have bits but no way to place a value into them:\n  {}",
        blind.len(),
        blind.join("\n  ")
    );
}

#[test]
fn every_bit_is_either_fixed_or_an_operand() {
    // The invariant that finds an under-derived probe, and the only one that
    // does. An instruction is a fixed pattern plus its operands, so a bit
    // belonging to NEITHER was never observed to move and was never claimed —
    // which means some probe value was missing rather than that the bit is
    // unused.
    //
    // All 189 32-bit rows satisfied it already. Two compressed rows did not,
    // and each had its own cause (`B-058a`):
    //
    //   `c.j`        — `probe_displacement` compared SEVEN opcode bits to
    //                  decide a probe had not changed the instruction, and a
    //                  compressed opcode is two. Every probe that moved bits
    //                  2-6 was thrown away, so six of eleven displacement bits
    //                  went unmapped and the mask took them for opcode.
    //   `c.addi4spn` — its immediate must be a multiple of four, so 4, 256 and
    //                  512 were the only values that could move bits 6, 9 and
    //                  10, and `PROBE_IMM` held none of them.
    //
    // Neither was caught by the mask/field overlap test below: the mask and
    // the field map were wrong TOGETHER, consistently, so they never
    // contradicted each other. Only counting the bits neither accounts for
    // finds that.
    let unaccounted: Vec<(String, u32)> = rows()
        .iter()
        .filter_map(|r| {
            let all: u32 = if r.bits == 32 { u32::MAX } else { 0xffff };
            let missing = all & !(r.mask | r.fields);
            (missing != 0).then(|| (r.insn.clone(), missing))
        })
        .collect();
    assert!(
        unaccounted.is_empty(),
        "{} encoding(s) have bits that are neither fixed nor an operand: {:x?}",
        unaccounted.len(),
        unaccounted
    );
}

#[test]
fn an_instruction_with_operands_never_claims_every_bit_is_fixed() {
    // An all-ones mask says "this instruction is one exact word". True for
    // `ecall`. For anything taking operands it means the mask was never
    // derived — no two variants ever differed — and the pattern therefore
    // still carries whatever registers the probe happened to use.
    let mut bad = Vec::new();
    for r in rows() {
        let full = if r.bits == 16 { 0xFFFF } else { 0xFFFF_FFFF };
        if r.shape != "(none)" && r.mask == full {
            bad.push(format!(
                "{}: mask {:#010x} fixes every bit, but it takes operands ({})",
                r.insn, r.mask, r.shape
            ));
        }
    }
    assert!(bad.is_empty(), "unsound encodings:\n  {}", bad.join("\n  "));
}

#[test]
fn no_bit_is_both_fixed_encoding_and_operand_field() {
    // A bit cannot be part of the opcode and part of an operand. When the two
    // derivations disagree, one of them is wrong and the encoder will either
    // clobber an operand or emit a corrupt opcode.
    //
    // This caught more than the shifts. The R-type candidate used rs1 = x0,
    // x2, x30, x6 — all even — so rs1's low bit never moved, and bit 15 read
    // as fixed encoding for `add`, `mul`, every load and 170 others. Harmless
    // while encoding, because the pattern holds a zero there, and fatal the
    // moment anything DECODES with the mask (B-013).
    let mut bad = Vec::new();
    for r in rows() {
        let overlap = r.mask & r.fields;
        if overlap != 0 {
            bad.push(format!(
                "{}: bits {overlap:#010x} are claimed as fixed encoding and as operand field",
                r.insn
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "contradictory rows:\n  {}",
        bad.join("\n  ")
    );
}

#[test]
fn the_pattern_never_sets_a_bit_the_mask_does_not_constrain() {
    // Bits outside the mask are operand territory. A pattern with one set
    // there would OR itself into every instruction of that family.
    let mut bad = Vec::new();
    for r in rows() {
        if r.pattern & !r.mask != 0 {
            bad.push(format!(
                "{}: pattern {:#010x} sets bits outside mask {:#010x}",
                r.insn, r.pattern, r.mask
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "stray pattern bits:\n  {}",
        bad.join("\n  ")
    );
}

#[test]
fn the_families_that_file_order_silently_decides_are_pinned() {
    // `B-056`. The encoder picks among a family's encodings by operand shape
    // and then by width. When several fit both, it takes whichever the file
    // lists first — so `गुणनम्` is always `mul` and never `mulh`, `भागः` is
    // always `div` and never `divu`, and every float operation is single or
    // double precision by alphabetical accident.
    //
    // This is the same defect that made every load a `lb` (`B-043`), and it
    // was found by a guard that refused to choose: it broke 1017 conformance
    // cases at once, because the corpus can only spell the choice file order
    // happens to make. Refusing is the right behaviour and cannot land until
    // 30 families have distinguishing names, which is a naming project rather
    // than a code change.
    //
    // `B-056` closed 22 of the original 34 with no new names at all: 15 groups
    // were the व्याप्ति width suffix applied where it already belonged (the
    // dotted `.w`/`.d`/`.s` on atomics and floats), and 7 more were ADR-0006's
    // अचिह्न- prefix applied to five signed/unsigned pairs.
    //
    // The 12 that remained were not spellings of one operation. `B-059` gave
    // eleven of them distinct names — `गुणनम्` covers four genuinely different
    // products and `यदिलङ्घनम्` six conditions, and those are separate
    // operations rather than one operation at several widths.
    //
    // `fcvt` was the last, and it was not a naming problem at all. Eighteen
    // conversions, one name, and the operands ALREADY say three of the four
    // ways they differ: the destination's register class says integer or
    // float, its type name says signed or unsigned, and the width says how
    // wide. Only what the conversion READS was unsayable. `B-075` gave the
    // language a second type suffix rather than eighteen names, and the count
    // reached zero.
    //
    // It stays pinned at zero. Nothing here may become ambiguous again.
    use std::collections::BTreeMap;
    /// What the encoder selects on: family, width, register count, whether
    /// there is an immediate, whether there is a displacement, the
    /// destination's register class, and the pair of types a conversion names.
    type SelectionKey = (String, String, usize, bool, bool, String, String);
    let mut groups: BTreeMap<SelectionKey, Vec<String>> = BTreeMap::new();
    let text = std::fs::read_to_string(root().join("spec/encodings-riscv64.tsv")).expect("read");
    for l in text.lines() {
        if l.starts_with('#') || l.starts_with("insn\t") || l.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() < 10 || f[5] != "32" {
            continue;
        }
        let slots: Vec<&str> = f[6].split('|').collect();
        let regs = slots
            .iter()
            .filter(|s| s.starts_with("reg:") || s.starts_with("freg:"))
            .count();
        // The DESTINATION's register class is part of what distinguishes an
        // encoding, because the encoder selects on it: `प्लवसंचारः` covers
        // four moves and the operands say which way each goes (`B-074`).
        // Leaving it out of the key made the census report a pair the encoder
        // can tell apart perfectly well. The CLASS, not only float-or-not: a
        // VECTOR destination (`vreg:`) is told apart the same way — `ld` and
        // `vlse64.v` (`V-009` (ii)) share family, mnemonic, width and shape, and
        // `encode.rs` rejects a candidate whose rd slot's vector-ness differs
        // from the destination register's (the `vle64.v v6, (ra)` incident).
        let dest_class = slots
            .first()
            .and_then(|s| s.split(':').next())
            .unwrap_or("")
            .to_string();
        let imm = slots.iter().any(|s| s.starts_with("imm:"));
        let disp = slots.iter().any(|s| s.starts_with("disp:"));
        groups
            .entry((
                f[1].into(),
                f[7].into(),
                regs,
                imm,
                disp,
                dest_class,
                f[8].into(),
            ))
            .or_default()
            .push(f[0].into());
    }
    let ambiguous: Vec<_> = groups.values().filter(|v| v.len() > 1).collect();
    let families: std::collections::BTreeSet<&String> = groups
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, _)| &k.0)
        .collect();

    assert_eq!(
        ambiguous.len(),
        0,
        "ambiguous selection groups changed: {ambiguous:#?}"
    );
    assert_eq!(
        families.len(),
        0,
        "ambiguous families changed: {families:?}"
    );
    println!("METRIC ambiguous_selection_groups {}", ambiguous.len());
    println!("METRIC ambiguous_families {}", families.len());
}

#[test]
fn a_signed_field_is_range_checked_as_signed() {
    // `B-061`. Twelve bits hold -2048..2047 signed and 0..4095 unsigned, and
    // judging one by the other is how a branch 4096 bytes forward encoded as
    // one 4096 bytes backward (`B-012`).
    //
    // Before this, `addi`'s imm[11] was not mapped at all — the probe only ever
    // tried positive values, and 2048 is one past the field's +2047. `fits`
    // therefore saw an eleven-bit field and rejected 2048 for the wrong reason,
    // while a negative immediate could not be encoded at any value.
    let all = sadhana::encode::encodings();
    let find = |name: &str| {
        all.iter()
            .find(|e| e.insn == name)
            .unwrap_or_else(|| panic!("{name} missing"))
    };

    let addi = find("addi");
    let imm = addi
        .slots
        .iter()
        .find(|s| s.kind == "simm")
        .expect("addi's immediate is signed");
    assert_eq!(imm.map.len(), 12, "twelve bits, sign included");
    assert!(imm.fits(2047u64), "+2047 is the largest");
    assert!(!imm.fits(2048u64), "+2048 is one past");
    assert!(imm.fits((-2048i64) as u64), "-2048 is the smallest");
    assert!(!imm.fits((-2049i64) as u64), "-2049 is one past");

    // A shift amount is a count, not a number, and the assembler refuses -1 for
    // it — so the field stays unsigned and its whole range is positive.
    let slli = find("slli");
    let shamt = slli
        .slots
        .iter()
        .find(|s| s.kind == "imm")
        .expect("a shift amount is unsigned");
    assert!(shamt.fits(63u64), "63 is a legal shift on RV64");
    assert!(!shamt.fits(64u64), "64 is not");

    // `lui` takes twenty unsigned bits.
    let lui = find("lui");
    let up = lui
        .slots
        .iter()
        .find(|s| s.kind == "imm")
        .expect("lui's immediate is unsigned");
    assert!(up.fits(1_048_575u64));
    assert!(!up.fits(1_048_576u64));
}
