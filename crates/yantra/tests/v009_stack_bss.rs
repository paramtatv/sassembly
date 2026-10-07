//! **`V-009` part (i-b2): THE 64 KiB STARTUP STACK IS RESERVED IN `.bss`, NOT
//! STORED IN THE FILE.**
//!
//! Owner ruling 2026-10-05: "yes, .bss. Reserve the 64 KiB startup stack as
//! uninitialized memory (.bss) across all compiled binaries."
//!
//! Until this row the startup object reserved `स्तूपः … स्तूपान्तः` with
//! `॥ स्थानम् ६५५३६ ॥` in `ॱदत्त`, so every compiled image FILE carried 65,536
//! zero octets of stack. It now sits in `ॱरिक्त` (`.bss`): `p_memsz` covers it,
//! `p_filesz` does not, and every loader in the path zero-fills the difference
//! (`yantra::Machine::load_elf_spanning` allocates RAM zeroed and copies only
//! `p_filesz`; `loader::load_application` zeroes every frame it takes; QEMU's
//! `-kernel` ELF loader zero-fills `p_memsz` beyond `p_filesz`).
//!
//! **WHERE IT GOES, AND WHY THERE.** At the START of the image's `.bss`: the
//! startup object is linked first, so its `.bss` is the image's first, and in it
//! the stack comes BEFORE the record region `रचनाक्षेत्रम्` (the 512 MiB heap,
//! when the image allocates). So the stack is the 64 KiB immediately after the
//! file-backed data — inside any RAM budget that holds the file at all — and the
//! heap begins at `स्तूपान्तः`, the stack's top, growing UP while the stack grows
//! DOWN away from it. Put after the region, the stack would sit 512 MiB up and
//! every `Span::FileBacked` budget (`W-363`, the browser's) would halt on the
//! first push.
//!
//! **THE TOP STAYS 16-ALIGNED.** In `.data` the stack's top was 16-aligned
//! because `.data` starts on a page. `.bss` starts wherever the data ends,
//! eight-aligned, so the linker now starts `.bss` on SIXTEEN and folds the pad
//! into the reported `.bss` length (both linkers, `samyojana.rs` and
//! `samyojana.t1`), keeping `sp` on the RISC-V ABI's 16.
//!
//! **THE STARTUP STILL BEGINS `auipc sp` / `addi sp, sp`**, which is what W-376's
//! thread host decodes at `e_entry` (opcode, `rd`, `funct3`, `rs1` — never the
//! immediates); those two words now address the `.bss` top.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::{Halt, Machine, Span};

mod qemu_leg;

const FUEL: u64 = 80_000_000_000;
const MODULE: &str = "स्तूपपरीक्षण";
const LOAD: u64 = 0x8000_0000;
/// `riscv64.rs`'s `STACK_BYTES` / `yantrotsarjana.t1`'s `यन्त्रस्तूपाष्टकाः`.
const STACK: u64 = 65_536;
/// The recursion's depth. Each level's frame is 48 octets as the emitter lays
/// it out today (MEASURED: 4,896 octets of `sp` travel at depth 100 = 48 × 100
/// plus the entry's and the printer's 96), so 1,300 levels take 62,496 octets —
/// 95% of the stack. At 3,800 (the first guess, which assumed 16-octet frames)
/// the recursion ran off the stack's bottom INTO THE TEXT below it and yantra
/// halted `Unimplemented` on an overwritten instruction — on the old `.data`
/// layout and the new `.bss` one alike, since the stack's bottom is the data
/// page's start in both when the data is empty. Nothing guards that edge; this
/// row does not add a guard.
const DEPTH: u64 = 1_300;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

/// A decimal numeral for a small integer, generated, never typed.
fn dec(n: u64) -> String {
    const DIGITS: [&str; 10] = ["०", "१", "२", "३", "४", "५", "६", "७", "८", "९"];
    n.to_string()
        .chars()
        .map(|c| DIGITS[c.to_digit(10).unwrap() as usize])
        .collect()
}

/// The probe: a recursive sum `1 + … + n` to `depth`, printed as eight
/// little-endian octets (so yantra's and QEMU's UARTs can be compared) and
/// answered `०` when right, `१` when not.
fn probe(depth: u64) -> String {
    let sum = depth * (depth + 1) / 2;
    format!(
        "मण्डलम् {MODULE} ॥
आयातः अष्टक ।

वृत्तिः मुद्रणम् आदाय मूल्यम् ॱॱ न६४ ददाति न६४ आदि
    चरः क्रमः ॱॱ न६४ भवति ० ।
    चरः अवगणना ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् ८ आदि
        चरः सरणम् ॱॱ न६४ भवति क्रमः गुणनम् ८ ।
        चरः सृतम् ॱॱ न६४ भवति मूल्यम् दक्षिणसृ सरणम् ।
        चरः अष्टकम् ॱॱ न६४ भवति सृतम् युक् २५५ ।
        अवगणना भवति अष्टकॱमुद्रणम् अष्टकम् ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    प्रत्यागमनम् ० ।
इति

वृत्तिः अवरोहणम् आदाय क ॱॱ न६४ ददाति न६४ आदि
    यदि क समम् ० आदि
        प्रत्यागमनम् ० ।
    इति
    चरः पूर्वः ॱॱ न६४ भवति क वियोगः १ ।
    चरः अनुफलम् ॱॱ न६४ भवति अवरोहणम् पूर्वः ।
    प्रत्यागमनम् क योगः अनुफलम् ।
इति

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः फलम् ॱॱ न६४ भवति अवरोहणम् {depth} ।
    चरः अवगणना ॱॱ न६४ भवति मुद्रणम् फलम् ।
    यदि फलम् समम् {sum} आदि
        प्रत्यागमनम् ० ।
    इति
    प्रत्यागमनम् १ ।
इति
",
        depth = dec(depth),
        sum = dec(sum),
    )
}

/// The image the `.t1` compiler builds, running in the interpreter (`CHAIN`).
fn t1_image(src: &str) -> Vec<u8> {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets(MODULE.as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    let image = it
        .call(
            "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
            vec![
                arena(vec![octets(src.as_bytes())]),
                arena(vec![octets(MODULE.as_bytes())]),
                Value::Int(1),
            ],
            FUEL,
        )
        .expect("मण्डलानिप्रतिबिम्बम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(
        !image.is_empty(),
        "the probe built no image: refusal {:?}, link {:?}",
        sadhana::t1::chain::refusal_site(&it),
        sadhana::t1::chain::link_refusals(&it)
    );
    image
}

/// THE RUST TWIN, as `v005_floats.rs` builds it — with `records` forced when
/// asked, so the record region is linked beside the stack and its address can be
/// read off the linker's symbols. Answers the image and those symbols.
fn rust_twin(src: &str, force_records: bool) -> (Vec<u8>, std::collections::BTreeMap<String, u64>) {
    use sadhana::encode::Target;
    use sadhana::nidana::Language;
    use sadhana::t1::chain::Front;
    use sadhana::t1::riscv64;
    use sadhana::{assemble_object, vastu};

    let mut front = Front::load(&spec_root()).expect("Front loads");
    front.lex(src).expect("lex");
    front.parse().expect("parse");
    front.resolve().expect("resolve");
    front.typecheck().expect("typecheck");
    front.build_ir().expect("build_ir");
    let module = front
        .module(MODULE, Some("मुख्यम्"))
        .expect("the module builds");
    let module_text = riscv64::emit_module(&module).expect("the Rust emitter emits");
    let startup_text = riscv64::emit_startup_object_with_records(
        Some(&format!("{MODULE}मुख्यम्")),
        force_records || riscv64::module_allocates(&module),
    );
    let to_object = |text: &str, name: Option<&str>| -> vastu::Object {
        let bytes = assemble_object(text, name, Target::Uncompressed, false, Language::English)
            .unwrap_or_else(|ds| {
                panic!("{name:?} does not assemble: {ds:?}\n--- text ---\n{text}")
            });
        vastu::read(&bytes).unwrap_or_else(|| panic!("{name:?} does not read back"))
    };
    let startup = to_object(&startup_text, Some("यन्त्रारम्भ"));
    let module_obj = to_object(&module_text, Some(MODULE));
    let linked = sadhana::samyojana::link_at(&[startup, module_obj], LOAD)
        .unwrap_or_else(|es| panic!("the Rust path does not link: {es:?}"));
    let image =
        sadhana::kosha::write_debuggable_at(&linked.text, &linked.data, &[], linked.bss, &[], LOAD);
    (image, linked.symbols)
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// One `PT_LOAD`: `(p_flags, p_vaddr, p_filesz, p_memsz)`.
#[derive(Debug, Clone, Copy)]
struct Load {
    flags: u32,
    vaddr: u64,
    filesz: u64,
    memsz: u64,
}

fn loads(img: &[u8]) -> Vec<Load> {
    let phoff = u64_at(img, 32) as usize;
    let phnum = u16::from_le_bytes([img[56], img[57]]) as usize;
    (0..phnum)
        .map(|i| phoff + i * 56)
        .filter(|&o| u32_at(img, o) == 1)
        .map(|o| Load {
            flags: u32_at(img, o + 4),
            vaddr: u64_at(img, o + 16),
            filesz: u64_at(img, o + 32),
            memsz: u64_at(img, o + 40),
        })
        .collect()
}

/// The stack's TOP as the startup addresses it: `auipc sp, hi` then
/// `addi sp, sp, lo` at `e_entry` — decoded exactly as W-376's host decodes it.
fn startup_sp(img: &[u8]) -> u64 {
    let entry = u64_at(img, 24);
    let text = loads(img)[0];
    assert_eq!(
        text.vaddr, entry,
        "the startup is the image's first instruction"
    );
    let off = {
        let phoff = u64_at(img, 32) as usize;
        u64_at(img, phoff + 8) as usize
    };
    let (w0, w1) = (u32_at(img, off), u32_at(img, off + 4));
    assert_eq!(w0 & 0xfff, 0x117, "word 0 is `auipc sp`: {w0:#010x}");
    assert_eq!(
        w1 & 0xf_ffff,
        0x1_0113,
        "word 1 is `addi sp, sp`: {w1:#010x}"
    );
    let hi = i64::from((w0 & 0xffff_f000) as i32);
    let lo = i64::from((w1 as i32) >> 20);
    entry.wrapping_add(hi as u64).wrapping_add(lo as u64)
}

/// THE LAYOUT CLAIM, on one image: the stack is wholly in `.bss` — above every
/// file-backed octet, below the end of `p_memsz` — and its top is 16-aligned.
fn assert_stack_in_bss(engine: &str, img: &[u8]) -> (Load, u64) {
    let ls = loads(img);
    assert_eq!(
        ls.len(),
        2,
        "{engine}: text R+X, then data+bss R+W (W-363): {ls:?}"
    );
    let data = ls[1];
    assert_eq!(data.flags, 0b110, "{engine}: the second PT_LOAD is R+W");
    let top = startup_sp(img);
    let bottom = top - STACK;
    println!(
        "METRIC v009b2_{engine}_layout file {} data_filesz {} data_memsz {} stack {:#x}..{:#x}",
        img.len(),
        data.filesz,
        data.memsz,
        bottom,
        top
    );
    assert!(
        bottom >= data.vaddr + data.filesz,
        "{engine}: the stack {bottom:#x}..{top:#x} overlaps the FILE-BACKED data, which \
         ends at {:#x} — the stack is still stored in the image file",
        data.vaddr + data.filesz
    );
    assert!(
        top <= data.vaddr + data.memsz,
        "{engine}: p_memsz ends at {:#x}, below the stack's top {top:#x}",
        data.vaddr + data.memsz
    );
    assert_eq!(top % 16, 0, "{engine}: sp {top:#x} is not 16-aligned");
    (data, top)
}

/// **(a) THE FILE NO LONGER CARRIES THE STACK.** Both emitters' images of the
/// probe: the stack lies wholly above `p_filesz` and inside `p_memsz`, and the
/// FILE sizes are pinned.
///
/// THE PINS, and why each is what it is. The probe has no constant pool and no
/// global, so before this row its whole data segment was the stack: `p_filesz`
/// = `p_memsz` = 65,536, files 66,208 (`.t1` chain) and 66,488 (Rust twin)
/// octets (MEASURED on d8780f27, the RED commit, at this depth). Now:
/// * `data_filesz` 0 — the startup object stores nothing and the module has no
///   `.data`, so the image's data segment is pure `.bss`;
/// * `data_memsz` 69,632 — the 4 KiB guard (finding 6) and the stack; the
///   data is empty, so `.bss` starts on the data page itself, 16-aligned, with
///   no pad;
/// * the files 720 and 1,000 octets: down by exactly 65,536 for the stack (672
///   and 952 before the canary), then up 48 — the canary's thirteen
///   instructions (seven arming it only when its slot is zero, six checking
///   it; 52 octets) less the 4 octets of pad the startup object's
///   17-instruction text used to need to reach the eight the next object
///   starts on (30 instructions need none).
///
/// The two engines' FILES differ in their section headers by design (`W-236`
/// compares the loadable segment and `e_entry`), so the pins are per engine.
#[test]
fn v009b2_the_image_file_excludes_the_stack_and_p_memsz_covers_it() {
    let src = probe(DEPTH);
    let t1 = t1_image(&src);
    let (rust, _) = rust_twin(&src, false);
    // Both files' sizes first, so a red run reports the two engines' figures.
    for (engine, img) in [("t1", &t1), ("rust", &rust)] {
        println!("METRIC v009b2_{engine}_file {}", img.len());
    }
    let (t1_data, t1_top) = assert_stack_in_bss("t1", &t1);
    let (rs_data, rs_top) = assert_stack_in_bss("rust", &rust);
    assert_eq!(
        t1_top, rs_top,
        "the two emitters put the stack's top in one place"
    );
    for (engine, data, len, want_len) in [
        ("t1", t1_data, t1.len(), T1_FILE),
        ("rust", rs_data, rust.len(), RUST_FILE),
    ] {
        assert_eq!(data.filesz, DATA_FILESZ, "{engine}: data p_filesz");
        assert_eq!(data.memsz, DATA_MEMSZ, "{engine}: data p_memsz");
        assert_eq!(len, want_len, "{engine}: the image file's octets");
    }
}

const DATA_FILESZ: u64 = 0;
const DATA_MEMSZ: u64 = GUARD + STACK;
const T1_FILE: usize = 66_208 - 65_536 + 48;
const RUST_FILE: usize = 66_488 - 65_536 + 48;
/// The `.bss` guard below the stack (finding 6).
const GUARD: u64 = 4_096;

/// **THE STACK BELOW THE HEAP.** With the record region linked (Rust twin,
/// `records` forced), `रचनाक्षेत्रम्` — the heap's start — is the stack's top:
/// the stack occupies the first 64 KiB of `.bss` and the heap the 512 MiB after
/// it, so the stack is inside any budget that holds the file plus 64 KiB.
#[test]
fn v009b2_the_stack_sits_between_the_data_and_the_heap() {
    let src = probe(DEPTH);
    let (img, symbols) = rust_twin(&src, true);
    let (data, top) = assert_stack_in_bss("rust_records", &img);
    let region = symbols["रचनाक्षेत्रम्"];
    let stack = symbols["स्तूपः"];
    let stack_end = symbols["स्तूपान्तः"];
    assert_eq!(stack_end, top, "the startup addresses स्तूपान्तः");
    assert_eq!(stack + STACK, stack_end, "स्तूपः … स्तूपान्तः is 64 KiB");
    assert_eq!(
        region, stack_end,
        "the heap begins where the stack's top is"
    );
    // THE PAD, PINNED: the data is the cursor `रचनासूचकः` alone, 8 octets, so
    // the data ends 8 past its page and `.bss` — the stack — starts 16 past it,
    // the 8 between counted into `p_memsz` as `.bss` by the linker.
    assert_eq!(data.filesz, 8, "the data is the record cursor alone");
    assert_eq!(
        stack,
        data.vaddr + 16 + GUARD,
        "the guard then the stack are the FIRST things in .bss, at the data's end rounded to 16"
    );
    assert_eq!(
        data.memsz,
        8 + 8 + GUARD + STACK + (1 << 29),
        "cursor, pad, guard, stack, region"
    );
    // The FILE: 66,576 octets before this row (MEASURED on d8780f27: the
    // cursor, the startup's own pad to 16 in `.data`, and the stack, all
    // stored), now 65,544 fewer — the stack and that in-file pad.
    // Plus 48 for the canary: thirteen instructions, less the startup's old pad.
    assert_eq!(img.len(), 66_576 - 65_544 + 48, "the records image's file");
    assert_eq!(
        data.vaddr + data.memsz,
        region + (1 << 29),
        "p_memsz ends at the 512 MiB region's end"
    );
}

/// Run `img` on yantra one instruction at a time, keeping the lowest `sp`.
fn run_tracking_sp(engine: &str, mut m: Machine) -> (Halt, Vec<u8>, u64) {
    let mut out: Vec<u8> = Vec::new();
    let mut low = u64::MAX;
    for _ in 0..50_000_000u64 {
        if let Some(h) = m.step(&mut out) {
            return (h, out, low);
        }
        low = low.min(m.x[2]);
    }
    panic!("{engine}: the probe did not halt in 50,000,000 steps");
}

fn finished_clean(engine: &str, h: &Halt) {
    match h {
        Halt::Finisher {
            status: Some(0), ..
        } => {}
        other => panic!("{engine}: the probe did not finish 0: {other:?}"),
    }
}

/// **(b) THE STACK STILL WORKS — ON YANTRA AND ON QEMU.** A recursion 1,300 deep
/// uses 95% of the 64 KiB; both emitters' images run it on yantra (the
/// lowest `sp` checked to lie inside the `.bss` stack after over 90% of it is used)
/// and on `qemu-system-riscv64 -bios none`, printing the same eight octets.
/// And under `Span::FileBacked` with RAM cut to EXACTLY the stack's top — no
/// heap, no headroom — it still runs: the stack is the first thing past the file.
#[test]
fn v009b2_deep_recursion_runs_on_the_bss_stack_on_yantra_and_qemu() {
    let src = probe(DEPTH);
    let want = (DEPTH * (DEPTH + 1) / 2).to_le_bytes().to_vec();
    for (engine, img) in [("t1", t1_image(&src)), ("rust", rust_twin(&src, false).0)] {
        let top = startup_sp(&img);
        let m = Machine::load_elf(&img, yantra::ram_for(&img)).expect("the image loads");
        let (h, ours, low) = run_tracking_sp(engine, m);
        finished_clean(engine, &h);
        assert_eq!(ours, want, "{engine}: yantra printed the wrong sum");
        let used = top - low;
        println!("METRIC v009b2_{engine}_stack_used {used} of {STACK}");
        assert!(low >= top - STACK, "{engine}: sp {low:#x} left the stack");
        assert!(
            used > STACK * 9 / 10,
            "{engine}: the recursion used only {used} octets — not a deep test"
        );

        // RAM = exactly up to the stack's top, under the file-backed span.
        let tight = usize::try_from(top - LOAD).unwrap();
        let m = Machine::load_elf_spanning(&img, tight, Span::FileBacked)
            .expect("the file fits below the stack");
        let (h, tight_out, _) = run_tracking_sp(engine, m);
        finished_clean(&format!("{engine} at RAM {tight}"), &h);
        assert_eq!(
            tight_out, want,
            "{engine}: the tight-RAM run printed the wrong sum"
        );

        let theirs = qemu_leg::run(&img).unwrap_or_else(|e| panic!("{engine} on qemu: {e}"));
        assert_eq!(
            theirs, ours,
            "{engine}: qemu and yantra printed different octets"
        );
    }
}

/// THE CANARY'S STATUS (coordinator's finding 6, 2026-10-06): the startup writes
/// the stack's bottom address INTO the stack's bottom word, and the epilogue,
/// before the finisher store, halts with this status instead of the program's
/// when that word has changed. A sifive-test FAILURE form, `0x3333 | n << 16`,
/// so QEMU exits on it as yantra does — as, since `W-381`, the lowering's
/// refusals do too (`0x355`, `0x359`, `0x35a` in the same form; they were raw
/// words, which QEMU ignored and ran past).
/// `n = 0x353B`: the store is 32 bits wide, so `n` has 16; QEMU's process
/// status is its low octet, `0x3B` = 59. QEMU's exit codes (that low octet) and
/// yantra-run's own exit codes (1, 64, 66, 75, 77, 81..88) are SEPARATE
/// NAMESPACES — yantra-run maps a non-zero finisher status to 1, never to `n`
/// (`crates/yantra/src/smp.rs`'s `exit_code`) — so `0x355`'s QEMU exit 85 beside
/// S5's 85 names nothing twice. 59 was chosen clear of both anyway. A PROGRAM
/// returning `0x353B` reads the same — the status space is the program's, so no
/// value is distinct from every return; this one is distinct from every code
/// the tree defines.
const CANARY_STATUS: u64 = 0x353B;

/// Deep enough to run off the stack: 48 × 1,400 + 96 = 67,296 octets of `sp`
/// travel, 1,760 past the bottom — inside the 4 KiB guard below it.
const OVERFLOW_DEPTH: u64 = 1_400;

/// **AN OVERFLOW IS DETECTED AT EXIT.** The 1,400-deep recursion overwrites the
/// canary; both emitters' images halt with [`CANARY_STATUS`] on yantra and exit
/// `0x3B` on QEMU — never `0`, never the program's sum. Detection at exit, not
/// prevention: the overflow has already written the guard when it is reported.
#[test]
fn v009b2_an_overflow_past_the_stack_bottom_halts_with_the_canary_status() {
    let src = probe(OVERFLOW_DEPTH);
    for (engine, img) in [("t1", t1_image(&src)), ("rust", rust_twin(&src, false).0)] {
        let mut m = Machine::load_elf(&img, yantra::ram_for(&img)).expect("the image loads");
        let mut out: Vec<u8> = Vec::new();
        let h = m.run(200_000_000, &mut out);
        match h {
            Halt::Finisher {
                value,
                status: Some(s),
            } if s == CANARY_STATUS => {
                assert_eq!(value, 0x353B_3333, "{engine}: the finisher word");
            }
            other => panic!("{engine}: an overflow must halt {CANARY_STATUS:#x}, got {other:?}"),
        }
        match qemu_leg::run(&img) {
            Err(e) if e.contains("exit status: 59") => {}
            other => panic!("{engine} on qemu: an overflow must exit 59, got {other:?}"),
        }
    }
}
