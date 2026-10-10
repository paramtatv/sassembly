//! **THE NETWORK FRAME DEVICE (ADR-0047, PROPOSED): a virtio-net device that moves raw
//! Ethernet FRAMES and nothing else.**
//!
//! # The rule: frames only
//!
//! The owner's ruling ("a-fully pure", 2026-10-07): this device moves frames. It has no
//! socket protocol logic, no TLS, no NAT and no TCP/IP, and none may be added here — the
//! whole stack above the frame is the guest's, written later in `.t1`. Nothing in this file
//! names the host network (`tests/w377_sockets.rs`'s ratchet holds `std::net` to
//! `bin/yantra-run.rs`) or reads a clock (`tests/w375_clock.rs`); the backends live in
//! `bin/yantra-run.rs` behind the [`FrameBackend`] trait.
//!
//! # What the guest sees: a LEGACY virtio-net device in slot 2
//!
//! The same register table as `virtio_mmio` (legacy, version 1), device ID 1 (virtio 1.2
//! §5.1.1), so the `.t1` driver that later runs on QEMU's virtio-net runs here unchanged:
//!
//! - THREE feature bits, standard virtio-net (QEMU parity), all in HostFeatures word 0:
//!   `VIRTIO_NET_F_MTU` (3), `VIRTIO_NET_F_MAC` (5), `VIRTIO_NET_F_STATUS` (16). No
//!   checksum or segmentation offload, no `MRG_RXBUF`, no control queue, no `MQ`, no
//!   `VERSION_1`. A driver that acknowledges any other bit is refused, by name.
//! - config space at `0x100` (§5.1.4): `mac[6]`, `status` (u16, `LINK_UP` = 1 always),
//!   `max_virtqueue_pairs` (u16, reads 0: no `MQ`), `mtu` (u16, `max_frame - 14`). Read-only.
//! - queue 0 `receiveq`, queue 1 `transmitq` (§5.1.2); QueueNumMax 256.
//! - every buffer begins with the 10-octet legacy `virtio_net_hdr` (§5.1.6.1). TX: the
//!   device ignores its contents. RX: the device writes it zero.
//!
//! Slot 2 because slot 0 is the GPU and slot 1 overlaps the file window (`virtio_mmio`'s
//! margin). A guest finds it as it finds QEMU's: scan the slots for DeviceID 1.
//!
//! # When a frame moves
//!
//! THE DRIVER MUST POST RX BUFFERS BEFORE IT WAITS. A frame is placed in a buffer the guest
//! has already made available; with none posted, a wait delivers `n=none` at once (the host
//! does not block for a frame that could only be dropped), and a buffer smaller than
//! 10 + the frame drops the frame. `YANTRA_VIRTIO_DEFER=n` defers BOTH completions by n
//! instructions, as the GPU's: a TX chain is used n instructions after QueueNotify (or at the
//! next WAIT or the end of the run, whichever is first, so the frame still leaves), and an RX
//! frame is placed n instructions after the wait that delivered it. A driver that reads a
//! used index once, right after notifying or waiting, passes the synchronous device and fails
//! this one; poll. Under a deferral the event word holds the frame's length when the frame
//! was ACCEPTED (queue set up, not filtered), because whether a buffer fits is decided later.
//!
//!
//! TX happens at QueueNotify of queue 1: each chain's frame is validated and queued in
//! [`VirtioNet::outbox`], which the HOST drains at the next WAIT or at the end of the run.
//! RX happens ONLY AT A WAIT: the host delivers at most one frame per `Halt::Wait` with
//! [`deliver`], so the count of instructions retired is a function of `(program, log)`.
//! The `SASEVENT` word receives the number of frame octets placed in an RX buffer, 0 if the
//! frame was dropped (no buffer posted, a buffer smaller than 10 + the frame, or the MAC
//! filter). A live wait BLOCKS until a frame arrives (no default timeout); `--net-timeout`
//! makes expiry a refusal, [`NET_TIMEOUT`].
//!
//! # The frame log
//!
//! ```text
//!   # yantra-net v1 mac=.. max-frame=1514 max-tx=65536 max-rx=65536 filter=0 ...   (header)
//!   n=<wait>:<handle>:<lowercase hex of one frame>      delivered at wait <wait>
//!   n=<wait>:<handle>:none                              nothing arrived at that wait
//!   @<thread>                                           (threaded runs only) run thread N
//! ```
//!
//! `<wait>` is the ordinal of the wait over the whole run, from 0, and `<handle>` the
//! receiving handle, 0 for the one device (a later row's connections get 1, 2, ...). Both are
//! CHECKED when the log is read, so a spliced or reordered log is refused. In a threaded run
//! an `@N` line precedes the `n=` record of the thread N that is resuming from a wait, as
//! in `threads.rs`'s `@N` / value pairs; the frame goes to that thread.
//!
//! # Refusals, by name
//!
//! The names below are the owner's (rulings 2026-10-08, ADR-0048): the Devanagari name and its code lead
//! the message, and [`RULED_REFUSALS`] lists every ruled `(code, name)` pair.

use crate::input::ThreadRecord;
use crate::virtio_mmio::{
    DEVICE_ID, GUEST_FEATURES, GUEST_FEATURES_SEL, GUEST_PAGE_SIZE, HOST_FEATURES,
    HOST_FEATURES_SEL, INTERRUPT_ACK, INTERRUPT_STATUS, LEGACY_VERSION, MAGIC, MAGIC_VALUE,
    QUEUE_ALIGN, QUEUE_NOTIFY, QUEUE_NUM, QUEUE_NUM_MAX, QUEUE_NUM_MAX_REG, QUEUE_PFN, QUEUE_SEL,
    QueueRegs, STATUS, VENDOR, VENDOR_ID, VERSION, legacy_layout,
};
use crate::{Halt, Machine, Output, virtqueue};

/// The slot the device occupies.
pub const NET_SLOT: u64 = 2;
/// virtio-net's device ID (§5.1.1).
pub const NET_DEVICE_ID: u32 = 1;
/// `VIRTIO_NET_F_MTU` (§5.1.3): the config space carries the MTU. Bit 3.
pub const F_MTU: u32 = 1 << 3;
/// `VIRTIO_NET_F_MAC`: the config space carries a MAC. Bit 5.
pub const F_MAC: u32 = 1 << 5;
/// `VIRTIO_NET_F_STATUS`: the config space carries a link status. Bit 16.
pub const F_STATUS: u32 = 1 << 16;
/// Every bit the device offers, in word 0.
pub const OFFERED: u32 = F_MTU | F_MAC | F_STATUS;
/// `VIRTIO_NET_S_LINK_UP`.
pub const LINK_UP: u16 = 1;
/// The legacy `virtio_net_hdr` without `num_buffers` (§5.1.6.1): 10 octets.
pub const HDR_LEN: usize = 10;
/// Queue 0.
pub const RECEIVEQ: usize = 0;
/// Queue 1.
pub const TRANSMITQ: usize = 1;
/// The queues the device has.
pub const QUEUES: usize = 2;
/// Config space offset.
pub const CONFIG: u64 = 0x100;
/// The config space's length: mac (6), status (2), max_virtqueue_pairs (2), mtu (2).
pub const CONFIG_LEN: u64 = 12;
/// An Ethernet header is 14 octets; a shorter frame is not a frame.
pub const MIN_FRAME: usize = 14;
/// `--net-max-frame`'s default: a 1500-octet payload and the 14-octet header.
pub const DEFAULT_MAX_FRAME: usize = 1514;
/// `--net-timeout`'s default, in milliseconds: a live wait that hears nothing for this long
/// halts with `NET_TIMEOUT` instead of hanging.
pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
/// The ceiling `--net-max-frame` itself may not pass.
pub const HARD_MAX_FRAME: usize = 65_535;
/// The per-run cap's default, for TX frames and, separately, for RX frames.
pub const DEFAULT_MAX_FRAMES: u64 = 65_536;
/// The default MAC: locally administered (bit 1 of octet 0), "SANS" in the rest.
pub const DEFAULT_MAC: [u8; 6] = [0x02, 0x53, 0x41, 0x4e, 0x53, 0x00];

/// The ruled refusals, `(code, name)`. A refusal word is `code << 16 | 0x3333` (ADR-0048). Codes
/// 0x35f to 0x36d and 0x371; the device raises 0x35f, 0x360, 0x361, 0x363 and 0x368 to 0x36c, the `.t1` stack
/// 0x362, 0x363, 0x364, 0x36d and 0x371.
pub const RULED_REFUSALS: [(u64, &str); 16] = [
    (0x35f, "सञ्चाराभावः"),
    (0x360, "पिण्डातिदीर्घनिषेधः"),
    (0x361, "पिण्डसङ्ख्यातिक्रमः"),
    (0x362, "परीक्षायोगभेदः"),
    (0x363, "पिण्डावैधरूपम्"),
    (0x364, "स्थाननिर्देशकालातीतम्"),
    (0x365, "नामानुपलब्धिः"),
    (0x366, "अवैधसन्धिः"),
    (0x367, "लक्ष्यनिषेधः"),
    (0x368, "उद्गमसम्बन्धनिषेधः"),
    (0x369, "सञ्चारकालातीतम्"),
    (0x36a, "प्रतिपक्षविच्छिन्नम्"),
    (0x36b, "आधारानुपलब्धम्"),
    (0x36c, "अभिलेखसङ्केतभेदः"),
    (0x36d, "प्रतिध्वन्यभावः"),
    (0x371, "उत्तरखण्डितम्"),
];

/// Reserved for Phase 2, ruled 2026-10-08, raised by nothing yet: connection refused, TLS
/// verification failed, record too large.
pub const RESERVED_REFUSALS: [(u64, &str); 3] = [
    (0x36e, "सन्धिप्रत्याख्यानम्"),
    (0x36f, "प्रमाणपत्रनिषेधः"),
    (0x370, "भारातिक्रमनिषेधः"),
];

/// The window touched with no `--net-*` flag. Ruled: 0x35f.
pub const NET_WINDOW_REFUSED: &str = "सञ्चाराभावः (0x35f): no virtio device in this virtio-mmio slot \
     (slot 2 is the network device, present only when a --net-* flag asks for one)";
/// A frame longer than the cap. Ruled: 0x360.
pub const NET_FRAME_TOO_LARGE: &str =
    "पिण्डातिदीर्घनिषेधः (0x360): a frame is longer than --net-max-frame (default 1514 octets)";
/// More TX frames than the cap. Ruled: 0x361 (frame-cap exceeded).
pub const NET_TX_COUNT_EXCEEDED: &str =
    "पिण्डसङ्ख्यातिक्रमः (0x361): the run has transmitted more frames than --net-max-tx-frames";
/// More RX frames than the cap. Ruled: 0x361 (frame-cap exceeded).
pub const NET_RX_COUNT_EXCEEDED: &str =
    "पिण्डसङ्ख्यातिक्रमः (0x361): the run has received more frames than --net-max-rx-frames";
/// A frame shorter than an Ethernet header. Ruled: 0x363 (malformed packet).
pub const NET_FRAME_RUNT: &str =
    "पिण्डावैधरूपम् (0x363): a frame is shorter than an Ethernet header (14 octets)";
/// A TX buffer shorter than the 10-octet header. Ruled: 0x363 (malformed packet).
pub const NET_TX_NO_HEADER: &str =
    "पिण्डावैधरूपम् (0x363): a transmit buffer is shorter than the 10-octet virtio_net_hdr";
/// A TX frame whose source MAC is not the device's, under `--net-mac`. Ruled: 0x368.
pub const NET_SRC_MAC_REFUSED: &str = "उद्गमसम्बन्धनिषेधः (0x368): a transmitted frame's source MAC is not the --net-mac allow-listed one";
/// A live wait that heard no frame within `--net-timeout`. Ruled: 0x369.
pub const NET_TIMEOUT: &str = "सञ्चारकालातीतम् (0x369): no frame arrived within --net-timeout";
/// The peer went away. Ruled: 0x36a.
pub const NET_PEER_CLOSED: &str = "प्रतिपक्षविच्छिन्नम् (0x36a): the frame peer closed the connection";
/// A backend this platform or build does not have. Ruled: 0x36b.
pub const NET_BACKEND_UNAVAILABLE: &str = "आधारानुपलब्धम् (0x36b)";
/// A log record whose wait index or handle does not match its place. Ruled: 0x36c.
pub const NET_LOG_TAG_MISMATCH: &str =
    "अभिलेखसङ्केतभेदः (0x36c): a net record's wait index or handle does not match its place";

const WHY_WIDTH: &str = "virtio-net control registers take 32-bit aligned accesses only \
     (virtio 1.2 §4.2.2); its config space takes 1, 2 or 4 octets within its 12 octets";
const WHY_READ_W: &str =
    "read of a write-only virtio-mmio register (virtio 1.2 §4.2.4 table, direction W)";
const WHY_WRITE_R: &str = "write to a read-only virtio-mmio register of the virtio-net device";
const WHY_NO_REG: &str =
    "no register at this virtio-mmio offset in the legacy table (virtio 1.2 §4.2.4)";
const WHY_FEATURE: &str = "virtio-net GuestFeatures acknowledges a feature bit this device does not offer \
     (it offers VIRTIO_NET_F_MTU, _MAC and _STATUS only)";
const WHY_IN_USE: &str = "virtio-net QueueNum, QueueAlign or GuestPageSize written while a queue is in use \
     (QueuePFN is nonzero); write QueuePFN 0 first";

/// The device's configuration: what a log header carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetConfig {
    /// The MAC the config space reads.
    pub mac: [u8; 6],
    /// Longest frame, in octets, either direction; the MTU is this less 14.
    pub max_frame: usize,
    /// Most frames the guest may transmit in one run.
    pub max_tx: u64,
    /// Most frames the host may offer in one run.
    pub max_rx: u64,
    /// The optional MAC allow-list (`--net-mac`): TX frames must carry `mac` as their
    /// source, and RX frames not addressed to `mac`, broadcast or a group are dropped.
    pub mac_filter: bool,
    /// Instructions between a notify (TX) or a delivery (RX) and its completion; 0 is
    /// synchronous. From `YANTRA_VIRTIO_DEFER`; carried in the log header.
    pub defer: u64,
    /// Which host backend the live run used; carried in the log header.
    pub backend: BackendKind,
}

/// The host backend a live run used, recorded in the log header for the reader's sake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    /// None: a replay, or a test's scripted backend.
    None,
    /// `--net-peer`.
    Peer,
    /// `--net-tap`.
    Tap,
}

impl BackendKind {
    /// The word in the header.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Peer => "peer",
            Self::Tap => "tap",
        }
    }
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            mac: DEFAULT_MAC,
            max_frame: DEFAULT_MAX_FRAME,
            max_tx: DEFAULT_MAX_FRAMES,
            max_rx: DEFAULT_MAX_FRAMES,
            mac_filter: false,
            defer: 0,
            backend: BackendKind::None,
        }
    }
}

/// A TX notify not yet served under a deferral: instructions left, and the store's `pc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingTx {
    /// Instructions still to begin before it is served.
    pub left: u64,
    /// The QueueNotify store's `pc`.
    pub pc: u64,
}

/// `02:53:41:4e:53:00`.
#[must_use]
pub fn mac_text(mac: &[u8; 6]) -> String {
    mac.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

/// Read `xx:xx:xx:xx:xx:xx`.
///
/// # Errors
/// Naming what is wrong.
pub fn parse_mac(s: &str) -> Result<[u8; 6], String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 6 {
        return Err(format!(
            "MAC {s:?}: six hex octets separated by ':' expected"
        ));
    }
    let mut mac = [0u8; 6];
    for (i, p) in parts.iter().enumerate() {
        if p.len() != 2 {
            return Err(format!("MAC {s:?}: octet {p:?} is not two hex digits"));
        }
        mac[i] = u8::from_str_radix(p, 16).map_err(|e| format!("MAC {s:?}: octet {p:?}: {e}"))?;
    }
    Ok(mac)
}

/// The device's state: only what a driver can observe, the cursors, and the counters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtioNet {
    /// MAC, frame cap, frame-count caps and the filter.
    pub config: NetConfig,
    /// Status as the driver last wrote it; zero after reset.
    pub status: u32,
    /// HostFeaturesSel.
    pub host_features_sel: u32,
    /// GuestFeaturesSel.
    pub guest_features_sel: u32,
    /// The feature bits (word 0) the driver acknowledged.
    pub driver_features: u32,
    /// GuestPageSize.
    pub page_size: u32,
    /// QueueSel.
    pub queue_sel: u32,
    /// receiveq and transmitq.
    pub queues: [QueueRegs; QUEUES],
    /// InterruptStatus; bit 0 is a used-buffer notification.
    pub interrupt_status: u32,
    /// Frames the guest transmitted that the host has not yet taken ([`take_tx`]).
    pub outbox: Vec<Vec<u8>>,
    /// Frames transmitted, over the run.
    pub tx_frames: u64,
    /// Frames the host offered, delivered, dropped or filtered, over the run.
    pub rx_seen: u64,
    /// Frames placed in an RX buffer.
    pub rx_frames: u64,
    /// Frames dropped: no buffer posted, or one too small.
    pub rx_dropped: u64,
    /// Frames the MAC allow-list dropped.
    pub rx_filtered: u64,
    /// A TX notify waiting out its deferral.
    pub pending_tx: Option<PendingTx>,
    /// RX frames accepted and waiting out their deferral: `(instructions left, frame)`.
    pub staged: Vec<(u64, Vec<u8>)>,
}

impl VirtioNet {
    /// A device with `config`.
    #[must_use]
    pub fn new(config: NetConfig) -> Self {
        Self {
            config,
            status: 0,
            host_features_sel: 0,
            guest_features_sel: 0,
            driver_features: 0,
            page_size: 0,
            queue_sel: 0,
            queues: Default::default(),
            interrupt_status: 0,
            outbox: Vec::new(),
            tx_frames: 0,
            rx_seen: 0,
            rx_frames: 0,
            rx_dropped: 0,
            rx_filtered: 0,
            pending_tx: None,
            staged: Vec::new(),
        }
    }

    /// Completions waiting out a deferral (the machine's per-step test counts them).
    #[must_use]
    pub fn waiting(&self) -> u32 {
        u32::from(self.pending_tx.is_some()) + self.staged.len() as u32
    }

    /// The MTU the config space reports: the frame cap less the Ethernet header.
    #[must_use]
    pub fn mtu(&self) -> u16 {
        (self.config.max_frame - MIN_FRAME) as u16
    }

    fn config_space(&self) -> [u8; CONFIG_LEN as usize] {
        let mut c = [0u8; CONFIG_LEN as usize];
        c[..6].copy_from_slice(&self.config.mac);
        c[6..8].copy_from_slice(&LINK_UP.to_le_bytes());
        // 8..10 max_virtqueue_pairs: VIRTIO_NET_F_MQ is not offered, so 0.
        c[10..12].copy_from_slice(&self.mtu().to_le_bytes());
        c
    }

    fn selected(&self) -> Option<usize> {
        let q = self.queue_sel as usize;
        (q < QUEUES).then_some(q)
    }

    fn check(off: u64, width: usize) -> Result<(), &'static str> {
        if off >= CONFIG {
            if off + width as u64 <= CONFIG + CONFIG_LEN && matches!(width, 1 | 2 | 4) {
                return Ok(());
            }
            return Err(WHY_WIDTH);
        }
        if width != 4 || !off.is_multiple_of(4) {
            return Err(WHY_WIDTH);
        }
        Ok(())
    }

    /// A read at `off` in the slot.
    ///
    /// # Errors
    /// The refusal, by name.
    pub fn read(&self, off: u64, width: usize) -> Result<u32, &'static str> {
        Self::check(off, width)?;
        if off >= CONFIG {
            let at = (off - CONFIG) as usize;
            let c = self.config_space();
            let mut v = 0u32;
            for i in 0..width {
                v |= u32::from(c[at + i]) << (8 * i);
            }
            return Ok(v);
        }
        match off {
            MAGIC_VALUE => Ok(MAGIC),
            VERSION => Ok(LEGACY_VERSION),
            DEVICE_ID => Ok(NET_DEVICE_ID),
            VENDOR_ID => Ok(VENDOR),
            STATUS => Ok(self.status),
            HOST_FEATURES => Ok(if self.host_features_sel == 0 {
                OFFERED
            } else {
                0
            }),
            QUEUE_NUM_MAX_REG => match self.selected() {
                Some(q) if self.queues[q].pfn != 0 => Err(
                    "virtio-net QueueNumMax read while the selected queue is in use \
                     (QueuePFN is nonzero)",
                ),
                Some(_) => Ok(QUEUE_NUM_MAX),
                None => Ok(0),
            },
            QUEUE_PFN => Ok(self.selected().map_or(0, |q| self.queues[q].pfn)),
            INTERRUPT_STATUS => Ok(self.interrupt_status),
            HOST_FEATURES_SEL | GUEST_FEATURES | GUEST_FEATURES_SEL | GUEST_PAGE_SIZE
            | QUEUE_SEL | QUEUE_NUM | QUEUE_ALIGN | QUEUE_NOTIFY | INTERRUPT_ACK => Err(WHY_READ_W),
            _ => Err(WHY_NO_REG),
        }
    }

    /// A write at `off`; `Some(q)` is a QueueNotify the caller must [`Self::notify`].
    ///
    /// # Errors
    /// The refusal, by name; every check runs before any register changes.
    pub fn write(
        &mut self,
        off: u64,
        width: usize,
        value: u64,
        ram: std::ops::Range<u64>,
    ) -> Result<Option<usize>, &'static str> {
        Self::check(off, width)?;
        if off >= CONFIG {
            return Err("write to the virtio-net config space: it is read-only \
                        (virtio 1.2 §5.1.4: the driver MUST NOT write the MAC)");
        }
        let v = value as u32;
        match off {
            MAGIC_VALUE | VERSION | DEVICE_ID | VENDOR_ID | HOST_FEATURES | QUEUE_NUM_MAX_REG
            | INTERRUPT_STATUS => Err(WHY_WRITE_R),
            STATUS => {
                if v == 0 {
                    // Reset: the registers and cursors go; the configuration, the counters
                    // and the frames not yet taken are the host's, and stay.
                    let mut fresh = Self::new(self.config);
                    fresh.outbox = std::mem::take(&mut self.outbox);
                    (fresh.tx_frames, fresh.rx_seen, fresh.rx_frames) =
                        (self.tx_frames, self.rx_seen, self.rx_frames);
                    (fresh.rx_dropped, fresh.rx_filtered) = (self.rx_dropped, self.rx_filtered);
                    *self = fresh;
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
            GUEST_FEATURES => {
                let offered = if self.guest_features_sel == 0 {
                    OFFERED
                } else {
                    0
                };
                if v & !offered != 0 {
                    return Err(WHY_FEATURE);
                }
                if self.guest_features_sel == 0 {
                    self.driver_features = v;
                }
                Ok(None)
            }
            GUEST_PAGE_SIZE if !v.is_power_of_two() => {
                Err("virtio-net GuestPageSize must be a power of 2 (virtio 1.2 §4.2.4 table)")
            }
            GUEST_PAGE_SIZE if self.queues.iter().any(|q| q.pfn != 0) => Err(WHY_IN_USE),
            GUEST_PAGE_SIZE => {
                self.page_size = v;
                Ok(None)
            }
            QUEUE_SEL => {
                self.queue_sel = v;
                Ok(None)
            }
            QUEUE_NUM => {
                let q = self
                    .selected()
                    .ok_or("virtio-net QueueNum for a queue this device does not have")?;
                if self.queues[q].pfn != 0 {
                    return Err(WHY_IN_USE);
                }
                if v == 0 || !v.is_power_of_two() || v > QUEUE_NUM_MAX {
                    return Err(
                        "virtio-net QueueNum must be a power of 2, at least 1 and at most QueueNumMax",
                    );
                }
                self.queues[q].num = v;
                Ok(None)
            }
            QUEUE_ALIGN => {
                let q = self
                    .selected()
                    .ok_or("virtio-net QueueAlign for a queue this device does not have")?;
                if self.queues[q].pfn != 0 {
                    return Err(WHY_IN_USE);
                }
                if !v.is_power_of_two() {
                    return Err(
                        "virtio-net QueueAlign must be a power of 2 (virtio 1.2 §4.2.4 table)",
                    );
                }
                self.queues[q].align = v;
                Ok(None)
            }
            QUEUE_PFN => {
                let q = self
                    .selected()
                    .ok_or("virtio-net QueuePFN for a queue this device does not have")?;
                if v == 0 {
                    self.queues[q].pfn = 0;
                    self.queues[q].last_avail = 0;
                    return Ok(None);
                }
                if self.page_size == 0 {
                    return Err("virtio-net QueuePFN written before GuestPageSize");
                }
                let r = &self.queues[q];
                if r.num == 0 || r.align == 0 {
                    return Err("virtio-net QueuePFN written before QueueNum and QueueAlign");
                }
                match legacy_layout(v, self.page_size, r.num, r.align) {
                    Some((desc, _, _, end)) if desc >= ram.start && end <= ram.end => {
                        self.queues[q].pfn = v;
                        self.queues[q].last_avail = 0;
                        Ok(None)
                    }
                    _ => Err("virtio-net QueuePFN places the queue's areas outside RAM"),
                }
            }
            QUEUE_NOTIFY => {
                let q = v as usize;
                if q >= QUEUES {
                    return Err("virtio-net QueueNotify names a queue this device does not have");
                }
                if self.queues[q].pfn == 0 {
                    return Err(
                        "virtio-net QueueNotify for a queue that is not set up (its QueuePFN is 0)",
                    );
                }
                Ok(Some(q))
            }
            INTERRUPT_ACK => {
                self.interrupt_status &= !v;
                Ok(None)
            }
            _ => Err(WHY_NO_REG),
        }
    }

    fn queue(&self, q: usize) -> virtqueue::Queue {
        let r = &self.queues[q];
        let (desc, avail, used, _) =
            legacy_layout(r.pfn, self.page_size, r.num, r.align).unwrap_or((0, 0, 0, 0));
        virtqueue::Queue {
            size: r.num as u16,
            desc,
            avail,
            used,
        }
    }

    /// QueueNotify of queue `q`. Queue 1 is walked: every offered chain is a frame to send.
    /// Queue 0 only posts buffers, which RX will find at the next WAIT.
    ///
    /// # Errors
    /// The refusal, by name. Frames accepted before it stay in the outbox.
    pub fn notify(
        &mut self,
        q: usize,
        mem: &mut [u8],
        base: u64,
        pc: u64,
    ) -> Result<(), &'static str> {
        if q == TRANSMITQ && self.config.defer > 0 {
            // COALESCING: a second notify while one is pending changes nothing. That is
            // safe because the deferred serve (`notify_now`) drains EVERY descriptor the
            // driver has made available by then, not just the first one's.
            if self.pending_tx.is_none() {
                self.pending_tx = Some(PendingTx {
                    left: self.config.defer,
                    pc,
                });
            }
            return Ok(());
        }
        self.notify_now(q, mem, base)
    }

    fn notify_now(&mut self, q: usize, mem: &mut [u8], base: u64) -> Result<(), &'static str> {
        if q != TRANSMITQ {
            return Ok(());
        }
        let queue = self.queue(q);
        let mut cursor = self.queues[q].last_avail;
        let mut fresh: Vec<Vec<u8>> = Vec::new();
        let mut why: Option<&'static str> = None;
        let c = self.config;
        let already = self.tx_frames;
        let mut served = false;
        while why.is_none() {
            let r = virtqueue::process(mem, base, &queue, &mut cursor, |_, req| {
                if req.len() < HDR_LEN {
                    why = Some(NET_TX_NO_HEADER);
                } else {
                    let frame = &req[HDR_LEN..];
                    if frame.len() < MIN_FRAME {
                        why = Some(NET_FRAME_RUNT);
                    } else if frame.len() > c.max_frame {
                        why = Some(NET_FRAME_TOO_LARGE);
                    } else if already + fresh.len() as u64 >= c.max_tx {
                        why = Some(NET_TX_COUNT_EXCEEDED);
                    } else if c.mac_filter && frame[6..12] != c.mac {
                        why = Some(NET_SRC_MAC_REFUSED);
                    } else {
                        fresh.push(frame.to_vec());
                    }
                }
                Vec::new()
            });
            match r {
                Ok(Some(_)) => served = true,
                Ok(None) => break,
                Err(e) => why = Some(e.name()),
            }
        }
        self.queues[q].last_avail = cursor;
        self.tx_frames += fresh.len() as u64;
        self.outbox.extend(fresh);
        if served {
            self.interrupt_status |= 1;
        }
        why.map_or(Ok(()), Err)
    }

    /// Offer one received frame to the guest: the caps are checked, the allow-list applied,
    /// then it is placed in the next receive buffer. Answers the octets placed (0 if dropped).
    ///
    /// # Errors
    /// A cap or a malformed ring, by name. Nothing is counted for a refused frame.
    pub fn receive(&mut self, mem: &mut [u8], base: u64, frame: &[u8]) -> Result<u64, String> {
        if frame.len() < MIN_FRAME {
            return Err(format!("{NET_FRAME_RUNT} ({} octets)", frame.len()));
        }
        if frame.len() > self.config.max_frame {
            return Err(format!(
                "{NET_FRAME_TOO_LARGE} ({} octets against {})",
                frame.len(),
                self.config.max_frame
            ));
        }
        if self.rx_seen >= self.config.max_rx {
            return Err(format!(
                "{NET_RX_COUNT_EXCEEDED} (the cap is {})",
                self.config.max_rx
            ));
        }
        self.rx_seen += 1;
        if self.config.mac_filter && frame[..6] != self.config.mac && frame[0] & 1 == 0 {
            self.rx_filtered += 1;
            return Ok(0);
        }
        if self.queues[RECEIVEQ].pfn == 0 {
            self.rx_dropped += 1;
            return Ok(0);
        }
        if self.config.defer > 0 {
            self.staged.push((self.config.defer, frame.to_vec()));
            return Ok(frame.len() as u64);
        }
        self.place(mem, base, frame).map_err(str::to_string)
    }

    /// Put `frame` in the next receive buffer: answers the octets placed, 0 if dropped.
    fn place(&mut self, mem: &mut [u8], base: u64, frame: &[u8]) -> Result<u64, &'static str> {
        if self.queues[RECEIVEQ].pfn == 0 {
            self.rx_dropped += 1;
            return Ok(0);
        }
        let queue = self.queue(RECEIVEQ);
        let mut cursor = self.queues[RECEIVEQ].last_avail;
        let mut answer = vec![0u8; HDR_LEN];
        answer.extend_from_slice(frame);
        let r = virtqueue::process(mem, base, &queue, &mut cursor, |_, _| answer.clone());
        match r {
            Ok(Some(_)) => {
                self.queues[RECEIVEQ].last_avail = cursor;
                self.rx_frames += 1;
                self.interrupt_status |= 1;
                Ok(frame.len() as u64)
            }
            Ok(None) | Err(virtqueue::Refusal::ResponseTooLong { .. }) => {
                self.rx_dropped += 1;
                Ok(0)
            }
            Err(e) => Err(e.name()),
        }
    }

    /// Whether a posted receive buffer is waiting for a frame that no staged frame has
    /// already claimed. A live wait with none does not block: the frame could only be dropped.
    #[must_use]
    pub fn rx_ready(&self, mem: &[u8], base: u64) -> bool {
        let r = &self.queues[RECEIVEQ];
        if r.pfn == 0 {
            return false;
        }
        let at = self
            .queue(RECEIVEQ)
            .avail
            .wrapping_sub(base)
            .wrapping_add(2) as usize;
        let Some(b) = mem.get(at..at + 2) else {
            return false;
        };
        let offered = u16::from_le_bytes([b[0], b[1]]);
        usize::from(offered.wrapping_sub(r.last_avail)) > self.staged.len()
    }

    /// One instruction has begun: every deferred completion is one nearer, and those at zero
    /// are served now. `pc` is the running instruction's, for a refusal found in an RX
    /// placement; a TX refusal names the notify store.
    ///
    /// # Errors
    /// `(pc, address, refusal)`.
    pub fn tick(
        &mut self,
        mem: &mut [u8],
        base: u64,
        pc: u64,
    ) -> Result<(), (u64, u64, &'static str)> {
        let at = 0x1000_1000 + NET_SLOT * 0x1000 + QUEUE_NOTIFY;
        if let Some(p) = self.pending_tx.as_mut() {
            p.left -= 1;
            if p.left == 0 {
                let p = *p;
                self.pending_tx = None;
                self.notify_now(TRANSMITQ, mem, base)
                    .map_err(|w| (p.pc, at, w))?;
            }
        }
        let mut i = 0;
        while i < self.staged.len() {
            self.staged[i].0 -= 1;
            if self.staged[i].0 == 0 {
                let (_, f) = self.staged.remove(i);
                self.place(mem, base, &f).map_err(|w| (pc, at, w))?;
            } else {
                i += 1;
            }
        }
        Ok(())
    }

    /// Serve a deferred TX now (the world moves while the guest waits, and at the end).
    ///
    /// # Errors
    /// `(pc, address, refusal)`.
    pub fn settle(&mut self, mem: &mut [u8], base: u64) -> Result<(), (u64, u64, &'static str)> {
        let at = 0x1000_1000 + NET_SLOT * 0x1000 + QUEUE_NOTIFY;
        if let Some(p) = self.pending_tx.take() {
            self.notify_now(TRANSMITQ, mem, base)
                .map_err(|w| (p.pc, at, w))?;
        }
        Ok(())
    }
}

/// Slot-2 read for the machine: the device, or the placeholder refusal.
pub(crate) fn load(net: Option<&VirtioNet>, off: u64, width: usize) -> Result<u32, &'static str> {
    net.ok_or(NET_WINDOW_REFUSED)?.read(off, width)
}

/// Serve a deferred TX at a wait or a halt, then answer a refusal found there as the halt it
/// is. Nothing to do on a synchronous device.
pub fn settle(m: &mut Machine) -> Option<Halt> {
    let base = m.base;
    let net = m.net.as_mut()?;
    let r = net.settle(&mut m.mem, base);
    m.resync_waiting();
    r.err()
        .map(|(pc, addr, why)| Halt::Device { pc, addr, why })
}

/// Take the frames the guest has transmitted since the last call.
pub fn take_tx(m: &mut Machine) -> Vec<Vec<u8>> {
    m.net
        .as_mut()
        .map(|n| std::mem::take(&mut n.outbox))
        .unwrap_or_default()
}

/// One record of a net log: what the host delivered at one wait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetRecord {
    /// One frame.
    Frame(Vec<u8>),
    /// Nothing arrived.
    None,
}

/// A parsed net log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetLog {
    /// The device configuration the header carried (the default if there was none).
    pub config: NetConfig,
    /// The records, one per wait, their tags checked.
    pub records: Vec<NetRecord>,
    /// A THREADED log's schedule: `Run(N)` for each `@N`, and `Value(k)` for the k-th
    /// record, in `threads.rs`'s own form. `None` for a single-thread log.
    pub schedule: Option<Vec<ThreadRecord>>,
}

/// The prefix that marks a log as a net log even before its first record.
pub const NET_LOG_PREFIX: &str = "# yantra-net v1 ";

/// The header line a live run writes, carrying `config`.
#[must_use]
pub fn log_header(c: &NetConfig) -> String {
    format!(
        "{NET_LOG_PREFIX}mac={} max-frame={} max-tx={} max-rx={} filter={} defer={} backend={} — \
         one n= record per wait, n=<wait>:<handle>:<hex frame|none>; replay with --events",
        mac_text(&c.mac),
        c.max_frame,
        c.max_tx,
        c.max_rx,
        u8::from(c.mac_filter),
        c.defer,
        c.backend.word()
    )
}

/// Record number `wait` as its log line, without the newline, for handle 0.
#[must_use]
pub fn record_line(wait: usize, r: &NetRecord) -> String {
    match r {
        NetRecord::None => format!("n={wait}:0:none"),
        NetRecord::Frame(f) => {
            let mut s = format!("n={wait}:0:");
            s.reserve(2 * f.len());
            for b in f {
                s.push_str(&format!("{b:02x}"));
            }
            s
        }
    }
}

/// Whether `text` is a net log: the header prefix, or a first record that starts `n=`.
#[must_use]
pub fn is_net_log(text: &str) -> bool {
    text.starts_with(NET_LOG_PREFIX)
        || text
            .lines()
            .map(str::trim)
            .find(|t| !t.is_empty() && !t.starts_with('#'))
            .is_some_and(|t| t.starts_with("n="))
}

fn parse_header(line: &str) -> Result<NetConfig, String> {
    let mut c = NetConfig::default();
    for tok in line[NET_LOG_PREFIX.len()..].split_whitespace() {
        let num = |k: &str, v: &str| {
            v.parse::<u64>()
                .map_err(|e| format!("header {k} {v:?}: {e}"))
        };
        if let Some(v) = tok.strip_prefix("mac=") {
            c.mac = parse_mac(v)?;
        } else if let Some(v) = tok.strip_prefix("max-frame=") {
            c.max_frame = num("max-frame", v)? as usize;
        } else if let Some(v) = tok.strip_prefix("max-tx=") {
            c.max_tx = num("max-tx", v)?;
        } else if let Some(v) = tok.strip_prefix("max-rx=") {
            c.max_rx = num("max-rx", v)?;
        } else if let Some(v) = tok.strip_prefix("filter=") {
            c.mac_filter = num("filter", v)? != 0;
        } else if let Some(v) = tok.strip_prefix("defer=") {
            c.defer = num("defer", v)?;
        } else if let Some(v) = tok.strip_prefix("backend=") {
            c.backend = match v {
                "none" => BackendKind::None,
                "peer" => BackendKind::Peer,
                "tap" => BackendKind::Tap,
                other => return Err(format!("header backend {other:?}: none, peer or tap")),
            };
        }
    }
    if !(MIN_FRAME..=HARD_MAX_FRAME).contains(&c.max_frame) {
        return Err(format!(
            "header max-frame {} is outside 14 to {HARD_MAX_FRAME}",
            c.max_frame
        ));
    }
    Ok(c)
}

/// Read a net log.
///
/// # Errors
/// Naming the line: a torn final record, a wait index or handle that does not match the
/// record's place, an odd-length or non-lowercase-hex body, a frame over [`HARD_MAX_FRAME`],
/// or any line that is not `n=` or `@N` (no mixed logs).
pub fn parse_net_log(text: &str) -> Result<NetLog, String> {
    crate::input::refuse_torn_final_record(text)?;
    let mut config = NetConfig::default();
    let mut records = Vec::new();
    let mut schedule: Vec<ThreadRecord> = Vec::new();
    let mut threaded = false;
    for (n, line) in text.lines().enumerate() {
        let t = line.trim();
        let no = n + 1;
        if t.starts_with(NET_LOG_PREFIX) {
            config = parse_header(t).map_err(|e| format!("line {no}: {e}"))?;
            continue;
        }
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some(k) = t.strip_prefix('@') {
            let k: u32 = k
                .parse()
                .map_err(|e| format!("line {no}: {t:?} is not `@N`: {e}"))?;
            threaded = true;
            schedule.push(ThreadRecord::Run(k));
            continue;
        }
        let Some(body) = t.strip_prefix("n=") else {
            return Err(format!(
                "line {no}: {t:?} is not a net record — a net log holds only n=<wait>:<handle>:<hex|none> \
                 and @N lines (ADR-0047: no mixed logs)"
            ));
        };
        let mut parts = body.splitn(3, ':');
        let (Some(w), Some(h), Some(data)) = (parts.next(), parts.next(), parts.next()) else {
            return Err(format!("line {no}: n= needs <wait>:<handle>:<hex|none>"));
        };
        let (w, h): (usize, u32) = (
            w.parse()
                .map_err(|e| format!("line {no}: wait index {w:?}: {e}"))?,
            h.parse()
                .map_err(|e| format!("line {no}: handle {h:?}: {e}"))?,
        );
        if w != records.len() || h != 0 {
            return Err(format!(
                "line {no}: {NET_LOG_TAG_MISMATCH} (the record is wait {w}, handle {h}; \
                 wait {} on handle 0 is due)",
                records.len()
            ));
        }
        schedule.push(ThreadRecord::Value(records.len() as u64));
        if data == "none" {
            records.push(NetRecord::None);
            continue;
        }
        if data.is_empty() || !data.len().is_multiple_of(2) {
            return Err(format!(
                "line {no}: the frame needs an even, nonzero count of hex digits"
            ));
        }
        if let Some(c) = data.chars().find(|c| !matches!(c, '0'..='9' | 'a'..='f')) {
            return Err(format!("line {no}: {c:?} is not a lowercase hex digit"));
        }
        if data.len() / 2 > HARD_MAX_FRAME {
            return Err(format!(
                "line {no}: the frame is over {HARD_MAX_FRAME} octets"
            ));
        }
        let f = (0..data.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&data[i..i + 2], 16).expect("checked hex above"))
            .collect();
        records.push(NetRecord::Frame(f));
    }
    Ok(NetLog {
        config,
        records,
        schedule: threaded.then_some(schedule),
    })
}

/// Deliver one record at a wait: the frame into the RX queue, and the octets placed into
/// the `SASEVENT` word at `tag + 8`. The one place RX happens, shared by the live host,
/// [`replay_net`] and the threaded runs.
///
/// # Errors
/// The refusal by name; the record is then neither applied nor (live) logged.
pub fn deliver(m: &mut Machine, tag: usize, record: &NetRecord) -> Result<u64, String> {
    let base = m.base;
    let placed = match record {
        NetRecord::None => 0,
        NetRecord::Frame(f) => {
            let net = m.net.as_mut().ok_or("no net device on this machine")?;
            net.receive(&mut m.mem, base, f)?
        }
    };
    m.mem[tag + 8..tag + 16].copy_from_slice(&placed.to_le_bytes());
    // A staged frame is a completion the per-step test must now watch.
    m.resync_waiting();
    Ok(placed)
}

/// How a net replay ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetReplayed {
    /// A halt that is not a wait, after `delivered` records.
    Halted {
        /// The halt.
        halt: Halt,
        /// Records delivered.
        delivered: usize,
    },
    /// The log ran out at wait `index` (never padded).
    Short {
        /// Which wait.
        index: usize,
        /// The wait store.
        pc: u64,
    },
    /// The record due refused (a cap): not applied.
    Refused {
        /// Which wait.
        index: usize,
        /// The wait store.
        pc: u64,
        /// The refusal's words.
        why: String,
    },
}

/// Replay a net log: run; at each wait hand the guest's transmitted frames to `tx`, deliver
/// the next record, and run again. No backend is consulted. Available on every target.
pub fn replay_net(
    m: &mut Machine,
    tag: usize,
    log: &[NetRecord],
    budget: u64,
    out: &mut impl Output,
    tx: &mut dyn FnMut(&[u8]),
) -> NetReplayed {
    let start = m.time;
    let mut delivered = 0;
    loop {
        let left = budget.saturating_sub(m.time - start);
        let mut h = m.run(left, out);
        if let Some(d) = settle(m) {
            h = d;
        }
        for f in take_tx(m) {
            tx(&f);
        }
        match h {
            Halt::Wait { pc } => {
                let Some(r) = log.get(delivered) else {
                    return NetReplayed::Short {
                        index: delivered,
                        pc,
                    };
                };
                if let Err(why) = deliver(m, tag, r) {
                    return NetReplayed::Refused {
                        index: delivered,
                        pc,
                        why,
                    };
                }
                delivered += 1;
            }
            halt => return NetReplayed::Halted { halt, delivered },
        }
    }
}

/// Where live frames go and come from. Implemented by the host (`yantra-run`); this file
/// knows no transport.
pub trait FrameBackend {
    /// Send one frame, best effort; a failure is the host's to say, never the guest's.
    fn send(&mut self, frame: &[u8]);
    /// Wait for one frame: forever, or until `--net-timeout`.
    ///
    /// # Errors
    /// [`NET_TIMEOUT`] on expiry, [`NET_PEER_CLOSED`] if the peer is gone.
    fn recv(&mut self) -> Result<Vec<u8>, String>;
    /// A frame if one is already here, without waiting.
    ///
    /// # Errors
    /// [`NET_PEER_CLOSED`] if the peer is gone.
    fn try_recv(&mut self) -> Result<Option<Vec<u8>>, String>;
}

/// LIVE MODE: run; at each wait send the guest's frames, take ONE frame from the backend
/// (blocking), deliver it, APPEND its record to `log` (flushed), and resume. Answers the
/// halt and the records written.
///
/// # Errors
/// A log write failure, a timeout or a closed peer, or a refused frame (a cap) — which is
/// not delivered or logged.
pub fn record_live_net(
    m: &mut Machine,
    tag: usize,
    budget: u64,
    out: &mut impl Output,
    log: &mut impl std::io::Write,
    backend: &mut dyn FrameBackend,
    sent: &mut Vec<u8>,
) -> Result<(Halt, usize), String> {
    let io = |e: std::io::Error| format!("writing the net log: {e}");
    let config = m
        .net
        .as_ref()
        .ok_or("no net device on this machine")?
        .config;
    writeln!(log, "{}", log_header(&config)).map_err(io)?;
    log.flush().map_err(io)?;
    let start = m.time;
    let mut delivered = 0;
    loop {
        let left = budget.saturating_sub(m.time - start);
        let mut h = m.run(left, out);
        if let Some(d) = settle(m) {
            h = d;
        }
        for f in take_tx(m) {
            note_sent(sent, &f);
            backend.send(&f);
        }
        match h {
            Halt::Wait { pc } => {
                // No posted buffer: nothing could be placed, so do not block for a frame.
                let ready = m.net.as_ref().is_some_and(|n| n.rx_ready(&m.mem, m.base));
                let record = if ready {
                    NetRecord::Frame(
                        backend
                            .recv()
                            .map_err(|e| format!("wait index {delivered} (pc {pc:#x}): {e}"))?,
                    )
                } else {
                    NetRecord::None
                };
                deliver(m, tag, &record)
                    .map_err(|why| format!("wait index {delivered} (pc {pc:#x}): {why}"))?;
                writeln!(log, "{}", record_line(delivered, &record)).map_err(io)?;
                log.flush().map_err(io)?;
                delivered += 1;
            }
            halt => return Ok((halt, delivered)),
        }
    }
}

/// What a THREADED net run carries beside the machine and `Threads`: the backend (live) or
/// the records (replay), and the digest input of the frames sent.
pub struct NetThreads {
    /// The live backend; `None` in a replay.
    pub backend: Option<Box<dyn FrameBackend>>,
    /// The records of a replay, in order.
    pub records: Vec<NetRecord>,
    /// The ordinal of the next wait to deliver.
    pub next: usize,
    /// Every frame sent, length-prefixed, as [`report_line`] hashes it.
    pub sent: Vec<u8>,
    /// A refusal found settling a deferred TX; the threaded loops end the run with it.
    pub fault: Option<Halt>,
}

impl NetThreads {
    /// A live run's state.
    #[must_use]
    pub fn live(backend: Box<dyn FrameBackend>) -> Self {
        Self {
            backend: Some(backend),
            records: Vec::new(),
            next: 0,
            sent: Vec::new(),
            fault: None,
        }
    }

    /// A replay's state.
    #[must_use]
    pub fn replay(records: Vec<NetRecord>) -> Self {
        Self {
            backend: None,
            records,
            next: 0,
            sent: Vec::new(),
            fault: None,
        }
    }

    /// Hand the frames the guest has transmitted to the backend (live) and the digest.
    pub fn flush(&mut self, m: &mut Machine) {
        if let Some(d) = settle(m) {
            self.fault.get_or_insert(d);
        }
        for f in take_tx(m) {
            note_sent(&mut self.sent, &f);
            if let Some(b) = self.backend.as_mut() {
                b.send(&f);
            }
        }
    }

    /// REPLAY: deliver record number `k` (which must be the next) at `tag`.
    ///
    /// # Errors
    /// A record out of order, or the refusal of [`deliver`].
    pub fn deliver_replayed(&mut self, m: &mut Machine, tag: usize, k: u64) -> Result<(), String> {
        if k != self.next as u64 {
            return Err(format!(
                "{NET_LOG_TAG_MISMATCH} (record {k} delivered as wait {})",
                self.next
            ));
        }
        let r = self
            .records
            .get(self.next)
            .ok_or_else(|| format!("the net log has no record for wait {}", self.next))?
            .clone();
        deliver(m, tag, &r)?;
        self.next += 1;
        Ok(())
    }

    /// LIVE: deliver `r` as the next wait's record and write its log line.
    ///
    /// # Errors
    /// The refusal of [`deliver`], or a log write failure.
    pub fn deliver_live(
        &mut self,
        m: &mut Machine,
        tag: usize,
        r: NetRecord,
        log: &mut impl std::io::Write,
    ) -> Result<(), String> {
        deliver(m, tag, &r)?;
        writeln!(log, "{}", record_line(self.next, &r))
            .map_err(|e| format!("writing the net log: {e}"))?;
        log.flush()
            .map_err(|e| format!("writing the net log: {e}"))?;
        self.next += 1;
        Ok(())
    }
}

/// The closing report line both modes print: counts, and the sha256 of every transmitted
/// frame, length-prefixed (two octets, big endian), in order.
#[must_use]
pub fn report_line(net: Option<&VirtioNet>, records: usize, sent: &[u8]) -> String {
    let n = net
        .cloned()
        .unwrap_or_else(|| VirtioNet::new(NetConfig::default()));
    let digest: String = crate::smp::sha256(sent)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    format!(
        "net: {} frames transmitted, {} received ({} dropped, {} filtered), {records} wait \
         records; sent sha256 {digest}",
        n.tx_frames, n.rx_frames, n.rx_dropped, n.rx_filtered
    )
}

/// Append `frame` to `sent` as [`report_line`] hashes it.
pub fn note_sent(sent: &mut Vec<u8>, frame: &[u8]) {
    sent.extend_from_slice(&(frame.len() as u16).to_be_bytes());
    sent.extend_from_slice(frame);
}
