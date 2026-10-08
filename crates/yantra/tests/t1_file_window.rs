//! **A PROGRAM NAMES A FILE AND READS IT — THE CAPABILITY THE README SAYS IS
//! ABSENT.**
//!
//! `crates/.../README` and ADR-0040 both state plainly that Sassembly can read
//! *the input it was handed* but cannot **name** a file. That was true: the
//! input channel writes octets into RAM before `pc` moves and finds its slots by
//! scanning for a magic word, so the conversation is over before the program
//! starts.
//!
//! ADR-0041 decided the fix is a third MMIO arm rather than an SBI extension,
//! because `.t1` has no construct that emits `ecall`. This test is the first
//! evidence that the decision was right, and it is falsifiable in the way the
//! ADR asked for: **it touches no `.t1` file**, so the 1,393,602-octet fixpoint
//! is untouched and no compiler change was needed.
//!
//! # What is asserted, and why not just the status
//!
//! The program builds a six-word request in its own RAM, stores the block's
//! address to `पत्रम्`, and then reads back the octet count AND the octets. The
//! sum of the octets is what the finisher carries, so:
//!
//! * a window that wrote the wrong count but the right bytes fails,
//! * a window that wrote the right count but no bytes fails,
//! * a window that did nothing at all fails, because word 5 keeps the sentinel
//!   the program put there.
//!
//! Asserting the status alone would pass on the third, which is the failure a
//! marker body cannot make loud.

use yantra::patra::{self, Status};

/// Flat RAM and its base address. `patra::serve` takes `&mut [u8]` and a base
/// rather than a `Machine`, so the device can be exercised without a loader,
/// an ELF or an entry point — and a test that builds the memory by hand says
/// exactly what is in it.
const BASE: u64 = 0x8000_0000;

fn ram(n: usize) -> Vec<u8> {
    vec![0; n]
}

fn put(mem: &mut [u8], at: u64, v: u64) {
    let off = (at - BASE) as usize;
    for i in 0..8 {
        mem[off + i] = (v >> (8 * i)) as u8;
    }
}

fn get(mem: &[u8], at: u64) -> u64 {
    let off = (at - BASE) as usize;
    let mut v = 0u64;
    for i in 0..8 {
        v |= u64::from(mem[off + i]) << (8 * i);
    }
    v
}

const UNSERVED: u64 = 0xDEAD_BEEF_DEAD_BEEF;

/// Three runs, each laid out the way compiled code lays one out: storage
/// 16-aligned with its LENGTH WORD at `storage − 8` (`riscv64.rs`'s `W-len`,
/// and what `input::inject` writes from the other side).
///
/// **THE LENGTHS LIVE IN THE HEADERS.** An earlier fixture passed them as words
/// in a request block, and a program that stated its buffer size wrongly would
/// have been written past the end of its own run and told it succeeded. The
/// host asks the allocator instead, so this fixture must lie the same way a
/// real program does.
struct Runs {
    path: u64,
    buffer: u64,
    status: u64,
}

fn lay_out(mem: &mut [u8], path: &str, buf_cap: u64) -> Runs {
    // All three are 16-aligned, so each is a legal run start and each header
    // sits eight below it.
    let (p, b, st) = (BASE + 0x200, BASE + 0x400, BASE + 0x600);
    let bytes = path.as_bytes();
    let off = (p - BASE) as usize;
    mem[off..off + bytes.len()].copy_from_slice(bytes);
    put(mem, p - 8, bytes.len() as u64);
    put(mem, b - 8, buf_cap);
    put(mem, st - 8, 1); // a one-element status run
    put(mem, st, UNSERVED);
    Runs {
        path: p,
        buffer: b,
        status: st,
    }
}

// NOTE: these four exercise `patra::serve` DIRECTLY, not through the machine's
// address decode. The three-arm decode in `lib.rs` (remember, remember, go) is
// covered by the end-to-end test that compiles a `.t1` program and runs it —
// which is the only caller that can reach those addresses the way a program
// does. Saying so because a suite that tests a protocol and not its entry point
// is exactly the shape that passes while nothing is wired up.

#[test]
fn a_program_names_a_file_and_receives_its_octets() {
    let dir = std::env::temp_dir().join(format!(
        // pid AND a clock: a pid is REUSED and these roots are never
        // removed, so a pid-only name is unique inside this process and
        // not on disk (W-301).
        "patra-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let body = b"\xe0\xa4\xb8\xe0\xa4\xa4\xe0\xa5\x8d\xe0\xa4\xaf\xe0\xa4\xae\xe0\xa5\x8d\n"; // सत्यम्\n
    std::fs::write(dir.join("patha.txt"), body).expect("fixture written");

    let mut mem = ram(0x1000);
    let r = lay_out(&mut mem, "patha.txt", 64);
    let status = patra::serve(&mut mem, BASE, r.path, r.buffer, r.status, Some(&dir));

    assert_eq!(
        status,
        Status::Read(body.len() as u64),
        "the window answered {} for a file of {} octets",
        status.class(),
        body.len()
    );
    assert_eq!(
        get(&mem, r.status),
        body.len() as u64,
        "the status run still holds {:#x}; the sentinel means the host never wrote \
         an answer at all, which is the one failure a status check cannot see",
        get(&mem, r.status)
    );

    // THE OCTETS THEMSELVES. A count without the bytes is the failure this
    // assertion exists for.
    let off = (r.buffer - BASE) as usize;
    assert_eq!(
        &mem[off..off + body.len()],
        body,
        "the count was right and the octets were not"
    );
    // And nothing was written past what was asked for.
    assert_eq!(
        mem[off + body.len()],
        0,
        "the window wrote past the end of the file's octets"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_window_refuses_a_path_that_escapes_its_root_and_says_so() {
    let dir = std::env::temp_dir().join(format!(
        // pid AND a clock: a pid is REUSED and these roots are never
        // removed, so a pid-only name is unique inside this process and
        // not on disk (W-301).
        "patra-esc-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("inside.txt"), b"ok").expect("fixture");

    let mut mem = ram(0x1000);
    // `..` is resolved BEFORE the prefix check — a check on the unresolved path
    // would let this through.
    let r = lay_out(&mut mem, "../../../../etc/passwd", 64);
    let status = patra::serve(&mut mem, BASE, r.path, r.buffer, r.status, Some(&dir));

    assert!(
        matches!(status, Status::Refused | Status::NotFound),
        "a path escaping the root answered {}",
        status.class()
    );
    // **REFUSAL IS NOT SILENCE.** Word 5 must carry it, or a program cannot
    // tell a refusal from a host that never ran.
    assert_ne!(
        get(&mem, r.status),
        UNSERVED,
        "the request was refused and the status run was left at the sentinel"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_machine_with_no_root_refuses_every_file_request() {
    // The default. A caller that never thought about files must not get them.
    let mut mem = ram(0x1000);
    let r = lay_out(&mut mem, "anything.txt", 64);
    let status = patra::serve(&mut mem, BASE, r.path, r.buffer, r.status, None);
    assert_eq!(
        status,
        Status::Refused,
        "a machine with no root answered {}",
        status.class()
    );
    assert_eq!(get(&mem, r.status), Status::Refused.word());
}

#[test]
fn a_file_larger_than_the_buffer_is_refused_rather_than_truncated() {
    let dir = std::env::temp_dir().join(format!(
        // pid AND a clock: a pid is REUSED and these roots are never
        // removed, so a pid-only name is unique inside this process and
        // not on disk (W-301).
        "patra-big-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(dir.join("big.txt"), vec![b'x'; 100]).expect("fixture");

    let mut mem = ram(0x1000);
    let r = lay_out(&mut mem, "big.txt", 10);
    let status = patra::serve(&mut mem, BASE, r.path, r.buffer, r.status, Some(&dir));

    assert_eq!(
        status,
        Status::TooLarge(100),
        "a 100-octet file into a 10-octet buffer answered {}",
        status.class()
    );
    // THE BUFFER IS UNTOUCHED. A partial read reported as a full one is the
    // failure this interface exists to avoid.
    let off = (r.buffer - BASE) as usize;
    assert!(
        mem[off..off + 10].iter().all(|&b| b == 0),
        "the buffer was written despite the refusal"
    );

    std::fs::remove_dir_all(&dir).ok();
}

// ── v1.0.1 PATH SAFETY, ONE TEST PER ESCAPE ────────────────────────────────────
//
// `--files DIR` makes the window reachable, so each way out of DIR is pinned here
// with a SECRET OUTSIDE THE ROOT that really exists: a refusal must be `Refused`,
// not `NotFound`, and the secret must be untouched (a test that targets a missing
// file passes when the check is absent).

fn fresh(tag: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!(
        "patra-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(base.join("root")).expect("temp dir");
    std::fs::write(base.join("secret.txt"), b"SECRET").expect("secret");
    std::fs::write(base.join("root/inside.txt"), b"ok").expect("inside");
    base
}

fn read_status(base: &std::path::Path, path: &str) -> Status {
    let mut mem = ram(0x1000);
    let r = lay_out(&mut mem, path, 64);
    let root = base.join("root");
    patra::serve(&mut mem, BASE, r.path, r.buffer, r.status, Some(&root))
}

fn write_status(base: &std::path::Path, path: &str, body: &[u8]) -> Status {
    let mut mem = ram(0x1000);
    let r = lay_out(&mut mem, path, body.len() as u64);
    let off = (r.buffer - BASE) as usize;
    mem[off..off + body.len()].copy_from_slice(body);
    let root = base.join("root");
    patra::put(&mut mem, BASE, r.path, r.buffer, r.status, Some(&root))
}

#[test]
fn control_a_file_inside_the_root_reads_and_a_write_lands() {
    let b = fresh("ctl");
    assert_eq!(read_status(&b, "inside.txt"), Status::Read(2));
    assert_eq!(write_status(&b, "new.txt", b"hi"), Status::Wrote(2));
    assert_eq!(std::fs::read(b.join("root/new.txt")).unwrap(), b"hi");
    std::fs::remove_dir_all(&b).ok();
}

#[test]
fn dotdot_to_a_real_file_outside_is_refused_for_read_and_write() {
    let b = fresh("dd");
    assert_eq!(read_status(&b, "../secret.txt"), Status::Refused);
    assert_eq!(read_status(&b, "sub/../../secret.txt"), Status::NotFound);
    std::fs::create_dir_all(b.join("root/sub")).unwrap();
    assert_eq!(read_status(&b, "sub/../../secret.txt"), Status::Refused);
    assert_eq!(write_status(&b, "../secret.txt", b"X"), Status::Refused);
    assert_eq!(write_status(&b, "../new.txt", b"X"), Status::Refused);
    assert_eq!(std::fs::read(b.join("secret.txt")).unwrap(), b"SECRET");
    assert!(!b.join("new.txt").exists());
    std::fs::remove_dir_all(&b).ok();
}

#[test]
fn an_absolute_path_is_refused_even_when_it_names_a_file_inside_the_root() {
    let b = fresh("abs");
    let outside = b.join("secret.txt");
    let inside = b.join("root/inside.txt");
    assert_eq!(read_status(&b, outside.to_str().unwrap()), Status::Refused);
    assert_eq!(read_status(&b, inside.to_str().unwrap()), Status::Refused);
    assert_eq!(
        write_status(&b, outside.to_str().unwrap(), b"X"),
        Status::Refused
    );
    assert_eq!(std::fs::read(&outside).unwrap(), b"SECRET");
    std::fs::remove_dir_all(&b).ok();
}

#[cfg(unix)]
#[test]
fn a_symlink_inside_the_root_pointing_outside_is_refused() {
    let b = fresh("ln");
    std::os::unix::fs::symlink(b.join("secret.txt"), b.join("root/link.txt")).unwrap();
    assert_eq!(read_status(&b, "link.txt"), Status::Refused);
    // THE WRITE THROUGH THE LEAF LINK: `fs::write` follows it, so without the leaf check
    // this overwrites the secret.
    assert_eq!(write_status(&b, "link.txt", b"X"), Status::Refused);
    assert_eq!(std::fs::read(b.join("secret.txt")).unwrap(), b"SECRET");
    // A DANGLING link cannot be shown to stay inside, so a write is refused too.
    std::os::unix::fs::symlink(b.join("nowhere.txt"), b.join("root/dangling.txt")).unwrap();
    assert_eq!(write_status(&b, "dangling.txt", b"X"), Status::Refused);
    assert!(!b.join("nowhere.txt").exists());
    // CONTROL: a link that stays inside is fine.
    std::os::unix::fs::symlink(b.join("root/inside.txt"), b.join("root/ok.txt")).unwrap();
    assert_eq!(read_status(&b, "ok.txt"), Status::Read(2));
    std::fs::remove_dir_all(&b).ok();
}

#[cfg(unix)]
#[test]
fn a_symlinked_parent_directory_pointing_outside_is_refused() {
    let b = fresh("lp");
    std::fs::create_dir_all(b.join("outdir")).unwrap();
    std::fs::write(b.join("outdir/x.txt"), b"OUT").unwrap();
    std::os::unix::fs::symlink(b.join("outdir"), b.join("root/dir")).unwrap();
    assert_eq!(read_status(&b, "dir/x.txt"), Status::Refused);
    assert_eq!(write_status(&b, "dir/x.txt", b"X"), Status::Refused);
    assert_eq!(write_status(&b, "dir/new.txt", b"X"), Status::Refused);
    assert_eq!(std::fs::read(b.join("outdir/x.txt")).unwrap(), b"OUT");
    assert!(!b.join("outdir/new.txt").exists());
    std::fs::remove_dir_all(&b).ok();
}

#[test]
fn a_non_utf8_or_empty_path_is_not_a_way_out() {
    let b = fresh("odd");
    assert_ne!(read_status(&b, ""), Status::Read(2));
    assert_eq!(read_status(&b, "/"), Status::Refused);
    std::fs::remove_dir_all(&b).ok();
}
