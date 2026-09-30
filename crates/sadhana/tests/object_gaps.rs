//! What an object cannot carry is refused, never dropped — task `B-096`.
//!
//! `B-096` asked to retire `बन्धकः` and route every build through objects.
//! Measuring first found four things the executable path does that the object
//! path does not, and **three of them were silent**:
//!
//! | capability            | as an object, before this |
//! |-----------------------|---------------------------|
//! | address in `ॱदत्त`    | refused; now `.rela.data` (`B-098`) |
//! | `ॱरिक्त`              | dropped; now `SHT_NOBITS` (`B-099`) |
//! | `--संक्षिप्त`         | ignored; now honoured (`B-069e`) |
//! | `-g`                  | ignored; now `.debug_line` (`B-100b`) |
//!
//! All four are closed. What remains of this file is the guard: each was
//! SILENT, and a gap that says nothing is indistinguishable from a feature that
//! works. The refusals are gone; the tests that they were reachable are not.
//!
//! A gap that says nothing is indistinguishable from a feature that works. That
//! is how `B-058b2b3` shipped a compressed flag reaching a linker that laid
//! instructions out at `i * 4`, and how `B-069d2b` shipped an address resolved
//! before the layout that decided it. So each gap now names itself and the task
//! that closes it, and `B-096` cannot be done until they are closed.

use std::process::Command;

use sadhana::encode::encode_object;
use sadhana::parse::assemble_program;

/// A program reserving `n` bytes in `ॱरिक्त`.
fn with_bss(n: &str) -> String {
    format!("कॱॱ\nयोगः अर्थ०म् शून्यःन ७न ।\n॥ कोष्ठकम् ॱरिक्त ॥\nबफरॱॱ\n॥ स्थानम् {n} ॥\n")
}

#[test]
fn reserved_space_reaches_the_object_and_the_program_linked_from_it() {
    // `--वस्तु` on this wrote a four-byte object and exited 0. The 64 bytes were
    // gone: a program linked from it would have run with its buffer overlapping
    // whatever the linker put next. It was refused as `E19` for three cycles;
    // `B-099` writes it.
    let p = assemble_program(&with_bss("६४")).expect("parses");
    assert_eq!(p.bss, 64, "the reservation is in the program");

    let (text, pending) = encode_object(&p).expect("an object carries it now");
    let bytes = sadhana::kosha::object(
        &text,
        &p,
        &pending,
        None,
        &sadhana::encode::layout_addresses(&p, sadhana::encode::Target::Uncompressed),
    );
    let o = sadhana::vastu::read(&bytes).expect("reads back");
    assert_eq!(o.bss, 64, "SHT_NOBITS carries a size and no bytes");

    // The name is in `.bss`, not `.data`. A buffer reported as initialised data
    // is a claim the file does not support, and `nm` printed `d` for it.
    let buffer = o
        .symbols
        .iter()
        .find(|s| s.name == "बफर")
        .expect("the buffer is named");
    assert_eq!(buffer.placement, sadhana::vastu::Placement::Bss);

    // And it survives linking, which is the half that makes it real: a
    // reservation the linker drops is the same defect one section later.
    let linked = sadhana::samyojana::link(&[o]).expect("links");
    assert_eq!(linked.bss, 64);
    assert!(
        linked.data.len() < 64,
        "reserved space must not become file bytes: {} of them",
        linked.data.len()
    );
    // Its address is past the data rather than on top of it.
    let at = *linked.symbols.get("बफर").expect("placed");
    assert!(
        at >= sadhana::kosha::LOAD_ADDRESS + linked.text.len() as u64,
        "the buffer overlaps the text at {at:#x}"
    );
}

#[test]
fn a_program_with_no_reservation_is_unaffected() {
    // The refusal must be keyed on the reservation and not on the section, or
    // every object in the tree stops assembling.
    let p = assemble_program("कॱॱ\nयोगः अर्थ०म् शून्यःन ७न ।\n").expect("parses");
    let (text, pending) = encode_object(&p).expect("no ॱरिक्त, nothing to carry");
    let bytes = sadhana::kosha::object(
        &text,
        &p,
        &pending,
        None,
        &sadhana::encode::layout_addresses(&p, sadhana::encode::Target::Uncompressed),
    );
    let o = sadhana::vastu::read(&bytes).expect("reads");
    assert_eq!(
        o.bss, 0,
        "no section is emitted for a reservation of nothing"
    );
}

#[test]
fn every_flag_an_object_takes_does_what_it_says() {
    // All four gaps `B-096` measured are closed, and the two that were flags
    // were closed by making them work rather than by refusing them. A flag that
    // writes a file and exits 0 while ignoring what was asked is
    // `B-058b2b3` — a compressed flag that reached a linker laying instructions
    // out at `i * 4`.
    //
    // So each is checked by its EFFECT. `--संक्षिप्त` must shrink a program
    // that has something to shrink; a test that only asserts exit 0 would pass
    // on the flag being dropped again.
    let dir = std::env::temp_dir().join(format!(
        // UNIQUE PER PROCESS AND PER RUN. A fixed name is SHARED: 43 worktrees
        // and several agents run gates on this machine at once, and two runs in
        // one directory corrupt each other. The clock is the load-bearing part --
        // pids are reused and these directories are never removed (W-301).
        "sansos-flags-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("क.sas");
    std::fs::write(
        &src,
        "कॱॱ\nयोगः अर्थ०म् शून्यःन ५न ।\nयोगः अर्थ१म् शून्यःन ७न ।\nलङ्घनम् शून्यःम् कय् ।\n",
    )
    .expect("write");

    let object = |args: &[&str], out: &str| {
        let path = dir.join(out);
        let r = Command::new(env!("CARGO_BIN_EXE_sadhana"))
            .args(args)
            .arg("--वस्तु")
            .arg(&src)
            .arg(&path)
            .output()
            .expect("run sadhana");
        assert!(
            r.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&r.stderr)
        );
        sadhana::vastu::read(&std::fs::read(&path).expect("read")).expect("reads")
    };

    let wide = object(&[], "wide.o");
    let short = object(&["--संक्षिप्त"], "short.o");
    assert_eq!(wide.text.len(), 12, "three wide instructions");
    assert_eq!(short.text.len(), 8, "two of them fit in a halfword each");

    let debugged = object(&["-g"], "debug.o");
    assert!(
        debugged.debug.iter().any(|(n, _)| n == ".debug_line"),
        "-g wrote no line table"
    );
    assert!(wide.debug.is_empty(), "and it is opt-in");
}

fn source(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("root")
        .join("spec")
        .join(name)
}

#[test]
fn every_code_these_gaps_raised_is_retired_with_them() {
    // The list above is only worth having if each entry points somewhere. A
    // diagnostic that says "not yet" and names no task is a dead end for
    // whoever reads it, and `B-096` depends on all three being findable.
    let table = std::fs::read_to_string(source("diagnostics.tsv")).expect("read");
    // Two of the four gaps are closed and their codes went with them: `E18`
    // named the refused address `B-098` now records, `E19` the dropped
    // reservation `B-099` now writes. A registered code nothing raises is a
    // message that cannot be reached.
    for retired in ["E18", "E19"] {
        assert!(
            !table
                .lines()
                .any(|l| l.starts_with(&format!("{retired}\t"))),
            "{retired} is still registered after the refusal it named was removed"
        );
    }

    // `C01` named the two flags an object could not honour, and both do now
    // (`B-069e`, `B-100b`). Every code this file was written about is retired,
    // which is what closing all four gaps means.
    assert!(
        !table.lines().any(|l| l.starts_with("C01\t")),
        "C01 is still registered after the refusals it named were removed"
    );
}
