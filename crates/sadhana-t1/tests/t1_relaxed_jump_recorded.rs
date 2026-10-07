//! `W-306` — THE `.t1` EMITTER NEVER PROVED IT **RECORDS** THE RELAXATION'S FAR JUMP.
//!
//! `यन्त्रोत्सर्जनॱयन्त्रशाखावतरणम्`'s relaxed arm (`yantrotsarjana.t1:1553`–`:1562`)
//! writes the inverted conditional, then the far `लङ्घनम्` that carries the real target,
//! then the skip label — and the ONLY thing that puts that far jump into the list the
//! range checker walks is the single call on `:1554`:
//!
//! ```text
//! यदि विश्रम्भ समम् सत्यम् आदि
//!     यन्त्रलङ्घनलेखः तदा ।
//!     उत्सर्जनॱपाठयोजनम् उक्तम् लङ्घनम् शून्यःम् इति ।
//! ```
//!
//! Drop that one line and the relaxed routine's far jump is never in
//! `यन्त्रलङ्घनलक्ष्यकोश` at all, so `यन्त्रशाखादूरपरीक्षा`'s `यावत्` never reaches it, no
//! distance is measured and the wrap returns `सत्यम्` in silence — which is the exact
//! defect `W-306` opened on, ONE LEVEL UP from the checker. Every `W-306` test so far
//! sits at or below the checker: `t1_jump_range_bound.rs` and `t1_jump_block_target.rs`
//! both SEED the list by hand (`यन्त्रलङ्घनलेखः` called directly, the address tables
//! installed whole) and so are green against a lowering that records nothing. A fourth
//! arena-seeded file would add no row; this one drives the LOWERING and reads the list
//! back.
//!
//! **`यन्त्रलङ्घनलेखः` IS ADVANCE-THEN-WRITE AND IS CALLED BEFORE THE LINE GOES OUT**
//! (`:1397`–`:1401`): it bumps `यन्त्रलङ्घनसंख्या` first, then copies
//! `यन्त्राज्ञागणना` — the count of instruction lines written SO FAR — into the new row.
//! So the address it stores is the jump's OWN line, and moving the call below
//! `यन्त्राज्ञान्तः` would store the line AFTER it and read every distance one word
//! short. That is why each case here asserts `यन्त्राज्ञागणना` AFTER the call too: the
//! stored address and the final count are different numbers, and a test that read only
//! the stored one could not tell a correct write from an off-by-one-line write at a
//! different starting count.
//!
//! **THE SKIP LABEL DOES NOT BUMP THE COUNT.** `यन्त्रचिह्नान्तः` on `:1561` ends a
//! LABEL line, and only `यन्त्राज्ञान्तः` advances `यन्त्राज्ञागणना` — so the relaxed arm
//! writes three lines and advances the counter by TWO. The near-arm case below pins that
//! from the other side: with both arms firing the two recorded addresses are CONSECUTIVE
//! (`आरम्भः+१`, `आरम्भः+२`), which a label line counted as an instruction makes
//! `आरम्भः+१`, `आरम्भः+३`.
//!
//! **THE TARGET IS ASSERTED AND NOT JUST THE COUNT, BECAUSE THE NEAR ARM WRITES INTO
//! THE SAME LIST.** `:1565` is `यन्त्रलङ्घनलेखः अन्यत्` — same routine, same arena — so a
//! swap of `तदा` for `अन्यत्` on `:1554` keeps the count exactly right and aims the far
//! jump at the WRONG block. Every case here gives `तदा` and `अन्यत्` DIFFERENT ids, and
//! [`both_arms_firing_record_तदा_then_अन्यत्_in_that_order`] is the direct falsifier:
//! the list must read `तदा` at row `१` and `अन्यत्` at row `२`, never the reverse.
//!
//! **THE CASE THAT MUST STILL RECORD NOTHING** is the near mode with
//! `अग्रिमम् समम् अन्यत्` — no relaxation, and the conditional already carries its own
//! target, so neither `यन्त्रलङ्घनलेखः` runs and the arena stays empty. Without it a
//! lowering that recorded a jump unconditionally passes the first case.
//!
//! The fixture needs no register allocation and no IR: `संयोज्यम् ०` takes the
//! `शर्तम्` arm, and `यन्त्रपठनम् ०` answers `अधिकरणग्रहणम्`'s zero location
//! (`utsarjana.t1:1103`) — कोष्ठ `०`, no load emitted. The output buffer is appended to
//! and never read.
//!
//! THIS TEST'S OWN LOADER, as every test here has its own.

use sadhana::t1::nirvahana::{Interpreter, Value};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// `lex.t1` carries the spec table every set reaches, and the emitter reads the IR
/// enums and the output buffer, so `ir.t1` and `utsarjana.t1` come with it.
fn loaded() -> Interpreter {
    let names = ["lex.t1", "ir.t1", "utsarjana.t1", "yantrotsarjana.t1"];
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("{names:?} load: {e:?}"))
}

fn set_int(it: &mut Interpreter, name: &str, v: i128) {
    assert!(
        it.set_global(name, Value::Int(v)),
        "`{name}` is a global the loaded corpus declares"
    );
}

fn global_int(it: &Interpreter, name: &str) -> i128 {
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("`{name}` is a numeric global of the loaded corpus"))
}

/// Read one `अङ्कः अन्तः` numeric arena back WHOLE, index `०` included.
///
/// THE ROWS COME BACK AS `Option`S AND THAT IS THE POINT. A 1-based arena grown by an
/// appender leaves `शून्यम्` at index `०`, and `शून्यम्` is also what an index the writer
/// never reached holds — so `None` at `i` means THERE IS NO ROW `i`, which is a
/// different fact from row `i` holding `०`. Reading these as plain numbers would make
/// `0` and `absent` the same answer, and `absent` is exactly what the near-arm case
/// below has to be able to say.
fn table(it: &Interpreter, name: &str) -> Vec<Option<i128>> {
    match it.global(name) {
        Some(Value::Arena(rows)) => rows
            .borrow()
            .iter()
            .map(|v| match v {
                Value::Nil => None,
                other => Some(
                    other
                        .as_int()
                        .unwrap_or_else(|| panic!("`{name}` holds numbers, not {other:?}")),
                ),
            })
            .collect(),
        other => panic!("`{name}` is an `अङ्कः अन्तः` global, not {other:?}"),
    }
}

/// Row `i` of such an arena, or `None` if the arena never grew that far — so a missing
/// row and an unwritten one read alike and neither panics.
fn row(rows: &[Option<i128>], i: usize) -> Option<i128> {
    rows.get(i).copied().flatten()
}

/// How many rows the arena actually HOLDS, index `०`'s placeholder excluded. This and
/// not the raw length is what tells a recorded jump from none.
///
/// The placeholder is skipped BY POSITION: it read `Nil` until `W-381` stage 4,
/// when the owner ruled (2026-10-06, gap reads option A) that an unwritten slot
/// of an integer run reads ० in the interpreter as it does natively — so index
/// ० of a 1-based arena now reads `Some(0)`, and only its position marks it.
fn live(rows: &[Option<i128>]) -> usize {
    rows.iter().skip(1).filter(|r| r.is_some()).count()
}

/// Install one `अङ्कः अन्तः अ६४` address table WHOLE. Installed and not appended to,
/// because an out-of-range arena READ is a `RunError` where a WRITE resizes.
fn set_table(it: &mut Interpreter, name: &str, entries: &[i128]) {
    let rows = entries.iter().copied().map(Value::Int).collect::<Vec<_>>();
    assert!(
        it.set_global(name, Value::Arena(Rc::new(RefCell::new(rows)))),
        "`{name}` is an address table the loaded corpus declares"
    );
}

/// What the jump list held after one lowering, and where the instruction counter ended.
struct Recorded {
    count: i128,
    addresses: Vec<Option<i128>>,
    targets: Vec<Option<i128>>,
    lines: i128,
}

/// Lower ONE conditional of block `१` — condition `०`, target block `तदा`, else block
/// `अन्यत्`, falling into block `अग्रिमम्` — with the routine already `आरम्भः`
/// instruction lines long, in mode `relaxed`. Then read the jump list back.
///
/// `संयोज्यम् ०` is the `शर्तम्` arm: `यन्त्रपठनम् ०` answers location `०` and emits no
/// load, so the conditional is exactly ONE line and the addresses below are exact.
fn lower(
    start: i128,
    then_block: i128,
    else_block: i128,
    next_block: i128,
    relaxed: bool,
) -> Recorded {
    let mut it = loaded();

    assert_eq!(
        global_int(&it, "यन्त्रलङ्घनसंख्या"),
        0,
        "the jump list starts empty, so every row below is this lowering's"
    );

    set_int(&mut it, "यन्त्राज्ञागणना", start);
    // index १ is block १'s slot; ऋण१ is `no conditional line recorded yet`, which is
    // what the lowering overwrites with यन्त्राज्ञागणना on its way in.
    set_table(&mut it, "यन्त्रशाखास्थानकोश", &[-1, -1]);

    it.call(
        "यन्त्रोत्सर्जनॱयन्त्रशाखावतरणम्",
        vec![
            Value::Int(1),          // शाखापर्वम् — the BRANCHING block
            Value::Int(0),          // शर्तम् — location ०, no load
            Value::Int(then_block), // तदा
            Value::Int(else_block), // अन्यत्
            Value::Int(next_block), // अग्रिमम्
            Value::Int(0),          // संयोज्यम् — no folded comparison
            Value::Bool(relaxed),   // विश्रम्भ
        ],
        5_000_000,
    )
    .unwrap_or_else(|e| panic!("`यन्त्रशाखावतरणम्` runs: {e}"));

    assert_eq!(
        row(&table(&it, "यन्त्रशाखास्थानकोश"), 1),
        Some(start),
        "the conditional's own line is recorded for block १, as the range checker's \
         B-type half reads it"
    );

    Recorded {
        count: global_int(&it, "यन्त्रलङ्घनसंख्या"),
        addresses: table(&it, "यन्त्रलङ्घनस्थानकोश"),
        targets: table(&it, "यन्त्रलङ्घनलक्ष्यकोश"),
        lines: global_int(&it, "यन्त्राज्ञागणना"),
    }
}

/// THE RELAXED ARM RECORDS EXACTLY ONE JUMP, AT ITS OWN LINE, AIMED AT `तदा`.
///
/// `अग्रिमम् समम् अन्यत्` silences the near arm, so the one row can only be `:1554`'s.
/// The routine is `४०` lines in; the inverted conditional makes it `४१`, the far
/// `लङ्घनम्` is line `४१` and is recorded as `४१`, and the counter ends at `४२` —
/// the skip label after it advances nothing.
#[test]
fn the_relaxed_arm_records_its_far_jump_at_its_own_line_aimed_at_the_then_block() {
    let r = lower(40, 2, 7, 7, true);

    assert_eq!(
        r.count, 1,
        "`यन्त्रलङ्घनलेखः तदा` ran once — a relaxed conditional's far jump IS in the \
         list the range checker walks"
    );
    assert_eq!(
        live(&r.addresses),
        1,
        "one row and no other: index ० of a 1-based arena is never live ({:?})",
        r.addresses
    );
    assert_eq!(
        row(&r.addresses, 1),
        Some(41),
        "the jump's OWN line (यन्त्राज्ञागणना before `लङ्घनम्` went out), not ४२ — the \
         writer is called BEFORE the line, and below यन्त्राज्ञान्तः it would read one \
         word short"
    );
    assert_eq!(
        row(&r.targets, 1),
        Some(2),
        "तदा and not अन्यत् — the far jump carries the REAL target, which is the whole \
         point of inverting the conditional"
    );
    assert_eq!(
        r.lines, 42,
        "two instruction lines, not three: the `…पर्व१अतिक्रम` skip label ends with \
         यन्त्रचिह्नान्तः and advances no address"
    );
}

/// THE CASE THAT MUST STILL RECORD NOTHING. Near mode, and the fall-through block IS
/// the else block, so neither `यन्त्रलङ्घनलेखः` runs: the conditional carries its own
/// target and nothing jumps. Without this a lowering that recorded unconditionally
/// passes the case above.
#[test]
fn the_near_arm_with_the_else_block_next_records_no_jump_at_all() {
    let r = lower(40, 2, 7, 7, false);

    assert_eq!(
        r.count, 0,
        "no `लङ्घनम्` line was written, so no row belongs in the list"
    );
    assert_eq!(
        live(&r.addresses),
        0,
        "and NO row was written — not a row holding ०, no row at all: {:?}",
        r.addresses
    );
    assert_eq!(
        live(&r.targets),
        0,
        "nor into the target list: {:?}",
        r.targets
    );
    assert_eq!(
        r.lines, 41,
        "one line and one only — the conditional, aimed at तदा with no skip label"
    );
}

/// BOTH ARMS FIRING, AND THE ORDER IS THE FALSIFIER FOR A SWAP. Relaxed with
/// `अग्रिमम् असमम् अन्यत्`: the far jump goes out first and the near one after it, so
/// row `१` is `तदा` and row `२` is `अन्यत्`. Exchanging the two `यन्त्रलङ्घनलेखः`
/// arguments leaves `यन्त्रलङ्घनसंख्या` at `२` and is invisible to any test that reads
/// only the count.
///
/// THE TWO ADDRESSES ARE CONSECUTIVE, which is the skip label asserted from the other
/// side: `४१` and `४२`. A label line counted as an instruction reads `४१` and `४३`.
#[test]
fn both_arms_firing_record_the_then_block_then_the_else_block_in_that_order() {
    let r = lower(40, 2, 7, 9, true);

    assert_eq!(r.count, 2, "the far jump and the near one, two rows");
    assert_eq!(
        live(&r.addresses),
        2,
        "two rows and no third: {:?}",
        r.addresses
    );
    assert_eq!(
        (row(&r.targets, 1), row(&r.targets, 2)),
        (Some(2), Some(7)),
        "तदा on the FAR jump and अन्यत् on the near one, in the order the lines went \
         out — never the reverse"
    );
    assert_eq!(
        (row(&r.addresses, 1), row(&r.addresses, 2)),
        (Some(41), Some(42)),
        "CONSECUTIVE: the skip label sits between the two `लङ्घनम्` lines and advances \
         no address"
    );
    assert_eq!(
        r.lines, 43,
        "three instruction lines — conditional, far jump, near jump — and one label"
    );
}
