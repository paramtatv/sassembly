//! **THE DRIVER FILLS THE SPEC-TABLE STORE, THROUGH THE WHOLE CHAIN** — the
//! half of `SAS-014` that had landed as code and was asserted by nothing.
//!
//! # What this file is the evidence for
//!
//! `lex.t1`'s four `समावेश*` globals were filled by the HOST (`nirvahana.rs:1000`
//! reads `spec/` at load), so a compiled compiler carried no mnemonic, directive
//! or akṣara table and assembled nothing. `sarani.t1` is that store written in
//! this language and `शृङ्खलाॱसङ्कलनारम्भः` (`shrinkhala.t1:705`) calls its fill
//! as its first statement.
//!
//! [`t1_sarani`] already runs `समावेशसारणीॱसारणीपूरणम्` and compares what it
//! writes against `spec/`. **That grades the MODULE, and deliberately so** — its
//! own loader carries two sources, `lex.t1` and `sarani.t1`, because loading the
//! chain would put six other modules' globals in scope. So nothing there says
//! the DRIVER calls it, and the driver's call is the whole of the open half.
//!
//! # Why the real `spec/` cannot see this and every existing test passes over it
//!
//! Measured 2026-09-17, by deleting the call at `shrinkhala.t1:705` and running
//! both roots:
//!
//! ```text
//!                         empty spec root                    real spec/
//!   as it stands     store ० -> १३, answers १८        store १३ BEFORE the call
//!   call deleted     store ०, REFUSES in पठनम्         store १३, answers १८
//! ```
//!
//! Against the real `spec/` the host has already written all thirteen by the
//! time the driver runs, so the mutation is **GREEN** there and the fill is
//! invisible. The empty root is the only place the driver's own fill is
//! observable, which is why this file has one — the same reason
//! [`t1_sarani`] gives, applied one caller up.
//!
//! [`t1_sarani`]: ../t1_sarani/index.html

use sadhana::t1::anita;
use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Value};
use std::path::{Path, PathBuf};

/// Unique scratch roots, shared with every other binary that needs one —
/// because five copies of `(pid, counter)` was five copies of one defect.
mod spec_fixture;

/// Enough for the fill plus the two table reads `सङ्कलनारम्भः` performs after
/// it. A bound on steps, not a timeout.
const FUEL: u64 = 40_000_000_000;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spec_root() -> PathBuf {
    repo_root().join("spec")
}

/// A spec root that exists and holds NOTHING, so the host's load-time fill
/// reads no file and leaves `समावेशसंख्या` at ०.
///
/// Per CALL, with a process id and a counter in the name, for the reason
/// `t1_exec_aksara.rs` gives: two tests that share a temp root can remove it
/// under each other.
fn empty_spec_root() -> PathBuf {
    // `unique_root` and not a local (pid, counter): that pair is unique inside
    // ONE PROCESS and not on disk, because pids are reused and these roots are
    // never removed. This family had accumulated its own pile of directories
    // under $TMPDIR (W-301).
    let dir = spec_fixture::unique_root("chain-tables-nospec");
    std::fs::create_dir_all(&dir).expect("temp spec root");
    dir
}

/// **THE LOADER IS PART OF THE TEST**, so this file has its own — but unlike
/// [`t1_sarani`](../t1_sarani/index.html)'s it is the WHOLE `CHAIN`, because the
/// chain is the thing under test. `शृङ्खला` sits at the top of the dependency
/// graph and `समावेशसारणी` is reachable from it only because `chain.rs:182`
/// lists `sarani.t1`; a two-source loader could not tell that.
///
/// The store is asserted EMPTY before anything is called. Without that, every
/// assertion below would be about the host's fill and not the driver's.
fn load_chain(spec: &Path) -> Interpreter {
    let it = Interpreter::load(CHAIN, spec).expect("the whole chain loads");
    assert_eq!(
        it.global("समावेशसंख्या").and_then(Value::as_int),
        Some(0),
        "an empty spec root must leave the store empty before the driver runs — \
         if this is not ०, what follows measures the HOST's fill, not the driver's"
    );
    it
}

/// `(table name, file bytes)` for every table `anita.rs` names, in its order —
/// which is the order `सारणीपूरणम्` indexes the three arenas by, because the
/// generator reads that same `const`. Read from disk, never listed here.
fn tables_from_disk() -> Vec<(&'static str, Vec<u8>)> {
    anita::table_names()
        .into_iter()
        .map(|n| {
            let rel = anita::table_path(n).unwrap_or_else(|| panic!("{n} has a file in TABLES"));
            let bytes = std::fs::read(spec_root().join(rel))
                .unwrap_or_else(|e| panic!("spec/{rel} is readable: {e}"));
            (n, bytes)
        })
        .collect()
}

fn entry(it: &Interpreter, global: &str, index: usize) -> Value {
    let v = it
        .global(global)
        .unwrap_or_else(|| panic!("{global} is a global of पदविभाग"));
    match v {
        Value::Arena(a) => a
            .borrow()
            .get(index)
            .unwrap_or_else(|| panic!("{global} has no entry {index}"))
            .clone(),
        other => panic!("{global} is not an arena, it is {other:?}"),
    }
}

fn entry_octets(it: &Interpreter, global: &str, index: usize) -> Vec<u8> {
    match entry(it, global, index) {
        Value::Octets(o) => o.as_slice().to_vec(),
        other => panic!("{global}[{index}] is not a run of octets, it is {other:?}"),
    }
}

/// **THE STORE A COMPILED COMPILER WOULD CARRY, PUT THERE BY THE DRIVER.**
///
/// Every table is compared against the file on disk, not against a count and
/// not against a list written here: `SAS-014` was filed because the native
/// image assembled nothing, and a store holding thirteen entries of the wrong
/// bytes would assemble nothing just as well.
#[test]
fn the_driver_fills_the_embed_store_through_the_whole_chain() {
    let dir = empty_spec_root();
    let mut it = load_chain(&dir);
    let want = tables_from_disk();

    it.call("शृङ्खलाॱसङ्कलनारम्भः", Vec::new(), FUEL)
        .expect("सङ्कलनारम्भः runs");

    let count = it
        .global("समावेशसंख्या")
        .and_then(Value::as_int)
        .expect("समावेशसंख्या is a number");
    assert_eq!(
        count,
        want.len() as i128,
        "समावेशसंख्या after the driver's call; anita.rs names {} tables. ० here \
         means the driver never filled the store — the defect SAS-014 was filed for",
        want.len()
    );

    for (i, (name, bytes)) in want.iter().enumerate() {
        let got_name = entry_octets(&it, "समावेशनामकोश", i);
        assert_eq!(
            String::from_utf8_lossy(&got_name),
            *name,
            "समावेशनामकोश[{i}]: the driver's store names a different table than anita.rs does"
        );
        let got = entry_octets(&it, "समावेशपाठकोश", i);
        assert_eq!(
            got.len(),
            bytes.len(),
            "समावेशपाठकोश[{i}] ({name}): {} octets reached the store, spec/ holds {}",
            got.len(),
            bytes.len()
        );
        assert!(
            got == *bytes,
            "समावेशपाठकोश[{i}] ({name}): the right LENGTH and the wrong octets"
        );
    }
    eprintln!("METRIC chain_driver_filled_tables {count}");
}

/// **THREE OUTCOMES, NOT TWO — AND THE ORDERING IS THE MIDDLE ONE.**
///
/// `सङ्कलनारम्भः` fills the store and then runs two readers over it, and its
/// own margin (`shrinkhala.t1:701`) states why the order is that way: the
/// reset CLEARS the table cursors, so reading first and resetting after
/// "empties them, which is a layout that assembles cleanly and branches to the
/// wrong place". A store-count assertion alone cannot see that — the store
/// would be full and the readers would still have seen nothing.
///
/// ```text
///   store ०                            the driver never filled it
///   store १३ and the reader answers ०  filled, but AFTER the readers ran
///   store १३ and the reader answers n  filled in time to be used
/// ```
///
/// **THE MIDDLE ROW IS NOT REACHABLE BY REORDERING, AND THAT WAS MEASURED
/// RATHER THAN REASONED.** Moving the fill below the two readers was expected
/// to produce it. It does not: `संज्ञाकुलपठनम्` REFUSES on an empty store —
/// *"`ॱ दैर्घ्य` read from Int(0)"* — so `सङ्कलनारम्भः` never returns at all and
/// this test reds on the call rather than on the count. The ordering is
/// therefore guarded by a refusal and not by a silent ०, which is the better
/// of the two and is not what `shrinkhala.t1:701`'s margin predicts (*"a layout
/// that assembles cleanly and branches to the wrong place"* — that margin is
/// about the REAL spec root, where the host has already filled the store and
/// the reader has something to read).
///
/// The row is still asserted. An unreachable-today state that is cheap to
/// cover is worth covering: the refusal above depends on `संज्ञाकुलपठनम्`
/// dereferencing an empty entry, and a reader that grew a guard would start
/// answering ० instead.
///
/// **THE RETURN IS AN ORACLE ON THE FILE, NOT A PIN.** `सङ्कलनारम्भः` hands
/// back `निर्देशाः`, the row count `निर्देशकोशपठनम्` read out of
/// `spec/directives.tsv` — which is asserted against that file's own data rows,
/// so it moves with the table instead of having to be re-taken when it does.
#[test]
fn the_table_readers_ran_after_the_fill_and_not_before_it() {
    let dir = empty_spec_root();
    let mut it = load_chain(&dir);

    let read = it
        .call("शृङ्खलाॱसङ्कलनारम्भः", Vec::new(), FUEL)
        .expect("सङ्कलनारम्भः runs")
        .as_int()
        .expect("सङ्कलनारम्भः answers a number");

    let count = it.global("समावेशसंख्या").and_then(Value::as_int).unwrap_or(0);
    assert!(
        count > 0,
        "the store is empty, so the readers had nothing to read"
    );

    // Data rows of `spec/directives.tsv`: comments and blanks out, header row
    // off. Derived here rather than pinned, so the table can grow.
    let text = std::fs::read_to_string(spec_root().join("directives.tsv"))
        .expect("spec/directives.tsv is readable");
    let rows = text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .count()
        - 1;

    assert_eq!(
        read, rows as i128,
        "the store holds {count} tables but निर्देशकोशपठनम् read {read} directives \
         of spec/directives.tsv's {rows}. ० means the readers ran BEFORE the fill"
    );
    eprintln!("METRIC chain_driver_directives_read {read}");
}

/// **THE CASE THAT MUST STILL BE REFUSED, AND IT DOES NOT REFUSE WHERE THE
/// MARGIN EXPECTS IT TO.**
///
/// `chain.rs:160-176` warns that naming a qualified routine before the list
/// carries its file lets `व्याकर` fall back to the one-argument index form and
/// refuse a line that is CORRECT. Measured here by dropping `sarani.t1` from
/// `CHAIN`: **the chain still LOADS** — the trap does not arm at load — and the
/// refusal comes at RUN time from `सङ्कलनारम्भः`, naming
/// `समावेशसारणीॱसारणीपूरणम्`.
///
/// That distinction is the point of the test. "Unlisted is caught at load" was
/// the plausible reading and it is wrong; what actually protects the chain is
/// that the driver's first statement cannot run, loudly and by name. A silent
/// ० would have been the bad outcome and it is asserted against.
#[test]
fn a_chain_without_the_store_module_refuses_by_name_rather_than_filling_nothing() {
    let dir = empty_spec_root();
    let without: Vec<(&str, &str)> = CHAIN
        .iter()
        .filter(|(n, _)| *n != "sarani.t1")
        .copied()
        .collect();
    assert_eq!(
        without.len(),
        CHAIN.len() - 1,
        "CHAIN no longer carries sarani.t1 under that name — this test drops nothing"
    );

    let mut it = Interpreter::load(&without, &dir).expect(
        "the chain without sarani.t1 still LOADS; the unlisted module is a RUN-time refusal",
    );
    let err = it
        .call("शृङ्खलाॱसङ्कलनारम्भः", Vec::new(), FUEL)
        .expect_err("सङ्कलनारम्भः must NOT succeed with no store module in the chain");
    // `err.reason`, NOT `{err:?}`: Debug escapes every non-ASCII char, so the
    // refusal reads `समाव\u{947}श…` there and a containment check on a
    // Devanagari name can never match. Display is the raw string.
    let reason = err.reason;
    assert!(
        reason.contains("समावेशसारणीॱसारणीपूरणम्"),
        "the refusal must NAME the missing routine, not merely fail; it said: {reason}"
    );
    assert_eq!(
        it.global("समावेशसंख्या").and_then(Value::as_int),
        Some(0),
        "nothing else may fill the store — if it is non-० the fill did not come from सारणी"
    );
}
