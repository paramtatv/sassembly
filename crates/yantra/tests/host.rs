//! `F-001f` — the host, measured on the one application in this tree.
//!
//! # What these tests are for, given that `tests/application.rs` already runs `atithi`
//!
//! That file is ADR-0015's acceptance: it arranges a machine by hand, in the test, and
//! reads all five clauses out of the run. This file asserts something narrower and
//! different — that [`yantra::host::host`] makes the *same* arrangement, so a caller who
//! is not a test (the browser, next) gets a machine with the supervisor's word inside its
//! own RAM, a frame pool above it, a grant it did not choose the number of, and U-mode.
//!
//! If the two ever disagree, the browser is running something other than what the
//! acceptance measured, and the whole "the artefact in the page is the artefact QEMU
//! runs" claim quietly stops covering applications.
//!
//! # And what it refuses
//!
//! Three of these tests are refusals, because a host that only ever reports success is
//! indistinguishable from one that reports success. A program linked into the
//! supervisor's gigabyte, a machine too small to hold the address space, and a budget too
//! short to finish are the three ways hosting fails that are *not* the program's fault —
//! each has to name itself rather than arrive as an empty surface.

use std::path::{Path, PathBuf};

use sadhana::encode::{Target, encode_object, layout_addresses};
use sadhana::kosha::{object, write_debuggable_at};
use sadhana::parse::assemble_program;
use sadhana::samyojana::link_at;
use sadhana::vastu::read;
use yantra::host::{self, BASE, Hosted, host};
use yantra::supervisor::Ended;

/// The one program in `spec/` that is an application.
const APPLICATION: &str = "atithi.sas";

/// What it puts on its surface. Held against the source by
/// `tests/application.rs::the_application_carries_the_string_this_test_expects`, so this
/// constant cannot drift into being its own fixture.
const STRING: &str = "अतिथिः";

/// 4 MiB — the loader takes about a dozen frames and the rest is slack.
const RAM: usize = 1 << 22;

/// Ten instructions and two calls.
const BUDGET: u64 = 200;

/// Neither address is the supervisor's gigabyte, and the program is told neither.
fn linked_at() -> [u64; 2] {
    let tsv = std::fs::read_to_string(root().join("spec").join("application-load.tsv"))
        .expect("read application-load.tsv");
    let mut addrs = vec![];
    for line in tsv.lines() {
        if line.starts_with("0x") {
            let hex = line.split('\t').next().unwrap().replace("_", "");
            addrs.push(u64::from_str_radix(&hex[2..], 16).unwrap());
        }
    }
    [addrs[0], addrs[1]]
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/yantra has a grandparent")
        .to_path_buf()
}

/// Assemble and link `spec/atithi.sas` at `load` and return the ELF — the same four calls
/// `sadhana`'s own `main` makes, so what is hosted below is what the toolchain writes to
/// a file rather than a fixture this test invents.
fn elf_at(load: u64) -> Vec<u8> {
    let source = std::fs::read_to_string(root().join("spec").join(APPLICATION)).expect("read");
    let program = assemble_program(&source).unwrap_or_else(|e| panic!("{APPLICATION}: {e:?}"));
    let (text, pending) =
        encode_object(&program).unwrap_or_else(|e| panic!("{APPLICATION}: {e:?}"));
    let bytes = object(
        &text,
        &program,
        &pending,
        None,
        &layout_addresses(&program, Target::Uncompressed),
    );
    let objects = [read(&bytes).expect("the object reads back")];
    let image = link_at(&objects, load).unwrap_or_else(|e| panic!("{APPLICATION}: {e:?}"));
    write_debuggable_at(&image.text, &image.data, &image.table, image.bss, &[], load)
}

/// **The acceptance for `F-001f`.** One call, and every clause the harness reads by hand
/// comes back in the [`Hosted`].
#[test]
fn one_call_hosts_the_application_and_says_what_happened() {
    let hosted = host(&elf_at(linked_at()[0]), RAM, BUDGET).expect("the image loads");
    let Hosted {
        ended,
        surface,
        uart,
        scause,
        entry,
    } = hosted;

    assert_eq!(
        ended,
        Ended::Exited { status: 0 },
        "it asked to be ended, and the status is call 1's own verdict (A3, A5)"
    );
    assert_eq!(
        surface,
        STRING.as_bytes(),
        "its bytes are on the surface it was handed, and this host chose the handle (A4)"
    );
    assert!(
        uart.is_empty(),
        "and nothing reached the machine's own device (A3, A4)"
    );
    assert_eq!(
        scause, 8,
        "the last trap was an environment call from U-mode, which S-mode cannot raise — \
         so the `sstatus.SPP` this host sets before loading really was cleared (A2)"
    );
    assert_eq!(entry, linked_at()[0], "entered at its own e_entry (A1)");
}

/// The load address is the caller's, not the program's: two ELFs, two entries, one string
/// each. A1 read out of the host rather than asserted about it.
#[test]
fn the_same_program_is_hosted_at_either_address() {
    for at in linked_at() {
        let hosted = host(&elf_at(at), RAM, BUDGET).expect("the image loads");
        assert_eq!(hosted.entry, at);
        assert_eq!(hosted.ended, Ended::Exited { status: 0 });
        assert_eq!(hosted.surface, STRING.as_bytes());
    }
}

/// A program linked into the supervisor's own gigabyte is **refused**, not quietly given
/// the supervisor's pages. The refusal names the address, because a host that says "could
/// not load" sends the reader to the ELF writer.
#[test]
fn a_program_linked_into_the_supervisors_gigabyte_is_refused() {
    let e = host(&elf_at(BASE + 0x2000), RAM, BUDGET).expect_err("this must not load");
    assert!(
        e.contains("superpage") && e.contains("may not be linked into it"),
        "the refusal has to name what is wrong; got {e:?}"
    );
}

/// A machine with no room for the address space says so before anything executes. The
/// pool ends where RAM does, so this is the loader running out of frames rather than a
/// program that wrote off the end of one.
#[test]
fn a_machine_too_small_to_hold_the_address_space_is_refused() {
    let e = host(&elf_at(linked_at()[0]), 0x1_1000, BUDGET).expect_err("this must not load");
    assert!(
        e.contains("out of frames"),
        "the refusal has to name what ran out; got {e:?}"
    );
}

/// A budget too short to reach the exit is [`Ended::OutOfBudget`] — not an exit status of
/// zero, and not a fault. The program is unfinished and the host says which.
#[test]
fn a_budget_too_short_ends_the_run_without_ending_the_program() {
    let hosted = host(&elf_at(linked_at()[0]), RAM, 3).expect("the image loads");
    assert_eq!(hosted.ended, Ended::OutOfBudget);
    assert!(
        hosted.surface.is_empty(),
        "it never reached call 1, so nothing is on the surface"
    );
    assert!(
        host::describe(&hosted.ended).contains("budget"),
        "and the words a person reads say so"
    );
}

/// The words are for a person, and each one names its number. Asserted here because the
/// browser will show these strings and a page that says "it faulted" with no `scause`
/// sends its reader to the wrong instruction.
#[test]
fn every_outcome_describes_itself_with_the_number_it_carries() {
    assert!(host::describe(&Ended::Exited { status: 0 }).contains("सिद्धम्"));
    assert!(host::describe(&Ended::Exited { status: 7 }).contains('7'));
    let faulted = host::describe(&Ended::Faulted {
        cause: 13,
        tval: 0x1234,
        epc: 0x2000_0010,
    });
    assert!(faulted.contains("13") && faulted.contains("0x1234") && faulted.contains("0x2000"));
}
