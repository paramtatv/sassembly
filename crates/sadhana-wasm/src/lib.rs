//! SADHANA compiled to wasm: the assembler's browser entry points.
#![allow(
    unsafe_code,
    reason = "no_mangle export attributes and nothing else — see the note above. There \
              is no unsafe block in this crate."
)]

use sadhana::{assemble, encode, nidana};
use std::sync::Mutex;

static INPUT: Mutex<String> = Mutex::new(String::new());
static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static ERROR: Mutex<String> = Mutex::new(String::new());

#[unsafe(no_mangle)]
/// The a function.
pub extern "C" fn sadhana_alloc(len: u32) -> u32 {
    let mut input = INPUT.lock().expect("single-threaded");
    input.clear();
    for _ in 0..len {
        input.push('\0');
    }
    input.as_mut_ptr() as u32
}

#[unsafe(no_mangle)]
/// Assemble the source written at [`sadhana_alloc`]'s pointer, linked at
/// `(base_hi << 32) | base_lo`. Answers 0 and fills `OUTPUT`, or 1 and fills
/// `ERROR`.
///
/// # Why the address is a parameter
///
/// It was `0x80200000`, hard-coded, and that is the QEMU `virt` reset vector —
/// right for a boot proof and wrong for a guest. An APPLICATION is linked at
/// `0x2000_0000` (`spec/application-load.tsv`), because `0x80200000` sits inside
/// the supervisor's own gigabyte and `yantra::host` refuses it by name rather
/// than quietly handing an application the supervisor's pages. With the address
/// fixed, this module could assemble bare-metal proofs and could not assemble an
/// application at all: the refusal arrived from the LOADER, one stage later,
/// reading as a bug in the program rather than in how it was linked.
///
/// Split across two `u32`s because a wasm export cannot take a `u64` and the
/// pair costs nothing; `tools/build-sassembly-web.sh` passes the same number to
/// the native CLI as `--स्थान`.
pub extern "C" fn sadhana_assemble(len: u32, base_lo: u32, base_hi: u32) -> u32 {
    let input = INPUT.lock().expect("single-threaded");
    let source = &input[..len as usize];

    let mut out = OUTPUT.lock().expect("single-threaded");
    let mut error = ERROR.lock().expect("single-threaded");
    out.clear();
    error.clear();

    let load_addr = (u64::from(base_hi) << 32) | u64::from(base_lo);

    match assemble(
        source,
        encode::Target::Uncompressed,
        load_addr,
        nidana::Language::English,
    ) {
        Ok(bytes) => {
            *out = bytes;
            0
        }
        Err(diags) => {
            let mut err_str = String::new();
            for d in diags {
                err_str.push_str(&format!(
                    "line {}, byte {}: {}\n",
                    d.line,
                    d.byte_index.unwrap_or(0),
                    d.reason
                ));
            }
            *error = err_str;
            1
        }
    }
}

#[unsafe(no_mangle)]
/// The a function.
pub extern "C" fn sadhana_out_ptr() -> u32 {
    OUTPUT.lock().expect("single-threaded").as_ptr() as u32
}

#[unsafe(no_mangle)]
/// The a function.
pub extern "C" fn sadhana_out_len() -> u32 {
    OUTPUT.lock().expect("single-threaded").len() as u32
}

#[unsafe(no_mangle)]
/// The a function.
pub extern "C" fn sadhana_err_ptr() -> u32 {
    ERROR.lock().expect("single-threaded").as_ptr() as u32
}

#[unsafe(no_mangle)]
/// The a function.
pub extern "C" fn sadhana_err_len() -> u32 {
    ERROR.lock().expect("single-threaded").len() as u32
}
