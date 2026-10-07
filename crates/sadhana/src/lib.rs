//! **साधनम्** (Sadhana) — the SANSOS toolchain, doc 17's name for assembler,
//! linker and compiler as one instrument.
//!
//! Today it holds the T0 lexer. The grammar it reads is frozen at
//! `spec/grammar-t0.ebnf`; the mnemonics it will resolve against are
//! `spec/mnemonics-riscv64.src.tsv`.

pub mod census;
pub mod devanagari8;
pub mod duplicates;
pub mod dwarf;
pub mod encode;
pub mod kosha;
pub mod lex;
pub mod nidana;
pub mod parse;
pub mod samyojana;
#[allow(missing_docs)]
pub mod t1;
pub mod vastu;
pub mod vishlesana;

/// A structured diagnostic from the toolchain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// 1-based line.
    pub line: usize,
    /// Akṣara index into the source.
    pub aksara_index: Option<usize>,
    /// Byte offset into the source.
    pub byte_index: Option<usize>,
    /// The message describing the fault.
    pub reason: String,
}

/// Assembles a single source file into an ELF image.
pub fn assemble(
    source: &str,
    target: encode::Target,
    load_addr: u64,
    lang: nidana::Language,
) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let tokens = lex::lex(source).map_err(|es| {
        es.into_iter()
            .map(|e| Diagnostic {
                line: e.line,
                aksara_index: Some(e.index),
                byte_index: Some(e.byte),
                reason: e.reason,
            })
            .collect::<Vec<_>>()
    })?;

    let program = parse::parse(&tokens).map_err(|es| {
        es.into_iter()
            .map(|e| Diagnostic {
                line: e.line,
                aksara_index: Some(e.aksara),
                byte_index: None,
                reason: e.message(lang),
            })
            .collect::<Vec<_>>()
    })?;

    let (text, pending) = encode::encode_object_for(&program, target).map_err(|es| {
        es.into_iter()
            .map(|e| Diagnostic {
                line: e.line,
                aksara_index: None,
                byte_index: None,
                reason: e.message(lang),
            })
            .collect::<Vec<_>>()
    })?;

    let bytes = kosha::object(
        &text,
        &program,
        &pending,
        None, // No debug info
        &encode::layout_addresses(&program, target),
    );

    let obj = vastu::read(&bytes).ok_or_else(|| {
        vec![Diagnostic {
            line: 0,
            aksara_index: None,
            byte_index: None,
            reason: "Internal error: generated object does not read back".into(),
        }]
    })?;

    let image = samyojana::link_at(&[obj], load_addr).map_err(|es| {
        es.into_iter()
            .map(|e| Diagnostic {
                line: 0,
                aksara_index: None,
                byte_index: None,
                reason: e,
            })
            .collect::<Vec<_>>()
    })?;

    let out_bytes = kosha::write_debuggable_at(
        &image.text,
        &image.data,
        &image.table,
        image.bss,
        &[],
        load_addr,
    );

    Ok(out_bytes)
}

/// Assembles a single source file into an object file.
pub fn assemble_object(
    source: &str,
    name: Option<&str>,
    target: encode::Target,
    debug: bool,
    lang: nidana::Language,
) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let tokens = lex::lex(source).map_err(|es| {
        es.into_iter()
            .map(|e| Diagnostic {
                line: e.line,
                aksara_index: Some(e.index),
                byte_index: Some(e.byte),
                reason: e.reason,
            })
            .collect::<Vec<_>>()
    })?;

    let program = parse::parse(&tokens).map_err(|es| {
        es.into_iter()
            .map(|e| Diagnostic {
                line: e.line,
                aksara_index: Some(e.aksara),
                byte_index: None,
                reason: e.message(lang),
            })
            .collect::<Vec<_>>()
    })?;

    let (text, pending) = encode::encode_object_for(&program, target).map_err(|es| {
        es.into_iter()
            .map(|e| Diagnostic {
                line: e.line,
                aksara_index: None,
                byte_index: None,
                reason: e.message(lang),
            })
            .collect::<Vec<_>>()
    })?;

    let bytes = kosha::object(
        &text,
        &program,
        &pending,
        debug.then_some(name.unwrap_or("")),
        &encode::layout_addresses(&program, target),
    );
    Ok(bytes)
}

/// Links previously assembled objects into an ELF image.
pub fn link_objects(objects: &[vastu::Object], load_addr: u64) -> Result<Vec<u8>, Vec<String>> {
    let image = samyojana::link_at(objects, load_addr)?;
    let debug: Vec<(&str, Vec<u8>)> = image
        .debug
        .iter()
        .map(|(n, b)| (n.as_str(), b.clone()))
        .collect();

    let bytes = kosha::write_debuggable_at(
        &image.text,
        &image.data,
        &image.table,
        image.bss,
        &debug,
        load_addr,
    );
    Ok(bytes)
}
