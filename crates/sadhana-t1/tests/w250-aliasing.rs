//! W-250, STEP 1: does `भवति` between two run-typed names SHARE?
//!
//! The row states the answer AT THE TYPE (`nirvahana.rs:194`
//! `Arena(Rc<RefCell<Vec<Value>>>)`). The type is not the evidence: a `Clone`
//! that bumps an `Rc` only aliases if `भवति` actually goes through `Clone` and
//! nothing on the assignment path deep-copies. So this file RUNS a program
//! both ways for each of the four value shapes a `भवति` can carry, and reads
//! the answer out of the interpreter rather than out of the declaration.
//!
//! The shape of every case is the same and is what makes it a proof:
//!
//! 1. bind `क`, 2. `ख भवति क`, 3. write THROUGH `क`, 4. read THROUGH `ख`.
//!
//! If step 4 sees step 3, the two names are one thing. A test that only wrote
//! through `क` and read through `क` would pass whatever the answer is.

use sadhana::t1::nirvahana::{Interpreter, Value};
use std::path::Path;

/// Run one self-contained program. No embed, so nothing here depends on `spec/`.
fn run(src: &str, entry: &str) -> Value {
    let mut it = Interpreter::load(&[("test", src)], Path::new("."))
        .unwrap_or_else(|e| panic!("`{entry}` must load: {}", e.reason));
    it.call(entry, Vec::new(), 1_000_000)
        .unwrap_or_else(|e| panic!("`{entry}` must run: {}", e.reason))
}

fn int(src: &str, entry: &str) -> i128 {
    let v = run(src, entry);
    v.as_int()
        .unwrap_or_else(|| panic!("`{entry}` must answer a number, not {v:?}"))
}

/// The single source every case below is a routine of. One module, so a case
/// cannot accidentally read another's global.
const SRC: &str = "मण्डलम् परीक्षा ॥

संरचना बिन्दु आरभ्य
    मान ॱॱ अ६४
समाप्तम् ।

॰ ── the two ARENA halves ────────────────────────────────────────────────
॰ An arena starts EMPTY, length ० (`W-355`), and a write at index १ takes
॰ it to २ — slot ० is filled only by growing past it. The whole question is
॰ whether ख sees it.

सार्वजनिक वृत्तिः आयतनसाझा ददाति अ६४ आदि
    चरः क ॱॱ अङ्कः अन्तः बिन्दु भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः बिन्दु भवति क ।
    चरः नव ॱॱ बिन्दु भवति ० ।
    नव ॱ मान भवति ७ ।
    क अङ्कः १ अन्तः भवति नव ।
    प्रत्यागमनम् ख ॱ दैर्घ्य ।
इति

॰ The control: the SAME program with step 2 deleted. If this also answered
॰ २ the case above would be measuring the write, not the share.
सार्वजनिक वृत्तिः आयतनएकाकी ददाति अ६४ आदि
    चरः क ॱॱ अङ्कः अन्तः बिन्दु भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः बिन्दु भवति ० ।
    चरः नव ॱॱ बिन्दु भवति ० ।
    नव ॱ मान भवति ७ ।
    क अङ्कः १ अन्तः भवति नव ।
    प्रत्यागमनम् ख ॱ दैर्घ्य ।
इति

॰ And the value read back through the alias, not just the length: a length
॰ guard proves the slot is in range, NOT that it holds what was written.
सार्वजनिक वृत्तिः आयतनमूल्यम् ददाति अ६४ आदि
    चरः क ॱॱ अङ्कः अन्तः बिन्दु भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः बिन्दु भवति क ।
    चरः नव ॱॱ बिन्दु भवति ० ।
    नव ॱ मान भवति ७ ।
    क अङ्कः १ अन्तः भवति नव ।
    प्रत्यागमनम् ख अङ्कः १ अन्तः ॱ मान ।
इति

॰ ── RECORD ──────────────────────────────────────────────────────────────
सार्वजनिक वृत्तिः संरचनासाझा ददाति अ६४ आदि
    चरः क ॱॱ बिन्दु भवति ० ।
    चरः ख ॱॱ बिन्दु भवति क ।
    क ॱ मान भवति ९ ।
    प्रत्यागमनम् ख ॱ मान ।
इति

सार्वजनिक वृत्तिः संरचनाएकाकी ददाति अ६४ आदि
    चरः क ॱॱ बिन्दु भवति ० ।
    चरः ख ॱॱ बिन्दु भवति ० ।
    क ॱ मान भवति ९ ।
    प्रत्यागमनम् ख ॱ मान ।
इति

॰ ── OCTETS, which the row says must NOT be assumed to follow the arenas ──
सार्वजनिक वृत्तिः अष्टकसाझा ददाति अ६४ आदि
    चरः क ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः अ८ भवति क ।
    क अङ्कः ० अन्तः भवति ६५ ।
    प्रत्यागमनम् ख ॱ दैर्घ्य ।
इति

॰ the write itself, through the name that was written
सार्वजनिक वृत्तिः अष्टकलेखितम् ददाति अ६४ आदि
    चरः क ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    चरः ख ॱॱ अङ्कः अन्तः अ८ भवति क ।
    क अङ्कः ० अन्तः भवति ६५ ।
    प्रत्यागमनम् क ॱ दैर्घ्य ।
इति

॰ ── the value types, which copy ─────────────────────────────────────────
सार्वजनिक वृत्तिः सङ्ख्यानकल्पः ददाति अ६४ आदि
    चरः क ॱॱ अ६४ भवति ५ ।
    चरः ख ॱॱ अ६४ भवति क ।
    क भवति ९ ।
    प्रत्यागमनम् ख ।
इति

॰ ── AND THE ROW'S OWN CASE: one local run assigned to GLOBALS ───────────
॰ 'assigning one empty local run to four globals put four names on one
॰ run'. Globals go down a different path in `assign` than locals do, so
॰ the local-to-local case above does not answer this one.
चरः कोशः ॱॱ अङ्कः अन्तः बिन्दु भवति ० ।
चरः अन्यकोशः ॱॱ अङ्कः अन्तः बिन्दु भवति ० ।

सार्वजनिक वृत्तिः वैश्विकसाझा ददाति अ६४ आदि
    चरः रिक्तम् ॱॱ अङ्कः अन्तः बिन्दु भवति ० ।
    कोशः भवति रिक्तम् ।
    अन्यकोशः भवति रिक्तम् ।
    चरः नव ॱॱ बिन्दु भवति ० ।
    नव ॱ मान भवति ७ ।
    कोशः अङ्कः १ अन्तः भवति नव ।
    प्रत्यागमनम् अन्यकोशः ॱ दैर्घ्य ।
इति
";

#[test]
fn an_arena_assigned_between_two_names_is_one_arena() {
    let shared = int(SRC, "आयतनसाझा");
    let alone = int(SRC, "आयतनएकाकी");
    assert_eq!(
        alone, 0,
        "the CONTROL: with `ख भवति क` deleted, the write through `क` must not \
         reach `ख`, which stays a fresh run — length ० since W-355, no slot at \
         index ०. If this is not ० the shared case below is measuring the \
         write and not the share (१ is the pre-W-355 zero run of one nil)"
    );
    assert_eq!(
        shared, 2,
        "`ख भवति क` between two `अङ्कः अन्तः बिन्दु` names SHARES: a write \
         through `क` lengthened the run `ख` names. Control answered {alone}"
    );
    assert_eq!(
        int(SRC, "आयतनमूल्यम्"),
        7,
        "and the slot read through `ख` holds what was written through `क` — \
         a length guard alone would prove only that the index is in range"
    );
}

#[test]
fn a_record_assigned_between_two_names_is_one_record() {
    assert_eq!(
        int(SRC, "संरचनाएकाकी"),
        0,
        "the CONTROL: two separately-initialised `बिन्दु` records do not share"
    );
    assert_eq!(
        int(SRC, "संरचनासाझा"),
        9,
        "`ख भवति क` between two `संरचना`-typed names SHARES: `क ॱ मान भवति ९` \
         is visible through `ख`"
    );
}

#[test]
fn a_run_of_octets_does_not_share_the_way_an_arena_does() {
    // W-250 (3): `Octets` is `Rc<Vec<u8>>` PLUS a range, and indexed
    // assignment rebuilds the run and stores it back through the base rather
    // than mutating in place (`nirvahana.rs:1377`, "COPY-ON-WRITE"). So a byte
    // run is a VALUE under `भवति` even though its backing buffer is an `Rc`.
    // This is the case the row said must not be assumed to follow the arenas,
    // and it does not.
    let through_written = int(SRC, "अष्टकलेखितम्");
    let through_alias = int(SRC, "अष्टकसाझा");
    assert_eq!(
        through_written, 1,
        "the write itself must land: `क अङ्कः ० अन्तः भवति ६५` takes `क` to \
         one octet. Without this the next assertion would pass on a write that \
         never happened"
    );
    assert_eq!(
        through_alias, 0,
        "`अङ्कः अन्तः अ८` does NOT share: the write through `क` left `ख` empty. \
         Byte runs copy where arenas and records alias — the one asymmetry in \
         what `भवति` means"
    );
}

#[test]
fn a_number_assigned_between_two_names_is_copied() {
    assert_eq!(
        int(SRC, "सङ्ख्यानकल्पः"),
        5,
        "the contrast that makes the aliasing findable: `ख भवति क` on `अ६४` \
         COPIES, and the two spellings are identical"
    );
}

#[test]
fn the_rows_own_case_one_local_run_assigned_to_two_globals() {
    assert_eq!(
        int(SRC, "वैश्विकसाझा"),
        2,
        "the failure W-250 was found by: one empty local run assigned to two \
         globals puts BOTH global names on that ONE run, so a write through \
         `कोशः` is visible through `अन्यकोशः`. Globals take a different arm of \
         `assign` than locals, so this is not implied by the local case"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// W-250, THE SITE WHERE IT IS ALREADY LOAD-BEARING
//
// `encode.t1:4844`–`:4846` (`सङ्कोचः`) is a save/restore of the module's
// one-slot error state, and the margin says what it is for: "its refusal is
// thrown away, which `.ok()` does for free and this module's one-slot error
// state does not".
//
//     चरः पूर्वदोषः ॱॱ सङ्केतनदोष भवति अन्तिमसङ्केतनदोषः ।   ॰ save
//     चरः पदम् ॱॱ … भवति स्थानसङ्केतनम् … ।                  ॰ may set the error
//     अन्तिमसङ्केतनदोषः भवति पूर्वदोषः ।                     ॰ restore
//
// The save is a SHARE. Whether the restore restores anything therefore does
// not depend on this routine at all — it depends on how the error is SET
// somewhere else. `सङ्केतनदोषरचना` (`:1558`) writes `अन्तिमसङ्केतनदोषः भवति
// नव`, REBINDING the global to a fresh record, so `पूर्वदोषः` still names the
// old one and the restore works. Rewrite that one line as the in-place field
// write a Rust port would naturally use — `self.last_error.kind = …` — and
// this save/restore silently becomes a no-op, with nothing at either site to
// say so. Both halves are run below.

const SAVE_RESTORE: &str = "मण्डलम् परीक्षा ॥

संरचना दोष आरभ्य
    सङ्ख्या ॱॱ अ६४
समाप्तम् ।

चरः अन्तिमः ॱॱ दोष भवति ० ।

॰ the setter as `encode.t1:1558` writes it: a FRESH record, rebound.
सार्वजनिक वृत्तिः पुनर्बन्धः ददाति अ६४ आदि
    चरः नव ॱॱ दोष भवति ० ।
    नव ॱ सङ्ख्या भवति ९ ।
    अन्तिमः भवति नव ।
    प्रत्यागमनम् ० ।
इति

॰ the same setter written in place — what `self.last_error.kind = …` ports to.
सार्वजनिक वृत्तिः परिवर्तनम् ददाति अ६४ आदि
    अन्तिमः ॱ सङ्ख्या भवति ९ ।
    प्रत्यागमनम् ० ।
इति

सार्वजनिक वृत्तिः रक्षणम्पुनर्बन्धेन ददाति अ६४ आदि
    अन्तिमः ॱ सङ्ख्या भवति १ ।
    चरः पूर्वः ॱॱ दोष भवति अन्तिमः ।
    चरः अवगणना ॱॱ अ६४ भवति पुनर्बन्धः ।
    अन्तिमः भवति पूर्वः ।
    प्रत्यागमनम् अन्तिमः ॱ सङ्ख्या ।
इति

सार्वजनिक वृत्तिः रक्षणम्परिवर्तनेन ददाति अ६४ आदि
    अन्तिमः ॱ सङ्ख्या भवति १ ।
    चरः पूर्वः ॱॱ दोष भवति अन्तिमः ।
    चरः अवगणना ॱॱ अ६४ भवति परिवर्तनम् ।
    अन्तिमः भवति पूर्वः ।
    प्रत्यागमनम् अन्तिमः ॱ सङ्ख्या ।
इति
";

#[test]
fn the_corpus_save_restore_works_only_because_the_setter_rebinds() {
    assert_eq!(
        int(SAVE_RESTORE, "रक्षणम्पुनर्बन्धेन"),
        1,
        "the corpus's own shape: the setter binds a FRESH record to the global, \
         so the saved name still holds the old one and the restore restores it"
    );
    assert_eq!(
        int(SAVE_RESTORE, "रक्षणम्परिवर्तनेन"),
        9,
        "ONE LINE CHANGED, in the setter and not at the save/restore: writing \
         the field in place instead of rebinding makes the save an alias of \
         the thing it is saving, and the restore a no-op. It answers ९ — the \
         value the restore was written to throw away. Nothing crashes, and \
         neither of the two sites that would have to change reads any \
         differently than it does today"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// W-250, THE CENSUS'S RESIDUE — checked rather than argued.
//
// `w250-shares.rs` cannot place eight sites, because their declared type is a
// name NO DECLARATION IN THE CORPUS DEFINES: `इ६४` is written as a type 46
// times and `पाठ` 42 times, and neither is ever a `संरचना` or anything else.
// The census's claim that those eight cannot be shares rests on what the
// interpreter does with an unknown type name, so that is run here instead of
// read off `zero_at`.

const UNDECLARED: &str = "मण्डलम् परीक्षा ॥

॰ `इ६४` is not a type this language has. Nothing refuses it.
सार्वजनिक वृत्तिः अघोषितप्रकारः ददाति अ६४ आदि
    चरः क ॱॱ इ६४ भवति ० ।
    चरः ख ॱॱ इ६४ भवति क ।
    क भवति ९ ।
    प्रत्यागमनम् ख ।
इति
";

#[test]
fn an_undeclared_type_name_is_refused_by_name_not_made_a_number() {
    // W-267 FLIPPED THIS, red first. It used to assert `int(UNDECLARED, …) == 0`:
    // an undeclared type name became `Value::Int(0)` and COPIED under `भवति`, which
    // is why the eight sites `w250-shares.rs` cannot place were not hidden shares.
    // That reasoning still holds for what those sites were; what changed is that
    // `इ६४` was respelled `अ६४` everywhere (W-267's first half) and `पाठ` now zeroes
    // to the empty text, so an undeclared name no longer has a corpus use to protect,
    // and binding it silently is the defect the row names.
    let mut it = Interpreter::load(&[("test", UNDECLARED)], Path::new(".")).unwrap_or_else(|e| {
        panic!(
            "UNDECLARED must load — the refusal is at the declaration: {}",
            e.reason
        )
    });
    match it.call("अघोषितप्रकारः", Vec::new(), 1_000_000) {
        Err(e) => assert!(
            e.reason.contains("इ६४"),
            "the refusal must name the undeclared type `इ६४`; it said: {}",
            e.reason
        ),
        Ok(v) => panic!("W-267: `चरः क ॱॱ इ६४ भवति ०` must be REFUSED, not bound to {v:?}"),
    }
}
