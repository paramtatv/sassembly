//! `F-015a` — the FROZEN gate check for the Sanskriti bridge.
//!
//! This test replays the pipeline deterministically (assemble -> link at two addresses ->
//! host) to assert the pipeline itself rather than the bridge. It explicitly checks that
//! the known application (`spec/atithi.sas`) is position-independent and satisfies A1-A5
//! at multiple addresses.

use std::path::{Path, PathBuf};

use sadhana::encode::{Target, encode_object, layout_addresses};
use sadhana::kosha::{object, write_debuggable_at};
use sadhana::parse::assemble_program;
use sadhana::samyojana::link_at;
use sadhana::vastu::read;
use yantra::host::host;
use yantra::supervisor::Ended;

/// The one program in `spec/` that is an application.
const APPLICATION: &str = "atithi.sas";

/// What it puts on its surface.
const STRING: &str = "अतिथिः";

/// 4 MiB RAM.
const RAM: usize = 1 << 22;

/// Ten instructions and two calls.
const BUDGET: u64 = 200;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/yantra has a grandparent")
        .to_path_buf()
}

/// Assemble and link `spec/atithi.sas` at `load` and return the ELF.
fn elf_at(load: u64) -> Vec<u8> {
    let source = std::fs::read_to_string(root().join("spec").join(APPLICATION)).expect("read");
    let program = assemble_program(&source).unwrap_or_else(|e| panic!("{APPLICATION}: {e:?}"));
    let (text, pending) =
        encode_object(&program).unwrap_or_else(|e| panic!("{APPLICATION}: {e:?}"));
    let bytes = object(
        &text,
        &program,
        &pending,
        None,
        &layout_addresses(&program, Target::Uncompressed), // Wait, default is compressed! Is uncompressed required here?
    );
    let objects = [read(&bytes).expect("the object reads back")];
    let image = link_at(&objects, load).unwrap_or_else(|e| panic!("{APPLICATION}: {e:?}"));
    write_debuggable_at(&image.text, &image.data, &image.table, image.bss, &[], load)
}

#[test]
fn the_pipeline_is_frozen_and_position_independent() {
    let addrs = [0x2000_0000, 0x4000_2000];

    for load in addrs {
        let elf = elf_at(load);
        let hosted = host(&elf, RAM, BUDGET).expect("the image loads");

        // Assert A1-A5 satisfaction
        assert_eq!(hosted.surface, STRING.as_bytes(), "failed at 0x{:x}", load);
        assert!(hosted.uart.is_empty(), "failed at 0x{:x}", load);

        match hosted.ended {
            Ended::Exited { status: 0 } => {}
            other => panic!("expected exit 0 at 0x{:x}, got {:?}", load, other),
        }
    }
}
