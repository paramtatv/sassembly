//! The loader — ADR-0015 clause A1, the register contract of ADR-0016, task `F-001e2a`.
//!
//! # This is not part of the machine
//!
//! Everything else in this crate is the hardware and the firmware below it. This module is
//! neither: **a loader is supervisor software**, and it is here rather than in `spec/`
//! because the supervisor that will own it does not exist yet. What it produces is
//! ordinary machine state — page tables in RAM, `satp`, `sepc`, `sstatus` and the 32
//! integer registers — so anything that can write those can replace it, in Rust or in
//! Sassembly, without changing a line of the interpreter.
//!
//! # It stops one instruction short, on purpose
//!
//! [`load_application`] does not put the machine in user mode. It leaves `sepc` at the
//! program's `e_entry`, `sstatus.SPP` clear, `satp` installed and every register the
//! contract names already set — and then it stops, because the next instruction is the
//! supervisor's `sret` and **the mode change is the machine's to make**. A loader that
//! assigned `mode = User` in Rust would be asserting the transition rather than performing
//! it, and the test that watched it happen would be watching this file rather than the
//! interpreter. See `tests/loader.rs`, which executes that one instruction and reads the
//! privilege back out.
//!
//! # What the address space contains, and why each part is in it
//!
//! | region | flags | who chose the address |
//! |---|---|---|
//! | the supervisor's RAM, one identity gigapage | `V R W X A D`, **no `U`** | the machine |
//! | the program's segments | `U` + the ELF's own `PF_R/W/X`, `A`, `D` when writable | the ELF |
//! | the stack, [`STACK_PAGES`] pages below [`STACK_TOP`] | `V R W U A D` | **the loader** |
//! | the handle vector, one page at [`HANDLE_VECTOR`] | `V R U A`, never `W` | **the loader** |
//!
//! The supervisor's own gigapage is in the *application's* page table because it has to
//! be: a U-mode `ecall` traps to `stvec`, which is a supervisor address, and a trap that
//! cannot fetch its own handler is a machine that stops. It carries no `U`, so the program
//! it is mapped beside cannot read a byte of it (`tests/user.rs`).
//!
//! **The program's frames are chosen here and appear nowhere the program can read them.**
//! Its virtual addresses are its own link addresses — `auipc` will hand it those, and A1
//! does not pretend otherwise — but the physical page it is standing on is the loader's
//! business, and no register at entry holds one.
//!
//! # Two measured limitations, stated rather than papered over
//!
//! - **`sadhana` emits exactly one `PT_LOAD`, `PF_R | PF_X`** (`kosha.rs`), covering text,
//!   data and `.bss` together. So an application built by this toolchain gets a
//!   *read-only* data section and its only writable memory is the stack this file maps.
//!   The loader honours `p_flags` rather than widening them: a program that needs writable
//!   data needs a second segment from the linker, which is a toolchain change and not a
//!   loader one.
//! - **Handles are one page**, so [`MAX_HANDLES`] of them. A refusal, not a truncation.

use crate::{Machine, PPN_MASK, PTE_A, PTE_D, PTE_R, PTE_U, PTE_V, PTE_W, PTE_X, SATP_SV39};

/// The value in `a2` at entry — `spec/application-entry.tsv`, row `a2`. A program that
/// finds a number it does not know here can exit rather than misread the rest.
pub const ABI_VERSION: u64 = 1;

/// The exclusive top of the stack the loader maps, and `sp` at entry.
///
/// It is 16-byte aligned and it is **not** derived from where the program was linked: A1's
/// "a stack pointer it did not choose" is the whole difference between an application and
/// a boot proof, which sets `sp` from a linker symbol it can see. The page at this address
/// is deliberately absent, so a stack that grows the wrong way faults instead of running
/// into whatever is above it.
pub const STACK_TOP: u64 = 0x0000_0020_0000_0000;

/// How many pages of stack. The page below the lowest is unmapped and is a guard page by
/// omission: an overflow is a store page fault the supervisor reports, not a quiet
/// overwrite of the neighbour.
pub const STACK_PAGES: u64 = 4;

/// Where the handle vector is mapped — `a1` at entry.
///
/// One page, readable and never writable: a program that could rewrite its own grants
/// would be granting itself, which is the thing doc 07 §3 forbids by making nothing
/// ambient.
pub const HANDLE_VECTOR: u64 = 0x0000_0010_0000_0000;

/// How many handles fit in the one page of [`HANDLE_VECTOR`].
pub const MAX_HANDLES: usize = 4096 / 8;

/// The size of a page, everywhere here.
const PAGE: u64 = 4096;

/// The leaf the supervisor's identity gigapage carries: everything except `U`.
const SUPERVISOR: u64 = PTE_V | PTE_R | PTE_W | PTE_X | PTE_A | PTE_D;

/// One `PT_LOAD` of an ELF, as the loader needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    /// Offset of the segment's bytes in the file.
    pub offset: usize,
    /// The virtual address it is to be placed at.
    pub vaddr: u64,
    /// How many bytes of it are in the file.
    pub filesz: usize,
    /// How many bytes it occupies in memory; the difference is zeroed (`.bss`).
    pub memsz: usize,
    /// `p_flags` — `PF_X` 1, `PF_W` 2, `PF_R` 4.
    pub flags: u32,
}

/// A parsed ELF: what both loaders in this crate agree a file has to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// `e_entry`.
    pub entry: u64,
    /// Every `PT_LOAD`, in header order.
    pub segments: Vec<Segment>,
}

impl Program {
    /// Read the header and the program headers.
    ///
    /// # Errors
    /// A string naming what is wrong with the file. Every check here is one whose failure
    /// would otherwise be a *wrong answer* rather than a crash: a 32-bit or big-endian or
    /// x86 file would be interpreted as something, and what it was interpreted as would be
    /// nonsense.
    pub fn parse(image: &[u8]) -> Result<Self, String> {
        let rd16 = |o: usize| u16::from_le_bytes([image[o], image[o + 1]]);
        let rd32 = |o: usize| u32::from_le_bytes(image[o..o + 4].try_into().unwrap());
        let rd64 = |o: usize| u64::from_le_bytes(image[o..o + 8].try_into().unwrap());

        if image.len() < 64 || &image[0..4] != b"\x7fELF" {
            return Err("not an ELF file".into());
        }
        if image[4] != 2 {
            return Err(format!(
                "ELF class {} — this loader is 64-bit only",
                image[4]
            ));
        }
        if image[5] != 1 {
            return Err("big-endian ELF — this loader is little-endian only".into());
        }
        if rd16(18) != 243 {
            return Err(format!(
                "e_machine {} — expected 243 (RISC-V). The browser VM runs the SAME \
                 artefact QEMU does; a different machine means a different artefact.",
                rd16(18)
            ));
        }

        let phoff = rd64(32) as usize;
        let phentsize = rd16(54) as usize;
        let phnum = rd16(56) as usize;
        let mut segments = Vec::new();
        for i in 0..phnum {
            let o = phoff + i * phentsize;
            if o + 56 > image.len() {
                return Err("program header runs past the end of the file".into());
            }
            if rd32(o) != 1 {
                continue; // not PT_LOAD
            }
            let offset = rd64(o + 8) as usize;
            let filesz = rd64(o + 32) as usize;
            if offset + filesz > image.len() {
                return Err("a segment's contents run past the end of the file".into());
            }
            segments.push(Segment {
                offset,
                vaddr: rd64(o + 16),
                filesz,
                memsz: rd64(o + 40) as usize,
                flags: rd32(o + 4),
            });
        }
        if segments.is_empty() {
            return Err("no PT_LOAD segment — nothing to run".into());
        }
        Ok(Program {
            entry: rd64(24),
            segments,
        })
    }
}

/// What the loader built, for a supervisor that has to remember it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Loaded {
    /// The program's entry point, which is also `sepc`.
    pub entry: u64,
    /// `sp` at entry — [`STACK_TOP`].
    pub sp: u64,
    /// `a1` at entry — [`HANDLE_VECTOR`], mapped whether or not any handle was granted.
    pub handles: u64,
    /// The `satp` the loader installed.
    pub satp: u64,
    /// The first physical frame the loader did **not** take. What is left of the pool.
    pub free: u64,
}

/// The frame pool: physical pages the loader may take, one at a time, upward.
///
/// A loader does not invent memory. It is handed a range and it refuses when the range
/// runs out, because the alternative — allocating past the end — is a program placed on
/// top of whatever was there.
struct Frames {
    next: u64,
    end: u64,
}

impl Frames {
    /// Take one zeroed frame.
    fn take(&mut self, m: &mut Machine) -> Result<u64, String> {
        if self.next + PAGE > self.end {
            return Err(format!(
                "out of frames at {:#x}: the pool ends at {:#x}",
                self.next, self.end
            ));
        }
        let pa = self.next;
        self.next += PAGE;
        let at = phys(m, pa, PAGE as usize)?;
        m.mem[at..at + PAGE as usize].fill(0);
        Ok(pa)
    }
}

/// The offset in `m.mem` of a physical span, or a stated refusal.
pub(crate) fn phys(m: &Machine, pa: u64, len: usize) -> Result<usize, String> {
    let at = pa
        .checked_sub(m.base)
        .ok_or_else(|| format!("{pa:#x} is below the base of RAM ({:#x})", m.base))?
        as usize;
    if at + len > m.mem.len() {
        return Err(format!(
            "{pa:#x}+{len} runs past the end of RAM ({:#x})",
            m.base + m.mem.len() as u64
        ));
    }
    Ok(at)
}

/// Read a `u64` from physical memory.
pub(crate) fn get(m: &Machine, pa: u64) -> Result<u64, String> {
    let at = phys(m, pa, 8)?;
    Ok(u64::from_le_bytes(m.mem[at..at + 8].try_into().unwrap()))
}

/// Write a `u64` to physical memory.
fn put(m: &mut Machine, pa: u64, v: u64) -> Result<(), String> {
    let at = phys(m, pa, 8)?;
    m.mem[at..at + 8].copy_from_slice(&v.to_le_bytes());
    Ok(())
}

/// Write one instruction word to physical memory.
///
/// Supervisor software has to be able to place its own text, and [`crate::supervisor`] is
/// exactly one word of it. The refusal is `phys`'s: an address outside RAM.
///
/// # Errors
/// A string naming what is wrong with the physical address.
pub(crate) fn put32(m: &mut Machine, pa: u64, w: u32) -> Result<(), String> {
    let at = phys(m, pa, 4)?;
    m.mem[at..at + 4].copy_from_slice(&w.to_le_bytes());
    Ok(())
}

/// A leaf entry for physical page `pa` with `flags`.
const fn leaf(pa: u64, flags: u64) -> u64 {
    (pa >> 12) << 10 | flags
}

/// Map one 4 KiB page, creating the two intermediate tables if they are absent.
///
/// Refuses to map an address twice, and refuses to walk *through* a leaf: both are the
/// loader silently disagreeing with itself about where something is, and the second is how
/// a program linked into the supervisor's gigabyte would be quietly given the supervisor's
/// pages instead of its own.
fn map(
    m: &mut Machine,
    frames: &mut Frames,
    root: u64,
    va: u64,
    pa: u64,
    flags: u64,
) -> Result<(), String> {
    let mut table = root;
    for level in [2u32, 1] {
        let slot = table + ((va >> (12 + 9 * level)) & 0x1ff) * 8;
        let pte = get(m, slot)?;
        table = if pte & PTE_V == 0 {
            let next = frames.take(m)?;
            put(m, slot, leaf(next, PTE_V))?;
            next
        } else {
            if pte & (PTE_R | PTE_W | PTE_X) != 0 {
                return Err(format!(
                    "{va:#x} is inside a level-{level} superpage that is already mapped — \
                     the supervisor's gigabyte is at {:#x} and an application may not be \
                     linked into it",
                    m.base
                ));
            }
            ((pte >> 10) & PPN_MASK) << 12
        };
    }
    let slot = table + ((va >> 12) & 0x1ff) * 8;
    if get(m, slot)? & PTE_V != 0 {
        return Err(format!("{va:#x} is mapped twice"));
    }
    put(m, slot, leaf(pa, flags | PTE_V | PTE_A))
}

/// Build the address space and the register file an application is entered with, and leave
/// the machine one `sret` short of running it.
///
/// `free` is the page-aligned physical address where the loader may start taking frames;
/// everything below it belongs to the supervisor. `handles` is the grant — every surface
/// the program may touch (A4), by number, and there is no other way for it to obtain one.
///
/// On return: `satp` is a fresh Sv39 space, `sepc` is `e_entry`, `sstatus.SPP` is clear,
/// and `x[0..32]` is exactly what `spec/application-entry.tsv` says. The machine is still
/// in supervisor mode and the next instruction is the supervisor's `sret`.
///
/// # Errors
/// A string. Every refusal here is a load the loader cannot honestly complete: a bad ELF,
/// a segment that is not page-aligned or that collides with something already mapped, more
/// handles than the vector holds, or a frame pool that ran out.
pub fn load_application(
    m: &mut Machine,
    image: &[u8],
    free: u64,
    handles: &[u64],
) -> Result<Loaded, String> {
    if !free.is_multiple_of(PAGE) {
        return Err(format!("the frame pool starts at {free:#x}, not on a page"));
    }
    if handles.len() > MAX_HANDLES {
        return Err(format!(
            "{} handles: the vector is one page, so {MAX_HANDLES} at most",
            handles.len()
        ));
    }
    let program = Program::parse(image)?;
    let end = m.base + m.mem.len() as u64;
    let mut frames = Frames { next: free, end };
    let root = frames.take(m)?;

    // The supervisor's own RAM, identically mapped and without `U`. A U-mode `ecall`
    // arrives at `stvec`, which is in here; a program cannot read a byte of it.
    let giga = m.base & !((1 << 30) - 1);
    put(m, root + ((giga >> 30) & 0x1ff) * 8, leaf(giga, SUPERVISOR))?;

    for segment in &program.segments {
        if !segment.vaddr.is_multiple_of(PAGE) {
            return Err(format!(
                "a segment is at {:#x}, which is not a page boundary — this loader places \
                 whole frames and will not shift a program to suit itself",
                segment.vaddr
            ));
        }
        let mut flags = PTE_U;
        if segment.flags & 4 != 0 {
            flags |= PTE_R;
        }
        if segment.flags & 2 != 0 {
            flags |= PTE_W | PTE_D;
        }
        if segment.flags & 1 != 0 {
            flags |= PTE_X;
        }
        let span = segment.memsz.max(segment.filesz) as u64;
        for page in 0..span.div_ceil(PAGE) {
            let frame = frames.take(m)?;
            // The bytes of this page that are in the file. The rest stays zero, which is
            // `.bss`; `p_offset` need not be page-aligned, so this is measured from the
            // segment's start rather than from the frame's.
            let from = (page * PAGE) as usize;
            let take = segment.filesz.saturating_sub(from).min(PAGE as usize);
            if take > 0 {
                let at = phys(m, frame, take)?;
                let src = segment.offset + from;
                m.mem[at..at + take].copy_from_slice(&image[src..src + take]);
            }
            map(
                m,
                &mut frames,
                root,
                segment.vaddr + page * PAGE,
                frame,
                flags,
            )?;
        }
    }

    for page in 1..=STACK_PAGES {
        let frame = frames.take(m)?;
        let va = STACK_TOP - page * PAGE;
        map(
            m,
            &mut frames,
            root,
            va,
            frame,
            PTE_R | PTE_W | PTE_U | PTE_D,
        )?;
    }

    let vector = frames.take(m)?;
    for (i, handle) in handles.iter().enumerate() {
        put(m, vector + i as u64 * 8, *handle)?;
    }
    map(m, &mut frames, root, HANDLE_VECTOR, vector, PTE_R | PTE_U)?;

    // The register file, all 32 of it. `spec/application-entry.tsv` states every one
    // because a contract that names three and says "the rest are whatever" is not a
    // contract, and this is the line that keeps the loader's leftovers out of `s3`.
    m.x = [0; 32];
    m.x[2] = STACK_TOP;
    m.x[10] = handles.len() as u64;
    m.x[11] = HANDLE_VECTOR;
    m.x[12] = ABI_VERSION;

    let satp = SATP_SV39 | (root >> 12);
    m.csr.satp = satp;
    m.csr.sepc = program.entry;
    m.csr.sstatus &= !(1 << 8); // SPP: the `sret` below returns to U-mode.
    Ok(Loaded {
        entry: program.entry,
        sp: STACK_TOP,
        handles: HANDLE_VECTOR,
        satp,
        free: frames.next,
    })
}
