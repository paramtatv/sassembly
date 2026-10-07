//! `C-015` — the virtio-gpu 2D wire format, first unit: bytes in, bytes out, and what is
//! refused.
//!
//! # What these tests are for
//!
//! `spec/virtio-gpu.sas` drives a real virtio-gpu under QEMU and `tools/check-virtio-gpu.sh`
//! photographs the screen. That is the row's evidence and it lives outside `crates/`.
//! `C-015` asks for evidence **under `crates/`**, and the first thing a test here can hold
//! the driver to is its bytes: the 40-byte `RESOURCE_CREATE_2D` it stores at `Q + 0x2000`,
//! the two descriptors it writes at `Q + 0` and `Q + 16`, the 24-byte reply it reads back.
//! This file pins those down against the spec and against the program, so that when the
//! device model (unit two) runs the program under `yantra`, the parser it hands the bytes
//! to has already been shown to agree with the driver.
//!
//! # The three claims, and how each is falsifiable
//!
//! 1. **The encoder writes what `spec/virtio-gpu.sas` writes.** The golden bytes in
//!    [`resource_create_2d_lays_its_bytes_where_spec_virtio_gpu_sas_stores_them`] are
//!    written out by hand from the program's stores (`virtio-gpu.sas:201-210`), **not**
//!    produced by the encoder — an encoder tested against itself is a mirror, and this
//!    crate's `tests/interpreter.rs` says why that is refused.
//! 2. **Every refusal fires on the bytes it names, and only those.** A response header
//!    shorter than 24 bytes, the `0xffffffff` fill, a request code in the reply slot,
//!    resource id 0 in four commands — each has a test that gives the bad bytes and one
//!    that gives the nearest good ones. `SET_SCANOUT` with id 0 is the control: the spec
//!    makes it legal, so a parser that refuses it has refused too much.
//! 3. **One round trip over the queue the program builds.** A fake guest memory laid out
//!    exactly as the program's `क्षेत्रम्` — descriptors at `Q`, command at `Q + 0x2000`,
//!    reply at `Q + 0x2100`, reply pre-filled with `0xff` — is walked from the descriptor
//!    table, framed, parsed, answered and read back. This is the transport as it exists in
//!    this tree: the program's, in Sassembly. There is no Rust virtqueue to drive.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use yantra::virtio_gpu::{
    CTRL_HDR_LEN, Command, DESC_F_NEXT, DESC_F_WRITE, DESC_LEN, DISPLAY_INFO_LEN, Desc, DisplayOne,
    FLAG_FENCE, FORMAT_B8G8R8X8_UNORM, GpuError, Header, MAX_SCANOUTS, MemEntry, Rect, Reply,
    Request, Response, chain, cmd, frame, resp,
};

fn put32(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn put64(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}

/// A refusal must name what it saw; asserting on a fragment keeps the message honest
/// without freezing its wording.
fn refused(r: Result<impl std::fmt::Debug, String>, fragment: &str) {
    match r {
        Ok(v) => panic!("accepted {v:?}; expected a refusal mentioning {fragment:?}"),
        Err(e) => assert!(
            e.contains(fragment),
            "refused, but the reason {e:?} does not mention {fragment:?}"
        ),
    }
}

// ---- 1. the header -------------------------------------------------------------------

#[test]
fn the_header_is_twenty_four_bytes_and_round_trips() {
    let h = Header {
        kind: cmd::RESOURCE_FLUSH,
        flags: FLAG_FENCE,
        fence_id: 0x0102_0304_0506_0708,
        ctx_id: 7,
        ring_idx: 3,
    };
    let b = h.encode();
    assert_eq!(b.len(), CTRL_HDR_LEN);
    assert_eq!(&b[0..4], &0x0104u32.to_le_bytes(), "type at +0");
    assert_eq!(&b[4..8], &1u32.to_le_bytes(), "flags at +4");
    assert_eq!(
        &b[8..16],
        &0x0102_0304_0506_0708u64.to_le_bytes(),
        "fence_id at +8"
    );
    assert_eq!(&b[16..20], &7u32.to_le_bytes(), "ctx_id at +16");
    assert_eq!(b[20], 3, "ring_idx at +20");
    assert_eq!(&b[21..24], &[0, 0, 0], "three bytes of padding");
    assert_eq!(Header::parse(&b), Ok(h));
}

#[test]
fn a_header_shorter_than_twenty_four_bytes_is_refused() {
    let full = Header::plain(cmd::GET_DISPLAY_INFO).encode();
    for n in 0..CTRL_HDR_LEN {
        refused(Header::parse(&full[..n]), "24 bytes");
        refused(Request::parse(&full[..n]), "24 bytes");
        refused(Response::parse(&full[..n]), "24 bytes");
    }
    assert!(Header::parse(&full).is_ok(), "exactly 24 is enough");
}

#[test]
fn unknown_flag_bits_are_refused_and_the_two_defined_ones_are_not() {
    let mut b = Header::plain(cmd::GET_DISPLAY_INFO).encode();
    put32(&mut b, 4, 0b11);
    assert!(
        Header::parse(&b).is_ok(),
        "FENCE | INFO_RING_IDX are the defined bits"
    );
    put32(&mut b, 4, 0b100);
    refused(Header::parse(&b), "bits the spec does not define");
    put32(&mut b, 4, 0x8000_0000);
    refused(Header::parse(&b), "bits the spec does not define");
}

// ---- 2. the six commands -------------------------------------------------------------

fn the_six() -> Vec<(Command, usize)> {
    let rect = Rect {
        x: 0,
        y: 0,
        width: 64,
        height: 32,
    };
    vec![
        (Command::GetDisplayInfo, 24),
        (
            Command::ResourceCreate2d {
                resource_id: 1,
                format: FORMAT_B8G8R8X8_UNORM,
                width: 64,
                height: 32,
            },
            40,
        ),
        (
            Command::ResourceAttachBacking {
                resource_id: 1,
                entries: vec![MemEntry {
                    addr: 0x8020_3000,
                    length: 8192,
                }],
            },
            48,
        ),
        (
            Command::SetScanout {
                rect,
                scanout_id: 0,
                resource_id: 1,
            },
            48,
        ),
        (
            Command::TransferToHost2d {
                rect,
                offset: 0,
                resource_id: 1,
            },
            56,
        ),
        (
            Command::ResourceFlush {
                rect,
                resource_id: 1,
            },
            48,
        ),
    ]
}

#[test]
fn each_of_the_six_controlq_commands_round_trips_at_its_spec_length() {
    // 24, 40, 48, 48, 56, 48 are also the values `spec/virtio-gpu.sas` loads into स्थिर७
    // before each कार्यम् (lines 211, 231, 248, 266, 283), except GET_DISPLAY_INFO, which
    // that program never issues — `C-009`'s row says so, and this is where it starts to.
    for (c, len) in the_six() {
        let r = Request::new(c.clone());
        let b = r.encode();
        assert_eq!(b.len(), len, "{c:?} is {len} bytes on the wire");
        assert_eq!(c.wire_len(), len);
        let back = Request::parse(&b).unwrap_or_else(|e| panic!("{c:?}: {e}"));
        assert_eq!(back, r);
        assert_eq!(back.hdr.kind, c.kind());
    }
}

#[test]
fn resource_create_2d_lays_its_bytes_where_spec_virtio_gpu_sas_stores_them() {
    // Hand-built from the program's stores, NOT from the encoder:
    //   virtio-gpu.sas:201-202  type 257 = 0x0101 at +0
    //   :203-204                resource id 1 at +24
    //   :205-206                format 2 at +28
    //   :207-208                width 64 at +32
    //   :209-210                height 32 at +36
    //   :211                    length 40
    let mut golden = [0u8; 40];
    put32(&mut golden, 0, 257);
    put32(&mut golden, 24, 1);
    put32(&mut golden, 28, 2);
    put32(&mut golden, 32, 64);
    put32(&mut golden, 36, 32);

    let want = Request::new(Command::ResourceCreate2d {
        resource_id: 1,
        format: 2,
        width: 64,
        height: 32,
    });
    assert_eq!(Request::parse(&golden), Ok(want.clone()));
    assert_eq!(want.encode(), golden.to_vec());
}

#[test]
fn attach_backing_lays_its_entry_where_spec_virtio_gpu_sas_stores_it() {
    //   virtio-gpu.sas:222  type 262 = 0x0106     :225  resource id 1 at +24
    //   :227                nr_entries 1 at +28    :228  addr (8 bytes) at +32
    //   :229-230            length 8192 at +40     :231  length 48
    let mut golden = [0u8; 48];
    put32(&mut golden, 0, 262);
    put32(&mut golden, 24, 1);
    put32(&mut golden, 28, 1);
    put64(&mut golden, 32, 0x8020_3000);
    put32(&mut golden, 40, 8192);
    let want = Request::new(Command::ResourceAttachBacking {
        resource_id: 1,
        entries: vec![MemEntry {
            addr: 0x8020_3000,
            length: 8192,
        }],
    });
    assert_eq!(Request::parse(&golden), Ok(want.clone()));
    assert_eq!(want.encode(), golden.to_vec());
}

#[test]
fn set_scanout_transfer_and_flush_put_the_resource_id_where_the_program_does() {
    // SET_SCANOUT: rect at +24, scanout at +40, resource at +44 (virtio-gpu.sas:238-247).
    let mut b = [0u8; 48];
    put32(&mut b, 0, 259);
    put32(&mut b, 32, 64);
    put32(&mut b, 36, 32);
    put32(&mut b, 44, 1);
    match Request::parse(&b).map(|r| r.cmd) {
        Ok(Command::SetScanout {
            rect,
            scanout_id,
            resource_id,
        }) => {
            assert_eq!(
                (rect.width, rect.height, scanout_id, resource_id),
                (64, 32, 0, 1)
            );
        }
        other => panic!("{other:?}"),
    }
    // TRANSFER_TO_HOST_2D: rect +24, offset +40 (8 bytes), resource +48 (:255-265).
    let mut b = [0u8; 56];
    put32(&mut b, 0, 261);
    put32(&mut b, 32, 64);
    put32(&mut b, 36, 32);
    put64(&mut b, 40, 260);
    put32(&mut b, 48, 1);
    match Request::parse(&b).map(|r| r.cmd) {
        Ok(Command::TransferToHost2d {
            offset,
            resource_id,
            ..
        }) => assert_eq!((offset, resource_id), (260, 1)),
        other => panic!("{other:?}"),
    }
    // RESOURCE_FLUSH: rect +24, resource +40 (:273-282).
    let mut b = [0u8; 48];
    put32(&mut b, 0, 260);
    put32(&mut b, 32, 64);
    put32(&mut b, 36, 32);
    put32(&mut b, 40, 1);
    match Request::parse(&b).map(|r| r.cmd) {
        Ok(Command::ResourceFlush { resource_id, .. }) => assert_eq!(resource_id, 1),
        other => panic!("{other:?}"),
    }
}

#[test]
fn trailing_bytes_after_a_command_are_ignored_as_a_device_ignores_them() {
    let mut b = Request::new(Command::GetDisplayInfo).encode();
    b.extend_from_slice(&[0xaa; 40]);
    assert_eq!(
        Request::parse(&b).map(|r| r.cmd),
        Ok(Command::GetDisplayInfo)
    );
}

// ---- 3. refusals on the request side -------------------------------------------------

#[test]
fn resource_id_zero_is_refused_for_create_attach_transfer_and_flush() {
    for (c, _) in the_six() {
        let mut r = Request::new(c);
        let zeroed = match &mut r.cmd {
            Command::ResourceCreate2d { resource_id, .. }
            | Command::ResourceAttachBacking { resource_id, .. }
            | Command::TransferToHost2d { resource_id, .. }
            | Command::ResourceFlush { resource_id, .. } => {
                *resource_id = 0;
                true
            }
            Command::GetDisplayInfo | Command::SetScanout { .. } => false,
        };
        if zeroed {
            refused(Request::parse(&r.encode()), "resource id 0");
        }
    }
}

#[test]
fn set_scanout_with_resource_id_zero_is_accepted_because_it_disables_the_head() {
    // The control for the test above. §5.7.6.8: "resource_id 0 disables the scanout".
    let r = Request::new(Command::SetScanout {
        rect: Rect::default(),
        scanout_id: 0,
        resource_id: 0,
    });
    assert_eq!(Request::parse(&r.encode()), Ok(r));
}

#[test]
fn a_zero_area_or_unknown_format_resource_is_refused() {
    let mk = |format, width, height| {
        Request::new(Command::ResourceCreate2d {
            resource_id: 1,
            format,
            width,
            height,
        })
        .encode()
    };
    refused(Request::parse(&mk(2, 0, 32)), "no pixels");
    refused(Request::parse(&mk(2, 64, 0)), "no pixels");
    refused(Request::parse(&mk(0, 64, 32)), "VIRTIO_GPU_FORMAT_");
    refused(Request::parse(&mk(5, 64, 32)), "VIRTIO_GPU_FORMAT_");
    assert!(
        Request::parse(&mk(134, 64, 32)).is_ok(),
        "R8G8B8X8_UNORM = 134 is real"
    );
}

#[test]
fn attach_backing_refuses_a_count_its_bytes_cannot_hold_and_a_count_of_zero() {
    let one = Request::new(Command::ResourceAttachBacking {
        resource_id: 1,
        entries: vec![MemEntry { addr: 0, length: 1 }],
    })
    .encode();
    let mut claims_two = one.clone();
    put32(&mut claims_two, 28, 2);
    refused(Request::parse(&claims_two), "names 2 entries");
    let mut claims_none = one.clone();
    put32(&mut claims_none, 28, 0);
    refused(Request::parse(&claims_none), "nr_entries 0");
    assert!(Request::parse(&one).is_ok());
}

#[test]
fn a_request_type_that_is_no_command_or_is_a_reply_code_is_refused() {
    let mut b = Header::plain(0).encode();
    refused(Request::parse(&b), "not a virtio-gpu controlq command");
    put32(&mut b, 0, resp::OK_NODATA);
    refused(Request::parse(&b), "REPLY code in a request slot");
    put32(&mut b, 0, cmd::RESOURCE_UNREF);
    refused(Request::parse(&b), "outside C-015's 2D set");
}

// ---- 4. the reply side ---------------------------------------------------------------

#[test]
fn ok_nodata_is_twenty_four_bytes_of_0x1100_and_round_trips() {
    let r = Response::new(Reply::OkNoData);
    let b = r.encode();
    assert_eq!(b.len(), 24);
    assert_eq!(&b[0..4], &0x1100u32.to_le_bytes());
    assert_eq!(Response::parse(&b), Ok(r));
}

#[test]
fn an_untouched_reply_buffer_is_refused_by_name() {
    // `spec/virtio-gpu.sas:330-335` fills the reply with 0xff first, so that "the device
    // never wrote" reads back as ffffffff. The parser must say exactly that, not "unknown".
    let fill = [0xffu8; 64];
    refused(Response::parse(&fill), "never wrote");
}

#[test]
fn a_request_code_in_the_reply_slot_is_refused() {
    let still_the_request = Request::new(Command::GetDisplayInfo).encode();
    refused(Response::parse(&still_the_request), "REQUEST code");
}

#[test]
fn every_error_code_decodes_to_its_name_and_back() {
    let all = [
        (0x1200, GpuError::Unspec),
        (0x1201, GpuError::OutOfMemory),
        (0x1202, GpuError::InvalidScanoutId),
        (0x1203, GpuError::InvalidResourceId),
        (0x1204, GpuError::InvalidContextId),
        (0x1205, GpuError::InvalidParameter),
    ];
    for (code, e) in all {
        assert_eq!(e.code(), code);
        let b = Header::plain(code).encode();
        assert_eq!(Response::parse(&b).map(|r| r.reply), Ok(Reply::Err(e)));
        assert_eq!(Response::new(Reply::Err(e)).encode(), b.to_vec());
    }
    let b = Header::plain(0x1206).encode();
    refused(Response::parse(&b), "not a virtio-gpu response code");
}

#[test]
fn a_display_info_reply_needs_all_sixteen_modes() {
    let mut modes = Box::new([DisplayOne::default(); MAX_SCANOUTS]);
    modes[0] = DisplayOne {
        r: Rect {
            x: 0,
            y: 0,
            width: 1024,
            height: 768,
        },
        enabled: 1,
        flags: 0,
    };
    let r = Response::new(Reply::OkDisplayInfo(modes));
    let b = r.encode();
    assert_eq!(b.len(), DISPLAY_INFO_LEN);
    assert_eq!(DISPLAY_INFO_LEN, 408, "24 + 16 * 24");
    assert_eq!(Response::parse(&b), Ok(r.clone()));
    // The 64-byte reply buffer `spec/virtio-gpu.sas` gives every command (line 345) is too
    // small for this reply. That is the measured reason GET_DISPLAY_INFO needs its own
    // buffer when the program starts issuing it, and this refusal is what says so.
    refused(Response::parse(&b[..64]), "408");
    match Response::parse(&b).map(|r| r.reply) {
        Ok(Reply::OkDisplayInfo(m)) => {
            assert_eq!((m[0].r.width, m[0].r.height, m[0].enabled), (1024, 768, 1));
            assert!(m[1..].iter().all(|d| d.enabled == 0));
        }
        other => panic!("{other:?}"),
    }
}

// ---- 5. the queue the program builds -------------------------------------------------

#[test]
fn a_descriptor_round_trips_and_a_missing_one_is_refused() {
    let d = Desc {
        addr: 0x8020_2000,
        len: 40,
        flags: DESC_F_NEXT,
        next: 1,
    };
    let table = d.encode();
    assert_eq!(table.len(), DESC_LEN);
    assert_eq!(Desc::parse(&table, 0), Ok(d));
    refused(Desc::parse(&table, 1), "the table is 16 bytes");
}

#[test]
fn a_chain_that_loops_or_leaves_the_queue_is_refused() {
    let mut table = vec![0u8; 8 * DESC_LEN];
    // 0 -> 1 -> 0: a loop.
    table[0..16].copy_from_slice(
        &Desc {
            addr: 0,
            len: 1,
            flags: DESC_F_NEXT,
            next: 1,
        }
        .encode(),
    );
    table[16..32].copy_from_slice(
        &Desc {
            addr: 0,
            len: 1,
            flags: DESC_F_NEXT,
            next: 0,
        }
        .encode(),
    );
    refused(chain(&table, 0, 8), "it loops");
    // 2 -> 9 with a queue of 8.
    table[32..48].copy_from_slice(
        &Desc {
            addr: 0,
            len: 1,
            flags: DESC_F_NEXT,
            next: 9,
        }
        .encode(),
    );
    refused(chain(&table, 2, 8), "past the queue of 8");
    refused(chain(&table, 8, 8), "past the queue of 8");
}

#[test]
fn a_reply_before_the_request_or_a_chain_missing_either_is_refused() {
    let rd = Desc {
        addr: 0,
        len: 40,
        flags: 0,
        next: 0,
    };
    let wr = Desc {
        addr: 0,
        len: 64,
        flags: DESC_F_WRITE,
        next: 0,
    };
    refused(frame(&[wr, rd]), "readable first");
    refused(frame(&[rd]), "nowhere for the reply");
    refused(frame(&[wr]), "no request");
    assert_eq!(frame(&[rd, wr]), Ok((vec![rd], vec![wr])));
}

#[test]
fn the_two_descriptor_chain_spec_virtio_gpu_sas_builds_carries_one_request_and_one_reply() {
    // Guest memory as the program lays it out (virtio-gpu.sas:37-43), with Q at 0 here:
    //   Q + 0       descriptor table, 8 x 16
    //   Q + 0x2000  the command; Q + 0x2100 the reply, pre-filled with 0xff (:330-335)
    const Q: u64 = 0x8020_1000;
    let mut mem = vec![0u8; 0x3000];
    let cmd_at = 0x2000usize;
    let rsp_at = 0x2100usize;
    mem[rsp_at..rsp_at + 64].fill(0xff);

    // The driver's half: store the command, write the two descriptors (:337-349).
    let request = Request::new(Command::ResourceCreate2d {
        resource_id: 1,
        format: FORMAT_B8G8R8X8_UNORM,
        width: 64,
        height: 32,
    });
    let bytes = request.encode();
    mem[cmd_at..cmd_at + bytes.len()].copy_from_slice(&bytes);
    let d0 = Desc {
        addr: Q + cmd_at as u64,
        len: 40,
        flags: DESC_F_NEXT,
        next: 1,
    };
    let d1 = Desc {
        addr: Q + rsp_at as u64,
        len: 64,
        flags: DESC_F_WRITE,
        next: 0,
    };
    mem[0..16].copy_from_slice(&d0.encode());
    mem[16..32].copy_from_slice(&d1.encode());

    // Before the device answers, the reply slot is the fill — and the parser says so.
    refused(Response::parse(&mem[rsp_at..rsp_at + 64]), "never wrote");

    // The device's half: walk the chain from head 0, frame it, read the request.
    let links = chain(&mem[..8 * DESC_LEN], 0, 8).expect("a two-link chain");
    assert_eq!(links, vec![d0, d1]);
    let (readable, writable) = frame(&links).expect("one readable, one writable");
    let at = |d: &Desc| {
        let o = usize::try_from(d.addr - Q).expect("inside the region");
        o..o + usize::try_from(d.len).expect("a length")
    };
    let got = Request::parse(&mem[at(&readable[0])]).expect("the driver's request");
    assert_eq!(got, request);

    // Answer it, as a device would, into the writable descriptor; read it back as the
    // driver does at virtio-gpu.sas:355.
    let answer = Response::new(Reply::OkNoData).encode();
    let slot = at(&writable[0]);
    assert!(
        answer.len() <= slot.len(),
        "OK_NODATA fits the 64-byte reply buffer"
    );
    mem[slot.start..slot.start + answer.len()].copy_from_slice(&answer);
    let read_back = Response::parse(&mem[slot]).expect("the device's reply");
    assert_eq!(read_back.reply, Reply::OkNoData);
    assert_eq!(
        read_back.hdr.kind,
        resp::OK_NODATA,
        "the 0x1100 the check script pairs by position"
    );
}

// ---- 6. the .t1 port of C-009, run on yantra -----------------------------------------
//
// `spec/virtio-gpu.t1` is the first half of `C-009` written in `.t1`: it builds the queue
// region at `C-009`'s offsets for each of the five commands that program issues and
// emits four windows of it on the console after each one. It touches no device: the MMIO
// primitive discovery, init and notify need does not exist in `.t1` yet. What it can be
// held to is its BYTES, and this section holds it to two independent references:
//
// * the command octets against `C-015`'s `Request::encode` for the same command and the
//   same values, and
// * the descriptor, avail and reply octets against goldens written out by hand from
//   `spec/virtio-gpu.sas`'s stores, line by line — NOT from `Desc::encode`, which is
//   only compared afterwards.
//
// The frame the program emits per command is 140 octets, in this order (its header):
//   Q + 0       descriptors 0 and 1          32
//   Q + 128     avail flags, idx, ring[8]    20
//   Q + 0x2000  the command buffer           64 (what शुद्धिः zeroes, :312-322)
//   Q + 0x2100  the reply buffer             24 (what कार्यम् fills with 0xff, :332-335)

/// The base the port is given, ruled by a peer session: `0xA080_0000`.
const PORT_Q: u64 = 0xA080_0000;
/// The `.t1` module `spec/virtio-gpu.t1` declares.
const PORT_MODULE: &str = "चित्रपङ्क्तिः";
/// One command's frame on the console: 32 + 20 + 64 + 24.
const FRAME: usize = 140;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn port_source() -> String {
    std::fs::read_to_string(spec_root().join("virtio-gpu.t1")).expect("read spec/virtio-gpu.t1")
}

/// Compile a `.t1` source to an image the way `t1_file_end_to_end.rs` does: the `.t1`
/// compiler, run by the interpreter over the chain.
fn build_t1(src: &str, module: &str) -> Vec<u8> {
    let octets = |b: &[u8]| Value::Octets(Octets::new(b));
    let arena = |vs: Vec<Value>| Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)));
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets(module.as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    it.call(
        "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
        vec![
            arena(vec![octets(src.as_bytes())]),
            arena(vec![octets(module.as_bytes())]),
            Value::Int(1),
        ],
        80_000_000_000,
    )
    .expect("मण्डलानिप्रतिबिम्बम् runs")
    .octets()
    .map(|o| o.as_slice().to_vec())
    .unwrap_or_default()
}

/// Build and run a source; answer the console octets. The halt is checked FIRST: a
/// program that faulted part-way would emit a prefix, and a prefix must not be judged.
fn run_port(src: &str) -> Vec<u8> {
    let image = build_t1(src, PORT_MODULE);
    assert!(!image.is_empty(), "spec/virtio-gpu.t1 built no image");
    let mut m =
        yantra::Machine::load_elf(&image, yantra::ram_for(&image)).expect("the image loads");
    let mut out = Vec::new();
    match m.run(200_000_000, &mut out) {
        yantra::Halt::Finisher {
            status: Some(0), ..
        } => {}
        other => panic!("spec/virtio-gpu.t1 did not finish cleanly: {other:?}"),
    }
    out
}

/// The unmodified port's output, built once for every test that reads it.
fn port_output() -> &'static [u8] {
    static OUT: OnceLock<Vec<u8>> = OnceLock::new();
    OUT.get_or_init(|| run_port(&port_source()))
}

/// `C-009`'s five commands, in its order, with its values (`virtio-gpu.sas:196-287`), and
/// the avail idx each is submitted under (`स्थिर८`, :212, :232, :249, :267, :284).
fn c009_five(q: u64) -> Vec<(&'static str, Command, u16)> {
    let rect = Rect {
        x: 0,
        y: 0,
        width: 64,
        height: 32,
    };
    vec![
        (
            "RESOURCE_CREATE_2D",
            Command::ResourceCreate2d {
                resource_id: 1,
                format: FORMAT_B8G8R8X8_UNORM,
                width: 64,
                height: 32,
            },
            1,
        ),
        (
            "RESOURCE_ATTACH_BACKING",
            Command::ResourceAttachBacking {
                resource_id: 1,
                // :113-114 स्थिर५ = Q + 0x3000, the framebuffer; :229 8192 = 64 x 32 x 4.
                entries: vec![MemEntry {
                    addr: q + 0x3000,
                    length: 8192,
                }],
            },
            2,
        ),
        (
            "SET_SCANOUT",
            Command::SetScanout {
                rect,
                scanout_id: 0,
                resource_id: 1,
            },
            3,
        ),
        (
            "TRANSFER_TO_HOST_2D",
            Command::TransferToHost2d {
                rect,
                offset: 0,
                resource_id: 1,
            },
            4,
        ),
        (
            "RESOURCE_FLUSH",
            Command::ResourceFlush {
                rect,
                resource_id: 1,
            },
            5,
        ),
    ]
}

fn put16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

/// Judge the port's console output against `C-015` and `C-009`. `Err` names the command
/// and the window that differs, so the negative controls below can assert WHICH check
/// fired rather than merely that one did.
fn judge(out: &[u8], q: u64) -> Result<(), String> {
    let five = c009_five(q);
    if out.len() != five.len() * FRAME {
        return Err(format!(
            "the port emitted {} octets; five frames of {FRAME} are {}",
            out.len(),
            five.len() * FRAME
        ));
    }
    for (i, (name, cmd, idx)) in five.into_iter().enumerate() {
        let f = &out[i * FRAME..(i + 1) * FRAME];
        let (descs, avail, command, reply) = (&f[0..32], &f[32..52], &f[52..116], &f[116..140]);

        // The command: exactly C-015's encode, then zeros to 64 (शुद्धिः, :312-322).
        let want = Request::new(cmd).encode();
        let len = want.len();
        if command[..len] != want[..] {
            return Err(format!(
                "{name}: command octets {:02x?} are not C-015's encode {want:02x?}",
                &command[..len]
            ));
        }
        if command[len..].iter().any(|&b| b != 0) {
            return Err(format!(
                "{name}: the command buffer past its {len} octets is not the zero \
                 शुद्धिः left: {:02x?}",
                &command[len..]
            ));
        }

        // The descriptors, by hand from virtio-gpu.sas.
        let mut golden = [0u8; 32];
        put64(&mut golden, 0, q + 0x2000); // :337  desc0.addr = स्थिर३ = Q + 0x2000
        put32(&mut golden, 8, u32::try_from(len).expect("a length")); // :338 desc0.len = स्थिर७
        put16(&mut golden, 12, 1); //         :339-340 desc0.flags = NEXT
        put16(&mut golden, 14, 1); //         :341  desc0.next = 1
        put64(&mut golden, 16, q + 0x2100); // :344 desc1.addr = स्थिर४ = Q + 0x2100
        put32(&mut golden, 24, 64); //         :345-346 desc1.len = 64
        put16(&mut golden, 28, 2); //          :347-348 desc1.flags = WRITE
        put16(&mut golden, 30, 0); //          :349  desc1.next = 0
        if descs != golden {
            return Err(format!(
                "{name}: descriptors {descs:02x?} are not what virtio-gpu.sas:337-349 \
                 stores, {golden:02x?}"
            ));
        }

        // The avail ring: flags 0, idx, ring[8] all 0 (:166-174, :364).
        let mut golden = [0u8; 20];
        put16(&mut golden, 2, idx);
        if avail != golden {
            return Err(format!(
                "{name}: avail ring {avail:02x?} is not flags 0, idx {idx}, ring[8] 0 \
                 (virtio-gpu.sas:166-174, :364)"
            ));
        }

        // The reply: the 24 octets of 0xff (:332-335).
        if reply.iter().any(|&b| b != 0xff) {
            return Err(format!(
                "{name}: reply buffer {reply:02x?} is not the 24 octets of 0xff \
                 virtio-gpu.sas:332-335 stores"
            ));
        }
    }
    Ok(())
}

#[test]
fn the_t1_port_emits_c009s_five_commands_octet_for_octet() {
    let out = port_output();
    assert!(
        !out.is_empty(),
        "the console is EMPTY — the program halted cleanly, so the octets never reached \
         the sink"
    );
    if let Err(e) = judge(out, PORT_Q) {
        panic!("{e}");
    }
}

#[test]
fn the_t1_ports_queue_is_one_a_device_can_walk_and_answer() {
    // The same octets read back through C-015's DEVICE side, which `judge` never uses:
    // walk the chain from head 0, frame it, parse the request, and see the reply slot
    // refused as "never wrote" — the device's view of what the driver left for it.
    let out = port_output();
    for (i, (name, cmd, idx)) in c009_five(PORT_Q).into_iter().enumerate() {
        let f = &out[i * FRAME..(i + 1) * FRAME];
        let mut table = vec![0u8; 8 * DESC_LEN];
        table[..32].copy_from_slice(&f[0..32]);
        // The ring slot this submission used is (idx - 1) mod 8; ring[] starts at +4.
        let slot = 32 + 4 + 2 * (usize::from(idx - 1) % 8);
        let head = u16::from_le_bytes([f[slot], f[slot + 1]]);
        assert_eq!(head, 0, "{name}: the avail ring names head 0");
        let links = chain(&table, head, 8).unwrap_or_else(|e| panic!("{name}: {e}"));
        let (readable, writable) = frame(&links).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(readable[0].addr, PORT_Q + 0x2000, "{name}");
        assert_eq!(writable[0].addr, PORT_Q + 0x2100, "{name}");
        let len = usize::try_from(readable[0].len).expect("a length");
        assert_eq!(
            Request::parse(&f[52..52 + len]),
            Ok(Request::new(cmd)),
            "{name}: the device parses back what C-009 sent"
        );
        refused(Response::parse(&f[116..140]), "never wrote");
    }
}

#[test]
fn the_judge_refuses_a_single_wrong_octet_in_each_window() {
    // NEGATIVE CONTROL, octet level: a judge that passes everything measures nothing. One
    // octet is changed in each of the four windows of a different frame, and each must be
    // refused with the command and the window named.
    let good = port_output();
    let cases = [
        // (frame, octet within the frame, what the refusal must name)
        (0, 52 + 32, "RESOURCE_CREATE_2D: command octets"), // width 64 -> 65
        (1, 8, "RESOURCE_ATTACH_BACKING: descriptors"),     // desc0.len
        (2, 32 + 2, "SET_SCANOUT: avail ring"),             // idx
        (3, 116 + 23, "TRANSFER_TO_HOST_2D: reply buffer"), // the last fill octet
        (4, 52 + 60, "RESOURCE_FLUSH: the command buffer past"), // a tail zero
    ];
    for (n, o, fragment) in cases {
        let mut bad = good.to_vec();
        bad[n * FRAME + o] ^= 1;
        refused(judge(&bad, PORT_Q), fragment);
    }
    refused(judge(&good[..good.len() - 1], PORT_Q), "five frames");
    refused(judge(good, PORT_Q + 0x1000), "descriptors");
}

#[test]
fn a_mutated_port_is_caught_end_to_end() {
    // NEGATIVE CONTROL, source level: the whole pipeline — the .t1 compiler, the machine,
    // the console, the judge — must be able to fail. SET_SCANOUT's resource id (+44,
    // virtio-gpu.sas:246-247) is changed from 1 to 2 in the SOURCE; the rebuilt program
    // must run cleanly and be refused, by name, at that command.
    let src = port_source();
    let line = "लिख आरभ्य लक्ष्यम् ऽ ०षोड्२०२इ ऽ १ ऽ ४ समाप्तम्";
    assert_eq!(
        src.matches(line).count(),
        1,
        "the mutation site moved: exactly one store of resource id 1 at +44 is expected, \
         SET_SCANOUT's — without it this control mutates nothing and passes vacuously"
    );
    let mutated = src.replacen(line, "लिख आरभ्य लक्ष्यम् ऽ ०षोड्२०२इ ऽ २ ऽ ४ समाप्तम्", 1);
    let out = run_port(&mutated);
    refused(judge(&out, PORT_Q), "SET_SCANOUT: command octets");
}
