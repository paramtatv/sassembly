//! Can the table read a real compressed instruction? — task `B-058b1`.
//!
//! The 33 sixteen-bit rows in `spec/encodings-riscv64.tsv` were derived and then
//! never used. `B-058a` checked them against an invariant over the table itself
//! — every bit fixed or an operand — and found two under-derived. This asks the
//! question that invariant cannot: given bytes `riscv64-elf-as` produced, does
//! the table say which instruction they are?
//!
//! The 32-bit half has had that check since `B-013`. This is the other half.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana has a grandparent")
        .to_path_buf()
}

/// `(assembly, halfword)` from the generated corpus.
fn corpus() -> Vec<(String, u16)> {
    let text = std::fs::read_to_string(root().join("spec/compressed-t0.tsv"))
        .expect("read spec/compressed-t0.tsv");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("assembly\t") && !l.trim().is_empty())
        .filter_map(|l| {
            let (asm, half) = l.split_once('\t')?;
            let half = u16::from_str_radix(half.trim().trim_start_matches("0x"), 16).ok()?;
            Some((asm.to_string(), half))
        })
        .collect()
}

/// The RISC-V mnemonic the assembly line asks for: `c.addi s0, 1` -> `c.addi`.
fn wanted(asm: &str) -> &str {
    asm.split_whitespace().next().unwrap_or(asm)
}

#[test]
fn every_compressed_instruction_the_assembler_emits_is_decodable() {
    let corpus = corpus();
    assert!(
        corpus.len() >= 40,
        "the corpus has shrunk to {} — regenerate it with tools/gen-compressed.py",
        corpus.len()
    );

    let mut wrong = Vec::new();
    for (asm, half) in &corpus {
        match sadhana::vishlesana::decode16(*half) {
            Some(d) if d.insn == wanted(asm) => {}
            Some(d) => wrong.push(format!("{asm} ({half:#06x}) decoded as `{}`", d.insn)),
            None => wrong.push(format!("{asm} ({half:#06x}) decoded as nothing")),
        }
    }
    assert!(
        wrong.is_empty(),
        "{} of {} compressed instructions decode wrongly:\n  {}",
        wrong.len(),
        corpus.len(),
        wrong.join("\n  ")
    );
    println!("METRIC compressed_decoded {}", corpus.len());
}

#[test]
fn the_length_is_read_from_the_low_bits_and_nothing_else() {
    // Every halfword in the corpus is a compressed instruction, so every one of
    // them must be two bytes wide by the length rule alone — without consulting
    // the table. That is what lets a stream be walked at all.
    for (asm, half) in corpus() {
        assert_eq!(
            sadhana::vishlesana::width_of(half),
            2,
            "{asm} ({half:#06x}) is compressed but reads as wide"
        );
    }
    // And a 32-bit instruction reads as four. `addi a0, zero, 1` is 0x00100513,
    // whose low halfword is 0x0513 — bottom bits 11.
    assert_eq!(sadhana::vishlesana::width_of(0x0513), 4);
}

#[test]
fn a_stream_of_mixed_widths_walks_correctly() {
    // The thing `B-058b2` will need and nothing has ever done: step through
    // bytes where the instruction size is not known in advance. Two compressed
    // instructions followed by a wide one.
    let (_, first) = corpus()[0];
    let (_, second) = corpus()[1];
    let wide: u32 = 0x0010_0513; // addi a0, zero, 1
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&first.to_le_bytes());
    bytes.extend_from_slice(&second.to_le_bytes());
    bytes.extend_from_slice(&wide.to_le_bytes());

    let mut at = 0usize;
    let mut seen = Vec::new();
    while at < bytes.len() {
        let (d, w) = sadhana::vishlesana::decode_at(&bytes, at).expect("decodes");
        seen.push(d.insn);
        at += w;
    }
    assert_eq!(at, bytes.len(), "the walk must land exactly on the end");
    assert_eq!(seen.len(), 3);
    assert_eq!(seen[2], "addi", "the wide one is read as wide");
}

#[test]
fn the_wide_decoder_refuses_a_compressed_word() {
    // A 16-bit pattern compared against a 32-bit word matches on the high bits
    // being zero, so keeping the two halves apart is what stops `c.nop` from
    // claiming every word that happens to start with one.
    for (asm, half) in corpus() {
        assert!(
            sadhana::vishlesana::decode(u32::from(half)).is_none(),
            "{asm} ({half:#06x}) was read as a 32-bit instruction"
        );
    }
}

/// `(assembly, halfword or "-")` — what GNU does with a WIDE instruction when
/// compression is allowed.
fn choices() -> Vec<(String, Option<u16>)> {
    let text = std::fs::read_to_string(root().join("spec/compression-choices.tsv"))
        .expect("read spec/compression-choices.tsv");
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("assembly\t") && !l.trim().is_empty())
        .filter_map(|l| {
            // Four columns since `B-058b2b3`: taking only the first tab left
            // the halfword glued to the instruction name and every row parsed
            // as "not compressed".
            let f: Vec<&str> = l.split('\t').collect();
            let half = f.get(1)?.trim();
            let word = (half != "-")
                .then(|| u16::from_str_radix(half.trim_start_matches("0x"), 16).ok())
                .flatten();
            Some((f.first()?.to_string(), word))
        })
        .collect()
}

#[test]
fn the_fusion_rules_are_recorded_rather_than_assumed() {
    // The measurement `B-058b2b2` turns on. Compression is not a filter over
    // encodings — it is a PATTERN MATCH on operands that no field layout
    // records: `add s0, s0, s1` compresses because rd equals rs1, and
    // `add s0, s1, a0` does not. Nothing in `spec/encodings-riscv64.tsv`
    // distinguishes them, which is why selection could not simply widen the
    // candidate filter.
    let rows = choices();
    assert!(
        rows.len() >= 30,
        "the derivation has shrunk to {}",
        rows.len()
    );

    let compressible = rows.iter().filter(|(_, w)| w.is_some()).count();
    let left_wide = rows.len() - compressible;
    assert!(
        compressible > 0 && left_wide > 0,
        "the corpus must contain both outcomes or it demonstrates nothing"
    );

    // The pair that states the rule most plainly, asserted by name so a
    // regeneration that lost it is loud.
    let find = |asm: &str| rows.iter().find(|(a, _)| a == asm).map(|(_, w)| *w);
    assert!(
        find("add s0, s0, s1").flatten().is_some(),
        "rd == rs1 compresses"
    );
    assert_eq!(
        find("add s0, s1, a0"),
        Some(None),
        "three distinct registers do not"
    );
    println!(
        "METRIC compression_choices {} ({compressible} compress)",
        rows.len()
    );
}

#[test]
fn the_compressed_encoder_agrees_with_the_assembler_where_it_can_choose() {
    // `compressed_at` matches operands to a 16-bit encoding's slots. Today that
    // succeeds only where the compressed form has the SAME shape as the wide
    // one — an instruction with no operands at all. One of 5949 conformance
    // cases, and that number is the size of what `B-058b2b2` has left to do.
    use std::collections::BTreeMap;
    let p = sadhana::parse::assemble_program("अन्वेषणविरामः ।\n").expect("parses");
    let inst = p.instructions.first().expect("one instruction");
    let got = sadhana::encode::compressed_at(inst, 0, &BTreeMap::new());
    // `c.ebreak` per riscv64-elf-as with `.option rvc`.
    assert_eq!(got, Some(0x9002));
}

/// The Sassembly for a wide RISC-V instruction, where one exists.
///
/// Only the forms the choices corpus uses; anything else returns `None` and is
/// counted as unchecked rather than passing silently.
fn sassembly(asm: &str) -> Option<String> {
    let (head, rest) = asm.split_once(' ')?;
    let ops: Vec<&str> = rest.split(',').map(str::trim).collect();
    let reg = |r: &str| match r {
        "zero" => Some("शून्यः"),
        "s0" => Some("स्थिर०"),
        "s1" => Some("स्थिर१"),
        "a0" => Some("अर्थ०"),
        "sp" => Some("स्तूपसूचकः"),
        _ => None,
    };
    let num = |n: &str| {
        let v: i64 = n.parse().ok()?;
        let dev: String = v
            .abs()
            .to_string()
            .chars()
            .map(|c| char::from_u32(0x966 + u32::from(c as u8 - b'0')).expect("digit"))
            .collect();
        Some(if v < 0 { format!("ऋण{dev}") } else { dev })
    };
    match head {
        "add" | "and" | "or" | "xor" | "sub" | "addw" => Some(format!(
            "{}{} {}म् {}न {}न ।",
            match head {
                "add" | "addw" => "योगः",
                "and" => "युक्तम्",
                "or" => "विकल्पः",
                "xor" => "वैषम्यम्",
                _ => "वियोगः",
            },
            if head == "addw" { "ॱअ३२" } else { "" },
            reg(ops.first()?)?,
            reg(ops.get(1)?)?,
            reg(ops.get(2)?)?
        )),
        "addi" => Some(format!(
            "योगः {}म् {}न {}न ।",
            reg(ops.first()?)?,
            reg(ops.get(1)?)?,
            num(ops.get(2)?)?
        )),
        _ => None,
    }
}

#[test]
fn the_encoder_makes_the_same_choice_as_the_assembler() {
    // `B-058b2b3`. The reduction is the whole of compression, and the RELATION
    // is what says which encoding a reduction meant: `add s0,s0,s1` and
    // `add s0,zero,s1` both leave two registers and become `c.add` and `c.mv`.
    //
    // Every expectation is `riscv64-elf-as` with `.option rvc`. We choose the
    // instruction; GNU chooses the bytes.
    use std::collections::BTreeMap;
    let mut checked = 0;
    let mut agreed = 0;
    let mut conservative = 0;
    let mut wrong = Vec::new();
    for (asm, want) in choices() {
        let Some(src) = sassembly(&asm) else { continue };
        let Ok(p) = sadhana::parse::assemble_program(&format!("{src}\n")) else {
            wrong.push(format!("{asm}: `{src}` does not parse"));
            continue;
        };
        let Some(inst) = p.instructions.first() else {
            continue;
        };
        checked += 1;
        let got = sadhana::encode::compressed_at(inst, 0, &BTreeMap::new());
        match (got, want) {
            // Agreed, either on a halfword or on leaving it wide.
            (a, b) if a == b => agreed += 1,
            // We stayed wide where the assembler compressed. Conservative, and
            // the wide form is always correct — this is the work `B-058b2b3`
            // has left, counted rather than hidden.
            (None, Some(_)) => conservative += 1,
            // Anything else is a WRONG instruction, and there is no acceptable
            // number of those.
            (a, b) => wrong.push(format!("{asm}: ours {a:x?}, as {b:x?}")),
        }
    }
    assert!(checked >= 12, "only {checked} choices were checkable");
    assert!(
        wrong.is_empty(),
        "{} of {checked} are the WRONG instruction:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );

    // Pinned so the conservative set can only shrink. The two are
    // `c.addi4spn` and `c.addi16sp`, whose immediates must be non-zero
    // multiples of four and sixteen — their maps are still partial, because a
    // field that rejects `lo | bit` for most `lo` is the one case the bounded
    // pair search does not reach.
    assert!(
        conservative <= 2,
        "{conservative} choices left wide, up from 2"
    );
    println!("METRIC compression_agreement {agreed}/{checked} ({conservative} left wide)");
}

#[test]
fn a_compressed_program_matches_the_assembler_byte_for_byte() {
    // `B-058b2b5`, and the first thing to make the relaxation fixpoint iterate
    // since `B-007` built it. Every instruction here compresses, so the branch
    // at offset 6 reaches its target at 10 — a displacement of 4 that is only
    // 4 because the two instructions between them are two bytes each. Laying
    // out at four bytes apiece would put the target at 20 and the branch would
    // be wrong by ten.
    //
    // The expected bytes are `riscv64-elf-as -march=rv64gc` on the equivalent
    // program, with `.option rvc`:
    //
    //   0: 4415  li    s0,5        6: c011  beqz s0,a
    //   2: 0405  addi  s0,s0,1     8: 0485  addi s1,s1,1
    //   4: 9426  add   s0,s0,s1    a: 4501  li   a0,0
    let src = "\
योगः स्थिर०म् शून्यःन ५न ।
योगः स्थिर०म् स्थिर०न १न ।
योगः स्थिर०म् स्थिर०न स्थिर१न ।
समलङ्घनम् स्थिर०न शून्यःत् समाप्तम्य् ।
योगः स्थिर१म् स्थिर१न १न ।
समाप्तम्ॱॱ
योगः अर्थ०म् शून्यःन ०न ।
";
    let p = sadhana::parse::assemble_program(src).expect("parses");
    let text = sadhana::encode::encode_program_for(&p, sadhana::encode::Target::Compressed)
        .unwrap_or_else(|e| panic!("did not encode: {e:?}"));

    let want: Vec<u8> = [0x4415u16, 0x0405, 0x9426, 0xc011, 0x0485, 0x4501]
        .iter()
        .flat_map(|h| h.to_le_bytes())
        .collect();
    assert_eq!(text, want, "bytes differ from riscv64-elf-as");
    assert_eq!(text.len(), 12, "six compressed instructions, twelve bytes");

    // And the same program at the other target is twice the size, which is what
    // the choice is for.
    let wide = sadhana::encode::encode_program(&p).expect("encodes");
    assert_eq!(wide.len(), 24, "six wide instructions");
}
