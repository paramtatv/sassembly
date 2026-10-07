//! **REFUSAL-CAUSE WITNESSES.** Every named refusal cause the compiler can
//! raise, each mapped to a witness that raises EXACTLY that cause, on every
//! front end that has it — and a ratchet on the ones with no witness.
//!
//! # What a cause is, and where the list comes from
//!
//! The causes are READ FROM THEIR STORAGE, never listed by hand, so a cause
//! added to a table after this file was written shows up here as UNCOVERED
//! instead of being silently missed:
//!
//! | family | storage | front ends |
//! |---|---|---|
//! | `diag` | `spec/diagnostics.tsv`'s `code` column | Rust T0 assembler, `.t1` T0 reader |
//! | `build` | `shrinkhala.t1`'s `वस्तुरचना…भेद` verdicts | `.t1` object builder |
//! | `compile` | `shrinkhala.t1`'s `सङ्कलन…भेद` and `निर्णय…भेद` verdicts | `.t1` chain |
//! | `stub` | `ir.t1`'s literal stub raises, named by `paradigm_encode.rs`'s `STUB_CAUSES` | `.t1` IR |
//! | `finisher` | `ir.t1`'s `…निषेधः` finisher values (`0x355`, `0x359`, …) | interpreter and native |
//! | `emit` | `yantrotsarjana.t1`'s `यन्त्र…निषेधभेद` rows, paired by name with `riscv64.rs`'s `Refusal` | Rust emitter, `.t1` emitter |
//! | `link` | `samyojana.t1`'s `…कूटः` linker causes | `.t1` linker |
//! | `status` | `spec/entry/assemble.t1`'s literal status returns | `.t1` wrapper |
//! | `reader` | `assemble.t1`'s `पठनविरामः भवति N` sites (status `४००० + N`) | `.t1` wrapper |
//! | `halt` | `yantra/src/lib.rs`'s `enum Halt` | native machine |
//! | `gate` | `sanskrit_text.t1`'s R-15-1 gate causes `परिधि…दोषः` | `.t1` source gate |
//! | `fail-open` | below, `FAIL_OPEN`: a cause the code promises and does not raise | — |
//!
//! OUT OF SCOPE, stated: the T1 PARSER's and TYPE CHECKER's own refusals — Rust's
//! `t1/parse.rs`, `resolve.rs`, `typecheck.rs` and the `.t1` parser's
//! `व्याकरॱदोषकोश` — carry free text and no cause code, so there is no table to
//! read them from. Only the chain's VERDICT that they refused (`compile`) is here.
//!
//! THE `.t1` T0 SIDE SEES ONLY THE ENCODER'S LAST ERROR: the encoder keeps one
//! record, `अन्तिमसङ्केतनदोषः`, where Rust returns every `EncodeError`. A `.t1`
//! T0 probe that "makes exactly [code]" therefore means the splitter's whole
//! record plus the encoder's LAST code; an earlier encoder error is invisible.
//!
//! The unit counted is a (cause, front end) PAIR. A pair is covered when a
//! witness exists for it:
//!
//! * a PROBE run here — a text and the front end, asserted to raise exactly
//!   that cause and nothing else (a probe that stops doing so is RED, so a
//!   broken probe can never quietly turn into an uncovered row);
//! * or a REFERENCED test elsewhere, `file::fn` — checked to exist, to be a
//!   `#[test]` that is not `#[ignore]`d, and to ASSERT the cause: its token
//!   must stand in CODE (comments stripped) in the body BELOW the signature —
//!   the test's own name never counts — inside an `assert`/`panic!`/`matches!`
//!   statement, or bound by a `let` whose name such a statement then uses.
//!
//! # The ratchet
//!
//! `UNCOVERED_PIN` is today's uncovered count; the count may only go down. A
//! new cause in any table raises it and goes red here, naming the cause.
//! Lower the pin when a witness lands.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Today's uncovered (cause, front end) pairs. May only go DOWN.
// 112 -> 115 on 2026-10-05, the batch merge onto V-005 (b0b19d31): V-005 added
// the FileMismatch refusal (यन्त्रवर्गविरोधनिषेधभेद) on both emitters and one more
// emitter cause. Their witnesses exist in crates/yantra/tests/v005_floats.rs,
// which this scanner does not read yet; teaching it to is the way back down.
const UNCOVERED_PIN: usize = 115;

const LOAD: u64 = 0x8000_0000;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let p = repo_root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} reads: {e}", p.display()))
}

/// A Devanagari numeral as a number; `None` for anything else.
fn deva(s: &str) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    s.chars().try_fold(0u64, |n, c| {
        let d = (c as u32).checked_sub(0x0966).filter(|d| *d <= 9)?;
        Some(n * 10 + u64::from(d))
    })
}

/// Code lines of a `.t1` text — a `॰`-led line is prose.
fn code_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('॰'))
}

/// Every `सार्वजनिक चरः <name> ॱॱ <ty> भवति <N> ।` in `text`, as (name, N).
fn declared_ints<'a>(text: &'a str, ty: &str) -> Vec<(&'a str, u64)> {
    let mid = format!(" ॱॱ {ty} भवति ");
    code_lines(text)
        .filter_map(|l| {
            let rest = l.strip_prefix("सार्वजनिक चरः ")?;
            let (name, tail) = rest.split_once(mid.as_str())?;
            let n = deva(tail.split_whitespace().next()?)?;
            Some((name, n))
        })
        .collect()
}

// ═══════════════════════════ the causes, from storage ═══════════════════════════

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Cause {
    family: &'static str,
    key: String,
    /// What the cause is called where it is stored.
    name: String,
    front_ends: Vec<&'static str>,
}

const RUST_T0: &str = "rust-t0";
const T1_T0: &str = "t1-t0";
const T1_CHAIN: &str = "t1-chain";
const T1_IR: &str = "t1-ir";
const ENGINES: &str = "interp+native";
const RUST_EMIT: &str = "rust-emit";
const T1_EMIT: &str = "t1-emit";
const T1_LINK: &str = "t1-link";
const T1_WRAPPER: &str = "t1-wrapper";
const MACHINE: &str = "machine";

fn diag_causes() -> Vec<Cause> {
    read("spec/diagnostics.tsv")
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with("code\t"))
        .filter_map(|l| {
            let mut f = l.split('\t');
            let code = f.next()?.trim();
            let term = f.next()?.trim();
            (!code.is_empty()).then(|| Cause {
                family: "diag",
                key: code.to_string(),
                name: term.to_string(),
                front_ends: vec![RUST_T0, T1_T0],
            })
        })
        .collect()
}

/// A shrinkhala verdict group: every `<prefix>…भेद` but the group's own
/// current-verdict variable (`<prefix>विरामभेद`) and success (value ०).
fn verdict_causes(family: &'static str, prefixes: &[&str]) -> Vec<Cause> {
    let text = read("crates/sadhana-t1/src/shrinkhala.t1");
    declared_ints(&text, "न६४")
        .into_iter()
        .filter(|(name, n)| {
            *n > 0
                && name.ends_with("भेद")
                && prefixes
                    .iter()
                    .any(|p| name.starts_with(p) && *name != format!("{p}विरामभेद").as_str())
        })
        .map(|(name, n)| Cause {
            family,
            key: format!("{name}={n}"),
            name: name.to_string(),
            front_ends: vec![T1_CHAIN],
        })
        .collect()
}

/// `ir.t1`'s stub causes: every literal cause number a raise site passes, and
/// every number `STUB_CAUSES` names. A number in one and not the other is
/// still a cause; its name says which list lacks it.
fn stub_causes() -> Vec<Cause> {
    let ir = read("crates/sadhana-t1/src/ir.t1");
    let mut raised = BTreeSet::new();
    for l in code_lines(&ir) {
        for lead in ["अपूर्णध्रुवम् ", "अपूर्णवाक्यम् ", "अपूर्णध्रुवस्थाने आरभ्य "]
        {
            for (at, _) in l.match_indices(lead) {
                if let Some(n) = l[at + lead.len()..]
                    .split_whitespace()
                    .next()
                    .and_then(deva)
                {
                    raised.insert(n);
                }
            }
        }
    }
    let table = read("crates/yantra/tests/paradigm_encode.rs");
    let body = table
        .split("const STUB_CAUSES")
        .nth(1)
        .and_then(|s| s.split("];").next())
        .expect("paradigm_encode.rs declares STUB_CAUSES");
    let mut named = BTreeMap::new();
    for l in body.lines().map(str::trim).filter(|l| l.starts_with('(')) {
        let inner = l.trim_start_matches('(');
        if let Some((n, rest)) = inner.split_once(',')
            && let Ok(n) = n.trim().parse::<u64>()
        {
            let name = rest
                .trim()
                .trim_matches(|c| c == '"' || c == ')' || c == ',');
            named.insert(n, name.trim_matches('"').to_string());
        }
    }
    assert!(
        !raised.is_empty() && !named.is_empty(),
        "the stub enumeration found {} raise sites and {} names — it is blind",
        raised.len(),
        named.len()
    );
    let all: BTreeSet<u64> = raised.iter().chain(named.keys()).copied().collect();
    all.into_iter()
        .map(|n| Cause {
            family: "stub",
            key: format!("{n}"),
            name: named
                .get(&n)
                .cloned()
                .unwrap_or_else(|| "(raised, not in STUB_CAUSES)".into()),
            front_ends: vec![T1_IR],
        })
        .collect()
}

/// `ir.t1`'s finisher refusal values: `सार्वजनिक चरः …निषेधः ॱॱ न६४ भवति N ।`.
fn finisher_causes() -> Vec<Cause> {
    let ir = read("crates/sadhana-t1/src/ir.t1");
    declared_ints(&ir, "न६४")
        .into_iter()
        .filter(|(name, _)| name.ends_with("निषेधः"))
        .map(|(name, n)| Cause {
            family: "finisher",
            key: format!("{n:#x}"),
            name: name.to_string(),
            front_ends: vec![ENGINES],
        })
        .collect()
}

/// `yantrotsarjana.t1`'s rows, each paired with the `Refusal` variant its
/// margin names; and every `Refusal` variant `riscv64.rs` declares that no row
/// names (a Rust-only cause).
fn emit_causes() -> Vec<Cause> {
    let t1 = read("crates/sadhana-t1/src/yantrotsarjana.t1");
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for l in t1.lines().map(str::trim) {
        let Some(rest) = l.strip_prefix("सार्वजनिक चरः यन्त्र")
        else {
            continue;
        };
        let Some((name, value)) = rest.split_once(" ॱॱ न६४ भवति ") else {
            continue;
        };
        // `यन्त्रनिषेधभेद ० ।` is the selector ("which variant"), not a cause.
        let value = value.split_whitespace().next().and_then(deva).unwrap_or(0);
        if !name.ends_with("निषेधभेद") || value == 0 {
            continue;
        }
        let variant = l
            .split("Refusal::")
            .nth(1)
            .and_then(|s| s.split_whitespace().next())
            .unwrap_or("?")
            .to_string();
        seen.insert(variant.clone());
        out.push(Cause {
            family: "emit",
            key: variant,
            name: format!("यन्त्र{name}"),
            front_ends: vec![RUST_EMIT, T1_EMIT],
        });
    }
    for v in enum_variants("crates/sadhana/src/t1/riscv64.rs", "pub enum Refusal {") {
        if !seen.contains(&v) {
            out.push(Cause {
                family: "emit",
                key: v.clone(),
                name: format!("Refusal::{v} (no .t1 row)"),
                front_ends: vec![RUST_EMIT],
            });
        }
    }
    out
}

/// The variant names of a Rust enum: lines at exactly four spaces' indent
/// beginning with an upper-case letter, up to the enum's closing brace.
fn enum_variants(file: &str, head: &str) -> Vec<String> {
    let text = read(file);
    let body = text
        .split(head)
        .nth(1)
        .unwrap_or_else(|| panic!("{file} declares `{head}`"));
    let mut out = Vec::new();
    for l in body.lines() {
        if l.starts_with('}') {
            break;
        }
        let Some(rest) = l.strip_prefix("    ") else {
            continue;
        };
        if rest.starts_with(' ') {
            continue;
        }
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
            out.push(name);
        }
    }
    assert!(
        !out.is_empty(),
        "{file}: `{head}` has no variants — the parser is blind"
    );
    out
}

fn link_causes() -> Vec<Cause> {
    let text = read("crates/sadhana-t1/src/samyojana.t1");
    declared_ints(&text, "अ६४")
        .into_iter()
        .filter(|(name, _)| name.ends_with("कूटः"))
        .map(|(name, n)| Cause {
            family: "link",
            key: format!("{n}"),
            name: name.to_string(),
            front_ends: vec![T1_LINK],
        })
        .collect()
}

/// `assemble.t1`'s statuses: every `प्रत्यागमनम् NNNN …` return, a composite
/// `NNNN योगः <k>` keyed by its base and `+k`.
fn status_causes() -> Vec<Cause> {
    let text = read("spec/entry/assemble.t1");
    let mut keys = BTreeSet::new();
    for l in code_lines(&text) {
        let Some(rest) = l.strip_prefix("प्रत्यागमनम् ") else {
            continue;
        };
        let mut w = rest.split_whitespace();
        let Some(n) = w.next().and_then(deva) else {
            continue;
        };
        if n < 1000 {
            continue;
        }
        let key = match w.next() {
            Some("योगः") => format!("{n}+{}", w.next().unwrap_or("?")),
            _ => format!("{n}"),
        };
        keys.insert(key);
    }
    keys.into_iter()
        // ४००० + k is enumerated per k, as the `reader` family.
        .filter(|k| !k.starts_with("4000+"))
        .map(|k| Cause {
            family: "status",
            name: k.clone(),
            key: k,
            front_ends: vec![T1_WRAPPER],
        })
        .collect()
}

fn reader_causes() -> Vec<Cause> {
    let text = read("spec/entry/assemble.t1");
    let ks: BTreeSet<u64> = code_lines(&text)
        .filter_map(|l| l.strip_prefix("पठनविरामः भवति "))
        .filter_map(|r| r.split_whitespace().next().and_then(deva))
        .filter(|k| *k > 0)
        .collect();
    ks.into_iter()
        .map(|k| Cause {
            family: "reader",
            key: format!("{k}"),
            name: format!("पठनविरामः {k} (status {})", 4000 + k),
            front_ends: vec![T1_WRAPPER],
        })
        .collect()
}

const T1_GATE: &str = "t1-gate";

/// The R-15-1 gate's causes: `सार्वजनिक चरः परिधि…दोषः ॱॱ न६४ भवति N ।`, N > ०.
fn gate_causes() -> Vec<Cause> {
    let text = read("crates/sadhana-t1/src/sanskrit_text.t1");
    let v: Vec<Cause> = declared_ints(&text, "न६४")
        .into_iter()
        .filter(|(name, n)| *n > 0 && name.starts_with("परिधि") && name.ends_with("दोषः"))
        .map(|(name, n)| Cause {
            family: "gate",
            key: name.to_string(),
            name: format!("{name}={n}"),
            front_ends: vec![T1_GATE],
        })
        .collect();
    assert!(
        !v.is_empty(),
        "sanskrit_text.t1's gate causes were not found — the reader is blind"
    );
    v
}

fn halt_causes() -> Vec<Cause> {
    enum_variants("crates/yantra/src/lib.rs", "pub enum Halt {")
        .into_iter()
        .map(|v| Cause {
            family: "halt",
            name: format!("Halt::{v}"),
            key: v,
            front_ends: vec![MACHINE],
        })
        .collect()
}

/// A cause the code PROMISES and does not raise for some input — listed as
/// UNCOVERED with the reason, and the cited text checked to still be there
/// (when it is gone, the entry is stale and this file says so).
/// (key, reason, file, a code line that shows the gap)
const FAIL_OPEN: &[(&str, &str, &str, &str)] = &[
    (
        "reader-4:wrapping-bounds",
        "assemble.t1 checks a section against the object's extent with अधिकम् on न६४, which \
         compares SIGNED, and adds offset+size unchecked: sh_size >= 2^63 (or an offset+size \
         that wraps) passes, so an empty .text is accepted where cause ४ is promised",
        "spec/entry/assemble.t1",
        "चरः स्थानान्तान्तः ॱॱ न६४ भवति स्थानान्तरम् योगः परिमाणम् ।",
    ),
    (
        "reader:relocation-range",
        "a RELA entry's type and r_offset are copied into the record with no range check \
         (no cause exists for them); a relocation past its section reaches the linker",
        "spec/entry/assemble.t1",
        "लेखा ॱ भेदः भवति लभेद ।",
    ),
    (
        "reader-5:second-rela",
        "the header promises cause ५ for two of one section, but a second RELA for the same \
         target is not refused: its entries are appended to the first's",
        "spec/entry/assemble.t1",
        "पाठलेखाः अङ्कः पलि अन्तः भवति लेखा ।",
    ),
    (
        "status:tables-not-loaded",
        "`शृङ्खलाॱसङ्कलनारम्भः`'s answer is bound and never read (mode २ and per source), so \
         a table load that fails is no status at all",
        "spec/entry/assemble.t1",
        "चरः तालिकाः ॱॱ न६४ भवति शृङ्खलाॱसङ्कलनारम्भः ।",
    ),
];

fn fail_open_causes() -> Vec<Cause> {
    FAIL_OPEN
        .iter()
        .map(|(key, _, file, needle)| {
            assert!(
                read(file).contains(needle),
                "FAIL_OPEN `{key}` cites `{needle}` in {file}, which is gone — re-read the code: \
                 if the gap is closed, delete the entry and give the cause a witness"
            );
            Cause {
                family: "fail-open",
                key: (*key).to_string(),
                name: (*key).to_string(),
                front_ends: vec![T1_WRAPPER],
            }
        })
        .collect()
}

fn all_causes() -> Vec<Cause> {
    let mut v = Vec::new();
    v.extend(diag_causes());
    v.extend(verdict_causes("build", &["वस्तुरचना"]));
    v.extend(verdict_causes("compile", &["सङ्कलन", "निर्णय"]));
    v.extend(stub_causes());
    v.extend(finisher_causes());
    v.extend(emit_causes());
    v.extend(link_causes());
    v.extend(status_causes());
    v.extend(reader_causes());
    v.extend(halt_causes());
    v.extend(gate_causes());
    v.extend(fail_open_causes());
    v
}

// ═══════════════════════════ referenced witnesses ═══════════════════════════

/// (family, key, front end, test file, test fn, a token the fn's body must hold)
/// Several tokens joined by `&&` must ALL be asserted.
const REFERENCED: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "gate",
        "परिधिअक्षरदोषः",
        T1_GATE,
        "crates/sadhana-t1/tests/w304-pass-zero-scan.rs",
        "the_scan_says_which_of_its_two_causes_refused_the_source",
        "परिधिअक्षरदोषः",
    ),
    (
        "gate",
        "परिधिउक्तदोषः",
        T1_GATE,
        "crates/sadhana-t1/tests/w304-pass-zero-scan.rs",
        "the_scan_says_which_of_its_two_causes_refused_the_source",
        "परिधिउक्तदोषः",
    ),
    (
        "finisher",
        "0x355",
        ENGINES,
        "crates/yantra/tests/w355_fresh_run.rs",
        "index_zero_of_a_fresh_run_is_refused_by_both_engines",
        "e.reason&&is_native_refusal",
    ),
    (
        "finisher",
        "0x359",
        ENGINES,
        "crates/yantra/tests/w359_param_growth.rs",
        "a_store_past_the_end_of_a_guarded_parameter_is_refused_by_both_engines",
        "cause(&e)&&is_native_refusal",
    ),
    (
        "finisher",
        "0x35a",
        ENGINES,
        "crates/yantra/tests/v008_vector_lowering.rs",
        "v008_unequal_lengths_are_refused_by_both_engines",
        "e.contains&&is_length_refusal",
    ),
    (
        "finisher",
        "0x35c",
        ENGINES,
        "crates/yantra/tests/w381_stage3_integer_semantics.rs",
        "w381_s3_checked_add_refuses_max_plus_one",
        "code&&OVERFLOW",
    ),
    (
        "finisher",
        "0x35e",
        ENGINES,
        "crates/yantra/tests/w381_stage3_integer_semantics.rs",
        "w381_s3_division_by_zero_is_refused",
        "refuses_everywhere&&DIV_ZERO",
    ),
    (
        "finisher",
        "0x35d",
        ENGINES,
        "crates/yantra/tests/w381_refusal_finisher_form.rs",
        "w381_stage4_negative_write_stops_qemu_as_it_stops_yantra",
        "assert_refuses_alike&&0x35d",
    ),
    (
        "emit",
        "EntryTakesParameters",
        RUST_EMIT,
        "crates/sadhana-t1/tests/t1_exec_riscv.rs",
        "both_twins_refuse_an_entry_that_takes_parameters_by_the_same_name",
        "EntryTakesParameters",
    ),
    (
        "emit",
        "EntryTakesParameters",
        T1_EMIT,
        "crates/sadhana-t1/tests/t1_exec_riscv.rs",
        "both_twins_refuse_an_entry_that_takes_parameters_by_the_same_name",
        "refusal_kind",
    ),
    (
        "emit",
        "StoreWidthUnnamed",
        RUST_EMIT,
        "crates/sadhana/src/t1/riscv64.rs",
        "a_narrow_store_takes_its_width_s_mnemonic_and_an_unnamed_width_refuses",
        "Refusal::StoreWidthUnnamed",
    ),
    (
        "emit",
        "Unreachable",
        RUST_EMIT,
        "crates/sadhana/src/t1/riscv64.rs",
        "unreachable_is_refused_by_block",
        "Refusal::Unreachable",
    ),
    (
        "emit",
        "LabelCollision",
        RUST_EMIT,
        "crates/sadhana/src/t1/riscv64.rs",
        "a_label_collision_is_refused_by_pair",
        "Refusal::LabelCollision",
    ),
    (
        "emit",
        "ParamAfterCall",
        RUST_EMIT,
        "crates/sadhana/src/t1/riscv64.rs",
        "a_param_after_a_call_is_refused",
        "Refusal::ParamAfterCall",
    ),
    (
        "emit",
        "FrameTooLarge",
        RUST_EMIT,
        "crates/sadhana/src/t1/riscv64.rs",
        "a_frame_one_addi_cannot_address_is_refused",
        "Refusal::FrameTooLarge",
    ),
    (
        "emit",
        "JumpOutOfRange",
        RUST_EMIT,
        "crates/sadhana/src/t1/riscv64.rs",
        "a_jump_past_one_mib_is_refused_in_both_modes",
        "Refusal::JumpOutOfRange",
    ),
    (
        "emit",
        "BranchOutOfRange",
        RUST_EMIT,
        "crates/sadhana/src/t1/riscv64.rs",
        "a_conditional_past_four_kib_forward_is_refused_and_4092_stands",
        "Refusal::BranchOutOfRange",
    ),
    (
        "emit",
        "JumpOutOfRange",
        T1_EMIT,
        "crates/sadhana-t1/tests/t1_jump_range_bound.rs",
        "the_j_type_jump_is_refused_at_one_mib_in_both_modes",
        "यन्त्रदूरलङ्घननिषेधभेद",
    ),
    (
        "emit",
        "BranchOutOfRange",
        T1_EMIT,
        "crates/sadhana-t1/tests/t1_branch_range_bound.rs",
        "the_b_type_conditional_is_refused_past_four_kib_on_both_sides",
        "यन्त्रदूरशाखानिषेधभेद",
    ),
    (
        "halt",
        "Finisher",
        MACHINE,
        "crates/yantra/tests/interpreter.rs",
        "the_finisher_decodes_success_and_failure",
        "Halt::Finisher",
    ),
    (
        "halt",
        "SpinForever",
        MACHINE,
        "crates/yantra/tests/interpreter.rs",
        "jal_links_the_return_address_and_a_self_jump_is_reported",
        "Halt::SpinForever",
    ),
    (
        "halt",
        "Unimplemented",
        MACHINE,
        // V-009 (i-f) lifted the non-RNE rounding refusal, and with it the old witness
        // (fp_extension.rs `a_rounding_mode_this_machine_cannot_honour_halts`).
        "crates/yantra/tests/atomics.rs",
        "an_lr_with_a_second_source_is_not_executed",
        "Halt::Unimplemented",
    ),
    (
        "halt",
        "BadAccess",
        MACHINE,
        "crates/yantra/tests/atomics.rs",
        "a_misaligned_atomic_performs_no_access",
        "Halt::BadAccess",
    ),
    (
        "halt",
        "BeyondRam",
        MACHINE,
        "crates/yantra/tests/interpreter.rs",
        "a_store_stops_at_the_store_limit_while_a_load_still_reaches_the_input_above_it",
        "Halt::BeyondRam",
    ),
    (
        "halt",
        "Device",
        MACHINE,
        "crates/yantra/tests/virtio_mmio.rs",
        "only_32_bit_aligned_accesses_are_answered",
        "Halt::Device",
    ),
    (
        "halt",
        "Shutdown",
        MACHINE,
        "crates/yantra/tests/application_abi.rs",
        "the_same_number_from_supervisor_mode_does_stop_it",
        "Halt::Shutdown",
    ),
    (
        "halt",
        "Sbi",
        MACHINE,
        "crates/yantra/tests/system.rs",
        "an_unimplemented_sbi_call_names_itself_rather_than_succeeding",
        "Halt::Sbi",
    ),
    (
        "halt",
        "Breakpoint",
        MACHINE,
        "crates/yantra/tests/traps.rs",
        "a_breakpoint_with_no_handler_installed_still_stops",
        "Halt::Breakpoint",
    ),
    (
        "halt",
        "Csr",
        MACHINE,
        "crates/yantra/tests/system.rs",
        "a_bare_read_is_not_reported_as_a_write",
        "Halt::Csr",
    ),
    (
        "halt",
        "PageFault",
        MACHINE,
        "crates/yantra/tests/paging.rs",
        "an_unmapped_fetch_faults_with_the_cause_a_fetch_raises",
        "Halt::PageFault",
    ),
    (
        "halt",
        "Undelivered",
        MACHINE,
        "crates/yantra/tests/interrupts.rs",
        "an_interrupt_with_no_vector_stops_rather_than_running_the_rubble_at_zero",
        "Halt::Undelivered",
    ),
    (
        "halt",
        "StepLimit",
        MACHINE,
        "crates/yantra/tests/interpreter.rs",
        "a_runaway_program_stops_instead_of_wedging_the_tab",
        "Halt::StepLimit",
    ),
    (
        "halt",
        "Wait",
        MACHINE,
        "crates/yantra/tests/w370_halt_wait.rs",
        "the_supervisor_reports_a_wait_as_the_machine_stopping",
        "Halt::Wait",
    ),
];

/// The body of `#[test] fn name` in `text`, or why it is not a live test.
/// The body begins AFTER the signature's opening brace, so the test's own name
/// is never part of it, and runs to the next `#[test]` (a `mod tests` in a
/// source file indents its tests).
fn test_body_in(text: &str, file: &str, name: &str) -> Result<String, String> {
    let sig = format!("fn {name}(");
    let at = text
        .find(&sig)
        .ok_or_else(|| format!("{file} has no `fn {name}`"))?;
    let before = &text[..at];
    let attrs_start = before
        .rfind("\n\n")
        .or_else(|| before.rfind("}\n"))
        .unwrap_or(0);
    let attrs = &before[attrs_start..];
    if !attrs.contains("#[test]") {
        return Err(format!("{file}::{name} is not a #[test]"));
    }
    if attrs.contains("#[ignore") {
        return Err(format!("{file}::{name} is #[ignore]d — no run reaches it"));
    }
    let open = text[at..]
        .find('{')
        .ok_or_else(|| format!("{file}::{name} has no body"))?;
    let rest = &text[at + open + 1..];
    let end = rest.find("#[test]").unwrap_or(rest.len());
    Ok(rest[..end].to_string())
}

fn test_body(file: &str, name: &str) -> Result<String, String> {
    test_body_in(&read(file), file, name)
}

/// The body with every `//` comment removed (doc and line comments alike).
fn code_of(body: &str) -> String {
    body.lines()
        .map(|l| l.find("//").map_or(l, |i| &l[..i]))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether `body` ASSERTS `token`: the token stands in code, in a statement
/// that asserts (`assert`, `panic!`, `matches!`), or in a `let NAME = …` whose
/// NAME an asserting statement then uses.
fn asserts(body: &str, token: &str) -> bool {
    let code = code_of(body);
    let stmts: Vec<&str> = code.split(';').collect();
    let asserting =
        |s: &str| s.contains("assert") || s.contains("panic!") || s.contains("matches!");
    stmts.iter().filter(|s| s.contains(token)).any(|s| {
        if asserting(s) {
            return true;
        }
        let Some(rest) = s.trim().strip_prefix("let ") else {
            return false;
        };
        let bound: String = rest
            .trim_start_matches("mut ")
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        !bound.is_empty()
            && stmts
                .iter()
                .any(|t| asserting(t) && t.contains(bound.as_str()))
    })
}

/// F1's control: a test that NAMES a cause — in its fn name and in a comment —
/// and asserts nothing about it is not its witness; one that asserts it is.
#[test]
fn a_test_that_only_names_a_cause_is_not_its_witness() {
    let text = "#[test]\n\
                fn refused_by_both_engines_halt_wait() {\n    \
                    // Halt::Wait is what this would check, and both engines\n    \
                    let x = 1;\n    \
                    assert_eq!(x, 1);\n\
                }\n\n\
                #[test]\n\
                fn really_asserts() {\n    \
                    let h = run();\n    \
                    assert!(matches!(h, Some(Halt::Wait)));\n    \
                    let k = konstant(\"परिधिउक्तदोषः\");\n    \
                    assert_eq!(cause(), k);\n\
                }\n";
    let named = test_body_in(text, "fixture", "refused_by_both_engines_halt_wait")
        .expect("the fixture's first test");
    assert!(
        !asserts(&named, "Halt::Wait"),
        "a comment is not an assertion"
    );
    assert!(
        !asserts(&named, "both"),
        "the test's own name is not its body"
    );
    let real = test_body_in(text, "fixture", "really_asserts").expect("the second test");
    assert!(
        asserts(&real, "Halt::Wait"),
        "an asserted variant is a witness"
    );
    assert!(
        asserts(&real, "परिधिउक्तदोषः"),
        "a let-bound cause then asserted is a witness"
    );
}

// ═══════════════════════════ probes: the T0 front ends ═══════════════════════════

/// A conditional branch over ११०० instructions — past the B-type's ±4 KiB.
fn far_branch() -> String {
    format!(
        "समलङ्घनम् अर्थ०न शून्यःत् दूरःय् ।\n{}दूरःॱॱ\n",
        "योगः अर्थ०म् अर्थ१न अर्थ२न ।\n".repeat(1100)
    )
}

/// (code, Sassembly text, the T0 front ends on which it makes EXACTLY `[code]`).
///
/// TWO TEXTS ARE ONE-SIDED BECAUSE THE TWINS DISAGREE ON THEM (found by this
/// file, 2026-10-04; both are notes on W-366):
/// * `योगः ५ अर्थ१न अर्थ२न ।` — Rust says P04, the `.t1` reader says P10. A CODE
///   CHOICE: the `.t1` lexer makes no numeral kind, so `५` is a word without a
///   kāraka sigil.
/// * a branch past ±4 KiB — Rust refuses BY RANGE with E07; the `.t1` encoder
///   raises E23, "laid out at X but written at Y", an internal-consistency
///   failure. A DEFECT, not a code choice: the `.t1` encoder never range-checks
///   the branch, and the inconsistency is what stops it.
fn diag_probes() -> Vec<(&'static str, String, &'static [&'static str])> {
    vec![
        ("E01", "व्यत्ययः अर्थ०म् अर्थ१न ।\n".into(), &[RUST_T0, T1_T0]),
        ("E02", "योगः अर्थ०म् अर्थ१न कखगन ।\n".into(), &[RUST_T0, T1_T0]),
        (
            "E03",
            "योगःॱप३२ अर्थ०म् अर्थ१न अर्थ२न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        ("E04", "योगः अर्थ०म् ।\n".into(), &[RUST_T0, T1_T0]),
        (
            "E09",
            "प्लवयोगःॱअ३२ प्लव०म् प्लव१न प्लव२न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "E10",
            "प्लवरूपान्तरम्ॱअ३२ अर्थ०म् प्लव१न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "E12",
            "समलङ्घनम् अर्थ०न शून्यःत् लक्ष्यन ।\nलक्ष्यॱॱ\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "E14",
            "आहारः क्षणिक०म् स्तूपसूचकःए ८न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "E15",
            "योगः अर्थ०म् अर्थ१न ५न ६न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "E16",
            "निधानम् क्षणिक०न क्षणिक१न ८न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "E17",
            "योगः अर्थ०म् अर्थ१न ५०००न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "E20",
            "सचिह्नदक्षिणसरणम्ॱन६४ अर्थ०म् अर्थ१न अर्थ२न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "E21",
            "अचिह्नाहारःॱअ३२ अर्थ०म् स्तूपसूचकःत् ०न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        ("L02", "॥ वैश्विकम् अज्ञात ॥\n".into(), &[RUST_T0, T1_T0]),
        (
            "P01",
            "॥ कोष्ठकम् ॱदत्त ॥\nयोगः अर्थ०म् अर्थ१न अर्थ२न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        ("P02", "योगः अर्थ०म् अर्थ१न अर्थ२न\n".into(), &[RUST_T0, T1_T0]),
        (
            "P03",
            "अज्ञातः अर्थ०म् अर्थ१न अर्थ२न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "P05",
            "योगः अर्थ०म् अर्थ१म् अर्थ२न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        ("P06", "ॱॱ\n".into(), &[RUST_T0, T1_T0]),
        ("P07", "क खॱॱ\n".into(), &[RUST_T0, T1_T0]),
        (
            "P08",
            "योगःॱअ३२ॱअ६४ॱअ८ अर्थ०म् अर्थ१न अर्थ२न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "P09",
            "अज्ञातम् अर्थ०म् अर्थ१न अर्थ२न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        ("P10", "योगः अर्थ० अर्थ१न अर्थ२न ।\n".into(), &[RUST_T0, T1_T0]),
        ("P13", "॥ अज्ञातादेशः ॥\n".into(), &[RUST_T0, T1_T0]),
        ("P14", "॥ समम् क ५ ॥\n".into(), &[RUST_T0, T1_T0]),
        ("P15", "॥ कोष्ठकम् ॱअज्ञात ॥\n".into(), &[RUST_T0, T1_T0]),
        (
            "P16",
            "॥ कोष्ठकम् ॱदत्त ॥\n॥ चतुरष्टकाः ॥\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "P17",
            "॥ कोष्ठकम् ॱदत्त ॥\n॥ चतुरष्टकाः उक्तम् क इति ॥\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "P18",
            "॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टकाः उक्तम् क इति ख ॥\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "P19",
            "॥ कोष्ठकम् ॱदत्त ॥\n॥ चतुरष्टकाः १२क ॥\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "P20",
            "॥ कोष्ठकम् ॱरिक्त ॥\n॥ चतुरष्टकाः ११५ ॥\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "P22",
            "॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टकाः उक्तम् क ॥\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "P23",
            "योगःॱअ१२८ अर्थ०म् अर्थ१न अर्थ२न ।\n".into(),
            &[RUST_T0, T1_T0],
        ),
        ("P24", "वैश्विकम् मुख्यम् ॥\n".into(), &[RUST_T0, T1_T0]),
        ("P25", "॥\n".into(), &[RUST_T0, T1_T0]),
        ("P26", "॥ ॥\n".into(), &[RUST_T0, T1_T0]),
        (
            "P27",
            "कॱॱ\n॥ कोष्ठकम् ॱदत्त ॥\n॥ चतुरष्टकाः क ॥\n".into(),
            &[RUST_T0, T1_T0],
        ),
        ("P28", "॥ चतुरष्टकाः ११५ ॥\n".into(), &[RUST_T0, T1_T0]),
        (
            "P29",
            "॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टाष्टकाः ९९९९९९९९९९९९९९९९९९९९९ ॥\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "P30",
            "॥ कोष्ठकम् ॱदत्त ॥\n॥ स्थानम् ऋण५ ॥\n".into(),
            &[RUST_T0, T1_T0],
        ),
        (
            "P31",
            "॥ कोष्ठकम् ॱदत्त ॥\n॥ अष्टकाः ३००० ॥\n".into(),
            &[RUST_T0, T1_T0],
        ),
        ("P04", "योगः ५ अर्थ१न अर्थ२न ।\n".into(), &[RUST_T0]),
        ("E07", far_branch(), &[RUST_T0]),
        ("E23", far_branch(), &[T1_T0]),
    ]
}

/// The codes Rust's T0 assembler gives `src`: lex, parse, then encode — the
/// structural codes, as `crates/sadhana/tests/rejected.rs` reads them.
fn rust_t0_codes(src: &str) -> Vec<String> {
    let tokens = match sadhana::lex::lex(src) {
        Ok(t) => t,
        Err(_) => return vec!["(lex error, no code)".into()],
    };
    match sadhana::parse::parse(&tokens) {
        Err(e) => e.iter().map(|x| x.code.to_string()).collect(),
        Ok(program) => {
            match sadhana::encode::encode_object_for(
                &program,
                sadhana::encode::Target::Uncompressed,
            ) {
                Ok(_) => Vec::new(),
                Err(es) => es.iter().map(|x| x.code.to_string()).collect(),
            }
        }
    }
}

fn load_chain() -> Interpreter {
    Interpreter::load(CHAIN, &repo_root().join("spec")).expect("the chain loads")
}

fn int_global(it: &Interpreter, name: &str) -> u64 {
    match it.global(name) {
        Some(Value::Int(n)) => u64::try_from(*n).unwrap_or(u64::MAX),
        other => panic!("`{name}` is an int, not {other:?}"),
    }
}

fn octets_text(v: Option<&Value>) -> String {
    v.and_then(|v| {
        v.octets()
            .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
    })
    .unwrap_or_default()
}

/// Build an object from `src` on the `.t1` chain, the driver's own order, and
/// answer (the builder's verdict, the codes recorded: the splitter's record
/// and, when the verdict is "encoding refused", the encoder's last).
fn t1_build(it: &mut Interpreter, src: &str) -> (u64, Vec<String>) {
    it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], 8_000_000_000)
        .expect("सङ्कलनारम्भः runs");
    it.call(
        "शृङ्खलाॱपाठवस्तुरचना",
        vec![Value::Octets(Octets::new(src.as_bytes()))],
        40_000_000_000,
    )
    .unwrap_or_else(|e| panic!("पाठवस्तुरचना runs on {src:?}: {e:?}"));
    let exit = int_global(it, "वस्तुरचनाविरामभेद");
    let count = int_global(it, "वाक्यविभागदोषसूचकाङ्क");
    let mut codes: Vec<String> = match it.global("वाक्यविभागदोषकोश")
    {
        Some(Value::Arena(a)) => {
            let a = a.borrow();
            (1..=usize::try_from(count).unwrap_or(0))
                .filter_map(|i| a.get(i))
                .map(|d| match d {
                    Value::Record(r) => octets_text(r.borrow().get("कूट")),
                    other => format!("{other:?}"),
                })
                .collect()
        }
        other => panic!("`वाक्यविभागदोषकोश` is an arena, not {other:?}"),
    };
    if exit == int_global(it, "वस्तुरचनाष्टकभेद") {
        let code = match it.global("अन्तिमसङ्केतनदोषः") {
            Some(Value::Record(r)) => octets_text(r.borrow().get("सङ्केताङ्क")),
            _ => String::new(),
        };
        codes.push(code);
    }
    (exit, codes)
}

// ═══════════════════════════ probes: the object builder's verdicts ═══════════════════════════

/// (verdict name, Sassembly text)
const BUILD_PROBES: &[(&str, &str)] = &[
    ("वस्तुरचनावाक्यभेद", "॰ nothing but a comment\n"),
    ("वस्तुरचनाष्टकभेद", "व्यत्ययः अर्थ०म् अर्थ१न ।\n"),
    ("वस्तुरचनावाक्यदोषभेद", "॥ चतुरष्टकाः ११५ ॥\n"),
];

// ═══════════════════════════ probes: the assemble.t1 wrapper ═══════════════════════════

/// `assemble.t1`'s `मुख्यम्` interpreted, as `yantra-run` runs its image: the
/// program name and `args` NUL-separated, their count, and the input.
fn wrapper(args: &[&str], input: &[u8]) -> u64 {
    let src = read("spec/entry/assemble.t1");
    let mut sources: Vec<(&str, &str)> = CHAIN.to_vec();
    sources.push(("assemble.t1", &src));
    let mut it = Interpreter::load(&sources, &repo_root().join("spec"))
        .expect("the chain and the wrapper load");
    let mut argv = b"assemble".to_vec();
    for a in args {
        argv.push(0);
        argv.extend_from_slice(a.as_bytes());
    }
    for (g, v) in [
        ("निवेशपाठः", Value::Octets(Octets::new(input))),
        ("आदेशपङ्क्तिः", Value::Octets(Octets::new(&argv))),
        ("आदेशगणना", Value::Int(1 + args.len() as i128)),
    ] {
        assert!(
            it.set_global(g, v),
            "`{g}` is a global of the chain + wrapper"
        );
    }
    let v = it
        .call("पाठसेतुःॱमुख्यम्", vec![], 4_000_000_000_000)
        .unwrap_or_else(|e| panic!("मुख्यम् runs: {e:?}"));
    u64::try_from(v.as_int().expect("मुख्यम् answers a status")).expect("a status is not negative")
}

/// A relocatable object Rust writes for `src`, with relocations when `src`
/// calls out (namaste-main calls into lib-mudraka).
fn rust_object(src: &str) -> Vec<u8> {
    sadhana::assemble_object(
        src,
        None,
        sadhana::encode::Target::Uncompressed,
        false,
        sadhana::nidana::Language::Sanskrit,
    )
    .unwrap_or_else(|e| {
        panic!(
            "Rust writes the object: {:?}",
            e.iter().map(|d| &d.reason).collect::<Vec<_>>()
        )
    })
}

fn le(b: &[u8], at: usize, n: usize) -> u64 {
    (0..n).fold(0u64, |v, i| v | (u64::from(b[at + i]) << (8 * i)))
}

fn put(b: &mut [u8], at: usize, n: usize, v: u64) {
    for i in 0..n {
        b[at + i] = (v >> (8 * i)) as u8;
    }
}

/// The section headers of an ELF64 object: (index, header offset, sh_type, sh_flags).
fn sections(b: &[u8]) -> Vec<(usize, usize, u64, u64)> {
    let shoff = le(b, 0x28, 8) as usize;
    let shnum = le(b, 0x3C, 2) as usize;
    (0..shnum)
        .map(|i| {
            let h = shoff + i * 64;
            (i, h, le(b, h + 4, 4), le(b, h + 8, 8))
        })
        .collect()
}

fn find_section(b: &[u8], ty: u64, flags: Option<u64>) -> (usize, usize) {
    sections(b)
        .into_iter()
        .find(|(_, _, t, f)| *t == ty && flags.is_none_or(|x| x == *f))
        .map(|(i, h, _, _)| (i, h))
        .unwrap_or_else(|| panic!("the object has a section of type {ty} flags {flags:?}"))
}

/// (k, how the object is broken). Every one is a CONTROL-object mutation:
/// the unbroken object is shown to read (status ०) first.
fn reader_probes(obj: &[u8]) -> Vec<(u64, Vec<u8>)> {
    let mut v = Vec::new();
    // १ — not `\x7fELF`.
    let mut b = obj.to_vec();
    b[1] = b'X';
    v.push((1, b));
    // २ — ELF32.
    let mut b = obj.to_vec();
    b[4] = 1;
    v.push((2, b));
    // ३ — section entries not ६४ octets.
    let mut b = obj.to_vec();
    put(&mut b, 0x3A, 2, 40);
    v.push((3, b));
    // ४ — .text's octets begin past the object.
    let mut b = obj.to_vec();
    let (_, h) = find_section(&b, 1, Some(6));
    put(&mut b, h + 24, 8, obj.len() as u64 + 64);
    v.push((4, b));
    // ५ — a section this model does not carry (SHT_NOTE).
    let mut b = obj.to_vec();
    let (_, h) = find_section(&b, 1, Some(6));
    put(&mut b, h + 4, 4, 7);
    v.push((5, b));
    // ६ — no .symtab: its type turned STRTAB, a type the reader knows and skips.
    let mut b = obj.to_vec();
    let (_, h) = find_section(&b, 2, None);
    put(&mut b, h + 4, 4, 3);
    v.push((6, b));
    // ७ — .symtab's entries not २४ octets.
    let mut b = obj.to_vec();
    let (_, h) = find_section(&b, 2, None);
    put(&mut b, h + 56, 8, 16);
    v.push((7, b));
    // ८ — a symbol in a section this model does not carry (.symtab's own index).
    let mut b = obj.to_vec();
    let (si, h) = find_section(&b, 2, None);
    let so = le(&b, h + 24, 8) as usize;
    let sn = le(&b, h + 32, 8) as usize / 24;
    let at = (1..sn)
        .map(|j| so + j * 24)
        .find(|e| le(&b, e + 6, 2) != 0)
        .expect("a symbol placed in a section");
    put(&mut b, at + 6, 2, si as u64);
    v.push((8, b));
    // ९ — a relocation names a symbol past the table.
    let mut b = obj.to_vec();
    let (_, h) = find_section(&b, 4, None);
    let ro = le(&b, h + 24, 8) as usize;
    let info = le(&b, ro + 8, 8);
    put(&mut b, ro + 8, 8, (info & 0xFFFF_FFFF) | (0xFFFF << 32));
    v.push((9, b));
    v
}

/// Why a row is uncovered, where the reason is known. INFORMATIONAL: a note
/// never covers a row. (family, key prefix, front end or "" for any, reason)
const NOTES: &[(&str, &str, &str, &str)] = &[
    (
        "diag",
        "P04",
        T1_T0,
        "TWINS DISAGREE (a code choice, W-366 note): on Rust's P04 text the .t1 reader says \
         P10 — the .t1 lexer makes no numeral kind",
    ),
    (
        "diag",
        "E07",
        T1_T0,
        "TWINS DISAGREE, A DEFECT (W-366 note): a branch past 4 KiB is refused by range (E07) \
         in Rust; the .t1 encoder raises E23, an internal-consistency failure, instead",
    ),
    (
        "diag",
        "E23",
        RUST_T0,
        "TWINS DISAGREE, A DEFECT (W-366 note): the .t1 encoder's E23 text is a range \
         refusal (E07) in Rust",
    ),
    (
        "diag",
        "P21",
        "",
        "its own message: 'live but unhandled — a bug'; no input should reach it",
    ),
    (
        "build",
        "वस्तुरचनानारब्धभेद",
        "",
        "a defect guard: 'no exit recorded'",
    ),
    (
        "compile",
        "सङ्कलनानारब्धभेद",
        "",
        "a defect guard: 'no exit was recorded'",
    ),
    (
        "compile",
        "निर्णयानारब्धभेद",
        "",
        "a defect guard: 'never entered, or an exit it did not record'",
    ),
    (
        "compile",
        "सङ्कलननामवैषम्यभेद",
        "",
        "a defect guard: names attempted differ from entries written",
    ),
    (
        "compile",
        "सङ्कलनानामभेद",
        "",
        "a defect guard: no name was recorded",
    ),
    (
        "compile",
        "सङ्कलनावृत्तिभेद",
        "",
        "not an error: data only, no routines",
    ),
    (
        "stub",
        "",
        "",
        "stubs are counted over the corpus (paradigm_encode.rs); no probe raises one alone",
    ),
];

// ═══════════════════════════ the test ═══════════════════════════

/// One row of the table this test prints.
struct Row {
    cause: Cause,
    front_end: &'static str,
    witness: Option<String>,
}

#[test]
fn every_refusal_cause_has_a_witness_or_is_counted() {
    let causes = all_causes();
    let mut witnessed: BTreeMap<(String, String, String), String> = BTreeMap::new();
    let mut wrong: Vec<String> = Vec::new();
    let mut w = |family: &str, key: &str, fe: &str, how: String| {
        witnessed.insert((family.into(), key.into(), fe.into()), how);
    };

    // ── the T0 diagnostics, both front ends ──
    let mut it = load_chain();
    for (code, src, fes) in diag_probes() {
        for fe in fes {
            let got = match *fe {
                RUST_T0 => rust_t0_codes(&src),
                T1_T0 => t1_build(&mut it, &src).1,
                other => panic!("no T0 front end {other}"),
            };
            if got == vec![(*code).to_string()] {
                let shown: String = src.chars().take(48).collect();
                w("diag", code, fe, format!("probe {shown:?}"));
            } else {
                let shown: String = src.chars().take(48).collect();
                wrong.push(format!(
                    "diag {code} on {fe}: {shown:?} made {got:?}, not exactly [{code}]"
                ));
            }
        }
    }

    // ── the object builder's verdicts ──
    for (verdict, src) in BUILD_PROBES {
        let want = int_global(&it, verdict);
        let (exit, _) = t1_build(&mut it, src);
        if exit == want {
            w(
                "build",
                &format!("{verdict}={want}"),
                T1_CHAIN,
                format!("probe {src:?}"),
            );
        } else {
            wrong.push(format!(
                "build {verdict}={want}: {src:?} gave verdict {exit}"
            ));
        }
    }

    // ── the wrapper's statuses, and its object reader ──
    let ok = "योगः स्थिर०म् शून्यःन ७न ।\n";
    let control = wrapper(&[&LOAD.to_string()], ok.as_bytes());
    assert_eq!(
        control, 0,
        "CONTROL: the wrapper assembles a plain source (status ०)"
    );
    for (key, args, input) in [
        ("3000", vec![], Vec::new()),
        ("3001", vec!["१२x"], ok.as_bytes().to_vec()),
        ("3002", vec!["2147483648", "7"], ok.as_bytes().to_vec()),
        (
            "3003",
            vec!["2147483648", "1"],
            [ok.as_bytes(), b"\0", ok.as_bytes()].concat(),
        ),
    ] {
        let want: u64 = key.parse().expect("a numeric status key");
        let got = wrapper(&args, &input);
        if got == want {
            w("status", key, T1_WRAPPER, format!("probe args {args:?}"));
        } else {
            wrong.push(format!("status {key}: args {args:?} gave {got}"));
        }
    }
    // १००० + k: the builder's verdict k=५ (a splitter refusal) through मुख्यम्.
    let k5 = int_global(&it, "वस्तुरचनावाक्यदोषभेद");
    let got = wrapper(&[], "॥ चतुरष्टकाः ११५ ॥\n".as_bytes());
    if got == 1000 + k5 {
        w(
            "status",
            "1000+शृङ्खलाॱवस्तुरचनाविरामभेद",
            T1_WRAPPER,
            "probe P28 text".into(),
        );
    } else {
        wrong.push(format!("status 1000+k: gave {got}, want {}", 1000 + k5));
    }
    // २००० + n: namaste-main alone — its callee is in lib-mudraka.
    let main = read("spec/namaste-main.sas");
    let got = wrapper(&[], main.as_bytes());
    if (2001..3000).contains(&got) {
        w(
            "status",
            "2000+संयोजनॱसंयोजनदोषसूचकाङ्क",
            T1_WRAPPER,
            "probe namaste-main alone".into(),
        );
    } else {
        wrong.push(format!("status 2000+n: namaste-main alone gave {got}"));
    }
    // ४००० + k: Rust's object of namaste-main (it has relocations), broken k ways.
    let obj = rust_object(&main);
    let lib = rust_object(&read("spec/lib-mudraka.sas"));
    let good = wrapper(
        &["2147483648", "2"],
        &[obj.as_slice(), lib.as_slice()].concat(),
    );
    assert_eq!(
        good, 0,
        "CONTROL: the two unbroken objects link in mode २ (status ०)"
    );
    for (k, broken) in reader_probes(&obj) {
        let got = wrapper(
            &["2147483648", "2"],
            &[broken.as_slice(), lib.as_slice()].concat(),
        );
        if got == 4000 + k {
            w(
                "reader",
                &k.to_string(),
                T1_WRAPPER,
                "probe: namaste-main's object, broken".into(),
            );
        } else {
            wrong.push(format!(
                "reader {k}: the broken object gave {got}, want {}",
                4000 + k
            ));
        }
    }

    // ── referenced witnesses ──
    for (family, key, fe, file, name, token) in REFERENCED {
        match test_body(file, name) {
            Ok(body) if token.split("&&").all(|t| asserts(&body, t)) => {
                w(family, key, fe, format!("{file}::{name}"))
            }
            Ok(_) => wrong.push(format!("{file}::{name} does not ASSERT `{token}`")),
            Err(why) => wrong.push(why),
        }
    }

    // ── the table, the uncovered, the ratchet ──
    let mut rows = Vec::new();
    for c in &causes {
        for fe in &c.front_ends {
            let witness = witnessed
                .get(&(c.family.to_string(), c.key.clone(), (*fe).to_string()))
                .cloned();
            rows.push(Row {
                cause: c.clone(),
                front_end: fe,
                witness,
            });
        }
    }
    let known: BTreeSet<(String, String, String)> = rows
        .iter()
        .map(|r| {
            (
                r.cause.family.to_string(),
                r.cause.key.clone(),
                r.front_end.to_string(),
            )
        })
        .collect();
    for k in witnessed.keys() {
        if !known.contains(k) {
            wrong.push(format!(
                "a witness for {k:?}, which no table holds — a stale witness"
            ));
        }
    }

    let mut by_family: BTreeMap<(&str, &str), (usize, usize)> = BTreeMap::new();
    for r in &rows {
        let e = by_family.entry((r.cause.family, r.front_end)).or_default();
        e.0 += 1;
        if r.witness.is_some() {
            e.1 += 1;
        }
    }
    println!(
        "REFUSAL-CAUSE WITNESSES: {} causes, {} (cause, front end) pairs",
        causes.len(),
        rows.len()
    );
    for ((family, fe), (n, cov)) in &by_family {
        println!("  {family:<10} {fe:<14} {cov:>3} of {n:>3} witnessed");
    }
    let uncovered: Vec<&Row> = rows.iter().filter(|r| r.witness.is_none()).collect();
    for r in &uncovered {
        let why = match r.cause.family {
            "fail-open" => FAIL_OPEN
                .iter()
                .find(|f| f.0 == r.cause.key)
                .map_or("", |f| f.1)
                .to_string(),
            _ => NOTES
                .iter()
                .find(|(f, k, fe, _)| {
                    *f == r.cause.family
                        && r.cause.key.starts_with(k)
                        && (fe.is_empty() || *fe == r.front_end)
                })
                .map_or("no witness yet", |n| n.3)
                .to_string(),
        };
        println!(
            "UNCOVERED {} {} [{}] on {}: {why}",
            r.cause.family, r.cause.key, r.cause.name, r.front_end
        );
    }
    println!("METRIC refusal_cause_witnesses_causes {}", causes.len());
    println!("METRIC refusal_cause_witnesses_pairs {}", rows.len());
    println!(
        "METRIC refusal_cause_witnesses_uncovered {}",
        uncovered.len()
    );

    assert!(
        wrong.is_empty(),
        "{} witness(es) do not raise exactly their cause:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
    assert!(
        uncovered.len() <= UNCOVERED_PIN,
        "{} uncovered (cause, front end) pairs, pinned at {UNCOVERED_PIN}: a cause was added \
         without a witness, or a witness was lost — see the UNCOVERED lines above",
        uncovered.len()
    );
    if uncovered.len() < UNCOVERED_PIN {
        println!(
            "LOWER THE PIN: {} uncovered, UNCOVERED_PIN is {UNCOVERED_PIN}",
            uncovered.len()
        );
    }
}
