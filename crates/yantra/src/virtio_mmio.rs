//! The virtio-mmio register window — `W-351`: the whole legacy register table, so a
//! driver can identify the device, set up its two queues and ring them.
//!
//! # What this is, and what it is not yet
//!
//! One LEGACY virtio-mmio transport (virtio v1.2 §4.2.4, "Legacy interface") in slot 0,
//! presenting a GPU (device ID 16, §5.7.1) with two queues, controlq and cursorq
//! (§5.7.2). Milestone 1 answered the identity registers and Status; milestone 2 (this)
//! adds the feature, page-size, queue and interrupt registers, and QueueNotify, which
//! hands the machine a queue to walk with `virtqueue::process` (`W-352`).
//!
//! WHAT A CHAIN GETS BACK IS NOT THIS FILE'S BUSINESS: the notify arm in `lib.rs` hands
//! each chain to the device model, `gpu.rs` (`W-378`), which this struct holds so that a
//! reset clears it with the queues.
//!
//! The device offers NO feature bits, so HostFeatures honestly reads 0: no
//! VIRTIO_F_INDIRECT_DESC (which is why `virtqueue::process` refuses one), no
//! VIRTIO_F_EVENT_IDX, no VIRTIO_GPU_F_*. A driver that acknowledges a bit it was not
//! offered is refused. Every offset the legacy table does not define is still REFUSED BY
//! NAME (`Halt::Device`), never answered with zero, and so is every write the table
//! forbids — each check runs BEFORE any register changes, so a refused write leaves the
//! device exactly as it was.
//!
//! LEGACY, not modern, by the 2026-10-01 ruling: the working driver is C-009
//! (`spec/virtio-gpu.sas`), which runs under QEMU on the legacy interface, and its `.t1`
//! port (`spec/virtio-gpu.t1`) talks the same register set. Version therefore reads 1.
//!
//! # Where the numbers come from
//!
//! Every offset and value is from the verbatim extracts under
//! `research/spec-extracts/virtio-1.2-csd01/`, not from memory:
//! - `4.2.4-mmio-legacy-interface.txt:21-38` — the legacy register table: MagicValue 0x000,
//!   Version 0x004 ("Legacy device returns value 0x1"), DeviceID 0x008, VendorID 0x00c,
//!   Status 0x070 (RW), and the write-only and not-yet-modelled registers listed below.
//! - `4.2.2-mmio-register-layout.txt:21` — MagicValue is 0x74726976, "virt" little endian.
//! - `4.2.2-mmio-register-layout.txt:85-89` — the driver MUST use 32-bit wide, aligned
//!   accesses for the control registers, and 32-bit accesses even for 64-bit config
//!   fields. So no register here needs a single 64-bit access, and the four-octet device
//!   seam `W-350` gave the compiler is the whole width this window has to honour.
//! - `5.7.1-5.7.3-gpu-id-queues-features.txt` — the GPU's device ID, 16, and its two
//!   queues, 0 controlq and 1 cursorq.
//! - `2.7-split-virtqueues.txt:49` — "Queue Size value is always a power of 2".
//! - `2.7.2-legacy-virtqueue-layout.txt` — the legacy layout: the descriptor table, the
//!   available ring right after it, padding to the next Queue Align boundary, then the
//!   used ring. That extract was taken 2026-10-03 from an HTML whose sha256 differs from
//!   the other extracts' (3fc4083b… against d3cfe908…); re-extracting all fourteen others
//!   from the new HTML reproduced every one of them byte for byte below its header, so the
//!   page's text did not change.
//!
//! # The address map, and the overlap it has with the file window
//!
//! C-009 (`spec/virtio-gpu.sas:67`) scans eight slots from `0x1000_1000` at a stride of
//! `0x1000` — QEMU riscv `virt`'s layout — so the slots cover `0x1000_1000..0x1000_9000`.
//! THE FILE WINDOW SITS INSIDE SLOT 1: `patra::PATRA_PATH` is `0x1000_2000`, slot 1's
//! base, and PUT ends at `0x1000_201f`. They coexist because `Machine::store` matches the
//! four patra addresses EXACTLY and BEFORE this window, so a store to them still reaches
//! the file window. Every other access in slot 1 — including any LOAD at a patra address,
//! which the file window never answered — is refused by name as the overlap. If a second
//! virtio device is ever wanted, it goes in slots 2..7, never 1; moving the file window is
//! the alternative, and that is an ABI change for every program that names it.

/// Slot 0's base. QEMU riscv `virt` and C-009 both start the virtio-mmio slots here.
pub const VIRTIO_MMIO_BASE: u64 = 0x1000_1000;
/// Distance between slots, as C-009 (`spec/virtio-gpu.sas:67`) scans them.
pub const VIRTIO_MMIO_STRIDE: u64 = 0x1000;
/// Number of slots C-009 scans. Slot 0 holds the GPU; slots 2..7 are empty; slot 1
/// overlaps the file window (see the module margin).
pub const VIRTIO_MMIO_SLOTS: u64 = 8;
/// The slot whose base is `patra::PATRA_PATH` — the overlap the margin describes.
pub const SLOT_OVERLAPPING_FILE_WINDOW: u64 = 1;

/// MagicValue, offset 0x000, read-only (§4.2.4 table; value from §4.2.2).
pub const MAGIC_VALUE: u64 = 0x000;
/// Version, offset 0x004, read-only. "Legacy device returns value 0x1."
pub const VERSION: u64 = 0x004;
/// DeviceID, offset 0x008, read-only.
pub const DEVICE_ID: u64 = 0x008;
/// VendorID, offset 0x00c, read-only.
pub const VENDOR_ID: u64 = 0x00c;
/// Status, offset 0x070, read-write. Writing zero resets the device (§4.2.4 table).
pub const STATUS: u64 = 0x070;

/// "virt" read as a little-endian 32-bit word (§4.2.2).
pub const MAGIC: u32 = 0x7472_6976;
/// The legacy interface's version number (§4.2.4).
pub const LEGACY_VERSION: u32 = 1;
/// virtio-gpu's device ID (§5.7.1).
pub const GPU_DEVICE_ID: u32 = 16;
/// The subsystem vendor ID. The specification prescribes no value, and no driver in this
/// tree reads it; it is "SANS" in ASCII, little endian, so a dump names this machine and
/// cannot be mistaken for QEMU's.
pub const VENDOR: u32 = 0x534E_4153;

/// The queues a GPU has (§5.7.2): 0 controlq, 1 cursorq.
pub const QUEUES: usize = 2;
/// QueueNumMax for each available queue. The specification sets no value; C-009 uses 8.
pub const QUEUE_NUM_MAX: u32 = 256;

/// HostFeatures, 0x010, read-only: the device's feature bits for the selected word.
pub const HOST_FEATURES: u64 = 0x010;
/// HostFeaturesSel, 0x014, write-only.
pub const HOST_FEATURES_SEL: u64 = 0x014;
/// GuestFeatures, 0x020, write-only: the bits the driver activated.
pub const GUEST_FEATURES: u64 = 0x020;
/// GuestFeaturesSel, 0x024, write-only.
pub const GUEST_FEATURES_SEL: u64 = 0x024;
/// GuestPageSize, 0x028, write-only: the page QueuePFN is counted in.
pub const GUEST_PAGE_SIZE: u64 = 0x028;
/// QueueSel, 0x030, write-only.
pub const QUEUE_SEL: u64 = 0x030;
/// QueueNumMax, 0x034, read-only.
pub const QUEUE_NUM_MAX_REG: u64 = 0x034;
/// QueueNum, 0x038, write-only.
pub const QUEUE_NUM: u64 = 0x038;
/// QueueAlign, 0x03c, write-only.
pub const QUEUE_ALIGN: u64 = 0x03c;
/// QueuePFN, 0x040, read-write.
pub const QUEUE_PFN: u64 = 0x040;
/// QueueNotify, 0x050, write-only.
pub const QUEUE_NOTIFY: u64 = 0x050;
/// InterruptStatus, 0x060, read-only.
pub const INTERRUPT_STATUS: u64 = 0x060;
/// InterruptACK, 0x064, write-only.
pub const INTERRUPT_ACK: u64 = 0x064;

/// The refusal for resizing or realigning a queue the device is already walking: its
/// rings would move under a cursor that still points into the old ones. The driver
/// writes QueuePFN 0 first, as §4.2.4 says it does when it stops using a queue.
const IN_USE: &str = "virtio-mmio QueueNum or QueueAlign written while the selected queue is in use \
                      (QueuePFN is nonzero); write QueuePFN 0 first";

/// One queue's registers as the driver set them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QueueRegs {
    /// QueueNum: entries in each ring; zero until the driver sets it.
    pub num: u32,
    /// QueueAlign: the used ring's alignment in bytes; zero until set.
    pub align: u32,
    /// QueuePFN: the queue's first page; zero means not in use.
    pub pfn: u32,
    /// The device's cursor into the available ring (`virtqueue::process`).
    pub last_avail: u16,
}

/// A QueueNotify whose chains the device has not served yet, under a DEFERRED completion
/// ([`VirtioMmio::defer`]): how many more instructions until it is, and the notifying
/// store's `pc` and address, which a refusal found when it is served still names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pending {
    /// Instructions still to begin before the chains are served; the completion lands
    /// in the step that brings this to zero, before that step's instruction.
    pub left: u64,
    /// The QueueNotify store's `pc`.
    pub pc: u64,
    /// The QueueNotify store's address.
    pub addr: u64,
}

/// The state of the one modelled device. Only what a driver can observe is kept, plus
/// each queue's available-ring cursor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VirtioMmio {
    /// The Status register as the driver last wrote it; zero after reset.
    pub status: u32,
    /// HostFeaturesSel.
    pub host_features_sel: u32,
    /// GuestFeaturesSel.
    pub guest_features_sel: u32,
    /// GuestPageSize; zero until the driver writes it.
    pub page_size: u32,
    /// QueueSel: the queue the next four queue registers apply to.
    pub queue_sel: u32,
    /// The two queues.
    pub queues: [QueueRegs; QUEUES],
    /// InterruptStatus: bit 0 is "used buffer notification" (§4.2.2).
    pub interrupt_status: u32,
    /// The device behind the controlq (`W-378`): its resources and scanout.
    pub gpu: crate::gpu::Gpu,
    /// THE DEFERRED COMPLETION: serve a notified queue's chains this many instructions
    /// after the QueueNotify store, not inside it. ZERO, the default, is the synchronous
    /// device every count in the tree was taken with.
    ///
    /// virtio 1.2 §2.7.13-14 does not promise a chain is used by the time the notify store
    /// returns, and QEMU's device completes it later (an ioeventfd and a bottom half). A
    /// driver that reads the used index ONCE after notifying therefore passes on a
    /// synchronous device and fails on QEMU: C-015's `spec/darshaka.t1` did exactly that
    /// (G 11 on QEMU 10.1) until it polled. With `defer` > 0 this machine fails it too.
    /// Set by `yantra-run` from `YANTRA_VIRTIO_DEFER`.
    pub defer: u64,
    /// Per queue, a notify not yet served (only when `defer` > 0). A second notify of a
    /// queue already waiting changes nothing: when the first is served, every chain
    /// offered by then is.
    pub pending: [Option<Pending>; QUEUES],
    /// How many entries of `pending` are `Some`: the per-step test is this one word.
    pub waiting: u32,
}

/// Which slot `addr` falls in and its offset there, or `None` outside the window.
#[must_use]
pub fn slot_of(addr: u64) -> Option<(u64, u64)> {
    let rel = addr.checked_sub(VIRTIO_MMIO_BASE)?;
    let slot = rel / VIRTIO_MMIO_STRIDE;
    (slot < VIRTIO_MMIO_SLOTS).then_some((slot, rel % VIRTIO_MMIO_STRIDE))
}

/// Why an access was refused, or `Ok` for slot 0 at a 32-bit aligned offset.
fn check(slot: u64, off: u64, width: usize) -> Result<(), &'static str> {
    if slot == SLOT_OVERLAPPING_FILE_WINDOW {
        return Err(
            "virtio-mmio slot 1 overlaps the file window (patra::PATRA_PATH is its base); \
             only the four patra stores are answered here and no virtio device sits in it",
        );
    }
    if slot != 0 {
        return Err("no virtio device in this virtio-mmio slot; the GPU is in slot 0");
    }
    if width != 4 || !off.is_multiple_of(4) {
        return Err(
            "virtio-mmio control registers take 32-bit aligned accesses only \
             (virtio 1.2 §4.2.2: the driver MUST use 32 bit wide and aligned accesses)",
        );
    }
    Ok(())
}

/// Where a queue's three areas lie under the legacy layout, as guest physical addresses:
/// `(desc, avail, used, end)`, `end` one past the used ring. `None` if `align` is not a
/// power of 2 — so 0 cannot underflow the mask and 3 cannot give a wrong one, whatever
/// the caller checked — or if any sum overflows.
///
/// `2.7.2-legacy-virtqueue-layout.txt`: the descriptor table (16 bytes per entry), the
/// available ring at once after it (flags, idx, ring[num], used_event: 2·(3 + num)
/// bytes), padding to the next Queue Align boundary, then the used ring (flags, idx,
/// {id, len}[num], avail_event: 6 + 8·num bytes).
#[must_use]
pub fn legacy_layout(pfn: u32, page: u32, num: u32, align: u32) -> Option<(u64, u64, u64, u64)> {
    if !align.is_power_of_two() {
        return None;
    }
    let desc = u64::from(pfn).checked_mul(u64::from(page))?;
    let avail = desc.checked_add(16 * u64::from(num))?;
    let avail_end = avail.checked_add(2 * (3 + u64::from(num)))?;
    let a = u64::from(align);
    let used = avail_end.checked_add(a - 1)? & !(a - 1);
    let end = used.checked_add(6 + 8 * u64::from(num))?;
    Some((desc, avail, used, end))
}

impl VirtioMmio {
    /// The queue QueueSel names, if it is one this device has.
    fn selected(&self) -> Option<usize> {
        let q = self.queue_sel as usize;
        (q < QUEUES).then_some(q)
    }

    /// A four-octet read at `off` in slot `slot`.
    ///
    /// # Errors
    /// The refusal, by name: a write-only register, an offset the table does not define,
    /// or QueueNumMax read while its queue is in use.
    pub fn read(&self, slot: u64, off: u64, width: usize) -> Result<u32, &'static str> {
        check(slot, off, width)?;
        match off {
            MAGIC_VALUE => Ok(MAGIC),
            VERSION => Ok(LEGACY_VERSION),
            DEVICE_ID => Ok(GPU_DEVICE_ID),
            VENDOR_ID => Ok(VENDOR),
            STATUS => Ok(self.status),
            // No feature bits are offered, in any word (see the margin).
            HOST_FEATURES => Ok(0),
            QUEUE_NUM_MAX_REG => match self.selected() {
                // "allowed only when QueuePFN is set to zero (0x0)" (4.2.4 table).
                Some(q) if self.queues[q].pfn != 0 => Err(
                    "virtio-mmio QueueNumMax read while the selected queue is in use \
                     (QueuePFN is nonzero); the legacy table allows it only when QueuePFN is 0",
                ),
                Some(_) => Ok(QUEUE_NUM_MAX),
                // "zero (0x0) if the queue is not available" — a real answer, not a gap.
                None => Ok(0),
            },
            QUEUE_PFN => Ok(self.selected().map_or(0, |q| self.queues[q].pfn)),
            INTERRUPT_STATUS => Ok(self.interrupt_status),
            HOST_FEATURES_SEL | GUEST_FEATURES | GUEST_FEATURES_SEL | GUEST_PAGE_SIZE
            | QUEUE_SEL | QUEUE_NUM | QUEUE_ALIGN | QUEUE_NOTIFY | INTERRUPT_ACK => Err(
                "read of a write-only virtio-mmio register (virtio 1.2 §4.2.4 table, direction W)",
            ),
            _ => Err(
                "no register at this virtio-mmio offset in the legacy table (virtio 1.2 §4.2.4)",
            ),
        }
    }

    /// A four-octet write of `value` at `off` in slot `slot`. `ram` is the guest physical
    /// range the machine has, so a queue placed outside it is refused at QueuePFN.
    ///
    /// Returns `Some(q)` when the write was QueueNotify for queue `q`, which the caller
    /// must then walk; `None` otherwise.
    ///
    /// # Errors
    /// The refusal, by name. Every check runs before any register changes.
    pub fn write(
        &mut self,
        slot: u64,
        off: u64,
        width: usize,
        value: u64,
        ram: std::ops::Range<u64>,
    ) -> Result<Option<usize>, &'static str> {
        check(slot, off, width)?;
        let v = value as u32;
        match off {
            MAGIC_VALUE | VERSION | DEVICE_ID | VENDOR_ID => Err(
                "write to a read-only virtio-mmio register (MagicValue, Version, DeviceID or VendorID)",
            ),
            HOST_FEATURES | QUEUE_NUM_MAX_REG | INTERRUPT_STATUS => Err(
                "write to a read-only virtio-mmio register (HostFeatures, QueueNumMax or InterruptStatus)",
            ),
            STATUS => {
                // Writing zero is the reset, and "the device sets QueuePFN to zero (0x0)
                // for all queues" (4.2.4 table). The cursors go with it, or a re-set-up
                // queue would skip the driver's first chains.
                // The GPU's resources and scanout go too — they are device state. Where
                // its picture is written is the HOST's setting, not the device's, so it
                // survives the reset; so does the completion delay (`defer`), while a
                // completion still pending is device state and is dropped with the queues.
                if v == 0 {
                    let dump = self.gpu.dump.take();
                    let defer = self.defer;
                    *self = Self::default();
                    self.gpu.dump = dump;
                    self.defer = defer;
                } else {
                    self.status = v;
                }
                Ok(None)
            }
            HOST_FEATURES_SEL => {
                self.host_features_sel = v;
                Ok(None)
            }
            GUEST_FEATURES_SEL => {
                self.guest_features_sel = v;
                Ok(None)
            }
            GUEST_FEATURES if v != 0 => Err(
                "virtio-mmio GuestFeatures acknowledges a feature bit this device does not offer \
                 (HostFeatures is 0 in every word)",
            ),
            GUEST_FEATURES => Ok(None),
            GUEST_PAGE_SIZE if !v.is_power_of_two() => {
                Err("virtio-mmio GuestPageSize must be a power of 2 (virtio 1.2 §4.2.4 table)")
            }
            // The same rule as IN_USE: every live queue's place was validated against
            // RAM with the page size it was set up under, and `queue` recomputes it.
            GUEST_PAGE_SIZE if self.queues.iter().any(|q| q.pfn != 0) => Err(
                "virtio-mmio GuestPageSize written while a queue is in use (a QueuePFN is nonzero); \
                 write QueuePFN 0 first",
            ),
            GUEST_PAGE_SIZE => {
                self.page_size = v;
                Ok(None)
            }
            QUEUE_SEL => {
                self.queue_sel = v;
                Ok(None)
            }
            QUEUE_NUM => {
                let Some(q) = self.selected() else {
                    return Err(
                        "virtio-mmio QueueNum for a queue this device does not have (QueueNumMax reads 0)",
                    );
                };
                if self.queues[q].pfn != 0 {
                    return Err(IN_USE);
                }
                if v == 0 || !v.is_power_of_two() || v > QUEUE_NUM_MAX {
                    return Err(
                        "virtio-mmio QueueNum must be a power of 2, at least 1 and at most QueueNumMax \
                         (virtio 1.2 §2.7: \"Queue Size value is always a power of 2\")",
                    );
                }
                self.queues[q].num = v;
                Ok(None)
            }
            QUEUE_ALIGN => {
                let Some(q) = self.selected() else {
                    return Err("virtio-mmio QueueAlign for a queue this device does not have");
                };
                if self.queues[q].pfn != 0 {
                    return Err(IN_USE);
                }
                if !v.is_power_of_two() {
                    return Err(
                        "virtio-mmio QueueAlign must be a power of 2 (virtio 1.2 §4.2.4 table)",
                    );
                }
                self.queues[q].align = v;
                Ok(None)
            }
            QUEUE_PFN => {
                let Some(q) = self.selected() else {
                    return Err("virtio-mmio QueuePFN for a queue this device does not have");
                };
                if v == 0 {
                    // "When the driver stops using the queue it writes zero."
                    self.queues[q].pfn = 0;
                    self.queues[q].last_avail = 0;
                    return Ok(None);
                }
                if self.page_size == 0 {
                    return Err(
                        "virtio-mmio QueuePFN written before GuestPageSize: the page it counts in is unknown \
                         (virtio 1.2 §4.2.4: the driver writes GuestPageSize before any queues are used)",
                    );
                }
                let r = &self.queues[q];
                if r.num == 0 || r.align == 0 {
                    return Err(
                        "virtio-mmio QueuePFN written before QueueNum and QueueAlign (§4.2.4 steps 5-7)",
                    );
                }
                match legacy_layout(v, self.page_size, r.num, r.align) {
                    Some((desc, _, _, end)) if desc >= ram.start && end <= ram.end => {
                        self.queues[q].pfn = v;
                        self.queues[q].last_avail = 0;
                        Ok(None)
                    }
                    _ => Err(
                        "virtio-mmio QueuePFN places the queue's three areas outside RAM \
                         (or past the end of the address space)",
                    ),
                }
            }
            QUEUE_NOTIFY => {
                let q = v as usize;
                if q >= QUEUES {
                    return Err("virtio-mmio QueueNotify names a queue this device does not have");
                }
                if self.queues[q].pfn == 0 {
                    return Err(
                        "virtio-mmio QueueNotify for a queue that is not set up (its QueuePFN is 0)",
                    );
                }
                Ok(Some(q))
            }
            INTERRUPT_ACK => {
                self.interrupt_status &= !v;
                Ok(None)
            }
            _ => Err(
                "no register at this virtio-mmio offset in the legacy table (virtio 1.2 §4.2.4)",
            ),
        }
    }

    /// Queue `q` as `virtqueue::process` takes it. Only called after QueueNotify
    /// accepted `q`, so its PFN, page size, size and alignment are set and in RAM.
    #[must_use]
    pub fn queue(&self, q: usize) -> crate::virtqueue::Queue {
        let r = &self.queues[q];
        let (desc, avail, used, _) =
            legacy_layout(r.pfn, self.page_size, r.num, r.align).unwrap_or((0, 0, 0, 0));
        crate::virtqueue::Queue {
            size: r.num as u16,
            desc,
            avail,
            used,
        }
    }
}
