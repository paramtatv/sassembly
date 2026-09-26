//! W-266 — `सङ्कोचः`'s save/restore of the module's one-slot error state,
//! run against **`encode.t1` itself** rather than against a model of it.
//!
//! # What W-250 already established, and what it did not
//!
//! `w250-aliasing.rs` proves the language rule (records alias under `भवति`)
//! and then demonstrates the consequence on a HAND-WRITTEN module — a
//! two-field `दोष` record and a four-line setter, typed into the test. That
//! is evidence about `भवति`. It is **not** evidence about `encode.t1`: every
//! one of its assertions stays green if someone rewrites `सङ्केतनदोषरचना`
//! (`encode.t1:1558`) in place tomorrow, because no assertion in that file
//! reads `encode.t1` at all.
//!
//! This file closes that gap. It loads the real `encode.t1`, uses the real
//! five-field `सङ्केतनदोष`, the real global `अन्तिमसङ्केतनदोषः` and the real
//! setter `सङ्केतनदोषरचना`, and runs the save/restore both ways.
//!
//! # The one stand-in, named
//!
//! `सङ्कोचः` cannot be called from here: it takes a `वाक्यविभागॱआज्ञा`, and
//! `वाक्यविभाग` is not in this image. So the probe below performs the save
//! and the restore with **the lines lifted out of `सङ्कोचः` by this file at
//! run time** (`save_lines`, which fails if the file no longer contains
//! them), and replaces the middle line — the call to
//! `स्थानसङ्केतनम्` — with a direct call to `सङ्केतनदोषरचना`, which is what
//! `स्थानसङ्केतनम्` (`encode.t1:3778`) itself does at NINETEEN places in its
//! own body, from `:3788` on, every time it refuses. The saved and restored
//! lines are therefore the corpus's; only the thing that sets the error in
//! the middle is reached directly instead of through one more frame.
//!
//! (An earlier draft of this note cited `:5123`, `:5221` and `:5284` for
//! that. Checked: those three are in `उत्सर्जनक्रमः`, `लक्ष्यसङ्केतनम्` and
//! `लक्ष्यवस्तुसङ्केतनम्`, not in `स्थानसङ्केतनम्` at all.)
//!
//! The marker is the `पङ्क्ति` field, because it is the one field of
//! `सङ्केतनदोष` that is a plain number at both ends: `सङ्केताङ्क` goes
//! through `सङ्केतनकूटः` and comes back as a run of octets.

use sadhana::t1::nirvahana::Interpreter;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spec_root() -> PathBuf {
    repo_root().join("spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// Replace `from` with `to`, and **fail unless `from` occurs exactly once.**
/// A mutation that matches nothing changes nothing and the test it guards
/// then passes for the wrong reason; a mutation that matches twice is not the
/// mutation described. Both are refused. (Same contract as
/// `t1_exec_encode.rs`'s helper, copied for the same reason that file gives.)
fn mutate(text: &str, from: &str, to: &str) -> String {
    let n = text.matches(from).count();
    assert_eq!(
        n, 1,
        "the mutation `{from}` -> `{to}` matches {n} places in encode.t1; \
         a mutation test is only evidence when it changes exactly one"
    );
    text.replace(from, to)
}

// ─────────────────────────────────────────────────────────────────────────
// The corpus text this file depends on, quoted once each.
// ─────────────────────────────────────────────────────────────────────────

/// `सङ्केतनदोषरचना`'s body as `encode.t1:1553`–`:1558` writes it: a FRESH
/// record, filled, then bound over the global.
const SETTER_REBINDING: &str = "\
    चरः नव ॱॱ सङ्केतनदोष भवति ० ।
    नव ॱ पङ्क्ति भवति पङ्क्तिः ।
    नव ॱ सङ्केताङ्क भवति सङ्केतनकूटः सङ्ख्या ।
    नव ॱ पदार्थारम्भ भवति पदार्थारम्भः ।
    नव ॱ पदार्थसंख्यान भवति पदार्थसंख्यानम् ।
    अन्तिमसङ्केतनदोषः भवति नव ।";

/// The same setter written the way a Rust port of `EncodeError` produces
/// naturally — `self.last_error.line = line;` — four field writes straight
/// through the global and no rebinding. Behaviourally identical at both of
/// the setter's own ends: same fields, same values, same global afterwards.
const SETTER_IN_PLACE: &str = "\
    अन्तिमसङ्केतनदोषः ॱ पङ्क्ति भवति पङ्क्तिः ।
    अन्तिमसङ्केतनदोषः ॱ सङ्केताङ्क भवति सङ्केतनकूटः सङ्ख्या ।
    अन्तिमसङ्केतनदोषः ॱ पदार्थारम्भ भवति पदार्थारम्भः ।
    अन्तिमसङ्केतनदोषः ॱ पदार्थसंख्यान भवति पदार्थसंख्यानम् ।";

/// The restore, exactly as `सङ्कोचः` (`encode.t1:4870`) writes it. Read back out of the
/// file by `save_and_restore_lines` below; this copy exists only so that the
/// extraction has something to look for.
const RESTORE_LINE: &str = "    अन्तिमसङ्केतनदोषः भवति पूर्वदोषः ।";

/// Everything `सङ्कोचः` saves with, whatever that currently is: the
/// `चरः पूर्वदोषः` declaration and every line after it that writes through
/// `पूर्वदोषः`.
///
/// Extracted rather than transcribed, so that a change to the save at
/// `:4857` is carried into this probe instead of being missed by it — and so
/// that the probe cannot silently keep exercising a save the corpus no
/// longer has.
fn save_lines(encode: &str) -> String {
    let mut saw = 0usize;
    let mut out = String::new();
    let mut collecting = false;
    for line in encode.lines() {
        let t = line.trim_start();
        if t.starts_with("चरः पूर्वदोषः") {
            saw += 1;
            collecting = true;
        } else if collecting && !t.starts_with("पूर्वदोषः ॱ") {
            collecting = false;
        }
        if collecting {
            out.push_str(line);
            out.push('\n');
        }
    }
    assert_eq!(
        saw, 1,
        "encode.t1 must declare `पूर्वदोषः` exactly once — the save in \
         `सङ्कोचः`. Found {saw}, so this probe no longer knows which one it \
         is lifting"
    );
    assert!(
        !out.is_empty(),
        "the save statement in `सङ्कोचः` came back empty"
    );
    out
}

/// The probe: `encode.t1` with one routine appended that does what
/// `सङ्कोचः` (`encode.t1:4857`–`:4870`) does, with the save and restore lifted from the
/// file and the middle call replaced (see the module docs).
/// `before` is what has already happened to the error slot when `सङ्कोचः` is
/// entered — a recorded refusal, or nothing at all on the first call of a
/// run, when the global is still `सङ्केतनदोष भवति ०`.
fn probe(encode: &str, before: &str) -> String {
    assert!(
        encode.contains(RESTORE_LINE),
        "`सङ्कोचः` must still restore with `{}`; if the restore changed \
         shape this probe is measuring something else",
        RESTORE_LINE.trim()
    );
    format!(
        "{encode}

सार्वजनिक वृत्तिः दोषरक्षणपरीक्षा ददाति न६४ आदि
{before}{save}    सङ्केतनदोषरचना ९ २३ ० ० ।
{restore}
    प्रत्यागमनम् अन्तिमसङ्केतनदोषः ॱ पङ्क्ति ।
इति
",
        save = save_lines(encode),
        restore = RESTORE_LINE,
    )
}

/// The slot holding a refusal on entry — the case the row is about.
const A_REFUSAL_IS_LIVE: &str = "    सङ्केतनदोषरचना १ २२ ० ० ।\n";
/// The slot untouched — `सङ्कोचः`'s FIRST call in a run, where the global is
/// still the zero-initialised record `encode.t1:1475` binds.
const NOTHING_RECORDED_YET: &str = "";

fn run_probe(encode: &str) -> i128 {
    run_probe_from(encode, A_REFUSAL_IS_LIVE)
}

fn run_probe_from(encode: &str, before: &str) -> i128 {
    let mut it = Interpreter::load(
        &[
            ("encode.t1", &probe(encode, before)),
            ("sanskrit_text.t1", &source("sanskrit_text.t1")),
        ],
        &spec_root(),
    )
    .unwrap_or_else(|e| panic!("the probe image loads: {}", e.reason));
    let v = it
        .call("सङ्केतनॱदोषरक्षणपरीक्षा", Vec::new(), 20_000_000)
        .unwrap_or_else(|e| panic!("the probe runs: {}", e.reason));
    v.as_int()
        .unwrap_or_else(|| panic!("the probe answers a number, not {v:?}"))
}

/// The setter's own contract, so that neither arm below can pass because the
/// setter stopped working: after `सङ्केतनदोषरचना ९ २३ ० ०` the global's line
/// is ९, however the setter is written.
fn setter_writes_the_line(encode: &str) -> i128 {
    let src = format!(
        "{encode}

सार्वजनिक वृत्तिः दोषनिर्धारणपरीक्षा ददाति न६४ आदि
    सङ्केतनदोषरचना ९ २३ ० ० ।
    प्रत्यागमनम् अन्तिमसङ्केतनदोषः ॱ पङ्क्ति ।
इति
"
    );
    let mut it = Interpreter::load(
        &[
            ("encode.t1", &src),
            ("sanskrit_text.t1", &source("sanskrit_text.t1")),
        ],
        &spec_root(),
    )
    .unwrap_or_else(|e| panic!("the setter image loads: {}", e.reason));
    it.call("सङ्केतनॱदोषनिर्धारणपरीक्षा", Vec::new(), 20_000_000)
        .unwrap_or_else(|e| panic!("the setter probe runs: {}", e.reason))
        .as_int()
        .expect("the setter probe answers a number")
}

// ─────────────────────────────────────────────────────────────────────────
// W-266 (1): the two answers, from encode.t1.
// ─────────────────────────────────────────────────────────────────────────

/// **THE ROW'S TEST, AND THE FIX'S.**
///
/// `सङ्कोचः` saves the error slot, lets a call overwrite it, and restores.
/// The restore must give back the saved line — ONE — and it must do so
/// whether or not `सङ्केतनदोषरचना` happens to rebind the global.
///
/// Before the fix, the second arm answered ९: the "save" was an alias of the
/// thing being saved, so an in-place setter mutated the copy too and the
/// restore was a no-op that put back the value it was written to throw away.
/// Nothing crashed, and neither of the two sites in `सङ्कोचः` read any
/// differently.
#[test]
fn the_save_restore_survives_a_setter_that_does_not_rebind() {
    let encode = source("encode.t1");

    // The controls: the setter must actually set, both ways round. Without
    // these, an arm answering १ could be a setter that did nothing.
    assert_eq!(
        setter_writes_the_line(&encode),
        9,
        "CONTROL: `सङ्केतनदोषरचना ९ २३ ० ०` must leave the global's `पङ्क्ति` \
         at ९. If it does not, the arms below prove nothing about the restore"
    );
    let in_place = mutate(&encode, SETTER_REBINDING, SETTER_IN_PLACE);
    assert_eq!(
        setter_writes_the_line(&in_place),
        9,
        "CONTROL: the in-place setter must write the same line the rebinding \
         one does — the two differ only in whether the record is fresh"
    );

    assert_eq!(
        run_probe(&encode),
        1,
        "with the corpus setter as it stands: `सङ्कोचः`'s save/restore gives \
         back the line that was live before the call"
    );
    assert_eq!(
        run_probe(&in_place),
        1,
        "AND WITH THE SETTER REWRITTEN IN PLACE — `self.last_error.line = …`, \
         what a Rust port of `EncodeError` produces naturally, changed at \
         `encode.t1:1553`-`:1558` and nowhere near `सङ्कोचः` — the save/restore \
         must STILL give back ONE. It answered ९ before W-266: the save was a \
         share, so the restore put back the value it was written to throw away"
    );

    // THE FIRST CALL OF A RUN, which is the common case and not the one the
    // row describes: nothing has been recorded, so the global is still the
    // zero-initialised record `encode.t1:1475` binds. The save now READS five
    // fields off that record where it used to bind the whole thing, so this
    // is a path the fix newly touches — and a zero-init record answering a
    // field read is not something the old save had to rely on.
    assert_eq!(
        run_probe_from(&encode, NOTHING_RECORDED_YET),
        0,
        "with nothing yet recorded, the save must copy the zero-initialised \
         slot and the restore must put back line ०, not leave ९ behind"
    );
    assert_eq!(
        run_probe_from(&in_place, NOTHING_RECORDED_YET),
        0,
        "and the same on the untouched slot with the setter in place"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// W-266 (3): the thing that was silently holding it up.
// ─────────────────────────────────────────────────────────────────────────

/// The save must copy **every** field of `सङ्केतनदोष`, and this asserts the
/// COUNT rather than that copying happens.
///
/// A field-by-field copy is the one thing that rots silently: add a sixth
/// field to the record and the save keeps compiling, keeps running, and
/// silently stops saving it. So the field list is counted out of the
/// `संरचना` and matched against the writes in `सङ्कोचः`'s save, by name.
#[test]
fn the_save_copies_every_field_the_record_declares() {
    let encode = source("encode.t1");

    // The record's fields, out of `सार्वजनिक संरचना सङ्केतनदोष आरभ्य … समाप्तम्`.
    let head = "सार्वजनिक संरचना सङ्केतनदोष आरभ्य";
    let start = encode
        .find(head)
        .expect("encode.t1 declares `संरचना सङ्केतनदोष`")
        + head.len();
    let body = &encode[start..];
    let end = body.find("समाप्तम्").expect("the record declaration closes");
    let declared: Vec<(String, String)> = body[..end]
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.is_empty() || l.starts_with('॰') {
                return None;
            }
            // `पङ्क्ति ॱॱ न६४ ऽ   ॰ line: 1-based` — name, then type up to the
            // field separator `ऽ` or the margin `॰`.
            let (name, rest) = l.split_once("ॱॱ")?;
            let ty = rest
                .split('॰')
                .next()
                .unwrap_or("")
                .replace('ऽ', "")
                .trim()
                .to_string();
            Some((name.trim().to_string(), ty))
        })
        .collect();
    assert!(
        declared.len() >= 5,
        "`सङ्केतनदोष` should declare at least the five fields W-266 saw; \
         found {declared:?}"
    );

    // THE HOLE A FIELD-BY-FIELD COPY STILL HAS, and it is not hypothetical:
    // `encode.t1:1470` records that this very record once carried
    // `आदानानि ॱॱ अङ्कः अन्तः पाठ`. An `अङ्कः अन्तः <record>` field is an
    // ARENA, and arenas SHARE under `भवति` (`w250-aliasing.rs`) — so copying
    // such a field one at a time would put the copy and the original on one
    // arena and leave the save exactly as aliased as it was before W-266.
    // The copy below is a real copy only while every field is a scalar or a
    // run of octets.
    for (name, ty) in &declared {
        assert!(
            !ty.starts_with("अङ्कः"),
            "field `{name}` of `सङ्केतनदोष` is declared `{ty}` — an ARENA, \
             which SHARES under `भवति`. `सङ्कोचः`'s save copies field by \
             field, so an arena field silently re-aliases the slot and W-266 \
             is back. Deep-copy it there, or keep this record scalar"
        );
    }
    let declared: Vec<String> = declared.into_iter().map(|(n, _)| n).collect();

    // The save, out of `सङ्कोचः`. Each field must be copied FROM the matching
    // field of the global — not merely assigned something, which
    // `पूर्वदोषः ॱ कारण भवति ०` would also satisfy.
    let save = save_lines(&encode);
    let copied = |f: &str| save.contains(&format!("पूर्वदोषः ॱ {f} भवति अन्तिमसङ्केतनदोषः ॱ {f}"));
    for field in &declared {
        assert!(
            copied(field),
            "the save in `सङ्कोचः` must copy `{field}` out of the global — \
             every field of `सङ्केतनदोष` ({declared:?}), or the restore \
             silently puts back a record missing one. The save reads:\n{save}"
        );
    }
    let written = declared.iter().filter(|f| copied(f)).count();
    assert_eq!(
        written,
        declared.len(),
        "COUNT: {} fields declared, {written} copied by the save. Asserting \
         the count and not just that copying happens: a save that copies four \
         of five fields is the same silent no-op W-266 was",
        declared.len()
    );
}

/// **THE TEST THE ROW ASKED FOR: it fails the moment `सङ्केतनदोषरचना` stops
/// rebinding a fresh record.**
///
/// Run, not read. A name is bound to the global, the corpus setter is called,
/// and the OLD name is read back. If the setter binds a fresh record the old
/// name still holds the old line (१); if it writes the global's fields in
/// place, the old name is the global and reads the new line (९). No text
/// matching: a source-text guard would also fail on a reflow, and would pass
/// on an in-place rewrite spelled differently.
///
/// This is the property that silently held `सङ्कोचः`'s save/restore up until
/// W-266. It is no longer load-bearing there — the test above runs the save
/// with the setter mutated in place and gets १ either way — so **a failure
/// here is a fact to check, not by itself a bug**: it says the module's
/// one-slot error state stopped being replaced and started being mutated, and
/// every remaining `भवति` onto `सङ्केतनदोष` becomes a W-250 candidate.
#[test]
fn the_setter_still_rebinds_a_fresh_record() {
    let encode = source("encode.t1");

    // The same probe against the setter rewritten in place, to prove this
    // instrument can tell the two apart at all. Without it, a probe that
    // always answered १ would look like a passing test.
    let in_place = mutate(&encode, SETTER_REBINDING, SETTER_IN_PLACE);
    assert_eq!(
        rebinding_probe(&in_place),
        9,
        "CONTROL: with `सङ्केतनदोषरचना` writing the global's fields in place, \
         a name bound to the global before the call MUST see ९ afterwards. If \
         this is not ९ the assertion below cannot detect the change it exists \
         to detect"
    );

    assert_eq!(
        rebinding_probe(&encode),
        1,
        "`सङ्केतनदोषरचना` (`encode.t1:1552`) builds a FRESH `सङ्केतनदोष` and \
         binds it over `अन्तिमसङ्केतनदोषः`, so a name bound to the global \
         before the call still holds the old record. It answered ९, which \
         means the setter now mutates the slot in place. Read this test's note"
    );
}

/// Bind a name to `अन्तिमसङ्केतनदोषः`, call the corpus setter, read the old
/// name's `पङ्क्ति`. १ if the setter rebinds, ९ if it writes in place.
fn rebinding_probe(encode: &str) -> i128 {
    let src = format!(
        "{encode}

सार्वजनिक वृत्तिः पुनर्बन्धपरीक्षा ददाति न६४ आदि
    सङ्केतनदोषरचना १ २२ ० ० ।
    चरः पूर्वः ॱॱ सङ्केतनदोष भवति अन्तिमसङ्केतनदोषः ।
    सङ्केतनदोषरचना ९ २३ ० ० ।
    प्रत्यागमनम् पूर्वः ॱ पङ्क्ति ।
इति
"
    );
    let mut it = Interpreter::load(
        &[
            ("encode.t1", &src),
            ("sanskrit_text.t1", &source("sanskrit_text.t1")),
        ],
        &spec_root(),
    )
    .unwrap_or_else(|e| panic!("the rebinding image loads: {}", e.reason));
    it.call("सङ्केतनॱपुनर्बन्धपरीक्षा", Vec::new(), 20_000_000)
        .unwrap_or_else(|e| panic!("the rebinding probe runs: {}", e.reason))
        .as_int()
        .expect("the rebinding probe answers a number")
}
