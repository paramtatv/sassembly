//! `W-302` — IDENTITY OF THE LOADED IMAGE, as ruled 2026-10-03 (`IDENTITY
//! RULED (a)` in the row): for each of the 48 `spec/*.sas` programs, the image
//! the `.t1` chain writes and the image Rust `sadhana` writes are equal in
//!
//! - the 64-octet ELF header EXCEPT the four section-table fields (`e_shoff`
//!   40..48, `e_shentsize` 58..60, `e_shnum` 60..62, `e_shstrndx` 62..64) —
//!   `e_flags` INCLUDED;
//! - every program header;
//! - every `PT_LOAD` segment's file bytes.
//!
//! EXCLUDED from the comparator, said plainly: the section and symbol tables.
//! `kosha.t1`'s writer emits none. The WRAPPER (`spec/entry/assemble.t1`,
//! W-302 option i) appends a minimal section table — `.text`, `.data`, `.bss`,
//! `.shstrtab`, no symbols — and the census sides that run it check those
//! headers against Rust's separately (`section_table_agrees`); an image the
//! wrapper links from OBJECT FILES also carries Rust's symbols, checked as a
//! set.
//!
//! Two parts:
//!
//! 1. **The comparator and its mutants** — ordinary tests, run on every
//!    `cargo test`. Each mutant must go red: one loaded byte, one
//!    program-header field, `e_flags`. The control — only the excluded fields
//!    changed — must stay identical. Two refusals a reviewer found missing:
//!    `e_phnum` ० on both sides is REFUSED (nothing was compared, so nothing
//!    is identical), and a MISSING output on either side is a FAILURE.
//! 2. **The census** — `#[ignore]`d, because it drives the `.t1` chain
//!    INTERPRETED over 48 programs at every address the check scripts pass
//!    (minutes in release). Run it with `tools/check-w302-loaded-identity.sh`,
//!    which builds Rust `sadhana`, hands its path in `W302_SADHANA`, and runs
//!
//!    ```text
//!    cargo test --release -p sadhana-t1 --test w302_loaded_identity -- \
//!        --ignored --exact the_48_programs_load_identically_from_both_assemblers --nocapture
//!    ```
//!
//! The census has TWO SIDES graded by the one comparator. INTERPRETED: the
//! chain's own routines driven in the Rust interpreter — the same `.t1` code:
//! `शृङ्खलाॱसङ्कलनारम्भः`, `शृङ्खलाॱपाठवस्तुरचना` on the text, then
//! `शृङ्खलाॱवस्तुप्रतिबिम्बम्` on the object with the load-address globals set.
//! NATIVE (`#[ignore]`d too, run by `tools/check-w302-native.sh`): the
//! SELF-HOSTED image — the corpus plus the entry module
//! `spec/entry/assemble.t1`, built by `t1_image --entry` — run under
//! `yantra-run` with the source on its input channel and the load address as
//! its argument, its stdout the image. Same sequence, compiled.
//!
//! Beside the 48, each side also builds the TWO-OBJECT LINK (`LINKED`:
//! `namaste-main` + `lib-mudraka`, an object per source then one link; the
//! native side takes the two sources as one input joined by NUL), graded
//! against BOTH of Rust's links — `sadhana a.sas b.sas` and `--वस्तु` per source
//! then `--संयोजय` — as `tools/check-link.sh` and `tools/check-separate.sh` run
//! them.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

const EHDR: usize = 64;
const PHDR: usize = 56;
const PT_LOAD: u32 = 1;

/// The ELF-header octets the ruling excludes: the section-table fields.
fn excluded(at: usize) -> bool {
    (40..48).contains(&at) || (58..64).contains(&at)
}

fn header_field(at: usize) -> &'static str {
    match at {
        0..=15 => "e_ident",
        16..=17 => "e_type",
        18..=19 => "e_machine",
        20..=23 => "e_version",
        24..=31 => "e_entry",
        32..=39 => "e_phoff",
        48..=51 => "e_flags",
        52..=53 => "e_ehsize",
        54..=55 => "e_phentsize",
        56..=57 => "e_phnum",
        _ => "excluded",
    }
}

const PH_FIELDS: [(&str, usize, usize); 8] = [
    ("p_type", 0, 4),
    ("p_flags", 4, 4),
    ("p_offset", 8, 8),
    ("p_vaddr", 16, 8),
    ("p_paddr", 24, 8),
    ("p_filesz", 32, 8),
    ("p_memsz", 40, 8),
    ("p_align", 48, 8),
];

fn le(b: &[u8], at: usize, size: usize) -> u64 {
    b[at..at + size]
        .iter()
        .rev()
        .fold(0u64, |acc, &x| (acc << 8) | u64::from(x))
}

/// What one comparison found.
#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    /// Equal under the ruling; carries how many `PT_LOAD`s were compared (≥ 1).
    Identical { loads: usize },
    /// A field or byte differs; the first cause.
    Differs(String),
    /// Nothing comparable: not an ELF, no program header, no `PT_LOAD`, a
    /// truncated table or segment. NEVER a match.
    Refused(String),
}

/// The comparator. `rust` and `t1` are whole image files.
fn compare(rust: &[u8], t1: &[u8]) -> Verdict {
    for (side, b) in [("rust", rust), ("t1", t1)] {
        if b.len() < EHDR || b[..4] != *b"\x7fELF" {
            return Verdict::Refused(format!("{side}: not an ELF file"));
        }
        // The walk below reads 64-bit little-endian headers of 56 octets; an
        // image that is not that shape is refused, not read wrongly.
        if b[4] != 2 || b[5] != 1 {
            return Verdict::Refused(format!("{side}: not ELF64 little-endian"));
        }
    }
    for at in (0..EHDR).filter(|&at| !excluded(at)) {
        if rust[at] != t1[at] {
            return Verdict::Differs(format!(
                "ELF header {} (octet {at}): rust {:#04x} t1 {:#04x}",
                header_field(at),
                rust[at],
                t1[at]
            ));
        }
    }
    // Equal from here on, so read once.
    let phnum = usize::try_from(le(rust, 56, 2)).expect("u16 fits");
    let phentsize = le(rust, 54, 2);
    let Ok(phoff) = usize::try_from(le(rust, 32, 8)) else {
        return Verdict::Refused("e_phoff does not fit".into());
    };
    if phnum == 0 {
        return Verdict::Refused("e_phnum is 0 on both sides: no program header to compare".into());
    }
    if phentsize != PHDR as u64 {
        return Verdict::Refused(format!("e_phentsize is {phentsize}, not {PHDR}"));
    }
    for k in 0..phnum {
        let at = phoff + PHDR * k;
        let (Some(a), Some(b)) = (rust.get(at..at + PHDR), t1.get(at..at + PHDR)) else {
            return Verdict::Refused(format!("program header {k} is truncated"));
        };
        for (name, off, size) in PH_FIELDS {
            if a[off..off + size] != b[off..off + size] {
                return Verdict::Differs(format!(
                    "program header {k} {name}: rust {:#x} t1 {:#x}",
                    le(a, off, size),
                    le(b, off, size)
                ));
            }
        }
    }
    let mut loads = 0;
    for k in 0..phnum {
        let at = phoff + PHDR * k;
        if le(rust, at, 4) != u64::from(PT_LOAD) {
            continue;
        }
        loads += 1;
        let (Ok(off), Ok(size)) = (
            usize::try_from(le(rust, at + 8, 8)),
            usize::try_from(le(rust, at + 32, 8)),
        ) else {
            return Verdict::Refused(format!("PT_LOAD {k} does not fit"));
        };
        let (Some(a), Some(b)) = (rust.get(off..off + size), t1.get(off..off + size)) else {
            return Verdict::Refused(format!(
                "PT_LOAD {k} is truncated: rust {} t1 {} of {off}+{size}",
                rust.len(),
                t1.len()
            ));
        };
        if a != b {
            let first = a.iter().zip(b).position(|(x, y)| x != y).unwrap_or(0);
            let n = a.iter().zip(b).filter(|(x, y)| x != y).count();
            return Verdict::Differs(format!(
                "PT_LOAD {k} byte {first:#x}: rust {:#04x} t1 {:#04x}; {n} octets differ",
                a[first], b[first]
            ));
        }
    }
    if loads == 0 {
        return Verdict::Refused("no PT_LOAD: no loaded byte was compared".into());
    }
    Verdict::Identical { loads }
}

/// The comparator over two output FILES. A missing file on either side is a
/// failure, never a match — a run that wrote nothing has not agreed.
fn compare_files(rust: &Path, t1: &Path) -> Verdict {
    match (std::fs::read(rust), std::fs::read(t1)) {
        (Ok(r), Ok(t)) => compare(&r, &t),
        (r, t) => Verdict::Refused(format!(
            "MISSING output: rust {} t1 {}",
            if r.is_ok() { "present" } else { "absent" },
            if t.is_ok() { "present" } else { "absent" }
        )),
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana-t1 has a grandparent")
        .to_path_buf()
}

/// The load addresses the `tools/check-*.sh` scripts pass for each program,
/// written as the scripts write them; `None` is a script that passes no
/// `--स्थान` (Rust's default, `kosha::LOAD_ADDRESS`). Read from the scripts
/// 2026-10-03: a program that any two-address script assembles is listed at
/// both; `atithi` is also linked at `spec/application-load.tsv`'s first row by
/// `check-roundtrip.sh`. An unlisted program is REFUSED by the census, not
/// run at a guessed address.
fn addresses(program: &str) -> Option<&'static [Option<&'static str>]> {
    const DEFAULT: &[Option<&str>] = &[None];
    const ONE: &[Option<&str>] = &[Some("०षोड्८०२०००००")];
    const TWO: &[Option<&str>] = &[Some("०षोड्८०२०००००"), Some("०षोड्८०४०००००")];
    const APPLICATION: &[Option<&str>] = &[None, Some("०षोड्२०००००००")];
    Some(match program {
        "bare-metal" | "jump-table" | "lib-mudraka" | "namaste" | "namaste-main" => DEFAULT,
        "atithi" => APPLICATION,
        "boot-sbi" | "fdt-header" | "virtio-blk" | "virtio-console" | "virtio-net" => ONE,
        "arena" | "boot-counter" | "bootloader" | "buddy" | "capability" | "compositor"
        | "context" | "crossing" | "freelist" | "higher-half" | "irq" | "journal" | "kernel"
        | "mapping" | "merge" | "milestone-k1" | "milestone-k2" | "network" | "notification"
        | "pager-handoff" | "pager" | "paging" | "powerbox" | "pty" | "root-task" | "schedule"
        | "second-space" | "split" | "stress-ram" | "stress" | "timer" | "trap-seam" | "trap"
        | "untyped" | "user-mode" | "virtio-gpu" | "virtio-sound" => TWO,
        _ => return None,
    })
}

/// Programs BOTH assemblers are expected to refuse, and why. `namaste-main`
/// calls a routine `lib-mudraka` defines; assembled alone it cannot link, on
/// either side. A refusal of any OTHER program is a failure, and an output
/// from one of these is a failure too (the expectation went stale).
const REFUSED_BY_BOTH: &[&str] = &["namaste-main"];

/// THE TWO-OBJECT LINK (`W-302`'s second part): programs that only exist as a
/// link of several sources, each assembled to an object and the objects linked
/// once. `namaste-main` calls `मुद्रकः`, which `lib-mudraka` exports. Rust
/// writes the reference two ways and BOTH must equal the `.t1` side:
/// `sadhana a.sas b.sas out` (`tools/check-link.sh`) and `sadhana --वस्तु` per
/// source then `--संयोजय` (`tools/check-separate.sh`). Both scripts link at
/// the default address.
const LINKED: &[(&str, &[&str])] =
    &[("namaste-main+lib-mudraka", &["namaste-main", "lib-mudraka"])];

/// The `.t1` chain's image of SEVERAL sources linked at `load`: an object per
/// source (`सङ्कलनारम्भः` before each, as `मण्डलानिप्रतिबिम्बम्` does), then ONE
/// `संयोजनॱसंयोजनम्` over the array and `कोशॱप्रतिबिम्बलेखनम्` — the body of
/// `वस्तुप्रतिबिम्बम्` with N members instead of one.
fn t1_link_or_names(srcs: &[Vec<u8>], load: u64) -> Result<Vec<u8>, (String, Vec<String>)> {
    let fuel = 4_000_000_000_000u64;
    let mut it = Interpreter::load(CHAIN, &repo_root().join("spec"))
        .map_err(|e| early(format!("the chain does not load: {e:?}")))?;
    let mut objects = Vec::new();
    for (k, src) in srcs.iter().enumerate() {
        it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], fuel)
            .map_err(|e| early(format!("FAULT in सङ्कलनारम्भः: {e:?}")))?;
        let object = it
            .call(
                "शृङ्खलाॱपाठवस्तुरचना",
                vec![Value::Octets(Octets::new(src))],
                fuel,
            )
            .map_err(|e| early(format!("FAULT in पाठवस्तुरचना: {e:?}")))?;
        if object.is_nil() {
            let exit = it
                .global("वस्तुरचनाविरामभेद")
                .and_then(Value::as_int)
                .unwrap_or(-1);
            return Err(early(format!(
                "REFUSED by पाठवस्तुरचना on source {k}, exit {exit}"
            )));
        }
        objects.push(object);
    }
    for g in ["भारणस्थानम्", "भारस्थानम्"] {
        if !it.set_global(g, Value::Int(i128::from(load))) {
            return Err(early(format!("`{g}` is not a global of the chain")));
        }
    }
    let linked = it
        .call(
            "संयोजनॱसंयोजनम्",
            vec![Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(
                objects,
            )))],
            fuel,
        )
        .map_err(|e| early(format!("FAULT in संयोजनम्: {e:?}")))?;
    let Value::Record(r) = &linked else {
        let names = link_names(&it);
        return Err((
            format!("REFUSED by संयोजनम्: link diagnostics {names:?}"),
            names,
        ));
    };
    let field = |k: &str| {
        r.borrow()
            .get(k)
            .cloned()
            .ok_or_else(|| early(format!("the linked record has no `{k}`")))
    };
    let (text, data, bss) = (field("पाठ्यम्")?, field("दत्तम्")?, field("बीजम्")?);
    let img = it
        .call(
            "कोशॱप्रतिबिम्बलेखनम्",
            vec![text, data, bss, Value::Int(i128::from(load))],
            fuel,
        )
        .map_err(|e| early(format!("FAULT in प्रतिबिम्बलेखनम्: {e:?}")))?;
    match img.octets() {
        Some(o) if !o.is_empty() => Ok(o.as_slice().to_vec()),
        _ => Err(early("प्रतिबिम्बलेखनम् wrote an EMPTY image".into())),
    }
}

/// The `.t1` chain's image of one source at `load`, or why it wrote none.
fn t1_image(src: &[u8], load: u64) -> Result<Vec<u8>, String> {
    t1_image_or_names(src, load).map_err(|(why, _)| why)
}

/// As [`t1_image`], and on a refusal also the names the linker recorded
/// diagnostics against (empty when the refusal came earlier).
fn t1_image_or_names(src: &[u8], load: u64) -> Result<Vec<u8>, (String, Vec<String>)> {
    let fuel = 4_000_000_000_000u64;
    let mut it = Interpreter::load(CHAIN, &repo_root().join("spec"))
        .map_err(|e| early(format!("the chain does not load: {e:?}")))?;
    it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], fuel)
        .map_err(|e| early(format!("FAULT in सङ्कलनारम्भः: {e:?}")))?;
    let object = it
        .call(
            "शृङ्खलाॱपाठवस्तुरचना",
            vec![Value::Octets(Octets::new(src))],
            fuel,
        )
        .map_err(|e| early(format!("FAULT in पाठवस्तुरचना: {e:?}")))?;
    if object.is_nil() {
        let exit = it
            .global("वस्तुरचनाविरामभेद")
            .and_then(Value::as_int)
            .unwrap_or(-1);
        return Err(early(format!("REFUSED by पाठवस्तुरचना, exit {exit}")));
    }
    for g in ["भारणस्थानम्", "भारस्थानम्"] {
        if !it.set_global(g, Value::Int(i128::from(load))) {
            return Err(early(format!("`{g}` is not a global of the chain")));
        }
    }
    let img = it
        .call("शृङ्खलाॱवस्तुप्रतिबिम्बम्", vec![object], fuel)
        .map_err(|e| early(format!("FAULT in वस्तुप्रतिबिम्बम्: {e:?}")))?;
    match img.octets() {
        Some(o) if !o.is_empty() => Ok(o.as_slice().to_vec()),
        _ => {
            let names = link_names(&it);
            Err((
                format!("REFUSED by वस्तुप्रतिबिम्बम्: link diagnostics {names:?}"),
                names,
            ))
        }
    }
}

fn early(why: String) -> (String, Vec<String>) {
    (why, Vec::new())
}

/// The names the chain's linker recorded diagnostics against.
fn link_names(it: &Interpreter) -> Vec<String> {
    let n = it
        .global("संयोजनदोषसूचकाङ्क")
        .and_then(Value::as_int)
        .unwrap_or(0);
    let Some(Value::Arena(a)) = it.global("संयोजनदोषकोश") else {
        return vec!["(no link diagnostics arena)".into()];
    };
    let a = a.borrow();
    (1..=usize::try_from(n).unwrap_or(0))
        .filter_map(|i| a.get(i))
        .map(|d| match d {
            Value::Record(r) => r
                .borrow()
                .get("नाम")
                .and_then(|v| {
                    v.octets()
                        .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
                })
                .unwrap_or_default(),
            other => format!("{other:?}"),
        })
        .collect()
}

// ---------------------------------------------------------------- the census

/// What the `.t1` side of one census run produced.
struct T1Run {
    /// The image, when one was written.
    image: Option<Vec<u8>>,
    /// What the side said, for the CENSUS line.
    said: String,
    /// Whether this is the SAME refusal Rust made, by the side's own evidence:
    /// interpreted, every name the chain's linker recorded a diagnostic against
    /// appears in Rust's message; native, the wrapper's status says the link
    /// refused (`२००० + n`, n ≥ १ diagnostics) — the image cannot hand over the
    /// names, so the native check is the weaker of the two and says so.
    same_refusal: bool,
    /// Interpreted only: the names the chain's linker recorded diagnostics
    /// against, each of which Rust's message must also name.
    names: Vec<String>,
}

/// Which `.t1` side the census grades against Rust.
enum Side {
    /// The chain's own routines, driven in the Rust interpreter.
    Interpreted,
    /// The SELF-HOSTED image `t1_image --entry पाठसेतुः मुख्यम्` built from the
    /// corpus plus `spec/entry/assemble.t1`, run natively under `yantra-run`:
    /// the source on the input channel, the load address as its one argument,
    /// the image on its stdout.
    Native { yantra: PathBuf, image: PathBuf },
    /// The wrapper module itself (`पाठसेतुःॱमुख्यम्`, mode ०) — interpreted or
    /// native — whose image now carries the minimal section table.
    Wrapper(Wrapper),
    /// THROUGH OBJECT FILES: the wrapper in mode १ writes each source's
    /// relocatable ELF, and in mode २ reads those files back and links them.
    Objects(Wrapper),
}

impl Side {
    /// Whether this side's images come from the wrapper, which appends the
    /// minimal section table (the interpreted chain's do not).
    fn writes_section_table(&self) -> bool {
        !matches!(self, Side::Interpreted)
    }

    /// One run over `srcs` (one source, or several to be linked) at `load`.
    fn run(&self, p: &str, srcs: &[PathBuf], out: &Path, load: u64) -> T1Run {
        match self {
            Side::Interpreted => {
                let texts: Vec<Vec<u8>> = srcs
                    .iter()
                    .map(|s| std::fs::read(s).expect("the source reads"))
                    .collect();
                let result = if texts.len() == 1 {
                    t1_image_or_names(&texts[0], load)
                } else {
                    t1_link_or_names(&texts, load)
                };
                match result {
                    Ok(img) => T1Run {
                        said: format!("image {} octets", img.len()),
                        image: Some(img),
                        same_refusal: false,
                        names: Vec::new(),
                    },
                    Err((why, names)) => T1Run {
                        image: None,
                        said: why,
                        same_refusal: false,
                        names: Vec::new(),
                    }
                    .with_names(names),
                }
            }
            // Several sources reach the image as ONE input, joined by NUL —
            // the separator `spec/entry/assemble.t1` splits on (NUL never
            // occurs in UTF-8 source, and a source holding one is refused).
            Side::Native { yantra, image } => {
                if let [one] = srcs {
                    return native_run(yantra, image, p, one, load);
                }
                let mut joined = Vec::new();
                for (k, s) in srcs.iter().enumerate() {
                    let b = std::fs::read(s).expect("the source reads");
                    assert!(!b.contains(&0), "{} holds a NUL octet", s.display());
                    if k > 0 {
                        joined.push(0);
                    }
                    joined.extend_from_slice(&b);
                }
                let input = out.join(format!("{p}.joined.sas"));
                std::fs::write(&input, &joined).expect("the joined input writes");
                native_run(yantra, image, p, &input, load)
            }
            Side::Wrapper(w) => {
                let mut joined = Vec::new();
                for (k, s) in srcs.iter().enumerate() {
                    if k > 0 {
                        joined.push(0);
                    }
                    joined.extend_from_slice(&std::fs::read(s).expect("the source reads"));
                }
                w.run(&[load, 0], &joined, out, p).into_run("image")
            }
            Side::Objects(w) => {
                let mut objects = Vec::new();
                for (k, s) in srcs.iter().enumerate() {
                    let src = std::fs::read(s).expect("the source reads");
                    let r = w.run(&[load, 1], &src, out, &format!("{p}.{k}"));
                    if r.status != Some(0) || r.out.is_empty() {
                        return T1Run {
                            image: None,
                            said: format!("mode 1 (object) on source {k}: {}", r.said),
                            same_refusal: false,
                            names: Vec::new(),
                        };
                    }
                    std::fs::write(out.join(format!("{p}.{k}.t1.o")), &r.out)
                        .expect("the object writes");
                    objects.extend_from_slice(&r.out);
                }
                let r = w.run(&[load, 2], &objects, out, &format!("{p}.link"));
                r.into_run("image linked from object files")
            }
        }
    }
}

/// One run of the wrapper module `spec/entry/assemble.t1`: its exit status
/// and every octet it wrote to the output channel.
struct WrapperOut {
    status: Option<i128>,
    out: Vec<u8>,
    said: String,
}

impl WrapperOut {
    /// A status of 0 with octets is a file; 2001..3000 with none is the link's
    /// own refusal (namaste-main alone), which the census pairs with Rust's.
    fn into_run(self, what: &str) -> T1Run {
        match self.status {
            Some(0) if !self.out.is_empty() => T1Run {
                said: format!("{what} {} octets; {}", self.out.len(), self.said),
                image: Some(self.out),
                same_refusal: false,
                names: Vec::new(),
            },
            Some(s) if (2001..3000).contains(&s) => T1Run {
                image: None,
                said: format!(
                    "REFUSED by the link: status {s} = 2000 + {}; {}",
                    s - 2000,
                    self.said
                ),
                same_refusal: self.out.is_empty(),
                names: Vec::new(),
            },
            s => T1Run {
                image: None,
                said: format!(
                    "FAILED: status {s:?}, {} octets; {}",
                    self.out.len(),
                    self.said
                ),
                same_refusal: false,
                names: Vec::new(),
            },
        }
    }
}

/// Where the wrapper runs.
enum Wrapper {
    /// `CHAIN` plus the wrapper module, in the Rust interpreter: the argument
    /// and input globals set as `yantra::input` sets them, the output channel
    /// read from the interpreter's sink.
    Interpreted,
    /// The self-hosted wrapper image under `yantra-run`.
    Native { yantra: PathBuf, image: PathBuf },
}

impl Wrapper {
    fn run(&self, args: &[u64], input: &[u8], scratch: &Path, tag: &str) -> WrapperOut {
        match self {
            Wrapper::Interpreted => wrapper_interpreted(args, input),
            Wrapper::Native { yantra, image } => {
                let path = scratch.join(format!("{tag}.in"));
                std::fs::write(&path, input).expect("the wrapper input writes");
                let ram = std::env::var("W302_YANTRA_RAM").unwrap_or_else(|_| "1073741824".into());
                let run = Command::new(yantra)
                    .env("YANTRA_INPUT", &path)
                    .env("YANTRA_INPUT_NAME", tag)
                    .env("YANTRA_RAM", ram)
                    .env("YANTRA_STEPS", "4000000000000")
                    .env_remove("YANTRA_INPUT_TRACE")
                    .arg(image)
                    .args(args.iter().map(u64::to_string))
                    .output()
                    .expect("yantra-run runs");
                let log = String::from_utf8_lossy(&run.stderr);
                let status = log
                    .lines()
                    .filter_map(|l| {
                        let at = l.find("status: Some(")? + "status: Some(".len();
                        l[at..].split(')').next()?.parse().ok()
                    })
                    .next_back();
                let overrun = log.contains("may have overwritten its own source");
                WrapperOut {
                    status: if overrun { None } else { status },
                    said: format!(
                        "native exit {:?}{}",
                        run.status.code(),
                        if overrun { ", RAM GUARD FIRED" } else { "" }
                    ),
                    out: run.stdout,
                }
            }
        }
    }
}

/// The wrapper module run in the interpreter, as `yantra-run` runs its image:
/// `आदेशपङ्क्तिः` is the program name and the arguments in ASCII decimal,
/// NUL-separated; `आदेशगणना` their count; `निवेशपाठः` the input.
fn wrapper_interpreted(args: &[u64], input: &[u8]) -> WrapperOut {
    let fuel = 4_000_000_000_000u64;
    let root = repo_root();
    let src = std::fs::read_to_string(root.join("spec/entry/assemble.t1"))
        .expect("spec/entry/assemble.t1 reads");
    let mut sources: Vec<(&str, &str)> = CHAIN.to_vec();
    sources.push(("assemble.t1", &src));
    let mut it = match Interpreter::load(&sources, &root.join("spec")) {
        Ok(it) => it,
        Err(e) => {
            return WrapperOut {
                status: None,
                out: Vec::new(),
                said: format!("the chain and the wrapper do not load: {e:?}"),
            };
        }
    };
    let mut argv = b"assemble".to_vec();
    for a in args {
        argv.push(0);
        argv.extend_from_slice(a.to_string().as_bytes());
    }
    for (g, v) in [
        ("निवेशपाठः", Value::Octets(Octets::new(input))),
        ("आदेशपङ्क्तिः", Value::Octets(Octets::new(&argv))),
        ("आदेशगणना", Value::Int(1 + args.len() as i128)),
    ] {
        assert!(
            it.set_global(g, v),
            "`{g}` is not a global of the chain + wrapper"
        );
    }
    match it.call("पाठसेतुःॱमुख्यम्", vec![], fuel) {
        Ok(v) => WrapperOut {
            status: v.as_int(),
            out: it.sink().to_vec(),
            said: "interpreted".into(),
        },
        Err(e) => WrapperOut {
            status: None,
            out: it.sink().to_vec(),
            said: format!("interpreted FAULT: {e:?}"),
        },
    }
}

impl T1Run {
    fn with_names(mut self, names: Vec<String>) -> Self {
        self.same_refusal = !names.is_empty() && names.iter().all(|n| !n.is_empty());
        self.names = names;
        self
    }
}

/// The wrapper's status table (`spec/entry/assemble.t1`): ० wrote an image,
/// `१००० + k` the object builder refused, `२००० + n` the link refused with n
/// diagnostics, `३०००` no input, `३००१` a bad address argument.
fn native_run(yantra: &Path, image: &Path, p: &str, src_path: &Path, load: u64) -> T1Run {
    let ram = std::env::var("W302_YANTRA_RAM").unwrap_or_else(|_| "1073741824".into());
    let run = Command::new(yantra)
        .env("YANTRA_INPUT", src_path)
        .env("YANTRA_INPUT_NAME", p)
        .env("YANTRA_RAM", ram)
        .env("YANTRA_STEPS", "4000000000000")
        .env("YANTRA_WATERMARK", "1")
        .env_remove("YANTRA_INPUT_TRACE")
        .arg(image)
        .arg(load.to_string())
        .output()
        .expect("yantra-run runs");
    let log = String::from_utf8_lossy(&run.stderr);
    let status: Option<u64> = log
        .lines()
        .filter_map(|l| {
            let at = l.find("status: Some(")? + "status: Some(".len();
            l[at..].split(')').next()?.parse().ok()
        })
        .next_back();
    let steps = log
        .lines()
        .find(|l| l.starts_with("steps:"))
        .unwrap_or("steps: ?")
        .to_string();
    let halt = log
        .lines()
        .find(|l| l.starts_with("halt:"))
        .unwrap_or("halt: ?")
        .to_string();
    // Anything the RAM guard said is a failure of the run, never a result.
    let overrun = log.contains("may have overwritten its own source");
    match status {
        Some(0) if run.status.success() && !overrun && !run.stdout.is_empty() => T1Run {
            said: format!("native image {} octets; {steps}", run.stdout.len()),
            image: Some(run.stdout),
            same_refusal: false,
            names: Vec::new(),
        },
        Some(s) if (2001..3000).contains(&s) && !overrun => T1Run {
            image: None,
            said: format!(
                "native REFUSED by the link: status {s} = 2000 + {} diagnostic(s); {halt}",
                s - 2000
            ),
            same_refusal: run.stdout.is_empty(),
            names: Vec::new(),
        },
        _ => T1Run {
            image: None,
            said: format!(
                "native FAILED: exit {:?}, {halt}, {steps}, {} stdout octets{}",
                run.status.code(),
                run.stdout.len(),
                if overrun { ", RAM GUARD FIRED" } else { "" }
            ),
            same_refusal: false,
            names: Vec::new(),
        },
    }
}

/// THE CENSUS, over either side. Every `spec/*.sas` (exactly 48, the ruling's
/// population), at every address the check scripts pass: Rust `sadhana
/// [--स्थान <addr>]` writes one file, the `.t1` side another, and the
/// comparator grades the pair. Earlier outputs are deleted before each run.
fn census(side: &Side, label: &str) {
    let sadhana = PathBuf::from(std::env::var_os("W302_SADHANA").expect(
        "W302_SADHANA names the Rust sadhana binary; the tools/check-w302-*.sh scripts build and pass it",
    ));
    assert!(sadhana.is_file(), "{} is not a file", sadhana.display());
    let out = std::env::var_os("W302_OUT").map_or_else(
        || std::env::temp_dir().join(format!("w302-{label}.{}", std::process::id())),
        PathBuf::from,
    );
    std::fs::create_dir_all(&out).expect("the output directory");

    let root = repo_root();
    let mut programs: Vec<String> = std::fs::read_dir(root.join("spec"))
        .expect("spec/ reads")
        .filter_map(Result::ok)
        .filter_map(|e| {
            e.file_name()
                .to_str()
                .and_then(|n| n.strip_suffix(".sas"))
                .map(str::to_string)
        })
        .collect();
    programs.sort();
    assert_eq!(
        programs.len(),
        48,
        "the ruling's population is the 48 spec/*.sas programs; found {}: {programs:?}",
        programs.len()
    );

    let (mut identical, mut refused_both, mut failed) = (0, 0, Vec::new());
    let (mut sections_agree, mut objects_identical) = (0usize, 0usize);
    for p in &programs {
        let Some(addrs) = addresses(p) else {
            failed.push(format!(
                "{p}: no load address recorded for it in this census"
            ));
            continue;
        };
        let src_path = root.join("spec").join(format!("{p}.sas"));
        for addr in addrs {
            let tag = addr.map_or_else(|| "default".to_string(), str::to_string);
            let load = addr.map_or(sadhana::kosha::LOAD_ADDRESS, |a| {
                sanskrit_text::numeral::value(a).expect("the address is a numeral")
            });
            let rust_out = out.join(format!("{p}@{load:#x}.rust.elf"));
            let t1_out = out.join(format!("{p}@{load:#x}.{label}.elf"));
            for f in [&rust_out, &t1_out] {
                match std::fs::remove_file(f) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => panic!("cannot delete the earlier {}: {e}", f.display()),
                }
                assert!(!f.exists(), "{} survived its deletion", f.display());
            }

            let mut cmd = Command::new(&sadhana);
            if let Some(a) = addr {
                cmd.arg("--स्थान").arg(a);
            }
            let rust = cmd
                .arg(&src_path)
                .arg(&rust_out)
                .output()
                .expect("sadhana runs");
            let rust_said = String::from_utf8_lossy(&rust.stderr)
                .trim()
                .replace('\n', " / ");

            let t1 = side.run(p, std::slice::from_ref(&src_path), &out, load);
            if let Some(img) = &t1.image {
                std::fs::write(&t1_out, img).expect("the .t1 image writes");
            }
            // An expected refusal must be the SAME refusal: interpreted, the
            // chain's linker names an undefined routine and Rust's message
            // names it too; native, the wrapper's status is a link refusal.
            let same_cause =
                t1.same_refusal && t1.names.iter().all(|n| rust_said.contains(n.as_str()));
            let t1_said = if t1.names.is_empty() {
                t1.said.clone()
            } else {
                format!("{} {:?}", t1.said, t1.names)
            };

            let mut verdict = compare_files(&rust_out, &t1_out);
            // THE SECTION TABLE (W-302 option i): an image the wrapper wrote
            // carries `.text`/`.data`/`.bss` headers equal to Rust's — what
            // `readelf -S` reads in five check scripts.
            if side.writes_section_table()
                && let (Verdict::Identical { .. }, Ok(r), Ok(t)) =
                    (&verdict, std::fs::read(&rust_out), std::fs::read(&t1_out))
            {
                match section_table_agrees_with(&r, &t, matches!(side, Side::Objects(_))) {
                    Ok(n) => sections_agree += n,
                    Err(why) => verdict = Verdict::Differs(format!("section table: {why}")),
                }
            }
            // THE OBJECT FILE beside Rust's: reported, and counted when the
            // two are octet-identical; the census grades the IMAGE linked
            // from it, which is the claim.
            if matches!(side, Side::Objects(_)) {
                let rust_o = out.join(format!("{p}@{load:#x}.rust.o"));
                let _ = Command::new(&sadhana)
                    .arg("--वस्तु")
                    .arg(&src_path)
                    .arg(&rust_o)
                    .output()
                    .expect("sadhana runs");
                let t1_o = out.join(format!("{p}.0.t1.o"));
                match (std::fs::read(&rust_o), std::fs::read(&t1_o)) {
                    (Ok(a), Ok(b)) if a == b => objects_identical += 1,
                    (Ok(a), Ok(b)) => println!(
                        "OBJECT {p} @ {tag}: differs from Rust's --वस्तु: rust {} octets, t1 {}, first at {:?}",
                        a.len(),
                        b.len(),
                        a.iter().zip(&b).position(|(x, y)| x != y)
                    ),
                    _ => println!("OBJECT {p} @ {tag}: one side wrote no object"),
                }
            }
            let expected_refusal = REFUSED_BY_BOTH.contains(&p.as_str());
            let line = match (&verdict, expected_refusal) {
                (Verdict::Identical { loads }, false) => {
                    identical += 1;
                    format!("IDENTICAL ({loads} PT_LOAD) — t1: {t1_said}")
                }
                (_, true)
                    if !rust_out.exists()
                        && !t1_out.exists()
                        && !rust.status.success()
                        && same_cause =>
                {
                    refused_both += 1;
                    format!(
                        "REFUSED BY BOTH (expected, same cause) — rust: {rust_said} — t1: {t1_said}"
                    )
                }
                (v, _) => {
                    let why = format!(
                        "{v:?}{} — rust exit {:?}: {rust_said} — t1: {t1_said}",
                        if expected_refusal {
                            " (EXPECTED both to refuse)"
                        } else {
                            ""
                        },
                        rust.status.code()
                    );
                    failed.push(format!("{p} @ {tag}: {why}"));
                    format!("FAIL {why}")
                }
            };
            println!("CENSUS {p} @ {tag} ({load:#x}): {line}");
        }
    }
    // THE TWO-OBJECT LINKS: the `.t1` side against BOTH of Rust's link paths.
    let mut linked_identical = 0;
    for (name, members) in LINKED {
        let srcs: Vec<PathBuf> = members
            .iter()
            .map(|m| root.join("spec").join(format!("{m}.sas")))
            .collect();
        let load = sadhana::kosha::LOAD_ADDRESS;
        let rust_whole = out.join(format!("{name}.rust-whole.elf"));
        let rust_objects = out.join(format!("{name}.rust-objects.elf"));
        let t1_out = out.join(format!("{name}.{label}.elf"));
        let objects: Vec<PathBuf> = members
            .iter()
            .map(|m| out.join(format!("{name}.{m}.o")))
            .collect();
        for f in [&rust_whole, &rust_objects, &t1_out]
            .into_iter()
            .chain(&objects)
        {
            match std::fs::remove_file(f) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => panic!("cannot delete the earlier {}: {e}", f.display()),
            }
        }
        // tools/check-link.sh: every source on one command line.
        let whole = Command::new(&sadhana)
            .args(&srcs)
            .arg(&rust_whole)
            .output()
            .expect("sadhana runs");
        // tools/check-separate.sh: an object per source, then the link.
        let mut said = vec![String::from_utf8_lossy(&whole.stderr).trim().to_string()];
        for (s, o) in srcs.iter().zip(&objects) {
            let r = Command::new(&sadhana)
                .arg("--वस्तु")
                .arg(s)
                .arg(o)
                .output()
                .expect("sadhana runs");
            said.push(String::from_utf8_lossy(&r.stderr).trim().to_string());
        }
        let r = Command::new(&sadhana)
            .arg("--संयोजय")
            .args(&objects)
            .arg(&rust_objects)
            .output()
            .expect("sadhana runs");
        said.push(String::from_utf8_lossy(&r.stderr).trim().to_string());

        let t1 = side.run(name, &srcs, &out, load);
        if let Some(img) = &t1.image {
            std::fs::write(&t1_out, img).expect("the .t1 image writes");
        }
        let v_whole = compare_files(&rust_whole, &t1_out);
        let mut v_objects = compare_files(&rust_objects, &t1_out);
        // through object files, the linked image carries Rust's names too
        // (check-separate.sh: `nm` finds मुद्रकः)
        if matches!(side, Side::Objects(_))
            && let (Verdict::Identical { .. }, Ok(r), Ok(t)) = (
                &v_objects,
                std::fs::read(&rust_objects),
                std::fs::read(&t1_out),
            )
            && let Err(why) = section_table_agrees_with(&r, &t, true)
        {
            v_objects = Verdict::Differs(format!("section table: {why}"));
        }
        let line = match (&v_whole, &v_objects) {
            (Verdict::Identical { loads }, Verdict::Identical { .. }) => {
                linked_identical += 1;
                format!(
                    "IDENTICAL ({loads} PT_LOAD) to BOTH Rust links (whole-program and \
                     --वस्तु/--संयोजय) — t1: {}",
                    t1.said
                )
            }
            _ => {
                let why = format!(
                    "whole-program {v_whole:?}; objects {v_objects:?} — rust: {} — t1: {}",
                    said.join(" / ").replace('\n', " / "),
                    t1.said
                );
                failed.push(format!("{name} (link): {why}"));
                format!("FAIL {why}")
            }
        };
        println!(
            "CENSUS {name} (link of {}) @ default ({load:#x}): {line}",
            members.join(" + ")
        );
    }
    println!(
        "METRIC w302_loaded_identity side={label} programs={} identical={identical} refused_by_both={refused_both} linked={linked_identical}/{} failed={} section_headers_equal={sections_agree} objects_octet_identical={objects_identical}",
        programs.len(),
        LINKED.len(),
        failed.len()
    );
    assert!(
        failed.is_empty(),
        "loaded-image identity fails ({label}):\n{}",
        failed.join("\n")
    );
}

/// THE CENSUS, INTERPRETED: the chain's own routines in the Rust interpreter.
#[test]
#[ignore = "census: the .t1 chain interpreted over 48 programs, minutes in release — run tools/check-w302-loaded-identity.sh"]
fn the_48_programs_load_identically_from_both_assemblers() {
    census(&Side::Interpreted, "t1");
}

/// THE CENSUS, NATIVE: the self-hosted image assembles each program. Needs
/// `W302_NATIVE` (the wrapper image `tools/check-w302-native.sh` builds) and
/// `W302_YANTRA_RUN` besides `W302_SADHANA`.
#[test]
#[ignore = "census: the self-hosted wrapper image run natively over 48 programs — run tools/check-w302-native.sh"]
fn the_48_programs_load_identically_from_the_native_image() {
    let var = |k: &str| {
        let p = PathBuf::from(std::env::var_os(k).unwrap_or_else(|| {
            panic!("{k} is unset; tools/check-w302-native.sh builds and passes it")
        }));
        assert!(p.is_file(), "{k}: {} is not a file", p.display());
        p
    };
    census(
        &Side::Native {
            yantra: var("W302_YANTRA_RUN"),
            image: var("W302_NATIVE"),
        },
        "native",
    );
}

// ------------------------------------------------- the comparator's mutants

/// A real pair: Rust's image and the `.t1` chain's of the same small program,
/// at the default address. Read from `spec/virtio-net.sas`, which both sides
/// assemble (census 2026-10-03) and whose image is a few hundred octets.
fn real_pair() -> (Vec<u8>, Vec<u8>) {
    let src = std::fs::read_to_string(repo_root().join("spec/virtio-net.sas"))
        .expect("spec/virtio-net.sas reads");
    let load = sadhana::kosha::LOAD_ADDRESS;
    let rust = sadhana::assemble(
        &src,
        sadhana::encode::Target::Uncompressed,
        load,
        sadhana::nidana::Language::Sanskrit,
    )
    .unwrap_or_else(|ds| {
        panic!(
            "Rust assembles virtio-net: {:?}",
            ds.iter().map(|d| &d.reason).collect::<Vec<_>>()
        )
    });
    let t1 = t1_image(src.as_bytes(), load).expect("the chain assembles virtio-net");
    (rust, t1)
}

fn first_load_offset(img: &[u8]) -> usize {
    let phoff = usize::try_from(le(img, 32, 8)).expect("fits");
    (0..usize::try_from(le(img, 56, 2)).expect("fits"))
        .map(|k| phoff + PHDR * k)
        .find(|&at| le(img, at, 4) == u64::from(PT_LOAD))
        .map(|at| usize::try_from(le(img, at + 8, 8)).expect("fits"))
        .expect("a PT_LOAD")
}

fn red(v: &Verdict, what: &str) {
    println!("MUTANT {what}: {v:?}");
    assert!(
        matches!(v, Verdict::Differs(_)),
        "{what} must go red (Differs), got {v:?}"
    );
}

/// THE CONTROL, ON A REAL PAIR: the two assemblers' images of one program are
/// identical under the ruling, while the whole files are NOT — the `.t1`
/// writer emits no section table. So the exclusion is exercised, not vacuous.
#[test]
fn a_real_pair_is_identical_though_the_files_differ() {
    let (rust, t1) = real_pair();
    let v = compare(&rust, &t1);
    println!(
        "METRIC w302_real_pair {v:?} rust={} t1={} e_shnum rust={} t1={}",
        rust.len(),
        t1.len(),
        le(&rust, 60, 2),
        le(&t1, 60, 2)
    );
    assert!(
        matches!(v, Verdict::Identical { loads } if loads >= 1),
        "got {v:?}"
    );
    assert_ne!(
        rust, t1,
        "the files differ (the section table), else the control proves nothing"
    );
}

/// Each mutant goes red; each change confined to the excluded fields does not.
#[test]
fn every_mutant_goes_red_and_the_excluded_fields_do_not() {
    let (rust, t1) = real_pair();
    assert!(matches!(compare(&rust, &t1), Verdict::Identical { .. }));

    // One byte of a loaded segment.
    let mut m = t1.clone();
    m[first_load_offset(&t1)] ^= 0x01;
    red(&compare(&rust, &m), "one loaded byte");

    // Every field of the first program header, one at a time.
    let phoff = usize::try_from(le(&t1, 32, 8)).expect("fits");
    for (name, off, _) in PH_FIELDS {
        let mut m = t1.clone();
        m[phoff + off] ^= 0x01;
        red(&compare(&rust, &m), &format!("program header {name}"));
    }

    // e_flags, included by the ruling.
    let mut m = t1.clone();
    m[48] ^= 0x04;
    red(&compare(&rust, &m), "e_flags");

    // e_entry, a header field that is not excluded.
    let mut m = t1.clone();
    m[24] ^= 0x04;
    red(&compare(&rust, &m), "e_entry");

    // THE CONTROL: only the four section-table fields changed — identical.
    let mut m = t1.clone();
    for at in (40..48).chain(58..64) {
        m[at] = !m[at];
    }
    let v = compare(&rust, &m);
    println!("CONTROL excluded fields flipped: {v:?}");
    assert!(
        matches!(v, Verdict::Identical { .. }),
        "excluded fields must not count, got {v:?}"
    );
}

/// FAULT (a): `e_phnum` ० on BOTH sides compares nothing, and must refuse —
/// not read IDENTICAL. Likewise a program-header table holding no `PT_LOAD`.
#[test]
fn nothing_to_compare_is_refused_not_identical() {
    let (rust, t1) = real_pair();
    let (mut r, mut t) = (rust.clone(), t1.clone());
    for b in [&mut r, &mut t] {
        b[56] = 0;
        b[57] = 0;
    }
    let v = compare(&r, &t);
    println!("FAULT e_phnum 0 on both sides: {v:?}");
    assert!(
        matches!(v, Verdict::Refused(_)),
        "e_phnum 0 must refuse, got {v:?}"
    );

    // Every header's p_type moved off PT_LOAD, on both sides.
    let (mut r, mut t) = (rust.clone(), t1.clone());
    let phoff = usize::try_from(le(&rust, 32, 8)).expect("fits");
    for k in 0..usize::try_from(le(&rust, 56, 2)).expect("fits") {
        for b in [&mut r, &mut t] {
            if le(b, phoff + PHDR * k, 4) == u64::from(PT_LOAD) {
                b[phoff + PHDR * k] = 4; // PT_NOTE
            }
        }
    }
    let v = compare(&r, &t);
    println!("FAULT no PT_LOAD on both sides: {v:?}");
    assert!(
        matches!(v, Verdict::Refused(_)),
        "no PT_LOAD must refuse, got {v:?}"
    );

    // Not an ELF at all, and an empty file, on both sides.
    for junk in [&b""[..], &[0u8; 128][..]] {
        let v = compare(junk, junk);
        assert!(
            matches!(v, Verdict::Refused(_)),
            "junk must refuse, got {v:?}"
        );
    }
}

/// FAULT (b): a MISSING output on either side is a failure, never a match —
/// and both missing is no match either.
#[test]
fn a_missing_output_is_a_failure() {
    let (rust, t1) = real_pair();
    let dir = std::env::temp_dir().join(format!("w302-missing.{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let (r, t) = (dir.join("r.elf"), dir.join("t.elf"));
    let gone = dir.join("never-written.elf");
    let _ = std::fs::remove_file(&gone);
    std::fs::write(&r, &rust).expect("write");
    std::fs::write(&t, &t1).expect("write");

    assert!(
        matches!(compare_files(&r, &t), Verdict::Identical { .. }),
        "the control pair"
    );
    for (label, a, b) in [
        ("rust missing", &gone, &t),
        ("t1 missing", &r, &gone),
        ("both missing", &gone, &gone),
    ] {
        let v = compare_files(a, b);
        println!("FAULT {label}: {v:?}");
        assert!(
            matches!(v, Verdict::Refused(ref w) if w.starts_with("MISSING")),
            "{label}: got {v:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ══ THE FILE FORMS (W-302, option i) ══════════════════════════════════════
//
// (1) The wrapper's image carries a MINIMAL SECTION TABLE — `.text`, `.data`
//     and `.bss` headers and `.shstrtab`, no symbols — equal, header for
//     header, to the ones Rust writes for the same loaded image.
// (2) An OBJECT FILE: mode १ writes a relocatable ELF from the `.t1` chain's
//     object, mode २ reads such files back and links them.

/// One section header: (name, sh_type, sh_flags, sh_addr, sh_offset,
/// sh_size, sh_addralign).
type Section = (String, u64, u64, u64, u64, u64, u64);

/// Every section header of an ELF64 little-endian file, named through its
/// `.shstrtab`. Refuses a table that is absent or does not fit.
fn sections(b: &[u8]) -> Result<Vec<Section>, String> {
    if b.len() < EHDR || b[..4] != *b"\x7fELF" {
        return Err("not an ELF file".into());
    }
    let shoff = usize::try_from(le(b, 40, 8)).map_err(|_| "e_shoff does not fit")?;
    let shentsize = le(b, 58, 2);
    let shnum = usize::try_from(le(b, 60, 2)).expect("u16 fits");
    let shstrndx = usize::try_from(le(b, 62, 2)).expect("u16 fits");
    if shnum == 0 || shoff == 0 {
        return Err("no section table".into());
    }
    if shentsize != 64 {
        return Err(format!("e_shentsize is {shentsize}"));
    }
    if shoff + 64 * shnum > b.len() || shstrndx >= shnum {
        return Err("the section table does not fit".into());
    }
    let field = |k: usize, at: usize, n: usize| le(b, shoff + 64 * k + at, n);
    let names_at = usize::try_from(field(shstrndx, 24, 8)).map_err(|_| "fit")?;
    let mut out = Vec::new();
    for k in 0..shnum {
        let at = names_at + usize::try_from(field(k, 0, 4)).expect("u32 fits");
        let name = b
            .get(at..)
            .and_then(|r| r.split(|&c| c == 0).next())
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .ok_or_else(|| format!("section {k}'s name is outside the file"))?;
        out.push((
            name,
            field(k, 4, 4),
            field(k, 8, 8),
            field(k, 16, 8),
            field(k, 24, 8),
            field(k, 32, 8),
            field(k, 48, 8),
        ));
    }
    Ok(out)
}

/// Every symbol of an ELF file's `.symtab` (the null one excluded), through
/// the `.strtab` its `sh_link` names: (name, st_info, st_shndx, st_value),
/// sorted — so two tables compare as sets.
fn symbols(b: &[u8]) -> Result<Vec<(String, u64, u64, u64)>, String> {
    let secs = sections(b)?;
    let shoff = usize::try_from(le(b, 40, 8)).expect("fits");
    let Some(k) = secs.iter().position(|s| s.1 == 2) else {
        return Ok(Vec::new());
    };
    let link = usize::try_from(le(b, shoff + 64 * k + 40, 4)).expect("fits");
    let names = usize::try_from(secs.get(link).ok_or("sh_link out of range")?.4).expect("fits");
    let (off, size) = (
        usize::try_from(secs[k].4).expect("fits"),
        usize::try_from(secs[k].5).expect("fits"),
    );
    let mut out = Vec::new();
    for e in (off + 24..off + size).step_by(24) {
        let at = names + usize::try_from(le(b, e, 4)).expect("fits");
        let name = b
            .get(at..)
            .and_then(|r| r.split(|&c| c == 0).next())
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .ok_or("a symbol name is outside the file")?;
        out.push((name, le(b, e + 4, 1), le(b, e + 6, 2), le(b, e + 8, 8)));
    }
    out.sort();
    Ok(out)
}

/// `.text`, `.data` and `.bss` agree between the two images in type, flags,
/// address, offset, size and alignment, each present on one side exactly when
/// on the other. The symbols: an image from source carries NO symbol table
/// (the ruling); an image linked from object files (`with_symbols`) carries
/// Rust's symbols, the same set of (name, binding and type, section, value).
/// Answers how many headers were compared.
fn section_table_agrees(rust: &[u8], t1: &[u8]) -> Result<usize, String> {
    section_table_agrees_with(rust, t1, false)
}

fn section_table_agrees_with(rust: &[u8], t1: &[u8], with_symbols: bool) -> Result<usize, String> {
    let (r, t) = (
        sections(rust).map_err(|e| format!("rust: {e}"))?,
        sections(t1).map_err(|e| format!("t1: {e}"))?,
    );
    let mut n = 0;
    for name in [".text", ".data", ".bss"] {
        let a = r.iter().find(|s| s.0 == name);
        let b = t.iter().find(|s| s.0 == name);
        match (a, b) {
            (Some(a), Some(b)) if a == b => n += 1,
            (None, None) => {}
            (a, b) => return Err(format!("{name}: rust {a:?} t1 {b:?}")),
        }
    }
    if n == 0 {
        return Err("no .text header on either side".into());
    }
    if with_symbols {
        let (a, b) = (symbols(rust)?, symbols(t1)?);
        if a.is_empty() || a != b {
            let only_rust: Vec<_> = a.iter().filter(|x| !b.contains(x)).take(4).collect();
            let only_t1: Vec<_> = b.iter().filter(|x| !a.contains(x)).take(4).collect();
            return Err(format!(
                "symbols: rust {} t1 {}; only in rust {only_rust:?}; only in t1 {only_t1:?}",
                a.len(),
                b.len()
            ));
        }
    } else if let Some(s) = t.iter().find(|s| s.1 == 2) {
        return Err(format!("the .t1 image carries a symbol table ({})", s.0));
    }
    Ok(n)
}

/// One relocation record: (patched section's name, r_offset, symbol index,
/// type, addend).
type Rela = (String, u64, u64, u64, i64);

/// The relocation records of a relocatable object, in file order.
fn relocations(o: &[u8]) -> Result<Vec<Rela>, String> {
    let secs = sections(o)?;
    let shoff = usize::try_from(le(o, 40, 8)).expect("fits");
    let mut out = Vec::new();
    for (k, s) in secs.iter().enumerate() {
        if s.1 != 4 {
            continue;
        }
        let info = usize::try_from(le(o, shoff + 64 * k + 44, 4)).expect("fits");
        let target = secs.get(info).map_or("?".to_string(), |t| t.0.clone());
        let (off, size) = (
            usize::try_from(s.4).expect("fits"),
            usize::try_from(s.5).expect("fits"),
        );
        for e in (off..off + size).step_by(24) {
            let r_info = le(o, e + 8, 8);
            out.push((
                target.clone(),
                le(o, e, 8),
                r_info >> 32,
                r_info & 0xffff_ffff,
                le(o, e + 16, 8) as i64,
            ));
        }
    }
    Ok(out)
}

fn rust_image(src: &Path, load: Option<&str>) -> Vec<u8> {
    let text = std::fs::read_to_string(src).expect("the source reads");
    let at = load.map_or(sadhana::kosha::LOAD_ADDRESS, |a| {
        sanskrit_text::numeral::value(a).expect("a numeral")
    });
    sadhana::assemble(
        &text,
        sadhana::encode::Target::Uncompressed,
        at,
        sadhana::nidana::Language::Sanskrit,
    )
    .unwrap_or_else(|ds| {
        panic!(
            "Rust assembles {}: {:?}",
            src.display(),
            ds.iter().map(|d| &d.reason).collect::<Vec<_>>()
        )
    })
}

fn spec(p: &str) -> PathBuf {
    repo_root().join("spec").join(format!("{p}.sas"))
}

fn scratch() -> PathBuf {
    let d = std::env::temp_dir().join(format!("w302-forms.{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

#[test]
fn the_wrapper_image_carries_rusts_section_headers() {
    // paging: .text, .data and a .bss reservation; virtio-net: no .bss.
    for (p, load) in [("paging", 0x8020_0000u64), ("virtio-net", 0x8020_0000)] {
        let rust = rust_image(&spec(p), Some("०षोड्८०२०००००"));
        let src = std::fs::read(spec(p)).expect("reads");
        let r = Wrapper::Interpreted.run(&[load, 0], &src, &scratch(), p);
        assert_eq!(r.status, Some(0), "{p}: {}", r.said);
        assert!(
            matches!(compare(&rust, &r.out), Verdict::Identical { .. }),
            "{p}: {:?}",
            compare(&rust, &r.out)
        );
        let n = section_table_agrees(&rust, &r.out).unwrap_or_else(|e| panic!("{p}: {e}"));
        println!(
            "SECTIONS {p}: {n} headers equal to Rust's; {:?}",
            sections(&r.out).expect("parses")
        );
        assert!(n >= 2, "{p}: only {n} headers compared");

        // the check bites: one field of one header changed goes red
        let mut m = r.out.clone();
        let shoff = usize::try_from(le(&m, 40, 8)).expect("fits");
        m[shoff + 64 + 32] ^= 0x04; // .text sh_size
        let v = section_table_agrees(&rust, &m);
        println!("MUTANT {p} .text sh_size: {v:?}");
        assert!(v.is_err(), "{p}: a changed .text size must disagree");
    }
}

#[test]
fn the_object_file_carries_the_relocations_readelf_counts() {
    // check-jump-table.sh: two R_RISCV_64 in the object (the table's entries)
    let src = std::fs::read(spec("jump-table")).expect("reads");
    let o = Wrapper::Interpreted.run(&[0x8000_0000, 1], &src, &scratch(), "jump-table");
    assert_eq!(o.status, Some(0), "{}", o.said);
    assert_eq!(le(&o.out, 16, 2), 1, "e_type is ET_REL");
    let rel = relocations(&o.out).expect("the object parses");
    println!("RELOCATIONS jump-table: {rel:?}");
    let r64 = rel.iter().filter(|r| r.3 == 2).count();
    assert_eq!(r64, 2, "two R_RISCV_64 records");
    assert!(
        rel.iter().filter(|r| r.3 == 2).all(|r| r.0 == ".data"),
        "against .data"
    );
    // check-separate.sh: the call and the address pair in namaste-main
    let src = std::fs::read(spec("namaste-main")).expect("reads");
    let o = Wrapper::Interpreted.run(&[0x8000_0000, 1], &src, &scratch(), "namaste-main");
    assert_eq!(o.status, Some(0), "{}", o.said);
    let rel = relocations(&o.out).expect("the object parses");
    println!("RELOCATIONS namaste-main: {rel:?}");
    assert!(rel.len() >= 2, "the call and the address pair");
}

#[test]
fn an_object_read_back_links_to_rusts_image() {
    let src = std::fs::read(spec("jump-table")).expect("reads");
    let o = Wrapper::Interpreted.run(&[0x8000_0000, 1], &src, &scratch(), "jump-table");
    assert_eq!(o.status, Some(0), "{}", o.said);
    let img = Wrapper::Interpreted.run(&[0x8000_0000, 2], &o.out, &scratch(), "jump-table.link");
    assert_eq!(img.status, Some(0), "{}", img.said);
    let rust = rust_image(&spec("jump-table"), None);
    let v = compare(&rust, &img.out);
    println!("LINKED FROM ITS OBJECT jump-table: {v:?}");
    assert!(matches!(v, Verdict::Identical { .. }), "{v:?}");
    // and the pair: namaste-main + lib-mudraka, two objects end to end
    let mut objs = Vec::new();
    for p in ["namaste-main", "lib-mudraka"] {
        let src = std::fs::read(spec(p)).expect("reads");
        let o = Wrapper::Interpreted.run(&[0x8000_0000, 1], &src, &scratch(), p);
        assert_eq!(o.status, Some(0), "{p}: {}", o.said);
        objs.extend_from_slice(&o.out);
    }
    let img = Wrapper::Interpreted.run(&[0x8000_0000, 2], &objs, &scratch(), "pair.link");
    assert_eq!(img.status, Some(0), "{}", img.said);
    // Rust's own --वस्तु per source then --संयोजय, as check-separate.sh does
    let rust_objects: Vec<sadhana::vastu::Object> = ["namaste-main", "lib-mudraka"]
        .iter()
        .map(|p| {
            let text = std::fs::read_to_string(spec(p)).expect("reads");
            let bytes = sadhana::assemble_object(
                &text,
                None,
                sadhana::encode::Target::Uncompressed,
                false,
                sadhana::nidana::Language::Sanskrit,
            )
            .unwrap_or_else(|_| panic!("Rust assembles {p} to an object"));
            sadhana::vastu::read(&bytes).expect("Rust reads its own object")
        })
        .collect();
    let rust = sadhana::link_objects(&rust_objects, sadhana::kosha::LOAD_ADDRESS)
        .expect("Rust links the pair");
    let v = compare(&rust, &img.out);
    println!("LINKED FROM TWO OBJECTS: {v:?}");
    assert!(matches!(v, Verdict::Identical { .. }), "{v:?}");
    // and it carries Rust's names, which check-separate.sh asks nm for
    let n = section_table_agrees_with(&rust, &img.out, true).unwrap_or_else(|e| panic!("{e}"));
    let names = symbols(&img.out).expect("the symbol table parses");
    println!("SYMBOLS of the pair linked from objects ({n} headers equal): {names:?}");
    assert!(
        names.iter().any(|s| s.0 == "मुद्रकः"),
        "nm would not find मुद्रकः"
    );
    // control: an image from source carries none
    let src = std::fs::read(spec("jump-table")).expect("reads");
    let plain = Wrapper::Interpreted.run(&[0x8000_0000, 0], &src, &scratch(), "plain");
    assert_eq!(
        symbols(&plain.out).expect("parses"),
        Vec::new(),
        "mode 0: no symbols"
    );
}

#[test]
fn a_malformed_object_is_refused_by_its_cause() {
    let src = std::fs::read(spec("jump-table")).expect("reads");
    let good = Wrapper::Interpreted.run(&[0x8000_0000, 1], &src, &scratch(), "jump-table");
    assert_eq!(good.status, Some(0), "{}", good.said);
    let o = good.out;
    // control: the object itself links
    let ctl = Wrapper::Interpreted.run(&[0x8000_0000, 2], &o, &scratch(), "ctl");
    assert_eq!(
        ctl.status,
        Some(0),
        "CONTROL the unmodified object: {}",
        ctl.said
    );

    let shoff = usize::try_from(le(&o, 40, 8)).expect("fits");
    let secs = sections(&o).expect("parses");
    let rela_data = secs
        .iter()
        .position(|s| s.0 == ".rela.data")
        .expect(".rela.data");
    let rela_at = usize::try_from(secs[rela_data].4).expect("fits");
    let text = secs.iter().position(|s| s.0 == ".text").expect(".text");

    let mut cases: Vec<(&str, Vec<u8>, i128)> = Vec::new();
    let mut m = o.clone();
    m[1] = b'X';
    cases.push(("bad magic", m, 4001));
    let mut m = o.clone();
    m[16] = 2; // ET_EXEC
    cases.push(("not ET_REL", m, 4002));
    cases.push((
        "truncated before its section table",
        o[..shoff].to_vec(),
        4003,
    ));
    let mut m = o.clone();
    m[shoff + 64 * text + 32] = 0xff; // .text sh_size past the object
    m[shoff + 64 * text + 33] = 0xff;
    cases.push(("a section outside the object", m, 4004));
    let mut m = o.clone();
    m[shoff + 64 * text + 8] = 0; // .text flags 0: a section the model has no place for
    cases.push(("an unmodelled section", m, 4005));
    let mut m = o.clone();
    m[rela_at + 12] = 0xee; // the first .rela.data record's symbol index
    cases.push(("a relocation naming no symbol", m, 4009));
    for (what, input, want) in cases {
        let r = Wrapper::Interpreted.run(&[0x8000_0000, 2], &input, &scratch(), "bad");
        println!(
            "REFUSAL {what}: status {:?}, {} octets written",
            r.status,
            r.out.len()
        );
        assert_eq!(r.status, Some(want), "{what}: {}", r.said);
        assert!(r.out.is_empty(), "{what}: a refused link writes nothing");
    }
}

#[test]
#[ignore = "census: every program through the wrapper's image (mode 0) and through object files (modes 1 and 2), interpreted — run tools/check-w302-loaded-identity.sh"]
fn the_48_programs_load_identically_from_the_wrapper_and_through_object_files() {
    census(&Side::Wrapper(Wrapper::Interpreted), "wrapper");
    census(&Side::Objects(Wrapper::Interpreted), "objects");
}

#[test]
#[ignore = "census: the self-hosted wrapper image's object files (modes 1 and 2), natively — run tools/check-w302-native.sh"]
fn the_48_programs_load_identically_through_native_object_files() {
    let var = |k: &str| {
        let p = PathBuf::from(std::env::var_os(k).unwrap_or_else(|| {
            panic!("{k} is unset; tools/check-w302-native.sh builds and passes it")
        }));
        assert!(p.is_file(), "{k}: {} is not a file", p.display());
        p
    };
    census(
        &Side::Objects(Wrapper::Native {
            yantra: var("W302_YANTRA_RUN"),
            image: var("W302_NATIVE"),
        }),
        "native-objects",
    );
}
