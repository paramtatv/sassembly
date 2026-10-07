//! The virtio-gpu device behind the controlq — `W-378`: the six 2D commands, served into
//! host-side resources, and the scanout written out as a picture.
//!
//! # What this is
//!
//! `W-351` gave the machine a virtio-mmio transport and `W-352` a queue walk; until this
//! row every chain was answered VIRTIO_GPU_RESP_ERR_UNSPEC, "not served". This is the
//! device that serves them: GET_DISPLAY_INFO, RESOURCE_CREATE_2D, RESOURCE_ATTACH_BACKING,
//! SET_SCANOUT, TRANSFER_TO_HOST_2D and RESOURCE_FLUSH (virtio 1.2 §5.7.6.8). Requests are
//! read with `C-015`'s [`Request::parse`] and answered with its [`Response`], so the bytes
//! on the wire have one statement in this crate. What is new is the STATE: the resources,
//! their backing, their pixels and the scanout.
//!
//! THE PICTURE leaves the machine as a binary PPM (`P6`) when the driver flushes the
//! resource that scanout 0 shows, to the path in [`Gpu::dump`] (`yantra-run` sets it from
//! `YANTRA_SCANOUT`). With no path set, nothing is written and a flush is still served.
//! ONE PATH, OVERWRITTEN: every flush rewrites the same file, so it always holds the LAST
//! frame flushed. That is deliberate — the file is "what the screen shows now" — and it is
//! tested (a second flush with different pixels must leave the second frame). A
//! PPM because it is the simplest image format a stranger can open and a test can compare
//! byte for byte: a header line and then R, G, B per pixel, rows top to bottom.
//!
//! # The choices the specification leaves open, each named
//!
//! §5.7.6.8 says which reply a command gets on success and little about failure. Where it
//! is silent this device chooses, and says so here rather than letting a reader assume a
//! citation:
//!
//! - an unknown resource id, or a CREATE_2D reusing a live one: ERR_INVALID_RESOURCE_ID;
//! - a scanout id other than 0: ERR_INVALID_SCANOUT_ID — this device has one head;
//! - a SET_SCANOUT rectangle the resource does not cover: ERR_INVALID_PARAMETER. This one
//!   IS the spec's rule ("Scanout rectangles must be completely covered by the underlying
//!   resource", `5.7.6.8-gpu-controlq.txt:133`); the code for breaking it is chosen;
//! - a TRANSFER or FLUSH rectangle outside the resource, a TRANSFER reading past the
//!   attached backing, or a backing span outside RAM: ERR_INVALID_PARAMETER;
//! - a TRANSFER with no backing attached: ERR_UNSPEC — there is nothing to read from;
//! - a format other than B8G8R8A8 (1) and B8G8R8X8 (2), both four octets per pixel:
//!   ERR_INVALID_PARAMETER. `C-015` accepts all eight `VIRTIO_GPU_FORMAT_*` values on the
//!   wire; this device serves two, and refuses the rest by name rather than drawing them
//!   in the wrong byte order;
//! - a resource larger than [`MAX_RESOURCE`] octets: ERR_OUT_OF_MEMORY;
//! - a request `C-015` cannot parse: ERR_UNSPEC;
//! - the one display mode, reported by GET_DISPLAY_INFO: scanout 0 enabled at
//!   [`DISPLAY_WIDTH`] × [`DISPLAY_HEIGHT`]. The specification prescribes none.
//!
//! EVERY REFUSAL HAS NO EFFECT: each command checks everything before it changes any
//! state, so a refused CREATE leaves no resource and a refused TRANSFER no pixel.
//!
//! # The transfer
//!
//! TRANSFER_TO_HOST_2D copies `rect` from the backing into the resource. The backing is
//! the concatenation of its spans in the order the driver attached them, and is read as
//! an image the resource's own width wide: row `r` of the rectangle starts at backing
//! octet `offset + r · stride`, `stride` = width · 4, and lands at resource row `rect.y +
//! r`, column `rect.x`. With `offset` = 0 and the whole resource as the rectangle — the
//! frame `C-009` and the GPU-driver project's oracle send — that is a straight copy of the backing.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::virtio_gpu::{Command, FLAG_FENCE, resp};
use crate::virtio_gpu::{
    DisplayOne, GpuError, MAX_SCANOUTS, MemEntry, Rect, Reply, Request, Response,
};

/// The display mode GET_DISPLAY_INFO reports for scanout 0 — chosen, see the margin.
pub const DISPLAY_WIDTH: u32 = 640;
/// See [`DISPLAY_WIDTH`].
pub const DISPLAY_HEIGHT: u32 = 480;
/// The largest resource this device creates, in octets: 64 MiB, a 4096 × 4096 frame.
pub const MAX_RESOURCE: u64 = 64 << 20;
/// The most HOST memory all live resources may hold together: 256 MiB, four
/// largest-size resources. Without it, MAX_RESOURCE bounds one resource and nothing
/// bounds their number — a thousand CREATEs with distinct ids would ask the host for
/// 64 GB, an abort rather than a refusal (a peer session's review). Chosen, not specified.
pub const MAX_TOTAL: u64 = 256 << 20;
/// Octets per pixel for both served formats.
const BPP: u64 = 4;

/// One host-side 2D resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    /// `VIRTIO_GPU_FORMAT_*`: 1 or 2.
    pub format: u32,
    /// Pixels.
    pub width: u32,
    /// Pixels.
    pub height: u32,
    /// The guest spans behind it, in attach order; empty until ATTACH_BACKING.
    pub backing: Vec<MemEntry>,
    /// The host copy, width · height · 4 octets, as the formats lay them out.
    pub pixels: Vec<u8>,
}

/// The device's state. Held inside [`crate::virtio_mmio::VirtioMmio`], so a Status-0
/// reset clears it with the queues.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Gpu {
    /// Live resources by id.
    pub resources: BTreeMap<u32, Resource>,
    /// What scanout 0 shows: the resource and the rectangle of it, or nothing.
    pub scanout: Option<(u32, Rect)>,
    /// Where a flush of scanout 0's resource writes its PPM; `None` writes nothing.
    pub dump: Option<PathBuf>,
    /// Set after the first failed PPM write, so the failure is said once, on stderr.
    pub dump_failed: bool,
}

/// `rect` lies wholly inside a `width` × `height` resource, computed without overflow.
fn covers(width: u32, height: u32, r: &Rect) -> bool {
    let right = u64::from(r.x) + u64::from(r.width);
    let bottom = u64::from(r.y) + u64::from(r.height);
    right <= u64::from(width) && bottom <= u64::from(height)
}

impl Gpu {
    /// Serve one controlq request. `mem` is guest RAM starting at `base`, read-only: a
    /// TRANSFER reads the backing from it. Returns the reply's octets.
    pub fn serve(&mut self, mem: &[u8], base: u64, request: &[u8]) -> Vec<u8> {
        let Ok(req) = Request::parse(request) else {
            return Response::new(Reply::Err(GpuError::Unspec)).encode();
        };
        let reply = match self.command(mem, base, &req.cmd) {
            Ok(r) => r,
            Err(e) => Reply::Err(e),
        };
        let mut out = Response::new(reply);
        // §5.7.6.7: a fenced request's reply carries the fence back.
        if req.hdr.flags & FLAG_FENCE != 0 {
            out.hdr.flags |= FLAG_FENCE;
            out.hdr.fence_id = req.hdr.fence_id;
        }
        out.encode()
    }

    fn command(&mut self, mem: &[u8], base: u64, cmd: &Command) -> Result<Reply, GpuError> {
        match cmd {
            Command::GetDisplayInfo => {
                let mut modes = Box::new([DisplayOne::default(); MAX_SCANOUTS]);
                modes[0] = DisplayOne {
                    r: Rect {
                        x: 0,
                        y: 0,
                        width: DISPLAY_WIDTH,
                        height: DISPLAY_HEIGHT,
                    },
                    enabled: 1,
                    flags: 0,
                };
                Ok(Reply::OkDisplayInfo(modes))
            }
            Command::ResourceCreate2d {
                resource_id,
                format,
                width,
                height,
            } => {
                if self.resources.contains_key(resource_id) {
                    return Err(GpuError::InvalidResourceId);
                }
                if *format != 1 && *format != 2 {
                    return Err(GpuError::InvalidParameter);
                }
                // Checked BEFORE the cap: u32::MAX² · 4 is about 2^66, which wraps a u64
                // to a small number that the cap would then wave through.
                let size = u64::from(*width)
                    .checked_mul(u64::from(*height))
                    .and_then(|n| n.checked_mul(BPP))
                    .filter(|&n| n <= MAX_RESOURCE)
                    .ok_or(GpuError::OutOfMemory)?;
                let held: u64 = self.resources.values().map(|r| r.pixels.len() as u64).sum();
                if held + size > MAX_TOTAL {
                    return Err(GpuError::OutOfMemory);
                }
                self.resources.insert(
                    *resource_id,
                    Resource {
                        format: *format,
                        width: *width,
                        height: *height,
                        backing: Vec::new(),
                        pixels: vec![0; size as usize],
                    },
                );
                Ok(Reply::OkNoData)
            }
            Command::ResourceAttachBacking {
                resource_id,
                entries,
            } => {
                if !self.resources.contains_key(resource_id) {
                    return Err(GpuError::InvalidResourceId);
                }
                let end = base + mem.len() as u64;
                let in_ram = |e: &MemEntry| {
                    e.addr >= base
                        && e.addr
                            .checked_add(u64::from(e.length))
                            .is_some_and(|x| x <= end)
                };
                if !entries.iter().all(in_ram) {
                    return Err(GpuError::InvalidParameter);
                }
                // Each span lies in RAM; their SUM must too. Spans may overlap, so this
                // is not implied, and the transfer below walks the list per row.
                let total: u64 = entries.iter().map(|e| u64::from(e.length)).sum();
                if total > mem.len() as u64 {
                    return Err(GpuError::InvalidParameter);
                }
                if let Some(r) = self.resources.get_mut(resource_id) {
                    r.backing.clone_from(entries);
                }
                Ok(Reply::OkNoData)
            }
            Command::SetScanout {
                rect,
                scanout_id,
                resource_id,
            } => {
                if *scanout_id != 0 {
                    return Err(GpuError::InvalidScanoutId);
                }
                if *resource_id == 0 {
                    self.scanout = None;
                    return Ok(Reply::OkNoData);
                }
                let r = self
                    .resources
                    .get(resource_id)
                    .ok_or(GpuError::InvalidResourceId)?;
                if !covers(r.width, r.height, rect) {
                    return Err(GpuError::InvalidParameter);
                }
                self.scanout = Some((*resource_id, *rect));
                Ok(Reply::OkNoData)
            }
            Command::TransferToHost2d {
                rect,
                offset,
                resource_id,
            } => {
                let r = self
                    .resources
                    .get(resource_id)
                    .ok_or(GpuError::InvalidResourceId)?;
                if r.backing.is_empty() {
                    return Err(GpuError::Unspec);
                }
                if !covers(r.width, r.height, rect) {
                    return Err(GpuError::InvalidParameter);
                }
                // The whole read must lie in the backing, checked BEFORE any pixel
                // changes. The backing is READ IN PLACE, row by row, through the span
                // list: no concatenated copy, whose size a guest would choose
                // (a peer session's review: N full-RAM spans, one transfer, N × RAM on the host).
                let total: u64 = r.backing.iter().map(|e| u64::from(e.length)).sum();
                let stride = u64::from(r.width) * BPP;
                let row = u64::from(rect.width) * BPP;
                if rect.height > 0 {
                    let last = offset
                        .checked_add(u64::from(rect.height - 1) * stride)
                        .and_then(|s| s.checked_add(row));
                    if last.is_none_or(|l| l > total) {
                        return Err(GpuError::InvalidParameter);
                    }
                }
                let r = self
                    .resources
                    .get_mut(resource_id)
                    .ok_or(GpuError::InvalidResourceId)?;
                for k in 0..u64::from(rect.height) {
                    let d = ((u64::from(rect.y) + k) * stride + u64::from(rect.x) * BPP) as usize;
                    gather(
                        mem,
                        base,
                        &r.backing,
                        offset + k * stride,
                        &mut r.pixels[d..d + row as usize],
                    );
                }
                Ok(Reply::OkNoData)
            }
            Command::ResourceFlush { rect, resource_id } => {
                let r = self
                    .resources
                    .get(resource_id)
                    .ok_or(GpuError::InvalidResourceId)?;
                if !covers(r.width, r.height, rect) {
                    return Err(GpuError::InvalidParameter);
                }
                // A failed write is the host's problem, not the guest's: the command was
                // served, and the reply must not depend on the disk.
                // A failed write is said once, on stderr: silent, it would read as "no
                // frame was ever flushed".
                if let (Some((id, shown)), Some(path)) = (self.scanout, &self.dump)
                    && id == *resource_id
                    && let Err(e) = std::fs::write(path, ppm(r, &shown))
                    && !self.dump_failed
                {
                    eprintln!(
                        "virtio-gpu: could not write the scanout to {}: {e}",
                        path.display()
                    );
                    self.dump_failed = true;
                }
                Ok(Reply::OkNoData)
            }
        }
    }
}

/// Copy `dst.len()` octets of the backing, starting `off` octets into it, where the
/// backing is `spans` laid end to end. The caller has checked the read lies within the
/// spans' total and each span within RAM, so every slice here is in bounds.
fn gather(mem: &[u8], base: u64, spans: &[MemEntry], mut off: u64, dst: &mut [u8]) {
    let mut filled = 0usize;
    for e in spans {
        if filled == dst.len() {
            break;
        }
        let len = u64::from(e.length);
        if off >= len {
            off -= len;
            continue;
        }
        let take = (len - off).min((dst.len() - filled) as u64) as usize;
        let at = (e.addr - base + off) as usize;
        dst[filled..filled + take].copy_from_slice(&mem[at..at + take]);
        filled += take;
        off = 0;
    }
}

/// `shown` of resource `r` as a binary PPM: `P6\n<w> <h>\n255\n`, then R, G, B per pixel,
/// rows top to bottom. Both served formats keep B, G, R in octets 0, 1, 2.
#[must_use]
pub fn ppm(r: &Resource, shown: &Rect) -> Vec<u8> {
    let mut out = format!("P6\n{} {}\n255\n", shown.width, shown.height).into_bytes();
    let stride = r.width as usize * BPP as usize;
    for y in shown.y..shown.y + shown.height {
        for x in shown.x..shown.x + shown.width {
            let p = y as usize * stride + x as usize * BPP as usize;
            out.extend_from_slice(&[r.pixels[p + 2], r.pixels[p + 1], r.pixels[p]]);
        }
    }
    out
}

/// The reply codes this device can send, so a reader can find them in one place.
pub const CODES: [u32; 7] = [
    resp::OK_NODATA,
    resp::OK_DISPLAY_INFO,
    resp::ERR_UNSPEC,
    resp::ERR_OUT_OF_MEMORY,
    resp::ERR_INVALID_SCANOUT_ID,
    resp::ERR_INVALID_RESOURCE_ID,
    resp::ERR_INVALID_PARAMETER,
];
