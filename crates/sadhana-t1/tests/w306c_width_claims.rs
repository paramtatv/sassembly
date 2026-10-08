//! `W-306c` — A KIND WHOSE DEFINITION CLAIMS A WIDTH, AND EVERY DECODER THAT
//! HAS TO READ IT.
//!
//! Two instruction kinds now carry a WIDTH IN OCTETS as a third field beside
//! their two value operands — `LoadIndex` २० (`W-294`) and `StoreAt` २३
//! (`W-306c`) — and `ir.t1`'s margin is where each one SAYS SO:
//!
//! ```text
//! सार्वजनिक चरः स्थाननिधानाज्ञाभेद ॱॱ न६४ भवति २३ । ॰ Instruction::StoreAt(ValueId, ValueId, u64) — `W-306c`: the third is the element width in OCTETS, carried in ध्रुवमूल्यम्
//! ```
//!
//! Four hand-written decode tables then transcribe that number, and each has to
//! do TWO things the existing guard cannot see.
//!
//! # What `every_copy_of_a_kind_table_decodes_what_ir_t1_defines` does not check
//!
//! That guard — `t1_transcriptions.rs`, the one `ir.rs:242` says "refuses a
//! partial landing BY NAME" — compares NUMBER TO VARIANT and nothing else. It
//! asserts that `23` decodes to `StoreAt` in all four copies. It is blind to the
//! ARITY of the arm and to WHICH field each argument reads, because it stops
//! scanning at the variant name.
//!
//! So the landing it refuses is a copy that has never heard of kind २३. The
//! landing it WAVES THROUGH is a copy that decodes २३ to `StoreAt` while
//! dropping the width — and dropping the width is the whole defect, because a
//! store whose width arrives as `०` is a store the emitter has to guess at.
//! Rust's own arity check is the only thing standing there today, and it covers
//! exactly one of the two failures: a missing ARGUMENT will not compile, and a
//! missing `० -> ८` MAPPING compiles perfectly and answers zero.
//!
//! # The `० -> ८` mapping is the half with no compiler behind it
//!
//! `ir.t1` does not write `ध्रुवमूल्यम्` for either kind yet, so `int_of` finds
//! no field and every instruction in the corpus arrives at every decoder as `०`.
//! `०` IS NOT A WIDTH. Each decoder maps it to `८`, a whole word, which is why
//! the corpus stays octet-identical. Delete that one arm from ONE of the four
//! and:
//!
//!   * it compiles — the arity is unchanged;
//!   * `every_copy_of_a_kind_table_decodes_what_ir_t1_defines` stays green — the
//!     number still decodes to the same variant;
//!   * the decoder answers width `०` where its three siblings answer `८`;
//!   * and the divergence is INVISIBLE TO A TWIN COMPARISON, because the two
//!     emitters compared by `measure_corpus_twin_emit` read the chain's copy,
//!     not `pathana.rs`'s or `t1_exec_riscv.rs`'s.
//!
//! `paradigm_encode.rs`'s own kind-२० margin states the rule this file enforces:
//! *"two decoders that default differently are worse than two that default
//! wrongly together, because only the first kind of disagreement is invisible to
//! a twin comparison."* That sentence was a note to the next reader. Here it is
//! an assertion.
//!
//! # How the set is derived, and why not from a list
//!
//! **NO KIND NUMBER AND NO VARIANT NAME IS SPELLED IN THIS FILE.** The
//! width-bearing kinds are read out of `ir.t1`'s margins — a margin that names
//! `ध्रुवमूल्यम्` and says `width` claims a width — and the copies are
//! DISCOVERED by walking `crates/`, the same three moves
//! `t1_transcriptions.rs`'s header sets out. A third width-bearing kind is
//! checked the day its margin says so, and a fifth decode table the day it is
//! written.
//!
//! The cost of deriving rather than pinning is stated plainly: if a margin stops
//! claiming a width, that kind silently leaves this file's scope. That is what
//! `the_width_claim_scan_is_not_vacuous` is for — it refuses a run that found
//! fewer than two claims or fewer than four copies of one, so an emptied scan
//! reds instead of passing having compared nothing.
//!
//! # The refused case is run, not described
//!
//! `complaints()` is a pure function of the arms it is given, so
//! `a_decoder_that_drops_the_width_is_named` feeds it SYNTHETIC decoder text
//! with the mapping removed and asserts it is named — which is the only way to
//! know the positive test is not green because the scanner found nothing to
//! look at.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────────────────────────────────
// Loaders. This file reads `ir.t1` and `crates/`; it shares nothing with
// `t1_transcriptions.rs`, which is a separate integration binary.
// ─────────────────────────────────────────────────────────────────────────

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/sadhana-t1 has a grandparent")
        .to_path_buf()
}

fn ir_text() -> String {
    let p = repo_root().join("crates/sadhana-t1/src/ir.t1");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// Every `.rs` under `crates/`, `target` excluded. Paths come back
/// repo-relative so a failure names something a reader can open.
fn rust_sources() -> Vec<(String, String)> {
    let root = repo_root();
    let mut out = Vec::new();
    let mut stack = vec![root.join("crates")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                let Ok(text) = std::fs::read_to_string(&p) else {
                    continue;
                };
                let rel = p
                    .strip_prefix(&root)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .into_owned();
                out.push((rel, text));
            }
        }
    }
    out.sort();
    out
}

/// A Devanagari numeral as a number: `२३` is 23. The definition writes `२३`,
/// never `23`, and a Devanagari `\b` matches nothing — so this decodes by
/// codepoint and never by a word-boundary pattern.
fn devanagari_number(s: &str) -> Option<u32> {
    let mut n: u32 = 0;
    let mut any = false;
    for c in s.chars() {
        let d = u32::from(c).checked_sub(0x0966)?;
        if d > 9 {
            return None;
        }
        n = n.checked_mul(10)?.checked_add(d)?;
        any = true;
    }
    any.then_some(n)
}

/// THE FIELD the width travels in. Named once, because it is the one string
/// both halves — the definition's claim and the decoder's `int_of` — have to
/// spell identically for the claim to mean anything.
const WIDTH_FIELD: &str = "ध्रुवमूल्यम्";

// ─────────────────────────────────────────────────────────────────────────
// THE DEFINITION: which kinds claim a width, read off `ir.t1`'s margins.
// ─────────────────────────────────────────────────────────────────────────

/// Kind number -> Rust variant, for every kind whose `ir.t1` margin CLAIMS a
/// width carried in [`WIDTH_FIELD`].
///
/// The claim is the margin saying both things: the field's name, and the word
/// `width`. A margin that merely has a trailing `u64` does not qualify —
/// `LoadField`'s is an OFFSET and `AllocRecord`'s is a SIZE, and neither takes
/// the `० -> ८` mapping, because zero is a perfectly good offset and a refused
/// size is caught elsewhere. Reading the claim rather than the signature is
/// what keeps those two out without naming them.
fn width_claims() -> BTreeMap<u32, String> {
    let mut out = BTreeMap::new();
    for line in ir_text().lines() {
        let Some(rest) = line.strip_prefix("सार्वजनिक चरः ") else {
            continue;
        };
        let Some((decl, margin)) = rest.split_once('॰') else {
            continue;
        };
        let Some((_, value)) = decl.split_once(" भवति ") else {
            continue;
        };
        let Some(number) = devanagari_number(value.trim().trim_end_matches('।').trim()) else {
            continue;
        };
        let margin = margin.trim();
        let Some(after) = margin.strip_prefix("Instruction::") else {
            continue;
        };
        let variant: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if variant.is_empty() {
            continue;
        }
        // THE CLAIM, both halves of it.
        if margin.contains(WIDTH_FIELD) && margin.to_lowercase().contains("width") {
            out.insert(number, variant);
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────
// THE COPIES: every `<number> => Instruction::<Variant>( .. )` arm.
// ─────────────────────────────────────────────────────────────────────────

/// One decode arm, with the whole of its argument list.
#[derive(Debug)]
struct Arm {
    path: String,
    kind: u32,
    variant: String,
    /// Everything between the variant's `(` and its matching `)`.
    args: String,
}

/// Walk back from a `Instruction::` occurrence over `=>` to the ASCII kind
/// number the arm matches on. `None` for every `Instruction::X(` that is a
/// CONSTRUCTION rather than a decode arm — `opt.rs` builds `StoreAt(a, v, 8)`
/// and must not be read as a table.
fn kind_before(text: &str, at: usize) -> Option<u32> {
    let head = &text[..at];
    let head = head.trim_end();
    let head = head.strip_suffix("=>")?.trim_end();
    let digits: String = head
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.chars().rev().collect::<String>().parse().ok()
}

/// Everything from `open` (the index of a `(`) to its matching `)`, exclusive.
/// `None` if the parens never balance, which is a truncated file and not an
/// arm.
fn balanced(text: &str, open: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return text.get(open + 1..i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Find every decode arm in one file for the kinds given. A pure function of
/// its text, so [`a_decoder_that_drops_the_width_is_named`] can hand it a
/// synthetic decoder.
fn arms_in(path: &str, text: &str, claims: &BTreeMap<u32, String>) -> Vec<Arm> {
    let mut out = Vec::new();
    for (kind, variant) in claims {
        let needle = format!("Instruction::{variant}(");
        let mut from = 0usize;
        while let Some(rel) = text[from..].find(&needle) {
            let at = from + rel;
            from = at + needle.len();
            if kind_before(text, at) != Some(*kind) {
                continue;
            }
            let open = at + needle.len() - 1;
            let Some(args) = balanced(text, open) else {
                continue;
            };
            out.push(Arm {
                path: path.to_string(),
                kind: *kind,
                variant: variant.clone(),
                args: args.to_string(),
            });
        }
    }
    out
}

fn arms(claims: &BTreeMap<u32, String>) -> Vec<Arm> {
    let mut out = Vec::new();
    for (path, text) in rust_sources() {
        out.extend(arms_in(&path, &text, claims));
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────
// THE GUARD, as a pure function so the refused case can be run.
// ─────────────────────────────────────────────────────────────────────────

/// What is wrong with these arms, one line each. Empty is agreement.
///
/// A failure names the file, the kind, the variant and WHICH of the two things
/// is missing, because the repairs differ: an arm that never reads the field
/// decodes a width of zero for every instruction, while an arm that reads it
/// without the mapping decodes zero only until `ir.t1` starts writing the
/// field — a defect that would arrive with an unrelated commit.
fn complaints(arms: &[Arm]) -> Vec<String> {
    let mut out = Vec::new();
    for a in arms {
        if !a.args.contains(WIDTH_FIELD) {
            out.push(format!(
                "{}: kind {} ({}) decodes without reading `{WIDTH_FIELD}` — \
                 its width is zero for every instruction, and `ir.t1`'s margin \
                 says the third field is a width in OCTETS",
                a.path, a.kind, a.variant
            ));
            continue;
        }
        // The mapping, matched on text with the whitespace taken out so a
        // reformat cannot red this.
        let flat: String = a.args.chars().filter(|c| !c.is_whitespace()).collect();
        if !flat.contains("0=>8") {
            out.push(format!(
                "{}: kind {} ({}) reads `{WIDTH_FIELD}` but has no `0 => 8` arm — \
                 `०` is not a width, it is an absent field, and a decoder that \
                 carries it answers a different width from its siblings while \
                 compiling and while every twin comparison stays green",
                a.path, a.kind, a.variant
            ));
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────
// The tests.
// ─────────────────────────────────────────────────────────────────────────

/// **THE GUARD.** Every decoder of a kind whose definition claims a width reads
/// the field it claims, and normalises the absent value the same way.
#[test]
fn every_decoder_of_a_width_bearing_kind_reads_the_width() {
    let claims = width_claims();
    let arms = arms(&claims);
    let bad = complaints(&arms);
    assert!(
        bad.is_empty(),
        "{} decoder(s) disagree with `ir.t1`'s width claim:\n  {}",
        bad.len(),
        bad.join("\n  ")
    );
    println!("METRIC t1_width_claims {}", claims.len());
    println!("METRIC t1_width_decode_arms {}", arms.len());
    for (k, v) in &claims {
        let n = arms.iter().filter(|a| a.kind == *k).count();
        println!("  claim: {k} {v} — {n} decoder(s)");
    }
}

/// THE SCAN FOUND BOTH SIDES.
///
/// Without this, a margin reworded or a `match` restyled empties the claim set
/// or the arm set and the guard above passes having compared nothing — the
/// exact failure `t1_transcriptions.rs`'s header names one level up.
///
/// FOUR IS THE FLOOR AND IT IS NOT A ROUND NUMBER: `ir.rs:242` inventories the
/// four decode tables by path, and `pathana.rs`'s own margin records that
/// `W-283`'s first pass found only three by grepping. A claim with three
/// decoders is a partial landing.
#[test]
#[ignore = "needs the full development repository's source census not in the public repository"]
fn the_width_claim_scan_is_not_vacuous() {
    let claims = width_claims();
    assert!(
        claims.len() >= 2,
        "ir.t1 claims {} width-bearing kind(s); two kinds carry a width today \
         ({WIDTH_FIELD} on the indexed load and on the store at an address), so \
         the margin reader is broken or a margin was reworded",
        claims.len()
    );
    let arms = arms(&claims);
    for (kind, variant) in &claims {
        let found: BTreeSet<&str> = arms
            .iter()
            .filter(|a| a.kind == *kind)
            .map(|a| a.path.as_str())
            .collect();
        assert!(
            found.len() >= 4,
            "kind {kind} ({variant}) was found in {} file(s) — {:?}. Four decode \
             tables transcribe this numbering; fewer means the arm scanner has \
             stopped seeing them, or a copy never learned this kind",
            found.len(),
            found
        );
    }
}

/// **THE CASE THAT MUST BE REFUSED, RUN AND NOT DESCRIBED.**
///
/// Both failures are fed to the same `complaints()` the guard uses, as text, so
/// a scanner that had stopped recognising arms would be caught here rather than
/// reported as agreement up there. Neither synthetic decoder would fail to
/// compile — the first changes no arity (the width is a literal), and the second
/// changes none either.
#[test]
fn a_decoder_that_drops_the_width_is_named() {
    let claims = width_claims();
    let (kind, variant) = claims
        .iter()
        .next()
        .map(|(k, v)| (*k, v.clone()))
        .expect("ir.t1 claims at least one width-bearing kind");

    // (a) THE FIELD IS NEVER READ. A plausible edit: someone adds the third
    //     argument to satisfy the compiler and puts a literal there.
    let never_reads = format!(
        r#"            {kind} => Instruction::{variant}(
                ValueId(need(&ins, "वाम", &at)?),
                ValueId(need(&ins, "दक्षिण", &at)?),
                8,
            ),"#
    );
    let bad = complaints(&arms_in("synthetic/never_reads.rs", &never_reads, &claims));
    assert_eq!(
        bad.len(),
        1,
        "a decoder that hard-codes the width must be named exactly once, got {bad:?}"
    );
    assert!(
        bad[0].contains(WIDTH_FIELD) && bad[0].contains("without reading"),
        "the complaint must say the field is unread: {}",
        bad[0]
    );

    // (b) THE FIELD IS READ AND `०` IS CARRIED. This is the one with no
    //     compiler behind it and no twin comparison over it.
    let carries_zero = format!(
        r#"            {kind} => Instruction::{variant}(
                ValueId(need(&ins, "वाम", &at)?),
                ValueId(need(&ins, "दक्षिण", &at)?),
                u64::try_from(int_of(&ins, "{WIDTH_FIELD}")).unwrap_or(0),
            ),"#
    );
    let bad = complaints(&arms_in(
        "synthetic/carries_zero.rs",
        &carries_zero,
        &claims,
    ));
    assert_eq!(
        bad.len(),
        1,
        "a decoder that carries `०` through must be named exactly once, got {bad:?}"
    );
    assert!(
        bad[0].contains("no `0 => 8` arm"),
        "the complaint must name the missing mapping: {}",
        bad[0]
    );

    // (c) AND THE CORRECT SHAPE MUST BE ACCEPTED, or (a) and (b) prove only
    //     that `complaints` complains.
    let right = format!(
        r#"            {kind} => Instruction::{variant}(
                ValueId(need(&ins, "वाम", &at)?),
                ValueId(need(&ins, "दक्षिण", &at)?),
                match u64::try_from(int_of(&ins, "{WIDTH_FIELD}")).unwrap_or(0)
                {{
                    0 => 8,
                    w => w,
                }},
            ),"#
    );
    let arms = arms_in("synthetic/right.rs", &right, &claims);
    assert_eq!(
        arms.len(),
        1,
        "the scanner must find the one arm in the correct shape, got {arms:?}"
    );
    assert!(
        complaints(&arms).is_empty(),
        "the correct shape must be accepted: {:?}",
        complaints(&arms)
    );
}

/// A CONSTRUCTION IS NOT A DECODE TABLE, and the arm scanner must tell them
/// apart or `opt.rs`'s `Instruction::StoreAt(addr, val, 8)` — a test fixture
/// building IR by hand — would be reported as a decoder that hard-codes its
/// width. That complaint would be TRUE of the text and WRONG about the file,
/// and a guard with a standing false positive gets switched off.
#[test]
fn a_hand_built_instruction_is_not_read_as_a_decoder() {
    let claims = width_claims();
    let (_, variant) = claims
        .iter()
        .next()
        .map(|(k, v)| (*k, v.clone()))
        .expect("ir.t1 claims at least one width-bearing kind");
    let built = format!("        let i = Instruction::{variant}(ValueId(0), ValueId(1), 8);");
    assert!(
        arms_in("synthetic/built.rs", &built, &claims).is_empty(),
        "a bare construction has no `N =>` before it and is not an arm"
    );
    // And the same text WITH an arm head is.
    let armed = format!("        7777 => Instruction::{variant}(a, b, 8),");
    let mut extra = claims.clone();
    extra.insert(7777, variant);
    assert_eq!(
        arms_in("synthetic/armed.rs", &armed, &extra).len(),
        1,
        "`N => Instruction::X(..)` is an arm; the discriminator is the arrow"
    );
}
