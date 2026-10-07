//! `W-382` — the decoded-instruction cache and the RAM-first memory path must not change
//! what a program computes.
//!
//! [`Machine::run`] with a budget of at least 2^16 executes through a decoded cache; a
//! loop over [`Machine::step`] does not. Each test runs one program BOTH ways and asserts
//! the two machines end identical — registers, `pc`, the clock (W-374's count), RAM and
//! the halt — and asserts the value a correct machine computes, so a cache that went
//! stale in both paths at once could not pass either.
//!
//! THE FALSIFIER is `a_store_into_executed_code_is_executed_as_stored`: the program runs
//! an instruction, overwrites it with a plain `sw`, and jumps back to it. A cache that
//! kept serving the first decoding computes 1 where the machine must compute 2. The row's
//! evidence records it RED against a cache whose validity test was removed (the entry
//! reused whenever its slot was filled) and GREEN against the shipped one. Its in-run
//! DEVICE twin, `a_device_write_into_executed_code_within_one_run_is_executed`, has
//! `patra` write over a cached word mid-run — a writer that never passes through the
//! cached store forms at all.
//!
//! THE CONTRACT CASES (WAIT, the counter, a page fault delivered and undelivered,
//! `BeyondRam`) exist because the W-370/W-374 contract tests run budgets below
//! `DecodeCache::MIN_BUDGET` and so never reach the cached path; here each runs through
//! it against a `step()` twin.

use yantra::{DEVICE_TOP, DEVICE_WINDOWS, Halt, Machine, Privilege};

const BASE: u64 = 0x8000_0000;

/// A bare machine with `words` at the entry point and 64 KiB of RAM.
fn machine(words: &[u32]) -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX,
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
    for (i, w) in words.iter().enumerate() {
        m.mem[4 * i..4 * i + 4].copy_from_slice(&w.to_le_bytes());
    }
    m
}

// Hand encodings, each checked against its format below by the instruction it builds.
const fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    ((imm as u32 & 0xfff) << 20) | (rs1 << 15) | (rd << 7) | 0x13
}
const fn auipc(rd: u32, imm20: u32) -> u32 {
    (imm20 << 12) | (rd << 7) | 0x17
}
const fn lw(rd: u32, rs1: u32, imm: i32) -> u32 {
    ((imm as u32 & 0xfff) << 20) | (rs1 << 15) | (2 << 12) | (rd << 7) | 0x03
}
const fn sw(rs2: u32, rs1: u32, imm: i32) -> u32 {
    let i = imm as u32 & 0xfff;
    ((i >> 5) << 25) | (rs2 << 20) | (rs1 << 15) | (2 << 12) | ((i & 0x1f) << 7) | 0x23
}
const fn lui(rd: u32, imm20: u32) -> u32 {
    (imm20 << 12) | (rd << 7) | 0x37
}
const fn ld(rd: u32, rs1: u32, imm: i32) -> u32 {
    ((imm as u32 & 0xfff) << 20) | (rs1 << 15) | (3 << 12) | (rd << 7) | 0x03
}
const fn sd(rs2: u32, rs1: u32, imm: i32) -> u32 {
    let i = imm as u32 & 0xfff;
    ((i >> 5) << 25) | (rs2 << 20) | (rs1 << 15) | (3 << 12) | ((i & 0x1f) << 7) | 0x23
}
const fn amoswap_w(rd: u32, rs1: u32, rs2: u32) -> u32 {
    (0x01 << 27) | (rs2 << 20) | (rs1 << 15) | (2 << 12) | (rd << 7) | 0x2f
}
const fn beq(rs1: u32, rs2: u32, off: i32) -> u32 {
    let o = off as u32 & 0x1fff;
    (((o >> 12) & 1) << 31)
        | (((o >> 5) & 0x3f) << 25)
        | (rs2 << 20)
        | (rs1 << 15)
        | (((o >> 1) & 0xf) << 8)
        | (((o >> 11) & 1) << 7)
        | 0x63
}
const fn jal(rd: u32, off: i32) -> u32 {
    let o = off as u32 & 0x1f_ffff;
    (((o >> 20) & 1) << 31)
        | (((o >> 1) & 0x3ff) << 21)
        | (((o >> 11) & 1) << 20)
        | (((o >> 12) & 0xff) << 12)
        | (rd << 7)
        | 0x6f
}

/// The patched program. `x10` is set by the instruction at 0x04, which is overwritten
/// after its first execution by `patch` (a plain store, or an AMO) with the word at
/// 0x100 (`addi x10, x0, 2`); the loop runs twice and parks at 0x28.
fn program(patch: u32) -> Machine {
    let words = [
        addi(5, 0, 0),   // 0x00  x5 = passes
        addi(10, 0, 1),  // 0x04  THE PATCHED INSTRUCTION: x10 = 1, later x10 = 2
        addi(5, 5, 1),   // 0x08  passes += 1
        addi(6, 0, 2),   // 0x0c
        beq(5, 6, 0x18), // 0x10  second pass -> 0x28
        auipc(7, 0),     // 0x14  x7 = 0x8000_0014
        lw(8, 7, 0xec),  // 0x18  x8 = the new word, at 0x100
        addi(9, 7, -16), // 0x1c  x9 = 0x8000_0004
        patch,           // 0x20  write x8 at x9
        jal(0, -0x20),   // 0x24  back to 0x04
        jal(0, 0),       // 0x28  park
    ];
    let mut m = machine(&words);
    m.mem[0x100..0x104].copy_from_slice(&addi(10, 0, 2).to_le_bytes());
    m
}

/// Run `m` through the cached path (`run` with a budget past the threshold) and a clone
/// of it through `step` one instruction at a time; require identical ends.
fn both_ways(mut cached: Machine) -> (Machine, Halt) {
    let mut stepped = machine(&[]);
    stepped.mem.clone_from(&cached.mem);
    stepped.x = cached.x;
    stepped.pc = cached.pc;
    stepped.csr = cached.csr.clone();
    stepped.mode = cached.mode;
    stepped.time = cached.time;
    stepped.timecmp = cached.timecmp;
    stepped.store_limit = cached.store_limit;

    let mut out_c = Vec::new();
    let halt_c = cached.run(1 << 20, &mut out_c);

    let mut out_s = Vec::new();
    let mut halt_s = None;
    for _ in 0..(1 << 20) {
        if let Some(h) = stepped.step(&mut out_s) {
            halt_s = Some(h);
            break;
        }
    }
    let halt_s = halt_s.expect("the stepped twin halts inside the budget");

    assert_eq!(halt_c, halt_s, "cached and stepped halts differ");
    assert_eq!(cached.x, stepped.x, "registers differ");
    assert_eq!(cached.pc, stepped.pc, "pc differs");
    assert_eq!(
        cached.time, stepped.time,
        "retired-instruction count differs (W-374)"
    );
    assert_eq!(cached.csr, stepped.csr, "CSRs differ");
    assert_eq!(cached.mode, stepped.mode, "privilege differs");
    assert!(cached.mem == stepped.mem, "RAM differs");
    assert_eq!(out_c, out_s, "UART output differs");
    (cached, halt_c)
}

#[test]
fn a_store_into_executed_code_is_executed_as_stored() {
    let (m, halt) = both_ways(program(sw(8, 9, 0)));
    assert_eq!(halt, Halt::SpinForever { pc: BASE + 0x28 });
    assert_eq!(m.x[5], 2, "the loop ran twice");
    assert_eq!(
        m.x[10], 2,
        "the second pass must execute the STORED word (addi x10, x0, 2), not a stale decoding"
    );
    // 10 (first pass, 0x00 through the jal) + 4 (second pass, to the taken beq) + 1
    // (the park, counted before it reports itself).
    assert_eq!(m.time, 15);
}

#[test]
fn an_amo_into_executed_code_is_executed_as_stored() {
    // AMOs execute in `step_inner`, not the cached forms — a writer the cache never sees
    // directly, which is the case validating by the word (rather than invalidating on a
    // store) exists for.
    let (m, halt) = both_ways(program(amoswap_w(0, 9, 8)));
    assert_eq!(halt, Halt::SpinForever { pc: BASE + 0x28 });
    assert_eq!(m.x[10], 2);
}

#[test]
fn a_device_write_into_executed_code_within_one_run_is_executed() {
    // WITHIN ONE RUN, by a device: a `patra` GO with no path or buffer named refuses into
    // the status run at the address stored — eight octets of `BadAddress`
    // (0xffff_ffff_ffff_fffe) written by `patra::serve` straight into `mem`, never through
    // a cached store form. The program aims it at its own first instruction, already
    // executed and cached, and jumps back. The machine must now fetch 0xffff_fffe and
    // refuse it; a cache serving the old `addi` runs a second pass and parks instead.
    let words = [
        addi(5, 5, 1),   // 0x00  THE OVERWRITTEN INSTRUCTION: passes += 1
        addi(6, 0, 2),   // 0x04
        beq(5, 6, 0x18), // 0x08  a second pass of the OLD word -> park at 0x20
        auipc(7, 0),     // 0x0c  x7 = 0x8000_000c
        addi(7, 7, -12), // 0x10  x7 = 0x8000_0000, the overwritten word
        lui(8, 0x10002), // 0x14  x8 = 0x1000_2000
        sd(7, 8, 0x10),  // 0x18  PATRA_GO <- x7: the status lands at 0x8000_0000
        jal(0, -0x1c),   // 0x1c  back to 0x00
        jal(0, 0),       // 0x20  park (reached only by a stale decoding)
    ];
    let (m, halt) = both_ways(machine(&words));
    assert_eq!(
        halt,
        Halt::Unimplemented {
            pc: BASE,
            word: 0xffff_fffe,
            opcode: 0x7e
        },
        "the fetch after the device wrote must see the device's octets"
    );
    assert_eq!(m.x[5], 1, "the overwritten instruction ran exactly once");
}

#[test]
fn a_wait_through_the_cache_retires_and_resumes() {
    // W-370: a store to WAIT halts with the store retired and `pc` past it.
    let words = [
        addi(10, 0, 3),  // 0x00
        lui(5, 0x10000), // 0x04  x5 = 0x1000_0000
        sw(0, 5, 0x100), // 0x08  WAIT
        addi(10, 10, 4), // 0x0c
        jal(0, 0),       // 0x10
    ];
    let (mut m, halt) = both_ways(machine(&words));
    assert_eq!(halt, Halt::Wait { pc: BASE + 8 });
    assert_eq!(
        m.pc,
        BASE + 0xc,
        "a WAIT resumes at the instruction after its store"
    );
    assert_eq!(m.time, 3, "the WAIT store is counted (W-374)");
    assert_eq!(
        m.run(1 << 20, &mut Vec::new()),
        Halt::SpinForever { pc: BASE + 0x10 }
    );
    assert_eq!(m.x[10], 7);
    assert_eq!(m.time, 5);
}

#[test]
fn the_counter_through_the_cache_answers_the_count() {
    // W-374: a load at COUNTER answers `time`, the reading instruction already counted.
    let words = [
        lui(5, 0x10000), // 0x00
        lw(6, 5, 0x108), // 0x04  low word: time = 2
        addi(0, 0, 0),   // 0x08
        ld(7, 5, 0x108), // 0x0c  whole register: time = 4
        jal(0, 0),       // 0x10
    ];
    let (m, halt) = both_ways(machine(&words));
    assert_eq!(halt, Halt::SpinForever { pc: BASE + 0x10 });
    assert_eq!(
        (m.x[6], m.x[7]),
        (2, 4),
        "two reads k instructions apart differ by k + 1"
    );
}

#[test]
fn beyond_ram_through_the_cache_is_refused_by_name() {
    // A load past the end of RAM halts BeyondRam, not BadAccess and not a wrong value.
    // `auipc x5, 0x10` at BASE is BASE + 64 KiB, one past this machine's RAM.
    let words = [auipc(5, 0x10), ld(6, 5, 0), jal(0, 0)];
    let (_, halt) = both_ways(machine(&words));
    assert_eq!(
        halt,
        Halt::BeyondRam {
            pc: BASE + 4,
            addr: BASE + (1 << 16),
            ram: 1 << 16
        }
    );
    // And a store past the end, likewise.
    let words = [auipc(5, 0x10), sd(0, 5, 0), jal(0, 0)];
    let (_, halt) = both_ways(machine(&words));
    assert_eq!(
        halt,
        Halt::BeyondRam {
            pc: BASE + 4,
            addr: BASE + (1 << 16),
            ram: 1 << 16
        }
    );
}

/// Sv39 on, one gigapage identity-mapping 0x8000_0000 (V R W X A D), root table at
/// BASE + 0x4000; 0x4000_0000 is unmapped. `stvec` as given.
fn paged(words: &[u32], stvec: u64) -> Machine {
    let mut m = machine(words);
    let root = 0x4000usize;
    let pte: u64 = ((BASE >> 12) << 10) | 0xcf;
    m.mem[root + 2 * 8..root + 3 * 8].copy_from_slice(&pte.to_le_bytes());
    m.csr.satp = (8 << 60) | ((BASE + root as u64) >> 12);
    m.csr.stvec = stvec;
    m
}

#[test]
fn a_page_fault_through_the_cache_is_delivered() {
    // A load from an unmapped page with a handler installed: cause 13 delivered to stvec,
    // `sepc` the load, `stval` the address — and the run goes on in the handler.
    let mut words = vec![lui(5, 0x40000), ld(6, 5, 8), jal(0, 0)];
    words.resize(0x200 / 4, 0);
    words.push(jal(0, 0)); // 0x200, the handler
    let (m, halt) = both_ways(paged(&words, BASE + 0x200));
    assert_eq!(halt, Halt::SpinForever { pc: BASE + 0x200 });
    assert_eq!(m.csr.scause, 13);
    assert_eq!(m.csr.sepc, BASE + 4);
    assert_eq!(m.csr.stval, 0x4000_0008);
}

#[test]
fn a_page_fault_through_the_cache_with_no_handler_halts() {
    let words = [lui(5, 0x40000), sd(0, 5, 8), jal(0, 0)];
    let (m, halt) = both_ways(paged(&words, 0));
    assert_eq!(
        halt,
        Halt::PageFault {
            pc: BASE + 4,
            addr: 0x4000_0008,
            cause: 15
        }
    );
    assert_eq!(m.pc, BASE + 4, "the faulting store took no effect");
}

/// The body of `fn <name>` in `src/lib.rs`, up to the next item at the same indent.
fn body_of<'a>(src: &'a str, name: &str) -> &'a str {
    let head = format!("fn {name}(");
    let at = src
        .find(&head)
        .unwrap_or_else(|| panic!("lib.rs has no `{head}`"));
    let rest = &src[at..];
    let end = rest[1..].find("\n    fn ").map_or(rest.len(), |e| e + 1);
    &rest[..end]
}

#[test]
fn device_top_covers_every_device() {
    // DERIVED FROM THE ARMS, NOT A SECOND LIST: every address `load_walk`, `store_walk`
    // and `counter` decode is read out of their source — `addr == NAME`, a
    // `checked_sub(NAME)` window, and the `virtio_mmio::slot_of(addr)` test — and must
    // have a row in `DEVICE_WINDOWS`, from which `DEVICE_TOP` is computed. The table
    // itself is hand-kept (the arms compare against the named constants); this is what
    // keeps it honest.
    let src = include_str!("../src/lib.rs");
    let mut named = std::collections::BTreeSet::new();
    for f in ["load_walk", "store_walk", "counter"] {
        let body = body_of(src, f);
        for (pat, stop) in [
            ("addr == ", &[' ', '{', ')'][..]),
            ("checked_sub(", &[')'][..]),
        ] {
            let mut from = 0;
            while let Some(i) = body[from..].find(pat) {
                let start = from + i + pat.len();
                let len = body[start..].find(stop).expect("an arm's name ends");
                let name = &body[start..start + len];
                // `checked_sub(self.base)` is the RAM arm, not a device.
                if name != "self.base" {
                    named.insert(name.to_string());
                }
                from = start;
            }
        }
        if body.contains("virtio_mmio::slot_of(addr)") {
            named.insert("virtio_mmio::slot_of".to_string());
        }
    }
    // The control: a scanner that found nothing would pass vacuously.
    assert!(
        named.len() >= 9,
        "the scan of the arms found only {named:?} — the scanner, not the table, is wrong"
    );
    let rows: Vec<&str> = DEVICE_WINDOWS.iter().map(|(n, _, _)| *n).collect();
    for name in &named {
        assert!(
            rows.contains(&name.as_str()),
            "an arm decodes `{name}` and DEVICE_WINDOWS has no row for it, so DEVICE_TOP may \
             lie below it and the fast path would skip that device"
        );
    }
    for (name, at, len) in DEVICE_WINDOWS {
        assert!(
            at + len <= DEVICE_TOP,
            "{name} at {at:#x}+{len} lies above DEVICE_TOP"
        );
    }
    // Both are constants, so the check is made at compile time (clippy::assertions_on_constants).
    const {
        assert!(
            DEVICE_TOP <= BASE,
            "RAM must start above every device for the fast path"
        )
    };
}

#[test]
fn device_stores_still_reach_their_devices_under_the_cache() {
    // UART and the finisher through the cached `run`: the store forms are cached, the
    // device addresses are below `DEVICE_TOP`, so they must fall to the original arms.
    // lui x5, 0x10000 (UART); addi x6, x0, 'K'; sb x6, 0(x5);
    // lui x7, 0x100 (FINISHER); lui x8, 5; addi x8, x8, 0x555; sw x8, 0(x7)
    let sb = |rs2: u32, rs1: u32| (rs2 << 20) | (rs1 << 15) | 0x23;
    let words = [
        lui(5, 0x10000),
        addi(6, 0, i32::from(b'K')),
        sb(6, 5),
        lui(7, 0x100),
        lui(8, 5),
        addi(8, 8, 0x555),
        sw(8, 7, 0),
    ];
    let mut m = machine(&words);
    let mut out = Vec::new();
    let halt = m.run(1 << 20, &mut out);
    assert_eq!(out, b"K");
    assert_eq!(
        halt,
        Halt::Finisher {
            value: 0x5555,
            status: Some(0)
        }
    );
    assert_eq!(m.time, 7);
}
