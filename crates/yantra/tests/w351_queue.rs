//! `W-351`, queue milestone: a driver identifies the GPU, sets up queue 0 on the legacy
//! layout, offers one chain and rings QueueNotify — and reads back a used entry, the
//! spec's "not served" reply and the interrupt bit.
//!
//! Driven by real RV64 `sw`/`lwu` through `Machine::step`, the path a `.t1` driver's
//! four-octet device intrinsics lower to (`W-350`). Every expected value is cited from
//! `research/spec-extracts/virtio-1.2-csd01/`, never read back from `virtio_mmio`'s own
//! constants. Each refusal test also checks the register it refused did not change: a
//! refusal that half-applies is the plausible wrong answer.

use yantra::{Halt, Machine, Privilege, virtio_mmio};

const RAM_BASE: u64 = 0x8000_0000;
const SLOT0: u32 = 0x1000_1000;

// The queue, placed by hand on the legacy layout (2.7.2-legacy-virtqueue-layout.txt):
// page 4096, PFN 0x80008 -> the descriptor table at 0x8000_8000; 8 entries, so the
// available ring follows at +128 and runs 2*(3+8) = 22 bytes to 0x8000_8096; the used
// ring is at the next 4096 boundary, 0x8000_9000.
const PAGE: u32 = 4096;
const PFN: u32 = 0x80008;
const NUM: u32 = 8;
const DESC: u64 = 0x8000_8000;
const AVAIL: u64 = 0x8000_8080;
const USED: u64 = 0x8000_9000;
const REQ: u64 = 0x8000_a000;
const RESP: u64 = 0x8000_a100;

fn lui(rd: u32, imm: u32) -> u32 {
    (imm & 0xffff_f000) | (rd << 7) | 0x37
}
fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    ((imm as u32 & 0xfff) << 20) | (rs1 << 15) | (rd << 7) | 0x13
}
fn sw(rs1: u32, rs2: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    ((imm >> 5 & 0x7f) << 25) | (rs2 << 20) | (rs1 << 15) | (0x2 << 12) | ((imm & 0x1f) << 7) | 0x23
}
fn lwu(rd: u32, rs1: u32, imm: i32) -> u32 {
    ((imm as u32 & 0xfff) << 20) | (rs1 << 15) | (0x6 << 12) | (rd << 7) | 0x03
}

/// One driver action: write `value` to the register at `off`, or read it into a slot.
#[derive(Clone, Copy)]
enum Op {
    W(i32, u32),
    R(i32),
}
use Op::{R, W};

/// The program for `ops`, with x1 = slot 0's base and each read landing in x10, x11, ...
fn program(ops: &[Op]) -> Vec<u32> {
    let mut t = vec![lui(1, SLOT0)];
    let mut rd = 10;
    for op in ops {
        match *op {
            W(off, v) => {
                // li x2, v — lui + addi, the addi's sign folded into the lui.
                let hi = v.wrapping_add(0x800) & 0xffff_f000;
                let lo = v.wrapping_sub(hi) as i32;
                t.push(lui(2, hi));
                t.push(addi(2, 2, lo));
                t.push(sw(1, 2, off));
            }
            R(off) => {
                t.push(lwu(rd, 1, off));
                rd += 1;
            }
        }
    }
    t
}

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

/// Run every instruction of `ops`' program, or stop at the first halt.
fn drive(m: &mut Machine, ops: &[Op]) -> Option<Halt> {
    let n = program(ops).len();
    let mut out = Vec::new();
    for _ in 0..n {
        if let Some(h) = m.step(&mut out) {
            return Some(h);
        }
    }
    None
}

fn at(addr: u64) -> usize {
    (addr - RAM_BASE) as usize
}
fn put(m: &mut Machine, addr: u64, bytes: &[u8]) {
    m.mem[at(addr)..at(addr) + bytes.len()].copy_from_slice(bytes);
}
fn rd16(m: &Machine, addr: u64) -> u16 {
    u16::from_le_bytes([m.mem[at(addr)], m.mem[at(addr) + 1]])
}
fn rd32(m: &Machine, addr: u64) -> u32 {
    u32::from_le_bytes(m.mem[at(addr)..at(addr) + 4].try_into().unwrap())
}

/// One chain on the queue: descriptor 0 readable (24 octets of request), descriptor 1
/// writable (24 octets for the reply), offered at available-ring slot 0 with idx 1.
/// Layouts from 2.7.4-2.7.8-framing-descriptors-rings.txt:84-99 and :186-191.
fn offer_one_chain(m: &mut Machine) {
    let desc = |addr: u64, len: u32, flags: u16, next: u16| {
        let mut d = addr.to_le_bytes().to_vec();
        d.extend(len.to_le_bytes());
        d.extend(flags.to_le_bytes());
        d.extend(next.to_le_bytes());
        d
    };
    put(m, DESC, &desc(REQ, 24, 1, 1)); // VIRTQ_DESC_F_NEXT
    put(m, DESC + 16, &desc(RESP, 24, 2, 0)); // VIRTQ_DESC_F_WRITE
    // Type 0x0999 is no virtio-gpu command, so the device answers the 24-octet
    // ERR_UNSPEC header (W-378 serves the real ones; GET_DISPLAY_INFO's 408-octet reply
    // would not fit this 24-octet slot, and the ring rightly refuses that).
    put(m, REQ, &0x0999u32.to_le_bytes());
    put(m, RESP, &[0xff; 24]);
    put(m, AVAIL + 2, &1u16.to_le_bytes()); // idx
    put(m, AVAIL + 4, &0u16.to_le_bytes()); // ring[0] = head 0
}

/// §4.2.4's legacy set-up of queue 0, steps 1-7, after ACKNOWLEDGE | DRIVER.
fn set_up() -> Vec<Op> {
    vec![
        W(0x070, 3),
        W(0x028, PAGE),
        W(0x030, 0),
        R(0x040), // x10: QueuePFN, expect 0 (step 2)
        R(0x034), // x11: QueueNumMax (step 3)
        W(0x038, NUM),
        W(0x03c, 4096),
        W(0x040, PFN),
    ]
}

#[test]
fn a_driver_sets_up_queue_0_rings_it_and_reads_back_a_used_entry() {
    let mut ops = set_up();
    ops.push(W(0x050, 0)); // QueueNotify, queue 0
    ops.push(R(0x060)); // x12: InterruptStatus
    ops.push(W(0x064, 1)); // InterruptACK bit 0
    ops.push(R(0x060)); // x13: InterruptStatus again
    ops.push(R(0x010)); // x14: HostFeatures
    let mut m = machine(&program(&ops));
    offer_one_chain(&mut m);
    assert_eq!(
        drive(&mut m, &ops),
        None,
        "the legacy set-up and one notify must not halt"
    );

    assert_eq!(
        m.x[10], 0,
        "4.2.4 step 2: QueuePFN reads 0 before the queue is used"
    );
    assert!(
        m.x[11] >= u64::from(NUM),
        "QueueNumMax must admit the 8 entries C-009 uses"
    );
    // 2.7.4-2.7.8:273-289 — used ring: le16 flags, le16 idx, then {le32 id, le32 len}.
    assert_eq!(
        rd16(&m, USED + 2),
        1,
        "used.idx must read 1 after one chain"
    );
    assert_eq!(rd32(&m, USED + 4), 0, "the used element names head 0");
    assert_eq!(
        rd32(&m, USED + 8),
        24,
        "the used element's len is the 24 octets written"
    );
    // 5.7.6.7: VIRTIO_GPU_RESP_ERR_UNSPEC = 0x1200, and the rest of the header zero.
    assert_eq!(
        rd32(&m, RESP),
        0x1200,
        "the reply is the spec's not-served header"
    );
    assert!(
        m.mem[at(RESP) + 4..at(RESP) + 24].iter().all(|&b| b == 0),
        "the header's other fields are zero"
    );
    assert_eq!(
        m.x[12], 1,
        "InterruptStatus bit 0, used buffer notification, is raised"
    );
    assert_eq!(m.x[13], 0, "InterruptACK clears the bit it names");
    assert_eq!(m.x[14], 0, "the device offers no feature bits");
}

#[test]
fn a_second_notify_with_nothing_new_serves_nothing() {
    // The cursor advanced to 1, so a second notify with avail.idx still 1 must not walk
    // the same chain again — that would re-run a command the driver issued once.
    let mut ops = set_up();
    ops.push(W(0x050, 0));
    ops.push(W(0x064, 1));
    ops.push(W(0x050, 0));
    ops.push(R(0x060)); // x12
    let mut m = machine(&program(&ops));
    offer_one_chain(&mut m);
    assert_eq!(drive(&mut m, &ops), None);
    assert_eq!(rd16(&m, USED + 2), 1, "used.idx stays 1");
    assert_eq!(m.x[12], 0, "no new used buffer, so no new interrupt");
}

fn refusal(m: &mut Machine, ops: &[Op], says: &str) {
    match drive(m, ops) {
        Some(Halt::Device { why, .. }) => assert!(
            why.contains(says),
            "refusal {why:?} should mention {says:?}"
        ),
        other => panic!("expected Halt::Device naming {says:?}, got {other:?}"),
    }
}

#[test]
fn refusal_1_a_queue_size_that_is_zero_not_a_power_of_2_or_too_large() {
    // 2.7-split-virtqueues.txt:49 — "Queue Size value is always a power of 2"; 4.2.4 — the
    // driver chooses "a queue size smaller than or equal to QueueNumMax".
    for bad in [0, 3, 6, virtio_mmio::QUEUE_NUM_MAX * 2] {
        let ops = [W(0x030, 0), W(0x038, bad)];
        let mut m = machine(&program(&ops));
        refusal(&mut m, &ops, "power of 2");
        assert_eq!(
            m.virtio.queues[0].num, 0,
            "QueueNum {bad} was refused and must not be held"
        );
    }
    // A queue the GPU does not have: QueueNumMax reads 0 (4.2.4: "zero (0x0) if the queue
    // is not available"), and setting its size is refused.
    let ops = [W(0x030, 2), R(0x034), W(0x038, 8)];
    let mut m = machine(&program(&ops));
    refusal(&mut m, &ops, "does not have");
    assert_eq!(m.x[10], 0, "QueueNumMax of queue 2 reads 0");
}

#[test]
fn refusal_2_a_queue_pfn_before_the_page_size_or_outside_ram() {
    // Before GuestPageSize: the page QueuePFN counts in is unknown (4.2.4 table).
    let ops = [W(0x030, 0), W(0x038, NUM), W(0x03c, 4096), W(0x040, PFN)];
    let mut m = machine(&program(&ops));
    refusal(&mut m, &ops, "before GuestPageSize");
    assert_eq!(
        m.virtio.queues[0].pfn, 0,
        "a refused QueuePFN must not be held"
    );

    // Above RAM: RAM is 64 KiB at 0x8000_0000, so page 0x80010 is its first byte past.
    let mut ops = set_up();
    *ops.last_mut().unwrap() = W(0x040, 0x80010);
    let mut m = machine(&program(&ops));
    refusal(&mut m, &ops, "outside RAM");
    assert_eq!(m.virtio.queues[0].pfn, 0);

    // The last page of RAM: the descriptor table fits, the used ring a page later does not.
    let mut ops = set_up();
    *ops.last_mut().unwrap() = W(0x040, 0x8000f);
    let mut m = machine(&program(&ops));
    refusal(&mut m, &ops, "outside RAM");
    assert_eq!(m.virtio.queues[0].pfn, 0);
}

#[test]
fn the_layout_cannot_wrap_and_its_extreme_is_refused() {
    // The overflow case the reviewer asked for. Every input is a 32-bit register, so the
    // largest descriptor address is (2^32 - 1) * 2^31 < 2^63, and the used ring's end adds
    // under 2^32 more: no sum can pass 2^64. The arithmetic is checked anyway; this pins
    // the extreme so a widening of any register is caught here rather than by a wrap.
    let max_page = 1u32 << 31;
    let (desc, avail, used, end) =
        virtio_mmio::legacy_layout(u32::MAX, max_page, virtio_mmio::QUEUE_NUM_MAX, max_page)
            .expect("the extreme layout must compute without overflow");
    assert_eq!(desc, u64::from(u32::MAX) * u64::from(max_page));
    assert!(
        desc < avail && avail < used && used < end,
        "the areas must ascend, not wrap"
    );
    // And the machine refuses it at QueuePFN, as above RAM.
    let ops = [
        W(0x028, max_page),
        W(0x030, 0),
        W(0x038, 256),
        W(0x03c, max_page),
        W(0x040, u32::MAX),
    ];
    let mut m = machine(&program(&ops));
    refusal(&mut m, &ops, "outside RAM");
    assert_eq!(m.virtio.queues[0].pfn, 0);
}

#[test]
fn the_other_refusals_name_themselves() {
    let cases: [(&[Op], &str); 6] = [
        (&[W(0x028, 3000)], "power of 2"),             // GuestPageSize
        (&[W(0x030, 0), W(0x03c, 100)], "power of 2"), // QueueAlign
        (&[W(0x020, 1)], "does not offer"),            // GuestFeatures, a bit not offered
        (&[W(0x050, 0)], "not set up"),                // notify before QueuePFN
        (&[W(0x050, 2)], "does not have"),             // notify a queue the GPU lacks
        (&[W(0x034, 1)], "read-only"),                 // QueueNumMax is R
    ];
    for (ops, says) in cases {
        let mut m = machine(&program(ops));
        refusal(&mut m, ops, says);
    }
    // In use: QueueNumMax "is allowed only when QueuePFN is set to zero" (4.2.4), and
    // resizing a live queue would move its rings under the device's cursor.
    for tail in [R(0x034), W(0x038, 4), W(0x03c, 8192)] {
        let mut ops = set_up();
        ops.push(tail);
        let mut m = machine(&program(&ops));
        refusal(&mut m, &ops, "in use");
        assert_eq!(m.virtio.queues[0].num, NUM, "the live queue keeps its size");
    }
}

#[test]
fn a_reset_stops_every_queue() {
    // 4.2.4 table, Status: "Writing zero (0x0) to this register triggers a device reset.
    // The device sets QueuePFN to zero (0x0) for all queues in the device."
    let mut ops = set_up();
    ops.push(W(0x050, 0));
    ops.push(W(0x070, 0)); // reset
    ops.push(R(0x040)); // x12: QueuePFN
    ops.push(R(0x034)); // x13: QueueNumMax, readable again
    ops.push(R(0x060)); // x14: InterruptStatus
    let mut m = machine(&program(&ops));
    offer_one_chain(&mut m);
    assert_eq!(drive(&mut m, &ops), None);
    assert_eq!(m.x[12], 0, "QueuePFN reads 0 after a reset");
    assert!(
        m.x[13] >= u64::from(NUM),
        "QueueNumMax is readable once the queue is stopped"
    );
    assert_eq!(m.x[14], 0, "a reset clears InterruptStatus");
    assert_eq!(
        m.virtio.queues[0].last_avail, 0,
        "the cursor resets with the queue"
    );
}

#[test]
fn a_chain_the_ring_refuses_halts_by_name_and_writes_nothing() {
    // An INDIRECT descriptor (flag 4): the device offers no VIRTIO_F_INDIRECT_DESC, so
    // virtqueue::process refuses it, and the machine says so rather than BadAccess.
    let mut ops = set_up();
    ops.push(W(0x050, 0));
    let mut m = machine(&program(&ops));
    offer_one_chain(&mut m);
    put(&mut m, DESC + 12, &4u16.to_le_bytes());
    refusal(&mut m, &ops, "INDIRECT");
    assert_eq!(
        rd16(&m, USED + 2),
        0,
        "a refused chain writes no used entry"
    );
    assert_eq!(rd32(&m, RESP), 0xffff_ffff, "and no reply");
}

#[test]
fn legacy_layout_refuses_an_alignment_that_is_not_a_power_of_2() {
    // A peer session's review: `legacy_layout` is public, and an align of 0 underflowed
    // `align - 1` (a panic under overflow checks, which the deep gate turns on) while 3
    // gave a wrong mask silently. It must refuse both on its own, whatever its callers do.
    assert_eq!(
        virtio_mmio::legacy_layout(PFN, PAGE, NUM, 0),
        None,
        "align 0"
    );
    assert_eq!(
        virtio_mmio::legacy_layout(PFN, PAGE, NUM, 3),
        None,
        "align 3"
    );
    assert!(
        virtio_mmio::legacy_layout(PFN, PAGE, NUM, 4096).is_some(),
        "align 4096 is the control"
    );
}

#[test]
fn the_page_size_cannot_change_under_a_live_queue() {
    // A peer session's review: QueueNum and QueueAlign refused while QueuePFN was nonzero, but
    // GuestPageSize did not, and the next notify recomputed the queue's place from the new
    // page — a layout never validated against RAM. The same rule as IN_USE now applies.
    let mut ops = set_up();
    ops.push(W(0x028, 8192));
    let mut m = machine(&program(&ops));
    refusal(&mut m, &ops, "in use");
    assert_eq!(
        m.virtio.page_size, PAGE,
        "the live queue's page size is kept"
    );
    // The control: the same write with the queue stopped (QueuePFN 0) is accepted.
    let mut ops = set_up();
    ops.push(W(0x040, 0));
    ops.push(W(0x028, 8192));
    let mut m = machine(&program(&ops));
    assert_eq!(
        drive(&mut m, &ops),
        None,
        "a stopped queue's page size may change"
    );
    assert_eq!(m.virtio.page_size, 8192);
}
