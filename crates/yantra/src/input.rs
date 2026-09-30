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
