//! `W-351`, first milestone: the virtio-mmio register window answers who it is, and
//! refuses BY NAME everything it does not answer. The queue milestone's tests are in
//! `w351_queue.rs`.
//!
//! Driven by real RV64 instructions through `Machine::step`, so the path under test is the
//! one a `.t1` driver's four-octet loads and stores take — `W-350`'s seam lowers to these
//! same `lw` / `sw`. Every expected value is the specification's, cited per assertion from
//! `research/spec-extracts/virtio-1.2-csd01/`, never this crate's own constants read back:
//! a test that compared `virtio_mmio::MAGIC` with what the machine returned would agree
//! with any number the module chose.

use yantra::{Halt, Machine, Privilege};

const RAM_BASE: u64 = 0x8000_0000;
/// Slot 0, as C-009 (`spec/virtio-gpu.sas:67`) and QEMU riscv `virt` place it.
const SLOT0: u32 = 0x1000_1000;

fn machine(text: &[u32]) -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX,
        patra_root: None,
        patra_path: None,
        patra_buffer: None,
        virtio: Default::default(),
        x: [0; 32],
        f: [0; 32],
        fcsr: 0,
        pc: RAM_BASE,
        base: RAM_BASE,
        mem: vec![0; 1 << 16],
        reservation: None,
        csr: yantra::Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
        socket: None,
    };
    for (n, w) in text.iter().enumerate() {
        m.mem[n * 4..n * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    m
}

// Encoders written out, not taken from `sadhana::encode`, so the test and the machine do
// not share one source of the ISA (the same rule `interpreter.rs` states).
fn lui(rd: u32, imm: u32) -> u32 {
    (imm & 0xffff_f000) | (rd << 7) | 0x37
}
fn load(f3: u32, rd: u32, rs1: u32, imm: i32) -> u32 {
    ((imm as u32 & 0xfff) << 20) | (rs1 << 15) | (f3 << 12) | (rd << 7) | 0x03
}
fn store(f3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    ((imm >> 5 & 0x7f) << 25) | (rs2 << 20) | (rs1 << 15) | (f3 << 12) | ((imm & 0x1f) << 7) | 0x23
}
fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    ((imm as u32 & 0xfff) << 20) | (rs1 << 15) | (rd << 7) | 0x13
}
const LW: u32 = 0x2;
const LWU: u32 = 0x6;
const LD: u32 = 0x3;
const SW: u32 = 0x2;

/// Run `text` to its end, or until the machine halts.
fn run(m: &mut Machine, steps: usize) -> Option<Halt> {
    let mut out = Vec::new();
    for _ in 0..steps {
        if let Some(h) = m.step(&mut out) {
            return Some(h);
        }
    }
    None
}

/// Read the 32-bit register at `off` in slot 0 with `lwu`, so the value is not sign-extended.
fn read_reg(off: i32) -> Result<u64, Halt> {
    let mut m = machine(&[lui(1, SLOT0), load(LWU, 2, 1, off)]);
    match run(&mut m, 2) {
        None => Ok(m.x[2]),
        Some(h) => Err(h),
    }
}

#[test]
fn the_identity_registers_answer_the_specifications_values() {
    // 4.2.2-mmio-register-layout.txt:21 — "0x74726976 (a Little Endian equivalent of the
    // “virt” string)".
    assert_eq!(
        read_reg(0x000),
        Ok(0x7472_6976),
        "MagicValue must read \"virt\""
    );
    // 4.2.4-mmio-legacy-interface.txt:22 — Version, "Legacy device returns value 0x1."
    assert_eq!(read_reg(0x004), Ok(1), "a LEGACY device's Version is 1");
    // 5.7.1 — the GPU device's ID is 16.
    assert_eq!(read_reg(0x008), Ok(16), "DeviceID must be the GPU's, 16");
    // VendorID (4.2.4:24) has no prescribed value; it must answer, and not as zero, so a
    // driver that prints it sees a real reading rather than an absence.
    let vendor = read_reg(0x00c).expect("VendorID must answer");
    assert_ne!(
        vendor, 0,
        "VendorID read as zero, which is indistinguishable from no device"
    );
}

#[test]
fn lw_sign_extends_the_magic_like_any_other_word() {
    // The magic's top bit is clear, so `lw` and `lwu` agree; this pins that the window
    // returns through the ordinary load path and its sign rule, not a side channel.
    let mut m = machine(&[lui(1, SLOT0), load(LW, 2, 1, 0)]);
    assert_eq!(run(&mut m, 2), None);
    assert_eq!(m.x[2], 0x7472_6976);
}

#[test]
fn status_holds_what_the_driver_wrote_and_zero_resets_it() {
    // 4.2.4:38 — Status is RW: "Reading from this register returns the current device
    // status flags. Writing non-zero values to this register sets the status flags".
    // ACKNOWLEDGE | DRIVER = 1 | 2 (2.1-device-status-field.txt), the first step of §3.1.
    let mut m = machine(&[
        lui(1, SLOT0),
        addi(2, 0, 3),
        store(SW, 1, 2, 0x070),
        load(LWU, 3, 1, 0x070),
        store(SW, 1, 0, 0x070), // write zero: the reset
        load(LWU, 4, 1, 0x070),
    ]);
    assert_eq!(
        run(&mut m, 6),
        None,
        "Status writes and reads must not halt"
    );
    assert_eq!(m.x[3], 3, "Status must read back what the driver wrote");
    assert_eq!(m.x[4], 0, "writing zero to Status resets it");
    assert!(
        m.mem[0x1000..].iter().all(|&b| b == 0),
        "a device register write must not leak into RAM"
    );
}

fn refused(h: Result<u64, Halt>, addr: u64, says: &str) {
    match h {
        Err(Halt::Device { addr: a, why, .. }) => {
            assert_eq!(a, addr, "the refusal must name the register address");
            assert!(
                why.contains(says),
                "refusal {why:?} should mention {says:?}"
            );
        }
        other => panic!("{addr:#x}: expected Halt::Device naming {says:?}, got {other:?}"),
    }
}

#[test]
fn a_register_with_no_read_is_refused_by_name_never_read_as_zero() {
    // Milestone 1 refused HostFeatures and QueuePFN as unbuilt; W-351's queue milestone
    // answers both (tests/w351_queue.rs). What stays refused: a WRITE-ONLY register read
    // (QueueNotify is W in 4.2.4:35), whose zero would be a plausible "nothing pending",
    // and an offset the legacy table does not define at all.
    refused(read_reg(0x050), 0x1000_1050, "write-only");
    refused(read_reg(0x030), 0x1000_1030, "write-only");
    // 0x0f0 is not in the legacy table at all.
    refused(read_reg(0x0f0), 0x1000_10f0, "no register");
}

#[test]
fn a_write_to_a_read_only_register_is_refused() {
    // MagicValue is R in 4.2.4:21; a device that accepted the write would let a driver
    // corrupt the one value it uses to find the device.
    let mut m = machine(&[lui(1, SLOT0), addi(2, 0, 1), store(SW, 1, 2, 0x000)]);
    match run(&mut m, 3) {
        Some(Halt::Device { addr, why, .. }) => {
            assert_eq!(addr, 0x1000_1000);
            assert!(why.contains("read-only"), "{why}");
        }
        other => panic!("a write to MagicValue must be refused by name, got {other:?}"),
    }
}

#[test]
fn only_32_bit_aligned_accesses_are_answered() {
    // 4.2.2-mmio-register-layout.txt:85 — "The driver MUST only use 32 bit wide and
    // aligned reads and writes to access the control registers".
    let mut m = machine(&[lui(1, SLOT0), load(LD, 2, 1, 0)]);
    match run(&mut m, 2) {
        Some(Halt::Device { why, .. }) => assert!(why.contains("32-bit"), "{why}"),
        other => panic!("a 64-bit read of MagicValue must be refused, got {other:?}"),
    }
    refused(read_reg(0x002), 0x1000_1002, "32-bit");
}

#[test]
fn the_empty_slots_refuse_and_name_themselves() {
    // Slot 2's MagicValue. Without the window this was `BadAccess` — the same statement
    // as a wild pointer — which is the confusion `Halt::Device` exists to remove.
    let mut m = machine(&[lui(1, 0x1000_3000), load(LWU, 2, 1, 0)]);
    match run(&mut m, 2) {
        Some(Halt::Device { addr, why, .. }) => {
            assert_eq!(addr, 0x1000_3000);
            assert!(why.contains("no virtio device"), "{why}");
        }
        other => panic!("an empty slot must refuse by name, got {other:?}"),
    }
}

#[test]
fn slot_1_overlaps_the_file_window_and_the_file_window_still_works() {
    // patra::PATRA_PATH is 0x1000_2000 — slot 1's base. Its STORE must still reach the file
    // window (the patra arm is matched exactly, before the virtio window)...
    let mut m = machine(&[lui(1, 0x1000_2000), addi(2, 0, 0x40), store(0x3, 1, 2, 0)]);
    assert_eq!(
        run(&mut m, 3),
        None,
        "the PATRA_PATH store must not be refused"
    );
    assert_eq!(
        m.patra_path,
        Some(0x40),
        "the PATRA_PATH store must reach the file window"
    );
    // ...while a LOAD there, which the file window never answered, is refused as the overlap.
    let mut m = machine(&[lui(1, 0x1000_2000), load(LWU, 2, 1, 0)]);
    match run(&mut m, 2) {
        Some(Halt::Device { why, .. }) => assert!(why.contains("file window"), "{why}"),
        other => panic!("a load in slot 1 must name the overlap, got {other:?}"),
    }
}

#[test]
fn addresses_beside_the_window_are_untouched() {
    // Just below slot 0 is the UART's page; just above slot 7 is nothing. The window must
    // not widen what the machine answers: an unmapped address stays `BadAccess`.
    let mut m = machine(&[lui(1, 0x1000_9000), load(LWU, 2, 1, 0)]);
    match run(&mut m, 2) {
        Some(Halt::BadAccess { addr, .. }) => assert_eq!(addr, 0x1000_9000),
        other => panic!("0x1000_9000 is past slot 7 and must stay BadAccess, got {other:?}"),
    }
}
