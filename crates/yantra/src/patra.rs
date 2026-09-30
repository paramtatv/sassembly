//! **THE FILE WINDOW — `पत्रम्`, a request a store completes before it returns.**
//!
//! ADR-0041 decided that file I/O is a third MMIO arm rather than an SBI
//! extension, because `.t1` has no construct that emits `ecall` and adding one
//! means AST nodes, IR lowering, `a0`–`a7` marshalling and encoding in a
//! compiler that must survive its own fixpoint. A store costs the language
//! nothing: `स्थाननिधानरचना` already exists.
//!
//! # Why the request block is in the program's own RAM
//!
//! The obvious shape for a device is a command register, an argument region and
//! a **status register the program polls**. That shape is wrong here, and
//! `lib.rs` says why: [`Machine::load`] has **no MMIO arm at all** and takes
//! `&self`. A status register readable by the program would be the first, and a
//! device whose read mutates state cannot be served from `&self`.
//!
//! So the store does the whole operation and writes its answer into the
//! program's RAM, which the program then reads with an ORDINARY LOAD. That is
//! `input::inject`'s model — the host writing into the guest's memory — pointed
//! the other way, and it removes the polling loop rather than optimising it.
//!
//! **This is also what makes ADR-0040's question not arise.** A socket read has
//! three outcomes and the third has no representation. Every operation here
//! completes before the store returns, so there is no "not yet" to encode. That
//! is a deliberate limit, not an oversight: this window is by construction
//! unable to carry an asynchronous answer, and is therefore no use for
//! networking.
//!
//! # Three runs, three stores
//!
//! The program hands over THREE RUNS, one per store, and the third triggers the
//! read:
//!
//! ```text
//!   store the PATH run    to 0x1000_2000   the host remembers it
//!   store the BUFFER run  to 0x1000_2008   the host remembers it
//!   store the STATUS run  to 0x1000_2010   the host reads the file NOW
//! ```
//!
//! **WHY THREE STORES AND NOT ONE BLOCK.** The first design passed a single
//! six-word request run holding the other runs' pointers. A program cannot
//! build it: `.t1` has no surface form that turns a run into a number, so
//! `विनतिः अङ्कः १ अन्तः भवति मार्गः` assigns a run into an `न६४` element and is
//! refused. The block was buildable only from a test — which is to say, the
//! capability did not exist. Three arguments need no such conversion, because
//! the lowering forwards each run's pointer the way `मुद्रणम्` forwards an
//! octet.
//!
//! **AND WHY THE STATUS IS A RUN.** A program cannot load from an MMIO address
//! — [`Machine::load`] has no device arm and takes `&self`, so a readable
//! status register would be the first one and could not mutate. The host
//! therefore writes the answer into memory the program already owns, and the
//! program reads it with ordinary indexing: `स्थितिः अङ्कः ० अन्तः`. That is
//! `input::inject`'s direction reused, and it means there is no polling loop to
//! get wrong.
//!
//! Every length comes from the run's OWN header at `storage − 8`
//! (`input.rs:43`), never from a word the program fills in. A caller that
//! overstated its buffer would otherwise be written past the end of its own run
//! and told it succeeded.
//!
//! **THE WHOLE EXCHANGE FINISHES INSIDE THE THIRD STORE**, which is why
//! ADR-0040's question does not arise here: there is no "not yet" to represent,
//! and correspondingly no use for this channel in a socket.

/// Where the program stores its PATH run. `0x1000_2000` sits above the UART at
/// `0x1000_0000` and below nothing else this machine decodes; QEMU `virt` puts
/// no device here, so a program reaching it on real hardware faults rather than
/// silently doing something else.
pub const PATRA_PATH: u64 = 0x1000_2000;
/// Where the program stores its BUFFER run.
pub const PATRA_BUFFER: u64 = 0x1000_2008;
/// Where the program stores its STATUS run — and the store that DOES the read.
/// Last on purpose: the other two are only remembered, so a half-built request
/// never reaches the filesystem.
pub const PATRA_GO: u64 = 0x1000_2010;
/// The same, for a WRITE. A separate address rather than a command word: the
/// two operations differ in which direction the octets move, and a reader of
/// the emitted text can see which one a program asked for without tracing a
/// value. It also means a program cannot turn a read into a write by computing
/// a wrong number.
pub const PATRA_PUT: u64 = 0x1000_2018;

/// What the host writes into word 5.
///
/// **THE ERRORS ARE NUMBERED FROM THE TOP OF THE WORD, NOT FROM ZERO**, because
/// success writes the octet COUNT there and a zero-octet file is a real answer.
/// A scheme that used 0 for "ok" and 1.. for errors would make an empty file
/// indistinguishable from a failure — the same collision `Origin` was split to
/// end elsewhere in this project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// The file was read. Carries the octet count.
    Read(u64),
    /// The file was written. Carries the octet count, which is the buffer run's
    /// own length — a short write is not possible here, because the host writes
    /// all of it or reports a failure.
    Wrote(u64),
    /// The request block named a command this host does not know.
    UnknownCommand,
    /// The path, the buffer, or the block itself is not inside the program's
    /// RAM. Named separately from "no such file" so a pointer bug does not read
    /// as a missing file.
    BadAddress,
    /// The path is not valid UTF-8. The machine's octets are not the host's
    /// strings and the conversion can fail.
    BadPath,
    /// The path escapes the root this machine was given, or the machine was
    /// given no root. **Distinct from `NotFound` on purpose**: a program that
    /// asks for `../../.ssh/id_rsa` must be told it was refused, not told the
    /// file does not exist — the second is a lie that hides a policy.
    Refused,
    /// The host could not open or read it.
    NotFound,
    /// The host reached the file and could not WRITE it — a directory that does
    /// not exist, or no permission. **Distinct from `NotFound`**, which for a
    /// write would be a lie: the point of a write is that the file need not
    /// exist yet.
    NotWritten,
    /// The file is larger than the buffer the program offered. The program's
    /// buffer is untouched; the count is NOT written, because a partial read
    /// reported as a full one is the failure this whole interface exists to
    /// avoid.
    TooLarge(u64),
}

impl Status {
    /// The word written into the block.
    #[must_use]
    pub fn word(self) -> u64 {
        match self {
            Status::Read(n) | Status::Wrote(n) => n,
            Status::UnknownCommand => u64::MAX,
            Status::BadAddress => u64::MAX - 1,
            Status::BadPath => u64::MAX - 2,
            Status::Refused => u64::MAX - 3,
            Status::NotFound => u64::MAX - 4,
            // The wanted size is not carried: it would need a second word, and
            // a program that asked for a file too big for its buffer can ask
            // again with a bigger one. The DISTINCTION is what matters.
            Status::TooLarge(_) => u64::MAX - 5,
            Status::NotWritten => u64::MAX - 6,
        }
    }

    /// A one-word name, for a test or a diagnostic that wants to say WHICH
    /// refusal rather than that there was one.
    #[must_use]
    pub fn class(self) -> &'static str {
        match self {
            Status::Read(_) => "read",
            Status::Wrote(_) => "wrote",
            Status::UnknownCommand => "unknown-command",
            Status::BadAddress => "bad-address",
            Status::BadPath => "bad-path",
            Status::Refused => "refused",
            Status::NotFound => "not-found",
            Status::TooLarge(_) => "too-large",
            Status::NotWritten => "not-written",
        }
    }
}

/// Serve one request, given the three run pointers the program stored.
///
/// The status is written into `status_run`'s element ० on EVERY path including
/// refusal — a program that finds its sentinel untouched knows the host never
/// ran, which is a state no status value can express.
pub fn serve(
    mem: &mut [u8],
    base: u64,
    path_run: u64,
    buffer_run: u64,
    status_run: u64,
    root: Option<&std::path::Path>,
) -> Status {
    let status = serve_inner(mem, base, path_run, buffer_run, root);
    put_word(mem, base, status_run, status.word());
    status
}

fn serve_inner(
    mem: &mut [u8],
    base: u64,
    path_run: u64,
    buffer_run: u64,
    root: Option<&std::path::Path>,
) -> Status {
    let (Some(path_len), Some(buf_cap)) =
        (run_len(mem, base, path_run), run_len(mem, base, buffer_run))
    else {
        return Status::BadAddress;
    };
    // Copied out before the buffer write below takes `mem` mutably. The copy is
    // the path only — never the file.
    let Some(path_octets) = slice_ref(mem, base, path_run, path_len).map(<[u8]>::to_vec) else {
        return Status::BadAddress;
    };
    let Ok(path_str) = String::from_utf8(path_octets) else {
        return Status::BadPath;
    };

    let real = match resolve(&path_str, root, false) {
        Ok(p) => p,
        Err(st) => return st,
    };

    let Ok(bytes) = std::fs::read(&real) else {
        return Status::NotFound;
    };
    let n = bytes.len() as u64;
    if n > buf_cap {
        return Status::TooLarge(n);
    }
    if slice(mem, base, buffer_run, n).is_none() {
        return Status::BadAddress;
    }
    let at = (buffer_run - base) as usize;
    mem[at..at + bytes.len()].copy_from_slice(&bytes);
    Status::Read(n)
}

/// The length a run carries in its own header, at `storage − 8`.
///
/// `None` when that word is not inside RAM — a pointer the program computed
/// wrongly must not be read as a length.
/// Resolve a program's path under the root, or refuse.
///
/// **`yantra` IS A DEVELOPER RUNTIME, NOT A SANDBOX** (ADR-0039). But a program
/// that can name any path is a program that can read `~/.ssh`, so a root must be
/// given and the resolved path must stay under it.
///
/// `for_write` is not a permission — it decides WHAT IS CANONICALIZED. A file
/// being created does not exist, so `canonicalize` on it fails; the parent
/// directory is resolved instead and the file name appended after the prefix
/// check. **Resolving the unresolved path would let `a/../../etc/passwd`
/// through**, which is the whole reason this is not a string comparison.
fn resolve(
    path_str: &str,
    root: Option<&std::path::Path>,
    for_write: bool,
) -> Result<std::path::PathBuf, Status> {
    let root = root.ok_or(Status::Refused)?;
    let real_root = root.canonicalize().map_err(|_| Status::Refused)?;
    let joined = root.join(path_str);
    if !for_write {
        let real = joined.canonicalize().map_err(|_| Status::NotFound)?;
        return if real.starts_with(&real_root) {
            Ok(real)
        } else {
            Err(Status::Refused)
        };
    }
    // A WRITE: the file may not exist, so resolve its PARENT and keep the name.
    let parent = joined.parent().ok_or(Status::Refused)?;
    let name = joined.file_name().ok_or(Status::Refused)?;
    let real_parent = parent.canonicalize().map_err(|_| Status::NotWritten)?;
    if !real_parent.starts_with(&real_root) {
        return Err(Status::Refused);
    }
    Ok(real_parent.join(name))
}

/// Serve a WRITE: the buffer run's octets become the file's contents.
pub fn put(
    mem: &mut [u8],
    base: u64,
    path_run: u64,
    buffer_run: u64,
    status_run: u64,
    root: Option<&std::path::Path>,
) -> Status {
    let status = put_inner(mem, base, path_run, buffer_run, root);
    put_word(mem, base, status_run, status.word());
    status
}

fn put_inner(
    mem: &[u8],
    base: u64,
    path_run: u64,
    buffer_run: u64,
    root: Option<&std::path::Path>,
) -> Status {
    let (Some(path_len), Some(buf_len)) =
        (run_len(mem, base, path_run), run_len(mem, base, buffer_run))
    else {
        return Status::BadAddress;
    };
    let Some(path_octets) = slice_ref(mem, base, path_run, path_len) else {
        return Status::BadAddress;
    };
    let Ok(path_str) = std::str::from_utf8(path_octets) else {
        return Status::BadPath;
    };
    let Some(body) = slice_ref(mem, base, buffer_run, buf_len) else {
        return Status::BadAddress;
    };
    let real = match resolve(path_str, root, true) {
        Ok(p) => p,
        Err(st) => return st,
    };
    // The WHOLE buffer or nothing. `std::fs::write` truncates and writes all of
    // it, so a short write is not a state this interface can report — which is
    // deliberate: a partial write reported as success is the failure a file
    // channel must not have.
    match std::fs::write(&real, body) {
        Ok(()) => Status::Wrote(buf_len),
        Err(_) => Status::NotWritten,
    }
}

fn run_len(mem: &[u8], base: u64, run: u64) -> Option<u64> {
    let hdr = run.checked_sub(8)?;
    let s = slice_ref(mem, base, hdr, 8)?;
    let mut v = 0u64;
    for (i, b) in s.iter().enumerate() {
        v |= u64::from(*b) << (8 * i);
    }
    Some(v)
}

fn put_word(mem: &mut [u8], base: u64, at: u64, v: u64) {
    if slice(mem, base, at, 8).is_none() {
        return;
    }
    let at = (at - base) as usize;
    for i in 0..8 {
        mem[at + i] = (v >> (8 * i)) as u8;
    }
}

/// `Some` only when the whole span is inside RAM. Checked with `checked_*` and
/// not with arithmetic that could wrap: an address near `u64::MAX` plus a
/// length is exactly where a wrapping add turns an out-of-range span into an
/// in-range one.
fn slice(mem: &mut [u8], base: u64, at: u64, len: u64) -> Option<()> {
    let off = at.checked_sub(base)?;
    let off = usize::try_from(off).ok()?;
    let len = usize::try_from(len).ok()?;
    (off.checked_add(len)? <= mem.len()).then_some(())
}

fn slice_ref(mem: &[u8], base: u64, at: u64, len: u64) -> Option<&[u8]> {
    let off = usize::try_from(at.checked_sub(base)?).ok()?;
    let len = usize::try_from(len).ok()?;
    let end = off.checked_add(len)?;
    (end <= mem.len()).then(|| &mem[off..end])
}
