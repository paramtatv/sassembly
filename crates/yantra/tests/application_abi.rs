//! `F-001e1` — the Sassembly application ABI, and the one property that makes it
//! hostable: nothing in its number space escapes U-mode to the firmware.
//!
//! # Two columns
//!
//! 1. **The tables are held against the registries they claim to be made of.**
//!    `spec/application-entry.tsv` names 32 registers; every name and number comes from
//!    `spec/registers-riscv64.tsv` rather than being typed a second time here. Every
//!    error name in `spec/application-abi.tsv` is held against `spec/lexicon.tsv`,
//!    because ADR-0016's claim that four of the five already existed with the right
//!    sense is a claim about that file and not a nice sentence.
//!
//! 2. **The machine is asked the sharp question.** ADR-0016 puts the application call
//!    number in `a7` — SBI's register — with numbers chosen without regard for SBI's, on
//!    the grounds that the *privilege* separates the two interfaces and the number never
//!    could. That is falsifiable in one line:
//!    [`sbi_shutdowns_own_number_does_not_stop_the_machine_from_user_mode`] puts 8 in
//!    `a7`, which from S-mode stops this machine dead, and executes it in U-mode.
//!
//! # What is NOT asserted here
//!
//! Nobody answers call 0 or call 1. `yantra` is the firmware, not the supervisor, and the
//! supervisor that will answer these is `F-001e2`. What these tests establish is the
//! precondition for it: every number in the table, from U-mode, is exception 8 delivered
//! to `stvec`, so the supervisor gets all of them and the firmware gets none.

use std::path::{Path, PathBuf};
use yantra::{Csrs, Halt, Machine, Privilege};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/yantra has a grandparent")
        .to_path_buf()
}

/// The rows of a `spec/` table, comments and blank lines dropped, header dropped, each
/// row split on tabs.
fn rows(name: &str) -> Vec<Vec<String>> {
    let text = std::fs::read_to_string(root().join("spec").join(name))
        .unwrap_or_else(|e| panic!("spec/{name}: {e}"));
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .skip(1)
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect()
}

const BASE: u64 = 0x8000_0000;
const RAM: usize = 1 << 20;
const HANDLER: u64 = BASE + 0x800;
const ECALL: u32 = 0x0000_0073;
/// `sstatus.SPP`, bit 8.
const SPP: u64 = 1 << 8;
/// `scause` 8 — environment call from U-mode.
const U_ECALL: u64 = 8;
/// SBI v0.1 `sbi_shutdown`. From S-mode this machine stops on it.
const SBI_SHUTDOWN: u64 = 8;

/// A machine holding one `ecall` at the entry point, with `a7` set and a handler
/// installed, in `mode`.
fn calling(a7: u64, mode: Privilege) -> Machine {
    let mut m = Machine {
        x: [0; 32],
        pc: BASE,
        base: BASE,
        mem: vec![0; RAM],
        reservation: None,
        csr: Csrs::default(),
        mode,
        time: 0,
        timecmp: None,
    };
    m.mem[0..4].copy_from_slice(&ECALL.to_le_bytes());
    m.csr.stvec = HANDLER;
    // The last trap came from S-mode. A U-mode `ecall` must clear this, and starting from
    // a set bit is what makes the clearing visible.
    m.csr.sstatus = SPP;
    m.x[17] = a7;
    m
}

// ---------------------------------------------------------------------------------------
// Column 1: the tables are made of the registries they name.

#[test]
fn the_entry_contract_names_every_integer_register_by_its_registered_number() {
    // A1's contract is the whole register file, and the names and numbers in it are
    // `spec/registers-riscv64.tsv`'s. If a register is renamed there, this fails rather
    // than the entry contract quietly describing a register that no longer exists.
    let registry: Vec<(String, String)> = rows("registers-riscv64.tsv")
        .into_iter()
        .filter(|r| r[3] == "int")
        .map(|r| (r[1].clone(), r[2].clone()))
        .collect();
    assert_eq!(registry.len(), 32, "32 integer registers in the registry");

    let entry = rows("application-entry.tsv");
    assert_eq!(
        entry.len(),
        32,
        "the entry contract states all 32, or it is not a contract"
    );
    for row in &entry {
        let (abi, number) = (&row[0], &row[1]);
        let found = registry
            .iter()
            .find(|(name, _)| name == abi)
            .unwrap_or_else(|| panic!("{abi} is not a register in spec/registers-riscv64.tsv"));
        assert_eq!(
            &found.1, number,
            "{abi} is x{} in the registry, not x{number}",
            found.1
        );
    }
}

#[test]
fn exactly_four_registers_carry_a_value_at_entry_and_ra_is_not_one_of_them() {
    // ADR-0016: 28 registers are stated to be zero, and `ra` is one of them — there is
    // nowhere to return to, and an application ends by CALLING call 0. A contract that
    // handed the program a return address would be offering it a second way out, and the
    // second way is the one nobody tests.
    let entry = rows("application-entry.tsv");
    let carried: Vec<(String, String)> = entry
        .iter()
        .filter(|r| r[2] != "0")
        .map(|r| (r[0].clone(), r[2].clone()))
        .collect();
    assert_eq!(
        carried,
        vec![
            ("sp".to_string(), "stack-top".to_string()),
            ("a0".to_string(), "handle-count".to_string()),
            ("a1".to_string(), "handle-vector".to_string()),
            ("a2".to_string(), "abi-version".to_string()),
        ],
        "four values at entry, in register order"
    );
    let ra = entry.iter().find(|r| r[0] == "ra").expect("ra is stated");
    assert_eq!(ra[2], "0", "ra is zero: returning from e_entry is a fault");
}

#[test]
fn every_call_number_is_unique_and_inside_the_space_this_file_may_assign() {
    // ADR-0016 reserves 2..=63 for this table and refuses 64 and above. A row outside
    // that range would be promising a per-module call space nothing implements.
    let mut seen: Vec<u64> = Vec::new();
    for row in rows("application-abi.tsv") {
        let n: u64 = row[0].parse().expect("call number");
        assert!(n < 64, "{n} is outside the space this table may assign");
        assert!(!seen.contains(&n), "call number {n} is assigned twice");
        seen.push(n);
        let arity: usize = row[3].parse().expect("arity");
        assert!(arity <= 6, "call {n} takes {arity} arguments; a0..a5 is 6");
        let args: Vec<&str> = if arity == 0 {
            Vec::new()
        } else {
            row[4].split(',').collect()
        };
        assert_eq!(args.len(), arity, "call {n}: arity disagrees with args");
        for (i, arg) in args.iter().enumerate() {
            let want = format!("a{i}=");
            assert!(
                arg.starts_with(&want),
                "call {n} argument {i} is {arg}, and arguments are a0..a5 in order"
            );
        }
    }
    assert_eq!(seen, vec![0, 1], "two calls, and ADR-0016 says why");
}

#[test]
fn every_error_the_abi_returns_is_a_word_the_lexicon_carries() {
    // ADR-0016's claim is that this ABI is made of vocabulary this system already had —
    // `अधिकाराभावः` already meant what a denied handle means. That is a claim about
    // `spec/lexicon.tsv`, so it is checked against it.
    let lexicon: Vec<String> = rows("lexicon.tsv")
        .into_iter()
        .map(|r| r[0].clone())
        .collect();
    let mut checked = 0;
    for row in rows("application-abi.tsv") {
        for name in row[6].split(',').filter(|s| *s != "-") {
            assert!(
                lexicon.contains(&name.to_string()),
                "error {name} is not a word in spec/lexicon.tsv"
            );
            checked += 1;
        }
        assert!(
            lexicon.contains(&row[1]),
            "call name {} is not a word in spec/lexicon.tsv",
            row[1]
        );
    }
    assert!(
        checked >= 4,
        "the failures are named, not left to a comment"
    );
}

// ---------------------------------------------------------------------------------------
// Column 2: the machine, asked the sharp question.

#[test]
fn sbi_shutdowns_own_number_does_not_stop_the_machine_from_user_mode() {
    // THE claim of ADR-0016. `a7 = 8` is `sbi_shutdown`; from S-mode this machine stops
    // on it, which is the control below. From U-mode the identical instruction with the
    // identical register is exception 8 delivered to `stvec`, and the machine runs on.
    // If this ever fails, an application can end the system it is a guest of, and the
    // decision to share SBI's register was wrong.
    let mut m = calling(SBI_SHUTDOWN, Privilege::User);
    assert_eq!(
        m.step(&mut Vec::new()),
        None,
        "the machine survives its guest asking to shut it down"
    );
    assert_eq!(m.csr.scause, U_ECALL, "cause 8, which S-mode cannot forge");
    assert_eq!(m.pc, HANDLER, "the supervisor decides what 8 means");
    assert_eq!(m.csr.sstatus & SPP, 0, "SPP says it came from U-mode");
}

#[test]
fn the_same_number_from_supervisor_mode_does_stop_it() {
    // The control, and the reason the test above is worth anything: 8 really is
    // `sbi_shutdown` on this machine, so U-mode's refusal to honour it is the privilege
    // and not an unimplemented number.
    let mut m = calling(SBI_SHUTDOWN, Privilege::Supervisor);
    assert_eq!(
        m.step(&mut Vec::new()),
        Some(Halt::Shutdown { pc: BASE }),
        "from S-mode, 8 is the firmware's shutdown"
    );
}

#[test]
fn no_call_in_the_abi_reaches_the_firmware_from_user_mode() {
    // The precondition for `F-001e2`: the supervisor gets every call in the table and the
    // firmware gets none of them. Both numbers collide with an SBI v0.1 call — 0 is
    // `sbi_set_timer`, 1 is `sbi_console_putchar` — so a machine that read `a7` before it
    // read the privilege would arm a timer or print a byte here.
    for row in rows("application-abi.tsv") {
        let n: u64 = row[0].parse().expect("call number");
        let mut m = calling(n, Privilege::User);
        m.x[10] = u64::from(b'x'); // a0: a plausible byte, were this sbi_console_putchar
        let mut out: Vec<u8> = Vec::new();
        assert_eq!(m.step(&mut out), None, "call {n} does not stop the machine");
        assert!(out.is_empty(), "call {n} did not reach the console");
        assert_eq!(m.csr.scause, U_ECALL, "call {n} is exception 8");
        assert_eq!(m.csr.sepc, BASE, "sepc is the ecall itself");
        assert_eq!(m.pc, HANDLER, "call {n} is the supervisor's to answer");
        assert_eq!(m.timecmp, None, "call {n} armed no timer");
    }
}

#[test]
fn returning_from_e_entry_is_a_fault_and_not_an_exit() {
    // `ra = 0` is A5 written into a register. A program that ends with `jalr x0, 0(ra)`
    // jumps to 0, which no application's address space maps, and the supervisor is told
    // — rather than the program leaving by a path the loader happened to leave open.
    // Without translation the machine reports it as a bad fetch; either way it is
    // diagnosable, which is the whole point of the zero.
    let mut m = calling(0, Privilege::User);
    m.mem[0..4].copy_from_slice(&0x0000_8067u32.to_le_bytes()); // jalr x0, 0(ra)
    m.x[1] = 0; // ra, exactly as the entry contract leaves it
    assert_eq!(m.step(&mut Vec::new()), None, "the jump itself succeeds");
    assert_eq!(m.pc, 0, "and lands on 0, which is the whole trick");
    let halt = m.step(&mut Vec::new());
    assert!(
        halt.is_some() || m.csr.scause != 0,
        "the fetch at 0 is reported, never a silent success"
    );
    assert_ne!(
        halt,
        Some(Halt::Shutdown { pc: 0 }),
        "and it is never a shutdown"
    );
}
