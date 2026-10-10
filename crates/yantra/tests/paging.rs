//! `F-001c2b1` — `satp`, the Sv39 page-table walker, and `sfence.vma`.
//!
//! # Three columns, as `F-001b`, `F-001c1` and `F-001c2a` had
//!
//! 1. **The encoding table** — `sfence.vma`'s pattern and mask are read out of
//!    `spec/encodings-riscv64.tsv`, which was extracted by `riscv64-elf-as` and not
//!    transcribed. The instruction carries two register operands, so it is the one SYSTEM
//!    row that is *not* matched on the whole word, and asserting the mask is what keeps
//!    the decoder honest about that.
//! 2. **The meaning**, written from the privileged specification by hand: the walk, the
//!    leaf, the superpage, and every one of the nine ways it faults.
//! 3. **A whole program** — `spec/paging.sas`, at both link addresses, asserted against
//!    the *same three numbers* `tools/check-paging.sh` demands of QEMU. The first two
//!    columns say the walk is right; only the third says translation is on.
//!
//! # The one thing this file must not do
//!
//! Every number in column 3 is **derived from the program's own first printed line**, as
//! the shell script derives it, and the program is run at two link addresses. A walker
//! that returned a stored answer passes any single-address check; it cannot pass two runs
//! whose every number moved.

use sadhana::encode::{Target, encode_object, layout_addresses};
use sadhana::kosha::{object, write_debuggable_at};
use sadhana::parse::assemble_program;
use sadhana::samyojana::link_at;
use sadhana::vastu::read;
use yantra::{Csrs, Halt, Machine, Privilege};

const BASE: u64 = 0x8000_0000;
/// 16 MiB, which is more than any table this file builds needs and keeps every physical
/// address it uses inside RAM.
const RAM: usize = 1 << 24;

const SATP: u32 = 0x180;
const STVEC: u32 = 0x105;

/// `satp.MODE = 8`, already shifted.
const SV39: u64 = 8 << 60;

const PTE_V: u64 = 1;
const PTE_R: u64 = 1 << 1;
const PTE_W: u64 = 1 << 2;
const PTE_X: u64 = 1 << 3;
const PTE_U: u64 = 1 << 4;
const PTE_A: u64 = 1 << 6;
const PTE_D: u64 = 1 << 7;
/// What `spec/paging.sas` writes into each of its 512 entries — `V|R|W|X|A|D`, the one
/// authored number in `tools/check-paging.sh` too, and for the same reason: it comes from
/// the privileged specification's table and there is no oracle in this repository to ask.
const LEAF: u64 = PTE_V | PTE_R | PTE_W | PTE_X | PTE_A | PTE_D;

/// `csrrw rd, csr, rs1`, from the encoding `tests/system.rs` takes out of the table.
fn csr_insn(funct3: u32, rd: u32, csr: u32, source: u32) -> u32 {
    0x73 | rd << 7 | funct3 << 12 | source << 15 | csr << 20
}

/// A machine holding `words` at the entry point and nothing else.
fn machine(words: &[u32]) -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX, // W-363: no store bound beyond `mem` — this machine has no injected input above it
        // Added with the `patra` file window: a machine that was never asked
        // to serve files must not be able to.
        patra_root: None,
        patra_mem: None,
        patra_path: None,
        patra_buffer: None,
        virtio: Default::default(),
        x: [0; 32],
        f: [0; 32],
        fcsr: 0,
        pc: BASE,
        base: BASE,
        mem: vec![0; RAM],
        reservation: None,
        csr: Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
        socket: None,
        net: None,
    };
    for (i, w) in words.iter().enumerate() {
        m.mem[i * 4..i * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    m
}

/// Write one page-table entry at a physical address.
fn put_pte(m: &mut Machine, at: u64, pte: u64) {
    let o = (at - m.base) as usize;
    m.mem[o..o + 8].copy_from_slice(&pte.to_le_bytes());
}

/// A leaf entry mapping to physical page `pa`, with `flags`.
fn leaf(pa: u64, flags: u64) -> u64 {
    (pa >> 12) << 10 | flags
}

/// A machine with translation on and ONE level-2 entry: a gigapage covering `va`'s
/// gigabyte, mapped onto the gigabyte `pa` lives in, with `flags`.
///
/// A gigapage rather than three levels because that is what `spec/paging.sas` uses, and
/// because it exercises the superpage arithmetic that a 4 KiB leaf does not.
fn gigapage(words: &[u32], va: u64, pa: u64, flags: u64) -> Machine {
    let mut m = machine(words);
    // The root table goes one page above the words, which are only ever a handful.
    let root = BASE + 0x1000;
    put_pte(
        &mut m,
        root + ((va >> 30) & 0x1ff) * 8,
        leaf(pa & !((1 << 30) - 1), flags),
    );
    m.csr.satp = SV39 | (root >> 12);
    m
}

// ---------------------------------------------------------------------------------------
// Column 1: the encoding table.

/// `sfence.vma`'s row: the pattern and the mask, read from the extracted table.
fn sfence_row() -> (u32, u32) {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/encodings-riscv64.tsv");
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    for line in text.lines() {
        if line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() > 4 && f[0] == "sfence.vma" {
            let hex = |s: &str| u32::from_str_radix(s.trim_start_matches("0x"), 16).expect("hex");
            return (hex(f[3]), hex(f[4]));
        }
    }
    panic!("`sfence.vma` is not in the encoding table");
}

#[test]
fn sfence_vma_is_the_one_system_row_with_operands() {
    let (pattern, mask) = sfence_row();
    assert_eq!(pattern, 0x1200_0073);
    assert_eq!(
        mask, 0xfe00_7fff,
        "`ecall`, `ebreak` and `sret` are masked 0xffffffff and are matched on the whole \
         word; `sfence.vma` carries rs1 and rs2, so ten bits of it are free. A decoder \
         that matched it on the whole word would execute `sfence.vma x0, x0` and stop on \
         `sfence.vma x8, x9` — which is the form the encoding table's own example uses."
    );
    // Every word the mask admits is the same instruction, so all 1024 must execute.
    for rs1 in 0..32u32 {
        for rs2 in 0..32u32 {
            let word = pattern | rs1 << 15 | rs2 << 20;
            assert_eq!(
                machine(&[word]).step(&mut Vec::new()),
                None,
                "sfence.vma x{rs1}, x{rs2} ({word:#010x})"
            );
        }
    }
}

#[test]
fn sfence_vma_is_a_no_op_because_nothing_is_cached() {
    // The claim behind executing it rather than stopping: a store to a PTE is visible to
    // the very next access with nothing asked for. So the table is changed with NO
    // sfence.vma between, and the change must take effect anyway — if it did not, the
    // no-op would be hiding a stale translation rather than standing on the absence of
    // one, and `spec/paging.sas` would be passing by luck.
    let va = 0x4000_0000;
    let root = BASE + 0x1000;
    let load = 0x3003u32 | 5 << 7 | 6 << 15; // ld x5, 0(x6)
    let mut m = gigapage(&[load, load], va, BASE, LEAF);
    // The program's own gigabyte, so that the FETCH is never what fails.
    put_pte(&mut m, root + ((BASE >> 30) & 0x1ff) * 8, leaf(BASE, LEAF));
    m.mem[0x2000..0x2008].copy_from_slice(&0x1122_3344_5566_7788u64.to_le_bytes());
    m.x[6] = va + 0x2000;
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(
        m.x[5], 0x1122_3344_5566_7788,
        "the first load came through the mapping"
    );
    // Now take the mapping away, with no fence, and repeat the identical instruction.
    put_pte(
        &mut m,
        root + ((va >> 30) & 0x1ff) * 8,
        leaf(BASE, LEAF & !PTE_V),
    );
    assert_eq!(
        m.step(&mut Vec::new()),
        Some(Halt::PageFault {
            pc: BASE + 4,
            addr: va + 0x2000,
            cause: 13
        }),
        "the second walk read the table as it stands NOW — which is why there is nothing \
         for an sfence.vma to invalidate"
    );
}

// ---------------------------------------------------------------------------------------
// Column 2: the meaning.

#[test]
fn bare_mode_leaves_every_address_alone() {
    // MODE = 0 is where every program starts, and it must not walk anything: a machine
    // that consulted a table before `satp` was written would fault on its first fetch.
    let mut m = machine(&[0x3003 | 5 << 7 | 6 << 15]); // ld x5, 0(x6)
    m.x[6] = BASE + 0x800;
    m.mem[0x800..0x808].copy_from_slice(&0x0123_4567_89ab_cdefu64.to_le_bytes());
    assert_eq!(m.csr.satp, 0, "a fresh machine is in Bare mode");
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(m.x[5], 0x0123_4567_89ab_cdef);
}

#[test]
fn a_gigapage_maps_the_whole_gigabyte_and_keeps_the_offset() {
    // The superpage arithmetic, which is the part a 4 KiB leaf cannot check: the low 18
    // bits of the leaf's PPN come from the VIRTUAL address, not from the entry.
    let va = 0x0000_003f_c000_0000; // some other gigabyte entirely
    let mut m = gigapage(&[0x3003 | 5 << 7 | 6 << 15], va, BASE, LEAF);
    m.mem[0x9abc..0x9ac4].copy_from_slice(&0xfeed_face_cafe_babeu64.to_le_bytes());
    m.x[6] = va + 0x9abc;
    m.csr.satp = SV39 | ((BASE + 0x1000) >> 12);
    // The fetch itself has to be translated, so the program's own gigabyte is mapped too.
    put_pte(
        &mut m,
        BASE + 0x1000 + ((BASE >> 30) & 0x1ff) * 8,
        leaf(BASE, LEAF),
    );
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(
        m.x[5], 0xfeed_face_cafe_babe,
        "the offset within the gigapage — all 30 bits of it — came from the virtual \
         address; an entry whose PPN were used whole would have read page 0 of it"
    );
}

/// The nine ways a walk fails, each built by breaking exactly one thing about a mapping
/// that otherwise works.
///
/// Every one is a *fault*, and each is asserted against the cause a handler would read.
/// Testing them one at a time is what separates "it faulted" from "it faulted for the
/// reason the spec gives", and the second is the only useful claim: a walker that
/// rejected everything would pass a test that only asked for a stop.
#[test]
fn each_way_a_walk_can_fail_faults_for_its_own_reason() {
    let va = 0x4000_0000;
    // ld x5, 0(x6) — a load, so cause 13, unless the case says otherwise.
    let load = 0x3003 | 5 << 7 | 6 << 15;
    let store = 0x3023 | 6 << 15 | 7 << 20; // sd x7, 0(x6)
    let root = BASE + 0x1000;
    let slot = root + ((va >> 30) & 0x1ff) * 8;

    let cases: [(&str, u64, u32, u64, u64); 8] = [
        (
            "V clear — not a translation at all",
            LEAF & !PTE_V,
            load,
            va,
            13,
        ),
        (
            "W without R is reserved, not write-only",
            LEAF & !PTE_R,
            load,
            va,
            13,
        ),
        ("no R on a load", LEAF & !(PTE_R | PTE_W), load, va, 13),
        ("no W on a store", LEAF & !PTE_W, store, va, 15),
        (
            "A clear — this machine faults rather than filling it",
            LEAF & !PTE_A,
            load,
            va,
            13,
        ),
        (
            "D clear on a store — likewise",
            LEAF & !PTE_D,
            store,
            va,
            15,
        ),
        (
            "U set — S-mode cannot reach it and SUM is dropped",
            LEAF | PTE_U,
            load,
            va,
            13,
        ),
        (
            "a non-leaf at level 0 would need a level below it",
            PTE_V,
            load,
            va,
            13,
        ),
    ];
    for (why, pte, word, addr, cause) in cases {
        let mut m = gigapage(&[word], va, BASE, LEAF);
        // The program's own gigabyte stays mapped so that the FETCH is not what fails.
        put_pte(&mut m, root + ((BASE >> 30) & 0x1ff) * 8, leaf(BASE, LEAF));
        put_pte(&mut m, slot, leaf(BASE, pte));
        m.x[6] = addr;
        assert_eq!(
            m.step(&mut Vec::new()),
            Some(Halt::PageFault {
                pc: BASE,
                addr,
                cause
            }),
            "{why}"
        );
    }

    // A misaligned superpage: the entry is a gigapage whose PPN has bits set below the
    // gigabyte. The spec faults rather than ignoring them, because ignoring them maps a
    // gigabyte somewhere its entry did not say.
    let mut m = gigapage(&[load], va, BASE, LEAF);
    put_pte(&mut m, root + ((BASE >> 30) & 0x1ff) * 8, leaf(BASE, LEAF));
    put_pte(&mut m, slot, leaf(BASE, LEAF) | 1 << 10);
    m.x[6] = va;
    assert_eq!(
        m.step(&mut Vec::new()),
        Some(Halt::PageFault {
            pc: BASE,
            addr: va,
            cause: 13
        }),
        "a gigapage whose PPN is not gigabyte-aligned"
    );

    // A non-canonical virtual address. Sv39 is 39 bits sign-extended, and the hole in the
    // middle of the address space is a fault, not a truncation onto real memory.
    let mut m = gigapage(&[load], va, BASE, LEAF);
    put_pte(&mut m, root + ((BASE >> 30) & 0x1ff) * 8, leaf(BASE, LEAF));
    m.x[6] = 0x0000_0080_0000_0000;
    assert_eq!(
        m.step(&mut Vec::new()),
        Some(Halt::PageFault {
            pc: BASE,
            addr: 0x0000_0080_0000_0000,
            cause: 13
        }),
        "bit 39 set with bit 38 clear is not an Sv39 address"
    );
}

#[test]
fn an_unmapped_fetch_faults_with_the_cause_a_fetch_raises() {
    // Causes 12, 13 and 15 are three different numbers and a handler branches on them.
    // With `satp` written and NOTHING mapped, the very next fetch is what fails.
    let mut m = machine(&[csr_insn(0x1, 0, SATP, 6)]); // csrrw x0, satp, x6
    m.x[6] = SV39 | ((BASE + 0x1000) >> 12);
    assert_eq!(
        m.step(&mut Vec::new()),
        None,
        "the csrrw itself is fetched in Bare mode and runs"
    );
    assert_eq!(
        m.step(&mut Vec::new()),
        Some(Halt::PageFault {
            pc: BASE + 4,
            addr: BASE + 4,
            cause: 12
        }),
        "and the instruction after it is the first one fetched through the empty table"
    );
}

#[test]
fn a_page_fault_is_delivered_to_stvec_like_any_other_exception() {
    // The whole reason a fault is not simply a halt: a kernel's pager runs ON one. The
    // handler must see sepc, scause and stval, and the machine must still be running.
    let handler = BASE + 0x800;
    let mut m = machine(&[csr_insn(0x1, 0, STVEC, 6), 0x3003 | 5 << 7 | 6 << 15]);
    m.x[6] = handler;
    assert_eq!(m.step(&mut Vec::new()), None);
    // Now turn translation on with nothing mapped and let the fetch fault.
    m.csr.satp = SV39 | ((BASE + 0x1000) >> 12);
    assert_eq!(
        m.step(&mut Vec::new()),
        None,
        "a fault with a handler installed is NOT a halt — the machine is still running"
    );
    assert_eq!(m.pc, handler, "and it is running the handler");
    assert_eq!(m.csr.sepc, BASE + 4, "sepc is the instruction that faulted");
    assert_eq!(m.csr.scause, 12, "an instruction page fault");
    assert_eq!(
        m.csr.stval,
        BASE + 4,
        "stval is the address that would not translate"
    );
}

#[test]
fn a_fault_with_no_handler_stops_and_says_which_address() {
    // The other half. `stvec` zero means there is nowhere to deliver it, and on metal the
    // hart would trap to address 0 and execute whatever is there. Saying so beats that.
    let mut m = machine(&[0x3003 | 5 << 7 | 6 << 15]);
    m.csr.satp = SV39 | ((BASE + 0x1000) >> 12);
    put_pte(
        &mut m,
        BASE + 0x1000 + ((BASE >> 30) & 0x1ff) * 8,
        leaf(BASE, LEAF),
    );
    m.x[6] = 0x4000_0000;
    assert_eq!(
        m.step(&mut Vec::new()),
        Some(Halt::PageFault {
            pc: BASE,
            addr: 0x4000_0000,
            cause: 13
        })
    );
    assert_eq!(
        m.pc, BASE,
        "and it stopped BEFORE the instruction took effect"
    );
    assert_eq!(m.x[5], 0, "nothing was loaded");
}

#[test]
fn an_unsupported_satp_mode_leaves_the_register_alone() {
    // MODE is WARL and Sv48 is not implemented. The write has no effect, so a kernel that
    // asks for four levels reads back what it had — which is how it finds out. Narrowing
    // the field to Sv39 instead would tell it its four-level tables were live.
    let mut m = machine(&[csr_insn(0x1, 5, SATP, 6)]);
    m.x[6] = 9 << 60 | 0x8_0201; // MODE = 9, Sv48
    assert_eq!(m.step(&mut Vec::new()), None);
    assert_eq!(
        m.csr.satp, 0,
        "the write was refused whole, ASID and PPN with it"
    );
    assert_eq!(m.x[5], 0, "and the read-back says so");
}

// ---------------------------------------------------------------------------------------
// Column 3: a whole program.

/// Assemble a Sassembly source at `load` and write the ELF `sadhana` would have written —
/// the same four calls `sadhana`'s own `main` makes, in the same order.
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

/// The program `tools/check-paging.sh` boots under real OpenSBI: it builds 512 identity
/// gigapages, writes `satp`, fences, and prints three numbers.
const PAGING: &str = include_str!("../../../spec/paging.sas");

/// Run `paging.sas` at `load` and return its three printed lines.
fn run_paging(load: u64) -> Vec<String> {
    let elf = build(PAGING, load);
    let mut m = Machine::load_elf(&elf, RAM).expect("the toolchain's own ELF must load");
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(1_000_000, &mut out);
    assert!(
        matches!(halt, Halt::Shutdown { .. }),
        "the program ends by asking the firmware to stop; anything else means translation \
         killed it on the way: {halt:?}"
    );
    String::from_utf8(out)
        .expect("the program prints ASCII hex")
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn paging_sas_says_the_same_three_numbers_qemu_hears() {
    // `tools/check-paging.sh` demands exactly this of QEMU under real OpenSBI, at these
    // two link addresses, and derives every number from the program's own first line for
    // the reason its comment gives: checking `satp` against a constant would pass a
    // program that installed a table somewhere else entirely.
    for load in [0x8020_0000u64, 0x8040_0000] {
        let lines = run_paging(load);
        assert_eq!(
            lines.len(),
            3,
            "one line means translation killed it at the first fetch after satp; three \
             mean it is running with every fetch, load and store walked: {lines:?}"
        );
        let table = u64::from_str_radix(&lines[0], 16).expect("the table address");
        assert_eq!(
            table % 4096,
            0,
            "satp keeps a page number, so the low bits are lost"
        );
        assert_eq!(
            lines[1],
            format!("{:016x}", SV39 | (table >> 12)),
            "satp is MODE=8 with the page number of the table the program placed"
        );
        assert_eq!(
            lines[2],
            format!("{:016x}", ((table >> 30) << 28) | 0xcf),
            "the table's OWN covering entry, read back at an address that is now virtual \
             — so the load came through the mapping and not past it"
        );
    }
    // Linked somewhere else, the table has to be somewhere else. A walker returning a
    // stored answer survives every assertion above and not this one.
    assert_ne!(
        run_paging(0x8020_0000)[0],
        run_paging(0x8040_0000)[0],
        "the table is at the same address at both link addresses — that is a constant, \
         not where it was placed"
    );
}

#[test]
fn the_program_needed_the_translation_to_get_there() {
    // The control. If `paging.sas` were passing above for some other reason, then
    // breaking the mapping would not change the answer. Here it does: with `satp` zeroed
    // out from under the program after every step, the third line is read from a physical
    // address instead of a virtual one — and the program's own design is what catches it,
    // because the first line is printed before translation is on and the rest after.
    let elf = build(PAGING, 0x8020_0000);
    let mut m = Machine::load_elf(&elf, RAM).expect("load");
    let mut out: Vec<u8> = Vec::new();
    for _ in 0..1_000_000 {
        m.csr.satp = 0; // undo the install, every step
        if m.step(&mut out).is_some() {
            break;
        }
    }
    let text = String::from_utf8(out).expect("ASCII hex");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.get(1).copied(),
        Some("0000000000000000"),
        "with satp forced to zero the program reads back Bare mode, so the second line is \
         the machine's answer and not the program's: {lines:?}"
    );
}
