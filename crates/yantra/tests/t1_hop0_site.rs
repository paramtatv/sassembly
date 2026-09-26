//! **THE FOUR SOURCES THAT STOPPED AT HOP 0 NOW EMIT, AND THE ONE LINE THAT MOVED
//! THEM IS MUTATED HERE TO PUT THEM BACK.**
//!
//! This file was written on 2026-09-17 to LOCALISE the refusal: `81c2ccc7` gave
//! `Refusal::UnnamedSymbol` (`यन्त्रानामसंज्ञानिषेधभेद`, भेद ४) a site code in
//! `यन्त्रनिषेधलक्ष्य`, because four of its five raising sites in `यन्त्रोत्सर्जन` wrote
//! the byte-identical record `भेद ४, पर्व ०, लक्ष्य ०, संख्या ०, वृत्ति ""` — the
//! offending symbol is ० in exactly the case that brings a source to hop 0, so
//! `संख्या` separates nothing there. Driving each source to hop 1 AND NO FURTHER,
//! on a fresh interpreter, answered the column in 51 s where the strict ladder
//! needs 1,651: all four read `लक्ष्य ३ = यन्त्रनामस्थानाह्वानम्` —
//! `यन्त्राह्वानोत्सर्जनम्`, the CALLEE's symbol.
//!
//! **`W-280` FIXED IT AND THIS FILE IS THE FALSIFIER ITS OWN LEDGER ENTRY NAMED.**
//! `ir.t1`'s `अभिव्यञ्जकरचना` accepted W-228(b)'s FOLDED spaced qualifier —
//! `सङ्केतन ॱ क्षेत्रारम्भः`, one नाम node with the member token in `दक्षिणसूचकाङ्क` —
//! by writing `स्वीकृतम् भवति सत्यम्` and nothing else, so `आह्वेयसंज्ञा` stayed at the
//! ० of its declaration, the `आह्वान` was appended with `संज्ञा ०`, and
//! `यन्त्रनामान्वेषणम् ०` missed. MEASURED before the fix, over all 21 corpus sources:
//! 128 calls emitted with `संज्ञा ०`, every one of them in these four and NONE in the
//! other seventeen. The folded shape now mints through `बाह्यसंज्ञाग्रहणम्`, shared
//! with the one-token shape that always did.
//!
//! ```text
//!                      before            after
//!   samyojana.t1       भेद 4 लक्ष्य 3     emits
//!   sanskrit_text.t1   भेद 4 लक्ष्य 3     emits
//!   vakyavibhaga.t1    भेद 4 लक्ष्य 3     emits
//!   vishlesana.t1      भेद 4 लक्ष्य 3     emits
//! ```
//!
//! **THE CASE THAT MUST STILL BE REFUSED IS NOW THE MUTATION OF THE FIX ITSELF.**
//! A file that only asserts "these four emit" would pass against a chain that had
//! stopped compiling them for some entirely different reason, and — worse — the
//! localisation machinery above (`unnamed_symbol_site`, the site codes, the
//! `लक्ष्य` field) would be carried with nothing exercising it. So the second test
//! puts the ONE LINE back: `ir.t1`'s mint call becomes `स्वीकृतम् भवति सत्यम्`, which
//! is verbatim what stood there before `W-280`, and all four must return to
//! refusing with `भेद ४` at `यन्त्रनामस्थानाह्वानम्`. It is a MUTATION OF THE SHIPPED
//! SOURCE, not a fixture, so it cannot drift away from what is actually compiled —
//! the day that line is rewritten, `mint_removed` fails on its own
//! `assert_ne!` rather than passing while mutating nothing.
//!
//! **ONE FRESH INTERPRETER PER SOURCE, AND THAT IS NOT AN OPTIMISATION TO REMOVE.**
//! `यन्त्रनिषेधः` keeps the FIRST refusal and returns early on every later one, so two
//! sources measured on one load would both report the first one's record and this file
//! would pass while measuring a single source twice.

mod hopladder;

use hopladder::{no_chain_reference_in, octets, unnamed_symbol_site};
use sadhana::t1::nirvahana::{Interpreter, Value};
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../sadhana-t1/src")
}

/// Every `.t1` in the corpus directory, read off DISK — the list comes from the
/// filesystem and is NOT trimmed to match any Rust-side roster, for the same reason
/// the strict ladder does not trim it: a file that breaks the load is a real
/// difference and must surface rather than be filtered away.
fn corpus_files() -> Vec<(String, String)> {
    let dir = corpus_dir();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|p| {
            let name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let text = std::fs::read_to_string(p)
                .unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()));
            (name, text)
        })
        .collect()
}

fn declared_module(src: &str) -> Option<String> {
    src.lines()
        .map(str::trim_start)
        .find(|l| l.starts_with("मण्डलम्"))
        .and_then(|l| l.split_whitespace().nth(1))
        .map(ToString::to_string)
}

/// THIS TEST'S OWN LOADER. `replaced` swaps one corpus member's text for a mutant,
/// which is how the refused case below is produced FROM THE SHIPPED SOURCE rather
/// than from a fixture that could drift away from it silently.
fn load_corpus(files: &[(String, String)], replaced: Option<(&str, &str)>) -> Interpreter {
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, t)| match replaced {
            Some((name, text)) if name == n.as_str() => (n.as_str(), text),
            _ => (n.as_str(), t.as_str()),
        })
        .collect();
    Interpreter::load(&refs, &spec_root())
        .unwrap_or_else(|e| panic!("corpus load of {} disk sources: {e:?}", refs.len()))
}

/// What one source's front half answered. `emitted` is kept apart from the site
/// because a source that EMITS has no refusal to name, and reading the two off one
/// field is how "no site recorded" and "did not refuse" would come to look alike.
struct Front {
    emitted: usize,
    variant: Option<i128>,
    target: Option<i128>,
    site: String,
}

/// Drive one source to hop 1 and no further, on a FRESH interpreter.
fn front_half(files: &[(String, String)], file: &str, replaced: Option<(&str, &str)>) -> Front {
    let src = files
        .iter()
        .find(|(n, _)| n == file)
        .map(|(_, t)| t.clone())
        .unwrap_or_else(|| panic!("{file} is in the corpus directory"));
    let module = declared_module(&src).unwrap_or_else(|| panic!("{file} declares a मण्डलम्"));
    let mut it = load_corpus(files, replaced);
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(src.as_bytes()), octets(module.as_bytes())],
            80_000_000_000,
        )
        .unwrap_or_else(|e| panic!("{file}: मण्डलसङ्कलनम् could not run: {e:?}"))
        .octets()
        .map(|o| o.as_slice().len())
        .unwrap_or(0);
    Front {
        emitted: text,
        variant: it.global("यन्त्रनिषेधभेद").and_then(Value::as_int),
        target: it.global("यन्त्रनिषेधलक्ष्य").and_then(Value::as_int),
        site: unnamed_symbol_site(&it),
    }
}

/// The four sources the 2026-09-16/17 census left at hop 0, and which `W-280`
/// moved. Stated here rather than discovered, so a source that goes BACK to
/// emitting nothing fails this file by name instead of quietly leaving the roster
/// — a census that shrinks to fit its answer measures the answer.
const WAS_HOP_ZERO: &[&str] = &[
    "samyojana.t1",
    "sanskrit_text.t1",
    "vakyavibhaga.t1",
    "vishlesana.t1",
];

/// **THE LINE `W-280` ADDED, AND THE LINE THAT STOOD THERE BEFORE IT.** The
/// mutation below swaps the first for the second — which is the pre-fix source
/// exactly: accept the folded spaced qualifier, mint nothing, emit `संज्ञा ०`.
const MINT: &str = "आह्वेयसंज्ञा भवति बाह्यसंज्ञाग्रहणम् आरभ्य पदविभागॱचिह्नकपाठः पृथङ्मण्डलचिह्नकम् ऽ पदविभागॱचिह्नकपाठः पृथक्सदस्यचिह्नकम् समाप्तम् ।";
const ACCEPT_ONLY: &str = "स्वीकृतम् भवति सत्यम् ।";

fn mint_removed(ir: &str) -> String {
    let out = ir.replace(MINT, ACCEPT_ONLY);
    assert_ne!(
        out, ir,
        "the mutation changed nothing — `ir.t1` no longer carries the folded-qualifier \
         mint as this file spells it, so the refused case below would pass against any \
         emitter at all. Re-take `MINT` from the source before trusting either test here."
    );
    out
}

#[test]
fn the_four_that_stopped_at_hop_zero_now_emit() {
    let here = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/t1_hop0_site.rs");
    let ladder = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/hopladder/mod.rs");
    no_chain_reference_in(&here);
    no_chain_reference_in(&ladder);

    let files = corpus_files();
    println!("METRIC t1_hop0_site_disk_sources {}", files.len());

    for file in WAS_HOP_ZERO {
        let started = std::time::Instant::now();
        let front = front_half(&files, file, None);
        println!(
            "METRIC t1_hop0_site_emitted_{} {}",
            file.replace(".t1", ""),
            front.emitted
        );
        println!(
            "  {file:18} emitted {:>7}  भेद {:?}  {}  {:5.1}s",
            front.emitted,
            front.variant,
            front.site,
            started.elapsed().as_secs_f64()
        );
        assert!(
            front.emitted > 0,
            "{file} emitted NOTHING — it is back at hop 0. `भेद {:?}`, {}. That is \
             movement backwards and must be said, not absorbed.",
            front.variant,
            front.site
        );
    }
}

/// **THE CASE THAT MUST STILL BE REFUSED.** With `W-280`'s mint put back to the bare
/// accept it replaced, every one of the four must return to refusing with
/// `यन्त्रानामसंज्ञानिषेधभेद` at `यन्त्रनामस्थानाह्वानम्` — the callee's symbol. This is
/// the mutation of the shipped `ir.t1`, so it cannot drift away from what is compiled,
/// and it is what keeps the site-code machinery above exercised rather than merely
/// carried.
#[test]
fn without_the_mint_the_same_four_refuse_at_the_callees_symbol() {
    let files = corpus_files();
    let ir = files
        .iter()
        .find(|(n, _)| n == "ir.t1")
        .map(|(_, t)| t.clone())
        .expect("ir.t1 is in the corpus directory");
    let mutant = mint_removed(&ir);

    let unnamed = {
        let it = load_corpus(&files, None);
        it.global("यन्त्रानामसंज्ञानिषेधभेद")
            .and_then(Value::as_int)
            .expect("`यन्त्रानामसंज्ञानिषेधभेद` is a numeric global of the corpus")
    };
    let call_site = {
        let it = load_corpus(&files, None);
        it.global("यन्त्रनामस्थानाह्वानम्")
            .and_then(Value::as_int)
            .expect("`यन्त्रनामस्थानाह्वानम्` is a numeric global of the corpus")
    };

    for file in WAS_HOP_ZERO {
        let front = front_half(&files, file, Some(("ir.t1", &mutant)));
        println!(
            "  {file:18} without the mint: emitted {:>7}  भेद {:?}  लक्ष्य {:?}  {}",
            front.emitted, front.variant, front.target, front.site
        );
        assert_eq!(
            front.emitted, 0,
            "{file} EMITTED with the mint removed, so the mint is not what moved it \
             and the test above measures something else."
        );
        assert_eq!(
            front.variant,
            Some(unnamed),
            "{file} refused with भेद {:?}, not `यन्त्रानामसंज्ञानिषेधभेद` ({unnamed}) — \
             a different variant is a different defect and this pair does not localise it.",
            front.variant
        );
        assert_eq!(
            front.target,
            Some(call_site),
            "{file} refused at लक्ष्य {:?}, not `यन्त्रनामस्थानाह्वानम्` ({call_site}) — \
             the un-minted call is supposed to reach `यन्त्राह्वानोत्सर्जनम्` and nowhere else.",
            front.target
        );
    }
}
