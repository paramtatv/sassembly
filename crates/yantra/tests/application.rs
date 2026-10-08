//! `F-001d` — what a Sassembly *application* is, and the measurement of every program in
//! this repository against it — and `F-001e3`, the one program that now passes.
//!
//! # Why this is a test and not only a document
//!
//! ADR-0015 defines an application in five clauses: entered by a loader (A1), never
//! leaving U-mode (A2), calling an application ABI rather than SBI (A3), writing to a
//! surface it was handed rather than to a device (A4), and exiting without taking the
//! machine with it (A5). A definition written only in prose drifts the moment somebody
//! adds a `.sas` file and calls it an app.
//!
//! So `spec/programs.tsv` carries the verdict for every program in `spec/`, and this file
//! re-derives every column of it from the source. The table cannot claim a program is
//! unprivileged when it writes `satp`, and **the word `app` costs a passing run**:
//! [`the_application_is_one_because_the_machine_ran_it_as_one`] loads every row that
//! claims it, in U-mode, through the real loader and the real supervisor.
//!
//! # What is derived, and from which oracle
//!
//! `privileged` comes from `spec/mnemonics-riscv64.src.tsv` — the registry that already
//! decides which Devanagari name is which RISC-V family — not from a list of words typed
//! here. If a name in that file changes, [`the_privileged_families_are_still_named`]
//! fails rather than this file silently deciding nothing is privileged.
//!
//! # The one clause a source cannot decide
//!
//! A2 and A4 are refusable on the face of the text: a program that writes `satp` is the
//! supervisor, and one that builds `0x10000000` with `lui` is talking to a device. **A3 is
//! not.** `spec/application-abi.tsv` says so in its own header — the number space is not
//! what separates this ABI from SBI, the *privilege* is, and `a7 = 0` means `sbi_set_timer`
//! from S-mode and `समापनम्` from U-mode. No amount of reading the source decides which,
//! because who answers the `ecall` is not written there.
//!
//! So the `clause` column for a boot proof is derived from its text, and the application's
//! is `-` — it fails none — with the claim carried instead by running it: the machine
//! reports `scause` 8, the supervisor answers both calls, the bytes land on the granted
//! surface, the UART stays empty, and the machine hosts it a second time.
//!
//! # The finding
//!
//! 48 programs, 47 boot proofs, **one application**. 18 of the proofs are the supervisor
//! themselves, 15 call SBI from S-mode, and 14 write a device address with `lui`. The count
//! was zero until `F-001e3`, and it could not have been anything else before it: the ABI
//! an application would call did not exist. See ADR-0015 for why "uses no privileged
//! instruction" — which 14 of these programs satisfy — is not the definition.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use sadhana::encode::{Target, encode_object, layout_addresses};
use sadhana::kosha::{object, write_debuggable_at};
use sadhana::parse::assemble_program;
use sadhana::samyojana::link_at;
use sadhana::vastu::read;
use yantra::host::{Hosted, host};
use yantra::loader::{STACK_TOP, load_application};
use yantra::supervisor::{Ended, Supervisor, Surface, install};
use yantra::{Csrs, Machine, Privilege};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/yantra has a grandparent")
        .to_path_buf()
}

/// The five families a program may not name and still be a guest of the machine:
/// the three CSR forms, `sfence.vma` and `sret`. `ecall` and `ebreak` are deliberately
/// absent — an application *does* make an environment call; A3 is about what answers it.
const PRIVILEGED_FAMILIES: [&str; 5] = ["csrrw", "csrrs", "csrrc", "sfence", "sret"];

/// `ecall`, by the name `spec/mnemonics-riscv64.src.tsv` gives it.
const ECALL: &str = "आज्ञापनम्";

/// `lui`. An upper immediate built from a hexadecimal literal is, in this tree, always a
/// machine address or a device's magic value.
const LUI: &str = "उपरिभारः";

/// The hexadecimal literal prefix — `०षोड्`, "zero sixteen".
const HEX: &str = "०षोड्";

/// `sp`, by the name `spec/registers-riscv64.tsv` gives it.
///
/// A boot proof sets its own stack pointer from a linker symbol it can see; A1's "a stack
/// pointer it did not choose" is the whole difference, so an application does not name the
/// register at all. Checked against the code lines only — this program's own header
/// explains that it never writes `स्तूपसूचकः`, and a comment is not an instruction.
const STACK_POINTER: &str = "स्तूपसूचकः";

/// The one program in `spec/` that is an application, and the file this test runs.
const APPLICATION: &str = "atithi.sas";

/// What it carries and puts on its surface. Not a fixture's string: the same octets are in
/// `spec/atithi.sas`, and [`the_application_carries_the_string_this_test_expects`] holds
/// the two together rather than letting a passing run mean the program wrote *something*.
const STRING: &str = "अतिथिः";

/// This file, by the name `spec/programs.tsv` records it under in the `checks` column.
const THIS_FILE: &str = "crates/yantra/tests/application.rs";

/// The Devanagari name of every privileged family, read from the registry.
fn privileged_names() -> BTreeSet<String> {
    let table = std::fs::read_to_string(root().join("spec/mnemonics-riscv64.src.tsv"))
        .expect("read spec/mnemonics-riscv64.src.tsv");
    let mut out = BTreeSet::new();
    for line in table.lines().filter(|l| !l.starts_with('#')) {
        let mut f = line.split('\t');
        let (Some(family), Some(name)) = (f.next(), f.next()) else {
            continue;
        };
        if PRIVILEGED_FAMILIES.contains(&family) {
            out.insert(name.to_string());
        }
    }
    out
}

/// A program's source with its comment lines removed. `॰` opens a comment, and a comment
/// that mentions `satp` must not make the program privileged.
fn code_lines(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter(|l| !l.trim_start().starts_with('॰'))
        .collect()
}

/// The head of a token, with any type or width suffix (`ॱअ८`) removed.
fn head(token: &str) -> &str {
    token.split('ॱ').next().unwrap_or(token)
}

/// What the source says, in the four columns `spec/programs.tsv` records.
struct Derived {
    privileged: bool,
    ecall: bool,
    address: bool,
}

impl Derived {
    fn of(source: &str, privileged_names: &BTreeSet<String>) -> Self {
        let lines = code_lines(source);
        let tokens: Vec<&str> = lines.iter().flat_map(|l| l.split_whitespace()).collect();
        Derived {
            privileged: tokens.iter().any(|t| privileged_names.contains(head(t))),
            ecall: tokens.iter().any(|t| head(t) == ECALL),
            address: lines
                .iter()
                .any(|l| l.trim_start().starts_with(LUI) && l.contains(HEX)),
        }
    }

    /// The **first** clause of ADR-0015 the program fails, in the order the clauses are
    /// written: a program that is the supervisor (A2) is not further asked what it calls.
    /// A5 decides nothing today only because every program fails an earlier clause; each
    /// of these also ends the machine or parks in `jal x0, .`.
    fn clause(&self) -> &'static str {
        if self.privileged {
            "A2"
        } else if self.address {
            "A4"
        } else if self.ecall {
            "A3"
        } else {
            "A1"
        }
    }

    fn yes(b: bool) -> &'static str {
        if b { "yes" } else { "no" }
    }
}

/// One row of `spec/programs.tsv`.
struct Row {
    class: String,
    clause: String,
    privileged: String,
    ecall: String,
    address: String,
    checks: String,
}

fn table() -> BTreeMap<String, Row> {
    let text = std::fs::read_to_string(root().join("spec/programs.tsv")).expect("read table");
    let mut out = BTreeMap::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split('\t').collect();
        if f[0] == "program" {
            continue;
        }
        assert_eq!(f.len(), 7, "spec/programs.tsv row has 7 columns: {line}");
        out.insert(
            f[0].to_string(),
            Row {
                class: f[1].to_string(),
                clause: f[2].to_string(),
                privileged: f[3].to_string(),
                ecall: f[4].to_string(),
                address: f[5].to_string(),
                checks: f[6].to_string(),
            },
        );
    }
    out
}

fn programs() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for entry in std::fs::read_dir(root().join("spec")).expect("read spec/") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "sas") {
            let name = path
                .file_name()
                .expect("named")
                .to_string_lossy()
                .into_owned();
            out.insert(name, std::fs::read_to_string(&path).expect("read program"));
        }
    }
    out
}

/// The registry still names all five families. Without this, a rename would make every
/// program look unprivileged and the whole classification would quietly become "A3".
#[test]
fn the_privileged_families_are_still_named() {
    assert_eq!(
        privileged_names().len(),
        PRIVILEGED_FAMILIES.len(),
        "spec/mnemonics-riscv64.src.tsv no longer names one of {PRIVILEGED_FAMILIES:?}"
    );
}

/// A program added to `spec/` is a program somebody must classify. This is the only guard
/// that notices one arriving.
#[test]
fn the_table_lists_exactly_the_programs_in_spec() {
    let on_disk: BTreeSet<String> = programs().into_keys().collect();
    let listed: BTreeSet<String> = table().into_keys().collect();
    assert_eq!(
        on_disk, listed,
        "spec/programs.tsv and spec/*.sas disagree — classify the new program per ADR-0015"
    );
}

/// Every column that can be read out of the source is read out of the source.
#[test]
fn every_derived_column_comes_from_the_source() {
    let names = privileged_names();
    let table = table();
    for (program, source) in programs() {
        let d = Derived::of(&source, &names);
        let row = table.get(&program).expect("listed");
        assert_eq!(
            row.privileged,
            Derived::yes(d.privileged),
            "{program}: privileged"
        );
        assert_eq!(row.ecall, Derived::yes(d.ecall), "{program}: ecall");
        assert_eq!(row.address, Derived::yes(d.address), "{program}: address");
        if row.class == "app" {
            // The two clauses a source CAN refuse, refused here rather than deferred to
            // the run: a program that writes `satp` or builds a device address is not an
            // application whatever it does afterwards, and saying so from the text is
            // cheaper and sharper than watching it succeed for the wrong reason.
            assert_eq!(
                row.clause, "-",
                "{program}: an application fails no clause, and A3 is not the source's to \
                 decide — see this file's header"
            );
            assert!(
                !d.privileged,
                "{program}: an application is not the supervisor (A2)"
            );
            assert!(!d.address, "{program}: an application names no device (A4)");
            assert!(
                d.ecall,
                "{program}: an application's only call outward is `ecall`, and A5's exit \
                 is one of them — a program that makes none cannot end"
            );
            assert!(
                !code_lines(&source)
                    .iter()
                    .any(|l| l.contains(STACK_POINTER)),
                "{program}: it sets its own `sp`, so it chose the stack it stands on (A1)"
            );
        } else {
            assert_eq!(row.clause, d.clause(), "{program}: clause");
        }
    }
}

/// Everything that asserts about a program, and the name the `checks` column records it
/// by: every `tools/check-*.sh`, and **this file**.
///
/// `F-001d` raised the claim that every Sassembly program in this repository exists to be
/// asserted about by a `check-*.sh`, which is what made all 36 of them proofs rather than
/// applications. That is still true of the proofs and it is the wrong shape for an
/// application: nothing about `atithi.sas` can be checked by booting it under QEMU,
/// because it is not booted at all — it is *loaded*, and the thing that loads it is
/// [`the_application_is_one_because_the_machine_ran_it_as_one`], below. So the set of
/// assertions grows by one file rather than the column growing an exception, and the claim
/// the column carries becomes the honest one: **somebody checks this**.
/// What this can discover, and what it therefore cannot promise.
///
/// **Every `tools/check-*.sh`, plus `THIS_FILE`, and nothing else.** A Rust test
/// that asserts about a program is invisible here unless it is that one
/// hardcoded path — so the `checks` column means "asserted about by a shell
/// check", not "asserted about by something". `B-058b2b7` found the gap the
/// honest way: `crates/sadhana/tests/trap_vector_alignment.rs` asserts a
/// property of **fourteen** programs and appears in the column for none, with
/// the gate green throughout.
///
/// Widening this to `crates/*/tests/*.rs` was measured before being rejected,
/// and the measurement is why it is not done here: it would add 70
/// (program, test) pairs across 16 programs, and **16 of those 70 are
/// comment-only** — the file names the program in prose and never touches it in
/// code. Matching by `contains` over a Rust test's doc comments records a
/// mention as an assertion, which is the drift this registry exists to prevent,
/// introduced by the fix for it. Widening needs a way to tell a mention from a
/// use first; that is its own row, not a scanner tweak.
fn assertions() -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = std::fs::read_dir(root().join("tools"))
        .expect("read tools/")
        .filter_map(|e| {
            let p = e.expect("entry").path();
            let name = p.file_name()?.to_string_lossy().into_owned();
            (name.starts_with("check-") && name.ends_with(".sh")).then_some((name, p))
        })
        .collect();
    out.push((THIS_FILE.to_string(), root().join(THIS_FILE)));
    out.sort();
    out
}

/// The `checks` column, re-derived.
#[test]
fn every_program_is_asserted_about_by_something() {
    let mut referenced: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let names: Vec<String> = programs().into_keys().collect();
    for (label, path) in assertions() {
        let text = std::fs::read_to_string(&path).expect("read the asserting file");
        for program in &names {
            if text.contains(program.as_str()) {
                referenced
                    .entry(program.clone())
                    .or_default()
                    .insert(label.clone());
            }
        }
    }

    let table = table();
    for program in &names {
        let got: Vec<String> = referenced
            .get(program)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        assert!(
            !got.is_empty(),
            "{program} is asserted about by no tools/check-*.sh and not by {THIS_FILE} — \
             it is neither a proof nor, on its own, an application (ADR-0015)"
        );
        assert_eq!(
            table.get(program).expect("listed").checks,
            got.join(","),
            "{program}: checks column"
        );
    }
}

/// The finding, held where it cannot rot: **one application, 36 boot proofs.**
///
/// The count was zero for the whole life of `F-001d`'s measurement, and moving it was the
/// entire content of `F-001e`. It is pinned rather than merely non-empty because the
/// failure this guards against is not "somebody wrote `app`" — the run below answers that
/// — but "somebody wrote it *twice*", one of them without noticing what it costs.
#[test]
fn exactly_one_program_is_an_application() {
    let table = table();
    let apps: Vec<&String> = table
        .iter()
        .filter(|(_, r)| r.class == "app")
        .map(|(p, _)| p)
        .collect();
    assert_eq!(
        apps,
        vec![&APPLICATION.to_string()],
        "spec/programs.tsv's applications. ADR-0015 A1-A5; a new one is a new run below."
    );
    assert!(
        table
            .values()
            .all(|r| r.class == "app" || r.class == "boot-proof"),
        "a program is a boot proof or an application; there is no third class"
    );
    // 37 -> 40 on 2026-08-27: `virtio-blk`, `virtio-console` and `virtio-net`
    // returned to `spec/`. They had been in `spec/` but in NEITHER
    // `programs.tsv` NOR the parse-shape oracle, which is why moving them to
    // `drafts/` briefly made this pass — the violation was pre-existing and the
    // move masked it. They are classified now, and all three PASS their QEMU
    // acceptance checks.
    assert_eq!(table.len(), 48, "the count of programs moved");

    let by_clause = |c: &str| table.values().filter(|r| r.clause == c).count();
    // A2 18 -> 19 and A4 14 -> 13 on 2026-10-06: the virtio-sound driver program gained a play-out
    // wait that reads the `time` CSR (a Zicsr family), so it is privileged and its first
    // failed clause is A2, by the derived-column rule above (agent/0c-dg-ubuntu).
    assert_eq!(by_clause("A2"), 19, "programs that ARE the supervisor");
    assert_eq!(by_clause("A3"), 15, "programs that call SBI from S-mode");
    assert_eq!(by_clause("A4"), 13, "programs that write a device address");
    assert_eq!(by_clause("-"), 1, "programs that fail no clause");
}

/// Every clause the table names is a clause the ADR defines. This is the join that keeps
/// the table and the definition from drifting apart in either direction.
#[test]
#[ignore = "census: needs docs/adr (the design records) not in the public repository"]
fn every_clause_the_table_names_is_defined_by_the_adr() {
    let adr = std::fs::read_to_string(
        root().join("docs/adr/0015-an-application-is-a-u-mode-program-with-an-environment.md"),
    )
    .expect("read ADR-0015");
    for clause in table()
        .values()
        .map(|r| r.clause.clone())
        .filter(|c| c != "-")
        .collect::<BTreeSet<_>>()
    {
        assert!(
            adr.contains(&format!("**{clause} —")),
            "spec/programs.tsv names {clause}, which ADR-0015 does not define"
        );
    }
}

// ---------------------------------------------------------------------------------------
// `F-001e3` — the word `app` costs a run.

/// Physical RAM's base, which is also the supervisor's own address and its `stvec`.
const BASE: u64 = 0x8000_0000;
/// 4 MiB. The loader takes about a dozen frames and the rest is slack.
const RAM: usize = 1 << 22;
/// Where the loader may start taking frames; everything below belongs to the supervisor.
const FREE: u64 = BASE + 0x1_0000;
/// The handle granted. Its value is the supervisor's to choose, and the program does not
/// know it — it reads it out of the vector, which is the whole of A4.
const SCREEN: u64 = 7;
/// Ten instructions and two calls. Anything longer is a program that did not end.
const BUDGET: u64 = 200;

/// Two addresses to link at, neither of which the program is told, and neither of which is
/// the supervisor's gigabyte.
///
/// A1 says an application "does not know its own load address". A program that were told
/// would have to be built twice to run twice; this one is built twice and the **octets are
/// the same both times**, because every address in it is reached from `pc`.
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

/// Assemble and link `source` at `load`; return the ELF and the text it holds.
///
/// The same four calls `sadhana`'s own `main` makes, in the same order — so what runs
/// below is what the toolchain would have written to a file, not a fixture this test
/// invents.
fn build(source: &str, load: u64) -> (Vec<u8>, Vec<u8>) {
    let program = assemble_program(source).unwrap_or_else(|e| panic!("{APPLICATION}: {e:?}"));
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
    let elf = write_debuggable_at(&image.text, &image.data, &image.table, image.bss, &[], load);
    (elf, image.text)
}

/// A machine with the supervisor's one instruction installed and `SPP` **set**, so a loader
/// that forgets to clear it returns to S-mode and this test says so instead of passing.
fn machine() -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX, // W-363: no store bound beyond `mem` — this machine has no injected input above it
        // Added with the `patra` file window: a machine that was never asked
        // to serve files must not be able to.
        patra_root: None,
        patra_path: None,
        patra_buffer: None,
        virtio: Default::default(),
        x: [0; 32],
        f: [0; 32],
        fcsr: 0,
        pc: 0,
        base: BASE,
        mem: vec![0; RAM],
        reservation: None,
        csr: Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
        socket: None,
    };
    m.csr.sstatus = 1 << 8;
    install(&mut m, BASE).expect("the supervisor's word is inside RAM");
    m
}

/// The string on the surface is the string in the source. Without this, a run that wrote
/// *something* would pass, and the acceptance would be that call 1 moves bytes rather than
/// that this program moves its own.
#[test]
fn the_application_carries_the_string_this_test_expects() {
    let source = std::fs::read_to_string(root().join("spec").join(APPLICATION)).expect("read");
    assert!(
        code_lines(&source)
            .iter()
            .any(|l| l.contains(&format!("उक्तम् {STRING} इति"))),
        "spec/{APPLICATION} no longer carries {STRING}"
    );
}

/// **The acceptance for `F-001e3`.** `spec/programs.tsv` says `app`; this is what the word
/// costs.
///
/// Every clause, and what each one is read from:
///
/// - **A1** — the loader placed it: `Loaded::entry` is the ELF's `e_entry` and `sp` is
///   [`STACK_TOP`], which the loader chose and no line of the program mentions. It is
///   linked at two addresses out of one set of octets, so it does not know either.
/// - **A2** — `scause` is 8 at the end, and 8 is ENVIRONMENT CALL FROM **U-MODE**. An
///   S-mode `ecall` raises 9; the number cannot be forged from the wrong privilege, which
///   is the same evidence `tools/check-user-mode.sh` treats as decisive.
/// - **A3** — the supervisor answered, the firmware did not. [`Ended::Stopped`] is what an
///   SBI call from this program would have produced, and the UART is empty besides.
/// - **A4** — the bytes are on the surface granted by handle, and the machine's own device
///   received nothing.
/// - **A5** — the second iteration of the loop is a second program on the same machine,
///   in the same RAM, from the same frame pool. If call 0 had stopped anything, there
///   would be no second run to read.
#[test]
fn the_application_is_one_because_the_machine_ran_it_as_one() {
    let source = std::fs::read_to_string(root().join("spec").join(APPLICATION)).expect("read");
    let linked_at_addrs = linked_at();
    let built: Vec<(Vec<u8>, Vec<u8>)> = linked_at_addrs
        .iter()
        .map(|&at| build(&source, at))
        .collect();
    // THE ENCODER'S INVARIANT, NOT A1 — `W-108`.
    //
    // This asserted A1's "does not know its own load address" until it was measured and
    // could not fail: `ॱउपरि`/`ॱअधः` measure from the instruction carrying them (`B-064`),
    // so a program's text is invariant under placement **by construction of the encoder**,
    // which `sadhana/tests/load_address.rs::moving_an_image_changes_no_instruction` already
    // asserts under its true name. Substituting a `उपरिभारः` mutant that hardcodes
    // `0x2000_0000` for `spec/atithi.sas` left this line PASSING, and a sweep of every
    // program in `spec/` that links standalone found 36 of 36 byte-identical at both
    // addresses — so on this corpus it cannot fail.
    //
    // It is blind to an absolute address written as a **literal**, because a literal is not
    // a relocation and nothing patches it. A tracked program in `spec/` does exactly that
    // — a `उपरिभारः` of the finisher address — which is the mechanism on real source rather
    // than only on a mutant. `W-108` names it; this file does not, because naming it here
    // would make `every_program_is_asserted_about_by_something` record a MENTION as an
    // assertion, which is the drift that registry exists to prevent (`B-058b2b7`).
    //
    // It stays rather than being deleted as a duplicate: `load_address.rs` asserts this of a
    // DIFFERENT program, so the two are near-duplicates, not duplicates, and dropping this
    // one would remove the invariant's coverage on the only application in the tree. (Which
    // program is in `W-108`; naming it here would register a mention as an assertion.)
    // A1 itself is carried by [`a_program_does_not_know_where_it_was_put`], which can fail
    // on input.
    assert_eq!(
        built[0].1,
        built[1].1,
        "the same octets at {:#x} and {:#x} — the ENCODER emits position-independent text \
         (`B-064`). This is not A1: a program hardcoding an absolute address as a literal \
         passes it. See `a_program_does_not_know_where_it_was_put`.",
        linked_at()[0],
        linked_at()[1]
    );

    let mut m = machine();
    let mut sup = Supervisor::new(vec![Surface::writable(SCREEN)]);
    let mut uart: Vec<u8> = Vec::new();

    let linked_at_addrs = linked_at();
    for (at, (elf, _)) in linked_at_addrs.iter().zip(&built) {
        let loaded = load_application(&mut m, elf, FREE, &sup.handles())
            .unwrap_or_else(|e| panic!("{APPLICATION} at {at:#x}: {e}"));
        assert_eq!(loaded.entry, *at, "entered at its own e_entry (A1)");
        assert_eq!(loaded.sp, STACK_TOP, "on a stack it did not choose (A1)");

        let ended = sup.run(&mut m, BUDGET, &mut uart);
        assert_eq!(
            ended,
            Ended::Exited { status: 0 },
            "it asked to be ended, and the status is सिद्धम् because the program passes \
             call 1's own verdict to call 0 rather than inventing a zero (A3, A5)"
        );
        assert_eq!(
            m.csr.scause, 8,
            "the last trap was an environment call from U-mode, which S-mode cannot \
             raise (A2)"
        );
        assert_eq!(
            m.x[11],
            STRING.len() as u64,
            "and `a1` still holds the count call 1 returned"
        );
    }

    assert_eq!(
        sup.surface(SCREEN).expect("granted").bytes,
        STRING.repeat(linked_at().len()).as_bytes(),
        "both runs put their own bytes on the surface they were handed (A4)"
    );
    assert!(
        uart.is_empty(),
        "and neither reached the machine's own device (A3, A4)"
    );
}

/// The four fields of a [`Hosted`] that describe what the PROGRAM did.
///
/// `entry` is excluded because it is the address the loader entered, which differs by
/// design — that is what linking at two addresses means. `Ended::Faulted` carries `epc`,
/// a program counter, for the same reason; it is zeroed rather than dropped so that a
/// fault's `cause` and `tval`, which are address-independent, still compare.
///
/// **Both exclusions were learned the hard way**, in the harness this test is derived
/// from: comparing whole `Hosted` values reported two *correct* programs as A1 violations
/// while printing two identical lines, because the renderer did not show the field that
/// differed; and leaving `epc` in reported a device write — an **A4** violation — as
/// "differs by load address", which is A1's message for A4's defect.
fn what_the_program_did(h: &Hosted) -> (Ended, Vec<u8>, usize, u64) {
    let ended = match &h.ended {
        Ended::Faulted { cause, tval, .. } => Ended::Faulted {
            cause: *cause,
            tval: *tval,
            epc: 0,
        },
        other => other.clone(),
    };
    (ended, h.surface.clone(), h.uart.len(), h.scause)
}

/// **A1, carried by an assertion that can fail on input** — task `W-108`.
///
/// The byte-equality check above wore A1's label and could not fail: the encoder emits
/// position-independent text by construction, so it passes for a program that hardcodes
/// an absolute address as a literal. This is the check that does not.
///
/// # Why this is a separate test rather than a comparison inside the loop above
///
/// **A5 wants the same machine; A1 wants independent runs.** That loop creates one
/// [`Machine`] *before* it and reuses it, deliberately — a second program on the same
/// machine is how A5 is asserted. Comparing per-run observables there would confound *the
/// load address changed* with *the previous run left state behind*, which is one
/// observation with two causes: the defect `W-108` exists to remove, reintroduced by its
/// own fix. [`host`] builds a fresh machine per call, so here the only difference between
/// the two runs is the address.
///
/// It also gives [`host`] a second caller inside the crate the gate tests, which is what
/// its own module doc asks for: *"the only thing in this tree that could host an
/// application was a test"*.
#[test]
fn a_program_does_not_know_where_it_was_put() {
    let source = std::fs::read_to_string(root().join("spec").join(APPLICATION)).expect("read");
    let linked_at_addrs = linked_at();
    let runs: Vec<Hosted> = linked_at_addrs
        .iter()
        .map(|&at| {
            let (elf, _) = build(&source, at);
            host(&elf, RAM, BUDGET).unwrap_or_else(|e| panic!("{APPLICATION} at {at:#x}: {e}"))
        })
        .collect();

    // The guard on the guard. If the loader entered the same address twice, the comparison
    // below would pass for a program that hardcodes its address — it would be testing
    // nothing, which is the fault this whole row is about.
    assert_ne!(
        runs[0].entry, runs[1].entry,
        "both links entered {:#x}, so this test would assert nothing",
        runs[0].entry
    );

    assert_eq!(
        what_the_program_did(&runs[0]),
        what_the_program_did(&runs[1]),
        "{APPLICATION} behaved differently at {:#x} and {:#x} — it knows where it was put \
         (A1). A program that builds an address with `उपरिभारः` and a literal, rather than \
         pc-relatively, fails here and passes the byte-equality check above.",
        linked_at()[0],
        linked_at()[1]
    );
}
