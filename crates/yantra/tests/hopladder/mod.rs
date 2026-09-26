//! **THE HOP LADDER, SHARED BY BOTH READINGS OF THE TRACKED NUMBER.**
//!
//! This module NAMES NO SYMBOL FROM `chain.rs`, and that is the point rather
//! than an accident: the strict figure means "no symbol from the Rust driver at all",
//! so the code measuring it must be reachable without one. The loader is passed
//! IN as a closure — the lenient caller builds an `Interpreter` from the
//! Rust driver module's embedded corpus, the strict caller from
//! `Interpreter::load_paths` over files
//! read off disk — and the ladder itself cannot tell which it was handed.
//!
//! **THE ABSENCE IS CHECKABLE, NOT INTENDED.** `no_chain_reference_in` reads
//! this file and its caller's source and asserts neither names the Rust driver module.
//! An import that creeps in later turns the strict figure into a second copy of
//! the lenient one, silently, and that is precisely how the 17-of-20 Rust-driven
//! ladder came to be read as the self-hosting one for weeks.
//!
//! THE LADDER:
//!   0  nothing — the front half produced no text
//!   1  EMIT      — Sassembly text exists
//!   2  ASSEMBLE  — that text became an object
//!   3  LINK      — the object became an ELF image
//!   4  LOAD      — `Machine::load_elf` accepted it
//!   5  RUN       — it executed without faulting

// EACH INTEGRATION-TEST BINARY COMPILES ITS OWN COPY of this module, so an item
// used by one arm is dead in the other. `no_chain_reference_in` is used only by
// the strict arm and `reach_with` by both; the allow is about Rust's per-binary
// view, not about anything here being unused.
#![allow(dead_code)]

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::Path;
use yantra::Machine;

pub const RAM: usize = yantra::DEFAULT_RAM;
pub const BUDGET: u64 = 200_000_000;

/// What a source reached, with **"could not run" kept distinct from "hop zero"**.
/// Different facts, and only one is about the product: hop 0 says the compiler
/// refused the source; `CouldNotRun` says the harness never got far enough to ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reach {
    CouldNotRun(String),
    Hops(u8),
}

impl Reach {
    /// −1 and not 0: a harness that cannot start must never be readable as a
    /// product that reached nothing.
    pub fn number(&self) -> i64 {
        match self {
            Reach::CouldNotRun(_) => -1,
            Reach::Hops(n) => i64::from(*n),
        }
    }
}

pub fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

/// A `.t1` octet global as text, for a diagnostic line. An empty run and an
/// absent global read differently on purpose: a refusal that recorded no symbol
/// is a different fact from one this side could not find.
pub fn text_of(v: Option<&Value>) -> String {
    match v {
        Some(Value::Octets(o)) if o.as_slice().is_empty() => "<empty>".to_string(),
        Some(Value::Octets(o)) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        Some(other) => format!("{other:?}"),
        None => "<no such global>".to_string(),
    }
}

/// **ASSERT THAT A SOURCE FILE NAMES NO `chain.rs` SYMBOL.** Used by the strict
/// arm on its own source and on this module, so "reaches no Rust driver" is a
/// checked property of the text rather than a claim in a margin.
/// `यन्त्रनिषेधलक्ष्य` as the reader needs it: for `UnnamedSymbol` (भेद ४) it is a
/// SITE CODE, and the site is the thing a reader of this ladder wants — five
/// routines raise that one variant and the record is otherwise identical at four of
/// them. For every other variant it is what it always was, a target, and is printed
/// unchanged. **THE NAMES ARE READ FROM THE MODULE, NOT LISTED HERE**: each is the
/// public constant `यन्त्रोत्सर्जन` declares, so a renumbering on that side moves this
/// line with it, and a code with no matching constant prints as unrecognised rather
/// than as the previous name.
pub fn unnamed_symbol_site(it: &Interpreter) -> String {
    let int = |n: &str| it.global(n).and_then(Value::as_int);
    let target = int("यन्त्रनिषेधलक्ष्य");
    if int("यन्त्रनिषेधभेद") != int("यन्त्रानामसंज्ञानिषेधभेद")
    {
        return format!("{target:?}");
    }
    const SITES: &[&str] = &[
        "यन्त्रनामस्थानचिह्नम्",
        "यन्त्रनामस्थानपरीक्षा",
        "यन्त्रनामस्थानाह्वानम्",
        "यन्त्रनामस्थानवृत्तिः",
        "यन्त्रनामस्थानप्रवेशः",
    ];
    match SITES.iter().find(|n| int(n).is_some() && int(n) == target) {
        Some(n) => format!("{target:?} ({n})"),
        // ० IS THE PRE-2026-09-17 VALUE AT EVERY SITE, so it is named as that and
        // not merely as unknown: a tree whose emitter predates the site codes reads
        // here, and "no site recorded" is the true statement about it.
        None if target == Some(0) => "Some(0) (NO SITE RECORDED)".to_string(),
        None => format!("{target:?} (no such site constant)"),
    }
}

pub fn no_chain_reference_in(path: &Path) {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", path.display()));
    // **THE NEEDLES ARE BUILT AT RUNTIME SO THIS FILE DOES NOT CONTAIN THEM.**
    // A first version wrote them as literals and the checker matched ITS OWN
    // needle list — three hits in a file that references nothing. A scanner whose
    // pattern appears in the text it scans reports the pattern, not the property.
    let sep = "::";
    let needles = [
        format!("t1{sep}chain"),
        format!("chain{sep}"),
        format!("CH{}", "AIN"),
    ];
    for needle in &needles {
        assert!(
            !text.contains(needle),
            "{} names `{needle}`. The STRICT figure means no symbol from the Rust driver \
             at all; an import here makes it a second copy of the lenient figure and \
             nothing would say so.",
            path.display()
        );
    }
}

/// Drive one source to the furthest hop it reaches, given a loader.
pub fn reach_with<F>(build: F, module: &str, src: &str) -> Reach
where
    F: FnOnce() -> Result<Interpreter, String>,
{
    let mut it = match build() {
        Ok(i) => i,
        Err(e) => return Reach::CouldNotRun(e),
    };

    // HOP 1 — EMIT.
    let text = match it.call(
        "शृङ्खलाॱमण्डलसङ्कलनम्",
        vec![octets(src.as_bytes()), octets(module.as_bytes())],
        80_000_000_000,
    ) {
        Ok(v) => v
            .octets()
            .map(|o| o.as_slice().to_vec())
            .unwrap_or_default(),
        Err(e) => return Reach::CouldNotRun(format!("मण्डलसङ्कलनम्: {e:?}")),
    };
    if text.is_empty() {
        // WHY IT EMITTED NOTHING IS ALREADY RECORDED — ASK. `मण्डलसङ्कलनम्` sets the
        // public `सङ्कलनविरामभेद` on every exit: a sentinel on entry, then
        // `सङ्कलनाघोषणाभेद` when nothing parsed and `सङ्कलनानिर्णयभेद` when the
        // checker refused. Its own margin explains the sentinel — "a return that
        // forgets to set the code reports emitted, and this routine's whole
        // history is returns that recorded nothing" — and the ladder then threw
        // the code away, so ten sources sat at `Hops(0)` with no cause. The
        // `Hops(1)`/`Hops(2)` split four lines below already reads a global.
        //
        // AND THE EXIT CODE CANNOT PARTITION THE TEN. `सङ्कलनसिद्धभेद` is the
        // SUCCESS code, set unconditionally at one point, so every source that
        // reaches the unexamined return reports it — one EXIT, cause count
        // unknown. The discriminator is one module further down and already
        // exists: `यन्त्रमण्डलोत्सर्जनम्` records its own refusal in the public
        // `यन्त्रनिषेधमस्ति`/`यन्त्रनिषेधभेद`, with the routine label in
        // `यन्त्रनिषेधवृत्ति`, and returns the empty run. Nothing outside
        // `यन्त्रोत्सर्जन` reads any of them.
        eprintln!(
            "  HOP 1 emitted nothing: सङ्कलनविरामभेद={:?} यन्त्रनिषेधमस्ति={:?} \
             यन्त्रनिषेधभेद={:?} संख्या={:?} पर्व={:?} लक्ष्य={} वृत्ति={}",
            it.global("सङ्कलनविरामभेद"),
            it.global("यन्त्रनिषेधमस्ति"),
            it.global("यन्त्रनिषेधभेद"),
            // THE SUBJECT IS HERE, NOT IN `चिह्न`. `यन्त्रनिषेधः` takes five
            // parameters and writes five globals; `यन्त्रनिषेधचिह्न` is not one of
            // them and has a single writer elsewhere in the file. Every
            // `UnnamedSymbol` site passes its symbol as the FIFTH argument, which
            // binds to `संख्या` — and there are FIVE such sites, each naming a
            // different subject (a symbol, a routine's name, a callee's symbol, an
            // entry symbol), all passing `वृत्ति` as `रिक्तम्` deliberately.
            //
            // **AND `संख्या` DOES NOT SEPARATE THEM, WHICH IS WHAT THIS MARGIN USED
            // TO CLAIM.** It said "the variant alone does not localise; this field is
            // what separates them" — true only while the symbol is NON-ZERO, and the
            // symbol is ० in exactly the case that brings a source here: a lookup of
            // symbol ० misses, so four of the five sites answer the identical record
            // `भेद ४, पर्व ०, लक्ष्य ०, संख्या ०, वृत्ति ""`. On 2026-09-16 four corpus
            // sources at hop 0 were read off this line as sharing ONE cause; that
            // pair of values names a FAMILY OF FOUR SITES and cannot support it.
            // `यन्त्रोत्सर्जन` now passes a SITE CODE in `लक्ष्य` at each of the five,
            // and `unnamed_symbol_site` below names it. Reproduced and held by
            // `crates/sadhana-t1/tests/t1_refusal_site.rs`, whose refused case is
            // both codes mutated back to ० so the collapse is a measurement.
            it.global("यन्त्रनिषेधसंख्या"),
            it.global("यन्त्रनिषेधपर्व"),
            unnamed_symbol_site(&it),
            text_of(it.global("यन्त्रनिषेधवृत्ति")),
        );
        return Reach::Hops(0);
    }

    // HOPS 2 AND 3 — ASSEMBLE and LINK, both inside `मण्डलानिप्रतिबिम्बम्`,
    // separated by the flag the object builder sets: the call answers empty
    // octets from either and the return value names neither.
    let arena = |vs: Vec<Value>| Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)));
    let image = match it.call(
        "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
        vec![
            arena(vec![octets(src.as_bytes())]),
            arena(vec![octets(module.as_bytes())]),
            Value::Int(1),
        ],
        80_000_000_000,
    ) {
        Ok(v) => v
            .octets()
            .map(|o| o.as_slice().to_vec())
            .unwrap_or_default(),
        Err(e) => return Reach::CouldNotRun(format!("मण्डलानिप्रतिबिम्बम्: {e:?}")),
    };
    if image.is_empty() {
        let refused_object = matches!(it.global("वस्तुरचनानिषेधः"), Some(Value::Bool(true)));
        // WHICH OF THE TWO HOP-2 CAUSES, FROM A COUNT THE PRODUCT ALREADY WRITES.
        // `मण्डलानिप्रतिबिम्बम्` answers an empty run from ASSEMBLE or from LINK and
        // "the return value names neither" (this file's own margin above). But
        // `shrinkhala.t1:2391` sets `संयोजितवस्तुसंख्या` BEFORE the link, and says
        // why: "so it says what went IN even when the link refuses. A count
        // written only on success cannot distinguish 'nothing was collected' from
        // 'the link failed', and those need different fixes."
        //   0  -> nothing was collected; the objects never reached the linker
        //   >0 -> that many objects went in and the LINK refused
        // Nine of twenty sources sit at this hop — the ladder's largest
        // population — and nothing had read the count.
        if !refused_object {
            // AND THE LINKER'S OWN DIAGNOSTIC, WHICH NOTHING READ. `samyojana.t1`
            // records every refusal of the current link in `संयोजनदोषकोश` — a
            // one-based arena of `संयोजनदोष { कूट, नाम, स्थानाङ्कः, मूल्यम् }` —
            // with EIGHT named kinds at `:676-683`, from "`{}` is not defined by
            // any object" to "`{}` has no 32-bit encoding". There is a recorder
            // and no formatter, so it has never been rendered anywhere.
            const KIND: [&str; 9] = [
                "-",
                "अपरिभाषित — not defined by any object",
                "पुनरुक्त — defined by more than one object",
                "असत्संज्ञा — a relocation names a symbol that is not there",
                "अनुपयुक्तभेद — relocation type not applied yet",
                "दत्तातिक्रम — a data relocation points past .data",
                "पाठातिक्रम — a relocation points past the text",
                "अविश्लेष्य — nothing decodes there",
                "असङ्केत — no 32-bit encoding",
            ];
            let first = match it.global("संयोजनदोषकोश") {
                Some(Value::Arena(a)) => a.borrow().get(1).cloned(),
                _ => None,
            };
            let detail = match first {
                Some(Value::Record(r)) => {
                    let r = r.borrow();
                    let k = match r.get("कूट") {
                        Some(Value::Int(i)) => *i as usize,
                        _ => 0,
                    };
                    format!(
                        "kind {k} = {}  नाम={}",
                        KIND.get(k).unwrap_or(&"?"),
                        text_of(r.get("नाम"))
                    )
                }
                _ => "no record at index 1".to_string(),
            };
            eprintln!(
                "  HOP 2 image empty: संयोजितवस्तुसंख्या={:?} दोषसूचकाङ्क={:?}\n      first link refusal: {detail}",
                it.global("संयोजितवस्तुसंख्या"),
                it.global("संयोजनदोषसूचकाङ्क"),
            );
        }
        return Reach::Hops(if refused_object { 1 } else { 2 });
    }

    // HOP 4 — LOAD, by the shipped loader and not by anything of ours.
    //
    // RAM IS SIZED FROM THE IMAGE, NOT FROM `DEFAULT_RAM`. `fa8fbc05` raised the
    // record region to 320 MiB for a full native self-compile, and this call kept
    // handing `load_elf` the 20 MiB constant — so every image declaring the new
    // region was refused AT LOAD and the ladder reported `Hops(3)`. Four sources
    // left `reached_run` between 09-13 and 09-15 and none of them had changed.
    // `yantra::ram_for` is the one statement of that sizing; `yantra-run` and the
    // census's runner already use it, which is why the same images ran correctly
    // there on the same day — two loaders, two answers.
    //
    // AND THE ERROR IS PRINTED RATHER THAN DISCARDED. `Err(_)` cost a seven-step
    // bisect to learn what the loader says outright: "segment needs 335,616,120
    // bytes and RAM is 20,971,520".
    let mut m = match Machine::load_elf(&image, yantra::ram_for(&image)) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("  HOP 4 load refused: {e:?}");
            return Reach::Hops(3);
        }
    };

    // HOP 5 — RUN. `BeyondRam` is what a startup that never set `sp` produces,
    // and no correct image reaches it.
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(BUDGET, &mut out);
    if matches!(halt, yantra::Halt::BeyondRam { .. }) {
        return Reach::Hops(4);
    }
    Reach::Hops(5)
}
