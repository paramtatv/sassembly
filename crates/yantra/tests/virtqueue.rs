//! `W-352` slice (a): one split virtqueue processes a descriptor chain and writes a used
//! element — and refuses, by name and WITHOUT writing anything, every chain it cannot walk
//! safely.
//!
//! The rings are laid out here BY HAND from the structs in
//! `research/spec-extracts/virtio-1.2-csd01/2.7.4-2.7.8-framing-descriptors-rings.txt`
//! (`virtq_desc` :84-99, `virtq_avail` :186-191, `virtq_used` / `virtq_used_elem`
//! :273-289), never from `yantra::virtqueue`'s own offsets, so a layout mistake in the
//! module cannot be mirrored by the test that checks it.

use yantra::virtqueue::{Queue, Refusal, process};

const BASE: u64 = 0x8000_0000;
const RAM: usize = 1 << 16;
const SIZE: u16 = 8;
// Guest physical addresses of the three areas and of two buffers, all inside RAM.
const DESC: u64 = BASE + 0x1000; // 16 bytes x SIZE
const AVAIL: u64 = BASE + 0x2000; // flags, idx, ring[SIZE]
const USED: u64 = BASE + 0x3000; // flags, idx, ring[SIZE] of {le32 id, le32 len}
const REQ: u64 = BASE + 0x4000;
const RESP: u64 = BASE + 0x5000;

const F_NEXT: u16 = 1; // VIRTQ_DESC_F_NEXT, :91
const F_WRITE: u16 = 2; // VIRTQ_DESC_F_WRITE, :93
const F_INDIRECT: u16 = 4; // VIRTQ_DESC_F_INDIRECT, :95

fn at(addr: u64) -> usize {
    (addr - BASE) as usize
}
fn put16(m: &mut [u8], a: u64, v: u16) {
    m[at(a)..at(a) + 2].copy_from_slice(&v.to_le_bytes());
}
fn put32(m: &mut [u8], a: u64, v: u32) {
    m[at(a)..at(a) + 4].copy_from_slice(&v.to_le_bytes());
}
fn get16(m: &[u8], a: u64) -> u16 {
    u16::from_le_bytes([m[at(a)], m[at(a) + 1]])
}
fn get32(m: &[u8], a: u64) -> u32 {
    u32::from_le_bytes(m[at(a)..at(a) + 4].try_into().unwrap())
}

/// `struct virtq_desc { le64 addr; le32 len; le16 flags; le16 next; }` — 16 bytes.
fn desc(m: &mut [u8], i: u16, addr: u64, len: u32, flags: u16, next: u16) {
    let d = DESC + 16 * u64::from(i);
    m[at(d)..at(d) + 8].copy_from_slice(&addr.to_le_bytes());
    put32(m, d + 8, len);
    put16(m, d + 12, flags);
    put16(m, d + 14, next);
}

/// Offer `head` in the available ring: `ring[idx % SIZE] = head`, then `idx += 1`.
fn offer(m: &mut [u8], head: u16) {
    let idx = get16(m, AVAIL + 2);
    put16(m, AVAIL + 4 + 2 * u64::from(idx % SIZE), head);
    put16(m, AVAIL + 2, idx.wrapping_add(1));
}

fn queue() -> Queue {
    Queue {
        size: SIZE,
        desc: DESC,
        avail: AVAIL,
        used: USED,
    }
}

/// RAM holding a two-descriptor request: 24 readable bytes, then 24 writable bytes.
fn two_descriptor_request(head: u16, second: u16) -> Vec<u8> {
    let mut m = vec![0u8; RAM];
    for (k, b) in (0..24u8).enumerate() {
        m[at(REQ) + k] = b + 1;
    }
    desc(&mut m, head, REQ, 24, F_NEXT, second);
    desc(&mut m, second, RESP, 24, F_WRITE, 0);
    offer(&mut m, head);
    m
}

/// The handler every test uses: answer with the request's first `n` bytes reversed.
fn reverse(n: usize) -> impl FnMut(&[u8], &[u8]) -> Vec<u8> {
    move |_mem: &[u8], req: &[u8]| req.iter().take(n).rev().copied().collect()
}

#[test]
fn a_chain_is_walked_and_its_used_element_names_the_head_and_the_bytes_written() {
    let mut m = two_descriptor_request(0, 1);
    let mut last = 0u16;
    let done = process(&mut m, BASE, &queue(), &mut last, reverse(24)).expect("must process");
    assert_eq!(done, Some((0, 24)), "the head index and the bytes written");
    // :282-289 — used.ring[0] = { id: head, len: bytes written into the writable part }.
    assert_eq!(
        get32(&m, USED + 4),
        0,
        "used elem id must be the chain's HEAD index"
    );
    assert_eq!(
        get32(&m, USED + 8),
        24,
        "used elem len must be the bytes written"
    );
    // :308 — used idx "starts at 0, and increases".
    assert_eq!(get16(&m, USED + 2), 1, "used idx must advance by one");
    assert_eq!(last, 1, "the device's own avail cursor must advance");
    let resp: Vec<u8> = (1..=24u8).rev().collect();
    assert_eq!(
        &m[at(RESP)..at(RESP) + 24],
        &resp[..],
        "the response must be in the writable buffer"
    );
    // :113 — "A device MUST NOT write to a device-readable buffer".
    let req: Vec<u8> = (1..=24u8).collect();
    assert_eq!(
        &m[at(REQ)..at(REQ) + 24],
        &req[..],
        "the readable buffer must be untouched"
    );
}

#[test]
fn the_used_id_is_the_head_index_not_the_slot_and_a_second_chain_takes_the_next_slot() {
    let mut m = two_descriptor_request(3, 5);
    let mut last = 0u16;
    assert_eq!(
        process(&mut m, BASE, &queue(), &mut last, reverse(8)).unwrap(),
        Some((3, 8))
    );
    desc(&mut m, 6, REQ, 4, F_NEXT, 2);
    desc(&mut m, 2, RESP + 0x100, 4, F_WRITE, 0);
    offer(&mut m, 6);
    assert_eq!(
        process(&mut m, BASE, &queue(), &mut last, reverse(4)).unwrap(),
        Some((6, 4))
    );
    assert_eq!(get32(&m, USED + 4), 3);
    assert_eq!(get32(&m, USED + 8), 8);
    assert_eq!(
        get32(&m, USED + 4 + 8),
        6,
        "the second element goes in used.ring[1]"
    );
    assert_eq!(get32(&m, USED + 4 + 12), 4);
    assert_eq!(get16(&m, USED + 2), 2);
}

#[test]
fn an_empty_queue_processes_nothing_and_writes_nothing() {
    let mut m = vec![0u8; RAM];
    let before = m.clone();
    let mut last = 0u16;
    assert_eq!(
        process(&mut m, BASE, &queue(), &mut last, reverse(0)).unwrap(),
        None
    );
    assert!(m == before, "an empty queue must not write");
}

/// A refusal must name itself AND leave RAM exactly as it was: no used element, no
/// partial response — a half-written reply is the plausible wrong answer.
fn refuses(mut m: Vec<u8>, want: impl Fn(&Refusal) -> bool, what: &str) {
    let before = m.clone();
    let mut last = 0u16;
    match process(&mut m, BASE, &queue(), &mut last, reverse(24)) {
        Err(r) if want(&r) => {}
        other => panic!("{what}: expected the named refusal, got {other:?}"),
    }
    assert!(m == before, "{what}: a refused chain must not change RAM");
    assert_eq!(
        last, 0,
        "{what}: a refused chain must not advance the avail cursor"
    );
}

#[test]
fn a_next_index_outside_the_table_is_refused() {
    let mut m = two_descriptor_request(0, 1);
    desc(&mut m, 0, REQ, 24, F_NEXT, SIZE); // next == Queue Size: no such descriptor
    refuses(
        m,
        |r| matches!(r, Refusal::BadNext { at: 0, next } if *next == SIZE),
        "bad next",
    );
}

#[test]
fn a_head_outside_the_table_is_refused() {
    let mut m = vec![0u8; RAM];
    offer(&mut m, SIZE + 1);
    refuses(m, |r| matches!(r, Refusal::BadHead { .. }), "bad head");
}

#[test]
fn a_buffer_outside_ram_is_refused() {
    let mut m = two_descriptor_request(0, 1);
    desc(&mut m, 1, BASE + RAM as u64 - 8, 24, F_WRITE, 0); // runs 16 bytes past the end
    refuses(m, |r| matches!(r, Refusal::OutOfRam { .. }), "out of RAM");
}

#[test]
fn a_ring_outside_ram_is_refused() {
    let mut m = two_descriptor_request(0, 1);
    let before = m.clone();
    let mut last = 0u16;
    let q = Queue {
        used: BASE + RAM as u64 - 4,
        ..queue()
    };
    match process(&mut m, BASE, &q, &mut last, reverse(24)) {
        Err(Refusal::OutOfRam { .. }) => {}
        other => panic!("a used ring past RAM must be refused, got {other:?}"),
    }
    assert!(m == before, "nothing written");
}

#[test]
fn a_next_loop_is_refused_not_walked_forever() {
    // :119-120 — "loops in the descriptor chain are forbidden"; :162 — no chain longer
    // than the Queue Size.
    let mut m = vec![0u8; RAM];
    desc(&mut m, 0, REQ, 4, F_NEXT, 1);
    desc(&mut m, 1, REQ, 4, F_NEXT, 0);
    offer(&mut m, 0);
    refuses(m, |r| matches!(r, Refusal::Loop { .. }), "loop");
}

#[test]
fn a_readable_descriptor_after_a_writable_one_is_refused() {
    // :52 — "The driver MUST place any device-writable descriptor elements after any
    // device-readable descriptor elements."
    let mut m = vec![0u8; RAM];
    desc(&mut m, 0, RESP, 24, F_WRITE | F_NEXT, 1);
    desc(&mut m, 1, REQ, 24, 0, 0);
    offer(&mut m, 0);
    refuses(
        m,
        |r| matches!(r, Refusal::ReadableAfterWritable { at: 1 }),
        "framing",
    );
}

#[test]
fn an_indirect_descriptor_is_refused_because_the_feature_is_not_offered() {
    // :157 — the driver MUST NOT set INDIRECT unless VIRTIO_F_INDIRECT_DESC was negotiated,
    // and this device does not offer it.
    let mut m = two_descriptor_request(0, 1);
    desc(&mut m, 0, REQ, 32, F_INDIRECT, 0);
    refuses(m, |r| matches!(r, Refusal::Indirect { at: 0 }), "indirect");
}

#[test]
fn a_response_longer_than_the_writable_space_is_refused_not_truncated() {
    let m = two_descriptor_request(0, 1);
    let before = m.clone();
    let mut m = m;
    let mut last = 0u16;
    let long = |_: &[u8], _: &[u8]| vec![7u8; 25]; // one byte more than the 24 writable
    match process(&mut m, BASE, &queue(), &mut last, long) {
        Err(Refusal::ResponseTooLong { have: 24, need: 25 }) => {}
        other => panic!("an overlong response must be refused, got {other:?}"),
    }
    assert!(m == before, "nothing written");
}

#[test]
fn a_zero_sized_queue_is_refused() {
    let mut m = vec![0u8; RAM];
    let mut last = 0u16;
    let q = Queue { size: 0, ..queue() };
    assert!(matches!(
        process(&mut m, BASE, &q, &mut last, reverse(0)),
        Err(Refusal::NoQueue)
    ));
}
