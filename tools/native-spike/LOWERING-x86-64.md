# RV64 → x86-64 lowering map (native route B). Reviewed by sansos-5f 2026-10-08: SOUND on all 9 divergences, findings 1–5 folded in. Next: sansos-ad, then the owner.

This is the executable form of `rv2x86.py` (spike, branch agent/native-b-spike). The .t1 translator must lower each RV64 instruction exactly as below. yantra is the oracle, checked two ways:
- byte-identical outputs;
- in `--count` mode, identical retired-instruction counts.

## Machine model
- **Guest RAM** is mapped at its own address (0x8000_0000 + RAM + the injected slab), so a guest address IS a host address.
- **Registers.** The 12 statically hottest x-registers live in rbx, rbp, r12–r15, rsi, rdi, r8–r11. The other x-registers and all f-registers live in `regs[32]` / `fregs[32]`.
  - x0 reads as 0, and a write to it is dropped.
  - rax, rcx and rdx are scratch.
- **Labels.** Every 4-byte word of the loaded span gets a label `G_<pc>` (superset translation; the images are uncompressed and mix code and data). A data word's code is never reached; an undecodable word lowers to `bad_insn(pc, word)`.
- **Slow path.** `slow(word, pc)` is a host interpreter of ONE instruction. The mapped registers are spilled to `regs[]` before the call and reloaded after it.
- **Counting** (attestation, optional): each instruction starts with `sub [budget],1; jb step_limit(pc)`. This gives exact `steps` and yantra's StepLimit pc.

## Integer

| RV64 | x86-64 | divergence handled |
|---|---|---|
| lui rd,imm | `mov rd, sext32(imm<<12)` | — |
| auipc rd,imm | `mov rax, pc+imm` (a constant at translation time) | — |
| addi / xori / ori / andi | `mov rax,rs1; op rax,imm12; mov rd,rax` (addi between mapped registers: `lea`) | — |
| slti / sltiu | `cmp rax,imm; setl/setb al; movzx` | sltiu compares against the SIGN-EXTENDED imm, unsigned (x86 does the same) |
| slli / srli / srai | `shl/shr/sar rax,sh6` | — |
| add / sub / and / or / xor | `op rax, rs2` | rs2 = x0 folded |
| slt / sltu | `cmp; setl/setb` | — |
| sll / srl / sra | `mov rcx,rs2; shl/shr/sar rax,cl` | x86 masks the count to 63, as RV does |
| addiw / slliw / srliw / sraiw, addw / subw / sllw / srlw / sraw / mulw | the 32-bit op on eax/ecx, then `movsxd rax,eax` | **the *W result sign-extends from bit 31**; shifts mask to 31, as on x86 |
| mul | `imul rax,rcx` | — |
| mulh / mulhu | one-operand `imul` / `mul rcx`; the result is rdx | — |
| **mulhsu** | slow: `(i128)(i64)a * (u128)b >> 64` | **x86 has no signed×unsigned form** |
| div / divw | rs2 = 0 → −1; (INT_MIN, −1) → INT_MIN; otherwise `cqo/cdq; idiv` | **x86 idiv TRAPS on both** |
| rem / remw | rs2 = 0 → the dividend; (INT_MIN, −1) → 0; otherwise `idiv`, result rdx | **RV gives the dividend for rem by 0** |
| divu / divuw | rs2 = 0 → all ones; otherwise `xor edx,edx; div` | x86 div traps on 0 |
| remu / remuw | rs2 = 0 → the dividend (sign-extended for W) | — |
| lb / lh / lw / ld / lbu / lhu / lwu | `movsx` / `movsxd` / `mov` / `movzx` from `[rs1+imm]`, behind the SAME guard as stores: below 2^31 → the host layer's MMIO load (COUNTER, socket, virtio, patra), past RAM+slab → BeyondRam (review 5f-1) | misaligned is allowed (x86 and yantra agree) |
| sb / sh / sw / sd | `mov [rs1+imm], cl/cx/ecx/rcx` after `addr>>31 == 0 ? mmio : ram` | **MMIO (UART, FINISHER, …) goes to the host layer**; a store below 2^31 never touches host memory |
| beq … bgeu | `cmp rs1,rs2; je/jne/jl/jge/jb/jae G_t` | an out-of-span target goes to `bad_jump` |
| jal rd,off | `mov rd,pc+4; jmp G_t` | — |
| jalr rd,rs1,imm | `t=(rs1+imm)&~1; rd=pc+4; jmp [jt+(t−base)*2]`, checked: in the span and 4-aligned | rd is written AFTER t is read (rd = rs1 is legal); a misaligned or out-of-span target reproduces yantra's halt (its kind and pc), pinned by a test (5f-4) |
| fence | nothing (one hart) | fence.i: see the open items |
| ecall | slow: SBI EID 1 putchar, EID 8 shutdown; any other EID is refused by name (as yantra) | — |
| ebreak / wfi / sret / mret / an undecodable word | `bad_insn` | — |
| csrr* | slow: sstatus stored; fflags/frm/fcsr live; **time/cycle/instret refused** unless `--count` answers them | — |
| A (lr / sc / amo*) | slow: one hart, one reservation | — |

## Floating point (F, D): IMPLEMENTED (anuvada.py runtime_fp, softfp.py; aarch64: runtime_fp_a64)

An image that uses F/D arithmetic, flw / fsw or the fflags / frm / fcsr CSRs (and only such an image: fld / fsd alone stay plain moves)
carries the F/D runtime and two data symbols (`fcsr`, `fsf`, after the data block); every other image is translated exactly as before.
- **Registers.** The f-registers live in `fregs[32]` (raw 64 bits; a single is NaN-boxed). An instruction lowers to: the integer source in
  rax, the f sources into xmm0..2 (`movq xmm, [fregs+8r]`), the rm field in ecx, `call fph_<id>`, the result from xmm0 (`movq [fregs+8r], xmm0`)
  or rax. A helper touches only rax rcx rdx xmm0-7. DYN arms the fault site first (frm 5..7 is an illegal instruction at that pc).
- **fcsr** (frm << 5 | fflags) is one word in memory; fflags accumulate there from MXCSR (a 4-bit reversal of PE UE OE ZE; IE is NV).
- **Arithmetic** (add sub mul div sqrt, fcvt.s.d, int to float, fma via VEX vfmadd213, FMA3 needed) runs on the SSE unit with MXCSR set
  per instruction (RC from the rm, FTZ = DAZ = 0, flags cleared): RNE / RTZ / RDN / RUP. Every NaN result is replaced by the canonical one.
  Tininess is detected after rounding by SSE as by RISC-V (measured: 0 disagreements). `inf * 0 + qNaN` raises NV by an explicit test.
- **RMM** has no MXCSR mode: the software engine (softfp.py, yantra's round_pack algorithm over 128-bit integers; one program, compiled
  by a backend per architecture, self-tested against an exact rational reference) does add sub mul div sqrt fma fcvt.s.d and int to float.
- **Float to integer** (all five modes) is exact integer arithmetic on the double's bits (fpx_f2i): saturation, NaN to the maximum, NV alone
  when saturated, NX when rounded. **fmin / fmax / fclass / fsgnj\* / fmv / compares** are bit logic and ucomisd / comisd (feq quiet, flt and fle signalling).
- **NaN-boxing:** a single source whose upper half is not all ones reads as the canonical NaN; results are boxed; fsgnj.s / fmv.x.w read the raw low half.
- **aarch64:** the FPU under FPCR (DN = 1 gives the canonical NaN, FZ = 0), FPSR to fflags (a 5-bit reversal), the conversion instruction chosen by the mode
  (fcvtns / zs / ms / ps / as), NaN fixed to the maximum; ARM detects tininess BEFORE rounding, so a result of exactly the smallest normal with UFC raised
  is redone by the software engine.

- **sstatus.FS** is modelled for an image that uses floats: csrrw / csrrs / csrrc (and the immediate forms) on sstatus with rd = x0 maintain an
  FS-enabled word (`fsen`; only FS, bits 14:13, of sstatus is kept), and every float instruction (fld fsd flw fsw, the F/D operations, the fflags / frm / fcsr
  CSRs) tests it first and, when it is off, halts as yantra does: `halt: Undelivered { pc: N, cause: 2 }` (an illegal instruction with no handler), exit 1.
  An image with no float instruction is translated exactly as before (sstatus writes lower to nothing).
- **FMA3** (x86-64): the fused multiply-adds test CPUID once (FMA, OSXSAVE, AVX; XCR0 enables xmm and ymm state) and use the software engine,
  which is exact, when the host lacks it. `ANUVADA_NO_FMA3=1` (a translation-time switch, tests only) forces that path; `ANUVADA_SOFT_ALL=1` sends every F/D
  operation through the engine.

## Known differences from yantra: encodings the translator refuses (deliberate, named)
Where yantra halts `Unimplemented` or `Illegal`, the native program refuses at the same pc with `native: not in milestone 1 (an instruction the translator
does not lower) at pc N` and exit 1: a reserved static rounding mode (rm 5 or 6) on an instruction that has an rm field; fsqrt with rs2 != 0; fcvt.s.d with
rs2 != 1 or fcvt.d.s with rs2 != 0; fsgnj with funct3 > 2, fmin / fmax with funct3 > 1, a compare with funct3 > 2; fcvt to or from an integer with rs2 > 3; fmv
with a wrong funct3 or rs2; a format field of half or quad (fmt 2, 3); the other CSRs (anything but fflags / frm / fcsr and sstatus without a read). A DYN
instruction while frm holds 5..7 names its own pc through the same refusal (yantra: an illegal instruction). Both exit non-zero having printed nothing more to
stdout; only the text on stderr differs. The other sstatus bits (SIE, SPP, SUM, ...) are not modelled, and a read of sstatus is refused.

## The host layer (the sandbox is kept, owner ruling)
UART → stdout; FINISHER → exit with yantra's status rule. The patra file window, net frames, WAIT, events and threads are to be ported with yantra's refusals unchanged:
- the patra root;
- O_NOFOLLOW;
- the frame caps;
- a replay header check.

## Open items for review
1. **Code-writing stores** (IN THE FIRST MILESTONE, agreed 5f-5). fence.i and stores into the code span must be refused (or fall back to yantra) instead of silently running stale code. The spike checks only "RAM vs MMIO"; the product adds a code-span bound to the store guard.
2. **Stores past yantra's store_limit** (agreed 5f-5) (into the injected slab): yantra halts BeyondRam; native must do the same (one more compare in the guard).
3. ~~RMM arithmetic~~: closed by the softfloat path.
4. **V (vector).** Not lowered (agreed for the compiler milestone): an image that uses vector words is refused until the V map is written.
5. **macOS / Windows.** The host layer needs libSystem / PE imports, beyond Linux syscalls (reviewer's estimate).

## Known differences from yantra: deliberate, NAMED refusals (reviewed SOUND by sansos-5f, 2026-10-08, @0086ae23)
Where yantra would execute, the native runtime refuses with a message that names the pc and address, and exits non-zero:
1. **A jump outside the translated code span, or to a pc that is not 4-aligned** (direct or jalr): yantra executes the target word (data, or an odd pc); native code exists only for the PF_X segment's words.
2. **A store into the code span** (the PF_X segment): yantra allows self-modifying code; native does not translate it.
Because of (2), fence.i is an exact no-op.
Everything else is identical to yantra: the outputs, the halt kinds (BeyondRam / BadAccess / Finisher) and, in attestation mode, the instruction counts.
3. **wasm32 only: a `jalr` to a pc that begins no translated block** (kind 11). The wasm target cuts blocks at control transfers and at every statically materialised code address (auipc/lui+addi pairs, 4-aligned code addresses held in 8-octet data words); a jalr target found by any other means halts naming the address, where yantra and the x86-64 targets (a label per word) would run it. See WASM32.md.

## Windows (aarch64-windows, x86_64-windows), 2026-10-10
The same lowering, with a PE32+ console image and kernel32.dll as the only import (GetStdHandle,
WriteFile, ReadFile, CreateFileW, GetFileSizeEx, VirtualAlloc, ExitProcess, GetEnvironmentStringsW,
GetCommandLineW, WideCharToMultiByte, MultiByteToWideChar). No CRT, no raw system calls.
- **Shims.** Each Linux syscall the runtime makes becomes a call to a shim with Linux's register
  contract: only the answer register changes, -1 means failure. The shim converts to the Windows ABI.
  - Win64: rcx, rdx, r8, r9, a 32-octet shadow space, rsp 16-aligned; r8..r11 and rdx are saved; MXCSR is saved and restored.
  - ARM64: x0..x7; x1..x17 and x30 are saved; x18 (the TEB) is never touched. The call runs on the thread's own stack, because the frame is not a stack Windows knows.
- **aarch64-windows** uses Darwin's addressing. ASLR is forced on ARM64, so the image is DYNAMICBASE: the code is PC-relative, the jump table holds offsets, and guest RAM sits wherever VirtualAlloc puts it, reached through x4 = delta. A one-block `.reloc` satisfies the loader.
- **x86_64-windows** uses Linux's addressing: a fixed-base image at 0x400000 with relocations stripped, and guest RAM at its own address (VirtualAlloc at 0x8000_0000; refused by name if that address is taken).
- **Environment and arguments.** GetEnvironmentStringsW and GetCommandLineW are converted to UTF-8 whole. The command line is split by Microsoft's rules (`win_split` in anuvada.py is the reference, checked against Microsoft's documented examples). Environment names are matched exactly; Windows itself treats them as case-insensitive. A value with a lone surrogate arrives as U+FFFD.
- **Measured on Windows 11 ARM64 (utmwin, 10.0.26200).**
  - aarch64-windows (native): e2e 7/7, including v1.0.0 Stage 2 byte-identical to yantra (status 1200); F/D differential 74/74; the 8 refusal images (`wintest/mkrefuse.sh`) match x86_64-linux in text and status.
  - x86_64-windows (under Prism): e2e 7/7 and the 8 refusals as above. **F/D:** every value is bit-exact, but Prism does not raise MXCSR's sticky exception flags. So the hardware path's fflags read 0: 57 of 74 images differ, in the flags column only. The software engine (ANUVADA_SOFT_ALL=1) passes all arithmetic; the five ops it leaves to hardware (fcmp, fmin/fmax, fcvt.s.d/d.s) differ in flags only. On x86-64 hardware this path is the Linux one. Not yet run on real x64 Windows.
- **YANTRA_INPUT_ENTRY:** the 21 cases of refuse.sh that UTF-16 can carry (`wintest/mkentry.sh`) match x86_64-linux in text and status on both targets. Its invalid-UTF-8 cases cannot be set in a Windows environment.
- **Arguments:** `wintest/r.args` with 13 command lines (`wintest/argcases.txt`) equals `win_split` on both targets.
- **.t1:** both targets port byte-exactly (36/36 on 2026-10-10), but with them main's Stage 1 needs more than 3.5 GiB, so anuvada.t1 is generated without them (mk.sh ANU_WIN=0, the default) and refuses 43620/34404 (10) until that is solved.
