//! **The input channel: a file placed in RAM before the first instruction.**
//!
//! The machine has no way to read. It implements exactly two SBI calls —
//! `sbi_console_putchar` (EID 1) and `sbi_shutdown` (EID 8) — and halts on any
//! other, and that surface is deliberate. So until this module every source a
//! program compiled had to be EMBEDDED in its image, and embedding cannot reach
//! a self-hosting compiler's own corpus: a real module is thousands of lines of
//! non-repetitive text, and a fixpoint needs the compiled compiler to compile
//! exactly that. The owner ruled on 2026-09-21 that the HOST may place it.
//!
//! **THIS ADDS NO `ecall`.** The bytes are in RAM before `pc` first moves, so
//! the program sees an ordinary run global that already holds them. The
//! two-call surface is unchanged.
//!
//! # How the program is found without a symbol table
//!
//! The images this machine runs carry **no section headers**, and so no
//! `.symtab` — measured on 2026-09-21, `e_shnum = 0`. The host therefore cannot
//! look a global up by name. It looks one up by VALUE: each slot the host fills
//! is a global declared IMMEDIATELY after a tag word, and the host requires each
//! tag **exactly once**, then writes the word at `tag + 8`.
//!
//! ```text
//!   INPUT_TAG  "SASINPUT"  then  निवेशपाठः        pointer word of the source run
//!   NAME_TAG   "SASINAME"  then  निवेशमण्डलनाम    pointer word of the module-name run
//!   TRACE_TAG  "SASTRACE"  then  निवेशानुरेखणम्   the trace LEVEL: 1 encoder markers,
//!                                                    2 each module's generated text
//! ```
//!
//! **ONE TAG PER SLOT, AND THE FIRST DRAFT HAD ONE TAG FOR ALL THREE.** It read
//! the slots at +8, +16 and +24. The emitted text refuted it before any image
//! was built: every run global gets 1024 octets of storage, and after each one
//! `riscv64::emit_data` returns to `ॱदत्त` with `॥ संरेखः १६ ॥`, so a word that
//! FOLLOWS a run is re-aligned to +16 or +24 depending on where the linker put
//! the tag. A word IMMEDIATELY BEFORE the next global is adjacent to it — only
//! an export directive and a label sit between them, never a section switch —
//! and that adjacency is what each tag relies on. `t1_input_channel.rs` checks
//! it in the EMITTED text, which is where the first draft should have looked.
//!
//! # Where the bytes go — and the claim about them that was FALSE
//!
//! Into a slab APPENDED to RAM, above the old top. Each run is laid out as the
//! compiled code expects one (`riscv64.rs`, `W-len`): a length word at
//! `storage − 8`, storage 16-aligned, and the run's pointer word re-pointed at
//! it — the act native run GROWTH performs on a full run.
//!
//! **THE FIRST VERSION OF THIS PARAGRAPH SAID THE PROGRAM "COULD NEVER ADDRESS"
//! THE MEMORY ABOVE THE OLD TOP. THAT IS FALSE, MEASURED 2026-09-21.** The
//! record allocator is a bump cursor with no upper bound: it grows past the
//! image's extent, through the headroom, and on into whatever lies above. The
//! first native self-image build did exactly that — high water 559,504,480
//! over 555,254,376 octets of the old RAM, straight through this slab, then
//! `BeyondRam`. The input is only safe while RAM − (the program's peak) leaves
//! the slab untouched, and nothing but the size of RAM enforces that.
//!
//! So the runner now CHECKS: if the store high-water mark reaches the slab,
//! it says so, because an allocator that overwrites its own input mid-read
//! compiles corrupted source and halts looking like an ordinary result. The
//! remedy is `YANTRA_RAM`, large enough that the slab sits above the peak;
//! RAM is mapped lazily, so only touched pages cost the host anything.

/// The tag in front of the SOURCE run's pointer word: `"SASINPUT"` read as a
/// little-endian word, so a hex dump shows the interface by name. All three tags
/// are below 2⁶³ on purpose: a word above that is negative to a signed lowering,
/// and the `.t1` literals that declare them go through that lowering.
///
/// The `.t1` literals and these constants are two statements of three values;
/// `tests/t1_input_channel.rs` reads the literals out of `shrinkhala.t1` and
/// fails if they ever differ.
pub const INPUT_TAG: u64 = 0x5455_504e_4953_4153;
/// The tag in front of the MODULE NAME run's pointer word: `"SASINAME"`.
pub const NAME_TAG: u64 = 0x454d_414e_4953_4153;
/// The tag in front of the TRACE word: `"SASTRACE"`.
pub const TRACE_TAG: u64 = 0x4543_4152_5453_4153;
/// The tag in front of the ARGUMENT run's pointer word: `"SASARGV\0"`.
///
/// **ARGUMENTS NEED NO DEVICE, AND THAT IS THE WHOLE DESIGN.** A file name must
/// be asked for WHILE RUNNING, which is why it cost an MMIO window and two
/// intrinsics (ADR-0041). Arguments are known before `pc` moves, so they fit the
/// channel that already exists: a tag, a global, and octets written into RAM
/// before the program starts. No compiler change, no `.t1` change, and
/// therefore no fixpoint round — the cheapest of the three capabilities by a
/// wide margin, and cheap for a reason rather than by luck.
///
/// Below 2⁶³ like the other three: a word above that is negative to a signed
/// lowering, and the `.t1` literal that declares it goes through that lowering.
pub const ARGV_TAG: u64 = 0x0056_4752_4153_4153;
/// The tag in front of the ARGUMENT COUNT word: `"SASARGC\0"`.
pub const ARGC_TAG: u64 = 0x0043_4752_4153_4153;

/// Where an injection put things, for the run's own report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Injected {
    /// Offsets of the three tags in RAM — source, name, trace.
    pub tags_at: [usize; 3],
    /// RAM length before the slab was appended.
    pub old_top: usize,
    /// RAM length after. The slab is `old_top..new_top`; a store at or above
    /// `old_top` means the program's allocator reached the input.
    pub new_top: usize,
    /// Guest address now held in the source run's pointer word.
    pub text_ptr: u64,
    /// Guest address now held in the name run's pointer word.
    pub name_ptr: u64,
}

fn align_up(v: usize, a: usize) -> usize {
    v.div_ceil(a) * a
}

fn put_word(mem: &mut [u8], at: usize, w: u64) {
    mem[at..at + 8].copy_from_slice(&w.to_le_bytes());
}

/// Append one run to `mem` in the compiled layout; answer its storage address.
fn append_run(mem: &mut Vec<u8>, base: u64, octets: &[u8]) -> u64 {
    // The length word sits eight below storage, and storage is 16-aligned.
    let storage = align_up(mem.len() + 8, 16);
    let header = storage - 8;
    mem.resize(storage + octets.len(), 0);
    put_word(mem, header, octets.len() as u64);
    mem[storage..storage + octets.len()].copy_from_slice(octets);
    base + storage as u64
}

/// Place `text` and `name` in RAM and point the program's input globals at them.
///
/// **Refuses rather than guesses.** A missing tag means the image was built
/// without the input interface — running it would compile nothing and look like
/// an empty source. A tag found twice means the scan cannot tell which slot the
/// program reads. Either way nothing is written.
pub fn inject(
    mem: &mut Vec<u8>,
    base: u64,
    text: &[u8],
    name: &[u8],
    trace: u64,
) -> Result<Injected, String> {
    let old_top = mem.len();
    let mut tags_at = [0usize; 3];
    for (k, (tag, what)) in [
        (INPUT_TAG, "SASINPUT"),
        (NAME_TAG, "SASINAME"),
        (TRACE_TAG, "SASTRACE"),
    ]
    .into_iter()
    .enumerate()
    {
        let word = tag.to_le_bytes();
        // EVERY OFFSET, NOT EVERY EIGHTH. Measured 2026-09-21 on the first real
        // image: all three tags present exactly once, 16 apart as designed, and
        // each at an address ≡ 5 (mod 8) — an ODD address, failing even halfword
        // alignment. `yantra` tolerates a misaligned load, which is why the
        // corpus ran; many RV64 cores would trap.
        //
        // THE CAUSE WAS THE LINKER, AND THIS MARGIN USED TO NAME THE WRONG ONE.
        // It said the emitted text asks `॥ संरेखः १६ ॥` and the linked image
        // honours it only relative to its section. `॥ संरेखः ॥` IS A `.text`
        // DIRECTIVE: `संरेखपूरणाष्टकम्` pads with `nop`, because 0x00000000
        // traps on RISC-V. It never governed a data word. What governs these is
        // `encode.t1:5916`, where the assembler puts each object's `.data` at
        // `संरेखान्तरम् पाठसीमा ८` — EIGHT-aligned, relative to that object —
        // while `samyojana.t1`'s `दत्तारम्भः` concatenated the objects with no
        // rounding at all, so an object's whole 8-aligned interior landed
        // wherever the preceding objects' lengths happened to end.
        //
        // Repaired 2026-09-23 by rounding in the linker: the tags now read
        // ≡ 0 (mod 8). They sit at ≡ 8 (mod 16) and that is CORRECT — eight is
        // what the assembler promises and what `ld`/`sd` require; sixteen was
        // never claimed for data by anything but this margin.
        //
        // The scan still walks every offset: the program reads a slot from its
        // label's address, and the host must write exactly there. An
        // 8-aligned-only scan found nothing and refused, correctly and for the
        // wrong reason.
        let hits: Vec<usize> = (0..old_top.saturating_sub(7))
            .filter(|&o| mem[o..o + 8] == word)
            .collect();
        tags_at[k] = match hits.as_slice() {
            [] => {
                return Err(format!(
                    "no input interface in this image: the {what} tag ({tag:#x}) appears at no \
                     word. It was built without the `निवेश…` globals — run the entry \
                     that declares them, or do not pass an input"
                ));
            }
            [one] if one + 16 <= old_top => *one,
            [one] => return Err(format!("the {what} tag at {one:#x} has no word after it")),
            many => {
                return Err(format!(
                    "the {what} tag appears at {} words ({:x?}); each tag must be \
                     unique or the scan cannot tell which slot the program reads",
                    many.len(),
                    many.iter().take(6).collect::<Vec<_>>()
                ));
            }
        };
    }
    let text_ptr = append_run(mem, base, text);
    let name_ptr = append_run(mem, base, name);
    put_word(mem, tags_at[0] + 8, text_ptr);
    put_word(mem, tags_at[1] + 8, name_ptr);
    put_word(mem, tags_at[2] + 8, trace);
    Ok(Injected {
        tags_at,
        old_top,
        new_top: mem.len(),
        text_ptr,
        name_ptr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: u64 = 0x8000_0000;

    /// An image with each tag at the given offset, as `emit_data` would leave it.
    fn image(tags: [usize; 3]) -> Vec<u8> {
        let mut m = vec![0u8; 4096];
        for (tag, at) in [INPUT_TAG, NAME_TAG, TRACE_TAG].into_iter().zip(tags) {
            m[at..at + 8].copy_from_slice(&tag.to_le_bytes());
        }
        m
    }

    fn word(m: &[u8], at: usize) -> u64 {
        u64::from_le_bytes(m[at..at + 8].try_into().unwrap())
    }

    /// Tags NOT evenly spaced, as a run global's `संरेखः १६` leaves them: the
    /// first draft's fixed +8/+16/+24 would have written into padding here.
    const REALIGNED: [usize; 3] = [0x100, 0x520, 0x940];

    #[test]
    fn each_slot_is_written_at_its_own_tag_plus_eight() {
        let mut m = image(REALIGNED);
        let text = "मण्डलम् क ॥".as_bytes();
        let r = inject(&mut m, BASE, text, b"k", 1).unwrap();
        assert_eq!(r.tags_at, REALIGNED);
        assert_eq!(
            word(&m, 0x108),
            r.text_ptr,
            "source pointer at INPUT_TAG + 8"
        );
        assert_eq!(word(&m, 0x528), r.name_ptr, "name pointer at NAME_TAG + 8");
        assert_eq!(word(&m, 0x948), 1, "trace flag at TRACE_TAG + 8");
    }

    #[test]
    fn a_run_is_laid_out_as_the_compiled_code_reads_one() {
        let mut m = image(REALIGNED);
        let text = "मण्डलम् क ॥".as_bytes();
        let r = inject(&mut m, BASE, text, b"k", 0).unwrap();
        let off = (r.text_ptr - BASE) as usize;
        assert_eq!(off % 16, 0, "storage must be 16-aligned");
        assert_eq!(
            word(&m, off - 8),
            text.len() as u64,
            "length word at storage - 8"
        );
        assert_eq!(&m[off..off + text.len()], text);
        let noff = (r.name_ptr - BASE) as usize;
        assert_eq!(word(&m, noff - 8), 1);
        assert_eq!(&m[noff..noff + 1], b"k");
    }

    #[test]
    fn nothing_below_the_old_top_moves_except_the_three_slots() {
        let mut m = image(REALIGNED);
        let slots = [0x108..0x110, 0x528..0x530, 0x948..0x950];
        let tags = [0x100..0x108, 0x520..0x528, 0x940..0x948];
        for (i, b) in m.iter_mut().enumerate() {
            if !tags.iter().any(|t| t.contains(&i)) {
                *b = (i % 251) as u8;
            }
        }
        let before = m.clone();
        let r = inject(&mut m, BASE, b"abc", b"n", 0).unwrap();
        for i in 0..r.old_top {
            if slots.iter().any(|s| s.contains(&i)) {
                continue;
            }
            assert_eq!(m[i], before[i], "octet {i:#x} changed");
        }
        assert!(
            r.text_ptr >= BASE + r.old_top as u64,
            "input lives ABOVE the old top"
        );
    }

    #[test]
    fn an_image_without_the_interface_is_refused_and_untouched() {
        let mut m = vec![0u8; 4096];
        let before = m.clone();
        let e = inject(&mut m, BASE, b"x", b"y", 0).unwrap_err();
        assert!(e.contains("SASINPUT"), "{e}");
        assert_eq!(m, before);
    }

    /// A PARTIAL interface is refused too — naming the tag that is missing. The
    /// one-tag design could not express this case at all.
    #[test]
    fn a_partial_interface_is_refused_naming_the_missing_tag() {
        let mut m = image(REALIGNED);
        m[0x940..0x948].fill(0); // no trace tag
        let before = m.clone();
        let e = inject(&mut m, BASE, b"x", b"y", 0).unwrap_err();
        assert!(e.contains("SASTRACE"), "{e}");
        assert_eq!(m, before, "nothing written when any tag is missing");
    }

    #[test]
    fn a_duplicated_tag_is_refused_and_untouched() {
        let mut m = image(REALIGNED);
        m[0xc00..0xc08].copy_from_slice(&NAME_TAG.to_le_bytes());
        let before = m.clone();
        let e = inject(&mut m, BASE, b"x", b"y", 0).unwrap_err();
        assert!(e.contains("SASINAME") && e.contains("2 words"), "{e}");
        assert_eq!(m, before);
    }

    /// A MISALIGNED tag IS the interface — measured on the first real image,
    /// where every tag sat at an address ≡ 5 (mod 8). The first version of this
    /// test asserted the opposite, and the scan it protected found nothing.
    #[test]
    fn a_misaligned_tag_is_found_and_its_slot_written_right_after_it() {
        let tags = [0x10d, 0x51d, 0x92d];
        let mut m = image(tags);
        let r = inject(&mut m, BASE, b"src", b"nm", 1).unwrap();
        assert_eq!(r.tags_at, tags);
        assert_eq!(
            word(&m, 0x115),
            r.text_ptr,
            "slot at the misaligned tag + 8"
        );
        assert_eq!(word(&m, 0x525), r.name_ptr);
        assert_eq!(word(&m, 0x935), 1);
    }
}

/// Where an argument injection put things.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arguments {
    /// Offsets of the two tags in RAM — the run, then the count.
    pub tags_at: [usize; 2],
    /// Guest address now held in the argument run's pointer word.
    pub argv_ptr: u64,
    /// How many arguments were handed over.
    pub argc: u64,
}

/// Hand the program its command-line arguments.
///
/// **THE ARGUMENTS ARE ONE RUN, JOINED BY A ZERO OCTET.** Not a run of runs:
/// `.t1` has no surface form that puts a run's pointer inside a run — the same
/// constraint that forced ADR-0041's file window to take three arguments rather
/// than one request block. A program walks the octets and splits on ०, which it
/// can already do with what it has.
///
/// **A ZERO SEPARATOR AND NOT A SPACE**, because an argument may contain a
/// space and cannot contain a zero — the shape every execve-family interface
/// settled on, for the same reason.
///
/// The COUNT is handed over separately rather than left to be derived. Counting
/// separators answers `n − 1` for `n` arguments and `0` for none, which cannot
/// be told from one empty argument. A program that must distinguish "no
/// arguments" from "one empty argument" can, and that is not a hypothetical:
/// `prog ""` is a thing a shell does.
///
/// Refuses rather than guesses, exactly as [`inject`] does: a missing tag means
/// the image was built without the argument globals, and a tag found twice
/// means the scan cannot tell which slot the program reads.
pub fn inject_arguments(mem: &mut Vec<u8>, base: u64, args: &[&[u8]]) -> Result<Arguments, String> {
    let mut tags_at = [0usize; 2];
    let old_top = mem.len();
    for (k, (tag, what)) in [(ARGV_TAG, "SASARGV"), (ARGC_TAG, "SASARGC")]
        .into_iter()
        .enumerate()
    {
        let word = tag.to_le_bytes();
        let hits: Vec<usize> = (0..old_top.saturating_sub(7))
            .filter(|&o| mem[o..o + 8] == word)
            .collect();
        tags_at[k] = match hits.as_slice() {
            [] => {
                return Err(format!(
                    "no argument interface in this image: the {what} tag ({tag:#x}) appears at \
                     no word. It was built without the argument globals — declare them, or do \
                     not pass arguments"
                ));
            }
            [one] => *one,
            many => {
                return Err(format!(
                    "the {what} tag appears {} times ({many:?}); the scan cannot tell which \
                     slot the program reads",
                    many.len()
                ));
            }
        };
    }

    let mut joined: Vec<u8> = Vec::new();
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            joined.push(0);
        }
        joined.extend_from_slice(a);
    }
    let argv_ptr = append_run(mem, base, &joined);
    put_word(mem, tags_at[0] + 8, argv_ptr);
    put_word(mem, tags_at[1] + 8, args.len() as u64);
    Ok(Arguments {
        tags_at,
        argv_ptr,
        argc: args.len() as u64,
    })
}

/// The tag in front of the EVENT word: `"SASEVENT"` (`W-371`, ADR-0040 Option C R2).
///
/// At each [`crate::Halt::Wait`] the host writes ONE event record into the word at
/// `tag + 8` and resumes — see [`find_event_slot`] and [`replay`]. Below 2⁶³ like the
/// others, for the same reason.
pub const EVENT_TAG: u64 = 0x544e_4556_4553_4153;
// Below 2⁶³, checked when the crate compiles rather than when a test runs.
const _: () = assert!(EVENT_TAG < 1 << 63);

/// Find the event word: the offset of the ONE `SASEVENT` tag in `mem`.
///
/// Refuses on a missing tag, a tag found more than once, or a tag with no word after it —
/// [`inject`]'s three refusals, for [`inject`]'s reasons. A missing tag is fatal ONLY when
/// the host was given an event log: an image that never declared the interface would
/// otherwise read whatever its own word held at every wait and replay a run that the log
/// does not describe. Nothing is written here; [`replay`] writes at each wait.
pub fn find_event_slot(mem: &[u8]) -> Result<usize, String> {
    let word = EVENT_TAG.to_le_bytes();
    // Every offset, not every eighth — the reason is at `inject`'s scan.
    let hits: Vec<usize> = (0..mem.len().saturating_sub(7))
        .filter(|&o| mem[o..o + 8] == word)
        .collect();
    match hits.as_slice() {
        [] => Err(format!(
            "no event interface in this image: the SASEVENT tag ({EVENT_TAG:#x}) appears at no \
             word. It was built without the event global — declare it, or do not pass an event log"
        )),
        [one] if one + 16 <= mem.len() => Ok(*one),
        [one] => Err(format!("the SASEVENT tag at {one:#x} has no word after it")),
        many => Err(format!(
            "the SASEVENT tag appears at {} words ({:x?}); the tag must be unique or the scan \
             cannot tell which word the program reads",
            many.len(),
            many.iter().take(6).collect::<Vec<_>>()
        )),
    }
}

/// Read an event log. **THE FORMAT, AND THE ONLY ONE:**
///
/// ```text
///   # a comment: any line whose first non-blank character is '#'
///   3              one RECORD per line, one record per WAIT, in the order the waits happen
///   0x10           a record is one 64-bit word: decimal, or hexadecimal after 0x
///                  (blank lines are skipped)
/// ```
///
/// A record is ONE WORD because the event global is one word: the host writes it at
/// `SASEVENT + 8` and the program reads it with one `ld`. A wider record — octets that
/// arrived, a run — is a later row's question; this one only has to make the count a
/// function of `(program, log)`. Anything else on a line is refused naming the line, never
/// read as zero: a typo'd record is a different log.
///
/// **THE CLOCK RECORD (`W-375`, ADR-0040 R6).** A line may instead read `t=<word>` (same
/// number syntax): a record whose value is the HOST'S TIME at that wait, in NANOSECONDS
/// SINCE THE UNIX EPOCH (1970-01-01T00:00:00Z), as [`stamp_event_time`] read it in live mode
/// ([`record_live`], `yantra-run --record-events`). Replay delivers the LOGGED value into
/// the same word, exactly as it delivers a plain record — the clock is never consulted.
/// The extension is compatible both ways: every W-371 log still parses unchanged (a plain
/// record is a record with no time field, delivered as before), and a `t=` record is
/// delivered by the same [`replay`].
///
/// **A THREAD RECORD `@N` IS REFUSED HERE (`W-376`).** It is a line of a THREADED log,
/// read by [`parse_thread_log`]; a single-thread consumer handed one is holding a log of
/// some other kind of run, and reading past the line would deliver the next value to the
/// wrong wait. So every single-thread consumer stays byte-unchanged on every log it could
/// already read, and refuses, by name, the one line kind it cannot.
///
/// **A SOCKET RECORD `s=` IS REFUSED HERE TOO (`W-377`)** — see `refuse_socket_record`.
pub fn parse_event_log(text: &str) -> Result<Vec<u64>, String> {
    refuse_torn_final_record(text)?;
    let mut records = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('@') {
            return Err(format!(
                "line {}: {t:?} is a THREAD record (W-376: `@N` runs thread N) — this is a \
                 threaded log, and a single-thread run has no thread to schedule; refused \
                 rather than skipped",
                n + 1
            ));
        }
        refuse_socket_record(t, n)?;
        records.push(parse_value_record(t, n)?);
    }
    Ok(records)
}

/// A TORN FINAL RECORD REFUSES (review, Naad lane): `lines()` accepts a last
/// line with no newline, so a live run killed mid-write could leave
/// `t=1791000000123456789` as `t=17` and replay a different clock silently.
/// `record_live` always ends a record with `\n`, so a final RECORD line
/// without one is a torn write, never a log someone meant.
pub(crate) fn refuse_torn_final_record(text: &str) -> Result<(), String> {
    if !text.is_empty() && !text.ends_with('\n') {
        let last = text.rsplit('\n').next().unwrap_or("");
        let t = last.trim();
        if !t.is_empty() && !t.starts_with('#') {
            return Err(format!(
                "line {}: {t:?} has no terminating newline — a torn final record \
                 (a live run stopped mid-write), refused rather than replayed",
                text.lines().count()
            ));
        }
    }
    Ok(())
}

/// **A SOCKET RECORD `s=` IS REFUSED BY NAME (`W-377`, addendum §2)**, by both readers
/// here, as `@` is by [`parse_event_log`]: it is a line of a SOCKET log, read by
/// [`crate::socket::parse_socket_log`], and a consumer of value records handed one holds a
/// log of some other kind of run. So every existing consumer — `Interpreter::set_events`
/// included — stays byte-unchanged on every log it could already read.
fn refuse_socket_record(t: &str, n: usize) -> Result<(), String> {
    if t.starts_with("s=") {
        return Err(format!(
            "line {}: {t:?} is a SOCKET record (W-377: `s=<hex>` or `s=end`) — this is a \
             socket log, read only when its first record is one; refused rather than skipped",
            n + 1
        ));
    }
    Ok(())
}

/// One value record — a plain word or a `t=` clock record — on the trimmed line `t`, which
/// is line `n` from 0. The one statement of the number syntax, shared by both log readers.
fn parse_value_record(t: &str, n: usize) -> Result<u64, String> {
    let word = t.strip_prefix("t=").unwrap_or(t);
    let parsed = match word.strip_prefix("0x").or_else(|| word.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(&hex.replace('_', ""), 16),
        None => word.replace('_', "").parse::<u64>(),
    };
    parsed.map_err(|e| {
        format!(
            "line {}: {t:?} is not a record — one unsigned 64-bit word per line, \
             decimal or 0x-hex, optionally after `t=` for a clock record ({e})",
            n + 1
        )
    })
}

/// One record of a THREADED event log (`W-376`, ADR-0040 addendum §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadRecord {
    /// A value for the wait the resuming thread is in: a plain record or a `t=` clock
    /// record, delivered into the `SASEVENT` word AT THE RESUME.
    Value(u64),
    /// `@N`: at this decision point, run thread `N`.
    Run(u32),
}

/// Read a THREADED event log: [`parse_event_log`]'s format, plus the line kind `@N`
/// ("run thread N", `N` decimal or 0x-hex, below 2³²). The order of the records is the
/// schedule: at each decision point (the start, every wait, every thread's end) the host
/// reads one `@N`, and when thread `N` is resuming from a wait, the value record after it.
/// Whether the records fit the run is [`crate::threads::replay_threads`]'s to judge; this
/// only reads them, refusing a line it cannot read by its number, never as zero.
pub fn parse_thread_log(text: &str) -> Result<Vec<ThreadRecord>, String> {
    refuse_torn_final_record(text)?;
    let mut records = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        refuse_socket_record(t, n)?;
        let Some(thread) = t.strip_prefix('@') else {
            records.push(ThreadRecord::Value(parse_value_record(t, n)?));
            continue;
        };
        let parsed = match thread
            .strip_prefix("0x")
            .or_else(|| thread.strip_prefix("0X"))
        {
            Some(hex) => u32::from_str_radix(hex, 16),
            None => thread.parse::<u32>(),
        };
        match parsed {
            Ok(k) => records.push(ThreadRecord::Run(k)),
            Err(e) => {
                return Err(format!(
                    "line {}: {t:?} is not a thread record — `@N` with N a thread number, \
                     decimal or 0x-hex, below 2^32 ({e})",
                    n + 1
                ));
            }
        }
    }
    Ok(records)
}

/// How a replay ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Replayed {
    /// The machine halted for a reason that is not a wait, after `delivered` records. A
    /// caller holding a longer log has a log this run did not consume — see [`replay`].
    Halted {
        /// The halt.
        halt: crate::Halt,
        /// Records written, one per wait.
        delivered: usize,
    },
    /// The program waited and the log had no record left for it. `index` counts waits
    /// from 0 — it equals the number of records the log held — and `pc` is the store that
    /// asked. The machine is left paused at that wait; nothing was written.
    Short {
        /// Which wait, from 0.
        index: usize,
        /// The wait store.
        pc: u64,
    },
}

/// Replay `log` against `m`: run; at each [`crate::Halt::Wait`] write the next record at
/// `tag + 8` (`tag` from [`find_event_slot`]) and run again; stop at any other halt.
///
/// `budget` is the ceiling on the WHOLE replay, not on each segment: each resume gets what
/// is left of it, measured by [`crate::Machine::time`], which accumulates across resumes.
///
/// **A SHORT LOG IS NEVER PADDED.** When the waits outnumber the records this returns
/// [`Replayed::Short`] and does not resume — resuming would hand the program whatever its
/// word last held, a zero or a stale record, and call that a replay.
///
/// A LONG LOG is the caller's to judge, from `delivered` against `log.len()`; `yantra-run`
/// REFUSES it, because a log not consumed exactly is a log of some other run.
pub fn replay(
    m: &mut crate::Machine,
    tag: usize,
    log: &[u64],
    budget: u64,
    out: &mut impl crate::Output,
) -> Replayed {
    let start = m.time;
    let mut delivered = 0;
    loop {
        let left = budget.saturating_sub(m.time - start);
        match m.run(left, out) {
            crate::Halt::Wait { pc } => {
                let Some(&record) = log.get(delivered) else {
                    return Replayed::Short {
                        index: delivered,
                        pc,
                    };
                };
                put_word(&mut m.mem, tag + 8, record);
                delivered += 1;
            }
            halt => return Replayed::Halted { halt, delivered },
        }
    }
}

/// **THE EVENT STAMPER — THE ONE PLACE `yantra` READS A WALL CLOCK (`W-375`, ADR-0040 R6).**
///
/// Answers the host's current time in NANOSECONDS SINCE THE UNIX EPOCH (1970-01-01T00:00:00Z,
/// UTC), as one `u64` — enough until the year 2554. A host clock set before 1970 answers 0
/// rather than panicking; the value is DATA delivered to a program, not a check.
///
/// It is called from exactly two sites, [`record_live`] at a [`crate::Halt::Wait`] and its
/// threaded twin [`crate::threads::record_live_threads`] when a thread RESUMES from one
/// (`W-376`) — so a program sees time only as a value delivered at a wait point it chose, never between two
/// instructions, and the value is LOGGED, so a replay reproduces the run without asking the
/// clock again. `tests/w375_clock.rs` holds a ratchet: no other line under `crates/yantra/src`
/// may name a wall clock. Keep it that way; a second clock read is a run no log describes.
pub fn stamp_event_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64)
}

/// The first line of a log [`record_live`] writes: a comment, so it carries no clock value
/// (a time with no wait would be exactly what `W-375` forbids).
pub const RECORDED_LOG_HEADER: &str = "# yantra-run --record-events (W-375): one clock record \
     per wait, t=<host time in ns since the Unix epoch, UTC>; replay with --events";

/// LIVE MODE: run `m`; at each [`crate::Halt::Wait`] stamp the host's time
/// ([`stamp_event_time`]), write it at `tag + 8`, APPEND the record `t=<ns>` to `log` (flushed
/// per record, so a run that dies mid-way leaves the log of what it was given), and resume.
/// Stops at the first halt that is not a wait and answers it with the number of records
/// written. `budget` covers the whole run, as in [`replay`].
///
/// The log is a valid input to [`parse_event_log`] and [`replay`], which deliver the same
/// words to the same waits — so the replayed run retires the same count.
pub fn record_live(
    m: &mut crate::Machine,
    tag: usize,
    budget: u64,
    out: &mut impl crate::Output,
    log: &mut impl std::io::Write,
) -> Result<(crate::Halt, usize), String> {
    let io = |e: std::io::Error| format!("writing the event log: {e}");
    writeln!(log, "{RECORDED_LOG_HEADER}").map_err(io)?;
    log.flush().map_err(io)?;
    let start = m.time;
    let mut delivered = 0;
    loop {
        let left = budget.saturating_sub(m.time - start);
        match m.run(left, out) {
            crate::Halt::Wait { .. } => {
                let now = stamp_event_time();
                writeln!(log, "t={now}").map_err(io)?;
                log.flush().map_err(io)?;
                put_word(&mut m.mem, tag + 8, now);
                delivered += 1;
            }
            halt => return Ok((halt, delivered)),
        }
    }
}

#[cfg(test)]
mod event_tests {
    #[test]
    fn a_torn_final_record_is_refused_not_replayed() {
        let e = super::parse_event_log("t=1\nt=17").unwrap_err();
        assert!(e.contains("line 2") && e.contains("torn"), "{e}");
        assert_eq!(super::parse_event_log("t=1\nt=17\n").unwrap(), vec![1, 17]);
        assert_eq!(super::parse_event_log("t=1\n# note").unwrap(), vec![1]);
    }

    use super::*;

    #[test]
    fn the_log_format_reads_decimal_hex_comments_and_blanks() {
        let log = "# header\n\n3\n  0x10 \n# mid\n1_000\n";
        assert_eq!(parse_event_log(log).unwrap(), vec![3, 16, 1000]);
        assert_eq!(parse_event_log("").unwrap(), Vec::<u64>::new());
    }

    #[test]
    fn a_bad_record_is_refused_naming_its_line_never_read_as_zero() {
        for bad in [
            "3\nfive\n",
            "3\n-1\n",
            "3\n0xzz\n",
            "3\n18446744073709551616\n",
        ] {
            let e = parse_event_log(bad).unwrap_err();
            assert!(e.starts_with("line 2:"), "{bad:?}: {e}");
        }
    }

    #[test]
    fn the_event_tag_is_found_once_and_refused_missing_duplicated_or_truncated() {
        let mut m = vec![0u8; 256];
        assert!(find_event_slot(&m).unwrap_err().contains("SASEVENT"));
        m[0x41..0x49].copy_from_slice(&EVENT_TAG.to_le_bytes());
        assert_eq!(
            find_event_slot(&m),
            Ok(0x41),
            "a misaligned tag is the interface"
        );
        m[0x80..0x88].copy_from_slice(&EVENT_TAG.to_le_bytes());
        assert!(find_event_slot(&m).unwrap_err().contains("2 words"));
        let mut t = vec![0u8; 16];
        t[4..12].copy_from_slice(&EVENT_TAG.to_le_bytes());
        assert!(
            find_event_slot(&t)
                .unwrap_err()
                .contains("no word after it")
        );
    }

    #[test]
    fn a_clock_record_reads_as_its_logged_value_beside_plain_records() {
        let log = format!("{RECORDED_LOG_HEADER}\nt=1791000000123456789\n7\nt=0x10\n");
        assert_eq!(
            parse_event_log(&log).unwrap(),
            vec![1_791_000_000_123_456_789, 7, 16]
        );
        let e = parse_event_log("t=\n").unwrap_err();
        assert!(e.starts_with("line 1:"), "{e}");
        assert!(parse_event_log("t=-1\n").is_err());
        assert!(parse_event_log("time=5\n").is_err());
    }

    #[test]
    fn a_thread_record_is_refused_by_the_single_thread_reader_by_name() {
        let e = parse_event_log("3\n@1\n5\n").unwrap_err();
        assert!(
            e.starts_with("line 2:") && e.contains("THREAD record"),
            "{e}"
        );
    }

    #[test]
    fn the_thread_log_reads_runs_values_and_clock_records() {
        let log = "# W-376\n@0\n@1\n@0x0\nt=7\n5\n";
        assert_eq!(
            parse_thread_log(log).unwrap(),
            vec![
                ThreadRecord::Run(0),
                ThreadRecord::Run(1),
                ThreadRecord::Run(0),
                ThreadRecord::Value(7),
                ThreadRecord::Value(5),
            ]
        );
        for bad in ["@\n", "@-1\n", "@x\n", "@4294967296\n"] {
            let e = parse_thread_log(bad).unwrap_err();
            assert!(e.starts_with("line 1:"), "{bad:?}: {e}");
        }
        assert!(parse_thread_log("@0\n@1").unwrap_err().contains("torn"));
    }

    #[test]
    fn the_tag_spells_sasevent() {
        assert_eq!(&EVENT_TAG.to_le_bytes(), b"SASEVENT");
    }
}
