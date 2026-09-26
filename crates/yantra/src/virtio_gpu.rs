//! The virtio-gpu 2D wire format — row `C-015`, first unit.
//!
//! # What this is, and what it is not
//!
//! This is the *bytes* of the virtio-gpu control queue: the 24-byte request header, the
//! six 2D commands, the replies a device writes back, and the 16-byte split-virtqueue
//! descriptor that carries them. Every struct here has an `encode` — what a driver stores
//! into guest memory — and a `parse` — what a device, or a test standing where the device
//! stands, reads back out of it. Both directions are here **on purpose**: the driver that
//! exists today is `spec/virtio-gpu.sas`, written in Sassembly and running under QEMU,
//! and the only way a test under `crates/` can say anything about what that driver put on
//! the wire is to parse the same bytes it wrote.
//!
//! It is **not** a transport. There is no virtqueue in Rust anywhere in this tree
//! (measured 2026-09-03: `grep -rn virtio crates/ --include='*.rs'` returns five comment
//! lines and no code), and there is no Rust code that runs on the riscv64 target for a
//! transport to live in. The transport this crate's machine will need — a virtio-mmio
//! device at `0x1000_1000..0x1000_9000` answering the legacy register set — is the second
//! unit and is designed beside the bridge at `Machine::store` (`lib.rs`, "THE BRIDGE").
//!
//! # Where the numbers come from
//!
//! virtio v1.2, §5.7 "GPU Device": device ID 16 (§5.7.1), two virtqueues — 0 controlq,
//! 1 cursorq (§5.7.2), the request header `virtio_gpu_ctrl_hdr` (§5.7.6.7), and the
//! controlq commands with their structs (§5.7.6.8). The descriptor is §2.7.5 "The
//! Virtqueue Descriptor Table"; the framing rule that device-writable descriptors follow
//! device-readable ones is §2.7.4 "Message Framing". The corpus copy is
//! `research/specs/platform/virtio-1.2.pdf` (`FETCH-LOG.tsv:485`); the section numbers
//! above are the spec's own and were **not** re-read from that PDF when this was written,
//! because the host had no PDF renderer — confirm them there before quoting them further.
//!
//! The byte offsets are also the ones `spec/virtio-gpu.sas` stores at, line by line:
//! `RESOURCE_CREATE_2D` puts the resource id at +24, the format at +28, width at +32,
//! height at +36 and is 40 bytes long (`virtio-gpu.sas:201-211`). Where the two disagree,
//! this module is wrong or that program is, and the test
//! `resource_create_2d_lays_its_bytes_where_spec_virtio_gpu_sas_stores_them` says which.
//!
//! # What is refused, and why each refusal is a statement
//!
//! A parser that accepts everything measures nothing. Each `Err` here names a byte
//! pattern that a real device would either reject or that would mean the device never
//! answered:
//!
//! - **a header shorter than 24 bytes** — there is no request or reply that short;
//! - **the `0xffffffff` fill** in a reply — `spec/virtio-gpu.sas:330-335` fills the reply
//!   buffer with `0xff` before every submission precisely so that "the device never wrote"
//!   cannot look like an answer. Parsing that fill as a reply would undo that;
//! - **a request type in the reply slot** — `0x01xx` where `0x11xx`/`0x12xx` is due is a
//!   buffer that still holds what the driver wrote, not what the device wrote;
//! - **resource id 0** for `RESOURCE_CREATE_2D`, `RESOURCE_ATTACH_BACKING`,
//!   `TRANSFER_TO_HOST_2D` and `RESOURCE_FLUSH` — QEMU answers
//!   `VIRTIO_GPU_RESP_ERR_INVALID_RESOURCE_ID` to a create with id 0, and no resource can
//!   ever have that id, so a command naming it is malformed on the wire. **`SET_SCANOUT`
//!   with resource id 0 is accepted**: §5.7.6.8 defines it as disabling the scanout, and a
//!   parser that refused it would refuse a legal command;
//! - **flag bits** other than `FENCE` and `INFO_RING_IDX`, **a zero-area resource**, a
//!   **format** outside the eight `VIRTIO_GPU_FORMAT_*` values, and a backing list whose
//!   **count outruns its bytes**;
//! - **a descriptor chain that loops** or whose `next` points past the queue.
//!
//! Every refusal is a `String` that names the byte and the value, in the style of
//! [`crate::loader::Program::parse`]: a wrong answer here is a pixel somewhere else, not a
//! crash, so the check has to be made where the bytes are read.

/// Length of `virtio_gpu_ctrl_hdr` (§5.7.6.7): type, flags, fence_id, ctx_id, ring_idx and
/// three bytes of padding.
pub const CTRL_HDR_LEN: usize = 24;

/// `VIRTIO_GPU_FLAG_FENCE` — the driver asks for `fence_id` to be echoed when done.
pub const FLAG_FENCE: u32 = 1 << 0;
/// `VIRTIO_GPU_FLAG_INFO_RING_IDX` — `ring_idx` is meaningful (needs `F_CONTEXT_INIT`).
pub const FLAG_INFO_RING_IDX: u32 = 1 << 1;

/// The number of `virtio_gpu_display_one` entries in a display-info reply. Fixed by the
/// spec (`VIRTIO_GPU_MAX_SCANOUTS`), not by how many scanouts the device has.
pub const MAX_SCANOUTS: usize = 16;

/// Length of one `virtio_gpu_display_one`: a rect and two `le32`.
const DISPLAY_ONE_LEN: usize = 24;
/// Length of `virtio_gpu_resp_display_info`: the header plus sixteen modes — 408 bytes.
pub const DISPLAY_INFO_LEN: usize = CTRL_HDR_LEN + MAX_SCANOUTS * DISPLAY_ONE_LEN;

/// Request type codes, §5.7.6.8 "Device Operation: controlq". Only the 2D set this row
/// needs; `RESOURCE_UNREF`, `DETACH_BACKING`, the capset and EDID commands are named in
/// [`Request::parse`]'s refusal so a reader knows they were seen, not missed.
pub mod cmd {
    /// `VIRTIO_GPU_CMD_GET_DISPLAY_INFO`.
    pub const GET_DISPLAY_INFO: u32 = 0x0100;
    /// `VIRTIO_GPU_CMD_RESOURCE_CREATE_2D`.
    pub const RESOURCE_CREATE_2D: u32 = 0x0101;
    /// `VIRTIO_GPU_CMD_RESOURCE_UNREF` — recognised, not implemented here.
    pub const RESOURCE_UNREF: u32 = 0x0102;
    /// `VIRTIO_GPU_CMD_SET_SCANOUT`.
    pub const SET_SCANOUT: u32 = 0x0103;
    /// `VIRTIO_GPU_CMD_RESOURCE_FLUSH`.
    pub const RESOURCE_FLUSH: u32 = 0x0104;
    /// `VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D`.
    pub const TRANSFER_TO_HOST_2D: u32 = 0x0105;
    /// `VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING`.
    pub const RESOURCE_ATTACH_BACKING: u32 = 0x0106;
    /// `VIRTIO_GPU_CMD_RESOURCE_DETACH_BACKING` — recognised, not implemented here.
    pub const RESOURCE_DETACH_BACKING: u32 = 0x0107;
}

/// Reply type codes, §5.7.6.7. `0x11xx` succeeds, `0x12xx` fails.
pub mod resp {
    /// `VIRTIO_GPU_RESP_OK_NODATA` — the answer every 2D command except display-info gives.
    pub const OK_NODATA: u32 = 0x1100;
    /// `VIRTIO_GPU_RESP_OK_DISPLAY_INFO`.
    pub const OK_DISPLAY_INFO: u32 = 0x1101;
    /// `VIRTIO_GPU_RESP_OK_CAPSET_INFO`.
    pub const OK_CAPSET_INFO: u32 = 0x1102;
    /// `VIRTIO_GPU_RESP_OK_CAPSET`.
    pub const OK_CAPSET: u32 = 0x1103;
    /// `VIRTIO_GPU_RESP_OK_EDID`.
    pub const OK_EDID: u32 = 0x1104;
    /// `VIRTIO_GPU_RESP_ERR_UNSPEC`.
    pub const ERR_UNSPEC: u32 = 0x1200;
    /// `VIRTIO_GPU_RESP_ERR_OUT_OF_MEMORY`.
    pub const ERR_OUT_OF_MEMORY: u32 = 0x1201;
    /// `VIRTIO_GPU_RESP_ERR_INVALID_SCANOUT_ID`.
    pub const ERR_INVALID_SCANOUT_ID: u32 = 0x1202;
    /// `VIRTIO_GPU_RESP_ERR_INVALID_RESOURCE_ID`.
    pub const ERR_INVALID_RESOURCE_ID: u32 = 0x1203;
    /// `VIRTIO_GPU_RESP_ERR_INVALID_CONTEXT_ID`.
    pub const ERR_INVALID_CONTEXT_ID: u32 = 0x1204;
    /// `VIRTIO_GPU_RESP_ERR_INVALID_PARAMETER`.
    pub const ERR_INVALID_PARAMETER: u32 = 0x1205;
}

/// `VIRTIO_GPU_FORMAT_*` for `RESOURCE_CREATE_2D`, §5.7.6.8. Eight values, all 32 bits per
/// pixel; `spec/virtio-gpu.sas:206` uses `B8G8R8X8_UNORM` = 2.
pub const FORMATS: [u32; 8] = [1, 2, 3, 4, 67, 68, 121, 134];

/// `VIRTIO_GPU_FORMAT_B8G8R8X8_UNORM` — bytes B, G, R, X in memory, so a `le32` reads
/// `0x00RRGGBB`. The format both `spec/virtio-gpu.sas` and `spec/compositor.sas` use.
pub const FORMAT_B8G8R8X8_UNORM: u32 = 2;

fn le16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn le32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn le64(b: &[u8], o: usize) -> u64 {
    let mut w = [0u8; 8];
    w.copy_from_slice(&b[o..o + 8]);
    u64::from_le_bytes(w)
}

/// The slice must hold `want` bytes for `what`; otherwise the refusal names both numbers.
fn need(b: &[u8], want: usize, what: &str) -> Result<(), String> {
    if b.len() < want {
        return Err(format!(
            "{what} is {want} bytes; only {} were given",
            b.len()
        ));
    }
    Ok(())
}

/// What a reply slot reads as when the device never wrote it — see `spec/virtio-gpu.sas:330`.
const NEVER_WROTE: &str = "reply type is ffffffff — the fill the driver wrote before \
                           submitting; the device never wrote a reply";

/// `virtio_gpu_ctrl_hdr`, §5.7.6.7 — the first 24 bytes of every request and every reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// `type`: a `cmd::*` value in a request, a `resp::*` value in a reply.
    pub kind: u32,
    /// `flags`: [`FLAG_FENCE`] and, since v1.2, [`FLAG_INFO_RING_IDX`]. Nothing else.
    pub flags: u32,
    /// `fence_id`: echoed by the device when `FENCE` is set.
    pub fence_id: u64,
    /// `ctx_id`: rendering context; always 0 on the 2D path.
    pub ctx_id: u32,
    /// `ring_idx`: meaningful only under `INFO_RING_IDX`; kept so a round trip is exact.
    pub ring_idx: u8,
}

impl Header {
    /// A request or reply header of `kind` with no flags, fence, context or ring — what
    /// every command on the 2D path carries.
    pub fn plain(kind: u32) -> Header {
        Header {
            kind,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
        }
    }

    /// The 24 bytes, little-endian, padding zero.
    pub fn encode(&self) -> [u8; CTRL_HDR_LEN] {
        let mut b = [0u8; CTRL_HDR_LEN];
        b[0..4].copy_from_slice(&self.kind.to_le_bytes());
        b[4..8].copy_from_slice(&self.flags.to_le_bytes());
        b[8..16].copy_from_slice(&self.fence_id.to_le_bytes());
        b[16..20].copy_from_slice(&self.ctx_id.to_le_bytes());
        b[20] = self.ring_idx;
        b
    }

    /// Read a header from the front of `b`.
    ///
    /// # Errors
    /// Fewer than 24 bytes, or a flag bit the spec does not define. The type is **not**
    /// judged here — a request parser and a reply parser want different sets, and each
    /// judges it itself.
    pub fn parse(b: &[u8]) -> Result<Header, String> {
        need(b, CTRL_HDR_LEN, "virtio_gpu_ctrl_hdr")?;
        let flags = le32(b, 4);
        let known = FLAG_FENCE | FLAG_INFO_RING_IDX;
        if flags & !known != 0 {
            return Err(format!(
                "flags {flags:#010x} set bits the spec does not define (only FENCE and \
                 INFO_RING_IDX exist)"
            ));
        }
        Ok(Header {
            kind: le32(b, 0),
            flags,
            fence_id: le64(b, 8),
            ctx_id: le32(b, 16),
            ring_idx: b[20],
        })
    }
}

/// `virtio_gpu_rect`: four `le32`, 16 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    /// Left edge, pixels.
    pub x: u32,
    /// Top edge, pixels.
    pub y: u32,
    /// Width, pixels.
    pub width: u32,
    /// Height, pixels.
    pub height: u32,
}

impl Rect {
    fn encode_into(&self, b: &mut [u8]) {
        b[0..4].copy_from_slice(&self.x.to_le_bytes());
        b[4..8].copy_from_slice(&self.y.to_le_bytes());
        b[8..12].copy_from_slice(&self.width.to_le_bytes());
        b[12..16].copy_from_slice(&self.height.to_le_bytes());
    }
    fn parse_at(b: &[u8], o: usize) -> Rect {
        Rect {
            x: le32(b, o),
            y: le32(b, o + 4),
            width: le32(b, o + 8),
            height: le32(b, o + 12),
        }
    }
}

/// `virtio_gpu_mem_entry`: one span of guest-physical memory backing a resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemEntry {
    /// Guest-physical address of the span.
    pub addr: u64,
    /// Length in bytes.
    pub length: u32,
}

/// The six controlq commands the 2D path uses, §5.7.6.8, with their bodies. The header is
/// carried separately in [`Request`] so a fenced command and a plain one are the same
/// `Command`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `VIRTIO_GPU_CMD_GET_DISPLAY_INFO`: header only; the reply is
    /// [`Reply::OkDisplayInfo`].
    GetDisplayInfo,
    /// `VIRTIO_GPU_CMD_RESOURCE_CREATE_2D`: a host-side 2D resource.
    ResourceCreate2d {
        /// Driver-chosen, non-zero.
        resource_id: u32,
        /// One of [`FORMATS`].
        format: u32,
        /// Pixels; non-zero.
        width: u32,
        /// Pixels; non-zero.
        height: u32,
    },
    /// `VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING`: name the guest pages behind a resource.
    ResourceAttachBacking {
        /// The resource, non-zero.
        resource_id: u32,
        /// At least one span.
        entries: Vec<MemEntry>,
    },
    /// `VIRTIO_GPU_CMD_SET_SCANOUT`: show `rect` of `resource_id` on `scanout_id`.
    /// A `resource_id` of 0 disables the scanout, and is legal.
    SetScanout {
        /// The region of the resource shown.
        rect: Rect,
        /// Which head; 0 is the only one QEMU `virt` offers by default.
        scanout_id: u32,
        /// The resource, or 0 to disable.
        resource_id: u32,
    },
    /// `VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D`: copy `rect` from the backing into the resource.
    TransferToHost2d {
        /// The region to copy.
        rect: Rect,
        /// Byte offset into the backing where `rect` begins.
        offset: u64,
        /// The resource, non-zero.
        resource_id: u32,
    },
    /// `VIRTIO_GPU_CMD_RESOURCE_FLUSH`: present `rect` of the resource on its scanouts.
    ResourceFlush {
        /// The region to present.
        rect: Rect,
        /// The resource, non-zero.
        resource_id: u32,
    },
}

impl Command {
    /// The `cmd::*` code for this command.
    pub fn kind(&self) -> u32 {
        match self {
            Command::GetDisplayInfo => cmd::GET_DISPLAY_INFO,
            Command::ResourceCreate2d { .. } => cmd::RESOURCE_CREATE_2D,
            Command::ResourceAttachBacking { .. } => cmd::RESOURCE_ATTACH_BACKING,
            Command::SetScanout { .. } => cmd::SET_SCANOUT,
            Command::TransferToHost2d { .. } => cmd::TRANSFER_TO_HOST_2D,
            Command::ResourceFlush { .. } => cmd::RESOURCE_FLUSH,
        }
    }

    /// Bytes on the wire including the header: 24, 40, 32 + 16·n, 48, 56, 48 — the values
    /// `spec/virtio-gpu.sas` loads into `स्थिर७` before each `कार्यम्`.
    pub fn wire_len(&self) -> usize {
        match self {
            Command::GetDisplayInfo => CTRL_HDR_LEN,
            Command::ResourceCreate2d { .. } => CTRL_HDR_LEN + 16,
            Command::ResourceAttachBacking { entries, .. } => CTRL_HDR_LEN + 8 + 16 * entries.len(),
            Command::SetScanout { .. } => CTRL_HDR_LEN + 24,
            Command::TransferToHost2d { .. } => CTRL_HDR_LEN + 32,
            Command::ResourceFlush { .. } => CTRL_HDR_LEN + 24,
        }
    }
}

/// A complete request: header plus command. [`Request::encode`] is the driver's side,
/// [`Request::parse`] the device's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The header. `hdr.kind` must agree with `cmd.kind()`; [`Request::new`] guarantees it.
    pub hdr: Header,
    /// The command.
    pub cmd: Command,
}

impl Request {
    /// A plain, unfenced request for `cmd`.
    pub fn new(cmd: Command) -> Request {
        Request {
            hdr: Header::plain(cmd.kind()),
            cmd,
        }
    }

    /// The bytes a driver stores in the device-readable descriptor. Exactly
    /// [`Command::wire_len`] long.
    pub fn encode(&self) -> Vec<u8> {
        let mut b = vec![0u8; self.cmd.wire_len()];
        b[..CTRL_HDR_LEN].copy_from_slice(&self.hdr.encode());
        let body = &mut b[CTRL_HDR_LEN..];
        match &self.cmd {
            Command::GetDisplayInfo => {}
            Command::ResourceCreate2d {
                resource_id,
                format,
                width,
                height,
            } => {
                body[0..4].copy_from_slice(&resource_id.to_le_bytes());
                body[4..8].copy_from_slice(&format.to_le_bytes());
                body[8..12].copy_from_slice(&width.to_le_bytes());
                body[12..16].copy_from_slice(&height.to_le_bytes());
            }
            Command::ResourceAttachBacking {
                resource_id,
                entries,
            } => {
                body[0..4].copy_from_slice(&resource_id.to_le_bytes());
                let n = u32::try_from(entries.len()).expect("a backing list fits in le32");
                body[4..8].copy_from_slice(&n.to_le_bytes());
                for (i, e) in entries.iter().enumerate() {
                    let o = 8 + 16 * i;
                    body[o..o + 8].copy_from_slice(&e.addr.to_le_bytes());
                    body[o + 8..o + 12].copy_from_slice(&e.length.to_le_bytes());
                }
            }
            Command::SetScanout {
                rect,
                scanout_id,
                resource_id,
            } => {
                rect.encode_into(&mut body[0..16]);
                body[16..20].copy_from_slice(&scanout_id.to_le_bytes());
                body[20..24].copy_from_slice(&resource_id.to_le_bytes());
            }
            Command::TransferToHost2d {
                rect,
                offset,
                resource_id,
            } => {
                rect.encode_into(&mut body[0..16]);
                body[16..24].copy_from_slice(&offset.to_le_bytes());
                body[24..28].copy_from_slice(&resource_id.to_le_bytes());
            }
            Command::ResourceFlush { rect, resource_id } => {
                rect.encode_into(&mut body[0..16]);
                body[16..20].copy_from_slice(&resource_id.to_le_bytes());
            }
        }
        b
    }

    /// Read a request from the bytes a device-readable descriptor points at. Trailing bytes
    /// beyond the command's length are ignored, as a device ignores them.
    ///
    /// # Errors
    /// A short buffer, an unknown or out-of-scope type, an undefined flag, a resource id of
    /// 0 where one is required, a zero-area or unknown-format resource, or a backing list
    /// whose count is 0 or does not fit in the bytes given. Each names what it saw.
    pub fn parse(b: &[u8]) -> Result<Request, String> {
        let hdr = Header::parse(b)?;
        let body = &b[CTRL_HDR_LEN..];
        let cmd = match hdr.kind {
            cmd::GET_DISPLAY_INFO => Command::GetDisplayInfo,
            cmd::RESOURCE_CREATE_2D => {
                need(body, 16, "virtio_gpu_resource_create_2d's body")?;
                let resource_id = nonzero(le32(body, 0), "RESOURCE_CREATE_2D")?;
                let format = le32(body, 4);
                if !FORMATS.contains(&format) {
                    return Err(format!(
                        "RESOURCE_CREATE_2D format {format} is not one of the eight \
                         VIRTIO_GPU_FORMAT_* values"
                    ));
                }
                let (width, height) = (le32(body, 8), le32(body, 12));
                if width == 0 || height == 0 {
                    return Err(format!(
                        "RESOURCE_CREATE_2D of {width}x{height}: a resource with no pixels \
                         is not a framebuffer"
                    ));
                }
                Command::ResourceCreate2d {
                    resource_id,
                    format,
                    width,
                    height,
                }
            }
            cmd::RESOURCE_ATTACH_BACKING => {
                need(body, 8, "virtio_gpu_resource_attach_backing's body")?;
                let resource_id = nonzero(le32(body, 0), "RESOURCE_ATTACH_BACKING")?;
                let n = le32(body, 4);
                if n == 0 {
                    return Err(
                        "RESOURCE_ATTACH_BACKING with nr_entries 0: nothing to attach".into(),
                    );
                }
                let n = usize::try_from(n).map_err(|_| "nr_entries does not fit".to_string())?;
                let want = 8 + 16 * n;
                if body.len() < want {
                    return Err(format!(
                        "RESOURCE_ATTACH_BACKING names {n} entries, which need {want} body \
                         bytes; only {} were given",
                        body.len()
                    ));
                }
                let entries = (0..n)
                    .map(|i| {
                        let o = 8 + 16 * i;
                        MemEntry {
                            addr: le64(body, o),
                            length: le32(body, o + 8),
                        }
                    })
                    .collect();
                Command::ResourceAttachBacking {
                    resource_id,
                    entries,
                }
            }
            cmd::SET_SCANOUT => {
                need(body, 24, "virtio_gpu_set_scanout's body")?;
                // resource_id 0 is LEGAL here: it disables the scanout (§5.7.6.8).
                Command::SetScanout {
                    rect: Rect::parse_at(body, 0),
                    scanout_id: le32(body, 16),
                    resource_id: le32(body, 20),
                }
            }
            cmd::TRANSFER_TO_HOST_2D => {
                need(body, 32, "virtio_gpu_transfer_to_host_2d's body")?;
                Command::TransferToHost2d {
                    rect: Rect::parse_at(body, 0),
                    offset: le64(body, 16),
                    resource_id: nonzero(le32(body, 24), "TRANSFER_TO_HOST_2D")?,
                }
            }
            cmd::RESOURCE_FLUSH => {
                need(body, 24, "virtio_gpu_resource_flush's body")?;
                Command::ResourceFlush {
                    rect: Rect::parse_at(body, 0),
                    resource_id: nonzero(le32(body, 16), "RESOURCE_FLUSH")?,
                }
            }
            cmd::RESOURCE_UNREF | cmd::RESOURCE_DETACH_BACKING => {
                return Err(format!(
                    "request type {:#06x} is a controlq command outside C-015's 2D set",
                    hdr.kind
                ));
            }
            other if (0x1100..0x1300).contains(&other) => {
                return Err(format!(
                    "type {other:#06x} is a REPLY code in a request slot"
                ));
            }
            other => {
                return Err(format!(
                    "request type {other:#06x} is not a virtio-gpu controlq command"
                ));
            }
        };
        Ok(Request { hdr, cmd })
    }
}

/// Resource id 0 is never a resource; naming it is malformed for every command but
/// `SET_SCANOUT`.
fn nonzero(id: u32, what: &str) -> Result<u32, String> {
    if id == 0 {
        return Err(format!(
            "{what} names resource id 0 — no resource has that id (QEMU answers \
             ERR_INVALID_RESOURCE_ID); only SET_SCANOUT may use 0, to disable a head"
        ));
    }
    Ok(id)
}

/// One `virtio_gpu_display_one`: a scanout's mode as the device reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DisplayOne {
    /// Position and size of the head.
    pub r: Rect,
    /// Non-zero when the head is connected.
    pub enabled: u32,
    /// Reserved; echoed.
    pub flags: u32,
}

/// The reply a device writes into the device-writable descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// `OK_NODATA`: the command was done. Header only.
    OkNoData,
    /// `OK_DISPLAY_INFO`: the sixteen heads, enabled or not.
    OkDisplayInfo(Box<[DisplayOne; MAX_SCANOUTS]>),
    /// `OK_CAPSET_INFO`, `OK_CAPSET` or `OK_EDID` — success codes off the 2D path. The body
    /// is not decoded; the code is kept so a reader sees which arrived.
    OkOther(u32),
    /// A `0x12xx` failure, by name.
    Err(GpuError),
}

/// The six `VIRTIO_GPU_RESP_ERR_*` codes, §5.7.6.7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuError {
    /// `ERR_UNSPEC`.
    Unspec,
    /// `ERR_OUT_OF_MEMORY`.
    OutOfMemory,
    /// `ERR_INVALID_SCANOUT_ID`.
    InvalidScanoutId,
    /// `ERR_INVALID_RESOURCE_ID`.
    InvalidResourceId,
    /// `ERR_INVALID_CONTEXT_ID`.
    InvalidContextId,
    /// `ERR_INVALID_PARAMETER`.
    InvalidParameter,
}

impl GpuError {
    /// The wire code.
    pub fn code(self) -> u32 {
        match self {
            GpuError::Unspec => resp::ERR_UNSPEC,
            GpuError::OutOfMemory => resp::ERR_OUT_OF_MEMORY,
            GpuError::InvalidScanoutId => resp::ERR_INVALID_SCANOUT_ID,
            GpuError::InvalidResourceId => resp::ERR_INVALID_RESOURCE_ID,
            GpuError::InvalidContextId => resp::ERR_INVALID_CONTEXT_ID,
            GpuError::InvalidParameter => resp::ERR_INVALID_PARAMETER,
        }
    }
}

/// A complete reply: header plus body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// The header; `hdr.kind` is the `resp::*` code.
    pub hdr: Header,
    /// What the device said.
    pub reply: Reply,
}

impl Response {
    /// The reply code for `reply`.
    pub fn kind_of(reply: &Reply) -> u32 {
        match reply {
            Reply::OkNoData => resp::OK_NODATA,
            Reply::OkDisplayInfo(_) => resp::OK_DISPLAY_INFO,
            Reply::OkOther(code) => *code,
            Reply::Err(e) => e.code(),
        }
    }

    /// A plain reply — the shape a device model writes for a plain request.
    pub fn new(reply: Reply) -> Response {
        Response {
            hdr: Header::plain(Response::kind_of(&reply)),
            reply,
        }
    }

    /// The bytes a device writes: 24 for everything but display-info, which is 408.
    pub fn encode(&self) -> Vec<u8> {
        let mut b = self.hdr.encode().to_vec();
        if let Reply::OkDisplayInfo(modes) = &self.reply {
            for m in modes.iter() {
                let mut one = [0u8; DISPLAY_ONE_LEN];
                m.r.encode_into(&mut one[0..16]);
                one[16..20].copy_from_slice(&m.enabled.to_le_bytes());
                one[20..24].copy_from_slice(&m.flags.to_le_bytes());
                b.extend_from_slice(&one);
            }
        }
        b
    }

    /// Read what the device wrote into the device-writable descriptor.
    ///
    /// # Errors
    /// A header shorter than 24 bytes; the `0xffffffff` fill, named as "the device never
    /// wrote"; a `0x01xx` request code still sitting in the reply slot; a display-info
    /// reply shorter than its 408 bytes; or a type that is no reply code at all.
    pub fn parse(b: &[u8]) -> Result<Response, String> {
        // The fill is judged BEFORE the header: 0xffffffff also fails the flag check, and
        // "undefined flag bits" would be a true statement that hides the one that matters.
        need(b, CTRL_HDR_LEN, "virtio_gpu_ctrl_hdr")?;
        if le32(b, 0) == 0xffff_ffff {
            return Err(NEVER_WROTE.into());
        }
        let hdr = Header::parse(b)?;
        let reply = match hdr.kind {
            resp::OK_NODATA => Reply::OkNoData,
            resp::OK_DISPLAY_INFO => {
                need(b, DISPLAY_INFO_LEN, "virtio_gpu_resp_display_info")?;
                let mut modes = Box::new([DisplayOne::default(); MAX_SCANOUTS]);
                for (i, m) in modes.iter_mut().enumerate() {
                    let o = CTRL_HDR_LEN + DISPLAY_ONE_LEN * i;
                    *m = DisplayOne {
                        r: Rect::parse_at(b, o),
                        enabled: le32(b, o + 16),
                        flags: le32(b, o + 20),
                    };
                }
                Reply::OkDisplayInfo(modes)
            }
            resp::OK_CAPSET_INFO | resp::OK_CAPSET | resp::OK_EDID => Reply::OkOther(hdr.kind),
            resp::ERR_UNSPEC => Reply::Err(GpuError::Unspec),
            resp::ERR_OUT_OF_MEMORY => Reply::Err(GpuError::OutOfMemory),
            resp::ERR_INVALID_SCANOUT_ID => Reply::Err(GpuError::InvalidScanoutId),
            resp::ERR_INVALID_RESOURCE_ID => Reply::Err(GpuError::InvalidResourceId),
            resp::ERR_INVALID_CONTEXT_ID => Reply::Err(GpuError::InvalidContextId),
            resp::ERR_INVALID_PARAMETER => Reply::Err(GpuError::InvalidParameter),
            other if (0x0100..0x0200).contains(&other) => {
                return Err(format!(
                    "reply type {other:#06x} is a REQUEST code: the buffer still holds what \
                     the driver wrote, not what the device wrote"
                ));
            }
            other => {
                return Err(format!(
                    "reply type {other:#010x} is not a virtio-gpu response code"
                ));
            }
        };
        Ok(Response { hdr, reply })
    }
}

/// Size of `struct virtq_desc`, §2.7.5.
pub const DESC_LEN: usize = 16;
/// `VIRTQ_DESC_F_NEXT`: `next` names the following descriptor of this chain.
pub const DESC_F_NEXT: u16 = 1;
/// `VIRTQ_DESC_F_WRITE`: the device writes this buffer; otherwise it only reads it.
pub const DESC_F_WRITE: u16 = 2;
/// `VIRTQ_DESC_F_INDIRECT`: the buffer is itself a descriptor table. Needs
/// `VIRTIO_F_INDIRECT_DESC`, which no driver in this tree negotiates.
pub const DESC_F_INDIRECT: u16 = 4;

/// One split-virtqueue descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Desc {
    /// Guest-physical address of the buffer.
    pub addr: u64,
    /// Length of the buffer.
    pub len: u32,
    /// `DESC_F_*`.
    pub flags: u16,
    /// The next descriptor, if `NEXT` is set.
    pub next: u16,
}

impl Desc {
    /// The 16 bytes, as `spec/virtio-gpu.sas:337-349` stores them.
    pub fn encode(&self) -> [u8; DESC_LEN] {
        let mut b = [0u8; DESC_LEN];
        b[0..8].copy_from_slice(&self.addr.to_le_bytes());
        b[8..12].copy_from_slice(&self.len.to_le_bytes());
        b[12..14].copy_from_slice(&self.flags.to_le_bytes());
        b[14..16].copy_from_slice(&self.next.to_le_bytes());
        b
    }

    /// Read descriptor `index` out of a descriptor table.
    ///
    /// # Errors
    /// The table does not hold `index`.
    pub fn parse(table: &[u8], index: u16) -> Result<Desc, String> {
        let o = usize::from(index) * DESC_LEN;
        if table.len() < o + DESC_LEN {
            return Err(format!(
                "descriptor {index} is at byte {o}; the table is {} bytes",
                table.len()
            ));
        }
        Ok(Desc {
            addr: le64(table, o),
            len: le32(table, o + 8),
            flags: le16(table, o + 12),
            next: le16(table, o + 14),
        })
    }
}

/// Follow a chain from `head` through a descriptor table of `queue_size` entries.
///
/// # Errors
/// A descriptor past the queue, an `INDIRECT` flag (not negotiated by any driver here), or a
/// chain longer than the queue — which can only mean it loops, and a device that followed
/// it would never finish.
pub fn chain(table: &[u8], head: u16, queue_size: u16) -> Result<Vec<Desc>, String> {
    let mut out = Vec::new();
    let mut at = head;
    loop {
        if at >= queue_size {
            return Err(format!("descriptor {at} is past the queue of {queue_size}"));
        }
        let d = Desc::parse(table, at)?;
        if d.flags & DESC_F_INDIRECT != 0 {
            return Err(format!(
                "descriptor {at} is INDIRECT; VIRTIO_F_INDIRECT_DESC was not negotiated"
            ));
        }
        out.push(d);
        if out.len() > usize::from(queue_size) {
            return Err(format!(
                "the chain from {head} has more than {queue_size} links: it loops"
            ));
        }
        if d.flags & DESC_F_NEXT == 0 {
            return Ok(out);
        }
        at = d.next;
    }
}

/// Split a controlq chain into what the device reads and what it writes, checking the
/// framing rule (§2.7.4): every device-readable descriptor precedes every device-writable
/// one, and there is at least one of each — a request with nowhere to put its reply is not
/// a request the device can complete.
///
/// # Errors
/// A writable descriptor before a readable one, no readable one, or no writable one.
pub fn frame(links: &[Desc]) -> Result<(Vec<Desc>, Vec<Desc>), String> {
    let mut readable = Vec::new();
    let mut writable = Vec::new();
    for (i, d) in links.iter().enumerate() {
        if d.flags & DESC_F_WRITE != 0 {
            writable.push(*d);
        } else if !writable.is_empty() {
            return Err(format!(
                "descriptor {i} of the chain is device-readable after a device-writable \
                 one; §2.7.4 requires readable first"
            ));
        } else {
            readable.push(*d);
        }
    }
    if readable.is_empty() {
        return Err("the chain has no device-readable descriptor: no request".into());
    }
    if writable.is_empty() {
        return Err("the chain has no device-writable descriptor: nowhere for the reply".into());
    }
    Ok((readable, writable))
}
