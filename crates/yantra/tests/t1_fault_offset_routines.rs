//! **WHICH ROUTINE IS AT THIS IMAGE OFFSET?** The only instrument in the tree
//! that answers it, and the census's fault reports are unreadable without it.
//!
//! `paradigm_encode`'s census reports a stopped source as
//! `BadAccess { pc: 2147483800, addr: 8 }` — an ADDRESS, after 1953 seconds. For
//! six sources it had never named a ROUTINE, and three fixtures were built on an
//! assumption about which one (`artha`'s entry) that turned out to be wrong: the
//! offset is in the callee the entry hands a record to, not in the entry.
//!
//! # Why it needs no image, and no closure
//!
//! A source's OWN object is an `ET_REL`, so `vastu::read` accepts it and its
//! `.text` symbols carry per-routine offsets. **`vastu::read` answers `None` on a
//! LINKED image, silently** — an absence that reads as clean — so this never
//! reads linked bytes back. Nothing here links, which is why it sidesteps the
//! synthetic-band problem entirely: `chain.rs:606` mints cross-module names from
//! `2_000_000` for the LINKER to answer, so any two-object link without a
//! dependency closure refuses them, and that refusal has cost two sessions a
//! probe each.
//!
//! # The three defects this has already had, which is why it is shaped this way
//!
//! **The third was in this instrument's own first output, and it was the quiet
//! kind.** Read defects 1 and 2 below first: both of those made the bracket
//! REFUSE, which is loud and safe. The third printed a falsehood.
//!
//! **1. Filter on `Placement::Text`, not on definedness.** `.data` and `.text`
//! offsets both start at 0 within their own section, so a definedness filter
//! interleaves them: the first version put two symbols at `+0` and a run of
//! `…भेद` discriminant constants spaced 8 apart where routines should be, and
//! those constants would have been named as the faulting routine. `vastu`'s
//! `Placement` exists for exactly this — its own margin records that matching
//! `section == 1` against `.text` stopped being true the moment `.bss` appeared.
//!
//! **2. `…पर्वN` labels are BLOCKS, not routines.** The emitter labels every
//! basic block, so a naive walk answers `मध्यरूपआरम्भःपर्व३२` — which is not a
//! routine name, and two adjacent blocks read as two different answers. The
//! routine is the label with its block suffix stripped, and stripping is what
//! turned `ir` and `vakyavibhaga` from DISAGREE into a named routine.
//!
//! **3. Report the ROUTINE and the LABEL, because the range belongs to the
//! label.** Having stripped the suffix for the name, the first run printed
//! `routine=वाक्यविभागकारकनाम [+22104, +22108)` — a routine against a FOUR-OCTET
//! span, because the containing label is a block and the next block begins four
//! octets on. **Nothing about that line looks malformed**, and a reader concludes
//! the routine is one instruction long. Defects 1 and 2 produced refusals; this
//! one produced a wrong fact, from mixing two granularities in a single line. So
//! both are printed, and `पर्व३२` / `पर्व५६८` are visible as blocks.
//!
//! # Why it brackets the base instead of computing it
//!
//! The census lays the image `[startup, own, needed…]`, so a source's own text
//! begins after the startup's — 56 octets with an entry, 28 without — plus
//! alignment padding this does not model. **So it tries several plausible bases
//! and answers only when they AGREE**, which is cheaper than computing the exact
//! one and fails LOUDLY rather than plausibly. The bracket disagreeing five ways
//! is the only reason defect 1 above was caught before a data constant was
//! reported as a routine.
//!
//! Containment is strict — `[lo, hi)` with the next symbol as the bound — and
//! there is **no nearest-below fallback**, because a sorted list always yields a
//! nearest-below answer even when the offset lies in padding, in a gap, or past
//! the end, and that answer is indistinguishable from a real hit.
//!
//! # What it cannot do
//!
//! An offset beyond `startup + own_text` lies in a LATER object of the image;
//! this reports that rather than guessing. `shrinkhala.t1`'s `+164492` across 16
//! module objects is such a case and has no routine named yet.
//!
//! **And the routine is where the machine WAS, not necessarily where the defect
//! is.** A wrong address computed in one routine faults where it is
//! dereferenced. The name opens the next question; it does not answer it.

use sadhana::encode::Target;
use sadhana::nidana::Language;
use sadhana::t1::chain::{self, Front};
use sadhana::t1::riscv64;
use sadhana::{assemble_object, vastu};
use std::path::{Path, PathBuf};

/// `pc − LOAD_ADDRESS` and `addr`, as the census last reported them.
///
/// `LOAD_ADDRESS` is `0x8000_0000` = 2147483648, so `artha`'s `pc: 2147483800`
/// is text offset 152. **These are TEXT OFFSETS, not addresses** — an
/// off-by-load-address error here produces a plausible NAME rather than an
/// error, which is why the arithmetic is stated rather than inlined.
const FAULTS: &[(&str, u64, u64)] = &[
    ("artha.t1", 152, 8),
    ("ir.t1", 1164, 8),
    ("samyojana.t1", 4612, 0),
    ("encode.t1", 17920, 0),
    ("vakyavibhaga.t1", 22200, 0),
    ("shrinkhala.t1", 164492, 0),
];

/// Plausible placements of a source's text after the startup's.
const BASES: &[u64] = &[48, 56, 64, 80, 96];

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// A label's ROUTINE: the label with any block or exit suffix removed.
///
/// `अर्थअर्थप्रकारयोजनम्पर्व१` and `…निर्गम` both belong to the routine
/// `अर्थअर्थप्रकारयोजनम्`. Without this, two adjacent blocks of one routine read
/// as two different answers and the bracket refuses a question it could answer.
fn routine_of(label: &str) -> &str {
    // LOOPS, because one strip is not enough if a label ever carries BOTH — a
    // `…पर्व१निर्गम` would hand back a block label silently. No such label has
    // been observed; the loop costs nothing and the silent case is the one that
    // would be believed.
    let mut cut = label;
    loop {
        if let Some(shorter) = cut.strip_suffix("निर्गम") {
            cut = shorter;
            continue;
        }
        if let Some(at) = cut.rfind("पर्व") {
            let tail = &cut[at + "पर्व".len()..];
            if !tail.is_empty() && tail.chars().all(|c| ('०'..='९').contains(&c)) {
                cut = &cut[..at];
                continue;
            }
        }
        return cut;
    }
}

#[test]
#[ignore = "measurement: compiles every faulting corpus source through the front end, minutes"]
fn which_routine_holds_each_faulting_offset() {
    for (file, offset, addr) in FAULTS {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../sadhana-t1/src")
            .join(file);
        let src = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                println!("METRIC t1_fault_routine_{file} UNREADABLE {e}");
                continue;
            }
        };
        let built = (|| -> Result<(usize, Vec<(u64, String)>), String> {
            let mut front = Front::load(&spec_root()).map_err(|e| format!("front: {e}"))?;
            front.lex(&src).map_err(|e| format!("lex: {e}"))?;
            front.parse().map_err(|e| format!("parse: {e}"))?;
            front.resolve().map_err(|e| format!("resolve: {e}"))?;
            front.typecheck().map_err(|e| format!("typecheck: {e}"))?;
            front.build_ir().map_err(|e| format!("ir: {e}"))?;
            let name = chain::module_name(&src).ok_or("the source declares no module")?;
            let module = front
                .module(&name, None)
                .map_err(|e| format!("module: {e}"))?;
            let text = riscv64::emit_module(&module).map_err(|r| format!("emit: {r:?}"))?;
            let obj = assemble_object(
                &text,
                Some(&name),
                Target::Uncompressed,
                false,
                Language::English,
            )
            .map_err(|ds| format!("assemble: {} diagnostic(s)", ds.len()))?;
            let o = vastu::read(&obj).ok_or("the object does not read back")?;
            let mut syms: Vec<(u64, String)> = o
                .symbols
                .iter()
                .filter(|s| s.placement == vastu::Placement::Text)
                .map(|s| (s.value, s.name.clone()))
                .collect();
            syms.sort();
            syms.dedup();
            Ok((o.text.len(), syms))
        })();

        let (text_len, syms) = match built {
            Ok(v) => v,
            // A front-end refusal is a DIFFERENT fact from the census's run
            // fault, and reporting it as one keeps the two unreconciled rather
            // than telling one story about them.
            Err(e) => {
                println!("METRIC t1_fault_routine_{file} STOPPED {e}");
                continue;
            }
        };

        let containing = |want: u64| -> Option<(String, u64, u64)> {
            if want >= text_len as u64 {
                return None;
            }
            for pair in syms.windows(2) {
                if pair[0].0 <= want && want < pair[1].0 {
                    return Some((pair[0].1.clone(), pair[0].0, pair[1].0));
                }
            }
            syms.last()
                .filter(|(v, _)| *v <= want)
                .map(|(v, n)| (n.clone(), *v, text_len as u64))
        };

        let mut named: Vec<String> = Vec::new();
        let mut at_label: Option<(String, u64, u64)> = None;
        for base in BASES {
            if offset < base {
                named.push("BELOW-STARTUP".to_string());
                continue;
            }
            match containing(offset - base) {
                Some((label, lo, hi)) => {
                    named.push(routine_of(&label).to_string());
                    at_label = Some((label, lo, hi));
                }
                None => named.push(format!("PAST-OWN-TEXT({text_len})")),
            }
        }
        named.sort();
        named.dedup();
        let agreed = named.len() == 1;
        println!(
            "METRIC t1_fault_routine_{file} +{offset} addr={addr} own_text={text_len} {} {}",
            if agreed { "AGREE" } else { "DISAGREE" },
            if agreed {
                // THE RANGE IS THE CONTAINING LABEL'S, NOT THE ROUTINE'S, and
                // saying so is the difference between a fact and a wrong one:
                // `वाक्यविभागकारकनाम` reported `[+22104, +22108)` — four octets,
                // one instruction — because the containing label is a BLOCK and
                // the next block begins four octets on. A reader given the
                // routine's NAME beside a block's SPAN concludes the routine is
                // four octets long.
                match at_label {
                    Some((label, lo, hi)) => {
                        format!("routine={} at label={label} [+{lo}, +{hi})", named[0])
                    }
                    None => named[0].clone(),
                }
            } else {
                format!("{named:?}")
            }
        );
    }
}
