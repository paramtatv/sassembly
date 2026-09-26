//! `W-271`: EVERY MINTER OF A NUMBERING WRITES EVERY ARENA THAT NUMBERING KEYS.
//!
//! THE ROW ASKED FOR A DIFFERENT RATCHET AND ITS ACCEPTANCE WAS WITHDRAWN, so
//! the reasoning is here rather than in a ledger nobody reads beside the code.
//! W-271 proposed pinning "no arena read bounded by its own `दैर्घ्य`" at zero.
//! Measured on the corpus before building: **twelve sites read an arena under a
//! `दैर्घ्य` guard, and every one of them is correct today.** A ratchet at zero
//! would demand rewriting all twelve — which is precisely the error the row was
//! filed to correct in ITS predecessor, where a `_not_cleared`-at-0 ratchet
//! would have demanded clearing fourteen correct sites and would have failed
//! the extent fix that closed `W-249`. A count pins a spelling. This pins the
//! thing that would actually break.
//!
//! WHAT THOSE TWELVE SITES DEPEND ON. Seven follow the read with a sentinel
//! check (`समम् ०`, `असमम् शून्यम्`). Four do not — `artha.t1`'s reads of
//! `संज्ञाघोषणाकोश`, `वास्तुॱवाक्यकोश` and `वास्तुॱअभिव्यञ्जककोश` — and they are
//! safe for one reason only: the arenas are DENSE. Being inside `दैर्घ्य` does
//! not mean WRITTEN, and a guard that refuses when out of range proves in-range
//! exactly as weakly as one that permits when in range; both are safe only if
//! every slot below the cursor was filled.
//!
//! THE HAZARD IS NOT HYPOTHETICAL AND THE CORPUS HAS ALREADY PAID FOR IT.
//! `अर्थ` had two minters and only `घोषणम्` filled all three symbol-keyed
//! arenas; `सञ्चयसंज्ञा` interned a cross-module member and wrote the type
//! alone. Every store member advanced the counter while filling one arena of
//! three, the interpreter padded the gap with `Nil`, and eleven of fifteen
//! corpus sources stopped at IR with "expected a number, found Nil" (`W-245`).
//! The writer was repaired (`artha.t1`, `सञ्चयसंज्ञा` now writes all three,
//! taking the kind from `घोषणासञ्चयॱप्रविष्टिभेदः` and leaving the declaration
//! honestly `०`). **Nothing stops a third minter from reintroducing it**, and
//! that is what this test is for: the invariant is kept by convention, and a
//! convention no pass reads is an honour system.
//!
//! NOT `#[ignore]`d. It reads one source and runs in milliseconds — it does not
//! parse the corpus, so it costs nothing in an ordinary `cargo test`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn crate_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The routine that mints a `SymbolId` and advances the counter. Every caller
/// of it is a MINTER, and every minter owes every symbol-keyed arena a write.
const MINT: &str = "संज्ञाग्रहणम्";

/// The arenas a routine WRITES **at an index derived from `key`**, by name.
///
/// A write is `NAME अङ्कः <index> अन्तः भवति <value>` — the arena on the LEFT of
/// `भवति`. A READ is `… भवति NAME अङ्कः <index> अन्तः`, the same three words in
/// the other order, and counting one as the other is the whole difficulty of
/// reading this by text. The order of `भवति` against `अङ्कः` is what separates
/// them, and nothing else does.
///
/// THE INDEX IS PART OF THE QUESTION AND THE FIRST VERSION OF THIS OMITTED IT.
/// A minter writes arenas under SEVERAL numberings: `सञ्चयसंज्ञा` fills
/// `सञ्चयसंज्ञाप्रविष्टयः` and `सञ्चयसंज्ञामूल्यानि` at its OWN cursor, and
/// `घोषणम्` fills `प्रविष्टयः` at another. Those are different numberings with
/// different invariants, and comparing them made the two minters look unequal
/// when the three symbol-keyed arenas they share are exactly equal. Only writes
/// indexed by the SYMBOL this routine just minted belong to this invariant.
fn arenas_written(body: &str, key: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in body.lines() {
        let code = line.split('॰').next().unwrap_or("");
        let Some(at) = code.find(" अङ्कः ") else {
            continue;
        };
        let Some(end) = code.find(" अन्तः भवति ") else {
            continue;
        };
        if end < at {
            continue;
        }
        // The index expression, between `अङ्कः` and `अन्तः`. It must name the
        // symbol this routine minted, or the write belongs to another numbering.
        let index = &code[at + " अङ्कः ".len()..end];
        if !index.split_whitespace().any(|w| w == key) {
            continue;
        }
        // A `भवति` BEFORE the arena name makes this a read, not a write.
        let head = &code[..at];
        if head.contains(" भवति ") || head.trim_start().starts_with("भवति ") {
            continue;
        }
        if let Some(name) = head
            .split_whitespace()
            .next_back()
            .filter(|n| !n.is_empty())
        {
            out.insert(name.to_string());
        }
    }
    out
}

/// Every routine that calls [`MINT`], with the arenas it writes.
///
/// A routine begins at `वृत्तिः`/`सार्वजनिक वृत्तिः` and ends at the next `इति`
/// in the first column, which is how every `.t1` source in this corpus is laid
/// out. The mint's OWN declaration is not a caller of itself and is skipped.
fn minters(src: &str) -> Vec<(String, BTreeSet<String>)> {
    let lines: Vec<&str> = src.lines().collect();
    let mut starts: Vec<(usize, String)> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        if (t.starts_with("वृत्तिः ") || t.starts_with("सार्वजनिक वृत्तिः "))
            && l.starts_with(|c: char| !c.is_whitespace())
        {
            let name = t
                .trim_start_matches("सार्वजनिक ")
                .trim_start_matches("वृत्तिः ")
                .split_whitespace()
                .next()
                .unwrap_or("?")
                .to_string();
            starts.push((i, name));
        }
    }
    let mut out = Vec::new();
    for (k, (start, name)) in starts.iter().enumerate() {
        let end = starts.get(k + 1).map_or(lines.len(), |(n, _)| *n);
        if name == MINT {
            continue; // the mint itself
        }
        let body = lines[*start..end].join("\n");
        // The variable this routine binds the minted SymbolId to — `चरः संज्ञा
        // ॱॱ न६४ भवति संज्ञाग्रहणम् …`. It is the key of the numbering, and the
        // arenas indexed by it are the ones this invariant governs. A call in a
        // margin binds nothing, so the margin is stripped before looking.
        let key = body.lines().find_map(|l| {
            let code = l.split('॰').next().unwrap_or("");
            let at = code.find(&format!("भवति {MINT}"))?;
            let decl = code[..at].trim();
            let var = decl.strip_prefix("चरः ")?.split_whitespace().next()?;
            Some(var.to_string())
        });
        if let Some(key) = key {
            out.push((name.clone(), arenas_written(&body, &key)));
        }
    }
    out
}

/// **The guard.** Every minter of the symbol numbering writes the same set of
/// symbol-keyed arenas. A new minter that fills one arena of three reintroduces
/// `W-245`'s `Nil` holes, and fails HERE, by name, instead of surfacing as a
/// build stopping with "expected a number, found Nil" some distance away.
///
/// It asserts the sets are EQUAL and never that there are three of them: a lane
/// may legitimately add an arena or a writer (`W-265` is in this file now), and
/// a pinned number would go red on correct work. The property is what holds.
#[test]
fn every_symbol_minter_writes_the_same_arenas() {
    let src = std::fs::read_to_string(crate_src().join("artha.t1")).expect("artha.t1 is readable");
    let found = minters(&src);

    assert!(
        found.len() >= 2,
        "fewer than two minters call `{MINT}` — either the corpus changed shape or this \
         instrument stopped finding them, and a test that finds nothing asserts nothing. \
         Found: {:?}",
        found.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );

    // TWO EMPTY SETS ARE EQUAL, so equality alone can pass vacuously: if the
    // extractor stopped recognising a write — a respelled `अङ्कः`, a renamed
    // mint — every minter would report {} and this ratchet would go green
    // having checked nothing. That is the failure it exists to prevent,
    // committed by the instrument instead of the code.
    for (name, set) in &found {
        assert!(
            !set.is_empty(),
            "`{name}` calls `{MINT}` and this instrument sees it write NO arena keyed by \
             the symbol it minted. Either the minter writes none — which is the defect — \
             or the extractor no longer recognises a write, which makes every comparison \
             below vacuous. Both need a person."
        );
    }

    let (first_name, first_set) = &found[0];
    for (name, set) in &found[1..] {
        assert_eq!(
            set,
            first_set,
            "`{name}` and `{first_name}` both mint a SymbolId and write DIFFERENT arenas. \
             Every slot the counter advances past must be written by every minter, or the \
             unwritten arenas grow holes the interpreter pads with `Nil` — `W-245`. \
             `{name}` writes {set:?}; `{first_name}` writes {first_set:?}. \
             Missing from `{name}`: {:?}",
            first_set.difference(set).collect::<Vec<_>>()
        );
    }
}

/// REFUSED: a second minter that fills one arena of three is caught. This is
/// `W-245`'s defect in miniature, and the ratchet above is only worth having if
/// it fails on it — a guard that cannot fail is a comment.
#[test]
fn a_minter_that_skips_an_arena_is_refused() {
    let synthetic = "\
वृत्तिः संज्ञाग्रहणम् आदाय निर्णायकः ॱॱ निर्णायक ददाति न६४ आदि
    प्रत्यागमनम् ० ।
इति
सार्वजनिक वृत्तिः घोषणम् आदाय निर्णायकः ॱॱ निर्णायक ददाति न६४ आदि
    चरः संज्ञा ॱॱ अ६४ भवति संज्ञाग्रहणम् निर्णायकः ।
    संज्ञाप्रकारकोश अङ्कः आरभ्य संज्ञा योगः १ समाप्तम् अन्तः भवति प्रकार ।
    संज्ञाभेदकोश अङ्कः आरभ्य संज्ञा योगः १ समाप्तम् अन्तः भवति भेदः ।
इति
वृत्तिः सञ्चयसंज्ञा आदाय निर्णायकः ॱॱ निर्णायक ददाति न६४ आदि
    चरः संज्ञा ॱॱ न६४ भवति संज्ञाग्रहणम् निर्णायकः ।
    संज्ञाप्रकारकोश अङ्कः आरभ्य संज्ञा योगः १ समाप्तम् अन्तः भवति प्रकारसूचकः ।
इति
";
    let found = minters(synthetic);
    assert_eq!(found.len(), 2, "both minters are seen: {found:?}");
    let sets: Vec<&BTreeSet<String>> = found.iter().map(|(_, s)| s).collect();
    assert_ne!(
        sets[0], sets[1],
        "the one-arena minter must be distinguishable from the three-arena one, \
         or the ratchet above cannot fail"
    );

    // AND THE READ/WRITE DIRECTION IS THE PART MOST EASILY GOT WRONG: the same
    // three words in the other order are a READ and must not count as a write.
    let read_only = "\
वृत्तिः संज्ञाग्रहणम् ददाति न६४ आदि
    प्रत्यागमनम् ० ।
इति
वृत्तिः पाठकः आदाय निर्णायकः ॱॱ निर्णायक ददाति न६४ आदि
    चरः संज्ञा ॱॱ न६४ भवति संज्ञाग्रहणम् निर्णायकः ।
    चरः भेदः ॱॱ न६४ भवति संज्ञाभेदकोश अङ्कः संज्ञा अन्तः ।
इति
";
    let r = minters(read_only);
    assert_eq!(r.len(), 1, "the reader is a minter: {r:?}");
    assert!(
        r[0].1.is_empty(),
        "a READ (`भवति NAME अङ्कः … अन्तः`) was counted as a write; the order of `भवति` \
         against `अङ्कः` is the only thing that separates them. Got {:?}",
        r[0].1
    );
}
