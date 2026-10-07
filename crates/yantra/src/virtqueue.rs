//! One split virtqueue: walk the next descriptor chain the driver offered, hand its
//! device-readable bytes to a handler, write the handler's answer into the chain's
//! device-writable buffers, and return the buffer through the used ring — `W-352`
//! slice (a).
//!
//! # What this is, and what it is not yet
//!
//! THE PROCESSOR ONLY. Nothing here is reachable from a running program: `W-352`'s
//! acceptance — a chain submitted by a `.t1` driver and its used entry read back by that
//! driver, end to end on `yantra` — needs `W-351`'s device to call [`process`] when the
//! driver writes QueueNotify, and to tell it where the three areas are (legacy QueuePFN,
//! §4.2.4). Until then this is a tested routine with no caller in the machine.
//!
//! The three areas are passed as guest physical addresses, which makes the walk the same
//! under the legacy interface (one contiguous region from QueuePFN, the used ring aligned
//! to QueueAlign) and the modern one (three separate addresses). Only where the numbers
//! come from differs, and that is the device's business, not the ring's.
//!
//! # Where the layout comes from
//!
//! `research/spec-extracts/virtio-1.2-csd01/2.7.4-2.7.8-framing-descriptors-rings.txt`:
//! `virtq_desc` (:84-99, 16 bytes: le64 addr, le32 len, le16 flags, le16 next),
//! `virtq_avail` (:186-191: le16 flags, le16 idx, le16 ring[size]), `virtq_used` and
//! `virtq_used_elem` (:273-289: le16 flags, le16 idx, then {le32 id, le32 len}[size]).
//!
//! # Every refusal writes nothing
//!
//! A chain this routine cannot walk safely is refused BY NAME ([`Refusal`]) before a
//! single byte of RAM changes: no response bytes, no used element, no cursor advance. A
//! half-written reply with a used element pointing at it is the plausible wrong answer a
//! driver would trust. The checks, each with the spec line it enforces:
//! - a head or `next` index outside the descriptor table;
//! - a ring or buffer that does not lie wholly in RAM;
//! - a chain longer than the queue size — which is how a `next` loop shows itself
//!   (:119-120 "loops in the descriptor chain are forbidden", :162);
//! - a device-readable descriptor after a device-writable one (:52, framing);
//! - an INDIRECT descriptor, because this device does not offer VIRTIO_F_INDIRECT_DESC
//!   (:157);
//! - a chain whose total length passes 2^32 bytes (:119);
//! - an answer longer than the writable space — refused, not truncated.

const DESC_LEN: u64 = 16;
const F_NEXT: u16 = 1;
const F_WRITE: u16 = 2;
const F_INDIRECT: u16 = 4;

/// Where one split virtqueue lives in guest memory, and how many entries it has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Queue {
    /// The queue size the driver set (QueueNum): entries in each of the three areas.
    pub size: u16,
    /// Guest physical address of the descriptor table.
    pub desc: u64,
    /// Guest physical address of the available ring.
    pub avail: u64,
    /// Guest physical address of the used ring.
    pub used: u64,
}

/// Why a chain was not processed. Every variant means RAM was left untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The queue has size zero: the driver never set it up.
    NoQueue,
    /// `len` bytes at `addr` — a ring or a buffer — do not lie wholly in RAM.
    OutOfRam {
        /// Start of the region.
        addr: u64,
        /// Its length in bytes.
        len: u64,
    },
    /// The available ring offered a head index outside the descriptor table.
    BadHead {
        /// The index offered.
        head: u16,
    },
    /// Descriptor `at` continues to `next`, which is outside the table.
    BadNext {
        /// The descriptor whose `next` is wrong.
        at: u16,
        /// The index it names.
        next: u16,
    },
    /// The chain ran past the queue size, so it loops; `at` is where the walk stopped.
    Loop {
        /// The descriptor reached once the chain was already queue-size long.
        at: u16,
    },
    /// Descriptor `at` is device-readable but follows a device-writable one.
    ReadableAfterWritable {
        /// The misplaced descriptor.
        at: u16,
    },
    /// Descriptor `at` is INDIRECT, a feature this device does not offer.
    Indirect {
        /// The indirect descriptor.
        at: u16,
    },
    /// The chain's buffers add up to more than 2^32 bytes.
    TooLong,
    /// The handler's answer needs `need` bytes and the chain has only `have` writable.
    ResponseTooLong {
        /// Writable bytes the chain provides.
        have: u64,
        /// Bytes the answer needs.
        need: u64,
    },
}

impl Refusal {
    /// The refusal as a fixed sentence, for `Halt::Device`'s `why`, which carries no data.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::NoQueue => "virtqueue refused: the queue has size zero",
            Self::OutOfRam { .. } => {
                "virtqueue refused: a ring or buffer does not lie wholly in RAM"
            }
            Self::BadHead { .. } => {
                "virtqueue refused: the available ring offered a head outside the descriptor table"
            }
            Self::BadNext { .. } => {
                "virtqueue refused: a descriptor's next is outside the descriptor table"
            }
            Self::Loop { .. } => {
                "virtqueue refused: the chain is longer than the queue, so it loops"
            }
            Self::ReadableAfterWritable { .. } => {
                "virtqueue refused: a device-readable descriptor follows a device-writable one"
            }
            Self::Indirect { .. } => {
                "virtqueue refused: an INDIRECT descriptor, a feature this device does not offer"
            }
            Self::TooLong => {
                "virtqueue refused: the chain's buffers add up to more than 2^32 bytes"
            }
            Self::ResponseTooLong { .. } => {
                "virtqueue refused: the answer is longer than the chain's writable space"
            }
        }
    }
}

/// Bytes `[addr, addr + len)` as an index range into `mem`, or the refusal.
fn span(mem: &[u8], base: u64, addr: u64, len: u64) -> Result<std::ops::Range<usize>, Refusal> {
    let out = Refusal::OutOfRam { addr, len };
    let start = addr.checked_sub(base).ok_or_else(|| out.clone())?;
    let end = start.checked_add(len).ok_or_else(|| out.clone())?;
    if end > mem.len() as u64 {
        return Err(out);
    }
    Ok(start as usize..end as usize)
}

fn rd16(mem: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([mem[at], mem[at + 1]])
}

/// One descriptor that has passed every check, and where its bytes are.
struct Segment {
    bytes: std::ops::Range<usize>,
    write: bool,
}

/// Process the next chain the driver offered, if there is one.
///
/// `last_avail` is the device's own cursor into the available ring — the count of chains
/// it has taken — and advances only when a chain is processed. `serve` receives a
/// READ-ONLY view of RAM and the chain's device-readable bytes, concatenated in chain
/// order, and returns the answer to write into its device-writable buffers. RAM is
/// passed because a device may have to read guest memory the request only names — a GPU
/// transfer reads its backing (`W-378`).
///
/// THE VIEW AND THE WRITES CANNOT ALIAS: the request is copied out of RAM into its own
/// buffer first, the view is a shared reborrow of `mem` that ends when `serve` returns,
/// and every write below happens after that. The borrow checker enforces the order, so
/// `serve` can never see a half-written reply or the used ring mid-update.
///
/// Returns `Ok(None)` when the driver has offered nothing new, and `Ok(Some((head, len)))`
/// after writing the used element `{ id: head, len }`.
///
/// # Errors
/// A [`Refusal`] naming why the chain could not be walked. RAM is unchanged.
pub fn process(
    mem: &mut [u8],
    base: u64,
    q: &Queue,
    last_avail: &mut u16,
    mut serve: impl FnMut(&[u8], &[u8]) -> Vec<u8>,
) -> Result<Option<(u16, u32)>, Refusal> {
    if q.size == 0 {
        return Err(Refusal::NoQueue);
    }
    let size = u64::from(q.size);
    // All three areas must lie in RAM before any is read. `used_event` / `avail_event`
    // are only present with VIRTIO_F_EVENT_IDX, which this device does not offer.
    let desc = span(mem, base, q.desc, DESC_LEN * size)?;
    let avail = span(mem, base, q.avail, 4 + 2 * size)?;
    let used = span(mem, base, q.used, 4 + 8 * size)?;

    let offered = rd16(mem, avail.start + 2);
    if offered == *last_avail {
        return Ok(None);
    }
    let slot = usize::from(*last_avail % q.size);
    let head = rd16(mem, avail.start + 4 + 2 * slot);
    if head >= q.size {
        return Err(Refusal::BadHead { head });
    }

    // THE WALK. Every descriptor is checked before anything is written.
    let mut chain: Vec<Segment> = Vec::new();
    let mut total: u64 = 0;
    let mut seen_write = false;
    let mut i = head;
    loop {
        if chain.len() >= usize::from(q.size) {
            return Err(Refusal::Loop { at: i });
        }
        let d = desc.start + DESC_LEN as usize * usize::from(i);
        let addr = u64::from_le_bytes(mem[d..d + 8].try_into().expect("8 bytes"));
        let len = u64::from(u32::from_le_bytes(
            mem[d + 8..d + 12].try_into().expect("4 bytes"),
        ));
        let flags = rd16(mem, d + 12);
        let next = rd16(mem, d + 14);
        if flags & F_INDIRECT != 0 {
            return Err(Refusal::Indirect { at: i });
        }
        let write = flags & F_WRITE != 0;
        if seen_write && !write {
            return Err(Refusal::ReadableAfterWritable { at: i });
        }
        seen_write |= write;
        total += len;
        if total > u64::from(u32::MAX) {
            return Err(Refusal::TooLong);
        }
        chain.push(Segment {
            bytes: span(mem, base, addr, len)?,
            write,
        });
        if flags & F_NEXT == 0 {
            break;
        }
        if next >= q.size {
            return Err(Refusal::BadNext { at: i, next });
        }
        i = next;
    }

    let request: Vec<u8> = chain
        .iter()
        .filter(|s| !s.write)
        .flat_map(|s| mem[s.bytes.clone()].iter().copied())
        .collect();
    let answer = serve(mem, &request);
    let have: u64 = chain
        .iter()
        .filter(|s| s.write)
        .map(|s| s.bytes.len() as u64)
        .sum();
    let need = answer.len() as u64;
    if need > have {
        return Err(Refusal::ResponseTooLong { have, need });
    }

    // THE WRITES, only now that nothing can be refused. The answer first, across the
    // writable buffers in chain order; then the used element; then the used idx — §2.7.8.2
    // "The device MUST set len prior to updating the used idx."
    let mut rest: &[u8] = &answer;
    for s in chain.iter().filter(|s| s.write) {
        let n = rest.len().min(s.bytes.len());
        mem[s.bytes.start..s.bytes.start + n].copy_from_slice(&rest[..n]);
        rest = &rest[n..];
    }
    let used_idx = rd16(mem, used.start + 2);
    let elem = used.start + 4 + 8 * usize::from(used_idx % q.size);
    let len = need as u32;
    mem[elem..elem + 4].copy_from_slice(&u32::from(head).to_le_bytes());
    mem[elem + 4..elem + 8].copy_from_slice(&len.to_le_bytes());
    mem[used.start + 2..used.start + 4].copy_from_slice(&used_idx.wrapping_add(1).to_le_bytes());
    *last_avail = last_avail.wrapping_add(1);
    Ok(Some((head, len)))
}
