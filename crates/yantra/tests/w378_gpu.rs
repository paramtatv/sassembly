//! `W-378`: the virtio-gpu device serves the six 2D commands into host-side resources and
//! writes scanout 0 out as a PPM.
//!
//! Requests are PACKED HERE, by hand, from the struct layouts in
//! `research/spec-extracts/virtio-1.2-csd01/5.7.6.7-gpu-request-header.txt` and
//! `5.7.6.8-gpu-controlq.txt` — never with `virtio_gpu::Request::encode`, which is what the
//! device parses with, so the test and the device do not share one statement of the wire.
//! The frame is the one the GPU-driver project's foreign oracle (a private repository `reference/virtio_gpu.py`)
//! packs, and its checksum is asserted equal to the number that oracle prints before the
//! device sees a byte. Reply codes are the spec's literals (5.7.6.7), not the crate's.

use yantra::gpu::Gpu;
use yantra::{Halt, Machine, Privilege};

const RAM_BASE: u64 = 0x8000_0000;
const RAM: usize = 2 << 20;
const SLOT0: u32 = 0x1000_1000;
const PFN: u32 = 0x80008; // the queue at 0x8000_8000, as in w351_queue.rs
const DESC: u64 = 0x8000_8000;
const AVAIL: u64 = 0x8000_8080;
const USED: u64 = 0x8000_9000;
const REQ: u64 = 0x8000_a000;
const RESP: u64 = 0x8000_b000;
const NOTIFY_AT: u64 = 0x8000_0800;
/// the GPU-driver project's oracle's backing address and frame (virtio_gpu.py: BACKING, W = H = 160).
const BACKING: u64 = 0x8010_0000;
const W: u32 = 160;

// 5.7.6.7's response codes, written out.
const OK_NODATA: u32 = 0x1100;
const OK_DISPLAY_INFO: u32 = 0x1101;
const ERR_UNSPEC: u32 = 0x1200;
const ERR_OUT_OF_MEMORY: u32 = 0x1201;
const ERR_INVALID_SCANOUT_ID: u32 = 0x1202;
const ERR_INVALID_RESOURCE_ID: u32 = 0x1203;
const ERR_INVALID_PARAMETER: u32 = 0x1205;

// ---- the wire, packed by hand ---------------------------------------------------------

fn le32(v: u32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}
/// virtio_gpu_ctrl_hdr: le32 type, le32 flags, le64 fence_id, le32 ctx_id, u8 ring_idx,
/// u8 padding[3] — 24 octets.
fn hdr_fenced(t: u32, flags: u32, fence: u64) -> Vec<u8> {
    let mut b = le32(t);
    b.extend(le32(flags));
    b.extend(fence.to_le_bytes());
    b.extend([0u8; 8]);
    b
}
fn hdr(t: u32) -> Vec<u8> {
    hdr_fenced(t, 0, 0)
}
fn rect(x: u32, y: u32, w: u32, h: u32) -> Vec<u8> {
    [x, y, w, h].iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn display_info() -> Vec<u8> {
    hdr(0x0100)
}
fn create(id: u32, format: u32, w: u32, h: u32) -> Vec<u8> {
    [hdr(0x0101), le32(id), le32(format), le32(w), le32(h)].concat()
}
fn attach(id: u32, addr: u64, len: u32) -> Vec<u8> {
    [
        hdr(0x0106),
        le32(id),
        le32(1),
        addr.to_le_bytes().to_vec(),
        le32(len),
        le32(0),
    ]
    .concat()
}
fn scanout(r: Vec<u8>, scanout_id: u32, id: u32) -> Vec<u8> {
    [hdr(0x0103), r, le32(scanout_id), le32(id)].concat()
}
fn transfer(r: Vec<u8>, offset: u64, id: u32) -> Vec<u8> {
    [
        hdr(0x0105),
        r,
        offset.to_le_bytes().to_vec(),
        le32(id),
        le32(0),
    ]
    .concat()
}
fn flush(r: Vec<u8>, id: u32) -> Vec<u8> {
    [hdr(0x0104), r, le32(id), le32(0)].concat()
}

/// virtio_gpu.py's checksum, transcribed.
fn checksum(octets: &[u8]) -> u64 {
    octets
        .iter()
        .fold(0u64, |h, &b| (h * 1_000_003 + u64::from(b)) % 2_147_483_647)
}

/// The oracle's frame: its six commands, in its order.
fn oracle_frame() -> Vec<Vec<u8>> {
    vec![
        display_info(),
        create(1, 2, W, W),
        attach(1, BACKING, W * W * 4),
        scanout(rect(0, 0, W, W), 0, 1),
        transfer(rect(0, 0, W, W), 0, 1),
        flush(rect(0, 0, W, W), 1),
    ]
}

// ---- the machine, driven through the transport ----------------------------------------

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
fn li(rd: u32, v: u32) -> [u32; 2] {
    let hi = v.wrapping_add(0x800) & 0xffff_f000;
    [lui(rd, hi), addi(rd, rd, v.wrapping_sub(hi) as i32)]
}

fn at(addr: u64) -> usize {
    (addr - RAM_BASE) as usize
}
fn put(m: &mut Machine, addr: u64, bytes: &[u8]) {
    m.mem[at(addr)..at(addr) + bytes.len()].copy_from_slice(bytes);
}
fn rd32(m: &Machine, addr: u64) -> u32 {
    u32::from_le_bytes(m.mem[at(addr)..at(addr) + 4].try_into().unwrap())
}

fn run_at(m: &mut Machine, pc: u64, n: usize) -> Option<Halt> {
    m.pc = pc;
    let mut out = Vec::new();
    (0..n).find_map(|_| m.step(&mut out))
}

/// A machine with queue 0 set up by §4.2.4's legacy sequence, run as real stores.
fn machine() -> Machine {
    let mut m = Machine {
        store_limit: usize::MAX,
        patra_root: None,
        patra_mem: None,
        patra_path: None,
        patra_buffer: None,
        virtio: Default::default(),
        x: [0; 32],
        f: [0; 32],
        fcsr: 0,
        pc: RAM_BASE,
        base: RAM_BASE,
        mem: vec![0; RAM],
        reservation: None,
        csr: yantra::Csrs::default(),
        mode: Privilege::Supervisor,
        time: 0,
        timecmp: None,
        vec: Default::default(),
        socket: None,
    };
    let mut t = vec![lui(1, SLOT0)];
    for (off, v) in [
        (0x070, 3),
        (0x028, 4096),
        (0x030, 0),
        (0x038, 8),
        (0x03c, 4096),
        (0x040, PFN),
    ] {
        t.extend(li(2, v));
        t.push(sw(1, 2, off));
    }
    let n = t.len();
    for (k, w) in t.iter().enumerate() {
        m.mem[k * 4..k * 4 + 4].copy_from_slice(&w.to_le_bytes());
    }
    // The notify routine: QueueNotify <- 0.
    for (k, w) in [lui(1, SLOT0), sw(1, 0, 0x050)].iter().enumerate() {
        let a = at(NOTIFY_AT) + k * 4;
        m.mem[a..a + 4].copy_from_slice(&w.to_le_bytes());
    }
    assert_eq!(
        run_at(&mut m, RAM_BASE, n),
        None,
        "the legacy set-up must not halt"
    );
    m
}

/// Offer `request` as one chain (readable request, writable 408-octet reply slot filled
/// with 0xff), ring QueueNotify through a real store, and return the reply's type.
fn submit(m: &mut Machine, request: &[u8]) -> u32 {
    let k = u16::from_le_bytes([m.mem[at(AVAIL + 2)], m.mem[at(AVAIL + 3)]]);
    let desc = |addr: u64, len: u32, flags: u16, next: u16| {
        [
            addr.to_le_bytes().to_vec(),
            le32(len),
            flags.to_le_bytes().to_vec(),
            next.to_le_bytes().to_vec(),
        ]
        .concat()
    };
    put(m, REQ, request);
    put(m, RESP, &[0xff; 408]);
    put(m, DESC, &desc(REQ, request.len() as u32, 1, 1));
    put(m, DESC + 16, &desc(RESP, 408, 2, 0));
    put(m, AVAIL + 4 + 2 * u64::from(k % 8), &0u16.to_le_bytes());
    put(m, AVAIL + 2, &k.wrapping_add(1).to_le_bytes());
    assert_eq!(run_at(m, NOTIFY_AT, 2), None, "QueueNotify must not halt");
    assert_eq!(
        u16::from_le_bytes([m.mem[at(USED + 2)], m.mem[at(USED + 3)]]),
        k.wrapping_add(1),
        "each submission returns one used entry"
    );
    rd32(m, RESP)
}

// ---- (a) the oracle's frame ------------------------------------------------------------

#[test]
fn the_oracles_six_commands_are_served_and_the_resource_holds_the_backing() {
    let frame = oracle_frame();
    let lens: Vec<usize> = frame.iter().map(Vec::len).collect();
    assert_eq!(lens, [24, 40, 48, 48, 56, 48], "virtio_gpu.py's lengths");
    assert_eq!(
        checksum(&frame.concat()),
        484_652_832,
        "the oracle prints 484652832 for these 264 octets"
    );

    let mut m = machine();
    // A backing every pixel of which differs from its neighbours, so a transfer that
    // drops, repeats or shifts a row cannot match.
    for i in 0..(W * W) as u64 {
        let px = (i.wrapping_mul(2_654_435_761) as u32) & 0x00ff_ffff;
        put(&mut m, BACKING + 4 * i, &px.to_le_bytes());
    }
    let replies: Vec<u32> = frame.iter().map(|c| submit(&mut m, c)).collect();
    assert_eq!(
        replies,
        [
            OK_DISPLAY_INFO,
            OK_NODATA,
            OK_NODATA,
            OK_NODATA,
            OK_NODATA,
            OK_NODATA
        ],
        "GET_DISPLAY_INFO, then five OK_NODATA"
    );
    let backing = &m.mem[at(BACKING)..at(BACKING) + (W * W * 4) as usize];
    let res = &m.virtio.gpu.resources[&1];
    assert!(
        res.pixels == backing,
        "after a whole-frame transfer the resource equals the backing"
    );
    assert_eq!(
        m.virtio.gpu.scanout.map(|(id, _)| id),
        Some(1),
        "scanout 0 shows resource 1"
    );
}

#[test]
fn display_info_reports_one_enabled_head() {
    let mut m = machine();
    assert_eq!(submit(&mut m, &display_info()), OK_DISPLAY_INFO);
    // virtio_gpu_display_one: rect (16), le32 enabled, le32 flags — after the 24-octet hdr.
    let one = |k: u64, f: u64| rd32(&m, RESP + 24 + 24 * k + f);
    assert_eq!(
        (one(0, 8), one(0, 12), one(0, 16)),
        (640, 480, 1),
        "head 0: 640 x 480, enabled"
    );
    assert!((1..16).all(|k| one(k, 16) == 0), "heads 1-15 disabled");
}

// ---- (b) the picture, twice ------------------------------------------------------------

#[test]
fn a_flush_writes_the_scanout_as_a_ppm_and_a_second_flush_overwrites_it() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "w378-ppm-{}-{}-{nanos}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("screen.ppm");
    let mut m = machine();
    m.virtio.gpu.dump = Some(path.clone());
    let back = 0x8002_0000;
    // 2 x 2, B8G8R8X8: octets B, G, R, X per pixel.
    put(
        &mut m,
        back,
        &[
            0x01, 0x02, 0x03, 0, 0x10, 0x20, 0x30, 0, 0xaa, 0xbb, 0xcc, 0, 0xff, 0x00, 0x7f, 0,
        ],
    );
    for c in [
        create(5, 2, 2, 2),
        attach(5, back, 16),
        scanout(rect(0, 0, 2, 2), 0, 5),
        transfer(rect(0, 0, 2, 2), 0, 5),
        flush(rect(0, 0, 2, 2), 5),
    ] {
        assert_eq!(submit(&mut m, &c), OK_NODATA);
    }
    let first: Vec<u8> = [
        b"P6\n2 2\n255\n".as_slice(),
        &[
            0x03, 0x02, 0x01, 0x30, 0x20, 0x10, 0xcc, 0xbb, 0xaa, 0x7f, 0x00, 0xff,
        ],
    ]
    .concat();
    assert_eq!(
        std::fs::read(&path).unwrap(),
        first,
        "the PPM is R, G, B per pixel, rows top to bottom"
    );

    put(&mut m, back, &[9, 8, 7, 0]);
    assert_eq!(submit(&mut m, &transfer(rect(0, 0, 2, 2), 0, 5)), OK_NODATA);
    assert_eq!(submit(&mut m, &flush(rect(0, 0, 2, 2), 5)), OK_NODATA);
    let mut second = first.clone();
    second[11..14].copy_from_slice(&[7, 8, 9]);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        second,
        "the file holds the SECOND frame: one path, overwritten"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_flush_of_a_resource_not_on_the_scanout_writes_nothing() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "w378-ppm-{}-{}-{nanos}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("screen.ppm");
    let mut m = machine();
    m.virtio.gpu.dump = Some(path.clone());
    for c in [create(5, 2, 2, 2), flush(rect(0, 0, 2, 2), 5)] {
        assert_eq!(submit(&mut m, &c), OK_NODATA);
    }
    assert!(
        !path.exists(),
        "no scanout shows resource 5, so nothing reaches the screen"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

// ---- (c) and (d): every refusal, its code, and no effect -------------------------------

/// Serve `reqs` in order on a fresh device over `mem` and return each reply's type.
fn codes(gpu: &mut Gpu, mem: &[u8], reqs: &[Vec<u8>]) -> Vec<u32> {
    reqs.iter()
        .map(|r| u32::from_le_bytes(gpu.serve(mem, RAM_BASE, r)[..4].try_into().unwrap()))
        .collect()
}

#[test]
fn each_refusal_gets_its_code() {
    let mem = vec![0u8; 1 << 16];
    let base = RAM_BASE;
    let cases: Vec<(&str, Vec<Vec<u8>>, u32)> = vec![
        (
            "attach to an unknown resource",
            vec![attach(9, base, 16)],
            ERR_INVALID_RESOURCE_ID,
        ),
        (
            "create a live id again",
            vec![create(1, 2, 2, 2), create(1, 2, 4, 4)],
            ERR_INVALID_RESOURCE_ID,
        ),
        (
            "a format the device does not serve",
            vec![create(1, 3, 2, 2)],
            ERR_INVALID_PARAMETER,
        ),
        (
            "a resource over the cap",
            vec![create(1, 2, 4096, 4097)],
            ERR_OUT_OF_MEMORY,
        ),
        (
            "width x height x 4 past 2^64",
            vec![create(1, 2, u32::MAX, u32::MAX)],
            ERR_OUT_OF_MEMORY,
        ),
        // 2^31 · 2^31 · 4 = 2^64 EXACTLY, which a wrapping product reads as 0 — under any
        // cap. u32::MAX² · 4 wraps to 2^64 − 2^35 + 4, still over the cap, so only this
        // case can tell a checked product from a wrapping one.
        (
            "width x height x 4 wrapping to 0",
            vec![create(1, 2, 1 << 31, 1 << 31)],
            ERR_OUT_OF_MEMORY,
        ),
        (
            "a backing span outside RAM",
            vec![create(1, 2, 2, 2), attach(1, base + (1 << 16) - 8, 16)],
            ERR_INVALID_PARAMETER,
        ),
        (
            "scanout 1 on a one-head device",
            vec![create(1, 2, 2, 2), scanout(rect(0, 0, 2, 2), 1, 1)],
            ERR_INVALID_SCANOUT_ID,
        ),
        (
            "a scanout the resource does not cover",
            vec![create(1, 2, 2, 2), scanout(rect(1, 0, 2, 2), 0, 1)],
            ERR_INVALID_PARAMETER,
        ),
        (
            "a transfer with no backing",
            vec![create(1, 2, 2, 2), transfer(rect(0, 0, 2, 2), 0, 1)],
            ERR_UNSPEC,
        ),
        (
            "a transfer outside the resource",
            vec![
                create(1, 2, 2, 2),
                attach(1, base, 16),
                transfer(rect(0, 1, 2, 2), 0, 1),
            ],
            ERR_INVALID_PARAMETER,
        ),
        (
            "a transfer past the backing",
            vec![
                create(1, 2, 2, 2),
                attach(1, base, 8),
                transfer(rect(0, 0, 2, 2), 0, 1),
            ],
            ERR_INVALID_PARAMETER,
        ),
        (
            "a transfer offset past the backing",
            vec![
                create(1, 2, 2, 2),
                attach(1, base, 16),
                transfer(rect(0, 0, 2, 1), 9, 1),
            ],
            ERR_INVALID_PARAMETER,
        ),
        (
            "a flush outside the resource",
            vec![create(1, 2, 2, 2), flush(rect(0, 0, 3, 2), 1)],
            ERR_INVALID_PARAMETER,
        ),
        (
            "a flush of an unknown resource",
            vec![flush(rect(0, 0, 1, 1), 9)],
            ERR_INVALID_RESOURCE_ID,
        ),
        ("a request no parser reads", vec![hdr(0x0999)], ERR_UNSPEC),
    ];
    for (what, reqs, want) in cases {
        let mut gpu = Gpu::default();
        let got = codes(&mut gpu, &mem, &reqs);
        assert_eq!(*got.last().unwrap(), want, "{what}: replies {got:x?}");
        assert!(
            got[..got.len() - 1].iter().all(|&c| c == OK_NODATA),
            "{what}: the set-up must succeed, got {got:x?}"
        );
    }
}

#[test]
fn a_refused_command_changes_nothing() {
    let mut mem = vec![0u8; 1 << 16];
    mem[..16].copy_from_slice(&[1; 16]);
    let mut gpu = Gpu::default();
    let ok = codes(
        &mut gpu,
        &mem,
        &[create(1, 2, 2, 2), attach(1, RAM_BASE, 16)],
    );
    assert_eq!(ok, [OK_NODATA, OK_NODATA]);
    let before = gpu.clone();
    // A duplicate create, an out-of-resource transfer and a transfer past the backing.
    let bad = codes(
        &mut gpu,
        &mem,
        &[
            create(1, 2, 4, 4),
            transfer(rect(1, 1, 2, 2), 0, 1),
            transfer(rect(0, 0, 2, 2), 4, 1),
        ],
    );
    assert_eq!(
        bad,
        [
            ERR_INVALID_RESOURCE_ID,
            ERR_INVALID_PARAMETER,
            ERR_INVALID_PARAMETER
        ]
    );
    assert_eq!(
        gpu, before,
        "three refusals and not one resource, size or pixel changed"
    );
}

#[test]
fn a_fenced_request_gets_its_fence_back() {
    // 5.7.6.7: VIRTIO_GPU_FLAG_FENCE = 1; the device echoes fence_id in the reply.
    let mut gpu = Gpu::default();
    let reply = gpu.serve(&[0u8; 64], RAM_BASE, &hdr_fenced(0x0100, 1, 77));
    assert_eq!(
        u32::from_le_bytes(reply[0..4].try_into().unwrap()),
        OK_DISPLAY_INFO
    );
    assert_eq!(
        u32::from_le_bytes(reply[4..8].try_into().unwrap()) & 1,
        1,
        "FENCE echoed"
    );
    assert_eq!(
        u64::from_le_bytes(reply[8..16].try_into().unwrap()),
        77,
        "fence_id echoed"
    );
}

#[test]
fn a_reset_clears_the_device_but_keeps_where_the_screen_goes() {
    let mut m = machine();
    m.virtio.gpu.dump = Some("/nonexistent/screen.ppm".into());
    assert_eq!(submit(&mut m, &create(1, 2, 2, 2)), OK_NODATA);
    // Status <- 0, a real store.
    put(
        &mut m,
        NOTIFY_AT + 0x40,
        &[lui(1, SLOT0), sw(1, 0, 0x070)]
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect::<Vec<_>>(),
    );
    assert_eq!(run_at(&mut m, NOTIFY_AT + 0x40, 2), None);
    assert!(
        m.virtio.gpu.resources.is_empty(),
        "a device reset destroys its resources"
    );
    assert!(
        m.virtio.gpu.dump.is_some(),
        "the host's screen path is not device state"
    );
}

// ---- a peer session's review: nothing a guest sends may make the HOST allocate unboundedly --

/// RESOURCE_ATTACH_BACKING with several spans: (addr, len) each.
fn attach_n(id: u32, spans: &[(u64, u32)]) -> Vec<u8> {
    let mut b = [hdr(0x0106), le32(id), le32(spans.len() as u32)].concat();
    for &(a, l) in spans {
        b.extend(a.to_le_bytes());
        b.extend(le32(l));
        b.extend(le32(0));
    }
    b
}

#[test]
fn spans_whose_sum_exceeds_ram_are_refused_at_attach() {
    // Each span lies in RAM; three of them each spanning ALL of it do not, together.
    // Before the fix they were accepted, and a TRANSFER concatenated them on the host.
    let ram = 1u32 << 16;
    let mem = vec![0u8; ram as usize];
    let mut gpu = Gpu::default();
    let got = codes(
        &mut gpu,
        &mem,
        &[create(1, 2, 2, 2), attach_n(1, &[(RAM_BASE, ram); 3])],
    );
    assert_eq!(
        got,
        [OK_NODATA, ERR_INVALID_PARAMETER],
        "the spans' sum must lie in RAM"
    );
    assert!(
        gpu.resources[&1].backing.is_empty(),
        "a refused attach leaves no backing"
    );
}

#[test]
fn the_device_holds_at_most_its_total_and_the_crossing_create_leaves_nothing() {
    // MAX_RESOURCE bounds one resource; MAX_TOTAL bounds them all. Four 4096 x 4096
    // resources are 256 MiB exactly; a fifth would cross it.
    let mem = vec![0u8; 64];
    let mut gpu = Gpu::default();
    let reqs: Vec<Vec<u8>> = (1..=5).map(|id| create(id, 2, 4096, 4096)).collect();
    let got = codes(&mut gpu, &mem, &reqs);
    assert_eq!(
        got,
        [
            OK_NODATA,
            OK_NODATA,
            OK_NODATA,
            OK_NODATA,
            ERR_OUT_OF_MEMORY
        ]
    );
    assert!(
        !gpu.resources.contains_key(&5),
        "the refused create leaves no resource"
    );
    assert_eq!(gpu.resources.len(), 4);
}

#[test]
fn a_backing_in_two_spans_out_of_memory_order_is_read_as_one() {
    // The backing is its spans END TO END in attach order, wherever they lie: span 0 at
    // +32 holds the first two pixels, span 1 at +0 the last two. Read in place, row by
    // row, the resource must equal the spans concatenated.
    let mut mem = vec![0u8; 1 << 12];
    mem[32..40].copy_from_slice(&[1, 2, 3, 0, 4, 5, 6, 0]);
    mem[0..8].copy_from_slice(&[7, 8, 9, 0, 10, 11, 12, 0]);
    let mut gpu = Gpu::default();
    let got = codes(
        &mut gpu,
        &mem,
        &[
            create(1, 2, 2, 2),
            attach_n(1, &[(RAM_BASE + 32, 8), (RAM_BASE, 8)]),
            transfer(rect(0, 0, 2, 2), 0, 1),
        ],
    );
    assert_eq!(got, [OK_NODATA, OK_NODATA, OK_NODATA]);
    assert_eq!(
        gpu.resources[&1].pixels,
        [1, 2, 3, 0, 4, 5, 6, 0, 7, 8, 9, 0, 10, 11, 12, 0]
    );
    // A sub-rectangle at an offset that starts inside span 0 and ends inside span 1.
    let got = codes(&mut gpu, &mem, &[transfer(rect(1, 1, 1, 1), 4, 1)]);
    assert_eq!(got, [OK_NODATA]);
    assert_eq!(
        &gpu.resources[&1].pixels[12..16],
        &[4, 5, 6, 0],
        "row 1 col 1 <- backing octets 4..8"
    );
}

// ---- the DEFERRED completion (`YANTRA_VIRTIO_DEFER`, `VirtioMmio::defer`) ----------------
//
// QEMU's virtio-gpu completes a chain after the QueueNotify store returns (an ioeventfd and a
// bottom half); this device, by default, inside it. A driver that reads the used index ONCE
// after notifying passes here and fails on QEMU — C-015's spec/darshaka.t1 did (G 11 on QEMU
// 10.1) until it polled. With `defer` = n the chains are served n instructions after the
// notify, before the n-th instruction begins, so the next n − 1 instructions see nothing.

/// Where the machine steps while a completion is pending: 2048 `addi x0, x0, 0`, clear of
/// the set-up code, the notify and reset routines (NOTIFY_AT..+0x48) and the queue.
const NOPS_AT: u64 = 0x8000_1000;
const NOPS: u64 = 2048;
const NOP: u32 = 0x0000_0013;

fn deferred(defer: u64) -> Machine {
    let mut m = machine();
    for i in 0..NOPS {
        put(&mut m, NOPS_AT + 4 * i, &NOP.to_le_bytes());
    }
    m.virtio.defer = defer;
    m
}

fn used_idx(m: &Machine) -> u16 {
    u16::from_le_bytes([m.mem[at(USED + 2)], m.mem[at(USED + 3)]])
}

/// `submit`'s chain, offered and notified through the real store, WITHOUT waiting for it;
/// the machine is left at the no-op field. Answers the used index before the notify.
fn notify_only(m: &mut Machine, request: &[u8]) -> u16 {
    let k = u16::from_le_bytes([m.mem[at(AVAIL + 2)], m.mem[at(AVAIL + 3)]]);
    let desc = |addr: u64, len: u32, flags: u16, next: u16| {
        [
            addr.to_le_bytes().to_vec(),
            le32(len),
            flags.to_le_bytes().to_vec(),
            next.to_le_bytes().to_vec(),
        ]
        .concat()
    };
    put(m, REQ, request);
    put(m, RESP, &[0xff; 408]);
    put(m, DESC, &desc(REQ, request.len() as u32, 1, 1));
    put(m, DESC + 16, &desc(RESP, 408, 2, 0));
    put(m, AVAIL + 4 + 2 * u64::from(k % 8), &0u16.to_le_bytes());
    put(m, AVAIL + 2, &k.wrapping_add(1).to_le_bytes());
    let before = used_idx(m);
    assert_eq!(run_at(m, NOTIFY_AT, 2), None, "QueueNotify must not halt");
    m.pc = NOPS_AT;
    before
}

/// `n` instructions from wherever the machine is, without moving `pc`.
fn steps(m: &mut Machine, n: u64) -> Option<Halt> {
    let mut out = Vec::new();
    (0..n).find_map(|_| m.step(&mut out))
}

#[test]
fn a_deferred_completion_lands_before_the_nth_instruction_after_the_notify() {
    const N: u64 = 5;
    let mut m = deferred(N);
    let k = notify_only(&mut m, &display_info());
    // THE SINGLE READ: right after the notify store, the chain is not used — a driver that
    // reads the used index once now sees its own last value, as it would on QEMU.
    assert_eq!(used_idx(&m), k, "nothing is used inside the notify store");
    assert_eq!(rd32(&m, RESP), 0xffff_ffff, "and nothing is written");
    assert_eq!(m.virtio.interrupt_status, 0, "nor is the interrupt raised");
    assert_eq!(m.virtio.waiting, 1);
    assert_eq!(steps(&mut m, N - 1), None);
    assert_eq!(used_idx(&m), k, "n - 1 instructions later, still nothing");
    assert_eq!(rd32(&m, RESP), 0xffff_ffff);
    assert_eq!(steps(&mut m, 1), None);
    assert_eq!(
        used_idx(&m),
        k.wrapping_add(1),
        "the n-th instruction sees it used"
    );
    assert_eq!(rd32(&m, RESP), OK_DISPLAY_INFO);
    assert_eq!(
        m.virtio.interrupt_status & 1,
        1,
        "and the used-buffer interrupt"
    );
    assert_eq!(m.virtio.waiting, 0);
    assert_eq!(m.virtio.pending, [None, None]);
}

#[test]
fn zero_is_the_synchronous_device() {
    let mut m = deferred(0);
    let k = notify_only(&mut m, &display_info());
    assert_eq!(
        used_idx(&m),
        k.wrapping_add(1),
        "used inside the notify store"
    );
    assert_eq!(rd32(&m, RESP), OK_DISPLAY_INFO);
    assert_eq!(m.virtio.waiting, 0);
}

/// A POLLING driver passes under any delay: the oracle's six commands, each notified and
/// then polled, answer and render exactly as on the synchronous device.
#[test]
fn a_polling_driver_gets_the_same_answers_and_resource_under_deferral() {
    for defer in [1, 7, 1000] {
        let mut m = deferred(defer);
        for i in 0..(W * W) as u64 {
            let px = (i.wrapping_mul(2_654_435_761) as u32) & 0x00ff_ffff;
            put(&mut m, BACKING + 4 * i, &px.to_le_bytes());
        }
        let mut replies = Vec::new();
        for c in oracle_frame() {
            let k = notify_only(&mut m, &c);
            let mut polls = 0;
            while used_idx(&m) == k {
                assert!(
                    polls < NOPS,
                    "defer {defer}: no completion within {NOPS} instructions"
                );
                assert_eq!(steps(&mut m, 1), None);
                polls += 1;
            }
            assert_eq!(
                polls, defer,
                "defer {defer}: used after exactly that many instructions"
            );
            replies.push(rd32(&m, RESP));
        }
        assert_eq!(
            replies,
            [
                OK_DISPLAY_INFO,
                OK_NODATA,
                OK_NODATA,
                OK_NODATA,
                OK_NODATA,
                OK_NODATA
            ],
            "defer {defer}: the synchronous device's six answers"
        );
        let backing = &m.mem[at(BACKING)..at(BACKING) + (W * W * 4) as usize];
        assert!(
            m.virtio.gpu.resources[&1].pixels == backing,
            "defer {defer}: the resource holds the backing"
        );
    }
}

/// A chain refused when the deferred completion lands halts THEN, naming the notify store
/// it answers — not the no-op that happens to be next.
#[test]
fn a_refusal_found_late_halts_when_it_lands_and_names_the_notify() {
    const N: u64 = 3;
    let mut m = deferred(N);
    let k = u16::from_le_bytes([m.mem[at(AVAIL + 2)], m.mem[at(AVAIL + 3)]]);
    // a head outside the 8-entry descriptor table
    put(
        &mut m,
        AVAIL + 4 + 2 * u64::from(k % 8),
        &9u16.to_le_bytes(),
    );
    put(&mut m, AVAIL + 2, &k.wrapping_add(1).to_le_bytes());
    assert_eq!(
        run_at(&mut m, NOTIFY_AT, 2),
        None,
        "the store itself is accepted"
    );
    m.pc = NOPS_AT;
    assert_eq!(steps(&mut m, N - 1), None);
    match steps(&mut m, 1) {
        Some(Halt::Device { pc, addr, .. }) => {
            assert_eq!(pc, NOTIFY_AT + 4, "the notifying sw");
            assert_eq!(addr, u64::from(SLOT0) + 0x050, "QueueNotify");
        }
        other => panic!("want a Device halt when the completion lands, got {other:?}"),
    }
}

/// A reset drops a pending completion (it is device state) and keeps the delay (the host's).
#[test]
fn a_reset_drops_a_pending_completion_and_keeps_the_delay() {
    let mut m = deferred(10);
    let k = notify_only(&mut m, &display_info());
    put(
        &mut m,
        NOTIFY_AT + 0x40,
        &[lui(1, SLOT0), sw(1, 0, 0x070)]
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect::<Vec<_>>(),
    );
    assert_eq!(run_at(&mut m, NOTIFY_AT + 0x40, 2), None, "Status <- 0");
    assert_eq!((m.virtio.waiting, m.virtio.pending), (0, [None, None]));
    assert_eq!(m.virtio.defer, 10, "the delay is the host's setting");
    m.pc = NOPS_AT;
    assert_eq!(steps(&mut m, 20), None);
    assert_eq!(used_idx(&m), k, "the dropped completion never lands");
}

/// A second notify of a queue already waiting does not restart its clock: when the first is
/// served, every chain offered by then is.
#[test]
fn a_second_notify_while_waiting_keeps_the_first_deadline() {
    let mut m = deferred(6);
    let k = notify_only(&mut m, &display_info());
    assert_eq!(steps(&mut m, 2), None);
    let _ = notify_only(&mut m, &create(1, 2, 2, 2)); // 2 + 2 instructions so far
    assert_eq!(m.virtio.waiting, 1, "still one queue waiting");
    assert_eq!(steps(&mut m, 1), None); // 5
    assert_eq!(used_idx(&m), k);
    assert_eq!(steps(&mut m, 1), None); // 6: the first deadline
    assert_eq!(
        used_idx(&m),
        k.wrapping_add(2),
        "both chains, at the first notify's deadline"
    );
    // (Both offers reuse descriptors 0 and 1, so both chains carry the second request; only
    // the used index is asserted here, which is what the deadline is about.)
}
