//! `F-001c1` — `SYSTEM`: what an `ecall` means when the interpreter IS the firmware.
//!
//! # Three columns, and why it takes three
//!
//! 1. **The encoding table**, `spec/encodings-riscv64.tsv`, extracted from
//!    `riscv64-elf-as` rather than transcribed — the same oracle `F-001b` used, for the
//!    same reason. It holds all ten 32-bit `SYSTEM` rows and it is where the *bits* come
//!    from. It knows nothing about meaning: it cannot tell you that `sret` returns from a
//!    trap, only that it is `0x10200073`.
//! 2. **The meaning, written from the mnemonic by hand** in this file. Two origins, so
//!    the pair is an oracle and not a mirror.
//! 3. **A whole program**, `spec/boot-sbi.sas`, assembled here by the project's own
//!    toolchain and run to its shutdown. The first two columns say the instructions are
//!    right; only the third says a *program* is.
//!
//! # Both directions, which is most of the value
//!
//! `SYSTEM` is the family where a catch-all arm is most tempting, because six of its ten
//! rows are CSR instructions that differ in three bits. So this file asserts what must
//! **not** execute as loudly as what must: `sret` and `sfence.vma` are real rows of the
//! same table and must still stop [`Halt::Unimplemented`], every CSR access must stop
//! [`Halt::Csr`] naming the register, and an unimplemented SBI extension must stop
//! [`Halt::Sbi`] naming the extension. A decoder that grew a `_ => {}` arm passes the
//! program at the bottom of this file and fails here.
//!
//! `c.ebreak` is the eleventh `SYSTEM` row and is 16 bits. Compressed encodings are not
//! decoded at all, so it is skipped here rather than asserted about — the same line
//! `atomics.rs` draws.

use sadhana::encode::{Target, encode_object, layout_addresses};
use sadhana::kosha::{object, write_debuggable_at};
use sadhana::parse::assemble_program;
use sadhana::samyojana::link_at;
use sadhana::vastu::read;
use yantra::{Halt, Machine, Privilege};

const BASE: u64 = 0x8000_0000;
/// Where OpenSBI hands control to an S-mode payload, and where `boot-sbi` is built.
const SBI_PAYLOAD: u64 = 0x8020_0000;

/// A machine with one instruction at the entry point and nothing else.
fn machine(word: u32) -> Machine {
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
        pc: BASE,
        base: BASE,
        mem: vec![0; 1 << 16],
        reservation: None,
        csr: yantra::Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
        socket: None,
    };
    m.mem[0..4].copy_from_slice(&word.to_le_bytes());
    m
}

// ---------------------------------------------------------------------------------------
// Column 1: the table.

/// The 32-bit `SYSTEM` rows of `spec/encodings-riscv64.tsv`, as `(mnemonic, pattern)`.
fn system_rows() -> Vec<(String, u32)> {
    // `spec/` is tracked, unlike `research/specs/`, so this file is always present and a
    // missing-file path would be dead code pretending to be care.
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/encodings-riscv64.tsv");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut rows = Vec::new();
    for line in text.lines().skip_while(|l| l.starts_with('#')).skip(1) {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 8 || f[5] != "32" {
            continue; // the compressed encodings are 16 bits and are not decoded
        }
        let pattern = u32::from_str_radix(f[3].trim_start_matches("0x"), 16).expect("pattern");
        if pattern & 0x7f == 0x73 {
            rows.push((f[0].to_string(), pattern));
        }
    }
    rows.sort();
    rows
}

#[test]
fn the_table_holds_the_family_this_file_claims_to_cover() {
    let names: Vec<String> = system_rows().into_iter().map(|(n, _)| n).collect();
    assert_eq!(
        names,
        [
            "csrrc",
            "csrrci",
            "csrrs",
            "csrrsi",
            "csrrw",
            "csrrwi",
            "ebreak",
            "ecall",
            "sfence.vma",
            "sret"
        ],
        "the ten 32-bit SYSTEM rows. If this fails the table grew a row and the arms \
         below have not been told about it — which is the point of asserting the set \
         rather than iterating whatever is there."
    );
}

// ---------------------------------------------------------------------------------------
// Column 2: the meaning, against every row.

/// A CSR that is still refused, for the rows below that must halt.
///
/// It was `stvec` until `F-001c2a` gave `stvec` real state, then `satp` until `F-001c2b1`
/// built the walker behind it, then `sie` until `F-001c2b3` built the interrupt. **The
/// measured list is now empty — all ten of the CSRs `spec/` names are implemented — so
/// this is `mstatus`, which no program in the tree names and this machine could not
/// honour if one did**: there is no machine mode here, because the interpreter IS the
/// firmware.
///
/// The example moved four times and never by weakening: each time, the register left this
/// constant because a row built the mechanism behind it. That it now has to leave the
/// measured list to find a refusal at all is the shape of the list being finished.
const REFUSED: u32 = 0x300;

#[test]
fn every_row_of_the_table_does_what_its_name_says() {
    let mut seen = 0;
    for (insn, pattern) in system_rows() {
        // Fields the pattern leaves free, filled the same way for every CSR row: rd = x5,
        // rs1 (or the uimm in the same five bits) = 6, and the CSR number. `ecall`,
        // `ebreak` and `sret` are masked `0xffffffff` — every other bit must be zero, so
        // their pattern IS their word.
        let csr_word = pattern | REFUSED << 20 | 5 << 7 | 6 << 15;
        let (word, expected) = match insn.as_str() {
            // With `a7` zero this is SBI extension 0 — `sbi_set_timer` — and with `a0`
            // zero it arms a deadline that is already past. It halted here until
            // `F-001c2b3`, naming the extension rather than returning; now it returns,
            // and `None` is the claim that it did. That the machine keeps running is the
            // whole of what this row asserts: what the armed deadline then does is
            // `tests/interrupts.rs`'s subject. `tests/system.rs` still holds the refusal
            // shape for an EID this machine does not implement, two tests below.
            "ecall" => (pattern, None),
            "ebreak" => (pattern, Some(Halt::Breakpoint { pc: BASE })),
            // csrrw and csrrwi always write; csrrs/csrrc set or clear the bits their
            // operand names, and the operand here is 6, which names some.
            "csrrw" | "csrrwi" | "csrrs" | "csrrsi" | "csrrc" | "csrrci" => (
                csr_word,
                Some(Halt::Csr {
                    pc: BASE,
                    csr: REFUSED as u16,
                    write: true,
                }),
            ),
            // `sret` executes as of `F-001c2a`, and as of `F-001c2b2` it executes from
            // here too. `sstatus` is zero on a fresh machine, so `SPP` says the trap came
            // from user mode — and that is now where it returns to, at `sepc`, which is
            // also zero. `None` is the assertion: the machine is still running, in U-mode,
            // about to fetch from address 0. What happens next is `tests/user.rs`'s
            // subject, not this file's; here the claim is that the row decoded and took
            // effect rather than halting.
            "sret" => (pattern, None),
            // `sfence.vma` executes as of `F-001c2b1`, and it is the one row here that
            // runs to completion: it orders a page-table write against the translations
            // a hart has cached, and this one caches none, so the ordering it asks for
            // already holds. `None` is the assertion — the instruction was decoded, took
            // effect, and the machine went on. It is the same shape of claim `fence`
            // has carried since `F-001b`, and the reason it can be made is that
            // `tests/paging.rs` proves the walk is done afresh on every access.
            "sfence.vma" => (pattern, None),
            other => panic!("{other}: a SYSTEM row this file has no opinion about"),
        };
        let mut out: Vec<u8> = Vec::new();
        assert_eq!(
            machine(word).step(&mut out),
            expected,
            "{insn} ({word:#010x})"
        );
        assert!(out.is_empty(), "{insn} put a byte on the console");
        seen += 1;
    }
    assert_eq!(seen, 10, "every row was exercised");
}

#[test]
fn a_bare_read_is_not_reported_as_a_write() {
    // `csrrs x5, stvec, x0` is how a read-only CSR access is written — no bits set means
    // no modification, which is the spec's own rule and not a special case.
    let word = 0x2073 | REFUSED << 20 | 5 << 7;
    assert_eq!(
        machine(word).step(&mut Vec::new()),
        Some(Halt::Csr {
            pc: BASE,
            csr: REFUSED as u16,
            write: false,
        })
    );
    // The immediate form with a zero uimm, likewise.
    let word = 0x6073 | REFUSED << 20 | 5 << 7;
    assert_eq!(
        machine(word).step(&mut Vec::new()),
        Some(Halt::Csr {
            pc: BASE,
            csr: REFUSED as u16,
            write: false,
        })
    );
}

#[test]
fn the_undefined_funct3_is_not_a_csr_instruction() {
    // funct3 = 4 is the one value SYSTEM leaves undefined, and it is not in the table.
    // Reporting it as a CSR access would be inventing a register the program never named.
    let word = 0x4073 | REFUSED << 20 | 5 << 7 | 6 << 15;
    assert_eq!(
        machine(word).step(&mut Vec::new()),
        Some(Halt::Unimplemented {
            pc: BASE,
            word,
            opcode: 0x73
        })
    );
}

const ECALL: u32 = 0x0000_0073;

#[test]
fn console_putchar_reaches_the_same_bridge_the_uart_does() {
    let mut m = machine(ECALL);
    m.x[17] = 1; // a7 = sbi_console_putchar
    m.x[10] = b'n'.into(); // a0 = the byte
    m.x[11] = 0x1234; // a1, which SBI is allowed to clobber
    m.x[12] = 0x5678; // a2, which it is not
    let mut out: Vec<u8> = Vec::new();
    assert_eq!(
        m.step(&mut out),
        None,
        "a putchar returns; it does not halt"
    );
    assert_eq!(out, b"n", "the byte reached the host");
    assert_eq!(m.x[10], 0, "the legacy call returns 0 in a0 for success");
    assert_eq!(m.x[12], 0x5678, "a2 is not the firmware's to touch");
    assert_eq!(m.pc, BASE + 4, "and execution went on");
}

#[test]
fn shutdown_stops_the_machine() {
    let mut m = machine(ECALL);
    m.x[17] = 8;
    assert_eq!(
        m.step(&mut Vec::new()),
        Some(Halt::Shutdown { pc: BASE }),
        "sbi_shutdown does not return, here or on hardware"
    );
}

#[test]
fn an_unimplemented_sbi_call_names_itself_rather_than_succeeding() {
    let mut m = machine(ECALL);
    m.x[17] = 0x4442_434e; // DBCN, the v0.2 debug console extension
    m.x[16] = 2; // console_write_byte
    let mut out: Vec<u8> = Vec::new();
    assert_eq!(
        m.step(&mut out),
        Some(Halt::Sbi {
            pc: BASE,
            eid: 0x4442_434e,
            fid: 2
        }),
        "the halt names the extension AND the function: an unrecognised EID is usually a \
         v0.2 call, and it takes the pair to say which one"
    );
    assert!(
        out.is_empty(),
        "and it emitted nothing — guessing that DBCN's write_byte means putchar is \
         exactly the plausible wrong answer this machine refuses to give"
    );
}

// ---------------------------------------------------------------------------------------
// Column 3: a whole program.

/// Assemble a Sassembly source at `load` and write the ELF `sadhana` would have written.
///
/// The same four calls `sadhana`'s own `main` makes, in the same order — so what runs
/// below is the artefact the toolchain produces, not one this file arranged to be easy.
fn build(source: &str, load: u64) -> Vec<u8> {
    let program = assemble_program(source).unwrap_or_else(|e| panic!("{e:?}"));
    let (text, pending) = encode_object(&program).unwrap_or_else(|e| panic!("{e:?}"));
    let bytes = object(
        &text,
        &program,
        &pending,
        None,
        &layout_addresses(&program, Target::Uncompressed),
    );
    let objects = [read(&bytes).expect("the object reads back")];
    let image = link_at(&objects, load).unwrap_or_else(|e| panic!("{e:?}"));
    write_debuggable_at(&image.text, &image.data, &image.table, image.bss, &[], load)
}

/// The one program in `spec/` that is a real SBI client: it runs *above* firmware at
/// `0x80200000`, so it cannot write the UART and asks `sbi_console_putchar` instead.
const BOOT_SBI: &str = include_str!("../../../spec/boot-sbi.sas");

#[test]
fn boot_sbi_runs_to_its_shutdown_and_says_the_same_thing_qemu_hears() {
    let elf = build(BOOT_SBI, SBI_PAYLOAD);
    let mut m = Machine::load_elf(&elf, 1 << 20).expect("the toolchain's own ELF must load");
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(10_000, &mut out);

    // `tools/check-sbi-boot.sh` boots this under real OpenSBI in QEMU and demands this
    // exact line. The message is SLP1, not Devanagari, because a boot payload runs before
    // any font exists — doc 01 §6.
    assert_eq!(
        String::from_utf8_lossy(&out),
        "namaste saMsAra\n",
        "the interpreter heard what QEMU hears"
    );
    assert!(
        matches!(halt, Halt::Shutdown { .. }),
        "and it stopped because the program asked the firmware to, not because it ran \
         out of budget or fell off the end: {halt:?}"
    );
}

#[test]
fn the_program_needed_the_ecall_to_get_there() {
    // The control for the test above. Without SYSTEM this program cannot say a word, so
    // if it were passing for some other reason — a stale expectation, an output buffer
    // that was already right — removing the ecall would not change the answer. Here the
    // first `ecall`'s byte is the whole message, so it does.
    let elf = build(BOOT_SBI, SBI_PAYLOAD);
    let mut m = Machine::load_elf(&elf, 1 << 20).expect("load");
    let mut out: Vec<u8> = Vec::new();
    for _ in 0..10_000 {
        // Rewrite a7 out from under it before every step: the extension the program
        // asked for becomes 0x99, which nothing implements.
        if m.x[17] == 1 {
            m.x[17] = 0x99;
        }
        if let Some(h) = m.step(&mut out) {
            assert!(
                matches!(h, Halt::Sbi { eid: 0x99, .. }),
                "it stopped at the console call, which is what carries the message: {h:?}"
            );
            assert!(out.is_empty(), "and nothing was printed");
            return;
        }
    }
    panic!("the program never reached an ecall — then the test above proves nothing");
}
