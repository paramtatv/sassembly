//! **`W-296` — A CROSS-MODULE GLOBAL READ NEEDS ITS SYMBOL NAMED, AND THE RUST
//! `Front` WAS THE ONE READER OF THREE THAT NEVER NAMED IT.**
//!
//! `6e3e03cc` (2026-09-13, *"the lowering half of वास्तु"*) stopped `ir.t1`
//! refusing a cross-module global read and lowered it to `वैश्विकाज्ञाभेद`
//! carrying the RESOLVER's symbol. The emitter turns a symbol into a label only
//! through the module's name table, so that landing had to fill the table in
//! every driver — and it filled TWO of the three:
//!
//! ```text
//!   shrinkhala.t1:449-470          the .t1 driver's नामसञ्चयः      filled
//!   tests/paradigm_encode.rs:1164  the census twin                 filled
//!   sadhana/src/t1/chain.rs        Front — t1_image, t1_boot, every probe   NOT
//! ```
//!
//! **THE CENSUS READ `twin AGREE` THE WHOLE TIME**, because the census drives the
//! first two and never the third. *A pair of twins that agree is not a statement
//! about a third reader.* The product path refused instead:
//!
//! ```text
//!   emit परीक्षा: UnnamedSymbol { symbol: SymbolId(4) }
//! ```
//!
//! # WHY THE LOOPS THAT WERE THERE COULD NOT HAVE CAUGHT IT
//!
//! `chain.rs` names globals by walking `वैश्विकमण्डलकोश`/`वैश्विकनामकोश` — **the
//! globals THIS module declares.** A foreign global is in neither arena, so there
//! is no entry to miss however carefully the loop is written; the symbol has to be
//! SPELLED from the resolver's registry (`अर्थॱसञ्चयसंज्ञाप्रविष्टयः` beside
//! `अर्थॱसञ्चयसंज्ञामूल्यानि`). This is `W-295`'s cause A one module over, and it
//! is the third time this shape has been paid for.
//!
//! # WHAT THIS FILE HOLDS
//!
//! * **THE LABEL AND NOT THE NUMBER.** The entry module's name table must map the
//!   loaded symbol to `(अन्यत्, मान)` — the declaring module and the member — so a
//!   renumbering in the resolver moves the reading with it rather than silencing
//!   it. The symbol is found BY SEARCHING the emitted IR for the `LoadGlobal`,
//!   never assumed to be `SymbolId(4)`.
//! * **THE CASE THAT MUST STILL BE REFUSED**, as a MUTATION of the product's own
//!   output rather than a fixture: that one entry is removed from the table the
//!   driver just built, and `emit_module` must refuse the module by name. A repair
//!   that labelled anything unknown would link a foreign read against whatever
//!   answered, which is `W-279`'s `परीक्षासंज्ञा४` against `अन्यत्संज्ञा४`.
//!
//! **WHAT IT DELIBERATELY DOES NOT HOLD: THE LINK AND THE RUN.** That the read
//! answers `77` is asserted by `t1_shared_arena.rs`'s
//! `measure_a_cross_module_global_scalar_against_its_own_module`, which already
//! carries the assemble-link-run harness AND the same-module control that makes a
//! 77 mean "the reader reached the declarer's storage" rather than "both arms read
//! the same module". Copying a hundred lines of that harness here would give two
//! readings that drift rather than two that agree. **This file's subject is the
//! NAME TABLE**, which is the thing no halt assertion can report: a link that
//! refuses and a symbol that was never labelled arrive at the same silence.
//!
//! **ITS OWN LOADER.** `t1_shared_arena.rs` carries the same two-pass harness and
//! this file does NOT import it: a loader is part of the test, and a shared one
//! turns two readings into one. The two passes are not a style — `व्याकरॱकार्यक्रमपठनम्`
//! resets `घोषणासूचकाङ्क`, so a single pass builds IR for the LAST source alone
//! and the other module's object exports nothing.

use sadhana::t1::ast::SymbolId;
use sadhana::t1::chain::Front;
use sadhana::t1::riscv64;
use std::path::{Path, PathBuf};

/// The declaring module: one global and one routine. **The routine is required,
/// not decoration** — `build_ir` refuses a source that declares none, so a
/// globals-only module never produces an object and nothing lays its storage.
const DECLARER: &str = "मण्डलम् अन्यत् ॥\n\
     सार्वजनिक चरः मान ॱॱ न६४ भवति ७७ ।\n\
     सार्वजनिक वृत्तिः अकर्म ददाति न६४ आदि\n    प्रत्यागमनम् ० ।\nइति\n";

/// The reader: its whole body is the one cross-module read.
const READER: &str = "मण्डलम् परीक्षा ॥\n\
     आयातः अन्यत् ।\n\
     सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n    प्रत्यागमनम् अन्यत्ॱमान ।\nइति\n";

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// Compile both sources through one `Front` and hand back `(declarer, reader)`.
///
/// PASS 1 lexes and parses every source so both modules' declarations are in
/// `घोषणासञ्चय` before anything resolves; PASS 2 re-lexes each source and takes
/// its module immediately, because the arenas hold one program at a time.
fn compile_pair() -> Result<(riscv64::Module, riscv64::Module), String> {
    let mut front = Front::load(&spec_root()).map_err(|e| format!("front: {e}"))?;
    for src in [DECLARER, READER] {
        front.lex(src).map_err(|e| format!("lex: {e}"))?;
        front.parse().map_err(|e| format!("parse: {e}"))?;
    }
    let mut out = Vec::new();
    for (src, name, entry) in [(DECLARER, "अन्यत्", None), (READER, "परीक्षा", Some("मुख्यम्"))]
    {
        front.lex(src).map_err(|e| format!("{name} re-lex: {e}"))?;
        front.parse().map_err(|e| format!("{name} re-parse: {e}"))?;
        front
            .resolve()
            .map_err(|e| format!("{name} resolve: {e}"))?;
        front
            .typecheck()
            .map_err(|e| format!("{name} typecheck: {e}"))?;
        front.build_ir().map_err(|e| format!("{name} ir: {e}"))?;
        out.push(
            front
                .module(name, entry)
                .map_err(|e| format!("module {name}: {e}"))?,
        );
    }
    let reader = out.pop().expect("two modules were compiled");
    let declarer = out.pop().expect("two modules were compiled");
    Ok((declarer, reader))
}

/// Every symbol the reader's IR loads or addresses as a global, in order.
fn global_symbols(m: &riscv64::Module) -> Vec<SymbolId> {
    use sadhana::t1::ir::Instruction;
    let mut out = Vec::new();
    for f in &m.functions {
        for b in f.blocks.values() {
            for (_, i) in &b.insts {
                match i {
                    Instruction::LoadGlobal(s) | Instruction::AddrOfGlobal(s) => out.push(*s),
                    _ => {}
                }
            }
        }
    }
    out
}

/// **THE NAME TABLE CARRIES THE DECLARING MODULE AND THE MEMBER.**
///
/// Asserted as a LABEL, not as an id: the pair `(अन्यत्, मान)` is what the linker
/// resolves against the declaring object's exported `अन्यत्मान`, and it is the
/// only thing that stays true across a resolver renumbering.
#[test]
fn the_reader_names_the_declaring_modules_global_and_still_refuses_an_unnamed_one() {
    let (declarer, reader) = compile_pair().expect("both modules compile");

    // The declaring object exports the storage exactly once, under that label.
    let labels: Vec<&str> = declarer
        .globals
        .iter()
        .map(|(l, _, _)| l.as_str())
        .collect();
    println!("METRIC w296_declarer_globals {labels:?}");
    assert!(
        labels.contains(&"अन्यत्मान"),
        "the declaring module must export `अन्यत्मान` for the read to resolve \
         against; it exports {labels:?}"
    );
    assert!(
        reader.globals.is_empty(),
        "the READER must declare no storage of its own — a second definition of \
         `अन्यत्मान` would be resolved by link order and the value a program reads \
         would be decided by object sequence. It declares {:?}",
        reader.globals
    );

    // FOUND BY SEARCH. `SymbolId(4)` is what this measured on 2026-09-17 and it
    // is not what is asserted: a renumbering must move this reading, not mute it.
    let loaded = global_symbols(&reader);
    println!("METRIC w296_reader_global_symbols {loaded:?}");
    assert_eq!(
        loaded.len(),
        1,
        "the reader's whole body is ONE cross-module global read, so its IR must \
         carry exactly one global access and it carries {loaded:?}"
    );
    let sym = loaded[0];

    let named = reader.names.get(&sym).cloned();
    println!("METRIC w296_reader_name_for_loaded_symbol {named:?}");
    assert_eq!(
        named,
        Some(("अन्यत्".to_string(), "मान".to_string())),
        "the symbol the reader LOADS must be named (declaring module, member) so \
         the emitter writes the label the declaring object exports. {sym:?} is \
         named {named:?}. `None` is the W-296 defect itself: `chain.rs` walks \
         this module's own globals arenas, which a foreign global is not in, so \
         the registry loop is what spells it"
    );

    // It emits, with the concatenated label present in the text.
    let text = riscv64::emit_module(&reader).expect("the reader emits");
    assert!(
        text.contains("अन्यत्मान"),
        "the emitted text must reach the declaring object's exported label; it does not"
    );

    // ── THE CASE THAT MUST STILL BE REFUSED ──────────────────────────────
    //
    // A MUTATION of what the driver just built, not a fixture: drop the one entry
    // the registry loop adds and nothing else. The emitter must refuse the module
    // BY NAME. If this passes, the repair was "label anything unknown" — which
    // would resolve a foreign read against whatever object answered, and that is
    // `W-279`'s `परीक्षासंज्ञा४` linking against `अन्यत्संज्ञा४`.
    let mut stripped = reader;
    stripped.names.remove(&sym);
    let refusal = riscv64::emit_module(&stripped);
    println!("METRIC w296_emit_without_the_name {refusal:?}");
    let Err(r) = refusal else {
        panic!(
            "with {sym:?} removed from the name table the emitter ACCEPTED the \
             module. The label for a foreign global is then being invented \
             somewhere rather than read from the registry, and the W-296 loop is \
             not what makes this work"
        );
    };
    let said = format!("{r:?}");
    assert!(
        said.contains("UnnamedSymbol"),
        "an unnamed global symbol must be refused as `UnnamedSymbol` and naming \
         the offender; the emitter said {said}"
    );
    assert!(
        said.contains(&format!("{}", sym.0)),
        "the refusal must name WHICH symbol is unnamed — `W-279`'s finding about \
         the five refusal sites is that a record identical at every field \
         localises nothing. It said {said}, and the symbol is {sym:?}"
    );
}
