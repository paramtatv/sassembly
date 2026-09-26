//! ॥ AN INTEGER MIRROR OF A RUST ENUM NAMES VARIANTS THAT EXIST, ALL OF THEM,
//! IN ORDER ॥ `W-264`.
//!
//! `samyojana.t1` cannot use a `गणना` that lives in another module — its own
//! margin says why: *"the enum lives in another module and this file must be
//! readable before that module is hand-ported"* — so it MIRRORS two Rust enums
//! as integers, each constant carrying the variant it stands for in its margin:
//!
//! ```text
//! सार्वजनिक चरः पाठनिवेशः ॱॱ न६४ भवति १ ।   ॰ Placement::Text
//! सार्वजनिक चरः पाठकोष्ठकम् ॱॱ न६४ भवति १ ।  ॰ SymSection::Text
//! ```
//!
//! **Those margins are the only thing tying the two languages together, and
//! nothing has ever read them.** A Rust enum can gain a variant, lose one, or
//! reorder, and the mirror keeps whatever integers it had.
//!
//! # Why this is not an arity check, and what an arity check already cost
//!
//! `encode.t1:5757` compares `कोश ॱ SymSection` (six variants) with
//! `वास्तु ॱ स्थापन` (six variants), finds the correspondence "ASYMMETRIC", and
//! concludes at `:5778` that **"THE GAP IS IN `स्थापन`, one variant wide."**
//!
//! `स्थापन` IS NOT `SymSection`'s MIRROR. Its twin is `Placement` in
//! `crates/sadhana/src/vastu.rs` — same module, same file stem, six variants
//! matching variant for variant IN ORDER — and that pair agrees completely.
//! `SymSection`'s mirror is `samyojana.t1`'s four `*कोष्ठकम्` constants, which
//! are **two** variants short, not one, and the deficiency is in the mirror
//! rather than in `स्थापन`.
//!
//! **Two enums in different modules were matched on cardinality alone, and six
//! equalling six is what made it look like a correspondence.** A row acting on
//! that sentence would add a seventh variant to the one pair that currently
//! agrees. So this file matches BY NAME, through those margins, and never by
//! counting.
//!
//! # What it does not decide
//!
//! Whether `%pcrel_lo12`'s missing section symbol belongs on the write side
//! (`कोश`/`SymSection`) or as a new `स्थापन` variant is a design question about
//! the linker, and it is not answered here. This file records that
//! `DebugLineSection` and `TextSection` have no mirror, and names them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate sits two levels under the repo root")
        .to_path_buf()
}

/// `०..९` — the numerals the corpus writes.
fn devanagari_number(s: &str) -> Option<u32> {
    let mut n = 0u32;
    let mut any = false;
    for c in s.chars() {
        let d = ('०'..='९').position(|x| x == c)? as u32;
        n = n.checked_mul(10)?.checked_add(d)?;
        any = true;
    }
    any.then_some(n)
}

/// Every variant of a Rust `enum`, in DECLARATION ORDER, found by name across
/// `crates/sadhana/src`. Located rather than path-pinned, so moving the file
/// does not silently make this vacuous.
fn rust_enum(name: &str) -> Vec<String> {
    let dir = repo_root().join("crates/sadhana/src");
    let mut files: Vec<PathBuf> = Vec::new();
    fn walk(d: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(d) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    walk(&dir, &mut files);
    files.sort();
    let head = format!("enum {name} {{");
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        let Some(at) = text.find(&head) else { continue };
        let body = &text[at + head.len()..];
        let body = &body[..body.find('}').expect("the enum closes")];
        let mut out = Vec::new();
        for line in body.lines() {
            let line = line.trim();
            // Attributes (`#[default]`), doc comments and plain comments sit
            // between variants and are not variants.
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }
            let variant: String = line
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !variant.is_empty() {
                out.push(variant);
            }
        }
        assert!(
            !out.is_empty(),
            "`enum {name}` was found in {} and parsed to no variants",
            f.display()
        );
        return out;
    }
    panic!(
        "no `enum {name}` under crates/sadhana/src — this guard is looking for a name that moved"
    );
}

/// One mirrored constant: `(value, variant)` keyed by the T1 name that holds it.
struct Mirror {
    /// `Enum::Variant` → the `न६४` the corpus gives it.
    by_variant: BTreeMap<String, u32>,
    /// The T1 constant that carries each, for a message that can be acted on.
    t1_name: BTreeMap<String, String>,
}

/// Every `सार्वजनिक चरः … भवति <numeral> ।  ॰ <Enum>::<Variant>` in
/// `samyojana.t1`, grouped by the enum its margin names.
fn mirrors() -> BTreeMap<String, Mirror> {
    let path = repo_root().join("crates/sadhana-t1/src/samyojana.t1");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
    let mut out: BTreeMap<String, Mirror> = BTreeMap::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("सार्वजनिक चरः ") else {
            continue;
        };
        let Some((decl, margin)) = rest.split_once('॰') else {
            continue;
        };
        let Some((t1_name, value)) = decl.split_once(" ॱॱ न६४ भवति ") else {
            continue;
        };
        let Some(n) = devanagari_number(value.trim().trim_end_matches('।').trim()) else {
            continue;
        };
        let Some((enum_name, variant)) = margin.trim().split_once("::") else {
            continue;
        };
        let variant: String = variant
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if variant.is_empty() {
            continue;
        }
        let e = out
            .entry(enum_name.trim().to_string())
            .or_insert_with(|| Mirror {
                by_variant: BTreeMap::new(),
                t1_name: BTreeMap::new(),
            });
        e.by_variant.insert(variant.clone(), n);
        e.t1_name.insert(variant, t1_name.trim().to_string());
    }
    out
}

/// Variants a mirror deliberately does NOT carry, with the reason.
///
/// EXACT, not a ceiling — a third would slip past a `<=`, which is the whole
/// argument of this file.
const UNMIRRORED: &[(&str, &str, &str)] = &[
    (
        "SymSection",
        "DebugLineSection",
        "a symbol that IS the .debug_line section, not one placed in it",
    ),
    (
        "SymSection",
        "TextSection",
        "the .text SECTION SYMBOL `%pcrel_lo12` needs — B-105, refused rather than guessed",
    ),
];

#[test]
fn every_mirrored_enum_names_variants_that_exist_all_of_them_in_order() {
    let mirrors = mirrors();
    assert!(
        !mirrors.is_empty(),
        "no `Enum::Variant` margins found in samyojana.t1 — this guard has gone \
         vacuous and would pass over any drift"
    );

    let mut wrong: Vec<String> = Vec::new();
    for (enum_name, mirror) in &mirrors {
        let variants = rust_enum(enum_name);
        let named = |v: &str| {
            mirror
                .t1_name
                .get(v)
                .cloned()
                .unwrap_or_else(|| "—".to_string())
        };

        // 1. Every mirrored name is a variant the Rust enum actually declares.
        for v in mirror.by_variant.keys() {
            if !variants.contains(v) {
                wrong.push(format!(
                    "samyojana.t1 `{}` mirrors {enum_name}::{v}, and {enum_name} \
                     declares no such variant — it has {:?}",
                    named(v),
                    variants
                ));
            }
        }

        // 2. Every variant is mirrored, or is RECORDED as deliberately not.
        for v in &variants {
            if mirror.by_variant.contains_key(v) {
                continue;
            }
            match UNMIRRORED
                .iter()
                .find(|(e, var, _)| e == enum_name && var == v)
            {
                Some(_) => {}
                None => wrong.push(format!(
                    "{enum_name}::{v} has NO mirror in samyojana.t1 — add one, or \
                     record it in UNMIRRORED with the reason. A Rust enum that \
                     grows a variant leaves the mirror behind silently, which is \
                     what this guard is for"
                )),
            }
        }

        // 3. The mirror's ORDER follows the enum's declaration order. A set
        //    check passes a reordering; the integers are positional, so a
        //    reordering is a miscompile rather than a tidiness.
        let mirrored_in_decl_order: Vec<&String> = variants
            .iter()
            .filter(|v| mirror.by_variant.contains_key(*v))
            .collect();
        let mut previous = 0u32;
        for v in &mirrored_in_decl_order {
            let n = mirror.by_variant[*v];
            if n <= previous {
                wrong.push(format!(
                    "samyojana.t1 `{}` gives {enum_name}::{v} the value {n}, which \
                     does not follow the previous variant's {previous} — the mirror \
                     is positional and its order must be the enum's",
                    named(v)
                ));
            }
            previous = n;
        }
    }

    // 4. Nothing is recorded as unmirrored that is not actually a variant, and
    //    nothing recorded is quietly mirrored after all.
    for (enum_name, variant, _) in UNMIRRORED {
        let variants = rust_enum(enum_name);
        if !variants.contains(&(*variant).to_string()) {
            wrong.push(format!(
                "UNMIRRORED records {enum_name}::{variant}, which {enum_name} no \
                 longer declares — drop the row"
            ));
        } else if mirrors
            .get(*enum_name)
            .is_some_and(|m| m.by_variant.contains_key(*variant))
        {
            wrong.push(format!(
                "UNMIRRORED records {enum_name}::{variant} as deliberately absent, \
                 and samyojana.t1 mirrors it — drop the row"
            ));
        }
    }

    assert!(
        wrong.is_empty(),
        "an integer mirror and the enum it mirrors disagree:\n  {}",
        wrong.join("\n  ")
    );
}

/// THE PAIR THAT AGREES, ASSERTED SO THE GUARD ABOVE HAS A POSITIVE CONTROL
/// THAT IS NOT SYNTHETIC.
///
/// `Placement` is mirrored COMPLETELY — six variants, six constants, in order —
/// so if the machinery above ever stops finding margins, or stops resolving
/// Rust enums, this goes red rather than passing vacuously. It is also the
/// pair that `encode.t1:5778` calls one variant short: it is not.
#[test]
fn the_placement_mirror_is_complete_and_that_is_what_a_correspondence_looks_like() {
    let mirrors = mirrors();
    let m = mirrors
        .get("Placement")
        .expect("samyojana.t1 mirrors Placement");
    let variants = rust_enum("Placement");
    assert_eq!(
        variants.len(),
        6,
        "Placement's variants changed: {variants:?} — this control pins the \
         complete pair, so a change here needs the mirror changed with it"
    );
    let missing: Vec<&String> = variants
        .iter()
        .filter(|v| !m.by_variant.contains_key(*v))
        .collect();
    assert!(
        missing.is_empty(),
        "Placement is the pair that AGREES and {missing:?} is unmirrored — if \
         this is intended, the guard above is now the only one checking anything"
    );
    // `वास्तु ॱ स्थापन`'s variants are numbered from ० by the interpreter, and the
    // mirror is deliberately variant PLUS ONE so that an unwritten record —
    // `Int(0)` — is distinguishable from the FIRST variant rather than silently
    // reading as `.text`. `samyojana.t1:52` carries the argument.
    assert_eq!(
        variants
            .iter()
            .map(|v| m.by_variant[v])
            .collect::<Vec<u32>>(),
        vec![1, 2, 3, 4, 5, 6],
        "the Placement mirror is variant PLUS ONE, in declaration order"
    );
}
