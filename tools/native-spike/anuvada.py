#!/usr/bin/env python3
# ROLE: ORACLE for the .t1 translator (native route B; owner: "lowering ok for x86_64").
# anuvāda = translation. RV64 image -> a STATIC x86-64 Linux ELF, BYTE FOR BYTE: its own
# x86-64 encoder, its own runtime written as machine code (Linux syscalls only, no libc),
# its own ELF writer. No assembler, no compiler, no C. The .t1 translator must emit the
# SAME OCTETS as this file for the same image; this file must, in turn, agree with yantra
# (byte-identical program output; the compiler's Stage 2 reproduces itself).
# Lowering: x86-64 (owner-approved 2026-10-08).
# MILESTONE 1 scope: RV64 I + M, fld/fsd as moves, the sstatus CSR write, UART, FINISHER.
# Every other instruction lowers to a refusal that names its pc and word at run time.
# Usage: anuvada.py [--target aarch64-linux] IMAGE.elf OUT [ARG0]   (ARG0: what the program
#        sees as argument 0, yantra's image path; default: IMAGE as given). The default target
#        is x86_64-linux; aarch64-linux is the second target (translate_a64, at the end),
#        aarch64-macos the third (translate_a64 with mac=True, macho_a64), x86_64-macos the
#        fourth (translate with mac=True, macho_x86), wasm32 (and wasm32-attest) the fifth: a .wasm
#        module, translate_wasm at the end (a wasm32 module).
import hashlib
import os
import struct
import sys

import softfp

TEXT = 0x400000          # the host image's text segment
HOST = [3, 5, 12, 13, 14, 15, 6, 7, 8, 9, 10, 11]   # rbx rbp r12..r15 rsi rdi r8..r11
RAX, RCX, RDX, RBX, RSP, RBP, RSI, RDI = range(8)
R8, R9, R10, R11 = 8, 9, 10, 11
CC = {"o": 0, "p": 10, "np": 11, "ns": 9, "b": 2, "ae": 3, "e": 4, "ne": 5, "be": 6, "a": 7, "s": 8, "l": 12, "ge": 13, "le": 14, "g": 15}
UART, FINISHER = 0x1000_0000, 0x0010_0000
# THE CLOCK (ADR-0040 Option C, W-375): a store to WAIT is a request for the host's wall time,
# answered live from the OS clock into the word after the SASEVENT tag; a deadline hint word
# two words after the tag makes the host sleep first (see rt_wait). YANTRA_RECORD_EVENTS=<log>
# appends yantra-run --record-events's records, so yantra-run --events replays a native run.
WAIT = 0x1000_0100
EVENT_TAG = 0x544E_4556_4553_4153
THREADS_TAG = 0x5344_5248_5453_4153       # yantra's SASTHRDS: a threaded image's waits are not clock waits
# yantra's RECORDED_LOG_HEADER (crates/yantra/src/input.rs), octet for octet, newline-ended
REC_HEADER = (b"# yantra-run --record-events (W-375): one clock record per wait, "
              b"t=<host time in ns since the Unix epoch, UTC>; replay with --events\n")
REC_HEADER_LEN = len(REC_HEADER)
DEFAULT_RAM, RAM_HEADROOM = 20 << 20, 16 << 20
# the addresses below base where yantra's Machine::load answers a device, not BadAccess:
# the retired-instruction counter and the socket (0x1000_0108..0x1000_0120), the virtio-mmio slots
LOAD_DEVICES = ((0x1000_0108, 0x18), (0x1000_1000, 0x8000))
M64 = (1 << 64) - 1


def sx(v, bits):
    v &= (1 << bits) - 1
    return v - (1 << bits) if v >> (bits - 1) else v


# ---------------------------------------------------------------- the encoder
class Asm:
    """Octets, labels and rel32 fixups. Every instruction form has ONE encoding (no short
    forms chosen by value), so a lowering's length never depends on where it lands."""

    def __init__(self, origin):
        self.b = bytearray()
        self.origin = origin
        self.labels = {}
        self.fix = []          # (offset of rel32, label)
        self.absfix = []       # (offset of abs32/abs64, label, width, addend)

    def here(self):
        return self.origin + len(self.b)

    def label(self, name):
        assert name not in self.labels, name
        self.labels[name] = self.here()

    def raw(self, *bs):
        for x in bs:
            if isinstance(x, (bytes, bytearray)):
                self.b += x
            else:
                self.b.append(x & 0xFF)

    def i32(self, v):
        self.b += struct.pack("<i", v)

    def u32(self, v):
        self.b += struct.pack("<I", v & 0xFFFFFFFF)

    def u64(self, v):
        self.b += struct.pack("<Q", v & 0xFFFFFFFFFFFFFFFF)

    def rel32(self, label):
        self.fix.append((len(self.b), label))
        self.i32(0)

    def abs32(self, label, addend=0):
        self.absfix.append((len(self.b), label, 4, addend))
        self.i32(0)

    def resolve(self, extra=None):
        labels = dict(self.labels)
        if extra:
            labels.update(extra)
        for off, lab in self.fix:
            struct.pack_into("<i", self.b, off, labels[lab] - (self.origin + off + 4))
        for off, lab, w, add in self.absfix:
            v = labels[lab] + add
            if w == 4:
                assert 0 <= v < 1 << 31, (lab, hex(v))
                struct.pack_into("<i", self.b, off, v)
            else:
                struct.pack_into("<Q", self.b, off, v)

    # -- operands: ("r", n) | ("abs", label, addend) | ("m", base, disp) | ("sib2", index, label)
    def _modrm(self, reg, op):
        """answer (rex_r, rex_x, rex_b) and append modrm/sib/disp"""
        kind = op[0]
        r = (reg >> 3) & 1
        if kind == "r":
            self.raw(0xC0 | (reg & 7) << 3 | (op[1] & 7))
            return r, 0, op[1] >> 3
        if kind == "abs":
            self.raw(0x04 | (reg & 7) << 3, 0x25)
            self.abs32(op[1], op[2])
            return r, 0, 0
        if kind == "m":
            base, disp = op[1], op[2]
            self.raw(0x80 | (reg & 7) << 3 | (base & 7))
            if base & 7 == 4:
                self.raw(0x24)
            self.i32(disp)
            return r, 0, base >> 3
        if kind == "sib2":
            idx = op[1]
            self.raw(0x04 | (reg & 7) << 3, 0x40 | (idx & 7) << 3 | 5)
            self.abs32(op[2])
            return r, idx >> 3, 0
        raise ValueError(op)

    def op(self, opcode, reg, rm, w=1, p66=False):
        """[66] REX opcode modrm…  — REX is ALWAYS emitted (fixed lengths; 0x40 when empty)"""
        if p66:
            self.raw(0x66)
        at = len(self.b)
        self.raw(0x40)
        self.raw(opcode)
        rr, rx, rb = self._modrm(reg, rm)
        self.b[at] = 0x40 | w << 3 | rr << 2 | rx << 1 | rb

    # the forms the lowerings use
    def mov_rm_r(self, rm, r, w=1, width=8):
        if width == 1:
            self.op(b"\x88", r, rm, 0)
        elif width == 2:
            self.op(b"\x89", r, rm, 0, p66=True)
        else:
            self.op(b"\x89", r, rm, 1 if width == 8 else 0)

    def mov_r_rm(self, r, rm, w=1):
        self.op(b"\x8b", r, rm, w)

    def mov_r_imm64(self, r, v):
        self.raw(0x48 | (r >> 3), 0xB8 | (r & 7))
        self.u64(v)

    def mov_rm_imm32(self, rm, v, w=1):
        self.op(b"\xc7", 0, rm, w)
        self.i32(v)

    def lea(self, r, rm):
        self.op(b"\x8d", r, rm)

    def alu_r_rm(self, name, r, rm, w=1):
        self.op(bytes([{"add": 0x03, "or": 0x0B, "adc": 0x13, "sbb": 0x1B, "and": 0x23, "sub": 0x2B, "xor": 0x33,
                        "cmp": 0x3B}[name]]), r, rm, w)

    def sse(self, prefix, opcode, reg, rm, w=0):
        """[prefix] REX 0F opcode modrm: an SSE form (prefix 0 = none); opcode is the bytes (0F xx)"""
        if prefix:
            self.raw(prefix)
        self.op(opcode, reg, rm, w)

    def alu_rm_imm(self, name, rm, v, w=1):
        self.op(b"\x81", {"add": 0, "or": 1, "and": 4, "sub": 5, "xor": 6, "cmp": 7}[name], rm, w)
        self.i32(v)

    def test_rm_r(self, rm, r, w=1):
        self.op(b"\x85", r, rm, w)

    def imul_r_rm(self, r, rm, w=1):
        self.op(b"\x0f\xaf", r, rm, w)

    def f7(self, digit, rm, w=1):          # 4 mul, 5 imul, 6 div, 7 idiv
        self.op(b"\xf7", digit, rm, w)

    def shift_imm(self, name, rm, n, w=1):
        self.op(b"\xc1", {"shl": 4, "shr": 5, "sar": 7}[name], rm, w)
        self.raw(n)

    def shift_cl(self, name, rm, w=1):
        self.op(b"\xd3", {"shl": 4, "shr": 5, "sar": 7}[name], rm, w)

    def setcc_al(self, cc):
        self.op(bytes([0x0F, 0x90 | CC[cc]]), 0, ("r", RAX), 0)

    def movzx(self, r, rm, width):
        self.op(b"\x0f\xb6" if width == 1 else b"\x0f\xb7", r, rm, 0)

    def movsx(self, r, rm, width):
        self.op({1: b"\x0f\xbe", 2: b"\x0f\xbf", 4: b"\x63"}[width], r, rm, 1)

    def cqo(self, w=1):
        self.raw(0x48 if w else 0x40, 0x99)

    def jcc(self, cc, label):
        self.raw(0x0F, 0x80 | CC[cc])
        self.rel32(label)

    def jmp(self, label):
        self.raw(0xE9)
        self.rel32(label)

    def call(self, label):
        self.raw(0xE8)
        self.rel32(label)

    def jmp_rm(self, rm):
        self.op(b"\xff", 4, rm, 0)

    def ret(self):
        self.raw(0xC3)

    def syscall(self):
        self.raw(0x0F, 0x05)

    def push(self, r):
        self.raw(0x40 | (r >> 3), 0x50 | (r & 7))

    def pop(self, r):
        self.raw(0x40 | (r >> 3), 0x58 | (r & 7))


# ---------------------------------------------------------------- the F and D extensions
# Operation ids (the .t1 decoder numbers them the same way): 67 flw, 68 fsw, then the groups below from 69.
# A decoded F/D instruction is (name, rd, rs1, rs2, imm) with imm = rm | rs3 << 3 (rm: the instruction's
# rounding-mode field, 7 = DYN); for the CSR forms rs1 is the register or the 5-bit immediate and imm the CSR.
FP_GROUPS = [
    # (names, shape) shape = (f sources, int source, f dest, int dest, has rm)
    (["fadd.s", "fadd.d", "fsub.s", "fsub.d", "fmul.s", "fmul.d", "fdiv.s", "fdiv.d"], (2, 0, 1, 0, 1)),
    (["fsqrt.s", "fsqrt.d", "fcvt.s.d", "fcvt.d.s"], (1, 0, 1, 0, 1)),
    (["fmadd.s", "fmadd.d", "fmsub.s", "fmsub.d", "fnmsub.s", "fnmsub.d", "fnmadd.s", "fnmadd.d"], (3, 0, 1, 0, 1)),
    (["fsgnj.s", "fsgnj.d", "fsgnjn.s", "fsgnjn.d", "fsgnjx.s", "fsgnjx.d", "fmin.s", "fmin.d", "fmax.s", "fmax.d"],
     (2, 0, 1, 0, 0)),
    (["feq.s", "feq.d", "flt.s", "flt.d", "fle.s", "fle.d"], (2, 0, 0, 1, 0)),
    (["fclass.s", "fclass.d", "fmv.x.w", "fmv.x.d"], (1, 0, 0, 1, 0)),
    (["fcvt.w.s", "fcvt.wu.s", "fcvt.l.s", "fcvt.lu.s", "fcvt.w.d", "fcvt.wu.d", "fcvt.l.d", "fcvt.lu.d"],
     (1, 0, 0, 1, 1)),
    (["fcvt.s.w", "fcvt.s.wu", "fcvt.s.l", "fcvt.s.lu", "fcvt.d.w", "fcvt.d.wu", "fcvt.d.l", "fcvt.d.lu"],
     (0, 1, 1, 0, 1)),
    (["fmv.w.x", "fmv.d.x"], (0, 1, 1, 0, 0)),
]
FP_NAMES = [n for g, _ in FP_GROUPS for n in g]
FP_ID = {n: 69 + i for i, n in enumerate(FP_NAMES)}          # 69 .. 126
FP_SHAPE = {n: sh for g, sh in FP_GROUPS for n in g}
CSR_OPS = ("csrrw", "csrrs", "csrrc", "csrrwi", "csrrsi", "csrrci")   # ids 127 .. 132, on fflags / frm / fcsr
FP_MEM = ("flw", "fsw")
FP_MOVES = ("fld", "fsd")
FSF_SLOTS = 160
FP_DATA = [("fcsr", 8), ("fsf", 8 * FSF_SLOTS), ("fma3", 8), ("fsen", 8)]       # fcsr (frm << 5 | fflags) and the soft-float frame, after the data block


def decode_fp(w, rd, f3, rs1, rs2, f7):
    """OP-FP (0x53). A reserved static rounding mode (5, 6) on an instruction with an rm field is illegal."""
    fmt = f7 & 3
    if fmt > 1:
        return None
    sd = ".d" if fmt else ".s"
    op5 = f7 >> 2
    if op5 in (0, 1, 2, 3):
        if f3 in (5, 6):
            return None
        return ("fadd fsub fmul fdiv".split()[op5] + sd, rd, rs1, rs2, f3)
    if op5 == 0x0B:
        if f3 in (5, 6) or rs2 != 0:
            return None
        return ("fsqrt" + sd, rd, rs1, 0, f3)
    if op5 == 8:                              # fcvt.s.d (fmt 0, rs2 1) / fcvt.d.s (fmt 1, rs2 0)
        if f3 in (5, 6) or rs2 != (0 if fmt else 1):
            return None
        return ("fcvt.d.s" if fmt else "fcvt.s.d", rd, rs1, 0, f3)
    if op5 == 4:
        n = {0: "fsgnj", 1: "fsgnjn", 2: "fsgnjx"}.get(f3)
        return (n + sd, rd, rs1, rs2, 0) if n else None
    if op5 == 5:
        n = {0: "fmin", 1: "fmax"}.get(f3)
        return (n + sd, rd, rs1, rs2, 0) if n else None
    if op5 == 0x14:
        n = {0: "fle", 1: "flt", 2: "feq"}.get(f3)
        return (n + sd, rd, rs1, rs2, 0) if n else None
    if op5 == 0x18:
        if f3 in (5, 6) or rs2 > 3:
            return None
        return ("fcvt.%s%s" % (("w", "wu", "l", "lu")[rs2], sd), rd, rs1, 0, f3)
    if op5 == 0x1A:
        if f3 in (5, 6) or rs2 > 3:
            return None
        return ("fcvt%s.%s" % (sd, ("w", "wu", "l", "lu")[rs2]), rd, rs1, 0, f3)
    if op5 == 0x1C:
        if f3 == 0:
            return ("fmv.x.d" if fmt else "fmv.x.w", rd, rs1, 0, 0)
        if f3 == 1:
            return ("fclass" + sd, rd, rs1, 0, 0)
        return None
    if op5 == 0x1E:
        if f3 != 0 or rs2 != 0:
            return None
        return ("fmv.d.x" if fmt else "fmv.w.x", rd, rs1, 0, 0)
    return None


# ---------------------------------------------------------------- the decoder
def decode(w):
    op = w & 0x7F
    rd, f3, rs1, rs2, f7 = (w >> 7) & 31, (w >> 12) & 7, (w >> 15) & 31, (w >> 20) & 31, w >> 25
    iI = sx(w >> 20, 12)
    iS = sx(((w >> 25) << 5) | ((w >> 7) & 31), 12)
    iB = sx(((w >> 31) & 1) << 12 | ((w >> 7) & 1) << 11 | ((w >> 25) & 63) << 5 | ((w >> 8) & 15) << 1, 13)
    iU = sx(w & 0xFFFFF000, 32)
    iJ = sx(((w >> 31) & 1) << 20 | ((w >> 12) & 255) << 12 | ((w >> 20) & 1) << 11 | ((w >> 21) & 1023) << 1, 21)
    if op == 0x37:
        return ("lui", rd, 0, 0, iU)
    if op == 0x17:
        return ("auipc", rd, 0, 0, iU)
    if op == 0x6F:
        return ("jal", rd, 0, 0, iJ)
    if op == 0x67 and f3 == 0:
        return ("jalr", rd, rs1, 0, iI)
    if op == 0x63:
        n = {0: "beq", 1: "bne", 4: "blt", 5: "bge", 6: "bltu", 7: "bgeu"}.get(f3)
        return (n, 0, rs1, rs2, iB) if n else None
    if op == 0x03:
        n = {0: "lb", 1: "lh", 2: "lw", 3: "ld", 4: "lbu", 5: "lhu", 6: "lwu"}.get(f3)
        return (n, rd, rs1, 0, iI) if n else None
    if op == 0x23:
        n = {0: "sb", 1: "sh", 2: "sw", 3: "sd"}.get(f3)
        return (n, 0, rs1, rs2, iS) if n else None
    if op == 0x13:
        sh = (w >> 20) & 63
        if f3 == 1:
            return ("slli", rd, rs1, 0, sh) if (w >> 26) == 0 else None
        if f3 == 5:
            if (w >> 26) == 0:
                return ("srli", rd, rs1, 0, sh)
            if (w >> 26) == 0x10:
                return ("srai", rd, rs1, 0, sh)
            return None
        return ({0: "addi", 2: "slti", 3: "sltiu", 4: "xori", 6: "ori", 7: "andi"}[f3], rd, rs1, 0, iI)
    if op == 0x1B:
        sh = (w >> 20) & 31
        if f3 == 0:
            return ("addiw", rd, rs1, 0, iI)
        if f3 == 1 and f7 == 0:
            return ("slliw", rd, rs1, 0, sh)
        if f3 == 5 and f7 == 0:
            return ("srliw", rd, rs1, 0, sh)
        if f3 == 5 and f7 == 0x20:
            return ("sraiw", rd, rs1, 0, sh)
        return None
    if op == 0x33:
        n = {0: {0: "add", 1: "sll", 2: "slt", 3: "sltu", 4: "xor", 5: "srl", 6: "or", 7: "and"},
             0x20: {0: "sub", 5: "sra"},
             1: {0: "mul", 1: "mulh", 2: "mulhsu", 3: "mulhu", 4: "div", 5: "divu", 6: "rem", 7: "remu"}
             }.get(f7, {}).get(f3)
        return (n, rd, rs1, rs2, 0) if n else None
    if op == 0x3B:
        n = {0: {0: "addw", 1: "sllw", 5: "srlw"}, 0x20: {0: "subw", 5: "sraw"},
             1: {0: "mulw", 4: "divw", 5: "divuw", 6: "remw", 7: "remuw"}}.get(f7, {}).get(f3)
        return (n, rd, rs1, rs2, 0) if n else None
    if op == 0x0F and f3 in (0, 1):
        return ("fence", 0, 0, 0, 0)   # fence and fence.i: no-op (stores into code are refused, so exact)
    if op == 0x07 and f3 == 3:
        return ("fld", rd, rs1, 0, iI)
    if op == 0x07 and f3 == 2:
        return ("flw", rd, rs1, 0, iI)
    if op == 0x27 and f3 == 3:
        return ("fsd", 0, rs1, rs2, iS)
    if op == 0x27 and f3 == 2:
        return ("fsw", 0, rs1, rs2, iS)
    if op == 0x53:
        return decode_fp(w, rd, f3, rs1, rs2, f7)
    if op in (0x43, 0x47, 0x4B, 0x4F) and (w >> 25) & 3 < 2 and f3 not in (5, 6):
        return ("fmadd fmsub fnmsub fnmadd".split()[(op >> 2) & 3] + (".d" if (w >> 25) & 3 else ".s"),
                rd, rs1, rs2, f3 | (w >> 27) << 3)
    if op == 0x73 and f3 in (1, 2, 3, 5, 6, 7) and (w >> 20) in (1, 2, 3):
        return (CSR_OPS[f3 - 1 if f3 < 4 else f3 - 2], rd, rs1, 0, w >> 20)
    if op == 0x73 and f3 in (1, 2, 3, 5, 6, 7) and rd == 0 and (w >> 20) == 0x100:
        # sstatus with no read: the startup's FS/VS enable. Only FS (bits 14:13) is modelled, and only by an image that
        # uses floats (it gates them); the operand rides in imm as f3 | rs1 << 3 (rs1 is the register or the 5-bit immediate)
        return ("csrs_sstatus", 0, 0, 0, f3 | rs1 << 3)
    return None


# ---------------------------------------------------------------- the translator
def parse_elf(img):
    assert img[:4] == b"\x7fELF" and img[4] == 2 and img[5] == 1 and struct.unpack_from("<H", img, 18)[0] == 243
    entry, phoff = struct.unpack_from("<QQ", img, 24)
    phentsize, phnum = struct.unpack_from("<HH", img, 54)
    segs = []
    code_end = None
    for i in range(phnum):
        t, fl, off, va, _pa, fsz, msz, _al = struct.unpack_from("<IIQQQQQQ", img, phoff + i * phentsize)
        if t == 1:
            segs.append((va, off, fsz, msz, fl))
            if fl & 1:
                code_end = va + fsz if code_end is None else max(code_end, va + fsz)
    base = min(s[0] for s in segs)
    fend = max(s[0] + s[2] for s in segs)
    mend = max(s[0] + max(s[2], s[3]) for s in segs)
    span = bytearray(fend - base)
    for va, off, fsz, _msz, _fl in segs:
        span[va - base:va - base + fsz] = img[off:off + fsz]
    # The code span is the executable segment's [p_vaddr, p_vaddr+p_filesz) and nothing else.
    # A store into it is refused at run time (self-modifying code is not translated), so the
    # PF_X segment must be the lowest-addressed PT_LOAD or its labels would alias data words.
    if code_end is not None:
        if min(segs, key=lambda s: s[0])[4] & 1 == 0:
            raise SystemExit("native: the executable PT_LOAD is not the lowest-addressed segment; this layout is not translated\n")
        code_size = code_end - base
        prot_size = code_size
    else:
        if len(segs) != 1:
            raise SystemExit("native: no executable PT_LOAD and more than one segment; this layout is not translated\n")
        code_size = fend - base       # a legacy image: translate the whole span…
        prot_size = 0                 # …but leave the code span unprotected (nothing is PF_X)
        sys.stderr.write("native: a legacy image (one writable PT_LOAD, no PF_X); the code span is left unprotected against stores\n")
    return entry, base, bytes(span), mend - base, code_size, prot_size


def elf_loads(img):
    """(p_vaddr, max(p_memsz, p_filesz)) of every PT_LOAD, in header order: the segments yantra's
    loader checks against RAM (Span::Declared), the first that does not fit refusing the run"""
    _entry, phoff = struct.unpack_from("<QQ", img, 24)
    phentsize, phnum = struct.unpack_from("<HH", img, 54)
    out = []
    for i in range(phnum):
        t, _fl, _off, va, _pa, fsz, msz, _al = struct.unpack_from("<IIQQQQQQ", img, phoff + i * phentsize)
        if t == 1:
            out.append((va, max(fsz, msz)))
    return out


def seg_tab(A, img):
    """"seg_tab", 8-aligned: u64 count, then (p_vaddr, need) u64 pairs in header order (elf_loads)"""
    loads = elf_loads(img)
    A.raw(b"\0" * (-len(A.b) % 8))
    A.label("seg_tab")
    A.raw(struct.pack("<Q", len(loads)) + b"".join(struct.pack("<QQ", va, need) for va, need in loads))


def x86_data():
    """the x86-64 data symbols in order, with their sizes (t1/authoring/mk.sh writes main.t1a's
    layout from this list, so the two cannot drift)"""
    return [("regs", 256), ("fregs", 256), ("lim_l1", 8), ("lim_l2", 8), ("lim_l4", 8),
            ("lim_l8", 8), ("lim_s1", 8), ("lim_s2", 8), ("lim_s4", 8), ("lim_s8", 8),
            ("fault_off", 8), ("fault_word", 8), ("fault_addr", 8), ("code_end", 8), ("sp_lo", 8), ("sp_span", 8),
            ("outlen", 8), ("argc", 8), ("argv", 8),
            ("envp", 8), ("ram", 8), ("top", 8), ("in_fd", 8), ("in_len", 8), ("name_ptr", 8),
            ("name_len", 8), ("args_len", 8), ("ev_slot", 8), ("rec_fd", 8), ("win_path", 8), ("win_buf", 8), ("win_st", 8),
            ("win_param", 8), ("pwv", 8 * PW_SLOTS), ("spn", SPN_SIZE), ("num", 32), ("statbuf", 144),
            ("outbuf", 65536), ("pwb", PWB_SIZE)]


def translate(img, arg0, mac=False):
    entry, base, span, extent, code_size, prot_size = parse_elf(img)
    assert base == 0x8000_0000, "milestone 1: images load at 0x8000_0000"
    nwords = code_size // 4
    words = struct.unpack_from("<%dI" % nwords, span)
    decs = [decode(w) for w in words]
    freq = [0] * 32
    for d in decs:
        if d:
            for r in d[1:4]:
                freq[r] += 1
    hot = sorted(range(1, 32), key=lambda r: (-freq[r], r))[:len(HOST)]
    M = dict(zip(hot, HOST))
    # an image that uses F/D arithmetic, flw/fsw or the float CSRs carries the F/D runtime and its data;
    # one that does not (fld / fsd are plain moves) is translated exactly as before
    fp_used = any(d and (d[0] in FP_ID or d[0] in CSR_OPS or d[0] in FP_MEM or d[0] in FP_MOVES) for d in decs)

    A = Asm(TEXT + 0x1000)   # code starts one page into the text segment (headers first)
    DATA = {}                # data symbols, laid out after the text

    def R(n):
        return ("r", M[n]) if n in M else ("abs", "regs", 8 * n)

    def get(n, s=RAX):
        if n == 0:
            A.alu_r_rm("xor", s, ("r", s), 0)
        else:
            A.mov_r_rm(s, R(n))

    def put(n, s=RAX):
        if n:
            A.mov_rm_r(R(n), s)

    def addr(rs1, imm):
        if rs1 == 0:
            A.mov_rm_imm32(("r", RAX), imm)
        elif rs1 in M:
            A.lea(RAX, ("m", M[rs1], imm))
        else:
            A.mov_r_rm(RAX, R(rs1))
            A.lea(RAX, ("m", RAX, imm))

    BR = ("beq", "bne", "blt", "bge", "bltu", "bgeu")
    MEM_L = ("lb", "lh", "lw", "ld", "lbu", "lhu", "lwu", "fld", "flw")
    MEM_S = ("sb", "sh", "sw", "sd", "fsd", "fsw")
    # ---- blocks (OPTIMISATION-PLAN #1). A LEADER is the entry, a direct branch/jal target,
    # and the word after any branch, jal, jalr, undecodable word or write to sp (x2). Each
    # block is translated twice: FAST (entered at F_pc after one check that sp lies in the
    # window, so every sp-relative load/store in it is a bare mov: exact, no such access can
    # then fault) and SAFE (G_pc, every access guarded). Within FAST sp never changes,
    # because a write to x2 ends the block.
    leader = [False] * nwords
    leader[(entry - base) // 4] = True
    for k, d in enumerate(decs):
        if d is None or d[0] in BR + ("jal", "jalr") or (d[1] == 2 and d[0] not in BR + MEM_S):
            if k + 1 < nwords:
                leader[k + 1] = True
        if d and d[0] in BR + ("jal",):
            t = k + d[4] // 4
            if 0 <= t < nwords and d[4] % 4 == 0:
                leader[t] = True
    fast_ok = 2 in M

    def tgt(t, from_fast=False):
        """a FAST block jumps past the target's sp check: sp has not changed since its own"""
        if not (base <= t < base + 4 * nwords) or t % 4:
            return "rt_bad_target"
        k = (t - base) // 4
        if fast_ok and leader[k]:
            return ("FB_%x" if from_fast else "F_%x") % t
        return "G_%x" % t

    def jmp_t(t, from_fast):
        lab = tgt(t, from_fast)
        if lab == "rt_bad_target":
            A.mov_r_imm64(RAX, t)      # rt_bad_target names the untranslated target
        A.jmp(lab)

    def jcc_t(cc, t, from_fast):
        lab = tgt(t, from_fast)
        if lab == "rt_bad_target":
            A.mov_r_imm64(RAX, t)      # rax clobbered on the fall-through is fine (scratch)
        A.jcc(cc, lab)

    cold = []
    slots = {}          # FAST only: sp offset -> mapped RV register holding the 8-octet slot

    def forget_reg(r):
        for o in [o for o, src in slots.items() if src == r]:
            del slots[o]

    def forget_range(off, width):
        for o in [o for o in slots if o < off + width and off < o + 8]:
            del slots[o]

    def lower(k, fast):
        V = "F" if fast else "G"
        pc = base + 4 * k

        def guard(width, kind):
            if kind == "l":
                A.lea(RDX, ("m", RAX, -base))
            else:
                A.mov_r_rm(RDX, ("r", RAX))
                A.alu_r_rm("sub", RDX, ("abs", "code_end", 0))
            A.alu_r_rm("cmp", RDX, ("abs", "lim_%s%d" % (kind, width), 0))
            A.jcc("ae", "C%s_%x" % (V, pc))
            cold.append((V, pc, kind, width))

        def mapped(n):
            return n in M

        d = decs[k]
        if d is None:
            A.mov_rm_imm32(("abs", "fault_off", 0), 4 * k, 0)
            A.mov_rm_imm32(("abs", "fault_word", 0), sx(words[k], 32), 0)
            A.jmp("rt_bad_insn")
            return
        op, rd, rs1, rs2, imm = d
        if op == "fence":
            return
        if op == "csrs_sstatus":
            if fp_used:                       # sstatus.FS gates every float instruction (yantra: illegal instruction)
                f3, r = imm & 7, imm >> 3
                if f3 in (1, 2, 3) and (f3 == 1 or r):
                    get(r)
                    A.shift_imm("shr", ("r", RAX), 13)
                    A.alu_rm_imm("and", ("r", RAX), 3, 0)
                    if f3 == 1:
                        A.mov_rm_r(("abs", "fsen", 0), RAX, width=4)
                    else:
                        A.mov_r_rm(RCX, ("abs", "fsen", 0), 0)
                        if f3 == 2:
                            A.alu_r_rm("or", RCX, ("r", RAX), 0)
                        else:
                            A.f7(2, ("r", RAX), 0)
                            A.alu_r_rm("and", RCX, ("r", RAX), 0)
                        A.mov_rm_r(("abs", "fsen", 0), RCX, width=4)
                elif f3 == 5:
                    A.mov_rm_imm32(("abs", "fsen", 0), 0, 0)
            return
        if fp_used and (op in FP_ID or op in CSR_OPS or op in FP_MEM or op in FP_MOVES):
            A.alu_rm_imm("cmp", ("abs", "fsen", 0), 0, 0)          # FS off: the halt yantra makes
            A.raw(0x75, 0)
            at = len(A.b)
            A.mov_rm_imm32(("abs", "fault_off", 0), 4 * k, 0)
            A.mov_rm_imm32(("abs", "fault_word", 0), sx(words[k], 32), 0)
            A.jmp("rt_illegal")
            A.b[at - 1] = len(A.b) - at
        if fast and rd and op in ("lui", "auipc", "jal", "jalr"):
            forget_reg(rd)
        if op == "lui":
            if rd:
                A.mov_rm_imm32(R(rd), imm)
            return
        if op == "auipc":
            if rd:
                if mapped(rd):
                    A.mov_r_imm64(M[rd], pc + imm)
                else:
                    A.mov_r_imm64(RAX, pc + imm)
                    put(rd)
            return
        if op == "jal":
            if rd:
                if mapped(rd):
                    A.mov_r_imm64(M[rd], pc + 4)
                else:
                    A.mov_r_imm64(RAX, pc + 4)
                    put(rd)
            # jal rd=x2 writes sp, so a FAST block must re-check the window at the target (F_, not FB_)
            jmp_t(pc + imm, fast and rd != 2)
            return
        if op == "jalr":
            get(rs1)
            A.alu_rm_imm("add", ("r", RAX), imm)
            A.alu_rm_imm("and", ("r", RAX), -2)
            if rd:
                A.mov_r_imm64(RCX, pc + 4)
                put(rd, RCX)
            A.jmp("rt_indirect")
            return
        if op in BR:
            cc = {"beq": "e", "bne": "ne", "blt": "l", "bge": "ge", "bltu": "b", "bgeu": "ae"}[op]
            if mapped(rs1):
                if rs2 == 0:
                    A.test_rm_r(("r", M[rs1]), M[rs1])
                else:
                    A.alu_r_rm("cmp", M[rs1], R(rs2))
            else:
                get(rs1)
                if rs2 == 0:
                    A.test_rm_r(("r", RAX), RAX)
                else:
                    A.alu_r_rm("cmp", RAX, R(rs2))
            jcc_t(cc, pc + imm, fast)
            return
        if op in MEM_L:
            width = {"lb": 1, "lbu": 1, "lh": 2, "lhu": 2, "lw": 4, "lwu": 4, "ld": 8, "fld": 8, "flw": 4}[op]
            if fast and rs1 == 2 and op == "ld" and rd and mapped(rd) and imm in slots:
                src = slots[imm]                   # STORE/LOAD FORWARDING: the slot's value is
                if src != rd:                      # already in a host register (exact: FAST,
                    A.mov_r_rm(M[rd], ("r", M[src]))   # so the access could not fault)
                forget_reg(rd)
                slots[imm] = src if src != rd else rd
                return
            if fast:
                forget_reg(rd)
            dst = M[rd] if (mapped(rd) and op not in ("fld", "flw")) else RAX
            if fast and rs1 == 2:
                m = ("m", M[2], imm)
            else:
                addr(rs1, imm)
                guard(width, "l")
                m = ("m", RAX, 0)
            A.label("B%s_%x" % (V, pc))
            if op == "flw":
                A.mov_r_rm(RAX, m, 0)          # a single is BOXED: the upper 32 bits all ones
                A.mov_r_imm64(RDX, 0xFFFFFFFF00000000)
                A.alu_r_rm("or", RAX, ("r", RDX))
            elif op == "lwu" or (rd == 0 and op != "fld"):
                if rd == 0:
                    A.mov_r_rm(RAX, m, 0)      # the access still happens (a fault is still a fault)
                else:
                    A.mov_r_rm(dst, m, 0)
            elif op in ("lb", "lh", "lw"):
                A.movsx(dst, m, width)
            elif op in ("lbu", "lhu"):
                A.movzx(dst, m, width)
            else:
                A.mov_r_rm(dst, m)
            if op in ("fld", "flw"):
                A.mov_rm_r(("abs", "fregs", 8 * rd), RAX)
            elif rd and dst == RAX:
                put(rd)
            if fast and rs1 == 2 and op == "ld" and rd and mapped(rd):
                slots[imm] = rd
            return
        if op in MEM_S:
            width = {"sb": 1, "sh": 2, "sw": 4, "sd": 8, "fsd": 8, "fsw": 4}[op]
            if op in ("fsd", "fsw"):
                A.mov_r_rm(RCX, ("abs", "fregs", 8 * rs2))
                val = RCX
            elif mapped(rs2):
                val = M[rs2]
            else:
                get(rs2, RCX)
                val = RCX
            if fast and rs1 == 2:
                A.mov_rm_r(("m", M[2], imm), val, width=width)
                forget_range(imm, width)
                if op == "sd" and rs2 and mapped(rs2):
                    slots[imm] = rs2
                return
            if fast:
                slots.clear()                      # a store through another base may alias the stack
            addr(rs1, imm)
            if val != RCX:
                A.mov_r_rm(RCX, ("r", val))        # the cold path expects the value in rcx
            guard(width, "s")
            A.mov_rm_r(("m", RAX, 0), RCX, width=width)
            A.label("B%s_%x" % (V, pc))
            return
        if op in FP_ID or op in CSR_OPS:
            # F / D / fflags-frm-fcsr: operands to xmm0..2 (f sources) or rax (the integer source), the
            # rounding mode to ecx, then ONE call of the runtime's helper (fp_runtime); the result is
            # xmm0 (f destination) or rax (integer destination). Helpers touch only rax rcx rdx xmm0-7.
            if op in CSR_OPS:
                kind = (CSR_OPS.index(op) % 3) + 1
                if CSR_OPS.index(op) < 3:
                    get(rs1)
                else:
                    A.mov_rm_imm32(("r", RAX), rs1)
                A.mov_rm_imm32(("r", RCX), imm | kind << 4, 0)
                A.call("fph_csr")
                if rd:
                    if fast:
                        forget_reg(rd)
                    put(rd)
                return
            nf, xs, fd, xd, rmf = FP_SHAPE[op]
            if xs:
                get(rs1)
            for i, fr in enumerate((rs1, rs2, imm >> 3)[:nf]):
                A.sse(0xF3, b"\x0f\x7e", i, ("abs", "fregs", 8 * fr), 0)       # movq xmm_i, [fregs + 8 fr]
            if rmf:
                if imm & 7 == 7:       # DYN: the helper may find frm reserved: it names this word
                    A.mov_rm_imm32(("abs", "fault_off", 0), 4 * k, 0)
                    A.mov_rm_imm32(("abs", "fault_word", 0), sx(words[k], 32), 0)
                A.mov_rm_imm32(("r", RCX), imm & 7, 0)
            A.call("fph_%d" % FP_ID[op])
            if fd:
                A.sse(0x66, b"\x0f\xd6", 0, ("abs", "fregs", 8 * rd), 0)        # movq [fregs + 8 rd], xmm0
            if xd and rd:
                if fast:
                    forget_reg(rd)
                put(rd)
            return
        if rd == 0:
            return
        if fast:
            forget_reg(rd)
        if op == "addi" and rs1 == 0:
            A.mov_rm_imm32(R(rd), imm)
            return
        if op == "addi" and mapped(rs1) and mapped(rd):
            A.lea(M[rd], ("m", M[rs1], imm))
            return
        if op in ("xori", "ori", "andi", "slli", "srli", "srai") and mapped(rd) and (rs1 == rd):
            if op in ("slli", "srli", "srai"):
                A.shift_imm({"slli": "shl", "srli": "shr", "srai": "sar"}[op], ("r", M[rd]), imm)
            else:
                A.alu_rm_imm({"xori": "xor", "ori": "or", "andi": "and"}[op], ("r", M[rd]), imm)
            return
        if op in ("addi", "xori", "ori", "andi", "slti", "sltiu", "slli", "srli", "srai"):
            get(rs1)
            if op in ("slti", "sltiu"):
                A.alu_rm_imm("cmp", ("r", RAX), imm)
                A.setcc_al("l" if op == "slti" else "b")
                A.movzx(RAX, ("r", RAX), 1)
            elif op in ("slli", "srli", "srai"):
                A.shift_imm({"slli": "shl", "srli": "shr", "srai": "sar"}[op], ("r", RAX), imm)
            else:
                A.alu_rm_imm({"addi": "add", "xori": "xor", "ori": "or", "andi": "and"}[op], ("r", RAX), imm)
            put(rd)
            return
        if op in ("addiw", "slliw", "srliw", "sraiw"):
            get(rs1)
            if op == "addiw":
                A.alu_rm_imm("add", ("r", RAX), imm, 0)
            else:
                A.shift_imm({"slliw": "shl", "srliw": "shr", "sraiw": "sar"}[op], ("r", RAX), imm, 0)
            A.movsx(RAX, ("r", RAX), 4)
            put(rd)
            return
        if op in ("add", "sub", "and", "or", "xor", "mul") and mapped(rd) and rs1 and rs2 \
                and mapped(rs1) and mapped(rs2):
            # OPTIMISATION-PLAN #2: direct two-operand forms on the host registers
            h, a, b = M[rd], M[rs1], M[rs2]
            comm = op != "sub"
            if rd == rs1 or (comm and rd == rs2):
                other = b if rd == rs1 else a
                if op == "mul":
                    A.imul_r_rm(h, ("r", other))
                else:
                    A.alu_r_rm(op, h, ("r", other))
                return
            if rd != rs2:
                A.mov_r_rm(h, ("r", a))
                if op == "mul":
                    A.imul_r_rm(h, ("r", b))
                else:
                    A.alu_r_rm(op, h, ("r", b))
                return
            # sub with rd == rs2 != rs1: through rax
        if op in ("add", "sub", "and", "or", "xor", "slt", "sltu", "mul"):
            get(rs1)
            if op in ("slt", "sltu"):
                A.alu_r_rm("cmp", RAX, R(rs2) if rs2 else ("r", RCX)) if rs2 else None
                if not rs2:
                    A.alu_rm_imm("cmp", ("r", RAX), 0)
                A.setcc_al("l" if op == "slt" else "b")
                A.movzx(RAX, ("r", RAX), 1)
            elif op == "mul":
                if rs2:
                    A.imul_r_rm(RAX, R(rs2))
                else:
                    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
            else:
                if rs2:
                    A.alu_r_rm(op, RAX, R(rs2))
                elif op == "and":
                    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
            put(rd)
            return
        if op in ("sll", "srl", "sra", "sllw", "srlw", "sraw"):
            get(rs2, RCX)
            get(rs1)
            w64 = not op.endswith("w")
            A.shift_cl({"sll": "shl", "srl": "shr", "sra": "sar"}[op[:3]], ("r", RAX), 1 if w64 else 0)
            if not w64:
                A.movsx(RAX, ("r", RAX), 4)
            put(rd)
            return
        if op in ("addw", "subw", "mulw"):
            get(rs2, RCX)
            get(rs1)
            if op == "mulw":
                A.imul_r_rm(RAX, ("r", RCX), 0)
            else:
                A.alu_r_rm(op[:3], RAX, ("r", RCX), 0)
            A.movsx(RAX, ("r", RAX), 4)
            put(rd)
            return
        if op in ("mulh", "mulhu"):
            get(rs2, RCX)
            get(rs1)
            A.f7(5 if op == "mulh" else 4, ("r", RCX))
            put(rd, RDX)
            return
        if op == "mulhsu":
            # (i128)a * (u128)b >> 64 = mulhu(a, b) - (a < 0 ? b : 0)
            get(rs2, RCX)
            get(rs1)
            A.f7(4, ("r", RCX))
            get(rs1)
            A.shift_imm("sar", ("r", RAX), 63)
            A.alu_r_rm("and", RAX, ("r", RCX))
            A.alu_r_rm("sub", RDX, ("r", RAX))
            put(rd, RDX)
            return
        if op in ("div", "rem", "divu", "remu", "divw", "remw", "divuw", "remuw"):
            # RV never traps: x/0 = all ones, x%0 = x, MIN/-1 = MIN, MIN%-1 = 0
            get(rs2, RCX)
            get(rs1)
            w = 0 if op.endswith("w") else 1
            L = "D%s_%x" % (V, pc)
            A.test_rm_r(("r", RCX), RCX, w)
            A.jcc("e", L + "z")
            signed = op in ("div", "rem", "divw", "remw")
            if signed:
                A.alu_rm_imm("cmp", ("r", RCX), -1, w)
                A.jcc("ne", L + "d")
                if w:
                    A.mov_r_imm64(RDX, 1 << 63)
                    A.alu_r_rm("cmp", RAX, ("r", RDX))
                else:
                    A.alu_rm_imm("cmp", ("r", RAX), -(1 << 31), 0)
                A.jcc("ne", L + "d")
                if op.startswith("rem"):
                    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
                elif not w:
                    A.movsx(RAX, ("r", RAX), 4)
                A.jmp(L + "e")
            A.label(L + "d")
            if signed:
                A.cqo(w)
                A.f7(7, ("r", RCX), w)
            else:
                A.alu_r_rm("xor", RDX, ("r", RDX), 0)
                A.f7(6, ("r", RCX), w)
            if op.startswith("rem"):
                A.mov_r_rm(RAX, ("r", RDX), w)
            if not w:
                A.movsx(RAX, ("r", RAX), 4)
            A.jmp(L + "e")
            A.label(L + "z")
            if op.startswith("div"):
                A.mov_rm_imm32(("r", RAX), -1)
            elif not w:
                A.movsx(RAX, ("r", RAX), 4)
            A.label(L + "e")
            put(rd)
            return
        raise SystemExit("untranslated %s at %#x" % (op, pc))

    # ---------------- the FAST region: blocks in address order, each opening with the sp check
    if fast_ok:
        for k in range(nwords):
            pc = base + 4 * k
            if leader[k]:
                slots.clear()
                A.label("F_%x" % pc)
                # (sp - sp_lo) <u sp_span, else this block runs SAFE
                A.mov_r_rm(RAX, ("r", M[2]))
                A.alu_r_rm("sub", RAX, ("abs", "sp_lo", 0))
                A.alu_r_rm("cmp", RAX, ("abs", "sp_span", 0))
                A.jcc("ae", "G_%x" % pc)
                A.label("FB_%x" % pc)
            lower(k, True)
        A.mov_r_imm64(RAX, base + 4 * nwords)   # fell off the end: name the pc past the last word
        A.jmp("rt_bad_target")
    # ---------------- the SAFE region: every word, every access guarded
    for k in range(nwords):
        A.label("G_%x" % (base + 4 * k))
        lower(k, False)
    A.mov_r_imm64(RAX, base + 4 * nwords)
    A.jmp("rt_bad_target")
    # ---------------- cold paths: a guard that failed names its site, then the runtime decides
    for V, pc, kind, width in cold:
        A.label("C%s_%x" % (V, pc))
        A.mov_rm_imm32(("abs", "fault_off", 0), pc - base, 0)
        A.mov_rm_imm32(("r", RDX), width, 0)
        if kind == "s":
            A.call("rt_store_slow")      # returns only for the UART
            A.jmp("B%s_%x" % (V, pc))
        else:
            A.jmp("rt_load_slow")
    runtime(A, M, base, entry, extent, len(span), code_size, prot_size, mac)
    if fp_used:
        runtime_fp(A, os.environ.get("ANUVADA_SOFT_ALL") == "1", mac, os.environ.get("ANUVADA_NO_FMA3") == "1")
    # ---------------- the jump table and the guest image
    A.raw(b"\0" * (-len(A.b) % 8))
    A.label("jt")
    jt_at = len(A.b)
    A.raw(b"\0" * (8 * nwords))
    A.label("image")
    A.raw(span)
    A.label("arg0")
    A.raw(arg0.encode() + b"\0")
    seg_tab(A, img)
    A.label("text_end")
    # ---------------- data (bss), page aligned after the text
    dpos = (A.here() + 0xFFF) & ~0xFFF
    for name, size in x86_data():
        DATA[name] = dpos
        dpos += size
    if fp_used:
        for name, size in FP_DATA:
            DATA[name] = dpos
            dpos += size
    data_end = dpos
    if mac == "win":
        # the import tables take the data's first page; the symbols follow on the next
        imp, labs = pe_imports(DATA["regs"] - TEXT)
        assert len(imp) <= 0x1000
        for k in DATA:
            DATA[k] += 0x1000
        DATA.update({k: DATA["regs"] - 0x1000 + v for k, v in labs.items()})
        data_end += 0x1000
    A.resolve(DATA)
    for k in range(nwords):
        struct.pack_into("<Q", A.b, jt_at + 8 * k, A.labels[tgt(base + 4 * k)])
    if mac == "win":
        # the IAT directory holds an RVA: the data's RVA plus the table's offset in it
        return pe(A, imp, data_end - (DATA["regs"] - 0x1000), TEXT, 0x8664,
                  DATA["regs"] - 0x1000 - TEXT + labs["pe_iat"],
                  labs["pe_iat_end"] - labs["pe_iat"], dynamic=False), M
    if mac:
        return macho_x86(A, data_end - DATA["regs"]), M
    return elf(A, data_end), M


MACSYS = {0: 3, 1: 4, 2: 5, 5: 339, 9: 197, 60: 1}   # Linux -> Darwin syscall numbers


def x86_sys(A, n, mac):
    """Linux syscall n: the syscall itself, Darwin's number for it, or (mac="win") a CALL to its shim"""
    if mac == "win":
        A.call("w_sys%d" % n)
        return
    A.mov_rm_imm32(("r", RAX), 0x2000000 + MACSYS[n] if mac else n)
    A.syscall()


def runtime_win(A):
    """x86_64-windows: the shims behind runtime()'s syscalls. Each is CALLed with Linux's
    contract (rdi rsi rdx in, rax out, rcx and r11 clobbered, nothing else): it saves rdx
    and r8..r11 (Win64's other caller-saved registers; rsi and rdi are callee-saved there),
    aligns rsp under a frame pointer, leaves the 32-octet shadow space, and calls kernel32
    through the import table. Answers as Linux: a count or a handle, -1 on failure."""
    SAVED = (RDX, R8, R9, R10, R11, RBP)
    R12, R13, R14, R15 = 12, 13, 14, 15

    def enter(label):
        A.label(label)
        for r in SAVED:
            A.push(r)
        A.mov_r_rm(RBP, ("r", RSP))
        A.alu_rm_imm("and", ("r", RSP), -16)
        A.alu_rm_imm("sub", ("r", RSP), 80)
        A.op(b"\x0f\xae", 3, ("m", RSP, 64), 0)      # stmxcsr: Win64 calls may clear its sticky flags

    def leave():
        A.op(b"\x0f\xae", 2, ("m", RSP, 64), 0)      # ldmxcsr: the guest's flags and rounding, back
        A.mov_r_rm(RSP, ("r", RBP))
        for r in reversed(SAVED):
            A.pop(r)
        A.ret()

    def imp(name):
        A.mov_r_rm(RAX, ("abs", "iat_" + name, 0))
        A.raw(0x40, 0xFF, 0xD0)                       # call rax

    def bool_count(fail, done):
        """after a BOOL call: rax = the DWORD at [rsp+48], or -1"""
        A.test_rm_r(("r", RAX), RAX, 0)
        A.jcc("e", fail)
        A.mov_r_rm(RAX, ("m", RSP, 48), 0)
        A.jmp(done)
        A.label(fail)
        A.mov_rm_imm32(("r", RAX), -1)
        A.label(done)

    # ---- write(rdi = fd 1 or 2, rsi = buf, rdx = n)
    enter("w_sys1")
    A.mov_rm_r(("m", RSP, 56), RDX)
    A.mov_rm_imm32(("r", RCX), -11)                    # STD_OUTPUT_HANDLE
    A.alu_rm_imm("cmp", ("r", RDI), 2)
    A.jcc("ne", "w_wr1")
    A.mov_rm_imm32(("r", RCX), -12)                    # STD_ERROR_HANDLE
    A.label("w_wr1")
    imp("GetStdHandle")
    A.mov_r_rm(RCX, ("r", RAX))
    A.mov_r_rm(RDX, ("r", RSI))
    A.mov_r_rm(R8, ("m", RSP, 56))
    A.lea(R9, ("m", RSP, 48))
    A.mov_rm_imm32(("m", RSP, 48), 0)
    A.mov_rm_imm32(("m", RSP, 32), 0)                  # lpOverlapped
    imp("WriteFile")
    bool_count("w_wr_f", "w_wr_d")
    leave()
    # ---- read(rdi = handle, rsi = buf, rdx = n), n capped at 2^30 (the caller loops)
    enter("w_sys0")
    A.mov_r_rm(RCX, ("r", RDI))
    A.mov_r_rm(R8, ("r", RDX))
    A.mov_r_imm64(RAX, 0x4000_0000)
    A.alu_r_rm("cmp", R8, ("r", RAX))
    A.jcc("be", "w_rd1")
    A.mov_r_rm(R8, ("r", RAX))
    A.label("w_rd1")
    A.mov_r_rm(RDX, ("r", RSI))
    A.lea(R9, ("m", RSP, 48))
    A.mov_rm_imm32(("m", RSP, 48), 0)
    A.mov_rm_imm32(("m", RSP, 32), 0)
    imp("ReadFile")
    bool_count("w_rd_f", "w_rd_d")
    leave()
    # ---- the stdin handle: GetStdHandle(STD_INPUT_HANDLE)
    enter("w_stdin")
    A.mov_rm_imm32(("r", RCX), -10)
    imp("GetStdHandle")
    leave()
    # ---- exit(rdi = code)
    enter("w_sys60")
    A.mov_r_rm(RCX, ("r", RDI))
    imp("ExitProcess")
    # ---- open(rdi = a UTF-8 path): MultiByteToWideChar into a fresh 128 KiB buffer, CreateFileW
    enter("w_sys2")
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.mov_rm_imm32(("r", RDX), 0x20000)
    A.mov_rm_imm32(("r", R8), 0x3000)                  # MEM_COMMIT | MEM_RESERVE
    A.mov_rm_imm32(("r", R9), 4)                       # PAGE_READWRITE
    imp("VirtualAlloc")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "w_op_f")
    A.mov_rm_r(("m", RSP, 56), RAX)
    A.mov_rm_imm32(("r", RCX), 65001)                  # CP_UTF8
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.mov_r_rm(R8, ("r", RDI))
    A.mov_rm_imm32(("r", R9), -1)                      # zero-ended
    A.mov_rm_r(("m", RSP, 32), RAX)
    A.mov_rm_imm32(("m", RSP, 40), 0x8000)
    imp("MultiByteToWideChar")
    A.test_rm_r(("r", RAX), RAX, 0)
    A.jcc("e", "w_op_f")
    A.mov_r_rm(RCX, ("m", RSP, 56))
    A.mov_rm_imm32(("r", RDX), -0x8000_0000, 0)        # GENERIC_READ
    A.mov_rm_imm32(("r", R8), 1)                       # FILE_SHARE_READ
    A.alu_r_rm("xor", R9, ("r", R9), 0)
    A.mov_rm_imm32(("m", RSP, 32), 3)                  # OPEN_EXISTING
    A.mov_rm_imm32(("m", RSP, 40), 0x80)               # FILE_ATTRIBUTE_NORMAL
    A.mov_rm_imm32(("m", RSP, 48), 0)
    imp("CreateFileW")                                 # INVALID_HANDLE_VALUE is -1 already
    A.jmp("w_op_d")
    A.label("w_op_f")
    A.mov_rm_imm32(("r", RAX), -1)
    A.label("w_op_d")
    leave()
    # ---- size(rdi = handle): GetFileSizeEx
    enter("w_size")
    A.mov_r_rm(RCX, ("r", RDI))
    A.lea(RDX, ("m", RSP, 48))
    A.mov_rm_imm32(("m", RSP, 48), 0)
    imp("GetFileSizeEx")
    A.test_rm_r(("r", RAX), RAX, 0)
    A.jcc("e", "w_sz_f")
    A.mov_r_rm(RAX, ("m", RSP, 48))
    A.jmp("w_sz_d")
    A.label("w_sz_f")
    A.mov_rm_imm32(("r", RAX), -1)
    A.label("w_sz_d")
    leave()
    # ---- alloc(rdi = the address wanted, rsi = size): VirtualAlloc there, or 0
    enter("w_alloc")
    A.mov_r_rm(RCX, ("r", RDI))
    A.mov_r_rm(RDX, ("r", RSI))
    A.mov_rm_imm32(("r", R8), 0x3000)
    A.mov_rm_imm32(("r", R9), 4)
    imp("VirtualAlloc")
    leave()
    # ---- init: argc, argv, envp in UTF-8, as runtime_win_a64's w_init (the same rules).
    # rbx r12..r15 rsi rdi are Win64's callee-saved registers and carry the state; nothing
    # is live yet and this never returns to Windows. rax = 1, or 0 if there is no memory.
    enter("w_init")
    imp("GetEnvironmentStringsW")
    A.mov_r_rm(RBX, ("r", RAX))
    A.mov_r_rm(RCX, ("r", RAX))
    A.label("w_in_e1")
    A.movzx(RDX, ("m", RCX, 0), 2)
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "w_in_e3")
    A.label("w_in_e2")
    A.movzx(RDX, ("m", RCX, 0), 2)
    A.alu_rm_imm("add", ("r", RCX), 2)
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("ne", "w_in_e2")
    A.jmp("w_in_e1")
    A.label("w_in_e3")
    A.alu_rm_imm("add", ("r", RCX), 2)
    A.alu_r_rm("sub", RCX, ("r", RBX))
    A.shift_imm("shr", ("r", RCX), 1)
    A.mov_r_rm(R12, ("r", RCX))                        # n: units, the final zero included
    imp("GetCommandLineW")
    A.mov_r_rm(R13, ("r", RAX))
    A.mov_r_rm(RCX, ("r", RAX))
    A.label("w_in_c1")
    A.movzx(RDX, ("m", RCX, 0), 2)
    A.alu_rm_imm("add", ("r", RCX), 2)
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("ne", "w_in_c1")
    A.alu_r_rm("sub", RCX, ("r", R13))
    A.shift_imm("shr", ("r", RCX), 1)
    A.mov_r_rm(R14, ("r", RCX))                        # m: units, the zero included
    A.mov_r_rm(RDX, ("r", R12))
    A.mov_rm_imm32(("r", RAX), 11)
    A.imul_r_rm(RDX, ("r", RAX))
    A.mov_r_rm(RAX, ("r", R14))
    A.mov_rm_imm32(("r", RCX), 14)
    A.imul_r_rm(RAX, ("r", RCX))
    A.alu_r_rm("add", RDX, ("r", RAX))
    A.alu_rm_imm("add", ("r", RDX), 72 + 0xFFF)
    A.alu_rm_imm("and", ("r", RDX), -0x1000)           # 11n + 14m + 72, in pages
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.mov_rm_imm32(("r", R8), 0x3000)
    A.mov_rm_imm32(("r", R9), 4)
    imp("VirtualAlloc")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "w_in_d")
    A.mov_r_rm(R15, ("r", RAX))                        # the environment, UTF-8
    A.mov_rm_imm32(("r", RCX), 65001)
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.mov_r_rm(R8, ("r", RBX))
    A.mov_r_rm(R9, ("r", R12))
    A.mov_rm_r(("m", RSP, 32), R15)
    A.mov_r_rm(RAX, ("r", R12))
    A.mov_rm_imm32(("r", RDI), 3)
    A.imul_r_rm(RAX, ("r", RDI))
    A.mov_r_rm(RSI, ("r", RAX))                        # 3n
    A.mov_rm_r(("m", RSP, 40), RAX)
    A.mov_rm_imm32(("m", RSP, 48), 0)
    A.mov_rm_imm32(("m", RSP, 56), 0)
    imp("WideCharToMultiByte")
    A.lea(RSI, ("m", RSI, 7))
    A.alu_r_rm("add", RSI, ("r", R15))
    A.alu_rm_imm("and", ("r", RSI), -8)                # envp[]
    A.mov_rm_r(("abs", "envp", 0), RSI)
    A.mov_r_rm(RCX, ("r", R15))
    A.label("w_in_v1")
    A.movzx(RDX, ("m", RCX, 0), 1)
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "w_in_v3")
    A.mov_rm_r(("m", RSI, 0), RCX)
    A.alu_rm_imm("add", ("r", RSI), 8)
    A.label("w_in_v2")
    A.movzx(RDX, ("m", RCX, 0), 1)
    A.alu_rm_imm("add", ("r", RCX), 1)
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("ne", "w_in_v2")
    A.jmp("w_in_v1")
    A.label("w_in_v3")
    A.mov_rm_imm32(("m", RSI, 0), 0)
    A.lea(R15, ("m", RSI, 8))                          # the command line, UTF-8
    A.mov_rm_imm32(("r", RCX), 65001)
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.mov_r_rm(R8, ("r", R13))
    A.mov_r_rm(R9, ("r", R14))
    A.mov_rm_r(("m", RSP, 32), R15)
    A.mov_r_rm(RAX, ("r", R14))
    A.mov_rm_imm32(("r", RDI), 3)
    A.imul_r_rm(RAX, ("r", RDI))
    A.mov_r_rm(RBX, ("r", RAX))                        # 3m
    A.mov_rm_r(("m", RSP, 40), RAX)
    A.mov_rm_imm32(("m", RSP, 48), 0)
    A.mov_rm_imm32(("m", RSP, 56), 0)
    imp("WideCharToMultiByte")
    A.lea(RDI, ("m", R15, 0))
    A.alu_r_rm("add", RDI, ("r", RBX))                 # out: the split strings
    A.lea(R12, ("m", RDI, 15))
    A.alu_r_rm("add", R12, ("r", RBX))
    A.alu_rm_imm("and", ("r", R12), -8)                # argv[]
    A.mov_rm_r(("abs", "argv", 0), R12)
    A.alu_r_rm("xor", R13, ("r", R13), 0)              # argc
    # r15 in, rdi out, r12 the argv cursor, r14 inside quotes, rbx backslashes, rdx the octet
    A.mov_rm_r(("m", R12, 0), RDI)
    A.alu_rm_imm("add", ("r", R12), 8)
    A.alu_rm_imm("add", ("r", R13), 1)
    A.movzx(RDX, ("m", R15, 0), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 0x22)
    A.jcc("ne", "w_a0b")
    A.alu_rm_imm("add", ("r", R15), 1)
    A.label("w_a0a")
    A.movzx(RDX, ("m", R15, 0), 1)
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "w_a0e")
    A.alu_rm_imm("add", ("r", R15), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 0x22)
    A.jcc("e", "w_a0e")
    A.mov_rm_r(("m", RDI, 0), RDX, 0, 1)
    A.alu_rm_imm("add", ("r", RDI), 1)
    A.jmp("w_a0a")
    A.label("w_a0b")
    A.movzx(RDX, ("m", R15, 0), 1)
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "w_a0e")
    A.alu_rm_imm("cmp", ("r", RDX), 0x20)
    A.jcc("e", "w_a0e")
    A.alu_rm_imm("cmp", ("r", RDX), 0x09)
    A.jcc("e", "w_a0e")
    A.mov_rm_r(("m", RDI, 0), RDX, 0, 1)
    A.alu_rm_imm("add", ("r", RDI), 1)
    A.alu_rm_imm("add", ("r", R15), 1)
    A.jmp("w_a0b")
    A.label("w_a0e")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.mov_rm_r(("m", RDI, 0), RAX, 0, 1)
    A.alu_rm_imm("add", ("r", RDI), 1)
    A.label("w_p0")                                    # between arguments
    A.movzx(RDX, ("m", R15, 0), 1)
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "w_pd")
    A.alu_rm_imm("cmp", ("r", RDX), 0x20)
    A.jcc("e", "w_p0s")
    A.alu_rm_imm("cmp", ("r", RDX), 0x09)
    A.jcc("ne", "w_pa")
    A.label("w_p0s")
    A.alu_rm_imm("add", ("r", R15), 1)
    A.jmp("w_p0")
    A.label("w_pa")                                    # an argument starts
    A.mov_rm_r(("m", R12, 0), RDI)
    A.alu_rm_imm("add", ("r", R12), 8)
    A.alu_rm_imm("add", ("r", R13), 1)
    A.alu_r_rm("xor", R14, ("r", R14), 0)
    A.label("w_p1")
    A.alu_r_rm("xor", RBX, ("r", RBX), 0)
    A.label("w_p1b")
    A.movzx(RDX, ("m", R15, 0), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 0x5C)
    A.jcc("ne", "w_p1c")
    A.alu_rm_imm("add", ("r", RBX), 1)
    A.alu_rm_imm("add", ("r", R15), 1)
    A.jmp("w_p1b")
    A.label("w_p1c")
    A.alu_rm_imm("cmp", ("r", RDX), 0x22)
    A.jcc("ne", "w_p1n")
    A.mov_r_rm(RCX, ("r", RBX))
    A.shift_imm("shr", ("r", RBX), 1)
    A.call("w_bs")
    A.alu_rm_imm("and", ("r", RCX), 1)                 # odd: n backslashes and a literal quote
    A.jcc("e", "w_p1e")
    A.mov_rm_r(("m", RDI, 0), RDX, 0, 1)
    A.alu_rm_imm("add", ("r", RDI), 1)
    A.alu_rm_imm("add", ("r", R15), 1)
    A.jmp("w_p1")
    A.label("w_p1e")
    A.test_rm_r(("r", R14), R14)
    A.jcc("e", "w_p1t")
    A.movzx(RCX, ("m", R15, 1), 1)
    A.alu_rm_imm("cmp", ("r", RCX), 0x22)
    A.jcc("ne", "w_p1t")
    A.mov_rm_r(("m", RDI, 0), RDX, 0, 1)               # "" inside quotes: one quote, still inside
    A.alu_rm_imm("add", ("r", RDI), 1)
    A.alu_rm_imm("add", ("r", R15), 2)
    A.jmp("w_p1")
    A.label("w_p1t")
    A.alu_rm_imm("xor", ("r", R14), 1)
    A.alu_rm_imm("add", ("r", R15), 1)
    A.jmp("w_p1")
    A.label("w_p1n")
    A.call("w_bs")
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "w_pe")
    A.test_rm_r(("r", R14), R14)
    A.jcc("ne", "w_p1k")
    A.alu_rm_imm("cmp", ("r", RDX), 0x20)
    A.jcc("e", "w_pe")
    A.alu_rm_imm("cmp", ("r", RDX), 0x09)
    A.jcc("e", "w_pe")
    A.label("w_p1k")
    A.mov_rm_r(("m", RDI, 0), RDX, 0, 1)
    A.alu_rm_imm("add", ("r", RDI), 1)
    A.alu_rm_imm("add", ("r", R15), 1)
    A.jmp("w_p1")
    A.label("w_pe")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.mov_rm_r(("m", RDI, 0), RAX, 0, 1)
    A.alu_rm_imm("add", ("r", RDI), 1)
    A.jmp("w_p0")
    A.label("w_pd")
    A.mov_rm_imm32(("m", R12, 0), 0)
    A.mov_rm_r(("abs", "argc", 0), R13)
    A.mov_rm_imm32(("r", RAX), 1)
    A.label("w_in_d")
    leave()
    # rbx backslashes to rdi; leaves rcx and rdx alone (rax is scratch)
    A.label("w_bs")
    A.test_rm_r(("r", RBX), RBX)
    A.jcc("e", "w_bs2")
    A.mov_rm_imm32(("r", RAX), 0x5C)
    A.mov_rm_r(("m", RDI, 0), RAX, 0, 1)
    A.alu_rm_imm("add", ("r", RDI), 1)
    A.alu_rm_imm("sub", ("r", RBX), 1)
    A.jmp("w_bs")
    A.label("w_bs2")
    A.ret()


# ---- the runtime window, 0x1000_2000..0x1000_207F (runtime-completeness-design.md §3-§5).
# Each store's value is a guest run pointer (its length at pointer - 8). PATH and BUFFER
# are remembered; an operation's store carries the STATUS run, answers into its word 0 and
# forgets PATH and BUFFER (yantra's GO). Unknown window addresses halt BadAccess as before.
WIN_PATH, WIN_BUFFER, WIN_STDIN, WIN_ENV = 0x00, 0x08, 0x68, 0x70     # offsets from 0x1000_2000
WINDOW = 0x1000_2000
# status words, numbered down from u64::MAX (patra.rs Status::word, then the design's §5)
ST_BAD_ADDRESS, ST_NOT_FOUND, ST_TOO_LARGE, ST_NOT_GRANTED = -2, -5, -6, -8


def runtime_window(A, base, prot_size, win, mac):
    """x86-64: rax = addr, rcx = value; returns with only rax rcx rdx changed. A run is
    readable inside [base, base + top) and writable inside [base + prot_size, base + ram)."""
    def sys_read():
        if win:
            A.call("w_sys0")
        else:
            A.mov_rm_imm32(("r", RAX), 0x2000000 + MACSYS[0] if mac else 0)
            A.syscall()

    SAVE = (RBX, RSI, RDI, R8, R9, R10, R11)
    A.label("rt_st_win")
    A.lea(RDX, ("m", RAX, -WINDOW))
    A.alu_rm_imm("cmp", ("r", RDX), 0x80)
    A.jcc("ae", "rt_beyond")
    for off, lab in ((WIN_PATH, "rt_w_path"), (WIN_BUFFER, "rt_w_buf"), (WIN_STDIN, "rt_w_stdin"),
                     (WIN_ENV, "rt_w_env"), (WIN_PARAM, "rt_w_param"), (WIN_SPAWN, "rt_w_spawn")) + \
            tuple((o, "rt_w_files") for o in WIN_FILE_OPS):
        A.alu_rm_imm("cmp", ("r", RDX), off)
        A.jcc("e", lab)
    A.jmp("rt_beyond")
    A.label("rt_w_path")
    A.mov_rm_r(("abs", "win_path", 0), RCX)
    A.ret()
    A.label("rt_w_param")
    A.mov_rm_r(("abs", "win_param", 0), RCX)
    A.ret()
    # ---- the file operations (files_engine): rdx = the store's offset, the answer in "st"
    P = PwX86(A, "win" if win else ("mac" if mac else "linux"))
    A.label("rt_w_files")
    for r in SAVE:
        A.push(r)
    A.mov_rm_r(("abs", "win_st", 0), RCX)
    A.mov_rm_r(P.slot("op"), RDX)
    A.call("pw_main")
    A.mov_r_rm(RAX, P.slot("st"))
    A.mov_r_rm(RCX, P.slot("keep"))
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "rt_w_keep")
    A.jmp("rt_w_answer")
    A.label("rt_w_buf")
    A.mov_rm_r(("abs", "win_buf", 0), RCX)
    A.ret()
    # ---- read stdin: fill BUFFER from fd 0 (Windows: STD_INPUT_HANDLE); the count, 0 = end
    A.label("rt_w_stdin")
    for r in SAVE:
        A.push(r)
    A.mov_rm_r(("abs", "win_st", 0), RCX)
    A.call("rt_flush")                         # what was printed is seen before the read blocks
    A.mov_r_rm(RSI, ("abs", "win_buf", 0))
    A.call("rt_w_hdr")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "rt_w_bad")
    A.call("rt_w_wr")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "rt_w_bad")
    A.mov_r_rm(R8, ("r", RSI))
    A.mov_r_rm(R9, ("r", RAX))
    A.alu_r_rm("xor", R10, ("r", R10), 0)
    if win:
        A.call("w_stdin")
        A.mov_r_rm(RBX, ("r", RAX))
    A.label("rt_w_si1")
    A.alu_r_rm("cmp", R10, ("r", R9))
    A.jcc("ae", "rt_w_si2")
    if win:
        A.mov_r_rm(RDI, ("r", RBX))
    else:
        A.alu_r_rm("xor", RDI, ("r", RDI), 0)
    A.mov_r_rm(RSI, ("r", R8))
    A.alu_r_rm("add", RSI, ("r", R10))
    A.mov_r_rm(RDX, ("r", R9))
    A.alu_r_rm("sub", RDX, ("r", R10))
    sys_read()
    if mac:
        A.jcc("b", "rt_w_si2")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("le", "rt_w_si2")                    # end of input, or an error (a broken pipe) as its end
    A.alu_r_rm("add", R10, ("r", RAX))
    A.jmp("rt_w_si1")
    A.label("rt_w_si2")
    A.mov_r_rm(RAX, ("r", R10))
    A.jmp("rt_w_answer")
    # ---- an environment variable: PATH names it, BUFFER receives the value
    A.label("rt_w_env")
    for r in SAVE:
        A.push(r)
    A.mov_rm_r(("abs", "win_st", 0), RCX)
    A.lea(RDI, ("abs", "env_grant", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_w_ngr")
    A.movzx(RDX, ("m", RAX, 0), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 0x31)
    A.jcc("ne", "rt_w_ngr")
    A.movzx(RDX, ("m", RAX, 1), 1)
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("ne", "rt_w_ngr")                    # granted only by exactly "1"
    A.mov_r_rm(RSI, ("abs", "win_path", 0))
    A.call("rt_w_hdr")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "rt_w_bad")
    A.call("rt_w_rd")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "rt_w_bad")
    A.mov_r_rm(R9, ("r", RSI))                 # the name
    A.mov_r_rm(R10, ("r", RAX))                # its length
    A.mov_r_rm(RSI, ("abs", "win_buf", 0))
    A.call("rt_w_hdr")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "rt_w_bad")
    A.mov_r_rm(R11, ("r", RAX))                # the buffer's capacity
    # a name that is empty or holds "=" or a zero octet names no variable
    A.test_rm_r(("r", R10), R10)
    A.jcc("e", "rt_w_nf")
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.label("rt_w_ev1")
    A.alu_r_rm("cmp", RCX, ("r", R10))
    A.jcc("ae", "rt_w_ev2")
    A.lea(RDX, ("m", R9, 0))
    A.alu_r_rm("add", RDX, ("r", RCX))
    A.movzx(RDX, ("m", RDX, 0), 1)
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "rt_w_nf")
    A.alu_rm_imm("cmp", ("r", RDX), 0x3D)
    A.jcc("e", "rt_w_nf")
    A.alu_rm_imm("add", ("r", RCX), 1)
    A.jmp("rt_w_ev1")
    A.label("rt_w_ev2")
    A.mov_r_rm(R8, ("abs", "envp", 0))         # the first "NAME=" entry, matched exactly
    A.label("rt_w_ee1")
    A.mov_r_rm(RDI, ("m", R8, 0))
    A.test_rm_r(("r", RDI), RDI)
    A.jcc("e", "rt_w_nf")
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.label("rt_w_ee2")
    A.alu_r_rm("cmp", RCX, ("r", R10))
    A.jcc("ae", "rt_w_ee3")
    A.lea(RAX, ("m", R9, 0))
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.movzx(RAX, ("m", RAX, 0), 1)
    A.lea(RDX, ("m", RDI, 0))
    A.alu_r_rm("add", RDX, ("r", RCX))
    A.movzx(RDX, ("m", RDX, 0), 1)
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    A.jcc("ne", "rt_w_ee4")
    A.alu_rm_imm("add", ("r", RCX), 1)
    A.jmp("rt_w_ee2")
    A.label("rt_w_ee3")
    A.alu_r_rm("add", RDI, ("r", R10))
    A.movzx(RDX, ("m", RDI, 0), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 0x3D)
    A.jcc("e", "rt_w_ee5")
    A.label("rt_w_ee4")
    A.alu_rm_imm("add", ("r", R8), 8)
    A.jmp("rt_w_ee1")
    A.label("rt_w_ee5")
    A.lea(R8, ("m", RDI, 1))                   # the value
    A.mov_r_rm(RSI, ("r", R8))
    A.call("rt_strlen")
    A.alu_r_rm("cmp", RAX, ("r", R11))
    A.jcc("a", "rt_w_big")                     # the buffer is left untouched
    A.mov_r_rm(RSI, ("abs", "win_buf", 0))
    A.call("rt_w_wr")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "rt_w_bad")
    A.mov_r_rm(RDI, ("r", RSI))
    A.mov_r_rm(RSI, ("r", R8))
    A.mov_r_rm(RCX, ("r", RAX))
    A.raw(0xF3, 0xA4)                          # rep movsb
    A.jmp("rt_w_answer")
    A.label("rt_w_ngr")
    A.mov_rm_imm32(("r", RAX), ST_NOT_GRANTED)
    A.jmp("rt_w_answer")
    A.label("rt_w_nf")
    A.mov_rm_imm32(("r", RAX), ST_NOT_FOUND)
    A.jmp("rt_w_answer")
    A.label("rt_w_big")
    A.mov_rm_imm32(("r", RAX), ST_TOO_LARGE)
    A.jmp("rt_w_answer")
    A.label("rt_w_bad")
    A.mov_rm_imm32(("r", RAX), ST_BAD_ADDRESS)
    # rax = the word: into STATUS[0] when that is writable; PATH and BUFFER are forgotten
    A.label("rt_w_answer")
    A.mov_r_rm(RBX, ("r", RAX))
    A.mov_r_rm(RSI, ("abs", "win_st", 0))
    A.mov_rm_imm32(("r", RAX), 8)
    A.call("rt_w_wr")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "rt_w_an1")
    A.mov_rm_r(("m", RSI, 0), RBX)
    A.label("rt_w_an1")
    A.mov_rm_imm32(("abs", "win_path", 0), 0)
    A.mov_rm_imm32(("abs", "win_buf", 0), 0)
    A.mov_rm_imm32(("abs", "win_param", 0), 0)
    A.label("rt_w_an2")
    for r in reversed(SAVE):
        A.pop(r)
    A.ret()
    # rsi = a run: rcx = 0 and rax = its length when its header is readable, else rcx = 1
    A.label("rt_w_hdr")
    A.mov_r_rm(RAX, ("r", RSI))
    A.mov_r_imm64(RDX, base + 8)
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    A.jcc("b", "rt_w_no")
    A.mov_r_imm64(RDX, base)
    A.alu_r_rm("sub", RAX, ("r", RDX))
    A.alu_r_rm("cmp", RAX, ("abs", "top", 0))
    A.jcc("a", "rt_w_no")
    A.mov_r_rm(RAX, ("m", RSI, -8))
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.ret()
    # rsi, rax = n: rcx = 0 when [rsi, rsi + n) is readable (rt_w_rd) or writable (rt_w_wr)
    for lab, lo, hi in (("rt_w_rd", base, "top"), ("rt_w_wr", base + prot_size, "ram")):
        A.label(lab)
        A.mov_r_rm(RDX, ("r", RSI))
        A.mov_r_imm64(RCX, lo)
        A.alu_r_rm("sub", RDX, ("r", RCX))
        A.jcc("b", "rt_w_no")
        A.mov_r_rm(RCX, ("abs", hi, 0))
        if lo != base:
            A.alu_rm_imm("sub", ("r", RCX), lo - base)
        A.alu_r_rm("cmp", RDX, ("r", RCX))
        A.jcc("a", "rt_w_no")
        A.alu_r_rm("sub", RCX, ("r", RDX))
        A.alu_r_rm("cmp", RAX, ("r", RCX))
        A.jcc("a", "rt_w_no")
        A.alu_r_rm("xor", RCX, ("r", RCX), 0)
        A.ret()
    A.label("rt_w_no")
    A.mov_rm_imm32(("r", RCX), 1)
    A.ret()
    # rax = the word, as rt_w_answer, but the remembered runs are KEPT: yantra's GO / PUT with
    # no PATH or no BUFFER named answers BadAddress and forgets nothing
    A.label("rt_w_keep")
    A.mov_r_rm(RBX, ("r", RAX))
    A.mov_r_rm(RSI, ("abs", "win_st", 0))
    A.mov_rm_imm32(("r", RAX), 8)
    A.call("rt_w_wr")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "rt_w_an2")
    A.mov_rm_r(("m", RSI, 0), RBX)
    A.jmp("rt_w_an2")
    files_engine(P, P.osn, "x86", base, prot_size)


# ---- the FILE operations of the runtime window (patra parity, then the design's §3 table).
# Written ONCE as a small program over a portable instruction set (softfp.engine's idea) and
# compiled by two backends, PwX86 and PwA64; the host calls are the only per-OS part:
# Linux syscalls, Darwin syscalls, or kernel32 (Windows), chosen here in Python.
#   set mov add sub and_ or_ xor shl shr mul udiv     d = a op b (b: a variable or an int)
#   br(cond, a, b, label)   jmp  call  ret  label  lab
#   ldm(d, p, off, w) / stm(p, v, off, w)            w-octet load (zero-extended) / store at host p+off
#   buf(d, name)  a scratch buffer's host address    lea(d, label)  a runtime label's address
#   rt(d, name)   a runtime word (top ram envp win_path win_buf win_param delta)
#   g2h(d, a)     guest address -> host address      sys(n)  a unix syscall: a0..a5 -> r (-errno)
#   win(name, n, ret32)  a kernel32 call: a0..a(n-1) -> r
# The handlers answer in the variable "st"; the window code writes it into STATUS[0].
WIN_GO, WIN_PUT, WIN_PARAM = 0x10, 0x18, 0x20
WIN_APPEND, WIN_DELETE, WIN_RENAME, WIN_LIST = 0x28, 0x30, 0x38, 0x40
WIN_STAT, WIN_MKDIR, WIN_READAT, WIN_WRITEAT = 0x48, 0x50, 0x58, 0x60
WIN_FILE_OPS = (WIN_GO, WIN_PUT, WIN_APPEND, WIN_DELETE, WIN_RENAME, WIN_LIST, WIN_STAT, WIN_MKDIR,
                WIN_READAT, WIN_WRITEAT)
# every status word by name (patra.rs Status::word, then the runtime design §5)
PW_ST = {"BadAddress": M64 - 1, "BadPath": M64 - 2, "Refused": M64 - 3, "NotFound": M64 - 4,
         "TooLarge": M64 - 5, "NotWritten": M64 - 6, "NotGranted": M64 - 7, "NotADirectory": M64 - 8,
         "IsADirectory": M64 - 9, "AlreadyExists": M64 - 10}
PW_SLOTS = 160
PW_LR = 32
PW_PATHCAP = 16384
PWB = {}
_o = 0
for _n, _sz in (("rr", PW_PATHCAP), ("p1", PW_PATHCAP), ("rp", PW_PATHCAP), ("p2", PW_PATHCAP),
                ("tp", PW_PATHCAP), ("p3", PW_PATHCAP), ("w1", 65536), ("w2", 65536), ("dents", 32768),
                ("dname", 1024), ("stbuf", 256), ("info", 64), ("find", 640), ("ov", 32), ("got", 16),
                ("fdn", 32), ("basep", 16)):
    PWB[_n] = (_o, _sz)
    _o += _sz
PWB_SIZE = _o
PW_FILES_ENV = b"YANTRA_FILES="
# unix host calls: (x86-64 Linux, aarch64 Linux, Darwin)
PW_SYS = {"openat": (257, 56, None), "open": (None, None, 5), "close": (3, 57, 6), "read": (0, 63, 3),
          "write": (1, 64, 4), "pread": (17, 67, 153), "pwrite": (18, 68, 154), "fstat": (5, 80, 339),
          "lstat": (None, None, 340), "fstatat": (262, 79, None), "readlinkat": (267, 78, None),
          "fcntl": (None, None, 92), "mkdirat": (258, 34, None), "mkdir": (None, None, 136),
          "unlinkat": (263, 35, None), "unlink": (None, None, 10), "renameat2": (316, 276, None),
          "renameatx_np": (None, None, 488), "getdents64": (217, 61, None),
          "getdirentries64": (None, None, 344), "mmap": (9, 222, 197), "munmap": (11, 215, 73)}


def files_engine(P, osn, isa, base, prot_size):
    """emit pw_main (op in "op"; answer in "st") and everything it calls. osn: linux mac win;
    isa: x86 a64 (Linux's syscall numbers and struct layouts differ by isa)"""
    L = P.lab
    win, mac, lin = osn == "win", osn == "mac", osn == "linux"
    SEP = 0x5C if win else 0x2F
    S = PW_ST

    def sysn(name):
        return PW_SYS[name][2 if mac else (0 if isa == "x86" else 1)]

    def sc(name, *args):
        for i, a in enumerate(args):
            P.mov("a%d" % i, a) if isinstance(a, str) else P.set("a%d" % i, a)
        P.sys(sysn(name))

    def wc(name, *args, ret32=False):
        for i, a in enumerate(args):
            P.mov("a%d" % i, a) if isinstance(a, str) else P.set("a%d" % i, a)
        P.win(name, len(args), ret32)

    def fail(status):
        P.set("st", S[status])
        P.ret()

    def unless(cond, a, b, status, *pre):
        """if NOT (a cond b): run pre (calls), answer status and return"""
        ok = L()
        P.br(cond, a, b, ok)
        for f in pre:
            f()
        fail(status)
        P.label(ok)

    def closer(var):
        return lambda: (P.mov("ph", var), P.call("pr_close"))

    # Linux flags differ by isa in two bits
    O_DIR = 0x10000 if isa == "x86" else 0x4000
    O_NOFOL = 0x20000 if isa == "x86" else 0x8000
    if mac:
        O_WR, O_CR, O_TR, O_AP, O_NB, O_NF, O_DI, O_CX = 1, 0x200, 0x400, 8, 4, 0x100, 0x100000, 0x1000000
    else:
        O_WR, O_CR, O_TR, O_AP, O_NB, O_NF, O_DI, O_CX = 1, 0x40, 0x200, 0x400, 0x800, O_NOFOL, O_DIR, 0x80000

    # ================================================================ the host calls
    # pr_open_path(pp) -> r: an object, its links followed, opened to be NAMED (no data access)
    P.label("pr_open_path")
    if win:
        P.mov("wsrc", "pp")
        P.buf("wdst", "w1")
        P.call("pr_wide")
        P.br("eq", "r", 0, "pr_wfail")
        wc("CreateFileW", "wdst", 0, 7, 0, 3, 0x0200_0000, 0)
    elif mac:
        sc("open", "pp", O_NB | O_CX, 0)
    else:
        sc("openat", -100, "pp", 0x200000 | O_CX, 0)       # O_PATH
    P.ret()
    # pr_open_read(pp) -> r
    P.label("pr_open_read")
    if win:
        P.mov("wsrc", "pp")
        P.buf("wdst", "w1")
        P.call("pr_wide")
        P.br("eq", "r", 0, "pr_wfail")
        wc("CreateFileW", "wdst", 0x8000_0000, 7, 0, 3, 0, 0)
    elif mac:
        sc("open", "pp", O_CX, 0)
    else:
        sc("openat", -100, "pp", O_CX, 0)
    P.ret()
    # pr_open_write(pp, pm: 0 create+truncate, 1 append, 2 create) -> r; a link at the leaf is not followed
    P.label("pr_open_write")
    if win:
        P.mov("wsrc", "pp")
        P.buf("wdst", "w1")
        P.call("pr_wide")
        P.br("eq", "r", 0, "pr_wfail")
        l1, l2 = L(), L()
        P.br("ne", "pm", 0, l1)
        wc("CreateFileW", "wdst", 0x4000_0000, 7, 0, 2, 0x80, 0)
        P.ret()
        P.label(l1)
        P.br("ne", "pm", 1, l2)
        wc("CreateFileW", "wdst", 0x0010_0004, 7, 0, 4, 0x80, 0)     # FILE_APPEND_DATA | SYNCHRONIZE, OPEN_ALWAYS
        P.ret()
        P.label(l2)
        wc("CreateFileW", "wdst", 0x4000_0000, 7, 0, 4, 0x80, 0)
        P.ret()
    else:
        l1, l2, l3 = L(), L(), L()
        P.set("pf", O_WR | O_CR | O_NF | O_CX | O_TR)
        P.br("ne", "pm", 1, l1)
        P.set("pf", O_WR | O_CR | O_NF | O_CX | O_AP)
        P.label(l1)
        P.br("ne", "pm", 2, l2)
        P.set("pf", O_WR | O_CR | O_NF | O_CX)
        P.label(l2)
        if mac:
            sc("open", "pp", "pf", 0o666)
        else:
            sc("openat", -100, "pp", "pf", 0o666)
        P.ret()
    # pr_close(ph)
    P.label("pr_close")
    if win:
        wc("CloseHandle", "ph", ret32=True)
    else:
        sc("close", "ph")
    P.ret()
    # pr_getpath(ph, po: a PW_PATHCAP buffer) -> r = the octets of the real path (zero-ended), or -1
    P.label("pr_getpath")
    bad = L()
    if win:
        P.buf("wdst", "w1")
        wc("GetFinalPathNameByHandleW", "ph", "wdst", 32768, 0, ret32=True)
        P.br("eq", "r", 0, bad)
        P.br("geu", "r", 32768, bad)
        wc("WideCharToMultiByte", 65001, 0, "wdst", -1, "po", PW_PATHCAP, 0, 0, ret32=True)
        P.br("eq", "r", 0, bad)
        P.sub("r", "r", 1)
        P.ret()
    elif mac:
        sc("fcntl", "ph", 50, "po")                 # F_GETPATH (MAXPATHLEN octets)
        P.br("lts", "r", 0, bad)
        P.mov("sa", "po")
        P.call("pw_strlen")
        P.mov("r", "sl")
        P.ret()
    else:
        # readlink("/proc/self/fd/N")
        P.buf("t0", "fdn")
        for i, c in enumerate(b"/proc/self/fd/"):
            P.stm("t0", c, i, 1)
        P.mov("t1", "ph")
        P.set("t2", 0)                              # digits
        lp, l2 = L(), L()
        P.label(lp)
        P.udiv("t3", "t1", 10)
        P.mul("t4", "t3", 10)
        P.sub("t4", "t1", "t4")
        P.add("t4", "t4", 0x30)
        P.buf("t5", "got")
        P.add("t5", "t5", "t2")
        P.stm("t5", "t4", 0, 1)
        P.add("t2", "t2", 1)
        P.mov("t1", "t3")
        P.br("ne", "t1", 0, lp)
        P.add("t0", "t0", 14)                       # the digits, reversed into place
        P.label(l2)
        P.sub("t2", "t2", 1)
        P.buf("t5", "got")
        P.add("t5", "t5", "t2")
        P.ldm("t4", "t5", 0, 1)
        P.stm("t0", "t4", 0, 1)
        P.add("t0", "t0", 1)
        P.br("ne", "t2", 0, l2)
        P.stm("t0", 0, 0, 1)
        P.buf("t0", "fdn")
        sc("readlinkat", -100, "t0", "po", PW_PATHCAP - 1)
        P.br("lts", "r", 0, bad)
        P.br("geu", "r", PW_PATHCAP - 1, bad)
        P.add("t0", "po", "r")
        P.stm("t0", 0, 0, 1)
        P.ret()
    P.label(bad)
    P.set("r", -1)
    P.ret()
    # pr_fkind(ph) -> r (< 0 failed), kind (1 file, 2 directory, 3 other), size (a file's, else 0)
    P.label("pr_fkind")
    bad, k2, k3, kd = L(), L(), L(), L()
    if win:
        P.buf("t0", "info")
        wc("GetFileInformationByHandle", "ph", "t0", ret32=True)
        P.br("eq", "r", 0, bad)
        P.ldm("t1", "t0", 0, 4)
        P.ldm("t2", "t0", 32, 4)
        P.ldm("t3", "t0", 36, 4)
        P.shl("t2", "t2", 32)
        P.or_("size", "t2", "t3")
        P.set("kind", 1)
        P.and_("t1", "t1", 0x10)
        P.br("eq", "t1", 0, kd)
        P.set("kind", 2)
        P.set("size", 0)
        P.label(kd)
        P.set("r", 0)
        P.ret()
    else:
        P.buf("t0", "stbuf")
        sc("fstat", "ph", "t0")
        P.br("lts", "r", 0, bad)
        if mac:
            P.ldm("t1", "t0", 4, 2)
            P.ldm("size", "t0", 96, 8)
        else:
            P.ldm("t1", "t0", 24 if isa == "x86" else 16, 4)
            P.ldm("size", "t0", 48, 8)
        P.and_("t1", "t1", 0xF000)
        P.set("kind", 1)
        P.br("eq", "t1", 0x8000, kd)
        P.set("size", 0)
        P.set("kind", 2)
        P.br("eq", "t1", 0x4000, kd)
        P.set("kind", 3)
        P.label(kd)
        P.set("r", 0)
        P.ret()
    P.label(bad)
    P.set("r", -1)
    P.ret()
    # pr_lkind(pp) -> kind of the entry itself, a link not followed: 0 absent, 1 file, 2 directory, 3 other
    P.label("pr_lkind")
    no, kd = L(), L()
    if win:
        P.mov("wsrc", "pp")
        P.buf("wdst", "w1")
        P.call("pr_wide")
        P.br("eq", "r", 0, no)
        wc("GetFileAttributesW", "wdst", ret32=True)
        P.br("eq", "r", 0xFFFF_FFFF, no)
        P.set("kind", 3)
        P.and_("t1", "r", 0x400)
        P.br("ne", "t1", 0, kd)                      # a reparse point (a link)
        P.set("kind", 2)
        P.and_("t1", "r", 0x10)
        P.br("ne", "t1", 0, kd)
        P.set("kind", 1)
        P.label(kd)
        P.ret()
    else:
        P.buf("t0", "stbuf")
        if mac:
            sc("lstat", "pp", "t0")
        else:
            sc("fstatat", -100, "pp", "t0", 0x100)     # AT_SYMLINK_NOFOLLOW
        P.br("lts", "r", 0, no)
        if mac:
            P.ldm("t1", "t0", 4, 2)
        else:
            P.ldm("t1", "t0", 24 if isa == "x86" else 16, 4)
        P.and_("t1", "t1", 0xF000)
        P.set("kind", 1)
        P.br("eq", "t1", 0x8000, kd)
        P.set("kind", 2)
        P.br("eq", "t1", 0x4000, kd)
        P.set("kind", 3)
        P.label(kd)
        P.ret()
    P.label(no)
    P.set("kind", 0)
    P.ret()
    # pr_read(ph, pb, pn) / pr_pread(+ poff) -> r (count, 0 at the end, < 0 failed); n <= 2^30
    # pr_write / pr_pwrite the same
    for name, wname, ov in (("pr_read", "ReadFile", 0), ("pr_pread", "ReadFile", 1),
                            ("pr_write", "WriteFile", 0), ("pr_pwrite", "WriteFile", 1)):
        P.label(name)
        if win:
            bad = L()
            P.buf("t0", "got")
            P.stm("t0", 0, 0, 8)
            if ov:
                P.buf("t1", "ov")
                for o in (0, 8, 16, 24):
                    P.stm("t1", 0, o, 8)
                P.stm("t1", "poff", 16, 8)           # OVERLAPPED's Offset, OffsetHigh
                wc(wname, "ph", "pb", "pn", "t0", "t1", ret32=True)
            else:
                wc(wname, "ph", "pb", "pn", "t0", 0, ret32=True)
            P.br("eq", "r", 0, bad)
            P.buf("t0", "got")
            P.ldm("r", "t0", 0, 4)
            P.ret()
            P.label(bad)
            P.set("r", -1)
            P.ret()
        else:
            base_n = {"pr_read": "read", "pr_pread": "pread", "pr_write": "write", "pr_pwrite": "pwrite"}[name]
            if ov:
                sc(base_n, "ph", "pb", "pn", "poff")
            else:
                sc(base_n, "ph", "pb", "pn")
            P.ret()
    # pr_mkdir(pp) / pr_unlink(pp) / pr_rename(pp, pq): r = 0 or < 0
    for name in ("pr_mkdir", "pr_unlink", "pr_rename"):
        P.label(name)
        if win:
            P.mov("wsrc", "pp")
            P.buf("wdst", "w1")
            P.call("pr_wide")
            P.br("eq", "r", 0, "pr_wfail")
            if name == "pr_rename":
                P.mov("wsrc", "pq")
                P.buf("wdst", "w2")
                P.call("pr_wide")
                P.br("eq", "r", 0, "pr_wfail")
                P.buf("t0", "w1")
                wc("MoveFileExW", "t0", "wdst", 0, ret32=True)
            elif name == "pr_mkdir":
                wc("CreateDirectoryW", "wdst", 0, ret32=True)
            else:
                wc("DeleteFileW", "wdst", ret32=True)
            ok = L()
            P.set("t0", 0)
            P.br("ne", "r", 0, ok)
            P.set("t0", -1)
            P.label(ok)
            P.mov("r", "t0")
        elif mac:
            if name == "pr_mkdir":
                sc("mkdir", "pp", 0o777)
            elif name == "pr_unlink":
                sc("unlink", "pp")
            else:
                sc("renameatx_np", -2, "pp", -2, "pq", 4)     # RENAME_EXCL
        else:
            if name == "pr_mkdir":
                sc("mkdirat", -100, "pp", 0o777)
            elif name == "pr_unlink":
                sc("unlinkat", -100, "pp", 0)
            else:
                sc("renameat2", -100, "pp", -100, "pq", 1)    # RENAME_NOREPLACE
        P.ret()
    # pr_dopen(pp) -> r (a handle; Windows: a find handle, its first entry pending); pr_dnext(ph)
    # -> dn (a zero-ended name) and dl (its octets), dl = -1 at the end; pr_dclose(ph)
    P.label("pr_dopen")
    if win:
        # the pattern: the directory, "\\*"
        P.buf("t0", "p2")
        P.mov("t1", "pp")
        lp, dn = L(), L()
        P.label(lp)
        P.ldm("t2", "t1", 0, 1)
        P.br("eq", "t2", 0, dn)
        P.stm("t0", "t2", 0, 1)
        P.add("t0", "t0", 1)
        P.add("t1", "t1", 1)
        P.jmp(lp)
        P.label(dn)
        P.stm("t0", 0x5C, 0, 1)
        P.stm("t0", 0x2A, 1, 1)
        P.stm("t0", 0, 2, 1)
        P.buf("wsrc", "p2")
        P.buf("wdst", "w1")
        P.call("pr_wide")
        P.br("eq", "r", 0, "pr_wfail")
        P.buf("t0", "find")
        wc("FindFirstFileW", "wdst", "t0")
        P.set("dfirst", 1)
    else:
        P.set("dpos", 0)
        P.set("dend", 0)
        if mac:
            sc("open", "pp", O_DI | O_CX | O_NB, 0)
        else:
            sc("openat", -100, "pp", O_DI | O_CX, 0)
    P.ret()
    P.label("pr_dnext")
    end = L()
    if win:
        got1 = L()
        P.br("ne", "dfirst", 0, got1)
        P.buf("t0", "find")
        wc("FindNextFileW", "ph", "t0", ret32=True)
        P.br("eq", "r", 0, end)
        P.label(got1)
        P.set("dfirst", 0)
        P.buf("t0", "find")
        P.add("t0", "t0", 44)                       # cFileName
        P.buf("dn", "dname")
        wc("WideCharToMultiByte", 65001, 0, "t0", -1, "dn", 1024, 0, 0, ret32=True)
        P.br("eq", "r", 0, end)
        P.sub("dl", "r", 1)
        P.ret()
    else:
        have = L()
        P.br("ltu", "dpos", "dend", have)
        P.buf("t0", "dents")
        if mac:
            P.buf("t1", "basep")
            sc("getdirentries64", "ph", "t0", 32768, "t1")
        else:
            sc("getdents64", "ph", "t0", 32768)
        P.br("les", "r", 0, end)
        P.set("dpos", 0)
        P.mov("dend", "r")
        P.label(have)
        P.buf("t0", "dents")
        P.add("t0", "t0", "dpos")
        P.ldm("t1", "t0", 16, 2)                     # d_reclen
        P.add("dpos", "dpos", "t1")
        P.add("dn", "t0", 21 if mac else 19)
        P.mov("sa", "dn")
        P.call("pw_strlen")
        P.mov("dl", "sl")
        P.ret()
    P.label(end)
    P.set("dl", -1)
    P.ret()
    P.label("pr_dclose")
    if win:
        nothing = L()
        P.br("eq", "ph", -1, nothing)
        wc("FindClose", "ph", ret32=True)
        P.label(nothing)
    else:
        sc("close", "ph")
    P.ret()
    # pr_alloc(pn) -> r (0 on failure); pr_free(pb, pn)
    P.label("pr_alloc")
    if win:
        wc("VirtualAlloc", 0, "pn", 0x3000, 4)
    else:
        ok = L()
        sc("mmap", 0, "pn", 3, 0x1002 if mac else 0x22, -1, 0)
        P.br("ges", "r", 0, ok)
        P.set("r", 0)
        P.label(ok)
    P.ret()
    P.label("pr_free")
    if win:
        wc("VirtualFree", "pb", 0, 0x8000, ret32=True)
    else:
        sc("munmap", "pb", "pn")
    P.ret()
    if win:
        # pr_wide(wsrc UTF-8 zero-ended, wdst 32768 units) -> r = units (0 = failure)
        P.label("pr_wide")
        wc("MultiByteToWideChar", 65001, 0, "wsrc", -1, "wdst", 32768, ret32=True)
        P.ret()
        P.label("pr_wfail")                         # a path Windows cannot take: -1
        P.set("r", -1)
        P.ret()

    # ================================================================ shared routines
    # pw_strlen(sa) -> sl
    P.label("pw_strlen")
    lp, dn = L(), L()
    P.set("sl", 0)
    P.label(lp)
    P.add("t9", "sa", "sl")
    P.ldm("t9", "t9", 0, 1)
    P.br("eq", "t9", 0, dn)
    P.add("sl", "sl", 1)
    P.jmp(lp)
    P.label(dn)
    P.ret()
    # pw_hdr(ra guest) -> ok, rl: the run's header is readable (ra >= base + 8, ra - base <= top)
    P.label("pw_hdr")
    no = L()
    P.set("ok", 0)
    P.br("ltu", "ra", base + 8, no)
    P.sub("t9", "ra", base)
    P.rt("t8", "top")
    P.br("gtu", "t9", "t8", no)
    P.g2h("t9", "ra")
    P.ldm("rl", "t9", -8, 8)
    P.set("ok", 1)
    P.label(no)
    P.ret()
    # pw_rd / pw_wr(ga guest, gn) -> ok: [ga, ga + gn) readable ([base, base+top)) / writable
    # ([base + prot_size, base + ram)), the shared window's rules (rt_w_rd, rt_w_wr)
    for lab, lo, hi in (("pw_rd", base, "top"), ("pw_wr", base + prot_size, "ram")):
        P.label(lab)
        no = L()
        P.set("ok", 0)
        P.br("ltu", "ga", lo, no)
        P.sub("t9", "ga", lo)
        P.rt("t8", hi)
        P.sub("t8", "t8", lo - base)
        P.br("gtu", "t9", "t8", no)
        P.sub("t8", "t8", "t9")
        P.br("gtu", "gn", "t8", no)
        P.set("ok", 1)
        P.label(no)
        P.ret()
    # pw_utf8(ua host, un) -> ok: Rust's from_utf8
    P.label("pw_utf8")
    lp, bad, one, nxt = L(), L(), L(), L()
    P.set("ok", 0)
    P.set("ui", 0)
    P.label(lp)
    P.br("geu", "ui", "un", one)
    P.add("t9", "ua", "ui")
    P.ldm("t8", "t9", 0, 1)
    P.add("ui", "ui", 1)
    P.br("ltu", "t8", 0x80, lp)
    P.br("ltu", "t8", 0xC2, bad)
    two, three, four, cont = L(), L(), L(), L()
    P.set("ulo", 0x80)
    P.set("uhi", 0xBF)
    P.br("ltu", "t8", 0xE0, two)
    P.br("ltu", "t8", 0xF0, three)
    P.br("geu", "t8", 0xF5, bad)
    # four octets
    nf = L()
    P.br("ne", "t8", 0xF0, nf)
    P.set("ulo", 0x90)
    P.label(nf)
    nf = L()
    P.br("ne", "t8", 0xF4, nf)
    P.set("uhi", 0x8F)
    P.label(nf)
    P.set("uc", 3)
    P.jmp(cont)
    P.label(three)
    nf = L()
    P.br("ne", "t8", 0xE0, nf)
    P.set("ulo", 0xA0)
    P.label(nf)
    nf = L()
    P.br("ne", "t8", 0xED, nf)
    P.set("uhi", 0x9F)
    P.label(nf)
    P.set("uc", 2)
    P.jmp(cont)
    P.label(two)
    P.set("uc", 1)
    P.label(cont)
    # the first continuation in [ulo, uhi], the rest in [0x80, 0xBF]
    P.add("t7", "ui", "uc")
    P.br("gtu", "t7", "un", bad)
    P.add("t9", "ua", "ui")
    P.ldm("t8", "t9", 0, 1)
    P.br("ltu", "t8", "ulo", bad)
    P.br("gtu", "t8", "uhi", bad)
    P.add("ui", "ui", 1)
    P.sub("uc", "uc", 1)
    P.set("ulo", 0x80)
    P.set("uhi", 0xBF)
    P.br("ne", "uc", 0, cont)
    P.jmp(lp)
    P.label(one)
    P.set("ok", 1)
    P.label(bad)
    P.ret()
    # pw_root -> ok; rr = the root's real path (rrlen); jr / jrlen = what paths are joined to
    # (unix: the real root, as yantra-run canonicalises DIR; Windows: DIR as given, so that
    # Win32 resolves "..", which a \\?\ path would not)
    P.label("pw_root")
    no = L()
    P.set("ok", 0)
    P.rt("t0", "envp")
    lp, nxt, hit = L(), L(), L()
    P.label(lp)
    P.ldm("t1", "t0", 0, 8)
    P.br("eq", "t1", 0, no)
    P.lea("t2", "env_files")
    P.set("t3", 0)
    lp2 = L()
    P.label(lp2)
    P.add("t4", "t2", "t3")
    P.ldm("t4", "t4", 0, 1)
    P.add("t5", "t1", "t3")
    P.ldm("t5", "t5", 0, 1)
    P.br("ne", "t4", "t5", nxt)
    P.add("t3", "t3", 1)
    P.br("ne", "t3", len(PW_FILES_ENV), lp2)
    P.add("fe", "t1", "t3")
    P.jmp(hit)
    P.label(nxt)
    P.add("t0", "t0", 8)
    P.jmp(lp)
    P.label(hit)
    P.mov("pp", "fe")
    P.call("pr_open_path")
    P.br("lts", "r", 0, no)
    P.mov("hroot", "r")
    P.mov("ph", "r")
    P.call("pr_fkind")
    P.mov("t6", "r")
    P.mov("t7", "kind")
    P.mov("ph", "hroot")
    P.buf("po", "rr")
    P.call("pr_getpath")
    P.mov("rrlen", "r")
    P.mov("ph", "hroot")
    P.call("pr_close")
    P.br("lts", "t6", 0, no)
    P.br("ne", "t7", 2, no)                         # DIR must be a directory
    P.br("lts", "rrlen", 0, no)
    if win:
        P.mov("jr", "fe")
        P.mov("sa", "fe")
        P.call("pw_strlen")
        P.mov("jrlen", "sl")
    else:
        P.buf("jr", "rr")
        P.mov("jrlen", "rrlen")
    P.set("ok", 1)
    P.label(no)
    P.ret()
    # pw_form(pa host, pn) -> ok = 0 when the path is absolute or drive-qualified
    P.label("pw_form")
    no, yes = L(), L()
    P.set("ok", 1)
    P.br("eq", "pn", 0, yes)
    P.ldm("t0", "pa", 0, 1)
    P.br("eq", "t0", 0x2F, no)
    if win:
        P.br("eq", "t0", 0x5C, no)
        P.br("ltu", "pn", 2, yes)
        P.ldm("t1", "pa", 1, 1)
        P.br("ne", "t1", 0x3A, yes)
        P.or_("t0", "t0", 0x20)
        P.sub("t0", "t0", 0x61)
        P.br("ltu", "t0", 26, no)
    P.jmp(yes)
    P.label(no)
    P.set("ok", 0)
    P.label(yes)
    P.ret()
    # the builder: bp the cursor, be its last usable octet, bok = 0 after a zero octet or overflow
    # pw_bput(cs, cn) appends; pw_bsep appends SEP unless the text ends with a separator;
    # pw_bfin ends it with a zero octet
    P.label("pw_bput")
    lp, dn, bad = L(), L(), L()
    P.set("t9", 0)
    P.label(lp)
    P.br("geu", "t9", "cn", dn)
    P.br("geu", "bp", "be", bad)
    P.add("t8", "cs", "t9")
    P.ldm("t8", "t8", 0, 1)
    P.br("eq", "t8", 0, bad)
    P.stm("bp", "t8", 0, 1)
    P.add("bp", "bp", 1)
    P.add("t9", "t9", 1)
    P.jmp(lp)
    P.label(bad)
    P.set("bok", 0)
    P.label(dn)
    P.ret()
    P.label("pw_bsep")
    dn, bad = L(), L()
    P.br("eq", "bp", "bstart", dn)                  # Rust's push: no separator after nothing
    P.ldm("t8", "bp", -1, 1)
    P.br("eq", "t8", SEP, dn)
    if win:
        P.br("eq", "t8", 0x2F, dn)
    P.br("geu", "bp", "be", bad)
    P.stm("bp", SEP, 0, 1)
    P.add("bp", "bp", 1)
    P.ret()
    P.label(bad)
    P.set("bok", 0)
    P.label(dn)
    P.ret()
    P.label("pw_bfin")
    P.stm("bp", 0, 0, 1)
    P.ret()

    def bopen(name):
        P.buf("bp", name)
        P.mov("bstart", "bp")
        P.add("be", "bp", PWB[name][1] - 1)
        P.set("bok", 1)

    # pw_canon(cp: a zero-ended path; co: the real path's buffer) -> rc (0 ok, 1 not opened,
    # 2 outside the root), h (open, rc = 0), con (the real path's octets)
    P.label("pw_canon")
    no, out = L(), L()
    P.set("rc", 1)
    P.mov("pp", "cp")
    P.call("pr_open_path")
    P.br("lts", "r", 0, no)
    P.mov("h", "r")
    P.mov("ph", "r")
    P.mov("po", "co")
    P.call("pr_getpath")
    P.mov("con", "r")
    P.br("lts", "r", 0, out)
    # the prefix: rr, then the end or a separator (or rr ends with one)
    P.set("rc", 2)
    P.br("ltu", "con", "rrlen", out)
    P.set("t0", 0)
    lp, cmpd = L(), L()
    P.label(lp)
    P.br("geu", "t0", "rrlen", cmpd)
    P.add("t1", "co", "t0")
    P.ldm("t1", "t1", 0, 1)
    P.buf("t2", "rr")
    P.add("t2", "t2", "t0")
    P.ldm("t2", "t2", 0, 1)
    P.br("ne", "t1", "t2", out)
    P.add("t0", "t0", 1)
    P.jmp(lp)
    P.label(cmpd)
    good = L()
    P.br("eq", "con", "rrlen", good)
    P.add("t1", "co", "rrlen")
    P.ldm("t1", "t1", 0, 1)
    P.br("eq", "t1", SEP, good)
    P.br("eq", "rrlen", 0, out)
    P.buf("t2", "rr")
    P.add("t2", "t2", "rrlen")
    P.ldm("t2", "t2", -1, 1)
    P.br("ne", "t2", SEP, out)
    P.label(good)
    P.set("rc", 0)
    P.ret()
    P.label(out)
    P.mov("ph", "h")
    P.call("pr_close")
    P.label(no)
    P.ret()
    # pw_runs(need: 1 PATH, 2 + BUFFER, 3 + BUFFER as a second path) -> fails with BadAddress /
    # BadPath (st set, "ok" = 0), else pa/pn (the path, host), bl (BUFFER's length), qa/qn
    P.label("pw_runs")
    bad, badp, done, keep = L(), L(), L(), L()
    P.rt("ra", "win_path")
    P.br("eq", "ra", 0, keep)                       # no PATH named: BadAddress, the window remembers on
    P.rt("t0", "win_buf")
    nb = L()
    P.br("eq", "need", 1, nb)
    P.br("eq", "t0", 0, keep)
    P.label(nb)
    P.call("pw_hdr")
    P.br("eq", "ok", 0, bad)
    P.mov("pn", "rl")
    has_b = L()
    P.br("eq", "need", 1, has_b)
    P.rt("ra", "win_buf")
    P.call("pw_hdr")
    P.br("eq", "ok", 0, bad)
    P.mov("bl", "rl")
    P.label(has_b)
    P.rt("ga", "win_path")
    P.mov("gn", "pn")
    P.call("pw_rd")
    P.br("eq", "ok", 0, bad)
    P.g2h("pa", "ga")
    P.mov("ua", "pa")
    P.mov("un", "pn")
    P.call("pw_utf8")
    P.br("eq", "ok", 0, badp)
    P.br("ne", "need", 3, done)
    P.rt("ga", "win_buf")
    P.mov("gn", "bl")
    P.call("pw_rd")
    P.br("eq", "ok", 0, bad)
    P.g2h("qa", "ga")
    P.mov("qn", "bl")
    P.mov("ua", "qa")
    P.mov("un", "qn")
    P.call("pw_utf8")
    P.br("eq", "ok", 0, badp)
    P.label(done)
    P.set("ok", 1)
    P.ret()
    P.label(bad)
    P.set("st", S["BadAddress"])
    P.set("ok", 0)
    P.ret()
    P.label(badp)
    P.set("st", S["BadPath"])
    P.set("ok", 0)
    P.ret()
    P.label(keep)
    P.set("keep", 1)
    P.jmp(bad)
    # pw_param -> ok, off = PARAM[0] (the run must hold a word)
    P.label("pw_param")
    no, has = L(), L()
    P.rt("ra", "win_param")
    P.br("ne", "ra", 0, has)
    P.set("keep", 1)
    P.set("ok", 0)
    P.ret()
    P.label(has)
    P.call("pw_hdr")
    P.br("eq", "ok", 0, no)
    P.set("ok", 0)
    P.br("eq", "rl", 0, no)
    P.rt("ga", "win_param")
    P.set("gn", 8)
    P.call("pw_rd")
    P.br("eq", "ok", 0, no)
    P.g2h("t0", "ga")
    P.ldm("off", "t0", 0, 8)
    P.label(no)
    P.ret()
    # pw_existing(pa, pn) -> rc (0: h open on the object, rp its real path (rpn); 1 not found;
    # 2 refused): yantra's read-side resolve, root.join(path).canonicalize() + the prefix
    P.label("pw_existing")
    no = L()
    bopen("p1")
    P.mov("cs", "jr")
    P.mov("cn", "jrlen")
    P.call("pw_bput")
    P.call("pw_bsep")
    P.mov("cs", "pa")
    P.mov("cn", "pn")
    P.call("pw_bput")
    P.call("pw_bfin")
    P.set("rc", 1)
    P.br("eq", "bok", 0, no)
    P.buf("cp", "p1")
    P.buf("co", "rp")
    P.call("pw_canon")
    P.mov("rpn", "con")
    P.label(no)
    P.ret()
    # pw_entry(pa, pn, follow) -> wr: 0 ok (tp: the target's path, zero-ended; tk its kind, 0 =
    # absent), 1 the parent did not resolve, 2 refused (an escape, or no file name), 3 the
    # target's name is unusable (a zero octet, too long), 4 the leaf is a link that did not
    # resolve (follow only). yantra's write-side resolve: the PARENT canonicalised and checked,
    # the name appended; with follow, an existing leaf is canonicalised and checked too.
    P.label("pw_entry")
    # the last component (not "" or "."): ls = where it starts, nl = its octets
    P.set("t0", 0)
    P.set("ls", 0)
    P.set("nl", 0)
    P.set("t1", 0)                                  # the current component's start
    lp, endc, sepc, skip = L(), L(), L(), L()
    P.label(lp)
    P.br("geu", "t0", "pn", endc)
    P.add("t2", "pa", "t0")
    P.ldm("t2", "t2", 0, 1)
    P.br("eq", "t2", 0x2F, sepc)
    if win:
        P.br("eq", "t2", 0x5C, sepc)
    P.add("t0", "t0", 1)
    P.jmp(lp)
    P.label(sepc)
    P.call("pw_comp")
    P.add("t0", "t0", 1)
    P.mov("t1", "t0")
    P.jmp(lp)
    P.label(endc)
    P.call("pw_comp")
    P.set("wr", 2)
    P.br("ne", "nl", 0, skip)
    P.ret()
    P.label(skip)
    nd = L()
    P.br("ne", "nl", 2, nd)
    P.add("t2", "pa", "ls")
    P.ldm("t3", "t2", 0, 1)
    P.ldm("t4", "t2", 1, 1)
    P.br("ne", "t3", 0x2E, nd)
    P.br("ne", "t4", 0x2E, nd)
    P.ret()                                         # ".." has no file name
    P.label(nd)
    bopen("p1")
    P.mov("cs", "jr")
    P.mov("cn", "jrlen")
    P.call("pw_bput")
    P.call("pw_bsep")
    P.mov("cs", "pa")
    P.mov("cn", "ls")
    P.call("pw_bput")
    P.call("pw_bfin")
    P.set("wr", 1)
    no = L()
    P.br("eq", "bok", 0, no)
    P.buf("cp", "p1")
    P.buf("co", "rp")
    P.call("pw_canon")
    P.mov("rpn", "con")
    P.br("eq", "rc", 1, no)
    P.set("wr", 2)
    P.br("eq", "rc", 2, no)
    P.mov("ph", "h")
    P.call("pr_close")
    bopen("p2")
    P.buf("cs", "rp")
    P.mov("cn", "rpn")
    P.call("pw_bput")
    P.call("pw_bsep")
    P.add("cs", "pa", "ls")
    P.mov("cn", "nl")
    P.call("pw_bput")
    P.call("pw_bfin")
    P.set("wr", 3)
    P.br("eq", "bok", 0, no)
    P.buf("tp", "p2")
    P.buf("pp", "p2")
    P.call("pr_lkind")
    P.mov("tk", "kind")
    P.set("wr", 0)
    P.br("eq", "follow", 0, no)
    P.br("eq", "tk", 0, no)
    # an existing leaf: canonicalised, under the root, or refused
    P.buf("cp", "p2")
    P.buf("co", "tp")
    P.call("pw_canon")
    P.set("wr", 4)
    P.br("ne", "rc", 0, no)
    P.mov("ph", "h")
    P.call("pr_fkind")
    P.mov("tk", "kind")
    P.mov("ph", "h")
    P.call("pr_close")
    P.buf("tp", "tp")
    P.set("wr", 0)
    P.label(no)
    P.ret()
    # pw_comp: the component [t1, t0) of pa, kept when not "" or "."
    P.label("pw_comp")
    keep, no = L(), L()
    P.sub("t5", "t0", "t1")
    P.br("eq", "t5", 0, no)
    P.br("ne", "t5", 1, keep)
    P.add("t6", "pa", "t1")
    P.ldm("t6", "t6", 0, 1)
    P.br("eq", "t6", 0x2E, no)
    P.label(keep)
    P.mov("ls", "t1")
    P.mov("nl", "t5")
    P.label(no)
    P.ret()

    # ================================================================ the operations
    P.label("pw_main")
    P.set("st", S["BadAddress"])
    P.set("keep", 0)
    for op, lab in ((WIN_GO, "pw_go"), (WIN_PUT, "pw_put"), (WIN_APPEND, "pw_append"),
                    (WIN_DELETE, "pw_delete"), (WIN_RENAME, "pw_rename"), (WIN_LIST, "pw_list"),
                    (WIN_STAT, "pw_stat"), (WIN_MKDIR, "pw_mkdir"), (WIN_READAT, "pw_readat"),
                    (WIN_WRITEAT, "pw_writeat")):
        P.br("eq", "op", op, lab)
    P.ret()

    def head(need, param=False, root_first=True):
        """the checks every operation starts with, in yantra's order: the runs, then the root
        and the path's form"""
        P.set("need", need)
        P.call("pw_runs")
        ok = L()
        P.br("ne", "ok", 0, ok)
        P.ret()
        P.label(ok)
        if param:
            P.call("pw_param")
            unless("ne", "ok", 0, "BadAddress")

    def root_and_form():
        P.call("pw_root")
        unless("ne", "ok", 0, "Refused")
        P.call("pw_form")
        unless("ne", "ok", 0, "Refused")

    # ---- GO: a whole-file read (patra serve)
    def read_object(label, at):
        P.label(label)
        head(2, param=at)
        root_and_form()
        P.call("pw_existing")
        unless("ne", "rc", 1, "NotFound")
        unless("ne", "rc", 2, "Refused")
        P.mov("ph", "h")
        P.call("pr_fkind")
        P.mov("k", "kind")
        P.mov("ph", "h")
        P.call("pr_close")
        unless("ne", "k", 2, "IsADirectory" if at else "NotFound")
        P.buf("pp", "rp")
        P.call("pr_open_read")
        unless("ges", "r", 0, "NotFound")
        P.mov("hh", "r")
        P.mov("ph", "hh")
        P.call("pr_fkind")
        unless("ges", "r", 0, "NotFound", closer("hh"))
        if at:
            z = L()
            P.br("ltu", "off", "size", z)
            P.mov("ph", "hh")
            P.call("pr_close")
            P.set("st", 0)                          # at or past the end
            P.ret()
            P.label(z)
            P.sub("n", "size", "off")
            c = L()
            P.br("leu", "n", "bl", c)
            P.mov("n", "bl")
            P.label(c)
        else:
            P.mov("n", "size")
            unless("leu", "n", "bl", "TooLarge", closer("hh"))
        P.rt("ga", "win_buf")
        P.mov("gn", "n")
        P.call("pw_wr")
        unless("ne", "ok", 0, "BadAddress", closer("hh"))
        P.g2h("dst", "ga")
        P.set("got", 0)
        lp, dn = L(), L()
        P.label(lp)
        P.br("geu", "got", "n", dn)
        P.mov("ph", "hh")
        P.add("pb", "dst", "got")
        P.sub("pn", "n", "got")
        c = L()
        P.br("leu", "pn", 1 << 30, c)
        P.set("pn", 1 << 30)
        P.label(c)
        if at:
            P.add("poff", "off", "got")
            P.call("pr_pread")
        else:
            P.call("pr_read")
        unless("ges", "r", 0, "NotFound", closer("hh"))
        P.br("eq", "r", 0, dn)
        P.add("got", "got", "r")
        P.jmp(lp)
        P.label(dn)
        P.mov("ph", "hh")
        P.call("pr_close")
        P.mov("st", "got")
        P.ret()

    read_object("pw_go", False)
    read_object("pw_readat", True)

    # ---- PUT / append / write at an offset (patra put)
    def write_object(label, mode):
        P.label(label)
        head(2, param=(mode == 2))
        P.rt("ga", "win_buf")
        P.mov("gn", "bl")
        P.call("pw_rd")
        unless("ne", "ok", 0, "BadAddress")
        P.g2h("body", "ga")
        root_and_form()
        P.set("follow", 1)
        P.call("pw_entry")
        unless("ne", "wr", 1, "NotWritten")
        unless("ne", "wr", 2, "Refused")
        unless("ne", "wr", 3, "NotWritten")
        unless("ne", "wr", 4, "Refused")
        if mode:
            unless("ne", "tk", 2, "IsADirectory")
        P.mov("pp", "tp")
        P.set("pm", mode)
        P.call("pr_open_write")
        unless("ges", "r", 0, "NotWritten")
        P.mov("hh", "r")
        P.set("got", 0)
        lp, dn = L(), L()
        P.label(lp)
        P.br("geu", "got", "bl", dn)
        P.mov("ph", "hh")
        P.add("pb", "body", "got")
        P.sub("pn", "bl", "got")
        c = L()
        P.br("leu", "pn", 1 << 30, c)
        P.set("pn", 1 << 30)
        P.label(c)
        if mode == 2:
            P.add("poff", "off", "got")
            P.call("pr_pwrite")
        else:
            P.call("pr_write")
        unless("gts", "r", 0, "NotWritten", closer("hh"))
        P.add("got", "got", "r")
        P.jmp(lp)
        P.label(dn)
        P.mov("ph", "hh")
        P.call("pr_close")
        P.mov("st", "bl")
        P.ret()

    write_object("pw_put", 0)
    write_object("pw_append", 1)
    write_object("pw_writeat", 2)

    # ---- delete a file
    P.label("pw_delete")
    head(1)
    root_and_form()
    P.set("follow", 0)
    P.call("pw_entry")
    unless("ne", "wr", 1, "NotFound")
    unless("ne", "wr", 2, "Refused")
    unless("ne", "wr", 3, "NotFound")
    unless("ne", "tk", 0, "NotFound")
    unless("ne", "tk", 2, "IsADirectory")
    P.mov("pp", "tp")
    P.call("pr_unlink")
    unless("ges", "r", 0, "NotWritten")
    P.set("st", 0)
    P.ret()
    # ---- create a directory
    P.label("pw_mkdir")
    head(1)
    root_and_form()
    P.set("follow", 0)
    P.call("pw_entry")
    unless("ne", "wr", 1, "NotWritten")
    unless("ne", "wr", 2, "Refused")
    unless("ne", "wr", 3, "NotWritten")
    unless("eq", "tk", 0, "AlreadyExists")
    P.mov("pp", "tp")
    P.call("pr_mkdir")
    unless("ges", "r", 0, "NotWritten")
    P.set("st", 0)
    P.ret()
    # ---- rename: PATH (from) to BUFFER (to, a path); never onto an existing name
    P.label("pw_rename")
    head(3)
    root_and_form()
    P.mov("sa0", "pa")
    P.mov("sn0", "pn")
    P.mov("pa", "qa")
    P.mov("pn", "qn")
    P.call("pw_form")
    unless("ne", "ok", 0, "Refused")
    P.mov("pa", "sa0")
    P.mov("pn", "sn0")
    P.set("follow", 0)
    P.call("pw_entry")
    unless("ne", "wr", 1, "NotFound")
    unless("ne", "wr", 2, "Refused")
    unless("ne", "wr", 3, "NotFound")
    unless("ne", "tk", 0, "NotFound")
    # the source's path, to p3
    P.buf("t0", "p2")
    P.buf("t1", "p3")
    lp, dn = L(), L()
    P.label(lp)
    P.ldm("t2", "t0", 0, 1)
    P.stm("t1", "t2", 0, 1)
    P.br("eq", "t2", 0, dn)
    P.add("t0", "t0", 1)
    P.add("t1", "t1", 1)
    P.jmp(lp)
    P.label(dn)
    P.mov("pa", "qa")
    P.mov("pn", "qn")
    P.call("pw_entry")
    unless("ne", "wr", 1, "NotWritten")
    unless("ne", "wr", 2, "Refused")
    unless("ne", "wr", 3, "NotWritten")
    unless("eq", "tk", 0, "AlreadyExists")
    P.buf("pp", "p3")
    P.mov("pq", "tp")
    P.call("pr_rename")
    unless("ges", "r", 0, "NotWritten")
    P.set("st", 0)
    P.ret()
    # ---- stat: BUFFER[0..8) the kind (1 file, 2 directory, 3 other), [8..16) a file's size
    P.label("pw_stat")
    head(2)
    root_and_form()
    P.call("pw_existing")
    unless("ne", "rc", 1, "NotFound")
    unless("ne", "rc", 2, "Refused")
    P.mov("ph", "h")
    P.call("pr_fkind")
    P.mov("t6", "r")
    P.mov("ph", "h")
    P.call("pr_close")
    unless("ges", "t6", 0, "NotFound")
    unless("geu", "bl", 16, "TooLarge")
    P.rt("ga", "win_buf")
    P.set("gn", 16)
    P.call("pw_wr")
    unless("ne", "ok", 0, "BadAddress")
    P.g2h("t0", "ga")
    P.stm("t0", "kind", 0, 8)
    P.stm("t0", "size", 8, 8)
    P.set("st", 16)
    P.ret()
    # ---- list a directory: the names (not "." or ".."), sorted by octets, zero-separated
    P.label("pw_list")
    head(2)
    root_and_form()
    P.call("pw_existing")
    unless("ne", "rc", 1, "NotFound")
    unless("ne", "rc", 2, "Refused")
    P.mov("ph", "h")
    P.call("pr_fkind")
    P.mov("k", "kind")
    P.mov("ph", "h")
    P.call("pr_close")
    unless("eq", "k", 2, "NotADirectory")
    # the BUFFER run must be writable for its whole header length BEFORE bl sizes anything: a
    # header near 2^64 would wrap the scratch size to one page (review B1, reproduced as SIGSEGV)
    P.rt("ga", "win_buf")
    P.mov("gn", "bl")
    P.call("pw_wr")
    unless("ne", "ok", 0, "BadAddress")
    # scratch: names (bl + 4096: the joined text never passes bl), then one word per name
    P.shr("t0", "bl", 1)
    P.add("t0", "t0", 2)
    P.shl("t0", "t0", 3)
    P.add("ar_n", "bl", 4096)
    P.and_("ar_n", "ar_n", -8)
    P.add("t0", "t0", "ar_n")
    P.add("t0", "t0", 0xFFF)
    P.and_("al_n", "t0", -0x1000)
    P.mov("pn", "al_n")
    P.call("pr_alloc")
    unless("ne", "r", 0, "TooLarge")
    P.mov("arena", "r")
    P.add("ptrs", "arena", "ar_n")
    P.mov("acur", "arena")
    P.set("cnt", 0)
    P.set("tot", 0)
    P.buf("pp", "rp")
    P.call("pr_dopen")
    P.mov("hd", "r")

    def list_cleanup():
        P.mov("ph", "hd")
        P.call("pr_dclose")
        P.mov("pb", "arena")
        P.mov("pn", "al_n")
        P.call("pr_free")

    if not win:                                     # Windows: a failed FindFirstFileW lists nothing
        unless("ges", "hd", 0, "NotFound", lambda: (P.mov("pb", "arena"), P.mov("pn", "al_n"),
                                                    P.call("pr_free")))
    lp, dn, take = L(), L(), L()
    P.label(lp)
    if win:
        P.br("eq", "hd", -1, dn)
    P.mov("ph", "hd")
    P.call("pr_dnext")
    P.br("eq", "dl", -1, dn)
    P.br("gtu", "dl", 2, take)
    P.br("eq", "dl", 0, lp)
    P.ldm("t0", "dn", 0, 1)
    P.br("ne", "t0", 0x2E, take)
    P.br("eq", "dl", 1, lp)
    P.ldm("t0", "dn", 1, 1)
    P.br("eq", "t0", 0x2E, lp)
    P.label(take)
    P.add("t1", "tot", "dl")
    first = L()
    P.br("eq", "cnt", 0, first)
    P.add("t1", "t1", 1)
    P.label(first)
    unless("leu", "t1", "bl", "TooLarge", list_cleanup)
    P.mov("tot", "t1")
    P.shl("t2", "cnt", 3)
    P.add("t2", "t2", "ptrs")
    P.stm("t2", "acur", 0, 8)
    P.add("cnt", "cnt", 1)
    P.set("t3", 0)
    cp, cpd = L(), L()
    P.label(cp)
    P.add("t4", "dn", "t3")
    P.ldm("t4", "t4", 0, 1)
    P.stm("acur", "t4", 0, 1)
    P.add("acur", "acur", 1)
    P.br("eq", "t4", 0, cpd)
    P.add("t3", "t3", 1)
    P.jmp(cp)
    P.label(cpd)
    P.jmp(lp)
    P.label(dn)
    # insertion sort of the name pointers, by unsigned octets (a prefix sorts first)
    P.set("i", 1)
    o1, o1d = L(), L()
    P.label(o1)
    P.br("geu", "i", "cnt", o1d)
    P.shl("t0", "i", 3)
    P.add("t0", "t0", "ptrs")
    P.ldm("x", "t0", 0, 8)
    P.mov("j", "i")
    i1, i1d = L(), L()
    P.label(i1)
    P.br("eq", "j", 0, i1d)
    P.shl("t0", "j", 3)
    P.add("t0", "t0", "ptrs")
    P.ldm("y", "t0", -8, 8)
    # compare y with x: y > x moves y up
    P.set("t3", 0)
    c1, gt = L(), L()
    P.label(c1)
    P.add("t4", "y", "t3")
    P.ldm("t4", "t4", 0, 1)
    P.add("t5", "x", "t3")
    P.ldm("t5", "t5", 0, 1)
    P.br("gtu", "t4", "t5", gt)
    P.br("ltu", "t4", "t5", i1d)
    P.br("eq", "t4", 0, i1d)                        # equal names (cannot happen): stop
    P.add("t3", "t3", 1)
    P.jmp(c1)
    P.label(gt)
    P.stm("t0", "y", 0, 8)
    P.sub("j", "j", 1)
    P.jmp(i1)
    P.label(i1d)
    P.shl("t0", "j", 3)
    P.add("t0", "t0", "ptrs")
    P.stm("t0", "x", 0, 8)
    P.add("i", "i", 1)
    P.jmp(o1)
    P.label(o1d)
    P.rt("ga", "win_buf")
    P.mov("gn", "tot")
    P.call("pw_wr")
    unless("ne", "ok", 0, "BadAddress", list_cleanup)
    P.g2h("dst", "ga")
    P.set("i", 0)
    o2, o2d = L(), L()
    P.label(o2)
    P.br("geu", "i", "cnt", o2d)
    sk = L()
    P.br("eq", "i", 0, sk)
    P.stm("dst", 0, 0, 1)
    P.add("dst", "dst", 1)
    P.label(sk)
    P.shl("t0", "i", 3)
    P.add("t0", "t0", "ptrs")
    P.ldm("t1", "t0", 0, 8)
    c2, c2d = L(), L()
    P.label(c2)
    P.ldm("t2", "t1", 0, 1)
    P.br("eq", "t2", 0, c2d)
    P.stm("dst", "t2", 0, 1)
    P.add("dst", "dst", 1)
    P.add("t1", "t1", 1)
    P.jmp(c2)
    P.label(c2d)
    P.add("i", "i", 1)
    P.jmp(o2)
    P.label(o2d)
    list_cleanup()
    P.mov("st", "tot")
    P.ret()


# ---- spawn: PARAM (0x20) is remembered like PATH and BUFFER; the store to
# SPAWN (0x78) runs PATH (a full path, no search) with the arguments in PARAM (zero-joined,
# argv[0] included), no stdin, stdout and stderr captured. STATUS = [exit code or an error
# word, stdout octets, stderr octets]; BUFFER = stdout then stderr. NATIVE ONLY.
WIN_SPAWN = 0x78                             # PARAM (0x20) is WIN_PARAM, defined with the file stores
ST_BAD_PATH, ST_SPAWN_FAILED = -3, -12
# the spawn's working words, one block (x86: the data symbol "spn"; A64: the frame's "spn")
SPN_WORDS = ["st", "s", "size", "cap", "lp", "lq", "path", "param", "buf", "z", "zq", "argv", "n", "cmd", "clen",
             "exit", "fail", "c0", "c1", "pid", "ans", "fds0", "fds1", "fds2", "pfd0", "pfd1", "pfd2", "ws", "one",
             "avail", "got", "hnul", "h0r", "h0w", "h1r", "h1w", "hpr", "hth", "wp", "wc", "sa0", "sa1", "sa2",
             "pi0", "pi1", "pi2", "lr", "lr2"]
SPN = {n: 8 * i for i, n in enumerate(SPN_WORDS)}
SPN["si"] = 8 * len(SPN_WORDS)                 # STARTUPINFOW, 104 octets
SPN_SIZE = SPN["si"] + 104


def runtime_spawn(A, base, win, mac):
    """x86-64: rt_w_spawn, an arm of rt_st_win (rcx = the STATUS run). The runs are bounded
    by runtime_window's rules (PATH and PARAM readable, BUFFER and all three STATUS words
    writable). In order: NotGranted without YANTRA_GRANT_SPAWN=1; BadAddress; BadPath (a
    zero inside PATH, PATH or PARAM not UTF-8, a quote in argv[0], which Windows cannot
    carry); SpawnFailed (no scratch, no pipe, no fork, or execve / CreateProcessW failed);
    TooLarge when stdout + stderr exceed BUFFER (BUFFER untouched); else the three words.
    The answer goes through rt_w_answer, which forgets PATH, BUFFER and PARAM."""
    def S(n, add=0):
        return ("abs", "spn", SPN[n] + add)

    def sysn(lin, dar):
        A.mov_rm_imm32(("r", RAX), 0x2000000 + dar if mac else lin)
        A.syscall()

    def onfail(label):
        """after a syscall: to label on an error (Darwin: the carry; Linux: negative)"""
        if mac:
            A.jcc("b", label)
        else:
            A.test_rm_r(("r", RAX), RAX)
            A.jcc("s", label)

    def ld32(r, rm):
        A.movsx(r, rm, 4)                              # an int (an fd), sign-extended

    def setb(v):
        A.raw(0xC6, 0x07, v)                           # mov byte [rdi], v

    A.label("rt_w_spawn")
    for r in (RBX, RSI, RDI, R8, R9, R10, R11):        # rt_w_answer's SAVE
        A.push(r)
    A.mov_rm_r(("abs", "win_st", 0), RCX)
    for r in (RBP, 12, 13, 14, 15):
        A.push(r)
    A.call("spn_main")
    for r in (15, 14, 13, 12, RBP):
        A.pop(r)
    A.jmp("rt_w_answer")

    # ---- dest: rcx = the stream (0 stdout, 1 stderr); rax = where its next octets go,
    # rdx = how many fit: its region (cap octets each) while it has room, else the discard
    # page (the count still grows; past cap the answer is TooLarge). Uses rax rdx.
    A.label("spn_dest")
    A.mov_r_rm(RAX, S("s"))
    A.mov_r_rm(RDX, S("c0"))
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("e", "spn_ds1")
    A.alu_r_rm("add", RAX, S("cap"))
    A.mov_r_rm(RDX, S("c1"))
    A.label("spn_ds1")
    A.alu_r_rm("cmp", RDX, S("cap"))
    A.jcc("ae", "spn_ds2")
    A.alu_r_rm("add", RAX, ("r", RDX))
    A.f7(3, ("r", RDX))                                # neg
    A.alu_r_rm("add", RDX, S("cap"))
    A.ret()
    A.label("spn_ds2")
    A.mov_r_rm(RAX, S("s"))
    A.alu_r_rm("add", RAX, S("cap"))
    A.alu_r_rm("add", RAX, S("cap"))
    A.mov_rm_imm32(("r", RDX), 4096)
    A.ret()

    # ---- main: every register free; rax = STATUS[0] (words 1 and 2 written on success)
    A.label("spn_main")
    A.mov_rm_imm32(S("ans"), ST_NOT_GRANTED)
    A.lea(RDI, ("abs", "env_spawn", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "spn_one")
    A.movzx(RCX, ("m", RAX, 0), 1)
    A.alu_rm_imm("cmp", ("r", RCX), 0x31)
    A.jcc("ne", "spn_one")
    A.movzx(RCX, ("m", RAX, 1), 1)
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "spn_one")                             # granted only by exactly "1"
    A.mov_rm_imm32(S("ans"), ST_BAD_ADDRESS)
    A.mov_r_rm(RSI, ("abs", "win_st", 0))
    A.mov_rm_imm32(("r", RAX), 24)
    A.call("rt_w_wr")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "spn_one")
    A.mov_rm_r(S("st"), RSI)
    for sym, ptr, ln, chk in (("win_path", "path", "lp", "rt_w_rd"), ("win_param", "param", "lq", "rt_w_rd"),
                              ("win_buf", "buf", "cap", "rt_w_wr")):
        A.mov_r_rm(RSI, ("abs", sym, 0))
        A.call("rt_w_hdr")
        A.test_rm_r(("r", RCX), RCX)
        A.jcc("ne", "spn_one")
        A.call(chk)
        A.test_rm_r(("r", RCX), RCX)
        A.jcc("ne", "spn_one")
        A.mov_rm_r(S(ptr), RSI)
        A.mov_rm_r(S(ln), RAX)
    # the scratch: stdout's region, stderr's (cap each), a discard page, then the copies,
    # argv and (Windows) the command lines: 2 cap + 8 KiB + 32 (lp + lq), in pages
    A.mov_rm_imm32(S("ans"), ST_SPAWN_FAILED)
    A.mov_r_rm(RSI, S("lp"))
    A.alu_r_rm("add", RSI, S("lq"))
    A.shift_imm("shl", ("r", RSI), 5)
    A.alu_r_rm("add", RSI, S("cap"))
    A.alu_r_rm("add", RSI, S("cap"))
    A.alu_rm_imm("add", ("r", RSI), 8192 + 4095)
    A.alu_rm_imm("and", ("r", RSI), -4096)
    A.mov_rm_r(S("size"), RSI)
    A.alu_r_rm("xor", RDI, ("r", RDI), 0)
    if win:
        A.call("w_alloc")                              # VirtualAlloc anywhere: an address, or 0
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "spn_one")
    else:
        A.mov_rm_imm32(("r", RDX), 3)
        A.mov_rm_imm32(("r", R10), 0x1002 if mac else 0x22)
        A.mov_rm_imm32(("r", R8), -1)
        A.alu_r_rm("xor", R9, ("r", R9), 0)
        sysn(9, 197)
        onfail("spn_one")
    A.mov_rm_r(S("s"), RAX)
    A.mov_r_rm(RDI, ("r", RAX))
    A.alu_r_rm("add", RDI, S("cap"))
    A.alu_r_rm("add", RDI, S("cap"))
    A.alu_rm_imm("add", ("r", RDI), 4096)
    A.mov_rm_r(S("z"), RDI)
    for ptr, ln in (("path", "lp"), ("param", "lq")):
        A.mov_r_rm(RSI, S(ptr))
        A.mov_r_rm(RCX, S(ln))
        A.raw(0xF3, 0xA4)                              # rep movsb
        setb(0)
        A.lea(RDI, ("m", RDI, 1))
        if ptr == "path":
            A.mov_rm_r(S("zq"), RDI)
    A.mov_r_rm(13, ("r", RDI))                         # r13 = the copies' end
    # BadPath: a zero inside the path, a copy that is not UTF-8, or a quote in argv[0]
    A.mov_rm_imm32(S("ans"), ST_BAD_PATH)
    A.mov_r_rm(RSI, S("z"))
    A.mov_r_rm(RCX, S("lp"))
    A.label("spn_pz")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("e", "spn_pz2")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "spn_free1")
    A.lea(RSI, ("m", RSI, 1))
    A.alu_rm_imm("sub", ("r", RCX), 1)
    A.jmp("spn_pz")
    A.label("spn_pz2")
    A.mov_r_rm(12, S("z"))
    A.label("spn_u8")
    A.alu_r_rm("cmp", 12, ("r", 13))
    A.jcc("ae", "spn_u8d")
    A.movzx(RAX, ("m", 12, 0), 1)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "spn_u8c")
    A.lea(12, ("m", 12, 1))
    A.jmp("spn_u8")
    A.label("spn_u8c")
    A.mov_r_rm(RSI, ("r", 12))
    A.call("rt_u8")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "spn_free1")
    A.alu_r_rm("add", 12, ("r", RDX))
    A.jmp("spn_u8")
    A.label("spn_u8d")
    A.mov_r_rm(RSI, S("zq"))
    A.label("spn_q0")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "spn_q0d")
    A.alu_rm_imm("cmp", ("r", RAX), 0x22)
    A.jcc("e", "spn_free1")
    A.lea(RSI, ("m", RSI, 1))
    A.jmp("spn_q0")
    A.label("spn_q0d")
    # argv: one pointer per zero-separated argument (an empty PARAM is one empty argument), then 0
    A.lea(12, ("m", 13, 7))
    A.alu_rm_imm("and", ("r", 12), -8)
    A.mov_rm_r(S("argv"), 12)
    A.mov_r_rm(RAX, S("zq"))
    A.mov_rm_r(("m", 12, 0), RAX)
    A.lea(15, ("m", 12, 8))
    A.mov_rm_imm32(("r", 14), 1)
    A.mov_r_rm(RSI, S("zq"))
    A.mov_r_rm(RCX, S("lq"))
    A.label("spn_av")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("e", "spn_avd")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.lea(RSI, ("m", RSI, 1))
    A.alu_rm_imm("sub", ("r", RCX), 1)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "spn_av")
    A.mov_rm_r(("m", 15, 0), RSI)
    A.lea(15, ("m", 15, 8))
    A.lea(14, ("m", 14, 1))
    A.jmp("spn_av")
    A.label("spn_avd")
    A.mov_rm_imm32(("m", 15, 0), 0)
    A.lea(15, ("m", 15, 8))
    A.mov_rm_r(S("n"), 14)
    if win:
        A.mov_rm_r(S("cmd"), 15)
        A.mov_r_rm(RDI, ("r", 15))
        A.call("spn_cmdline")
        A.alu_r_rm("sub", RDI, S("cmd"))
        A.mov_rm_r(S("clen"), RDI)
    for k in ("c0", "c1", "exit"):
        A.mov_rm_imm32(S(k), 0)
    A.mov_rm_imm32(S("fail"), 1)
    A.call("w_spawn" if win else "spn_launch")
    A.mov_rm_imm32(S("ans"), ST_SPAWN_FAILED)
    A.mov_r_rm(RAX, S("fail"))
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "spn_free1")
    A.mov_rm_imm32(S("ans"), ST_TOO_LARGE)
    A.mov_r_rm(RAX, S("c0"))
    A.alu_r_rm("add", RAX, S("c1"))
    A.alu_r_rm("cmp", RAX, S("cap"))
    A.jcc("a", "spn_free1")
    # it fits: stdout then stderr into BUFFER, then words 1 and 2; word 0 by rt_w_answer
    A.mov_r_rm(RDI, S("buf"))
    A.mov_r_rm(RSI, S("s"))
    A.mov_r_rm(RCX, S("c0"))
    A.raw(0xF3, 0xA4)
    A.mov_r_rm(RSI, S("s"))
    A.alu_r_rm("add", RSI, S("cap"))
    A.mov_r_rm(RCX, S("c1"))
    A.raw(0xF3, 0xA4)
    A.mov_r_rm(RDI, S("st"))
    for i, k in ((1, "c0"), (2, "c1")):
        A.mov_r_rm(RAX, S(k))
        A.mov_rm_r(("m", RDI, 8 * i), RAX)
    A.mov_r_rm(RAX, S("exit"))
    A.mov_rm_r(S("ans"), RAX)
    A.label("spn_free1")
    A.mov_r_rm(RDI, S("s"))
    if win:
        A.call("w_free")
    else:
        A.mov_r_rm(RSI, S("size"))
        sysn(11, 73)                                   # munmap
    A.label("spn_one")
    A.mov_r_rm(RAX, S("ans"))
    A.ret()

    if win:
        # ---- cmdline: argv (S(argv), S(n)) as one Windows command line at rdi, zero-ended;
        # rdi ends at the zero. win_join is the reference (argv[0] has no quote: checked).
        A.label("spn_cmdline")
        A.mov_r_rm(RBX, S("argv"))
        A.mov_r_rm(RSI, ("m", RBX, 0))
        A.movzx(RAX, ("m", RSI, 0), 1)
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "spn_c0q")
        A.mov_r_rm(RCX, ("r", RSI))
        A.label("spn_c0s")
        A.movzx(RAX, ("m", RCX, 0), 1)
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "spn_c0p")
        A.alu_rm_imm("cmp", ("r", RAX), 0x20)
        A.jcc("e", "spn_c0q")
        A.alu_rm_imm("cmp", ("r", RAX), 0x09)
        A.jcc("e", "spn_c0q")
        A.lea(RCX, ("m", RCX, 1))
        A.jmp("spn_c0s")
        A.label("spn_c0p")
        A.call("rt_copyz")
        A.jmp("spn_cargs")
        A.label("spn_c0q")
        setb(0x22)
        A.lea(RDI, ("m", RDI, 1))
        A.call("rt_copyz")
        setb(0x22)
        A.lea(RDI, ("m", RDI, 1))
        A.label("spn_cargs")
        A.mov_rm_imm32(("r", R8), 1)
        A.label("spn_ca")
        A.alu_r_rm("cmp", R8, S("n"))
        A.jcc("ae", "spn_cend")
        setb(0x20)
        A.lea(RDI, ("m", RDI, 1))
        A.mov_r_rm(RAX, ("r", R8))
        A.shift_imm("shl", ("r", RAX), 3)
        A.alu_r_rm("add", RAX, ("r", RBX))
        A.mov_r_rm(RSI, ("m", RAX, 0))
        A.lea(R8, ("m", R8, 1))
        A.movzx(RAX, ("m", RSI, 0), 1)
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "spn_cq")
        A.mov_r_rm(RCX, ("r", RSI))
        A.label("spn_cs")
        A.movzx(RAX, ("m", RCX, 0), 1)
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "spn_cp")
        for c in (0x20, 0x09, 0x22):
            A.alu_rm_imm("cmp", ("r", RAX), c)
            A.jcc("e", "spn_cq")
        A.lea(RCX, ("m", RCX, 1))
        A.jmp("spn_cs")
        A.label("spn_cp")
        A.call("rt_copyz")
        A.jmp("spn_ca")
        A.label("spn_cq")
        setb(0x22)
        A.lea(RDI, ("m", RDI, 1))
        A.alu_r_rm("xor", R9, ("r", R9), 0)            # backslashes pending
        A.label("spn_cql")
        A.movzx(RAX, ("m", RSI, 0), 1)
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "spn_cqe")
        A.lea(RSI, ("m", RSI, 1))
        A.alu_rm_imm("cmp", ("r", RAX), 0x5C)
        A.jcc("ne", "spn_cqn")
        A.lea(R9, ("m", R9, 1))
        A.jmp("spn_cql")
        A.label("spn_cqn")
        A.alu_rm_imm("cmp", ("r", RAX), 0x22)
        A.jcc("ne", "spn_cqc")
        A.alu_r_rm("add", R9, ("r", R9))
        A.lea(R9, ("m", R9, 1))                        # 2b + 1 before a quote
        A.label("spn_cqc")
        A.call("spn_bs")
        A.raw(0x88, 0x07)                              # mov [rdi], al
        A.lea(RDI, ("m", RDI, 1))
        A.jmp("spn_cql")
        A.label("spn_cqe")
        A.alu_r_rm("add", R9, ("r", R9))               # 2b before the closing quote
        A.call("spn_bs")
        setb(0x22)
        A.lea(RDI, ("m", RDI, 1))
        A.jmp("spn_ca")
        A.label("spn_cend")
        setb(0)
        A.ret()
        A.label("spn_bs")                              # r9 backslashes to rdi; r9 = 0; rax kept
        A.test_rm_r(("r", R9), R9)
        A.jcc("e", "spn_bs2")
        setb(0x5C)
        A.lea(RDI, ("m", RDI, 1))
        A.alu_rm_imm("sub", ("r", R9), 1)
        A.jmp("spn_bs")
        A.label("spn_bs2")
        A.ret()
    else:
        # ---- launch (Linux, Darwin): three pipes (stdout, stderr, and one that carries a
        # failed execve's octet), all close-on-exec; fork; the child takes /dev/null and
        # the two pipes and execve's the program with the runtime's envp; the parent polls
        # the three read ends to their ends (neither stream can block the other), then
        # wait4. S(fail) = 0 once the child exists, 1 again if its execve failed.
        A.label("spn_launch")
        for k in range(3):
            A.mov_rm_imm32(S("fds%d" % k), -1)
        for k in range(3):
            if mac:
                sysn(0, 42)                            # pipe: rax the read end, rdx the write end
                onfail("spn_lf")
                A.mov_rm_r(S("fds%d" % k), RAX, 0, 4)
                A.mov_rm_r(S("fds%d" % k, 4), RDX, 0, 4)
                for half in (0, 4):
                    ld32(RDI, S("fds%d" % k, half))
                    A.mov_rm_imm32(("r", RSI), 2)      # F_SETFD
                    A.mov_rm_imm32(("r", RDX), 1)      # FD_CLOEXEC
                    sysn(0, 92)
            else:
                A.lea(RDI, S("fds%d" % k))
                A.mov_rm_imm32(("r", RSI), 0x80000)    # O_CLOEXEC
                sysn(293, 0)
                onfail("spn_lf")
        sysn(57, 2)                                    # fork
        onfail("spn_lf")
        if mac:
            A.test_rm_r(("r", RDX), RDX)               # Darwin: rdx = 1 in the child
            A.jcc("ne", "spn_child")
        else:
            A.test_rm_r(("r", RAX), RAX)
            A.jcc("e", "spn_child")
        A.mov_rm_r(S("pid"), RAX)
        A.mov_rm_imm32(S("fail"), 0)
        for k in range(3):
            ld32(RDI, S("fds%d" % k, 4))
            sysn(3, 6)                                 # close the write ends
            ld32(RAX, S("fds%d" % k))
            A.mov_rm_r(S("pfd%d" % k), RAX, 0, 4)
            A.mov_rm_imm32(S("pfd%d" % k, 4), 1, 0)    # events POLLIN, revents 0
        A.label("spn_poll")
        A.mov_rm_imm32(("r", RAX), -1, 0)
        for k in range(3):
            A.alu_r_rm("and", RAX, S("pfd%d" % k), 0)  # all three closed: their AND is negative
        A.test_rm_r(("r", RAX), RAX, 0)
        A.jcc("s", "spn_wait")
        A.lea(RDI, S("pfd0"))
        A.mov_rm_imm32(("r", RSI), 3)
        A.mov_rm_imm32(("r", RDX), -1)
        sysn(7, 230)
        if mac:
            A.jcc("ae", "spn_pok")
            A.alu_rm_imm("cmp", ("r", RAX), 4)         # EINTR
        else:
            A.test_rm_r(("r", RAX), RAX)
            A.jcc("ns", "spn_pok")
            A.alu_rm_imm("cmp", ("r", RAX), -4)
        A.jcc("e", "spn_poll")
        A.jmp("spn_wait")
        A.label("spn_pok")
        for k in range(3):
            nxt = "spn_pn%d" % k
            ld32(RAX, S("pfd%d" % k))
            A.test_rm_r(("r", RAX), RAX)
            A.jcc("s", nxt)
            A.movzx(RAX, S("pfd%d" % k, 6), 2)
            A.test_rm_r(("r", RAX), RAX)
            A.jcc("e", nxt)
            if k < 2:
                A.mov_rm_imm32(("r", RCX), k)
                A.call("spn_dest")
                A.mov_r_rm(RSI, ("r", RAX))
            else:
                A.lea(RSI, S("one"))
                A.mov_rm_imm32(("r", RDX), 8)
            ld32(RDI, S("pfd%d" % k))
            sysn(0, 3)
            if mac:
                A.jcc("b", "spn_pc%d" % k)
            A.test_rm_r(("r", RAX), RAX)
            A.jcc("le", "spn_pc%d" % k)
            if k < 2:
                A.alu_r_rm("add", RAX, S("c%d" % k))
                A.mov_rm_r(S("c%d" % k), RAX)
            else:
                A.mov_rm_imm32(S("fail"), 1)
            A.jmp(nxt)
            A.label("spn_pc%d" % k)
            ld32(RDI, S("pfd%d" % k))
            sysn(3, 6)
            A.mov_rm_imm32(S("pfd%d" % k), -1, 0)
            A.label(nxt)
        A.jmp("spn_poll")
        A.label("spn_wait")
        A.mov_r_rm(RDI, S("pid"))
        A.lea(RSI, S("ws"))
        A.alu_r_rm("xor", RDX, ("r", RDX), 0)
        A.alu_r_rm("xor", R10, ("r", R10), 0)
        sysn(61, 7)
        if mac:
            A.jcc("ae", "spn_wok")
            A.alu_rm_imm("cmp", ("r", RAX), 4)
        else:
            A.test_rm_r(("r", RAX), RAX)
            A.jcc("ns", "spn_wok")
            A.alu_rm_imm("cmp", ("r", RAX), -4)
        A.jcc("e", "spn_wait")
        A.label("spn_wok")
        # exited: (ws >> 8) & 255; killed by signal s: 128 + s
        A.mov_r_rm(RAX, S("ws"), 0)
        A.mov_r_rm(RCX, ("r", RAX), 0)
        A.alu_rm_imm("and", ("r", RCX), 0x7F)
        A.jcc("ne", "spn_sig")
        A.shift_imm("shr", ("r", RAX), 8)
        A.alu_rm_imm("and", ("r", RAX), 0xFF)
        A.mov_rm_r(S("exit"), RAX)
        A.ret()
        A.label("spn_sig")
        A.alu_rm_imm("add", ("r", RCX), 128)
        A.mov_rm_r(S("exit"), RCX)
        A.ret()
        A.label("spn_lf")                              # a pipe or the fork failed: close what is open
        for k in range(3):
            for half in (0, 4):
                lab = "spn_lf%d%d" % (k, half)
                ld32(RDI, S("fds%d" % k, half))
                A.test_rm_r(("r", RDI), RDI)
                A.jcc("s", lab)
                sysn(3, 6)
                A.label(lab)
        A.ret()
        # the child: nothing here returns
        A.label("spn_child")
        for k, fd in ((0, 1), (1, 2)):
            ld32(RDI, S("fds%d" % k, 4))
            A.mov_rm_imm32(("r", RSI), fd)
            sysn(33, 90)                               # dup2
        A.lea(RDI, ("abs", "s_devnull", 0))
        A.alu_r_rm("xor", RSI, ("r", RSI), 0)
        A.alu_r_rm("xor", RDX, ("r", RDX), 0)
        sysn(2, 5)                                     # open /dev/null
        A.mov_r_rm(RDI, ("r", RAX))
        A.alu_r_rm("xor", RSI, ("r", RSI), 0)
        sysn(33, 90)
        # the environment WITHOUT YANTRA_*: a grant (YANTRA_GRANT_*, YANTRA_FILES) is this
        # process's, never its child's (review M1). The child's own copy of envp is
        # compacted in place, octet by octet (a short string is never read past its zero).
        A.mov_r_rm(RSI, ("abs", "envp", 0))
        A.mov_r_rm(RDI, ("r", RSI))
        A.label("spn_ev")
        A.mov_r_rm(RAX, ("m", RSI, 0))
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "spn_evd")
        A.alu_rm_imm("add", ("r", RSI), 8)
        for k, ch in enumerate(b"YANTRA_"):
            A.movzx(RCX, ("m", RAX, k), 1)
            A.alu_rm_imm("cmp", ("r", RCX), ch)
            A.jcc("ne", "spn_evk")
        A.jmp("spn_ev")
        A.label("spn_evk")
        A.mov_rm_r(("m", RDI, 0), RAX)
        A.alu_rm_imm("add", ("r", RDI), 8)
        A.jmp("spn_ev")
        A.label("spn_evd")
        A.mov_rm_imm32(("m", RDI, 0), 0)
        A.mov_r_rm(RDI, S("z"))
        A.mov_r_rm(RSI, S("argv"))
        A.mov_r_rm(RDX, ("abs", "envp", 0))
        sysn(59, 59)                                   # execve
        ld32(RDI, S("fds2", 4))                        # it failed: one octet says so
        A.lea(RSI, S("one"))
        A.mov_rm_imm32(("r", RDX), 1)
        sysn(1, 4)
        A.mov_rm_imm32(("r", RDI), 127)
        sysn(60, 1)
    A.label("env_spawn")
    A.raw(b"YANTRA_GRANT_SPAWN=\0")
    A.label("s_devnull")
    A.raw(b"/dev/null\0")


def runtime_win_spawn(A):
    """x86_64-windows: the spawn's shims. w_spawn reads and writes the spn words only:
    the program (S(z)) and the command line (S(cmd)) to UTF-16, two pipes (their read ends
    not inherited), NUL for stdin, CreateProcessW with STARTF_USESTDHANDLES and inherited
    handles; both pipes are drained with PeekNamedPipe + ReadFile (a pipe with nothing
    waiting is never read, so neither stream can block the other; with nothing to read it
    waits on the process for 1 ms); then WaitForSingleObject and GetExitCodeProcess.
    w_free(rdi): VirtualFree. runtime_win's register contract; rbx rsi rdi r12..r15 are
    free here (rt_w_spawn saved them)."""
    SAVED = (RDX, R8, R9, R10, R11, RBP)
    R12 = 12

    def S(n, add=0):
        return ("abs", "spn", SPN[n] + add)

    def enter(label, extra=0):
        A.label(label)
        for r in SAVED:
            A.push(r)
        A.mov_r_rm(RBP, ("r", RSP))
        A.alu_rm_imm("and", ("r", RSP), -16)
        A.alu_rm_imm("sub", ("r", RSP), 80)
        A.op(b"\x0f\xae", 3, ("m", RSP, 64), 0)      # stmxcsr
        if extra:
            A.alu_rm_imm("sub", ("r", RSP), extra)

    def leave(extra=0):
        if extra:
            A.alu_rm_imm("add", ("r", RSP), extra)
        A.op(b"\x0f\xae", 2, ("m", RSP, 64), 0)      # ldmxcsr
        A.mov_r_rm(RSP, ("r", RBP))
        for r in reversed(SAVED):
            A.pop(r)
        A.ret()

    def imp(name):
        A.mov_r_rm(RAX, ("abs", "iat_" + name, 0))
        A.raw(0x40, 0xFF, 0xD0)                       # call rax

    def arg(i, v):
        """stack argument i (the 5th is 4) = v: a register, or ('i', imm32)"""
        if isinstance(v, tuple):
            A.mov_rm_imm32(("m", RSP, 8 * i), v[1])
        else:
            A.mov_rm_r(("m", RSP, 8 * i), v)

    def close(h, lab):
        A.mov_r_rm(RCX, S(h))
        A.test_rm_r(("r", RCX), RCX)
        A.jcc("e", lab)
        imp("CloseHandle")
        A.mov_rm_imm32(S(h), 0)
        A.label(lab)

    X = 96                                            # room for CreateProcessW's ten arguments
    enter("w_spawn", X)
    for h in ("hnul", "h0r", "h0w", "h1r", "h1w", "hpr", "hth"):
        A.mov_rm_imm32(S(h), 0)
    # the UTF-16 copies: wp after the command line, wc after wp
    A.mov_r_rm(RAX, S("cmd"))
    A.alu_r_rm("add", RAX, S("clen"))
    A.alu_rm_imm("add", ("r", RAX), 8)
    A.alu_rm_imm("and", ("r", RAX), -8)
    A.mov_rm_r(S("wp"), RAX)
    A.mov_r_rm(RCX, S("lp"))
    A.lea(RCX, ("m", RCX, 8))
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.alu_rm_imm("and", ("r", RAX), -8)
    A.mov_rm_r(S("wc"), RAX)
    for src, dst, ln in (("z", "wp", "lp"), ("cmd", "wc", "clen")):
        A.mov_rm_imm32(("r", RCX), 65001)              # CP_UTF8
        A.alu_r_rm("xor", RDX, ("r", RDX), 0)
        A.mov_r_rm(R8, S(src))
        A.mov_rm_imm32(("r", R9), -1)
        A.mov_r_rm(RAX, S(dst))
        arg(4, RAX)
        A.mov_r_rm(RAX, S(ln))
        A.lea(RAX, ("m", RAX, 1))
        arg(5, RAX)
        imp("MultiByteToWideChar")
        A.test_rm_r(("r", RAX), RAX, 0)
        A.jcc("e", "w_sp_fail")
    A.mov_rm_imm32(S("sa0"), 24)                       # SECURITY_ATTRIBUTES: inherited
    A.mov_rm_imm32(S("sa1"), 0)
    A.mov_rm_imm32(S("sa2"), 1)
    for k in range(2):
        A.lea(RCX, S("h%dr" % k))
        A.lea(RDX, S("h%dw" % k))
        A.lea(R8, S("sa0"))
        A.alu_r_rm("xor", R9, ("r", R9), 0)
        imp("CreatePipe")
        A.test_rm_r(("r", RAX), RAX, 0)
        A.jcc("e", "w_sp_fail")
        A.mov_r_rm(RCX, S("h%dr" % k))
        A.mov_rm_imm32(("r", RDX), 1)                  # HANDLE_FLAG_INHERIT
        A.alu_r_rm("xor", R8, ("r", R8), 0)
        imp("SetHandleInformation")
    A.lea(RCX, ("abs", "w_nul", 0))
    A.mov_rm_imm32(("r", RDX), -0x8000_0000, 0)        # GENERIC_READ
    A.mov_rm_imm32(("r", R8), 3)                       # FILE_SHARE_READ | FILE_SHARE_WRITE
    A.lea(R9, S("sa0"))
    arg(4, ("i", 3))                                   # OPEN_EXISTING
    arg(5, ("i", 0))
    arg(6, ("i", 0))
    imp("CreateFileW")
    A.mov_rm_imm32(("r", RCX), -1)
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("e", "w_sp_fail")
    A.mov_rm_r(S("hnul"), RAX)
    A.lea(RDI, S("si"))
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.mov_rm_imm32(("r", RCX), 104)
    A.raw(0xF3, 0xAA)                                  # rep stosb
    A.mov_rm_imm32(S("si"), 104, 0)                    # cb
    A.mov_rm_imm32(S("si", 60), 0x100, 0)              # STARTF_USESTDHANDLES
    for off, h in ((80, "hnul"), (88, "h0w"), (96, "h1w")):
        A.mov_r_rm(RAX, S(h))
        A.mov_rm_r(S("si", off), RAX)
    A.mov_r_rm(RCX, S("wp"))
    A.mov_r_rm(RDX, S("wc"))
    A.alu_r_rm("xor", R8, ("r", R8), 0)
    A.alu_r_rm("xor", R9, ("r", R9), 0)
    arg(4, ("i", 1))                                   # bInheritHandles
    arg(5, ("i", 0))
    arg(6, ("i", 0))
    arg(7, ("i", 0))
    A.lea(RAX, S("si"))
    arg(8, RAX)
    A.lea(RAX, S("pi0"))
    arg(9, RAX)
    imp("CreateProcessW")
    A.test_rm_r(("r", RAX), RAX, 0)
    A.jcc("e", "w_sp_fail")
    A.mov_r_rm(RAX, S("pi0"))
    A.mov_rm_r(S("hpr"), RAX)
    A.mov_r_rm(RAX, S("pi1"))
    A.mov_rm_r(S("hth"), RAX)
    for h in ("h0w", "h1w", "hnul", "hth"):
        close(h, "w_sp_cl_" + h)
    A.mov_rm_imm32(S("fail"), 0)
    A.label("w_sp_loop")
    A.alu_r_rm("xor", R12, ("r", R12), 0)              # progress
    for k in range(2):
        nxt, cl = "w_sp_n%d" % k, "w_sp_c%d" % k
        A.mov_r_rm(RCX, S("h%dr" % k))
        A.test_rm_r(("r", RCX), RCX)
        A.jcc("e", nxt)
        A.alu_r_rm("xor", RDX, ("r", RDX), 0)
        A.alu_r_rm("xor", R8, ("r", R8), 0)
        A.alu_r_rm("xor", R9, ("r", R9), 0)
        A.mov_rm_imm32(S("avail"), 0)
        A.lea(RAX, S("avail"))
        arg(4, RAX)
        arg(5, ("i", 0))
        imp("PeekNamedPipe")
        A.test_rm_r(("r", RAX), RAX, 0)
        A.jcc("e", cl)                                 # broken: the child's ends are all closed
        A.mov_r_rm(RAX, S("avail"))
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", nxt)
        A.mov_rm_imm32(("r", RCX), k)
        A.call("spn_dest")
        A.alu_r_rm("cmp", RDX, S("avail"))
        A.jcc("be", "w_sp_r%d" % k)
        A.mov_r_rm(RDX, S("avail"))
        A.label("w_sp_r%d" % k)
        A.mov_r_rm(R8, ("r", RDX))
        A.mov_r_rm(RDX, ("r", RAX))
        A.mov_r_rm(RCX, S("h%dr" % k))
        A.mov_rm_imm32(S("got"), 0)
        A.lea(R9, S("got"))
        arg(4, ("i", 0))
        imp("ReadFile")
        A.test_rm_r(("r", RAX), RAX, 0)
        A.jcc("e", cl)
        A.mov_r_rm(RAX, S("got"))
        A.alu_r_rm("add", RAX, S("c%d" % k))
        A.mov_rm_r(S("c%d" % k), RAX)
        A.mov_rm_imm32(("r", R12), 1)
        A.jmp(nxt)
        A.label(cl)
        close("h%dr" % k, "w_sp_cr%d" % k)
        A.label(nxt)
    A.mov_r_rm(RAX, S("h0r"))
    A.alu_r_rm("or", RAX, S("h1r"))
    A.jcc("e", "w_sp_done")
    A.test_rm_r(("r", R12), R12)
    A.jcc("ne", "w_sp_loop")
    A.mov_r_rm(RCX, S("hpr"))
    A.mov_rm_imm32(("r", RDX), 1)
    imp("WaitForSingleObject")
    A.jmp("w_sp_loop")
    A.label("w_sp_done")
    A.mov_r_rm(RCX, S("hpr"))
    A.mov_rm_imm32(("r", RDX), -1)                     # INFINITE
    imp("WaitForSingleObject")
    A.mov_rm_imm32(S("exit"), 0)
    A.mov_r_rm(RCX, S("hpr"))
    A.lea(RDX, S("exit"))
    imp("GetExitCodeProcess")
    close("hpr", "w_sp_cl_hpr")
    leave(X)
    A.label("w_sp_fail")
    for h in ("h0r", "h0w", "h1r", "h1w", "hnul"):
        close(h, "w_sp_f_" + h)
    leave(X)
    # ---- free(rdi = an address from w_alloc)
    enter("w_free")
    A.mov_r_rm(RCX, ("r", RDI))
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.mov_rm_imm32(("r", R8), 0x8000)                  # MEM_RELEASE
    imp("VirtualFree")
    leave()
    # no alignment: x86 reads the UTF-16 string at any address, and a recorded part must not depend on where it lands
    A.label("w_nul")
    A.raw("NUL\0".encode("utf-16-le"))


def runtime(A, M, base, entry, extent, filesz, code_size, prot_size, mac=False):
    """The host layer, in machine code. Linux x86-64 syscalls: 0 read, 1 write, 2 open,
    3 close, 5 fstat, 9 mmap, 60 exit. Registers it may clobber: all of them before the
    guest starts; after, only rax/rcx/rdx unless it saves the rest.
    mac: x86-64 Darwin: syscall with rax = 0x2000000 + (3 read, 4 write, 5 open, 339 fstat64,
    197 mmap, 1 exit); an error sets the carry (rax = errno); rdx may be written too. Entered
    from dyld through LC_MAIN: rdi argc, rsi argv, rdx envp.
    mac="win": x86_64-windows, Linux's paths with every syscall a CALL to a shim (runtime_win)."""
    win = mac == "win"
    mac = mac is True

    def sc(n):
        return 0x2000000 + MACSYS[n] if mac else n
    def sys_(n):
        x86_sys(A, n, "win" if win else mac)

    def msg(label, text):
        # write(2, text, len) then fall into nothing; callers jump here
        A.label(label)
        A.lea(RSI, ("abs", label + "_s", 0))
        A.mov_rm_imm32(("r", RDX), len(text))
        A.mov_rm_imm32(("r", RDI), 2)
        sys_(1)
        A.ret()
        strings.append((label + "_s", text))

    strings = []
    # ---- _start
    A.label("rt_start")
    if win:
        A.alu_rm_imm("and", ("r", RSP), -16)
        A.call("w_init")                        # argc, argv, envp from kernel32
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "rt_no_ram")
    elif mac:
        A.mov_rm_r(("abs", "argc", 0), RDI)     # LC_MAIN: argc, argv, envp in rdi rsi rdx
        A.mov_rm_r(("abs", "argv", 0), RSI)
        A.mov_rm_r(("abs", "envp", 0), RDX)
    else:
        A.mov_r_rm(RAX, ("m", RSP, 0))
        A.mov_rm_r(("abs", "argc", 0), RAX)
        A.lea(RCX, ("m", RSP, 8))
        A.mov_rm_r(("abs", "argv", 0), RCX)
        A.lea(RCX, ("m", RCX, 8))   # envp = argv + 8*(argc+1)
        A.shift_imm("shl", ("r", RAX), 3)
        A.alu_r_rm("add", RCX, ("r", RAX))
        A.mov_rm_r(("abs", "envp", 0), RCX)
    A.alu_rm_imm("and", ("r", RSP), -16)
    # ram = YANTRA_RAM, or max(DEFAULT_RAM, extent + HEADROOM)
    ram_default = max(DEFAULT_RAM, extent + RAM_HEADROOM)
    A.mov_r_imm64(RAX, ram_default)
    A.mov_rm_r(("abs", "ram", 0), RAX)
    A.lea(RDI, ("abs", "env_ram", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_s1")
    A.mov_r_rm(RSI, ("r", RAX))
    A.call("rt_atou_ram")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "rt_s1")                      # not a usize: the default, as yantra-run
    A.mov_rm_r(("abs", "ram", 0), RAX)
    A.label("rt_s1")
    # yantra's loader: the first segment of seg_tab (header order) that RAM cannot hold refuses the run
    A.lea(RBX, ("abs", "seg_tab", 0))
    A.mov_r_rm(12, ("m", RBX, 0))
    A.lea(RBX, ("m", RBX, 8))
    A.label("rt_sc1")
    A.test_rm_r(("r", 12), 12)
    A.jcc("e", "rt_sc2")
    A.mov_r_rm(RAX, ("m", RBX, 0))
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("sub", RAX, ("r", RCX))
    A.alu_r_rm("add", RAX, ("m", RBX, 8))
    A.mov_r_rm(RCX, ("abs", "ram", 0))
    A.alu_r_rm("cmp", RCX, ("r", RAX))
    A.jcc("b", "rt_seg_fail")                # ram < vaddr - base + need
    A.lea(RBX, ("m", RBX, 16))
    A.alu_rm_imm("sub", ("r", 12), 1)
    A.jmp("rt_sc1")
    A.label("rt_sc2")
    # YANTRA_RECORD_EVENTS=<log>: the clock log, yantra-run --record-events's format; rec_fd = fd + 1
    A.lea(RDI, ("abs", "env_rec", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_s1b")
    A.mov_r_rm(RDI, ("r", RAX))
    if mac:
        A.mov_rm_imm32(("r", RSI), 0x601)                   # O_WRONLY | O_CREAT | O_TRUNC
    else:
        A.mov_rm_imm32(("r", RSI), 0x241)
    A.mov_rm_imm32(("r", RDX), 420)
    sys_(2)
    if mac:
        A.jcc("b", "rt_no_rec")
    else:
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("s", "rt_no_rec")
    A.lea(RDI, ("m", RAX, 1))
    A.mov_rm_r(("abs", "rec_fd", 0), RDI)
    A.mov_r_rm(RDI, ("r", RAX))
    A.lea(RSI, ("abs", "rec_hdr", 0))
    A.mov_rm_imm32(("r", RDX), REC_HEADER_LEN)
    sys_(1)
    A.label("rt_s1b")
    # the input file: open + fstat for its length
    A.mov_rm_imm32(("abs", "in_fd", 0), -1)
    A.mov_rm_imm32(("abs", "in_len", 0), 0)
    A.mov_rm_imm32(("abs", "name_len", 0), 0)
    A.lea(RDI, ("abs", "env_input", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_s2")
    A.mov_r_rm(RDI, ("r", RAX))
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)
    sys_(2)
    if mac:
        A.jcc("b", "rt_no_input")
    else:
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("s", "rt_no_input")
    A.mov_rm_r(("abs", "in_fd", 0), RAX)
    A.mov_r_rm(RDI, ("r", RAX))
    if win:
        A.call("w_size")
    else:
        A.lea(RSI, ("abs", "statbuf", 0))
        sys_(5)
    if win:
        pass
    elif mac:
        A.mov_r_rm(RAX, ("abs", "statbuf", 96))     # st_size (struct stat64)
    else:
        A.mov_r_rm(RAX, ("abs", "statbuf", 48))     # st_size
    A.mov_rm_r(("abs", "in_len", 0), RAX)
    # YANTRA_INPUT_ENTRY="<module> <routine>" (yantra-run): the name run is module NUL routine
    A.lea(RDI, ("abs", "env_entry", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_entry")
    A.lea(RDI, ("abs", "env_name", 0))
    A.call("rt_getenv")
    A.mov_rm_r(("abs", "name_ptr", 0), RAX)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_no_name")                  # neither a name nor an entry: refused, as yantra-run
    A.mov_r_rm(RSI, ("r", RAX))
    A.call("rt_strlen")
    A.mov_rm_r(("abs", "name_len", 0), RAX)
    A.label("rt_s2")
    # the arguments: arg0 (yantra's image path) and argv[1..], joined by zero octets
    A.call("rt_args_len")
    A.mov_rm_r(("abs", "args_len", 0), RAX)
    # mmap(base, ram + in_len + name_len + args_len + 64, RW, PRIVATE|ANON|FIXED_NOREPLACE)
    A.mov_r_rm(RSI, ("abs", "ram", 0))
    A.alu_r_rm("add", RSI, ("abs", "in_len", 0))
    A.alu_r_rm("add", RSI, ("abs", "name_len", 0))
    A.alu_r_rm("add", RSI, ("abs", "args_len", 0))
    A.alu_rm_imm("add", ("r", RSI), 64 + 0xFFF)
    A.alu_rm_imm("and", ("r", RSI), -0x1000)
    A.mov_r_imm64(RDI, base)
    if win:
        A.call("w_alloc")                             # VirtualAlloc at base: base, or 0
    else:
        A.mov_rm_imm32(("r", RDX), 3)
        if mac:
            A.mov_rm_imm32(("r", R10), 0x1002)            # MAP_ANON | MAP_PRIVATE: the address is a hint, checked
        else:
            A.mov_rm_imm32(("r", R10), 0x22 | 0x100000)
        A.mov_rm_imm32(("r", R8), -1)
        A.alu_r_rm("xor", R9, ("r", R9), 0)
        sys_(9)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("ne", "rt_no_ram")
    # copy the image
    A.mov_r_imm64(RDI, base)
    A.lea(RSI, ("abs", "image", 0))
    A.mov_rm_imm32(("r", RCX), filesz)
    A.raw(0xF3, 0xA4)                              # rep movsb
    # top = ram; inject the input (yantra input.rs order: input, then arguments)
    A.mov_r_rm(RAX, ("abs", "ram", 0))
    A.mov_rm_r(("abs", "top", 0), RAX)
    A.mov_r_rm(RAX, ("abs", "in_fd", 0))
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_s3")
    for tag in (0x5455_504E_4953_4153, 0x454D_414E_4953_4153, 0x4543_4152_5453_4153):
        A.mov_r_imm64(RAX, tag)
        A.call("rt_find_tag")
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "rt_no_tag")
        A.push(RAX)
    # the source run: its storage, then read() the file straight into it
    A.call("rt_run_open")            # rax = storage address; rdi kept = storage
    A.mov_r_rm(RDX, ("abs", "in_len", 0))
    A.mov_rm_r(("m", RAX, -8), RDX)
    A.mov_r_rm(RSI, ("r", RAX))
    A.push(RAX)
    A.label("rt_read")
    A.mov_r_rm(RDX, ("abs", "in_len", 0))
    A.alu_r_rm("add", RDX, ("m", RSP, 8 * 0))     # end = storage + len
    A.alu_r_rm("sub", RDX, ("r", RSI))
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "rt_read_done")
    A.mov_r_rm(RDI, ("abs", "in_fd", 0))
    A.push(RSI)
    if win:
        A.call("w_sys0")
    elif mac:
        A.mov_rm_imm32(("r", RAX), sc(0))
        A.syscall()
    else:
        A.alu_r_rm("xor", RAX, ("r", RAX), 0)
        A.syscall()
    A.pop(RSI)
    if mac:
        A.jcc("b", "rt_no_input")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("le", "rt_no_input")
    A.alu_r_rm("add", RSI, ("r", RAX))
    A.jmp("rt_read")
    A.label("rt_read_done")
    A.pop(RAX)                      # storage of the source run
    A.mov_r_rm(RCX, ("abs", "in_len", 0))
    A.alu_r_rm("add", RCX, ("r", RAX))
    A.mov_r_imm64(RDX, base)
    A.alu_r_rm("sub", RCX, ("r", RDX))
    A.mov_rm_r(("abs", "top", 0), RCX)
    A.pop(RCX)          # trace tag at
    A.pop(RDX)          # name tag at
    A.pop(RSI)          # input tag at
    A.mov_rm_r(("m", RSI, 8), RAX)
    A.push(RDX)
    A.push(RCX)
    # the name run
    A.call("rt_run_open")
    A.mov_r_rm(RCX, ("abs", "name_len", 0))
    A.mov_rm_r(("m", RAX, -8), RCX)
    A.mov_r_rm(RDI, ("r", RAX))
    A.mov_r_rm(RSI, ("abs", "name_ptr", 0))
    A.raw(0xF3, 0xA4)
    A.mov_r_rm(RCX, ("abs", "name_len", 0))
    A.alu_r_rm("add", RCX, ("r", RAX))
    A.mov_r_imm64(RDX, base)
    A.alu_r_rm("sub", RCX, ("r", RDX))
    A.mov_rm_r(("abs", "top", 0), RCX)
    A.pop(RCX)
    A.pop(RDX)
    A.mov_rm_r(("m", RDX, 8), RAX)
    # the trace word: YANTRA_INPUT_TRACE or 0
    A.push(RCX)
    A.lea(RDI, ("abs", "env_trace", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_s4")
    A.mov_r_rm(RSI, ("r", RAX))
    A.call("rt_atou")
    A.label("rt_s4")
    A.pop(RCX)
    A.mov_rm_r(("m", RCX, 8), RAX)
    A.label("rt_s3")
    # arguments, when the image declares both tags
    A.mov_r_imm64(RAX, 0x0056_4752_4153_4153)
    A.call("rt_find_tag")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_s5")
    A.push(RAX)
    A.mov_r_imm64(RAX, 0x0043_4752_4153_4153)
    A.call("rt_find_tag")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_s5p")
    A.push(RAX)
    A.call("rt_run_open")
    A.mov_r_rm(RDI, ("r", RAX))
    A.push(RAX)
    # arg0, then argv[1..]
    A.lea(RSI, ("abs", "arg0", 0))
    A.call("rt_copyz")
    A.mov_rm_imm32(("r", R8), 1)
    A.label("rt_argl")
    A.alu_r_rm("cmp", R8, ("abs", "argc", 0))
    A.jcc("ae", "rt_arge")
    A.raw(0xC6, 0x07, 0x00)                       # mov byte [rdi], 0
    A.lea(RDI, ("m", RDI, 1))
    A.mov_r_rm(RSI, ("abs", "argv", 0))
    A.raw(0x4A, 0x8B, 0x34, 0xC6)                 # mov rsi, [rsi + r8*8]
    A.call("rt_copyz")
    A.lea(R8, ("m", R8, 1))
    A.jmp("rt_argl")
    A.label("rt_arge")
    A.pop(RAX)                                     # storage
    A.mov_r_rm(RCX, ("r", RDI))
    A.alu_r_rm("sub", RCX, ("r", RAX))
    A.mov_rm_r(("m", RAX, -8), RCX)
    A.mov_r_rm(RDX, ("r", RDI))
    A.mov_r_imm64(RSI, base)
    A.alu_r_rm("sub", RDX, ("r", RSI))
    A.mov_rm_r(("abs", "top", 0), RDX)
    A.pop(RCX)          # argc tag
    A.pop(RDX)          # argv tag
    A.mov_rm_r(("m", RDX, 8), RAX)
    A.mov_r_rm(RAX, ("abs", "argc", 0))
    A.mov_rm_r(("m", RCX, 8), RAX)
    A.jmp("rt_s5")
    A.label("rt_s5p")
    A.pop(RAX)
    A.label("rt_s5")
    # the event slot: the SASEVENT tag's address if it occurs exactly once (0 otherwise)
    A.mov_r_imm64(RAX, EVENT_TAG)
    A.call("rt_find_tag")
    A.mov_rm_r(("abs", "ev_slot", 0), RAX)
    # A THREADED IMAGE (SASTHRDS anywhere) IS REFUSED BEFORE IT RUNS: yantra-run schedules its threads
    # from an event log (and refuses it with none, or past 64); natively it would run as one thread
    # and silently print something else. The refusal is a `native: not in milestone 1` line, so
    # t1-run reruns the image whole under yantra-run.
    A.mov_r_imm64(RAX, THREADS_TAG)
    A.call("rt_find_tag")
    A.test_rm_r(("r", R8), R8)
    A.jcc("ne", "rt_threaded")
    # limits: a load may reach top (the slab), a store only ram (yantra's store_limit)
    A.mov_r_imm64(RAX, base + prot_size)      # code_end bounds stores: the protected code span only
    A.mov_rm_r(("abs", "code_end", 0), RAX)
    for width in (1, 2, 4, 8):
        A.mov_r_rm(RAX, ("abs", "top", 0))
        A.alu_rm_imm("sub", ("r", RAX), width - 1)
        A.mov_rm_r(("abs", "lim_l%d" % width, 0), RAX)
        A.mov_r_rm(RAX, ("abs", "ram", 0))
        A.alu_rm_imm("sub", ("r", RAX), prot_size + width - 1)
        A.mov_rm_r(("abs", "lim_s%d" % width, 0), RAX)
    # the sp window for FAST blocks: sp in [code_end + 2048, base + ram - 2056] makes every
    # sp+imm (|imm| <= 2048, width <= 8) a RAM access outside the code span; span = hi - lo + 1
    A.mov_r_imm64(RAX, base + code_size + 2048)
    A.mov_rm_r(("abs", "sp_lo", 0), RAX)
    A.mov_r_rm(RAX, ("abs", "ram", 0))
    A.alu_rm_imm("sub", ("r", RAX), code_size + 4103)
    A.jcc("ns", "rt_spw")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.label("rt_spw")
    A.mov_rm_r(("abs", "sp_span", 0), RAX)
    # the guest: every register zero, pc = entry
    for n, h in M.items():
        A.alu_r_rm("xor", h, ("r", h), 0)
    A.mov_r_imm64(RAX, entry)
    A.jmp("rt_indirect")

    # ---- jalr: rax = target
    A.label("rt_indirect")
    A.lea(RDX, ("m", RAX, -base))
    A.alu_rm_imm("cmp", ("r", RDX), code_size)
    A.jcc("ae", "rt_bad_target")
    A.op(b"\xf6", 0, ("r", RDX), 0)
    A.raw(3)                                       # test dl, 3
    A.jcc("ne", "rt_bad_target")
    A.jmp_rm(("sib2", RDX, "jt"))

    # ---- a store that missed RAM: rax = addr, rcx = value, edx = width, fault_off = pc - base
    A.label("rt_store_slow")
    A.push(RDX)
    A.mov_r_imm64(RDX, UART)
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    A.pop(RDX)
    A.jcc("ne", "rt_st2")
    A.mov_r_rm(RDX, ("abs", "outlen", 0))
    A.lea(RAX, ("abs", "outbuf", 0))
    A.raw(0x88, 0x0C, 0x10)                        # mov [rax+rdx], cl
    A.lea(RDX, ("m", RDX, 1))
    A.mov_rm_r(("abs", "outlen", 0), RDX)
    A.alu_rm_imm("cmp", ("r", RDX), 65536)
    A.jcc("b", "rt_st1")
    A.call("rt_flush")
    A.label("rt_st1")
    A.ret()
    A.label("rt_st2")
    A.mov_rm_imm32(("r", RDX), WAIT)
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    A.jcc("ne", "rt_st3")
    A.jmp("rt_wait")                               # returns to the cold stub's caller
    A.label("rt_st3")
    A.mov_rm_imm32(("r", RDX), FINISHER)
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    A.jcc("ne", "rt_st_win")
    # FINISHER: flush, print yantra's halt line, exit 0 on 0x5555, else 1
    A.push(RCX)
    A.call("rt_flush")
    A.call("m_fin1")
    A.mov_r_rm(RAX, ("m", RSP, 0))
    A.call("rt_putu")
    A.pop(RCX)
    A.mov_r_rm(RAX, ("r", RCX))
    A.alu_rm_imm("and", ("r", RAX), 0xFFFF)
    A.alu_rm_imm("cmp", ("r", RAX), 0x5555)
    A.jcc("e", "rt_fin_ok")
    A.alu_rm_imm("cmp", ("r", RAX), 0x3333)
    A.jcc("ne", "rt_fin_none")
    A.push(RCX)
    A.call("m_fin2")
    A.pop(RAX)
    A.shift_imm("shr", ("r", RAX), 16)
    A.call("rt_putu")
    A.call("m_fin3")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_fin_ok")
    A.call("m_fin2")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.call("rt_putu")
    A.call("m_fin3")
    A.alu_r_rm("xor", RDI, ("r", RDI), 0)
    A.jmp("rt_exit")
    A.label("rt_fin_none")
    A.call("m_fin4")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    runtime_window(A, base, prot_size, win, mac)
    runtime_spawn(A, base, win, mac)
    # a store that missed RAM and is not UART/FINISHER: rax = addr
    A.label("rt_beyond")
    A.mov_rm_r(("abs", "fault_addr", 0), RAX)
    A.call("rt_flush")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.mov_r_imm64(RCX, 1 << 31)
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("b", "rt_store_bad")            # below 2^31, no device
    A.mov_r_rm(RCX, ("abs", "code_end", 0))
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("b", "rt_store_code")           # in the code span
    A.call("m_beyondstore")
    A.jmp("rt_fault_tail")
    A.label("rt_store_bad")
    A.call("m_badstore")
    A.jmp("rt_fault_tail")
    A.label("rt_store_code")
    A.call("m_codestore")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("m_addr")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.call("rt_puthex")
    A.call("m_codestore2")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    # a load that missed RAM: rax = addr
    A.label("rt_load_slow")
    A.mov_rm_r(("abs", "fault_addr", 0), RAX)
    A.call("rt_flush")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.mov_r_imm64(RCX, 1 << 31)
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("b", "rt_load_bad")             # below 2^31 (a device): not in milestone 1
    A.call("m_beyondload")
    A.jmp("rt_fault_tail")
    A.label("rt_load_bad")
    # yantra's load answers BadAccess below base except at its load devices: the counter and the
    # socket (0x1000_0108..0x1000_0120) and the virtio-mmio slots (0x1000_1000..0x1000_9000).
    # Those are not in milestone 1; every other address halts as yantra-run prints it.
    for lo, n in LOAD_DEVICES:
        A.mov_r_rm(RCX, ("r", RAX))
        A.mov_r_imm64(RDX, lo)
        A.alu_r_rm("sub", RCX, ("r", RDX))
        A.alu_rm_imm("cmp", ("r", RCX), n)
        A.jcc("b", "rt_load_dev")
    A.call("m_bada1")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("m_bada2")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.call("rt_putu")
    A.call("m_bada3")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_load_dev")
    A.call("m_badload")
    A.label("rt_fault_tail")              # prints pc (decimal) then addr (hex)
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("m_addr")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.call("rt_puthex")
    A.call("m_nl")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_bad_insn")                # pc only (no address)
    A.call("rt_flush")
    A.call("m_insn")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("m_nl")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_bad_target")             # rax = the untranslated target
    A.mov_rm_r(("abs", "fault_addr", 0), RAX)
    A.call("rt_flush")
    A.call("m_jump1")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.call("rt_puthex")
    A.call("m_jump2")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    # ---- YANTRA_INPUT_ENTRY: rax = its value. Refused, as yantra-run refuses it, when
    # YANTRA_INPUT_NAME is set too, or unless it is exactly one ASCII space between two
    # non-empty halves free of whitespace (char::is_whitespace); else the space becomes NUL
    A.label("rt_entry")
    A.mov_rm_r(("abs", "name_ptr", 0), RAX)
    A.lea(RDI, ("abs", "env_name", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_entry_both")
    A.mov_r_rm(RSI, ("abs", "name_ptr", 0))
    A.call("rt_strlen")
    A.mov_rm_r(("abs", "name_len", 0), RAX)
    # over 4096 octets, or not UTF-8: refused before the shape is looked at (yantra-run)
    A.alu_rm_imm("cmp", ("r", RAX), 4096)
    A.jcc("a", "rt_entry_long")
    A.label("rt_eu1")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_eu2")
    A.call("rt_u8")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "rt_entry_bad8")
    A.alu_r_rm("add", RSI, ("r", RDX))
    A.jmp("rt_eu1")
    A.label("rt_eu2")
    A.mov_r_rm(RSI, ("abs", "name_ptr", 0))
    A.mov_r_rm(RAX, ("abs", "name_len", 0))
    A.mov_r_rm(R10, ("r", RAX))
    A.alu_r_rm("xor", R9, ("r", R9), 0)
    A.label("rt_en1")
    A.alu_r_rm("cmp", R9, ("r", R10))
    A.jcc("ae", "rt_entry_bad")
    A.mov_r_rm(RDI, ("r", RSI))
    A.alu_r_rm("add", RDI, ("r", R9))
    A.movzx(RDX, ("m", RDI, 0), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 32)
    A.jcc("e", "rt_en2")
    A.lea(R9, ("m", R9, 1))
    A.jmp("rt_en1")
    A.label("rt_en2")
    A.test_rm_r(("r", R9), R9)
    A.jcc("e", "rt_entry_bad")
    A.lea(RDX, ("m", R9, 1))
    A.alu_r_rm("cmp", RDX, ("r", R10))
    A.jcc("ae", "rt_entry_bad")
    A.alu_r_rm("xor", R8, ("r", R8), 0)
    A.label("rt_ew1")
    A.alu_r_rm("cmp", R8, ("r", R10))
    A.jcc("ae", "rt_ew_ok")
    A.alu_r_rm("cmp", R8, ("r", R9))
    A.jcc("e", "rt_ew_next")
    A.mov_r_rm(RDI, ("r", RSI))
    A.alu_r_rm("add", RDI, ("r", R8))
    A.movzx(RDX, ("m", RDI, 0), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 32)
    A.jcc("e", "rt_entry_bad")
    A.alu_rm_imm("cmp", ("r", RDX), 9)
    A.jcc("b", "rt_ew_hi")
    A.alu_rm_imm("cmp", ("r", RDX), 13)
    A.jcc("be", "rt_entry_bad")
    A.label("rt_ew_hi")
    A.alu_rm_imm("cmp", ("r", RDX), 0xC2)              # U+0085 U+00A0
    A.jcc("ne", "rt_ew_e1")
    A.movzx(RDX, ("m", RDI, 1), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 0x85)
    A.jcc("e", "rt_entry_bad")
    A.alu_rm_imm("cmp", ("r", RDX), 0xA0)
    A.jcc("e", "rt_entry_bad")
    A.jmp("rt_ew_next")
    A.label("rt_ew_e1")
    A.alu_rm_imm("cmp", ("r", RDX), 0xE1)              # U+1680
    A.jcc("ne", "rt_ew_e2")
    A.movzx(RDX, ("m", RDI, 1), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 0x9A)
    A.jcc("ne", "rt_ew_next")
    A.movzx(RDX, ("m", RDI, 2), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 0x80)
    A.jcc("e", "rt_entry_bad")
    A.jmp("rt_ew_next")
    A.label("rt_ew_e2")
    A.alu_rm_imm("cmp", ("r", RDX), 0xE2)              # U+2000..200A 2028 2029 202F 205F
    A.jcc("ne", "rt_ew_e3")
    A.movzx(RDX, ("m", RDI, 1), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 0x81)
    A.jcc("e", "rt_ew_e2b")
    A.alu_rm_imm("cmp", ("r", RDX), 0x80)
    A.jcc("ne", "rt_ew_next")
    A.movzx(RDX, ("m", RDI, 2), 1)
    A.alu_rm_imm("sub", ("r", RDX), 0x80)
    A.alu_rm_imm("cmp", ("r", RDX), 10)
    A.jcc("be", "rt_entry_bad")
    A.alu_rm_imm("cmp", ("r", RDX), 0x28)
    A.jcc("e", "rt_entry_bad")
    A.alu_rm_imm("cmp", ("r", RDX), 0x29)
    A.jcc("e", "rt_entry_bad")
    A.alu_rm_imm("cmp", ("r", RDX), 0x2F)
    A.jcc("e", "rt_entry_bad")
    A.jmp("rt_ew_next")
    A.label("rt_ew_e2b")
    A.movzx(RDX, ("m", RDI, 2), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 0x9F)
    A.jcc("e", "rt_entry_bad")
    A.jmp("rt_ew_next")
    A.label("rt_ew_e3")
    A.alu_rm_imm("cmp", ("r", RDX), 0xE3)              # U+3000
    A.jcc("ne", "rt_ew_next")
    A.movzx(RDX, ("m", RDI, 1), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 0x80)
    A.jcc("ne", "rt_ew_next")
    A.movzx(RDX, ("m", RDI, 2), 1)
    A.alu_rm_imm("cmp", ("r", RDX), 0x80)
    A.jcc("e", "rt_entry_bad")
    A.label("rt_ew_next")
    A.lea(R8, ("m", R8, 1))
    A.jmp("rt_ew1")
    A.label("rt_ew_ok")
    A.mov_r_rm(RDI, ("r", RSI))
    A.alu_r_rm("add", RDI, ("r", R9))
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.mov_rm_r(("m", RDI, 0), RCX, width=1)            # module NUL routine
    A.jmp("rt_s2")
    A.label("rt_no_name")
    A.call("m_noname")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_entry_both")
    A.call("m_entry_both")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_entry_long")
    A.call("m_long1")
    A.mov_r_rm(RAX, ("abs", "name_len", 0))
    A.call("rt_putu")
    A.call("m_long2")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_entry_bad8")
    A.call("m_entry1")
    A.call("rt_putq")
    A.call("m_bad8")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    # ---- u8: rsi = a position in a zero-ended string. Rust's UTF-8 rules (maximal
    # subparts): rcx = 0 and rdx = the length of the valid character there, or rcx = the
    # length (1..3) of the invalid sequence there. Uses rax, rcx, rdx, r9, r11.
    A.label("rt_u8")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.alu_rm_imm("cmp", ("r", RAX), 0x80)
    A.jcc("b", "rt_u8_1")
    A.alu_rm_imm("cmp", ("r", RAX), 0xC2)
    A.jcc("b", "rt_u8_bad1")
    A.alu_rm_imm("cmp", ("r", RAX), 0xE0)
    A.jcc("b", "rt_u8_2")
    A.alu_rm_imm("cmp", ("r", RAX), 0xF0)
    A.jcc("b", "rt_u8_3")
    A.alu_rm_imm("cmp", ("r", RAX), 0xF5)
    A.jcc("b", "rt_u8_4")
    A.label("rt_u8_bad1")
    A.mov_rm_imm32(("r", RCX), 1)
    A.ret()
    A.label("rt_u8_bad2")
    A.mov_rm_imm32(("r", RCX), 2)
    A.ret()
    A.label("rt_u8_bad3")
    A.mov_rm_imm32(("r", RCX), 3)
    A.ret()
    A.label("rt_u8_1")
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.mov_rm_imm32(("r", RDX), 1)
    A.ret()
    A.label("rt_u8_2")
    A.movzx(RDX, ("m", RSI, 1), 1)
    A.alu_rm_imm("sub", ("r", RDX), 0x80)
    A.alu_rm_imm("cmp", ("r", RDX), 0x3F)
    A.jcc("a", "rt_u8_bad1")
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.mov_rm_imm32(("r", RDX), 2)
    A.ret()
    A.label("rt_u8_3")
    A.mov_rm_imm32(("r", R11), 0x80)
    A.mov_rm_imm32(("r", R9), 0xBF)
    A.alu_rm_imm("cmp", ("r", RAX), 0xE0)
    A.jcc("ne", "rt_u8_3a")
    A.mov_rm_imm32(("r", R11), 0xA0)
    A.label("rt_u8_3a")
    A.alu_rm_imm("cmp", ("r", RAX), 0xED)
    A.jcc("ne", "rt_u8_3b")
    A.mov_rm_imm32(("r", R9), 0x9F)
    A.label("rt_u8_3b")
    A.movzx(RDX, ("m", RSI, 1), 1)
    A.alu_r_rm("cmp", RDX, ("r", R11))
    A.jcc("b", "rt_u8_bad1")
    A.alu_r_rm("cmp", RDX, ("r", R9))
    A.jcc("a", "rt_u8_bad1")
    A.movzx(RDX, ("m", RSI, 2), 1)
    A.alu_rm_imm("sub", ("r", RDX), 0x80)
    A.alu_rm_imm("cmp", ("r", RDX), 0x3F)
    A.jcc("a", "rt_u8_bad2")
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.mov_rm_imm32(("r", RDX), 3)
    A.ret()
    A.label("rt_u8_4")
    A.mov_rm_imm32(("r", R11), 0x80)
    A.mov_rm_imm32(("r", R9), 0xBF)
    A.alu_rm_imm("cmp", ("r", RAX), 0xF0)
    A.jcc("ne", "rt_u8_4a")
    A.mov_rm_imm32(("r", R11), 0x90)
    A.label("rt_u8_4a")
    A.alu_rm_imm("cmp", ("r", RAX), 0xF4)
    A.jcc("ne", "rt_u8_4b")
    A.mov_rm_imm32(("r", R9), 0x8F)
    A.label("rt_u8_4b")
    A.movzx(RDX, ("m", RSI, 1), 1)
    A.alu_r_rm("cmp", RDX, ("r", R11))
    A.jcc("b", "rt_u8_bad1")
    A.alu_r_rm("cmp", RDX, ("r", R9))
    A.jcc("a", "rt_u8_bad1")
    A.movzx(RDX, ("m", RSI, 2), 1)
    A.alu_rm_imm("sub", ("r", RDX), 0x80)
    A.alu_rm_imm("cmp", ("r", RDX), 0x3F)
    A.jcc("a", "rt_u8_bad2")
    A.movzx(RDX, ("m", RSI, 3), 1)
    A.alu_rm_imm("sub", ("r", RDX), 0x80)
    A.alu_rm_imm("cmp", ("r", RDX), 0x3F)
    A.jcc("a", "rt_u8_bad3")
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.mov_rm_imm32(("r", RDX), 4)
    A.ret()
    A.label("rt_entry_bad")
    A.call("m_entry1")
    A.call("rt_putq")
    A.call("m_entry2")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    # ---- putq: the entry value as Rust's {:?} writes it, to stderr (outbuf is the scratch):
    # \" \\ \t \n \r, \u{..} for controls (C0, DEL, C1), U+00AD and the Devanagari
    # combining marks; every other octet as it is
    A.label("rt_putq")
    A.mov_r_rm(RSI, ("abs", "name_ptr", 0))
    A.lea(RDI, ("abs", "outbuf", 0))
    A.mov_rm_imm32(("r", RDX), 0x22)
    A.mov_rm_r(("m", RDI, 0), RDX, width=1)
    A.lea(RDI, ("m", RDI, 1))
    A.label("rt_q1")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_qend")
    # an invalid UTF-8 sequence goes out as \\xHH per octet (Rust's OsStr {:?})
    A.call("rt_u8")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "rt_q_x")
    A.mov_r_rm(RBX, ("r", RDX))                # the valid character's octets
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.alu_rm_imm("cmp", ("r", RAX), 0x22)
    A.jcc("e", "rt_q_esc")
    A.alu_rm_imm("cmp", ("r", RAX), 0x5C)
    A.jcc("e", "rt_q_esc")
    A.alu_rm_imm("cmp", ("r", RAX), 9)
    A.jcc("e", "rt_q_t")
    A.alu_rm_imm("cmp", ("r", RAX), 10)
    A.jcc("e", "rt_q_n")
    A.alu_rm_imm("cmp", ("r", RAX), 13)
    A.jcc("e", "rt_q_r")
    A.alu_rm_imm("cmp", ("r", RAX), 0x20)
    A.jcc("b", "rt_q_u1")
    A.alu_rm_imm("cmp", ("r", RAX), 0x7F)
    A.jcc("e", "rt_q_u1")
    A.alu_rm_imm("cmp", ("r", RAX), 0xC2)
    A.jcc("b", "rt_q_plain")
    A.alu_rm_imm("cmp", ("r", RAX), 0xE0)
    A.jcc("b", "rt_q_2")
    A.alu_rm_imm("cmp", ("r", RAX), 0xF0)
    A.jcc("ae", "rt_q_plain")
    # three octets: both continuations, or the lead goes out as it is
    A.movzx(RDX, ("m", RSI, 1), 1)
    A.mov_r_rm(RCX, ("r", RDX))
    A.alu_rm_imm("and", ("r", RCX), 0xC0)
    A.alu_rm_imm("cmp", ("r", RCX), 0x80)
    A.jcc("ne", "rt_q_plain")
    A.movzx(R10, ("m", RSI, 2), 1)
    A.mov_r_rm(RCX, ("r", R10))
    A.alu_rm_imm("and", ("r", RCX), 0xC0)
    A.alu_rm_imm("cmp", ("r", RCX), 0x80)
    A.jcc("ne", "rt_q_plain")
    A.alu_rm_imm("and", ("r", R10), 0x3F)
    A.alu_rm_imm("and", ("r", RDX), 0x3F)
    A.shift_imm("shl", ("r", RDX), 6)
    A.alu_r_rm("add", R10, ("r", RDX))
    A.mov_r_rm(RDX, ("r", RAX))
    A.alu_rm_imm("and", ("r", RDX), 0xF)
    A.shift_imm("shl", ("r", RDX), 12)
    A.alu_r_rm("add", R10, ("r", RDX))
    A.mov_rm_imm32(("r", R8), 3)
    A.jmp("rt_q_tab")
    A.label("rt_q_2")
    A.movzx(RDX, ("m", RSI, 1), 1)
    A.mov_r_rm(RCX, ("r", RDX))
    A.alu_rm_imm("and", ("r", RCX), 0xC0)
    A.alu_rm_imm("cmp", ("r", RCX), 0x80)
    A.jcc("ne", "rt_q_plain")
    A.mov_r_rm(R10, ("r", RDX))
    A.alu_rm_imm("and", ("r", R10), 0x3F)
    A.mov_r_rm(RDX, ("r", RAX))
    A.alu_rm_imm("and", ("r", RDX), 0x1F)
    A.shift_imm("shl", ("r", RDX), 6)
    A.alu_r_rm("add", R10, ("r", RDX))
    A.mov_rm_imm32(("r", R8), 2)
    A.label("rt_q_tab")
    A.lea(R9, ("abs", "rt_esctab", 0))
    A.label("rt_qt1")
    A.mov_r_rm(RDX, ("m", R9, 0), 0)
    A.mov_r_rm(RCX, ("m", R9, 4), 0)
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("e", "rt_q_plain")
    A.alu_r_rm("cmp", R10, ("r", RDX))
    A.jcc("b", "rt_qt2")
    A.alu_r_rm("cmp", R10, ("r", RCX))
    A.jcc("be", "rt_q_u")
    A.label("rt_qt2")
    A.lea(R9, ("m", R9, 8))
    A.jmp("rt_qt1")
    A.label("rt_q_plain")
    A.mov_rm_r(("m", RDI, 0), RAX, width=1)
    A.lea(RDI, ("m", RDI, 1))
    A.lea(RSI, ("m", RSI, 1))
    A.label("rt_q_rest")                       # the character's other octets, as they are
    A.alu_rm_imm("sub", ("r", RBX), 1)
    A.jcc("e", "rt_q1")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.mov_rm_r(("m", RDI, 0), RAX, width=1)
    A.lea(RDI, ("m", RDI, 1))
    A.lea(RSI, ("m", RSI, 1))
    A.jmp("rt_q_rest")
    A.label("rt_q_esc")
    A.mov_rm_imm32(("r", RDX), 0x5C)
    A.mov_rm_r(("m", RDI, 0), RDX, width=1)
    A.lea(RDI, ("m", RDI, 1))
    A.jmp("rt_q_plain")
    A.label("rt_q_t")
    A.mov_rm_imm32(("r", RAX), 0x74)
    A.jmp("rt_q_esc")
    A.label("rt_q_n")
    A.mov_rm_imm32(("r", RAX), 0x6E)
    A.jmp("rt_q_esc")
    A.label("rt_q_r")
    A.mov_rm_imm32(("r", RAX), 0x72)
    A.jmp("rt_q_esc")
    A.label("rt_q_x")
    A.mov_r_rm(R8, ("r", RCX))
    A.label("rt_q_x1")
    A.movzx(RAX, ("m", RSI, 0), 1)
    for c in (0x5C, 0x78):
        A.mov_rm_imm32(("r", RDX), c)
        A.mov_rm_r(("m", RDI, 0), RDX, width=1)
        A.lea(RDI, ("m", RDI, 1))
    for sh in (4, 0):
        A.mov_r_rm(RDX, ("r", RAX))
        A.shift_imm("shr", ("r", RDX), sh)
        A.alu_rm_imm("and", ("r", RDX), 15)
        A.alu_rm_imm("cmp", ("r", RDX), 10)
        A.jcc("b", f"rt_qxd{sh}")
        A.alu_rm_imm("add", ("r", RDX), 7)
        A.label(f"rt_qxd{sh}")
        A.alu_rm_imm("add", ("r", RDX), 48)
        A.mov_rm_r(("m", RDI, 0), RDX, width=1)
        A.lea(RDI, ("m", RDI, 1))
    A.lea(RSI, ("m", RSI, 1))
    A.alu_rm_imm("sub", ("r", R8), 1)
    A.jcc("ne", "rt_q_x1")
    A.jmp("rt_q1")
    A.label("rt_q_u1")
    A.mov_r_rm(R10, ("r", RAX))
    A.mov_rm_imm32(("r", R8), 1)
    # \\u{cp}: r10 = cp (< 0x10000), r8 = its octets; minimal lowercase hex digits
    A.label("rt_q_u")
    for c in (0x5C, 0x75, 0x7B):
        A.mov_rm_imm32(("r", RDX), c)
        A.mov_rm_r(("m", RDI, 0), RDX, width=1)
        A.lea(RDI, ("m", RDI, 1))
    A.alu_r_rm("xor", R11, ("r", R11), 0)
    for sh in (12, 8, 4):
        A.mov_r_rm(RDX, ("r", R10))
        A.shift_imm("shr", ("r", RDX), sh)
        A.alu_rm_imm("and", ("r", RDX), 15)
        A.test_rm_r(("r", RDX), RDX)
        A.jcc("ne", f"rt_qd{sh}")
        A.test_rm_r(("r", R11), R11)
        A.jcc("e", f"rt_qs{sh}")
        A.label(f"rt_qd{sh}")
        A.mov_rm_imm32(("r", R11), 1)
        A.alu_rm_imm("cmp", ("r", RDX), 10)
        A.jcc("b", f"rt_qx{sh}")
        A.alu_rm_imm("add", ("r", RDX), 39)
        A.label(f"rt_qx{sh}")
        A.alu_rm_imm("add", ("r", RDX), 48)
        A.mov_rm_r(("m", RDI, 0), RDX, width=1)
        A.lea(RDI, ("m", RDI, 1))
        A.label(f"rt_qs{sh}")
    A.mov_r_rm(RDX, ("r", R10))                      # the last digit always shows
    A.alu_rm_imm("and", ("r", RDX), 15)
    A.alu_rm_imm("cmp", ("r", RDX), 10)
    A.jcc("b", "rt_qx0")
    A.alu_rm_imm("add", ("r", RDX), 39)
    A.label("rt_qx0")
    A.alu_rm_imm("add", ("r", RDX), 48)
    A.mov_rm_r(("m", RDI, 0), RDX, width=1)
    A.lea(RDI, ("m", RDI, 1))
    A.mov_rm_imm32(("r", RDX), 0x7D)
    A.mov_rm_r(("m", RDI, 0), RDX, width=1)
    A.lea(RDI, ("m", RDI, 1))
    A.alu_r_rm("add", RSI, ("r", R8))
    A.jmp("rt_q1")
    A.label("rt_qend")
    A.mov_rm_imm32(("r", RDX), 0x22)
    A.mov_rm_r(("m", RDI, 0), RDX, width=1)
    A.lea(RDI, ("m", RDI, 1))
    A.mov_r_rm(RDX, ("r", RDI))
    A.lea(RSI, ("abs", "outbuf", 0))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    A.mov_rm_imm32(("r", RDI), 2)
    sys_(1)
    A.ret()
    A.label("rt_no_input")
    A.call("m_noinput")
    A.mov_rm_imm32(("r", RDI), 2)
    A.jmp("rt_exit")
    A.label("rt_no_ram")
    A.call("m_noram")
    A.mov_rm_imm32(("r", RDI), 2)
    A.jmp("rt_exit")
    # "{arg0}: segment at {vaddr:#x} needs {need} bytes and RAM is {ram} — raise it", exit 1
    A.label("rt_seg_fail")                   # rbx = the segment's (vaddr, need)
    A.lea(RSI, ("abs", "arg0", 0))
    A.call("rt_strlen")
    A.mov_r_rm(RDX, ("r", RAX))
    A.lea(RSI, ("abs", "arg0", 0))
    A.mov_rm_imm32(("r", RDI), 2)
    sys_(1)
    A.call("m_seg0")
    A.mov_r_rm(RAX, ("m", RBX, 0))
    A.call("rt_puthex")
    A.call("m_seg1")
    A.mov_r_rm(RAX, ("m", RBX, 8))
    A.call("rt_putu")
    A.call("m_seg2")
    A.mov_r_rm(RAX, ("abs", "ram", 0))
    A.call("rt_putu")
    A.call("m_seg3")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_no_rec")
    A.call("m_norec")
    A.mov_rm_imm32(("r", RDI), 2)
    A.jmp("rt_exit")
    A.label("rt_threaded")
    A.call("m_threaded")
    A.mov_rm_imm32(("r", RDI), 2)
    A.jmp("rt_exit")
    A.label("rt_wt_notag")
    A.call("rt_flush")
    A.call("m_waitnotag")
    A.mov_rm_imm32(("r", RDI), 2)
    A.jmp("rt_exit")
    A.label("rt_wt_plain")
    A.call("rt_flush")
    A.call("m_waitplain")
    A.mov_rm_imm32(("r", RDI), 2)
    A.jmp("rt_exit")
    A.label("rt_no_tag")
    A.call("m_notag")
    A.mov_rm_imm32(("r", RDI), 2)
    A.label("rt_exit")
    sys_(60)

    # ---- flush: write(1, outbuf, outlen); saves every register the syscall touches
    A.label("rt_flush")
    for reg in (RAX, RCX, RDX, RSI, RDI, R11):
        A.push(reg)
    A.lea(RSI, ("abs", "outbuf", 0))
    A.mov_r_rm(RDX, ("abs", "outlen", 0))
    A.label("rt_fl1")
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "rt_fl2")
    A.mov_rm_imm32(("r", RDI), 1)
    if not win:
        A.mov_rm_imm32(("r", RAX), sc(1))
    A.push(RDX)
    A.push(RSI)
    if win:
        A.call("w_sys1")
    else:
        A.syscall()
    A.pop(RSI)
    A.pop(RDX)
    if mac:
        A.jcc("b", "rt_fl2")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("le", "rt_fl2")
    A.alu_r_rm("add", RSI, ("r", RAX))
    A.alu_r_rm("sub", RDX, ("r", RAX))
    A.jmp("rt_fl1")
    A.label("rt_fl2")
    A.mov_rm_imm32(("abs", "outlen", 0), 0)
    for reg in (R11, RDI, RSI, RDX, RCX, RAX):
        A.pop(reg)
    A.ret()

    # ---- getenv: rdi = "NAME=" (zero-ended); answer rax = value pointer or 0
    A.label("rt_getenv")
    A.mov_r_rm(R8, ("abs", "envp", 0))
    A.label("rt_ge1")
    A.mov_r_rm(RSI, ("m", R8, 0))
    A.test_rm_r(("r", RSI), RSI)
    A.jcc("e", "rt_ge_no")
    A.mov_r_rm(RCX, ("r", RDI))
    A.label("rt_ge2")
    A.movzx(RAX, ("m", RCX, 0), 1)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_ge_yes")
    A.movzx(RDX, ("m", RSI, 0), 1)
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    A.jcc("ne", "rt_ge_next")
    A.lea(RCX, ("m", RCX, 1))
    A.lea(RSI, ("m", RSI, 1))
    A.jmp("rt_ge2")
    A.label("rt_ge_next")
    A.lea(R8, ("m", R8, 8))
    A.jmp("rt_ge1")
    A.label("rt_ge_yes")
    A.mov_r_rm(RAX, ("r", RSI))
    A.ret()
    A.label("rt_ge_no")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.ret()

    # ---- atou: rsi = decimal digits; rax = value
    A.label("rt_atou")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.label("rt_at1")
    A.movzx(RCX, ("m", RSI, 0), 1)
    A.alu_rm_imm("sub", ("r", RCX), 48)
    A.alu_rm_imm("cmp", ("r", RCX), 9)
    A.jcc("a", "rt_at2")
    A.raw(0x48, 0x6B, 0xC0, 0x0A)                  # imul rax, rax, 10
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.lea(RSI, ("m", RSI, 1))
    A.jmp("rt_at1")
    A.label("rt_at2")
    A.ret()

    # ---- atou_ram: rsi = YANTRA_RAM's value; rax = it as Rust's usize::from_str reads it (an
    # optional '+', then one or more decimal digits, nothing else, at most 2^64-1); rcx = 0, or 1
    # when it is not one (rdx, r8 clobbered)
    A.label("rt_atou_ram")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.mov_rm_imm32(("r", R8), 10)
    A.movzx(RCX, ("m", RSI, 0), 1)
    A.alu_rm_imm("cmp", ("r", RCX), 43)            # '+'
    A.jcc("ne", "rt_ar0")
    A.lea(RSI, ("m", RSI, 1))
    A.label("rt_ar0")
    A.movzx(RCX, ("m", RSI, 0), 1)
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("e", "rt_ar_bad")                        # no digit
    A.label("rt_ar1")
    A.movzx(RCX, ("m", RSI, 0), 1)
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("e", "rt_ar_ok")
    A.alu_rm_imm("sub", ("r", RCX), 48)
    A.alu_rm_imm("cmp", ("r", RCX), 9)
    A.jcc("a", "rt_ar_bad")
    A.f7(4, ("r", R8))                             # mul r8: rdx:rax = rax * 10
    A.jcc("b", "rt_ar_bad")                        # CF: the product left 64 bits
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.jcc("b", "rt_ar_bad")
    A.lea(RSI, ("m", RSI, 1))
    A.jmp("rt_ar1")
    A.label("rt_ar_ok")
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.ret()
    A.label("rt_ar_bad")
    A.mov_rm_imm32(("r", RCX), 1)
    A.ret()

    # ---- strlen: rsi; rax = length
    A.label("rt_strlen")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.label("rt_sl1")
    A.raw(0x80, 0x3C, 0x06, 0x00)                  # cmp byte [rsi+rax], 0
    A.jcc("e", "rt_sl2")
    A.lea(RAX, ("m", RAX, 1))
    A.jmp("rt_sl1")
    A.label("rt_sl2")
    A.ret()

    # ---- args_len: strlen(arg0) + 1 + sum(strlen(argv[i]) + 1)
    A.label("rt_args_len")
    A.lea(RSI, ("abs", "arg0", 0))
    A.call("rt_strlen")
    A.lea(R9, ("m", RAX, 1))
    A.mov_rm_imm32(("r", R8), 1)
    A.label("rt_al1")
    A.alu_r_rm("cmp", R8, ("abs", "argc", 0))
    A.jcc("ae", "rt_al2")
    A.mov_r_rm(RSI, ("abs", "argv", 0))
    A.raw(0x4A, 0x8B, 0x34, 0xC6)                  # mov rsi, [rsi + r8*8]
    A.call("rt_strlen")
    A.lea(R9, ("m", R9, 1))
    A.alu_r_rm("add", R9, ("r", RAX))
    A.lea(R8, ("m", R8, 1))
    A.jmp("rt_al1")
    A.label("rt_al2")
    A.mov_r_rm(RAX, ("r", R9))
    A.ret()

    # ---- copyz: copy rsi's zero-ended string to rdi (no terminator); rdi advances
    A.label("rt_copyz")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_cz2")
    A.raw(0x88, 0x07)                              # mov [rdi], al
    A.lea(RDI, ("m", RDI, 1))
    A.lea(RSI, ("m", RSI, 1))
    A.jmp("rt_copyz")
    A.label("rt_cz2")
    A.ret()

    # ---- run_open: storage = align16(top + 8) above base; rax = its address
    A.label("rt_run_open")
    A.mov_r_rm(RAX, ("abs", "top", 0))
    A.alu_rm_imm("add", ("r", RAX), 8 + 15)
    A.alu_rm_imm("and", ("r", RAX), -16)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.ret()

    # ---- find_tag: rax = the 8-octet tag; answer rax = its address if it occurs EXACTLY
    # once in the file-backed image, else 0 (yantra scans every offset)
    A.label("rt_find_tag")
    A.mov_r_imm64(RSI, base)
    A.mov_r_imm64(RCX, base + filesz - 7)
    A.alu_r_rm("xor", RDI, ("r", RDI), 0)          # hit
    A.alu_r_rm("xor", R8, ("r", R8), 0)            # count
    A.label("rt_ft1")
    A.alu_r_rm("cmp", RSI, ("r", RCX))
    A.jcc("ae", "rt_ft3")
    A.alu_r_rm("cmp", RAX, ("m", RSI, 0))
    A.jcc("ne", "rt_ft2")
    A.mov_r_rm(RDI, ("r", RSI))
    A.lea(R8, ("m", R8, 1))
    A.label("rt_ft2")
    A.lea(RSI, ("m", RSI, 1))
    A.jmp("rt_ft1")
    A.label("rt_ft3")
    A.alu_rm_imm("cmp", ("r", R8), 1)
    A.jcc("ne", "rt_ft4")
    A.mov_r_rm(RAX, ("r", RDI))
    A.ret()
    A.label("rt_ft4")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.ret()

    # ---- clock: rax = the wall time now in ns since the Unix epoch (clobbers rcx, rdx, rsi,
    # rdi, r11 and `num`; the caller saves them). Linux: clock_gettime(CLOCK_REALTIME);
    # Darwin: gettimeofday (microseconds, scaled), the BSD syscall, not the commpage.
    A.label("rt_clock")
    if mac:
        A.lea(RDI, ("abs", "num", 0))
        A.alu_r_rm("xor", RSI, ("r", RSI), 0)
        A.alu_r_rm("xor", RDX, ("r", RDX), 0)
        A.mov_rm_imm32(("r", RAX), 0x2000000 + 116)
        A.syscall()
        A.mov_r_rm(RCX, ("abs", "num", 8), 0)          # tv_usec (32 bits)
        A.mov_r_rm(RAX, ("r", RCX))
        A.mov_rm_imm32(("r", RCX), 1000)
        A.f7(4, ("r", RCX))
        A.mov_r_rm(RSI, ("r", RAX))                    # usec * 1000
    else:
        A.alu_r_rm("xor", RDI, ("r", RDI), 0)          # CLOCK_REALTIME
        A.lea(RSI, ("abs", "num", 0))
        A.mov_rm_imm32(("r", RAX), 228)
        A.syscall()
        A.mov_r_rm(RSI, ("abs", "num", 8))             # tv_nsec
    A.mov_r_rm(RAX, ("abs", "num", 0))
    A.mov_rm_imm32(("r", RCX), 1000000000)
    A.f7(4, ("r", RCX))
    A.alu_r_rm("add", RAX, ("r", RSI))
    A.ret()

    # ---- THE WAIT: the guest stored to WAIT. Preserves every guest register. If the word two
    # after the SASEVENT tag (the deadline hint, wall ns) is later than now, sleep until it
    # (Linux nanosleep; Darwin poll with no descriptors, whole milliseconds, rounded up);
    # then write the wall time at tag + 8 and append `t=<ns>` to the log (rec_fd), if any.
    A.label("rt_wait")
    for reg in (RSI, RDI, R8, R9, R10, R11):
        A.push(reg)
    A.mov_r_rm(RAX, ("abs", "ev_slot", 0))
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_wt_notag")
    A.mov_r_rm(RCX, ("abs", "rec_fd", 0))              # no YANTRA_RECORD_EVENTS: yantra halts here
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("e", "rt_wt_plain")
    A.mov_r_rm(R9, ("m", RAX, 16))                     # r9 = the hint
    A.test_rm_r(("r", R9), R9)
    A.jcc("e", "rt_wt3")
    A.call("rt_clock")
    A.alu_r_rm("cmp", R9, ("r", RAX))
    A.jcc("be", "rt_wt3")                              # the hint has passed
    A.alu_r_rm("sub", R9, ("r", RAX))                  # r9 = ns to sleep
    # a hint more than about an hour (838 * 2^32 ns) ahead is IGNORED, never slept: yantra
    # ignores every hint and returns at once, so liveness must not depend on the hint's size
    A.mov_r_rm(RAX, ("r", R9))
    A.shift_imm("shr", ("r", RAX), 32)
    A.alu_rm_imm("cmp", ("r", RAX), 838)
    A.jcc("a", "rt_wt3")
    A.mov_r_rm(RAX, ("r", R9))
    if mac:
        A.alu_rm_imm("add", ("r", RAX), 999999)
        A.alu_r_rm("xor", RDX, ("r", RDX), 0)
        A.mov_rm_imm32(("r", RCX), 1000000)
        A.f7(6, ("r", RCX))                            # rax = ms, rounded up
        A.mov_rm_imm32(("r", RCX), 0x7FFFFFFF)
        A.alu_r_rm("cmp", RAX, ("r", RCX))
        A.jcc("be", "rt_wt2")
        A.mov_r_rm(RAX, ("r", RCX))
        A.label("rt_wt2")
        A.mov_r_rm(RDX, ("r", RAX))                    # poll(NULL, 0, ms)
        A.alu_r_rm("xor", RDI, ("r", RDI), 0)
        A.alu_r_rm("xor", RSI, ("r", RSI), 0)
        A.mov_rm_imm32(("r", RAX), 0x2000000 + 230)
        A.syscall()
    else:
        A.alu_r_rm("xor", RDX, ("r", RDX), 0)
        A.mov_rm_imm32(("r", RCX), 1000000000)
        A.f7(6, ("r", RCX))                            # rax = seconds, rdx = ns
        A.mov_rm_r(("abs", "num", 0), RAX)
        A.mov_rm_r(("abs", "num", 8), RDX)
        A.lea(RDI, ("abs", "num", 0))                  # nanosleep(&ts, NULL)
        A.alu_r_rm("xor", RSI, ("r", RSI), 0)
        A.mov_rm_imm32(("r", RAX), 35)
        A.syscall()
    A.label("rt_wt3")
    A.call("rt_clock")
    A.mov_r_rm(RCX, ("abs", "ev_slot", 0))
    A.mov_rm_r(("m", RCX, 8), RAX)
    A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
    A.test_rm_r(("r", RDI), RDI)
    A.jcc("e", "rt_wt4")
    A.lea(RSI, ("abs", "num", 31))                     # "t=<digits>\n", built backwards
    A.raw(0xC6, 0x06, 0x0A)                            # mov byte [rsi], 10
    A.mov_rm_imm32(("r", RCX), 10)
    A.label("rt_wt5")
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.f7(6, ("r", RCX))
    A.lea(RDX, ("m", RDX, 48))
    A.lea(RSI, ("m", RSI, -1))
    A.raw(0x88, 0x16)                                  # mov [rsi], dl
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_wt5")
    A.lea(RSI, ("m", RSI, -1))
    A.raw(0xC6, 0x06, 0x3D)                            # mov byte [rsi], '='
    A.lea(RSI, ("m", RSI, -1))
    A.raw(0xC6, 0x06, 0x74)                            # mov byte [rsi], 't'
    A.lea(RDX, ("abs", "num", 32))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    A.alu_rm_imm("sub", ("r", RDI), 1)
    A.mov_rm_imm32(("r", RAX), sc(1))
    A.syscall()
    A.label("rt_wt4")
    for reg in (R11, R10, R9, R8, RDI, RSI):
        A.pop(reg)
    A.ret()

    # ---- putu: rax as decimal to stderr
    A.label("rt_putu")
    A.lea(RSI, ("abs", "num", 31))
    A.mov_rm_imm32(("r", RCX), 10)
    A.label("rt_pu1")
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.f7(6, ("r", RCX))
    A.lea(RDX, ("m", RDX, 48))
    A.lea(RSI, ("m", RSI, -1))
    A.raw(0x88, 0x16)                              # mov [rsi], dl
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_pu1")
    A.lea(RDX, ("abs", "num", 31))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    A.mov_rm_imm32(("r", RDI), 2)
    sys_(1)
    A.ret()

    # ---- puthex: rax as 0x… hex (minimal digits) to stderr
    A.label("rt_puthex")
    A.lea(RSI, ("abs", "num", 31))
    A.label("rt_px1")
    A.mov_r_rm(RCX, ("r", RAX))
    A.alu_rm_imm("and", ("r", RCX), 15)
    A.alu_rm_imm("cmp", ("r", RCX), 10)
    A.jcc("b", "rt_px2")
    A.alu_rm_imm("add", ("r", RCX), 39)       # 'a'..'f'
    A.label("rt_px2")
    A.alu_rm_imm("add", ("r", RCX), 48)
    A.lea(RSI, ("m", RSI, -1))
    A.raw(0x88, 0x0E)                          # mov [rsi], cl
    A.shift_imm("shr", ("r", RAX), 4)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_px1")
    A.lea(RSI, ("m", RSI, -1))
    A.raw(0xC6, 0x06, 0x78)                    # mov byte [rsi], 'x'
    A.lea(RSI, ("m", RSI, -1))
    A.raw(0xC6, 0x06, 0x30)                    # mov byte [rsi], '0'
    A.lea(RDX, ("abs", "num", 31))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    A.mov_rm_imm32(("r", RDI), 2)
    sys_(1)
    A.ret()

    msg("m_fin1", b"halt: Finisher { value: ")
    msg("m_fin2", b", status: Some(")
    msg("m_fin3", b") }\n")
    msg("m_fin4", b", status: None }\n")
    msg("m_addr", b" addr ")
    msg("m_beyondstore", b"halt: BeyondRam (a store past RAM) at pc ")
    msg("m_badstore", b"halt: BadAccess (a store below 2^31 to no device) at pc ")
    msg("m_codestore", b"native: refused a store into the code span at pc ")
    msg("m_codestore2", b" (yantra would allow it; self-modifying code is not translated)\n")
    msg("m_beyondload", b"halt: BeyondRam (a load past RAM) at pc ")
    msg("m_badload", b"halt: BadAccess (a load below 2^31; MMIO loads are not in milestone 1) at pc ")
    msg("m_bada1", b"halt: BadAccess { pc: ")
    msg("m_bada2", b", addr: ")
    msg("m_bada3", b" }\n")
    msg("m_seg0", b": segment at ")
    msg("m_seg1", b" needs ")
    msg("m_seg2", b" bytes and RAM is ")
    msg("m_seg3", b" \xe2\x80\x94 raise it\n")
    msg("m_insn", b"native: not in milestone 1 (an instruction the translator does not lower) at pc ")
    msg("m_jump1", b"native: jump to ")
    msg("m_jump2", b", outside the translated code (yantra would execute it)\n")
    msg("m_nl", b"\n")
    msg("m_noinput", b"native: YANTRA_INPUT could not be read\n")
    msg("m_noram", b"native: guest RAM could not be mapped at its own address\n")
    msg("m_norec", b"native: YANTRA_RECORD_EVENTS could not be opened for writing\n")
    msg("m_waitnotag", b"native: not in milestone 1 (a WAIT that is no plain clock wait: the SASEVENT tag is missing or duplicated; run it under yantra-run)\n")
    msg("m_waitplain", b"native: not in milestone 1 (a WAIT with no YANTRA_RECORD_EVENTS: yantra-run halts there with exit 75, W-370; run it under yantra-run)\n")
    msg("m_notag", b"native: the image lacks an input tag, or has one twice\n")
    msg("m_threaded", b"native: not in milestone 1 (a threaded image: it carries the SASTHRDS tag, and yantra-run's threads are not native; run it under yantra-run)\n")
    msg("m_entry_both", b"input: YANTRA_INPUT_ENTRY and YANTRA_INPUT_NAME are both set \xe2\x80\x94 the entry form builds the name itself (module NUL routine); set one\n")
    msg("m_entry1", b"input: YANTRA_INPUT_ENTRY ")
    msg("m_long1", b"input: YANTRA_INPUT_ENTRY is ")
    msg("m_long2", b" octets, over the 4096 limit\n")
    msg("m_bad8", b" is not UTF-8 \xe2\x80\x94 module and routine names are UTF-8\n")
    msg("m_noname", b"input: YANTRA_INPUT needs YANTRA_INPUT_NAME (or YANTRA_INPUT_ENTRY) \xe2\x80\x94 the module name is passed, never guessed\n")
    msg("m_entry2", b" is malformed \xe2\x80\x94 it must be \"<module> <routine>\": exactly one ASCII space, both halves non-empty and without whitespace\n")
    if win:
        runtime_win(A)
        runtime_win_spawn(A)
    for lab, text in strings + [("env_ram", b"YANTRA_RAM=\0"), ("env_input", b"YANTRA_INPUT=\0"),
                                ("env_grant", b"YANTRA_GRANT_ENV=\0"), ("env_files", b"YANTRA_FILES=\0"),
                                ("env_name", b"YANTRA_INPUT_NAME=\0"), ("env_trace", b"YANTRA_INPUT_TRACE=\0"),
                                ("env_entry", b"YANTRA_INPUT_ENTRY=\0"),
                                ("env_rec", b"YANTRA_RECORD_EVENTS=\0"), ("rec_hdr", REC_HEADER)]:
        A.label(lab)
        A.raw(text)
    # the code points Rust's {:?} writes as \\u{..} that putq knows (frozen; (lo, hi) as u32
    # pairs, (0, 0) ends): C1, NBSP, soft hyphen, combining diacritics, the Devanagari marks,
    # Unicode spaces and format controls
    A.label("rt_esctab")
    A.raw(b"\x80\x00\x00\x00\xa0\x00\x00\x00\xad\x00\x00\x00\xad\x00\x00\x00\x00\x03\x00\x00\x6f\x03\x00\x00\x00\x09\x00\x00\x02\x09\x00\x00\x3a\x09\x00\x00\x3a\x09\x00\x00\x3c\x09\x00\x00\x3c\x09\x00\x00\x41\x09\x00\x00\x48\x09\x00\x00\x4d\x09\x00\x00\x4d\x09\x00\x00\x51\x09\x00\x00\x57\x09\x00\x00\x62\x09\x00\x00\x63\x09\x00\x00\x80\x16\x00\x00\x80\x16\x00\x00\x00\x20\x00\x00\x0f\x20\x00\x00\x28\x20\x00\x00\x2f\x20\x00\x00\x5f\x20\x00\x00\x6f\x20\x00\x00\x00\x30\x00\x00\x00\x30\x00\x00\xff\xfe\x00\x00\xff\xfe\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00")



class SoftX86:
    """softfp.engine's backend for x86-64: each variable is a quadword of the frame `fsf`; rax rcx rdx only"""
    CC = {"eq": "e", "ne": "ne", "ltu": "b", "leu": "be", "gtu": "a", "geu": "ae", "lts": "l", "les": "le",
          "gts": "g", "ges": "ge"}

    def __init__(self, A):
        self.A, self.slots, self.n = A, {}, 0

    def lab(self):
        self.n += 1
        return "sfl%d" % self.n

    def slot(self, name):
        if name not in self.slots:
            assert len(self.slots) < FSF_SLOTS, name
            self.slots[name] = len(self.slots)
        return ("abs", "fsf", 8 * self.slots[name])

    def ld(self, r, x):
        if isinstance(x, int):
            v = x & M64
            sv = v - (1 << 64) if v >> 63 else v
            if -(1 << 31) <= sv < (1 << 31):
                self.A.mov_rm_imm32(("r", r), sv)
            else:
                self.A.mov_r_imm64(r, v)
        else:
            self.A.mov_r_rm(r, self.slot(x))

    def st(self, d, r):
        self.A.mov_rm_r(self.slot(d), r)

    def label(self, n):
        self.A.label(n)

    def jmp(self, n):
        self.A.jmp(n)

    def call(self, n):
        self.A.call(n)

    def ret(self):
        self.A.ret()

    def set(self, d, imm):
        self.ld(RAX, imm)
        self.st(d, RAX)

    def mov(self, d, a):
        self.ld(RAX, a)
        self.st(d, RAX)

    def _rm(self, b):
        if isinstance(b, int):
            self.ld(RCX, b)
            return ("r", RCX)
        return self.slot(b)

    def _alu(self, op, d, a, b):
        self.ld(RAX, a)
        self.A.alu_r_rm(op, RAX, self._rm(b))
        self.st(d, RAX)

    def add(self, d, a, b):
        self._alu("add", d, a, b)

    def sub(self, d, a, b):
        self._alu("sub", d, a, b)

    def and_(self, d, a, b):
        self._alu("and", d, a, b)

    def or_(self, d, a, b):
        self._alu("or", d, a, b)

    def xor(self, d, a, b):
        self._alu("xor", d, a, b)

    def mul(self, d, a, b):
        self.ld(RAX, a)
        self.A.imul_r_rm(RAX, self._rm(b))
        self.st(d, RAX)

    def umulh(self, d, a, b):
        self.ld(RAX, a)
        self.ld(RCX, b)
        self.A.f7(4, ("r", RCX))
        self.st(d, RDX)

    def _shift(self, name, d, a, b):
        A = self.A
        self.ld(RAX, a)
        if isinstance(b, int):
            if b & M64 >= 64:
                if name == "sar":
                    A.shift_imm("sar", ("r", RAX), 63)
                else:
                    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
            else:
                A.shift_imm(name, ("r", RAX), b)
        else:
            ok, done = self.lab(), self.lab()
            self.ld(RCX, b)
            A.alu_rm_imm("cmp", ("r", RCX), 64)
            A.jcc("b", ok)
            if name == "sar":
                A.shift_imm("sar", ("r", RAX), 63)
            else:
                A.alu_r_rm("xor", RAX, ("r", RAX), 0)
            A.jmp(done)
            A.label(ok)
            A.shift_cl(name, ("r", RAX))
            A.label(done)
        self.st(d, RAX)

    def shl(self, d, a, b):
        self._shift("shl", d, a, b)

    def shr(self, d, a, b):
        self._shift("shr", d, a, b)

    def sar(self, d, a, b):
        self._shift("sar", d, a, b)

    def clz(self, d, a):
        A = self.A
        z, done = self.lab(), self.lab()
        self.ld(RCX, a)
        A.test_rm_r(("r", RCX), RCX)
        A.jcc("e", z)
        A.op(b"\x0f\xbd", RAX, ("r", RCX), 1)        # bsr
        A.alu_rm_imm("xor", ("r", RAX), 63)
        A.jmp(done)
        A.label(z)
        A.mov_rm_imm32(("r", RAX), 64)
        A.label(done)
        self.st(d, RAX)

    def br(self, cond, a, b, lab):
        self.ld(RAX, a)
        self.A.alu_r_rm("cmp", RAX, self._rm(b))
        self.A.jcc(self.CC[cond], lab)

# ---------------------------------------------------------------- the F / D runtime (x86-64)
CAN_D, CAN_S, BOX_HI = 0x7FF8000000000000, 0x7FC00000, 0xFFFFFFFF00000000
REV4 = 0
for _n in range(16):                       # the fflags permutation of MXCSR's PE UE OE ZE bits: a 4-bit reversal
    REV4 |= int("{:04b}".format(_n)[::-1], 2) << (4 * _n)


def runtime_fp(A, soft_all=False, mac=False, no_fma3=False):
    """The F / D helpers. One entry per instruction, fph_<id> (FP_ID), called from the lowering with
         xmm0 xmm1 xmm2 = the raw 64-bit f sources (frs1 frs2 frs3)    rax = the integer source
         ecx = the rounding-mode field (0..4, 7 = DYN)   (the fault site is armed when it is DYN)
       and answering xmm0 (the raw 64-bit f result, a single NaN-boxed) or rax. Only rax rcx rdx and xmm0-7 are
       touched (a helper that wants more saves them). The flags accumulate in [fcsr] (fflags in bits 4..0, frm in
       7..5). IEEE arithmetic is the SSE unit's, in MXCSR's mode (FTZ = DAZ = 0), with the divergences fixed:
       every NaN result is the canonical one, singles are NaN-boxed and read through the box check, the
       conversions to integers saturate, fmin / fmax / fclass / sign injection are bit logic, and RMM (which
       MXCSR lacks) is the software engine's."""
    n = [0]
    stubs = []                             # (label, emitter): the RMM paths, emitted after the helpers
    SF = SoftX86(A)

    def lab():
        n[0] += 1
        return "fpl%d" % n[0]

    def movq_xr(x, r):
        A.sse(0x66, b"\x0f\x6e", x, ("r", r), 1)

    def movq_rx(r, x):
        A.sse(0x66, b"\x0f\x7e", x, ("r", r), 1)

    def movd_xr(x, r):
        A.sse(0x66, b"\x0f\x6e", x, ("r", r), 0)

    def movd_rx(r, x):
        A.sse(0x66, b"\x0f\x7e", x, ("r", r), 0)

    def movq_xx(d, s):
        A.sse(0xF3, b"\x0f\x7e", d, ("r", s), 0)

    def sse_rr(sd, opc, d, s, w=0):                 # scalar op: F2 (double) / F3 (single) 0F opc
        A.sse(0xF2 if sd == "d" else 0xF3, bytes([0x0F, opc]), d, ("r", s), w)

    def mov32(r, v):
        A.mov_rm_imm32(("r", r), sx(v, 32), 0)

    def mov64(r, v):
        A.mov_r_imm64(r, v)

    def cmp32(r, v):
        A.alu_rm_imm("cmp", ("r", r), v, 0)

    def mov_rr(d, s, w=1):
        A.mov_r_rm(d, ("r", s), w)

    def unbox(x):
        ok = lab()
        movq_rx(RAX, x)
        mov_rr(RDX, RAX)
        A.shift_imm("shr", ("r", RDX), 32)
        cmp32(RDX, -1)
        A.jcc("e", ok)
        mov32(RAX, CAN_S)
        A.label(ok)
        movd_xr(x, RAX)

    def box0():
        movd_rx(RAX, 0)
        mov64(RDX, BOX_HI)
        A.alu_r_rm("or", RAX, ("r", RDX))
        movq_xr(0, RAX)

    # ---- sstatus.FS off: yantra's halt for an illegal instruction (no handler: Undelivered, cause 2)
    L1, L2 = b"halt: Undelivered { pc: ", b", cause: 2 }\n"
    A.label("rt_illegal")
    A.call("rt_flush")
    A.call("fs_m1")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, 0x8000_0000)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("fs_m2")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    for nm_, txt in (("fs_m1", L1), ("fs_m2", L2)):
        A.label(nm_)
        A.lea(RSI, ("abs", nm_ + "_s", 0))
        A.mov_rm_imm32(("r", RDX), len(txt))
        A.mov_rm_imm32(("r", RDI), 2)
        x86_sys(A, 1, mac)
        A.ret()
        A.label(nm_ + "_s")
        A.raw(txt)
    # ---- FMA3 present? (CPUID leaf 1: FMA, OSXSAVE, AVX; XCR0 enables xmm and ymm state): fma3 = 1 yes, 2 no
    A.label("fpx_cpuid")
    for r_ in (RBX, RCX, RDX):
        A.push(r_)
    if no_fma3:
        A.mov_rm_imm32(("abs", "fma3", 0), 2, 0)
    else:
        no_, out_ = lab(), lab()
        A.mov_rm_imm32(("r", RAX), 1, 0)
        A.raw(0x0F, 0xA2)                                  # cpuid
        A.mov_r_rm(RDX, ("r", RCX), 0)
        A.alu_rm_imm("and", ("r", RDX), (1 << 12) | (1 << 27) | (1 << 28), 0)
        A.alu_rm_imm("cmp", ("r", RDX), (1 << 12) | (1 << 27) | (1 << 28), 0)
        A.jcc("ne", no_)
        A.alu_r_rm("xor", RCX, ("r", RCX), 0)
        A.raw(0x0F, 0x01, 0xD0)                            # xgetbv
        A.alu_rm_imm("and", ("r", RAX), 6, 0)
        A.alu_rm_imm("cmp", ("r", RAX), 6, 0)
        A.jcc("ne", no_)
        A.mov_rm_imm32(("abs", "fma3", 0), 1, 0)
        A.jmp(out_)
        A.label(no_)
        A.mov_rm_imm32(("abs", "fma3", 0), 2, 0)
        A.label(out_)
    for r_ in (RDX, RCX, RBX):
        A.pop(r_)
    A.ret()

    # ---- the shared routines
    A.label("fpx_rm")                      # ecx = the rm field -> the effective mode 0..4; reserved: illegal instruction
    ok = lab()
    cmp32(RCX, 7)
    A.jcc("ne", ok)
    A.mov_r_rm(RCX, ("abs", "fcsr", 0), 0)
    A.shift_imm("shr", ("r", RCX), 5, 0)
    A.alu_rm_imm("and", ("r", RCX), 7, 0)
    A.label(ok)
    cmp32(RCX, 4)
    A.jcc("a", "rt_bad_insn")
    A.ret()

    A.label("fpx_enter")                   # ecx = 0..3 (RNE RTZ RDN RUP): MXCSR = default with that RC, flags clear
    A.shift_imm("shl", ("r", RCX), 1, 0)
    mov32(RDX, 0x9C)                       # RC by mode: 0, 3, 1, 2 in two-bit fields
    A.shift_cl("shr", ("r", RDX), 0)
    A.alu_rm_imm("and", ("r", RDX), 3, 0)
    A.shift_imm("shl", ("r", RDX), 13, 0)
    A.alu_rm_imm("or", ("r", RDX), 0x1F80, 0)
    A.push(RDX)
    A.op(b"\x0f\xae", 2, ("m", RSP, 0), 0)         # ldmxcsr [rsp]
    A.pop(RDX)
    A.ret()

    A.label("fpx_flags")                   # fcsr |= fflags(MXCSR); keeps rax
    A.push(RAX)
    A.push(RAX)                            # the scratch slot for stmxcsr
    A.op(b"\x0f\xae", 3, ("m", RSP, 0), 0)         # stmxcsr [rsp]
    A.mov_r_rm(RAX, ("m", RSP, 0), 0)
    mov_rr(RCX, RAX, 0)
    A.alu_rm_imm("and", ("r", RAX), 1, 0)          # IE -> NV (bit 4)
    A.shift_imm("shl", ("r", RAX), 4, 0)
    A.shift_imm("shr", ("r", RCX), 2, 0)           # ZE OE UE PE (bits 2..5) -> DZ OF UF NX (bits 3..0): reversed
    A.alu_rm_imm("and", ("r", RCX), 15, 0)
    A.shift_imm("shl", ("r", RCX), 2, 0)
    mov64(RDX, REV4)
    A.shift_cl("shr", ("r", RDX), 1)
    A.alu_rm_imm("and", ("r", RDX), 15, 0)
    A.alu_r_rm("or", RAX, ("r", RDX), 0)
    A.mov_r_rm(RDX, ("abs", "fcsr", 0), 0)
    A.alu_r_rm("or", RDX, ("r", RAX), 0)
    A.mov_rm_r(("abs", "fcsr", 0), RDX, width=4)
    A.alu_rm_imm("add", ("r", RSP), 8)
    A.pop(RAX)
    A.ret()

    A.label("fpx_done_d")                  # a double result in xmm0: flags, the canonical NaN
    A.call("fpx_flags")
    ok = lab()
    A.sse(0x66, b"\x0f\x2e", 0, ("r", 0), 0)       # ucomisd xmm0, xmm0
    A.jcc("np", ok)
    mov64(RAX, CAN_D)
    movq_xr(0, RAX)
    A.label(ok)
    A.ret()

    A.label("fpx_done_s")                  # a single result in xmm0: flags, the canonical NaN, boxed
    A.call("fpx_flags")
    ok = lab()
    A.sse(0, b"\x0f\x2e", 0, ("r", 0), 0)          # ucomiss xmm0, xmm0
    A.jcc("np", ok)
    mov32(RAX, CAN_S)
    movd_xr(0, RAX)
    A.label(ok)
    box0()
    A.ret()

    A.label("fpx_box_s")                   # box xmm0 (a single in its low 32 bits)
    box0()
    A.ret()

    def soft_branch(*kind):                # ecx = 4 (RMM) goes to the software engine (soft_all: every mode does)
        name = "fps_%d" % len(stubs)
        stubs.append((name, kind))
        if soft_all:
            A.jmp(name)
        else:
            cmp32(RCX, 4)
            A.jcc("e", name)
        return name

    def helper(name):
        A.label("fph_%d" % FP_ID[name])

    # ---- fadd fsub fmul fdiv
    for nm, opc in (("fadd", 0x58), ("fsub", 0x5C), ("fmul", 0x59), ("fdiv", 0x5E)):
        for sd in "sd":
            helper("%s.%s" % (nm, sd))
            if sd == "s":
                unbox(0)
                unbox(1)
            A.call("fpx_rm")
            soft_branch("bin", {0x58: "sf_e_add", 0x5C: "sf_e_sub", 0x59: "sf_e_mul", 0x5E: "sf_e_div"}[opc], sd)
            A.call("fpx_enter")
            sse_rr(sd, opc, 0, 1)
            A.jmp("fpx_done_" + sd)
    # ---- fsqrt
    for sd in "sd":
        helper("fsqrt." + sd)
        if sd == "s":
            unbox(0)
        A.call("fpx_rm")
        soft_branch("sqrt", sd)
        A.call("fpx_enter")
        sse_rr(sd, 0x51, 0, 0)
        A.jmp("fpx_done_" + sd)
    # ---- fcvt.s.d (rounds) and fcvt.d.s (exact; only the flags and the canonical NaN)
    helper("fcvt.s.d")
    A.call("fpx_rm")
    soft_branch("cvtds")
    A.call("fpx_enter")
    A.sse(0xF2, b"\x0f\x5a", 0, ("r", 0), 0)
    A.jmp("fpx_done_s")
    helper("fcvt.d.s")
    unbox(0)
    A.call("fpx_rm")
    ok = lab()
    cmp32(RCX, 4)
    A.jcc("ne", ok)
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.label(ok)
    A.call("fpx_enter")
    A.sse(0xF3, b"\x0f\x5a", 0, ("r", 0), 0)
    A.jmp("fpx_done_d")
    # ---- the fused multiply-adds: vfmadd213 / vfmsub213 / vfnmadd213 / vfnmsub213 xmm0, xmm1, xmm2
    for nm, opc in (("fmadd", 0xA9), ("fmsub", 0xAB), ("fnmsub", 0xAD), ("fnmadd", 0xAF)):
        for sd in "sd":
            helper("%s.%s" % (nm, sd))
            if sd == "s":
                unbox(0)
                unbox(1)
                unbox(2)
            A.call("fpx_rm")
            stub = soft_branch("fma", sd, 1 if nm in ("fnmsub", "fnmadd") else 0, 1 if nm in ("fmsub", "fnmadd") else 0)
            have = lab()                    # no FMA3 on this host (CPUID, once): the engine does it, exactly
            A.alu_rm_imm("cmp", ("abs", "fma3", 0), 1, 0)
            A.jcc("e", have)
            A.alu_rm_imm("cmp", ("abs", "fma3", 0), 0, 0)
            A.jcc("ne", stub)
            A.call("fpx_cpuid")
            A.alu_rm_imm("cmp", ("abs", "fma3", 0), 1, 0)
            A.jcc("ne", stub)
            A.label(have)
            A.call("fpx_enter")
            # inf * 0 is invalid even when the addend is a quiet NaN (the SSE unit does not say so then)
            w = 1 if sd == "d" else 0
            inf2 = 0xFFE0000000000000 if sd == "d" else 0xFF000000
            movq_rx(RAX, 0)
            movq_rx(RDX, 1)
            A.shift_imm("shl", ("r", RAX), 1, w)
            A.shift_imm("shl", ("r", RDX), 1, w)
            if sd == "d":
                mov64(RCX, inf2)
            else:
                mov32(RCX, inf2)
            nz, setnv, end = lab(), lab(), lab()
            A.alu_r_rm("cmp", RAX, ("r", RCX), w)
            A.jcc("ne", nz)
            A.test_rm_r(("r", RDX), RDX, w)             # a = inf: b = 0 ?
            A.jcc("e", setnv)
            A.jmp(end)
            A.label(nz)
            A.test_rm_r(("r", RAX), RAX, w)            # a = 0: b = inf ?
            A.jcc("ne", end)
            A.alu_r_rm("cmp", RDX, ("r", RCX), w)
            A.jcc("ne", end)
            A.label(setnv)
            A.mov_r_rm(RCX, ("abs", "fcsr", 0), 0)
            A.alu_rm_imm("or", ("r", RCX), 16, 0)
            A.mov_rm_r(("abs", "fcsr", 0), RCX, width=4)
            A.label(end)
            A.raw(0xC4, 0xE2, 0xF1 if sd == "d" else 0x71, opc, 0xC2)       # vfmXXX213s? xmm0, xmm1, xmm2
            A.jmp("fpx_done_" + sd)
    # ---- sign injection, fmin / fmax
    for nm in ("fsgnj", "fsgnjn", "fsgnjx"):
        for sd in "sd":
            helper("%s.%s" % (nm, sd))
            if sd == "d":
                movq_rx(RAX, 0)
                movq_rx(RDX, 1)
                mov64(RCX, 1 << 63)
            else:
                movd_rx(RAX, 0)                         # the raw low 32 bits: no box check
                movd_rx(RDX, 1)
                mov32(RCX, 1 << 31)
            w = 1 if sd == "d" else 0
            if nm == "fsgnj":
                A.alu_r_rm("and", RDX, ("r", RCX), w)       # the sign of b
                A.f7(2, ("r", RCX), w)                      # not rcx: everything but the sign
                A.alu_r_rm("and", RAX, ("r", RCX), w)
                A.alu_r_rm("or", RAX, ("r", RDX), w)
            elif nm == "fsgnjn":
                A.f7(2, ("r", RDX), w)                      # not b, then its sign
                A.alu_r_rm("and", RDX, ("r", RCX), w)
                A.f7(2, ("r", RCX), w)
                A.alu_r_rm("and", RAX, ("r", RCX), w)
                A.alu_r_rm("or", RAX, ("r", RDX), w)
            else:
                A.alu_r_rm("and", RDX, ("r", RCX), w)
                A.alu_r_rm("xor", RAX, ("r", RDX), w)
            if sd == "d":
                movq_xr(0, RAX)
                A.ret()
            else:
                movd_xr(0, RAX)
                A.jmp("fpx_box_s")
    for nm, is_max in (("fmin", 0), ("fmax", 1)):
        for sd in "sd":
            helper("%s.%s" % (nm, sd))
            if sd == "s":
                unbox(0)
                unbox(1)
            pf = 0x66 if sd == "d" else 0

            def ucom(a_, b_):
                A.sse(pf, b"\x0f\x2e", a_, ("r", b_), 0)
            A.alu_r_rm("xor", RCX, ("r", RCX), 0)
            A.call("fpx_enter")
            ucom(0, 1)                                   # IE only for a signalling NaN
            A.call("fpx_flags")
            ucom(0, 1)                                   # again, for the branches
            unordered, equal, aless, b_only_nan, a_nan = lab(), lab(), lab(), lab(), lab()
            fin = "fpx_fin_" + sd
            A.jcc("p", unordered)
            A.jcc("e", equal)
            A.jcc("b", aless)
            if not is_max:                               # a > b: min is b
                movq_xx(0, 1)
            A.jmp(fin)
            A.label(aless)                               # a < b: max is b
            if is_max:
                movq_xx(0, 1)
            A.jmp(fin)
            A.label(equal)                               # equal: only +0 and -0 differ in bits; min is the negative
            movq_rx(RAX, 0)
            movq_rx(RDX, 1)
            A.alu_r_rm("and" if is_max else "or", RAX, ("r", RDX), 1)
            movq_xr(0, RAX)
            A.jmp(fin)
            A.label(unordered)
            ucom(0, 0)
            A.jcc("p", a_nan)
            A.jmp(fin)                                   # only b is a NaN: the answer is a (already xmm0)
            A.label(a_nan)
            ucom(1, 1)
            A.jcc("np", b_only_nan)
            if sd == "d":                                # both: the canonical NaN
                mov64(RAX, CAN_D)
                movq_xr(0, RAX)
            else:
                mov32(RAX, CAN_S)
                movd_xr(0, RAX)
            A.jmp(fin)
            A.label(b_only_nan)                          # only a is a NaN: the answer is b
            movq_xx(0, 1)
            A.jmp(fin)
    A.label("fpx_fin_d")
    A.ret()
    A.label("fpx_fin_s")
    A.jmp("fpx_box_s")
    # ---- comparisons: feq (quiet) flt fle (signalling)
    for nm in ("feq", "flt", "fle"):
        for sd in "sd":
            helper("%s.%s" % (nm, sd))
            if sd == "s":
                unbox(0)
                unbox(1)
            A.alu_r_rm("xor", RCX, ("r", RCX), 0)
            A.call("fpx_enter")
            pfx = 0x66 if sd == "d" else 0
            if nm == "feq":
                A.sse(pfx, b"\x0f\x2e", 0, ("r", 1), 0)       # ucomis? xmm0, xmm1
                A.op(bytes([0x0F, 0x90 | CC["e"]]), 0, ("r", RAX), 0)
                A.op(bytes([0x0F, 0x90 | CC["np"]]), 0, ("r", RCX), 0)
                A.movzx(RAX, ("r", RAX), 1)
                A.movzx(RCX, ("r", RCX), 1)
                A.alu_r_rm("and", RAX, ("r", RCX), 0)
            else:
                A.sse(pfx, b"\x0f\x2f", 1, ("r", 0), 0)       # comis? xmm1, xmm0: b > a (flt) / b >= a (fle)
                A.op(bytes([0x0F, 0x90 | CC["a" if nm == "flt" else "ae"]]), 0, ("r", RAX), 0)
                A.movzx(RAX, ("r", RAX), 1)
            A.call("fpx_flags")
            A.ret()
    # ---- fclass, fmv.x.*, fmv.*.x
    for sd in "sd":
        helper("fclass." + sd)
        if sd == "s":
            unbox(0)
            movd_rx(RAX, 0)
            wd, expm, quiet, hidden = 31, 0x7F800000, 1 << 22, 1 << 23
            A.mov_r_rm(RCX, ("r", RAX), 0)
            A.shift_imm("shr", ("r", RCX), 31, 0)
            A.alu_rm_imm("and", ("r", RAX), 0x7FFFFFFF, 0)
            mov32(RDX, expm)
            w = 0
        else:
            movq_rx(RAX, 0)
            wd, expm, quiet, hidden = 63, 0x7FF0000000000000, 1 << 51, 1 << 52
            mov_rr(RCX, RAX)
            A.shift_imm("shr", ("r", RCX), 63)
            A.shift_imm("shl", ("r", RAX), 1)
            A.shift_imm("shr", ("r", RAX), 1)
            mov64(RDX, expm)
            w = 1
        isnan, isinf, subz, ret = lab(), lab(), lab(), lab()
        A.alu_r_rm("cmp", RAX, ("r", RDX), w)
        A.jcc("a", isnan)
        A.jcc("e", isinf)
        if sd == "s":
            mov32(RDX, hidden)
        else:
            mov64(RDX, hidden)
        A.alu_r_rm("cmp", RAX, ("r", RDX), w)
        A.jcc("b", subz)

        def pick(pos, neg):
            nn = lab()
            mov32(RAX, 1 << pos)
            A.test_rm_r(("r", RCX), RCX, 0)
            A.jcc("e", nn)
            mov32(RAX, 1 << neg)
            A.label(nn)
        pick(6, 1)                                         # normal
        A.ret()
        A.label(subz)
        zero = lab()
        A.test_rm_r(("r", RAX), RAX, w)
        A.jcc("e", zero)
        pick(5, 2)                                         # subnormal
        A.ret()
        A.label(zero)
        pick(4, 3)
        A.ret()
        A.label(isinf)
        pick(7, 0)
        A.ret()
        A.label(isnan)
        if sd == "s":
            A.alu_rm_imm("and", ("r", RAX), quiet, 0)
        else:
            mov64(RDX, quiet)
            A.alu_r_rm("and", RAX, ("r", RDX), 1)
        qn = lab()
        A.test_rm_r(("r", RAX), RAX, w)
        A.jcc("ne", qn)
        mov32(RAX, 1 << 8)
        A.ret()
        A.label(qn)
        mov32(RAX, 1 << 9)
        A.ret()
    helper("fmv.x.w")
    movd_rx(RAX, 0)
    A.movsx(RAX, ("r", RAX), 4)
    A.ret()
    helper("fmv.x.d")
    movq_rx(RAX, 0)
    A.ret()
    helper("fmv.w.x")
    movd_xr(0, RAX)
    A.jmp("fpx_box_s")
    helper("fmv.d.x")
    movq_xr(0, RAX)
    A.ret()
    # ---- integer -> float: rax = the integer; one rounding in the mode (MXCSR); lu halves with a sticky bit
    for sd in "sd":
        for t, tn in enumerate(("w", "wu", "l", "lu")):
            helper("fcvt.%s.%s" % (sd, tn))
            A.call("fpx_rm")
            soft_branch("fromint", sd, tn)
            A.call("fpx_enter")
            if tn == "w":
                A.movsx(RAX, ("r", RAX), 4)
            elif tn == "wu":
                mov_rr(RAX, RAX, 0)
            cvt = (0xF2 if sd == "d" else 0xF3, b"\x0f\x2a")
            if tn == "lu":
                direct = lab()
                A.test_rm_r(("r", RAX), RAX)
                A.jcc("ns", direct)
                mov_rr(RDX, RAX)
                A.shift_imm("shr", ("r", RAX), 1)
                A.alu_rm_imm("and", ("r", RDX), 1)
                A.alu_r_rm("or", RAX, ("r", RDX))
                A.sse(cvt[0], cvt[1], 0, ("r", RAX), 1)
                sse_rr(sd, 0x58, 0, 0)                  # x 2: exact
                A.jmp("fpx_done_" + sd)
                A.label(direct)
            A.sse(cvt[0], cvt[1], 0, ("r", RAX), 1)
            A.jmp("fpx_done_" + sd)
    # ---- float -> integer: all five modes, in integers (fpx_f2i); a single is widened first (exact)
    for sd in "sd":
        for t, tn in enumerate(("w", "wu", "l", "lu")):
            helper("fcvt.%s.%s" % (tn, sd))
            if sd == "s":
                unbox(0)
                A.sse(0xF3, b"\x0f\x5a", 0, ("r", 0), 0)            # cvtss2sd (exact)
            movq_rx(RAX, 0)
            A.call("fpx_rm")
            mov32(RDX, t)
            A.jmp("fpx_f2i")
    A.label("fpx_f2i")                     # rax = the double's bits, ecx = mode 0..4, edx = type (w wu l lu)
    for r in (RBX, RSI, RDI, R8, R9, R10, R11):
        A.push(r)
    # r8 = x, r9 = sign, r10 = q (the rounded magnitude), rsi = inexact, rdi = type, r11 = mode, rbx = scratch
    mov_rr(R8, RAX)
    mov_rr(RDI, RDX)
    mov_rr(R11, RCX)
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)
    mov_rr(R9, R8)
    A.shift_imm("shr", ("r", R9), 63)
    mov_rr(RDX, R8)
    A.shift_imm("shr", ("r", RDX), 52)
    A.alu_rm_imm("and", ("r", RDX), 0x7FF, 0)
    fin, isnan, big, small, frac, haveq, up_done, invalid = (lab() for _ in range(8))
    cmp32(RDX, 0x7FF)
    A.jcc("ne", fin)
    mov_rr(RAX, R8)
    A.shift_imm("shl", ("r", RAX), 12)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", isnan)
    A.jmp(invalid)                                       # +-inf: saturate by its sign
    A.label(isnan)
    A.alu_r_rm("xor", R9, ("r", R9), 0)                  # NaN -> the maximum
    A.jmp(invalid)
    A.label(fin)
    cmp32(RDX, 1023)
    A.jcc("b", small)
    mov_rr(RBX, R8)                                      # M = mantissa | 1 << 52
    A.shift_imm("shl", ("r", RBX), 12)
    A.shift_imm("shr", ("r", RBX), 12)
    mov32(RAX, 1)
    A.shift_imm("shl", ("r", RAX), 52)
    A.alu_r_rm("or", RBX, ("r", RAX))
    A.alu_rm_imm("sub", ("r", RDX), 1075)                # s = e - 1075
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("s", frac)
    A.alu_rm_imm("cmp", ("r", RDX), 11)
    A.jcc("g", invalid)                                  # >= 2^64 (or > 2^63 + ...): out of every range
    mov_rr(RCX, RDX)
    A.shift_cl("shl", ("r", RBX))
    mov_rr(R10, RBX)
    A.jmp(haveq)
    A.label(frac)                                        # r = -s fractional bits (1..52)
    A.f7(3, ("r", RDX))                                  # neg
    mov_rr(RCX, RDX)
    mov_rr(R10, RBX)
    A.shift_cl("shr", ("r", R10))                        # q
    mov32(RAX, 1)
    A.shift_cl("shl", ("r", RAX))
    A.alu_rm_imm("sub", ("r", RAX), 1)
    A.alu_r_rm("and", RAX, ("r", RBX))                   # rem
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", haveq)
    mov32(RSI, 1)
    mov32(RDX, 1)
    A.alu_rm_imm("sub", ("r", RCX), 1)
    A.shift_cl("shl", ("r", RDX))                        # half
    # up?  by mode
    rne, rtz, rdn, rup, rmm = (lab() for _ in range(5))
    cmp32(R11, 1)
    A.jcc("e", up_done)                                  # RTZ: nothing
    A.jcc("l", rne)                                      # RNE
    cmp32(R11, 2)
    A.jcc("e", rdn)
    cmp32(R11, 3)
    A.jcc("e", rup)
    A.alu_r_rm("cmp", RAX, ("r", RDX))                   # RMM: rem >= half
    A.jcc("b", up_done)
    A.alu_rm_imm("add", ("r", R10), 1)
    A.jmp(up_done)
    A.label(rne)
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    A.jcc("b", up_done)
    A.jcc("a", rtz)
    mov_rr(RBX, R10)                                     # tie: up iff q is odd
    A.alu_rm_imm("and", ("r", RBX), 1)
    A.jcc("e", up_done)
    A.label(rtz)
    A.alu_rm_imm("add", ("r", R10), 1)
    A.jmp(up_done)
    A.label(rdn)                                         # up iff the sign is negative
    A.test_rm_r(("r", R9), R9)
    A.jcc("e", up_done)
    A.alu_rm_imm("add", ("r", R10), 1)
    A.jmp(up_done)
    A.label(rup)
    A.test_rm_r(("r", R9), R9)
    A.jcc("ne", up_done)
    A.alu_rm_imm("add", ("r", R10), 1)
    A.jmp(up_done)
    A.label(up_done)
    A.jmp(haveq)
    A.label(small)                                       # |x| < 1: q is 0 or 1
    A.alu_r_rm("xor", R10, ("r", R10))
    mov_rr(RAX, R8)
    A.shift_imm("shl", ("r", RAX), 1)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", haveq)                                    # +-0: exact
    mov32(RSI, 1)
    # up by mode: RTZ no; RDN sign; RUP !sign; RNE |x| > 0.5; RMM |x| >= 0.5.  (e == 1022 means |x| >= 0.5)
    s_rne, s_rdn, s_rup, s_up = (lab() for _ in range(4))
    cmp32(R11, 1)
    A.jcc("e", haveq)
    A.jcc("l", s_rne)
    cmp32(R11, 2)
    A.jcc("e", s_rdn)
    cmp32(R11, 3)
    A.jcc("e", s_rup)
    cmp32(RDX, 1022)                                     # RMM
    A.jcc("e", s_up)
    A.jmp(haveq)
    A.label(s_rne)
    cmp32(RDX, 1022)
    A.jcc("ne", haveq)
    mov_rr(RAX, R8)                                      # |x| > 0.5 iff the mantissa is not zero
    A.shift_imm("shl", ("r", RAX), 12)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", haveq)
    A.jmp(s_up)
    A.label(s_rdn)
    A.test_rm_r(("r", R9), R9)
    A.jcc("e", haveq)
    A.jmp(s_up)
    A.label(s_rup)
    A.test_rm_r(("r", R9), R9)
    A.jcc("ne", haveq)
    A.label(s_up)
    mov32(R10, 1)
    # ---- the magnitude q is in r10: range-check by type, apply the sign
    A.label(haveq)
    ok, okw, done = lab(), lab(), lab()
    for t in range(4):
        nxt, pos = lab(), lab()
        cmp32(RDI, t)
        A.jcc("ne", nxt)
        target = okw if t < 2 else ok
        A.test_rm_r(("r", R9), R9)
        A.jcc("e", pos)
        if t in (1, 3):                                  # unsigned and negative: only 0 converts (to 0)
            A.test_rm_r(("r", R10), R10)
            A.jcc("ne", invalid)
        else:                                            # signed and negative: q <= 2^(n-1)
            mov64(RAX, (1 << 31) if t == 0 else (1 << 63))
            A.alu_r_rm("cmp", R10, ("r", RAX))
            A.jcc("a", invalid)
        A.jmp(target)
        A.label(pos)
        if t == 3:
            A.jmp(target)                                # any q < 2^64
        else:
            mov64(RAX, (0x7FFFFFFF, 0xFFFFFFFF, 0x7FFFFFFFFFFFFFFF)[t])
            A.alu_r_rm("cmp", R10, ("r", RAX))
            A.jcc("a", invalid)
            A.jmp(target)
        A.label(nxt)
    A.label(ok)                                          # the 64-bit result: -q or q
    mov_rr(RAX, R10)
    pz = lab()
    A.test_rm_r(("r", R9), R9)
    A.jcc("e", pz)
    A.f7(3, ("r", RAX))
    A.label(pz)
    A.jmp(done)
    A.label(okw)                                         # a 32-bit type: the result sign-extends (wu too)
    mov_rr(RAX, R10)
    pw = lab()
    A.test_rm_r(("r", R9), R9)
    A.jcc("e", pw)
    A.f7(3, ("r", RAX))
    A.label(pw)
    A.movsx(RAX, ("r", RAX), 4)
    A.label(done)
    A.test_rm_r(("r", RSI), RSI)                         # inexact: NX
    fl_out = lab()
    A.jcc("e", fl_out)
    A.mov_r_rm(RCX, ("abs", "fcsr", 0), 0)
    A.alu_rm_imm("or", ("r", RCX), 1, 0)
    A.mov_rm_r(("abs", "fcsr", 0), RCX, width=4)
    A.label(fl_out)
    out = lab()
    A.jmp(out)
    A.label(invalid)                                     # saturate: NV alone; r9 = sign (0 for NaN)
    A.mov_r_rm(RCX, ("abs", "fcsr", 0), 0)
    A.alu_rm_imm("or", ("r", RCX), 16, 0)
    A.mov_rm_r(("abs", "fcsr", 0), RCX, width=4)
    for t in range(4):
        nxt = lab()
        cmp32(RDI, t)
        A.jcc("ne", nxt)
        pos = lab()
        A.test_rm_r(("r", R9), R9)
        A.jcc("e", pos)
        mov64(RAX, (0xFFFFFFFF80000000, 0, 0x8000000000000000, 0)[t])
        A.jmp(out)
        A.label(pos)
        mov64(RAX, (0x7FFFFFFF, M64, 0x7FFFFFFFFFFFFFFF, M64)[t])
        A.jmp(out)
        A.label(nxt)
    A.label(out)
    for r in (R11, R10, R9, R8, RDI, RSI, RBX):
        A.pop(r)
    A.ret()
    # ---- the float CSRs: rax = the source, ecx = csr (1 fflags, 2 frm, 3 fcsr) | kind << 4 (1 w, 2 s, 3 c); rax = old
    A.label("fph_csr")
    A.push(RBX)
    A.push(RSI)
    mov_rr(RBX, RCX, 0)
    A.alu_rm_imm("and", ("r", RBX), 15, 0)
    A.shift_imm("shr", ("r", RCX), 4, 0)
    A.mov_r_rm(RDX, ("abs", "fcsr", 0), 0)
    mov_rr(RSI, RDX, 0)
    c2, c3, oldok = lab(), lab(), lab()
    cmp32(RBX, 1)
    A.jcc("ne", c2)
    A.alu_rm_imm("and", ("r", RSI), 0x1F, 0)
    A.jmp(oldok)
    A.label(c2)
    cmp32(RBX, 2)
    A.jcc("ne", c3)
    A.shift_imm("shr", ("r", RSI), 5, 0)
    A.alu_rm_imm("and", ("r", RSI), 7, 0)
    A.jmp(oldok)
    A.label(c3)
    A.alu_rm_imm("and", ("r", RSI), 0xFF, 0)
    A.label(oldok)
    newok, k3 = lab(), lab()
    cmp32(RCX, 1)
    A.jcc("e", newok)
    cmp32(RCX, 2)
    A.jcc("ne", k3)
    A.alu_r_rm("or", RAX, ("r", RSI))
    A.jmp(newok)
    A.label(k3)
    A.f7(2, ("r", RAX))
    A.alu_r_rm("and", RAX, ("r", RSI))
    A.label(newok)
    w2, w3, wd = lab(), lab(), lab()
    cmp32(RBX, 1)
    A.jcc("ne", w2)
    A.alu_rm_imm("and", ("r", RAX), 0x1F, 0)
    A.alu_rm_imm("and", ("r", RDX), 0xE0, 0)
    A.alu_r_rm("or", RDX, ("r", RAX), 0)
    A.jmp(wd)
    A.label(w2)
    cmp32(RBX, 2)
    A.jcc("ne", w3)
    A.alu_rm_imm("and", ("r", RAX), 7, 0)
    A.shift_imm("shl", ("r", RAX), 5, 0)
    A.alu_rm_imm("and", ("r", RDX), 0x1F, 0)
    A.alu_r_rm("or", RDX, ("r", RAX), 0)
    A.jmp(wd)
    A.label(w3)
    mov_rr(RDX, RAX, 0)
    A.alu_rm_imm("and", ("r", RDX), 0xFF, 0)
    A.label(wd)
    A.mov_rm_r(("abs", "fcsr", 0), RDX, width=4)
    mov_rr(RAX, RSI)
    A.pop(RSI)
    A.pop(RBX)
    A.ret()

    # ---- the RMM paths: the software engine (softfp.py) runs the operation; its answer is xmm0 / a boxed single
    def sv(name):
        return SF.slot(name)

    def put_sf(name, r):
        A.mov_rm_r(sv(name), r)

    def stub_tail(sd, fl_first=False):
        A.jmp("fpx_stail_" + sd)

    for name, kind in stubs:
        A.label(name)
        k = kind[0]
        if k == "bin":
            fn, sd = kind[1], kind[2]
            movq_rx(RAX, 0)
            put_sf("sf_a", RAX)
            movq_rx(RAX, 1)
            put_sf("sf_b", RAX)
        elif k == "sqrt":
            fn, sd = "sf_e_sqrt", kind[1]
            movq_rx(RAX, 0)
            put_sf("sf_a", RAX)
        elif k == "cvtds":
            fn, sd = "sf_e_cvt_ds", "s"
            movq_rx(RAX, 0)
            put_sf("sf_a", RAX)
        elif k == "fma":
            fn, sd = "sf_e_fma", kind[1]
            movq_rx(RAX, 0)
            put_sf("sf_a", RAX)
            movq_rx(RAX, 1)
            put_sf("sf_b", RAX)
            movq_rx(RAX, 2)
            put_sf("sf_c", RAX)
            A.mov_rm_imm32(sv("ng_p"), kind[2])
            A.mov_rm_imm32(sv("ng_a"), kind[3])
        else:                              # fromint: rax = the integer; fi_neg / fi_mag
            fn, sd, tn = "sf_e_fromint", kind[1], kind[2]
            if tn == "w":
                A.movsx(RAX, ("r", RAX), 4)
            elif tn == "wu":
                mov_rr(RAX, RAX, 0)
            if tn in ("w", "l"):
                mov_rr(RDX, RAX)
                A.shift_imm("shr", ("r", RDX), 63)
                A.mov_rm_r(sv("fi_neg"), RDX)
                A.test_rm_r(("r", RAX), RAX)
                pz = lab()
                A.jcc("ns", pz)
                A.f7(3, ("r", RAX))             # |x| (-2^63 stays 2^63 as a magnitude)
                A.label(pz)
            else:
                A.mov_rm_imm32(sv("fi_neg"), 0)
            put_sf("fi_mag", RAX)
        mov_rr(RAX, RCX, 0)                    # the mode, zero-extended
        put_sf("rp_rm", RAX)
        A.mov_rm_imm32(sv("fl"), 0)
        if k != "cvtds":
            A.call("sf_fmt_" + sd)
        A.call(fn)
        A.jmp("fpx_stail_" + sd)
    A.label("fpx_stail_d")
    A.mov_r_rm(RAX, sv("fl"), 0)
    A.mov_r_rm(RDX, ("abs", "fcsr", 0), 0)
    A.alu_r_rm("or", RDX, ("r", RAX), 0)
    A.mov_rm_r(("abs", "fcsr", 0), RDX, width=4)
    A.mov_r_rm(RAX, sv("sf_res"))
    movq_xr(0, RAX)
    A.ret()
    A.label("fpx_stail_s")
    A.mov_r_rm(RAX, sv("fl"), 0)
    A.mov_r_rm(RDX, ("abs", "fcsr", 0), 0)
    A.alu_r_rm("or", RDX, ("r", RAX), 0)
    A.mov_rm_r(("abs", "fcsr", 0), RDX, width=4)
    A.mov_r_rm(RAX, sv("sf_res"))
    movd_xr(0, RAX)
    A.jmp("fpx_box_s")
    softfp.engine(SF)



def elf(A, data_end):
    """ELF64, x86-64, ET_EXEC: PT_LOAD text (headers + code + table + image, R X) at TEXT,
    PT_LOAD data (bss only, R W) after it."""
    text = bytearray(0x1000) + A.b
    tsize = len(text)
    dstart = (TEXT + tsize + 0xFFF) & ~0xFFF
    hdr = struct.pack("<4sBBBBB7xHHIQQQIHHHHHH", b"\x7fELF", 2, 1, 1, 0, 0, 2, 62, 1,
                      A.labels["rt_start"], 64, 0, 0, 64, 56, 2, 64, 0, 0)
    ph1 = struct.pack("<IIQQQQQQ", 1, 5, 0, TEXT, TEXT, tsize, tsize, 0x1000)
    ph2 = struct.pack("<IIQQQQQQ", 1, 6, 0, dstart, dstart, 0, data_end - dstart, 0x1000)
    text[0:len(hdr)] = hdr
    text[64:64 + 56] = ph1
    text[120:120 + 56] = ph2
    return bytes(text)



# ================================================================ the AArch64 target
# `--target aarch64-linux`: the same guest model, lowered to A64 (fixed 32-bit words), with
# its own encoder, its own runtime in A64 machine code (Linux arm64 syscalls through svc #0,
# x8 = the number: 63 read, 64 write, 56 openat, 80 fstat, 222 mmap, 93 exit) and its own
# ELF writer (EM_AARCH64). Lowering: the x86-64 lowering's semantics, with these A64 forms:
#   div/divu: sdiv/udiv (no trap: x/0 = 0, MIN/-1 = MIN), then csinv for RV's x/0 = all ones;
#   rem/remu: sdiv/udiv + msub (x%0 = x and MIN%-1 = 0 fall out); *W: the 32-bit op + sxtw;
#   sll/srl/sra: lslv/lsrv/asrv (the count masked by the width, as RV); mulh smulh, mulhu
#   umulh, mulhsu umulh - (b & a>>63); slt/sltu cmp + cset; imm logic ops through T1.
#   Guards: ldp (lo, lim) from the frame; (addr - lo) <u lim, else b.lo over a B to the cold stub.
#
# REGISTERS (fixed, not by frequency). RV xN lives in host xN, except: x15 -> x0, x16 -> x3,
# x17 -> x4, x18 -> x28 (host x18 is the platform register on macOS and is never used).
# RV x3 (gp), x4 (tp), x28 (t3) and x31 (t6) live in the frame's regs[]; the v1.0.0 Stage 1
# image uses them 0, 0, 4 and 0 times. Scratch: x16 (T0, addresses), x17 (T1), x15 (T2).
# The host sp is THE FRAME: it points at the data symbols for the whole run and is never
# pushed, so every symbol is one [sp, #off] access. x30 is a guest register: a cold stub
# saves it around its bl.
A64_MAP = {1: 1, 2: 2, 5: 5, 6: 6, 7: 7, 8: 8, 9: 9, 10: 10, 11: 11, 12: 12, 13: 13, 14: 14,
           15: 0, 16: 3, 17: 4, 18: 28, 19: 19, 20: 20, 21: 21, 22: 22, 23: 23, 24: 24, 25: 25,
           26: 26, 27: 27, 29: 29, 30: 30}
A64_MAP_MAC = {k: v for k, v in A64_MAP.items() if k != 17}
DELTA = 4                # Darwin: host x4 = hbase - base
TBASE_MAC = 0x1_0000_0000
T0, T1, T2 = 16, 17, 15
ZR = 31                  # xzr in data-processing operands, sp as a load/store base
SPR = 31
EQ, NE, HS, LO, MI, PL, HI, LS, GE, LT, GT, LE = 0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13
VS, VC = 6, 7
# the frame: data symbols at fixed offsets from sp (the guard pairs first: ldp reaches 504)
def _a64_frame(extra):
    fr, o = {}, 0
    for n, sz in [("g_l1", 16), ("g_l2", 16), ("g_l4", 16), ("g_l8", 16), ("g_s1", 16), ("g_s2", 16),
                  ("g_s4", 16), ("g_s8", 16), ("sp_win", 16), ("sv01", 16), ("sv28", 16), ("regs", 256),
                  ("fregs", 256), ("fault_off", 8), ("fault_word", 8), ("fault_addr", 8), ("code_end", 8),
                  ("outlen", 8), ("argc", 8), ("argv", 8), ("envp", 8), ("ram", 8), ("top", 8), ("in_fd", 8),
                  ("in_len", 8), ("name_ptr", 8), ("name_len", 8), ("args_len", 8), ("jt_addr", 8),
                  ("fin_val", 8), ("lr_cold", 8), ("lr_slow", 8), ("ev_slot", 8), ("rec_fd", 8),
                  ("wt_tmp", 8), ("win_path", 8), ("win_buf", 8),
                  ("win_st", 8), ("win_sv", 8 * 16), ("win_param", 8), ("pwv", 8 * PW_SLOTS),
                  ("pwlr", 8 * PW_LR), ("spn", SPN_SIZE), ("spn_sv", 8 * 11)] + extra + [
                  ("num", 32), ("statbuf", 144), ("outbuf", 65536), ("pwb", PWB_SIZE)]:
        fr[n] = o
        o += sz
    return fr, o


A64_FRAME, A64_FRAME_SIZE = _a64_frame([])
# ---- aarch64-macos (Darwin arm64): the same lowering with three differences, all forced by
# the platform (measured on macOS 27, M2 Pro): (1) arm64 processes must have a HARD 4 GiB
# __PAGEZERO (a smaller one is killed), so guest RAM cannot sit at its own address: it is
# mmap'ed anywhere (hbase) and every guest access goes through x4 = delta = hbase - base
# ([x4, addr]); RV x17 therefore lives in the frame. (2) MH_PIE is mandatory (non-PIE is
# killed), so the image slides: addresses are PC-relative (adrp+add) and the jump table
# holds offsets from itself. (3) Static executables are killed: dyld must load the image
# (LC_LOAD_DYLINKER, LC_MAIN, LC_UUID); libSystem is NOT needed when LC_BUILD_VERSION is
# absent. The code signature (ad hoc, SHA-256 over 4 KiB pages) is written here.
A64_FRAME_MAC, A64_FRAME_MAC_SIZE = _a64_frame([("hbase", 8), ("delta", 8)])
# ---- aarch64-windows: Darwin's addressing (PE images are DYNAMICBASE, ASLR is forced on
# ARM64, so the code is PC-relative, the table holds offsets, and guest RAM is wherever
# VirtualAlloc puts it, through x4 = delta), and Linux's register contract for the host
# calls: a "syscall" is a bl to a shim that saves everything but x0, switches to the
# thread's own stack (the frame is not one Windows knows), calls kernel32 through the
# import table and answers as Linux would (a count, or -1). x18 is the TEB and is never
# touched. Passed through translate_a64 as mac="win": every `if mac` holds for it.
WBASE = 0x1_4000_0000
A64_FRAME_WIN, A64_FRAME_WIN_SIZE = _a64_frame([("hbase", 8), ("delta", 8), ("os_sp", 8), ("lr_sys", 8),
                                                ("wsave", 8 * 18)])
WIN_IMPORTS = ("GetStdHandle", "WriteFile", "ReadFile", "CreateFileW", "GetFileSizeEx", "VirtualAlloc",
               "ExitProcess", "GetEnvironmentStringsW", "GetCommandLineW", "WideCharToMultiByte",
               "MultiByteToWideChar", "CloseHandle", "GetFinalPathNameByHandleW", "GetFileInformationByHandle",
               "GetFileAttributesW", "CreateDirectoryW", "DeleteFileW", "MoveFileExW", "FindFirstFileW",
               "FindNextFileW", "FindClose", "VirtualFree",
               # spawn, appended; CloseHandle and VirtualFree are above
               "CreatePipe", "SetHandleInformation", "CreateProcessW", "PeekNamedPipe",
               "WaitForSingleObject", "GetExitCodeProcess")


def a64_frame(mac):
    return (A64_FRAME_WIN if mac == "win" else A64_FRAME_MAC) if mac else A64_FRAME


def a64_frame_size(mac):
    return (A64_FRAME_WIN_SIZE if mac == "win" else A64_FRAME_MAC_SIZE) if mac else A64_FRAME_SIZE
# logical immediates the lowerings use, as N:immr:imms
LOG_NOT1 = 1 << 12 | 63 << 6 | 62        # ~1
LOG_3 = 1 << 12 | 0 << 6 | 1              # 3
LOG_NOT15 = 1 << 12 | 60 << 6 | 59        # ~15
LOG_NOT4095 = 1 << 12 | 52 << 6 | 51      # ~0xFFF
LOG_1 = 1 << 12                           # 1
LOG_NOT7 = 1 << 12 | 61 << 6 | 60         # ~7
# memory forms: (unsigned-offset opcode, size); unscaled = opcode - 0x01000000
A64_MEM = {"ldrb": (0x39400000, 1), "ldrsb": (0x39800000, 1), "ldrh": (0x79400000, 2),
           "ldrsh": (0x79800000, 2), "ldrw": (0xB9400000, 4), "ldrsw": (0xB9800000, 4),
           "ldr": (0xF9400000, 8), "strb": (0x39000000, 1), "strh": (0x79000000, 2),
           "strw": (0xB9000000, 4), "str": (0xF9000000, 8)}
A64_RRR = {"add": 0x0B000000, "sub": 0x4B000000, "subs": 0x6B000000, "and": 0x0A000000,
           "orr": 0x2A000000, "eor": 0x4A000000}
A64_DP2 = {"udiv": 2, "sdiv": 3, "lslv": 8, "lsrv": 9, "asrv": 10}


class A64:
    """Words, labels and fixups. Every lowering's length depends only on the guest word
    (its immediates), never on where it lands: a far branch is always B, and a conditional
    one is a B.cond over a B."""

    def __init__(self, origin):
        self.b = bytearray()
        self.origin = origin
        self.labels = {}
        self.fix = []          # (offset, label, kind)  kind: 0 b26, 1 b19, 2 movz/movk x4, 3 adrp+add

    def here(self):
        return self.origin + len(self.b)

    def label(self, name):
        assert name not in self.labels, name
        self.labels[name] = self.here()

    def w(self, v):
        self.b += struct.pack("<I", v & 0xFFFFFFFF)

    def raw(self, bs):
        self.b += bs

    def resolve(self, extra=None):
        labels = dict(self.labels)
        if extra:
            labels.update(extra)
        for off, lab, kind in self.fix:
            t = labels[lab]
            if kind == 3:
                rd = struct.unpack_from("<I", self.b, off)[0] & 31
                pg = ((t >> 12) - ((self.origin + off) >> 12)) & 0x1FFFFF
                struct.pack_into("<I", self.b, off, 0x90000000 | (pg & 3) << 29 | (pg >> 2) << 5 | rd)
                struct.pack_into("<I", self.b, off + 4, 0x91000000 | (t & 0xFFF) << 10 | rd << 5 | rd)
                continue
            if kind == 2:
                rd = struct.unpack_from("<I", self.b, off)[0] & 31
                for i in range(4):
                    struct.pack_into("<I", self.b, off + 4 * i,
                                     (0xD2800000 if i == 0 else 0xF2800000) | i << 21 | ((t >> 16 * i) & 0xFFFF) << 5 | rd)
                continue
            d = (t - (self.origin + off)) >> 2
            op = struct.unpack_from("<I", self.b, off)[0]
            if kind == 0:
                assert -(1 << 25) <= d < 1 << 25, lab
                op |= d & 0x3FFFFFF
            else:
                assert -(1 << 18) <= d < 1 << 18, lab
                op |= (d & 0x7FFFF) << 5
            struct.pack_into("<I", self.b, off, op)

    # ---- branches
    def b_(self, label):
        self.fix.append((len(self.b), label, 0))
        self.w(0x14000000)

    def bl(self, label):
        self.fix.append((len(self.b), label, 0))
        self.w(0x94000000)

    def bcond(self, cc, label):
        self.fix.append((len(self.b), label, 1))
        self.w(0x54000000 | cc)

    def cbz(self, r, label, sf=1):
        self.fix.append((len(self.b), label, 1))
        self.w((0xB4000000 if sf else 0x34000000) | r)

    def cbnz(self, r, label, sf=1):
        self.fix.append((len(self.b), label, 1))
        self.w((0xB5000000 if sf else 0x35000000) | r)

    def bskip(self, cc, n):
        """b.cc over the next n-1 words (to n words ahead)"""
        self.w(0x54000000 | (n & 0x7FFFF) << 5 | cc)

    def br(self, r):
        self.w(0xD61F0000 | r << 5)

    def ret(self):
        self.w(0xD65F03C0)

    def svc(self):
        self.w(0xD4000001)

    def svc80(self):
        self.w(0xD4001001)

    def blr(self, r):
        self.w(0xD63F0000 | r << 5)

    def align(self, n):
        """zero octets up to a multiple of n (a recorder writes this as a run-time loop)"""
        self.raw(b"\0" * (-len(self.b) % n))

    # ---- constants
    def movz(self, rd, imm, hw=0):
        self.w(0xD2800000 | hw << 21 | (imm & 0xFFFF) << 5 | rd)

    def movn(self, rd, imm, hw=0):
        self.w(0x92800000 | hw << 21 | (imm & 0xFFFF) << 5 | rd)

    def movk(self, rd, imm, hw):
        self.w(0xF2800000 | hw << 21 | (imm & 0xFFFF) << 5 | rd)

    def movi(self, rd, v):
        """any 64-bit value: movz (or movn when more chunks are all ones) + movk per chunk"""
        v &= 0xFFFFFFFFFFFFFFFF
        c = [(v >> 16 * i) & 0xFFFF for i in range(4)]
        inv = c.count(0xFFFF) > c.count(0)
        skip = 0xFFFF if inv else 0
        first = 0
        while first < 4 and c[first] == skip:
            first += 1
        if first == 4:
            first = 0
        if inv:
            self.movn(rd, ~c[first], first)
        else:
            self.movz(rd, c[first], first)
        for i in range(first + 1, 4):
            if c[i] != skip:
                self.movk(rd, c[i], i)

    def adrl(self, rd, label):
        """a label's address, PC-relative: adrp + add (a slid PIE image)"""
        self.fix.append((len(self.b), label, 3))
        self.w(rd)
        self.w(0)

    def movl(self, rd, label):
        """a label's absolute address: always movz + 3 movk"""
        self.fix.append((len(self.b), label, 2))
        self.w(rd)
        self.w(0)
        self.w(0)
        self.w(0)

    # ---- data processing
    def add_i(self, rd, rn, imm, sf=1):
        """rd = rn + imm, |imm| < 4096 (31 is sp here)"""
        assert -4096 < imm < 4096, imm
        if imm >= 0:
            self.w(0x11000000 | sf << 31 | imm << 10 | rn << 5 | rd)
        else:
            self.w(0x51000000 | sf << 31 | (-imm) << 10 | rn << 5 | rd)

    def cmp_i(self, rn, imm, sf=1):
        assert -4096 < imm < 4096, imm
        if imm >= 0:
            self.w(0x71000000 | sf << 31 | imm << 10 | rn << 5 | 31)
        else:
            self.w(0x31000000 | sf << 31 | (-imm) << 10 | rn << 5 | 31)

    def cmp_i12(self, rn, imm):
        """cmp rn, #imm << 12"""
        self.w(0xF1400000 | imm << 10 | rn << 5 | 31)

    def rrr(self, op, rd, rn, rm, sf=1, shift=0, amt=0):
        """add sub subs and orr eor, shifted register (31 is xzr here)"""
        self.w(A64_RRR[op] | sf << 31 | shift << 22 | rm << 16 | amt << 10 | rn << 5 | rd)

    def mov(self, rd, rm):
        self.rrr("orr", rd, ZR, rm)

    def cmp(self, rn, rm, sf=1):
        self.rrr("subs", ZR, rn, rm, sf)

    def dp2(self, op, rd, rn, rm, sf=1):
        self.w(0x1AC00000 | sf << 31 | rm << 16 | A64_DP2[op] << 10 | rn << 5 | rd)

    def madd(self, rd, rn, rm, ra, sf=1):
        self.w(0x1B000000 | sf << 31 | rm << 16 | ra << 10 | rn << 5 | rd)

    def msub(self, rd, rn, rm, ra, sf=1):
        self.w(0x1B008000 | sf << 31 | rm << 16 | ra << 10 | rn << 5 | rd)

    def smulh(self, rd, rn, rm):
        self.w(0x9B407C00 | rm << 16 | rn << 5 | rd)

    def umulh(self, rd, rn, rm):
        self.w(0x9BC07C00 | rm << 16 | rn << 5 | rd)

    def ubfm(self, rd, rn, immr, imms, sf=1):
        self.w(0x53000000 | sf << 31 | sf << 22 | immr << 16 | imms << 10 | rn << 5 | rd)

    def sbfm(self, rd, rn, immr, imms, sf=1):
        self.w(0x13000000 | sf << 31 | sf << 22 | immr << 16 | imms << 10 | rn << 5 | rd)

    def lsl_i(self, rd, rn, s, sf=1):
        bits = 64 if sf else 32
        self.ubfm(rd, rn, (bits - s) % bits, bits - 1 - s, sf)

    def lsr_i(self, rd, rn, s, sf=1):
        self.ubfm(rd, rn, s, 63 if sf else 31, sf)

    def asr_i(self, rd, rn, s, sf=1):
        self.sbfm(rd, rn, s, 63 if sf else 31, sf)

    def sxtw(self, rd, rn):
        self.sbfm(rd, rn, 0, 31)

    def csinv(self, rd, rn, rm, cc):
        self.w(0xDA800000 | rm << 16 | cc << 12 | rn << 5 | rd)

    def cset(self, rd, cc):
        self.w(0x9A800400 | 31 << 16 | (cc ^ 1) << 12 | 31 << 5 | rd)

    def and_l(self, rd, rn, enc):
        self.w(0x92000000 | enc << 10 | rn << 5 | rd)

    def tst_l(self, rn, enc):
        self.w(0xF2000000 | enc << 10 | rn << 5 | 31)

    def sp_from(self, rn):
        self.w(0x91000000 | rn << 5 | 31)        # mov sp, rn

    def from_sp(self, rd):
        self.w(0x91000000 | 31 << 5 | rd)        # mov rd, sp

    # ---- memory
    def mem(self, kind, rt, rn, off):
        """[rn, #off]: the scaled form, else the unscaled one, else through T2"""
        op, size = A64_MEM[kind]
        if off >= 0 and off % size == 0 and off // size < 4096:
            self.w(op | (off // size) << 10 | rn << 5 | rt)
        elif -256 <= off < 256:
            self.w(op - 0x01000000 | (off & 0x1FF) << 12 | rn << 5 | rt)
        else:
            self.add_i(T2, rn, off)
            self.w(op | T2 << 5 | rt)

    def memr(self, kind, rt, rn, rm, scaled=0):
        """[rn, rm] or [rn, rm, lsl #size]"""
        op, _size = A64_MEM[kind]
        self.w(op - 0x01000000 | 0x00206800 | scaled << 12 | rm << 16 | rn << 5 | rt)

    # ---- scalar floating point (sd: 0 single, 1 double); v registers are written by number
    def add_i12(self, rd, rn, imm):
        """rd = rn + (imm << 12), imm < 4096 (31 is sp)"""
        assert 0 <= imm < 4096
        self.w(0x91400000 | imm << 10 | rn << 5 | rd)

    def cmn_i(self, rn, imm, sf=1):
        self.w(0x31000000 | sf << 31 | imm << 10 | rn << 5 | 31)

    def csel(self, rd, rn, rm, cc, sf=1):
        self.w(0x1A800000 | sf << 31 | rm << 16 | cc << 12 | rn << 5 | rd)

    def clz(self, rd, rn):
        self.w(0xDAC01000 | rn << 5 | rd)

    def rbit(self, rd, rn):
        self.w(0xDAC00000 | rn << 5 | rd)

    def fldr(self, sd, vt, rn, off):
        sz = 8 if sd else 4
        assert off % sz == 0 and 0 <= off // sz < 4096, off
        self.w((0xFD400000 if sd else 0xBD400000) | (off // sz) << 10 | rn << 5 | vt)

    def fstr(self, sd, vt, rn, off):
        sz = 8 if sd else 4
        assert off % sz == 0 and 0 <= off // sz < 4096, off
        self.w((0xFD000000 if sd else 0xBD000000) | (off // sz) << 10 | rn << 5 | vt)

    def fp2(self, op, sd, rd, rn, rm):
        """fmul fdiv fadd fsub"""
        self.w(0x1E200800 | sd << 22 | rm << 16 | {"fmul": 0, "fdiv": 1, "fadd": 2, "fsub": 3}[op] << 12 | rn << 5 | rd)

    def fsqrt(self, sd, rd, rn):
        self.w(0x1E21C000 | sd << 22 | rn << 5 | rd)

    def fmov_v(self, sd, rd, rn):
        self.w(0x1E204000 | sd << 22 | rn << 5 | rd)

    def fp3(self, op, sd, rd, rn, rm, ra):
        """fmadd fmsub fnmadd fnmsub: fmadd = ra + rn*rm, fmsub = ra - rn*rm, fnmadd = -ra - rn*rm, fnmsub = -ra + rn*rm"""
        self.w(0x1F000000 | sd << 22 | {"fmadd": 0, "fmsub": 0x8000, "fnmadd": 0x200000, "fnmsub": 0x208000}[op]
               | rm << 16 | ra << 10 | rn << 5 | rd)

    def fcvt_ff(self, to_sd, rd, rn):
        """fcvt: double -> single (to_sd 0) or single -> double (1)"""
        self.w((0x1E624000 if not to_sd else 0x1E22C000) | rn << 5 | rd)

    def fcvt_int(self, sf, sd, rmode, opcode, rd, rn):
        """the float <-> integer group: fcvt{n,p,m,z,a}{s,u} (rd x/w <- rn s/d), scvtf / ucvtf (rd s/d <- rn x/w),
        fmov (opcode 6: x <- d, 7: d <- x; sf 0: w <-> s)"""
        self.w(0x1E200000 | sf << 31 | sd << 22 | rmode << 19 | opcode << 16 | rn << 5 | rd)

    def fcmp(self, sd, rn, rm, e=0):
        self.w(0x1E202000 | sd << 22 | rm << 16 | rn << 5 | (0x10 if e else 0))

    def msr_fpcr(self, rt):
        self.w(0xD51B4400 | rt)

    def mrs_fpcr(self, rt):
        self.w(0xD53B4400 | rt)

    def msr_fpsr(self, rt):
        self.w(0xD51B4420 | rt)

    def mrs_fpsr(self, rt):
        self.w(0xD53B4420 | rt)

    def ldp(self, rt, rt2, rn, off):
        assert off % 8 == 0 and -512 <= off < 512, off
        self.w(0xA9400000 | ((off >> 3) & 0x7F) << 15 | rt2 << 10 | rn << 5 | rt)

    def stp(self, rt, rt2, rn, off):
        assert off % 8 == 0 and -512 <= off < 512, off
        self.w(0xA9000000 | ((off >> 3) & 0x7F) << 15 | rt2 << 10 | rn << 5 | rt)


def translate_a64(img, arg0, mac=False):
    entry, base, span, extent, code_size, prot_size = parse_elf(img)
    assert base == 0x8000_0000, "milestone 1: images load at 0x8000_0000"
    nwords = code_size // 4
    words = struct.unpack_from("<%dI" % nwords, span)
    decs = [decode(w) for w in words]
    M = A64_MAP_MAC if mac else A64_MAP
    FR = a64_frame(mac)
    A = A64((WBASE if mac == "win" else TBASE_MAC if mac else TEXT) + 0x1000)

    def gmem(kind, r, rn, off):
        """a GUEST access [rn + off]: on Darwin through delta, [x4, rn + off]"""
        if not mac:
            A.mem(kind, r, rn, off)
        elif off == 0:
            A.memr(kind, r, DELTA, rn)
        else:
            A.add_i(T2, rn, off)
            A.memr(kind, r, DELTA, T2)

    BR = ("beq", "bne", "blt", "bge", "bltu", "bgeu")
    MEM_L = ("lb", "lh", "lw", "ld", "lbu", "lhu", "lwu", "fld", "flw")
    MEM_S = ("sb", "sh", "sw", "sd", "fsd", "fsw")
    fp_used = any(d and (d[0] in FP_ID or d[0] in CSR_OPS or d[0] in FP_MEM or d[0] in FP_MOVES) for d in decs)
    leader = [False] * nwords
    leader[(entry - base) // 4] = True
    for k, d in enumerate(decs):
        if d is None or d[0] in BR + ("jal", "jalr") or (d[1] == 2 and d[0] not in BR + MEM_S):
            if k + 1 < nwords:
                leader[k + 1] = True
        if d and d[0] in BR + ("jal",):
            t = k + d[4] // 4
            if 0 <= t < nwords and d[4] % 4 == 0:
                leader[t] = True

    def tgt(t, from_fast=False):
        if not (base <= t < base + 4 * nwords) or t % 4:
            return "rt_bad_target"
        k = (t - base) // 4
        if leader[k]:
            return ("FB_%x" if from_fast else "F_%x") % t
        return "G_%x" % t

    def jmp_t(t, from_fast):
        lab = tgt(t, from_fast)
        if lab == "rt_bad_target":
            A.movi(T0, t)                 # rt_bad_target names the untranslated target
        A.b_(lab)

    def jcc_t(cc, t, from_fast):
        """b.!cc over the far jump; the skip is patched once its length is known"""
        lab = tgt(t, from_fast)
        at = len(A.b)
        A.w(0)
        if lab == "rt_bad_target":
            A.movi(T0, t)
        A.b_(lab)
        struct.pack_into("<I", A.b, at, 0x54000000 | ((len(A.b) - at) >> 2) << 5 | (cc ^ 1))

    def src(n, s):
        """a host register holding RV n, where 31 reads as zero"""
        if n == 0:
            return ZR
        if n in M:
            return M[n]
        A.mem("ldr", s, SPR, FR["regs"] + 8 * n)
        return s

    def srcn(n, s):
        """the same, for an operand where 31 would be sp: x0 is materialised"""
        if n == 0:
            A.movz(s, 0)
            return s
        return src(n, s)

    def dst(n):
        return M[n] if n in M else T0

    def fin(n, r):
        if n not in M:
            A.mem("str", r, SPR, FR["regs"] + 8 * n)

    cold = []
    slots = {}

    def forget_reg(r):
        for o in [o for o, s in slots.items() if s == r]:
            del slots[o]

    def forget_range(off, width):
        for o in [o for o in slots if o < off + width and off < o + 8]:
            del slots[o]

    def lower(k, fast):
        V = "F" if fast else "G"
        pc = base + 4 * k

        def guard(width, kind, val):
            # (addr - lo) <u lim, else the cold stub; lo/lim are one ldp from the frame
            A.ldp(T1, T2, SPR, FR["g_%s%d" % (kind, width)])
            A.rrr("sub", T1, T0, T1)
            A.cmp(T1, T2)
            A.bskip(LO, 2)
            A.b_("C%s_%x" % (V, pc))
            cold.append((V, pc, kind, val))

        def addr(rs1, imm):
            if rs1 == 0:
                A.movi(T0, imm)
            else:
                A.add_i(T0, src(rs1, T0), imm)

        d = decs[k]
        if d is None:
            A.movi(T2, 4 * k)
            A.mem("str", T2, SPR, FR["fault_off"])
            A.movi(T2, sx(words[k], 32))
            A.mem("str", T2, SPR, FR["fault_word"])
            A.b_("rt_bad_insn")
            return
        op, rd, rs1, rs2, imm = d
        if op == "fence":
            return
        FPO, FSEN = a64_frame_size(mac), fp_a64_off("fsen")

        def fs_addr(r):
            A.add_i12(r, SPR, FPO >> 12)
            A.add_i(r, r, FPO & 0xFFF)
        if op == "csrs_sstatus":
            if fp_used:                       # sstatus.FS gates every float instruction (yantra: illegal instruction)
                f3, r = imm & 7, imm >> 3
                if f3 in (1, 2, 3) and (f3 == 1 or r):
                    r0 = srcn(r, T0)
                    A.lsr_i(T0, r0, 13)
                    A.and_l(T0, T0, LOG_3)
                    fs_addr(T2)
                    if f3 == 1:
                        A.mem("str", T0, T2, FSEN)
                    else:
                        A.mem("ldr", T1, T2, FSEN)
                        if f3 == 2:
                            A.rrr("orr", T1, T1, T0)
                        else:
                            A.w(0x8A200000 | T0 << 16 | T1 << 5 | T1)      # bic x17, x17, x16
                        A.mem("str", T1, T2, FSEN)
                elif f3 == 5:
                    fs_addr(T2)
                    A.mem("str", ZR, T2, FSEN)
            return
        if fp_used and (op in FP_ID or op in CSR_OPS or op in FP_MEM or op in FP_MOVES):
            fs_addr(T2)                       # FS off: the halt yantra makes
            A.mem("ldr", T2, T2, FSEN)
            at = len(A.b)
            A.w(0)
            A.movi(T2, 4 * k)
            A.mem("str", T2, SPR, FR["fault_off"])
            A.movi(T2, sx(words[k], 32))
            A.mem("str", T2, SPR, FR["fault_word"])
            A.b_("rt_illegal")
            struct.pack_into("<I", A.b, at, 0xB5000000 | ((len(A.b) - at) >> 2) << 5 | T2)     # cbnz x15, here
        if fast and rd and op in ("lui", "auipc", "jal", "jalr"):
            forget_reg(rd)
        if op in ("lui", "auipc"):
            if rd:
                r = dst(rd)
                A.movi(r, imm if op == "lui" else pc + imm)
                fin(rd, r)
            return
        if op == "jal":
            if rd:
                r = dst(rd)
                A.movi(r, pc + 4)
                fin(rd, r)
            # jal rd=x2 writes sp, so a FAST block must re-check the window at the target (F_, not FB_)
            jmp_t(pc + imm, fast and rd != 2)
            return
        if op == "jalr":
            A.add_i(T0, srcn(rs1, T0), imm)
            A.and_l(T0, T0, LOG_NOT1)
            if rd:
                r = M[rd] if rd in M else T1    # rd is written AFTER the target is read
                A.movi(r, pc + 4)
                fin(rd, r)
            A.b_("rt_indirect")
            return
        if op in BR:
            A.cmp(src(rs1, T0), src(rs2, T1))
            jcc_t({"beq": EQ, "bne": NE, "blt": LT, "bge": GE, "bltu": LO, "bgeu": HS}[op], pc + imm, fast)
            return
        if op in MEM_L:
            width = {"lb": 1, "lbu": 1, "lh": 2, "lhu": 2, "lw": 4, "lwu": 4, "ld": 8, "fld": 8, "flw": 4}[op]
            if fast and rs1 == 2 and op == "ld" and rd and rd in M and imm in slots:
                s = slots[imm]                     # STORE/LOAD FORWARDING (as on x86-64)
                if s != rd:
                    A.mov(M[rd], M[s])
                forget_reg(rd)
                slots[imm] = s if s != rd else rd
                return
            if fast:
                forget_reg(rd)
            kind = {"lb": "ldrsb", "lh": "ldrsh", "lw": "ldrsw", "ld": "ldr", "lbu": "ldrb",
                    "lhu": "ldrh", "lwu": "ldrw", "fld": "ldr", "flw": "ldrw"}[op]
            if op in ("fld", "flw"):
                r = T1
            elif rd == 0:
                r = ZR                             # the access still happens
            else:
                r = M[rd] if rd in M else T1
            if fast and rs1 == 2:
                gmem(kind, r, M[2], imm)
            else:
                addr(rs1, imm)
                guard(width, "l", 0)
                gmem(kind, r, T0, 0)
            if op in ("fld", "flw"):
                if op == "flw":
                    A.movi(T2, 0xFFFFFFFF00000000)         # a single is BOXED: the upper 32 bits all ones
                    A.rrr("orr", T1, T1, T2)
                A.mem("str", T1, SPR, FR["fregs"] + 8 * rd)
            elif rd:
                fin(rd, r)
            if fast and rs1 == 2 and op == "ld" and rd and rd in M:
                slots[imm] = rd
            return
        if op in MEM_S:
            width = {"sb": 1, "sh": 2, "sw": 4, "sd": 8, "fsd": 8, "fsw": 4}[op]
            kind = {1: "strb", 2: "strh", 4: "strw", 8: "str"}[width]
            val = 32 + rs2 if op in ("fsd", "fsw") else rs2     # 32 + n: f-register n

            def value():
                if op in ("fsd", "fsw"):
                    A.mem("ldr", T1, SPR, FR["fregs"] + 8 * rs2)
                    return T1
                return src(rs2, T1)
            if fast and rs1 == 2:
                gmem(kind, value(), M[2], imm)
                forget_range(imm, width)
                if op == "sd" and rs2 and rs2 in M:
                    slots[imm] = rs2
                return
            if fast:
                slots.clear()
            addr(rs1, imm)
            guard(width, "s", val)
            gmem(kind, value(), T0, 0)
            A.label("B%s_%x" % (V, pc))
            return
        if op in FP_ID or op in CSR_OPS:
            # the A64 form of the x86-64 lowering: the integer source in x16, the f sources in d0..d2, the rm field in
            # x17, `bl fph_<id>` (x30 is a guest register: saved around the call), the answer in d0 or x16
            if op in CSR_OPS:
                kind = (CSR_OPS.index(op) % 3) + 1
                if CSR_OPS.index(op) < 3:
                    r0 = srcn(rs1, T0)
                    if r0 != T0:
                        A.mov(T0, r0)
                else:
                    A.movz(T0, rs1)
                A.movz(T1, imm | kind << 4)
                A.mem("str", 30, SPR, FR["lr_cold"])
                A.bl("fph_csr")
                A.mem("ldr", 30, SPR, FR["lr_cold"])
                if rd:
                    if fast:
                        forget_reg(rd)
                    r = dst(rd)
                    A.mov(r, T0)
                    fin(rd, r)
                return
            nf, xs, fd, xd, rmf = FP_SHAPE[op]
            if xs:
                r0 = srcn(rs1, T0)
                if r0 != T0:
                    A.mov(T0, r0)
            for i, fr in enumerate((rs1, rs2, imm >> 3)[:nf]):
                A.fldr(1, i, SPR, FR["fregs"] + 8 * fr)
            if rmf:
                if imm & 7 == 7:
                    A.movi(T2, 4 * k)
                    A.mem("str", T2, SPR, FR["fault_off"])
                    A.movi(T2, sx(words[k], 32))
                    A.mem("str", T2, SPR, FR["fault_word"])
                A.movz(T1, imm & 7)
            A.mem("str", 30, SPR, FR["lr_cold"])
            A.bl("fph_%d" % FP_ID[op])
            A.mem("ldr", 30, SPR, FR["lr_cold"])
            if fd:
                A.fstr(1, 0, SPR, FR["fregs"] + 8 * rd)
            if xd and rd:
                if fast:
                    forget_reg(rd)
                r = dst(rd)
                A.mov(r, T0)
                fin(rd, r)
            return
        if rd == 0:
            return
        if fast:
            forget_reg(rd)
        r = dst(rd)
        if op == "addi" and rs1 == 0:
            A.movi(r, imm)
        elif op == "addi":
            A.add_i(r, src(rs1, T0), imm)
        elif op in ("slti", "sltiu"):
            A.cmp_i(srcn(rs1, T0), imm)
            A.cset(r, LT if op == "slti" else LO)
        elif op in ("xori", "ori", "andi"):
            a = src(rs1, T0)
            A.movi(T1, imm)
            A.rrr({"xori": "eor", "ori": "orr", "andi": "and"}[op], r, a, T1)
        elif op in ("slli", "srli", "srai", "slliw", "srliw", "sraiw"):
            sf = 0 if op.endswith("w") else 1
            {"sll": A.lsl_i, "srl": A.lsr_i, "sra": A.asr_i}[op[:3]](r, src(rs1, T0), imm, sf)
            if not sf:
                A.sxtw(r, r)
        elif op == "addiw":
            A.add_i(r, srcn(rs1, T0), imm, 0)
            A.sxtw(r, r)
        else:
            a = src(rs1, T0)
            b = src(rs2, T1)
            sf = 0 if op.endswith("w") else 1
            if op in ("add", "sub", "and", "or", "xor", "addw", "subw"):
                A.rrr({"add": "add", "addw": "add", "sub": "sub", "subw": "sub", "and": "and", "or": "orr", "xor": "eor"}[op], r, a, b, sf)
            elif op in ("slt", "sltu"):
                A.cmp(a, b)
                A.cset(r, LT if op == "slt" else LO)
            elif op in ("mul", "mulw"):
                A.madd(r, a, b, ZR, sf)
            elif op in ("sll", "srl", "sra", "sllw", "srlw", "sraw"):
                A.dp2({"sll": "lslv", "srl": "lsrv", "sra": "asrv"}[op[:3]], r, a, b, sf)
            elif op == "mulh":
                A.smulh(r, a, b)
            elif op == "mulhu":
                A.umulh(r, a, b)
            elif op == "mulhsu":
                # (i128)a * (u128)b >> 64 = mulhu(a, b) - (a < 0 ? b : 0)
                A.umulh(T2, a, b)
                A.rrr("and", T1, b, a, 1, 2, 63)
                A.rrr("sub", r, T2, T1)
            elif op in ("div", "divu", "divw", "divuw"):
                # RV: x/0 = all ones; A64 gives 0, and MIN/-1 = MIN as RV wants (no trap)
                A.dp2("sdiv" if op in ("div", "divw") else "udiv", T2, a, b, sf)
                if not sf:
                    A.sxtw(T2, T2)
                A.cmp(b, ZR, sf)
                A.csinv(r, T2, ZR, NE)
            elif op in ("rem", "remu", "remw", "remuw"):
                # a - (a/b)*b: x%0 = x (the quotient is 0) and MIN%-1 = 0, as RV wants
                A.dp2("sdiv" if op in ("rem", "remw") else "udiv", T2, a, b, sf)
                if sf:
                    A.msub(r, T2, b, a)
                else:
                    A.msub(T2, T2, b, a, 0)
                    A.sxtw(r, T2)
            else:
                raise SystemExit("untranslated %s at %#x" % (op, pc))
            if not sf and op in ("addw", "subw", "mulw", "sllw", "srlw", "sraw"):
                A.sxtw(r, r)
        fin(rd, r)

    # ---------------- the FAST region: blocks in address order, each opening with the sp check
    for k in range(nwords):
        pc = base + 4 * k
        if leader[k]:
            slots.clear()
            A.label("F_%x" % pc)
            # (sp - sp_lo) <u sp_span, else this block runs SAFE
            A.ldp(T1, T2, SPR, FR["sp_win"])
            A.rrr("sub", T1, M[2], T1)
            A.cmp(T1, T2)
            A.bskip(LO, 2)
            A.b_("G_%x" % pc)
            A.label("FB_%x" % pc)
        lower(k, True)
    A.movi(T0, base + 4 * nwords)
    A.b_("rt_bad_target")
    # ---------------- the SAFE region: every word, every access guarded
    for k in range(nwords):
        A.label("G_%x" % (base + 4 * k))
        lower(k, False)
    A.movi(T0, base + 4 * nwords)
    A.b_("rt_bad_target")
    # ---------------- cold paths: a guard that failed names its site, then the runtime decides
    for V, pc, kind, val in cold:
        A.label("C%s_%x" % (V, pc))
        A.movi(T2, pc - base)
        A.mem("str", T2, SPR, FR["fault_off"])
        if kind == "s":
            if val >= 32:
                A.mem("ldr", T1, SPR, FR["fregs"] + 8 * (val - 32))
            else:
                s = src(val, T1)
                if s != T1:
                    A.mov(T1, s)
            A.mem("str", 30, SPR, FR["lr_cold"])
            A.bl("rt_store_slow")              # returns only for the UART
            A.mem("ldr", 30, SPR, FR["lr_cold"])
            A.b_("B%s_%x" % (V, pc))
        else:
            A.b_("rt_load_slow")
    runtime_a64(A, base, entry, max(DEFAULT_RAM, extent + RAM_HEADROOM), len(span), code_size, prot_size, mac)
    if fp_used:
        A.raw(b"\0" * (-len(A.b) % 4))              # the runtime's strings end unaligned; the F/D code is words
        runtime_fp_a64(A, a64_frame_size(mac), os.environ.get("ANUVADA_SOFT_ALL") == "1", mac)
    # ---------------- the jump table and the guest image
    A.raw(b"\0" * (-len(A.b) % 8))
    A.label("jt")
    jt_at = len(A.b)
    A.raw(b"\0" * (8 * nwords))
    A.label("image")
    A.raw(span)
    A.label("arg0")
    A.raw(arg0.encode() + b"\0")
    seg_tab(A, img)
    A.label("text_end")
    if mac == "win":
        # .data: the import tables, then the frame (zero fill); the table holds offsets
        data = (A.here() + 0xFFF) & ~0xFFF
        imp, labs = pe_imports(data - WBASE)
        frame = (data + len(imp) + 0xFFF) & ~0xFFF
        labs = {k: data + v for k, v in labs.items()}
        labs["frame"] = frame
        A.resolve(labs)
        for k in range(nwords):
            struct.pack_into("<q", A.b, jt_at + 8 * k, A.labels[tgt(base + 4 * k)] - A.labels["jt"])
        return pe_a64(A, imp, frame - data + A64_FRAME_WIN_SIZE + (FP_A64_SIZE if fp_used else 0),
                      labs["pe_iat"] - WBASE, labs["pe_iat_end"] - labs["pe_iat"]), M
    if mac:
        # the frame (bss) is __DATA, right after __TEXT (16 KiB pages); the table holds offsets
        frame = (A.here() + 0x3FFF) & ~0x3FFF
        A.resolve({"frame": frame})
        for k in range(nwords):
            struct.pack_into("<q", A.b, jt_at + 8 * k, A.labels[tgt(base + 4 * k)] - A.labels["jt"])
        return macho_a64(A, A64_FRAME_MAC_SIZE + (FP_A64_SIZE if fp_used else 0)), M
    # ---------------- the frame (bss), 64 KiB aligned after the text (any page size)
    frame = (A.here() + 0xFFFF) & ~0xFFFF
    A.resolve({"frame": frame})
    for k in range(nwords):
        struct.pack_into("<Q", A.b, jt_at + 8 * k, A.labels[tgt(base + 4 * k)])
    return elf_a64(A, frame + A64_FRAME_SIZE + (FP_A64_SIZE if fp_used else 0)), M


def runtime_a64(A, base, entry, ram_default, filesz, code_size, prot_size, mac=False):
    """The host layer, in A64. Linux arm64 syscalls (svc #0, x8 = the number): 63 read,
    64 write, 56 openat (AT_FDCWD = -100), 80 fstat, 222 mmap, 93 exit. Before the guest
    starts it may clobber every register; after, only T0..T2, unless it saves the rest
    (rt_flush saves x0 x1 x2 x8, the registers svc reads or writes).
    mac: Darwin arm64 instead (svc #0x80, x16 = the number: 3 read, 4 write, 5 open,
    339 fstat64, 197 mmap, 1 exit; an error sets the carry, x0 = errno; svc also writes
    x1). Entered from dyld through LC_MAIN (x0 argc, x1 argv, x2 envp); the image is PIE,
    so every address is PC-relative (adrp+add) and the jump table holds offsets; guest RAM
    is wherever mmap puts it (hbase), and x4 holds delta = hbase - base for the guest."""
    FR = a64_frame(mac)
    MAP = A64_MAP_MAC if mac else A64_MAP

    def adr(rd, lab):
        (A.adrl if mac else A.movl)(rd, lab)

    darwin = mac and mac != "win"

    def wcall(shim):
        """Windows: a host call is a bl to a shim (x30 is the caller's, so it is kept)"""
        A.mem("str", 30, SPR, FR["lr_sys"])
        A.bl(shim)
        A.mem("ldr", 30, SPR, FR["lr_sys"])

    def sysc(n, nm):
        if mac == "win":
            wcall("w_sys%d" % n)
        elif mac:
            A.movz(16, nm)
            A.svc80()
        else:
            A.movz(8, n)
            A.svc()

    def guest_word(r, tag):
        """[tag + 8] = r as a GUEST address (r - delta on Darwin)"""
        if mac:
            A.mem("ldr", 9, SPR, FR["delta"])
            A.rrr("sub", 9, r, 9)
            A.mem("str", 9, tag, 8)
        else:
            A.mem("str", r, tag, 8)

    def hbase(rd):
        """the host address of guest RAM (base on Linux, where guest = host)"""
        if mac:
            A.mem("ldr", rd, SPR, FR["hbase"])
        else:
            A.movi(rd, base)

    def msg(label, text):
        A.label(label)
        adr(1, label + "_s")
        A.movi(2, len(text))
        A.movz(0, 2)
        sysc(64, 4)
        A.ret()
        strings.append((label + "_s", text))

    strings = []
    # ---- _start: argc, argv, envp from the initial stack; then sp becomes the frame
    A.label("rt_start")
    if mac == "win":
        # the thread's stack stays the host calls' stack; w_init builds argc, argv, envp
        A.from_sp(9)
        adr(13, "frame")
        A.sp_from(13)
        A.mem("str", 9, SPR, FR["os_sp"])
        wcall("w_init")
        A.cbz(0, "rt_no_ram")
    elif mac:
        A.mov(10, 0)                           # LC_MAIN: argc, argv, envp in x0 x1 x2
        A.mov(11, 1)
        A.mov(12, 2)
    else:
        A.from_sp(9)
        A.mem("ldr", 10, 9, 0)
        A.add_i(11, 9, 8)
        A.rrr("add", 12, 11, 10, 1, 0, 3)      # argv + 8*argc
        A.add_i(12, 12, 8)                     # envp = argv + 8*(argc+1)
    if mac != "win":
        adr(13, "frame")
        A.sp_from(13)
        A.mem("str", 10, SPR, FR["argc"])
        A.mem("str", 11, SPR, FR["argv"])
        A.mem("str", 12, SPR, FR["envp"])
    # ram = YANTRA_RAM, or ram_default = max(DEFAULT_RAM, extent + HEADROOM)
    A.movi(9, ram_default)
    A.mem("str", 9, SPR, FR["ram"])
    adr(0, "env_ram")
    A.bl("rt_getenv")
    A.cbz(0, "rt_s1")
    A.bl("rt_atou_ram")
    A.cbnz(1, "rt_s1")                         # not a usize: the default, as yantra-run
    A.mem("str", 0, SPR, FR["ram"])
    A.label("rt_s1")
    # yantra's loader: the first segment of seg_tab (header order) that RAM cannot hold refuses the run
    adr(19, "seg_tab")
    A.mem("ldr", 20, 19, 0)
    A.add_i(19, 19, 8)
    A.label("rt_sc1")
    A.cbz(20, "rt_sc2")
    A.mem("ldr", 9, 19, 0)
    A.movi(10, base)
    A.rrr("sub", 9, 9, 10)
    A.mem("ldr", 10, 19, 8)
    A.rrr("add", 9, 9, 10)
    A.mem("ldr", 10, SPR, FR["ram"])
    A.cmp(10, 9)
    A.bcond(LO, "rt_seg_fail")                 # ram < vaddr - base + need
    A.add_i(19, 19, 16)
    A.add_i(20, 20, -1)
    A.b_("rt_sc1")
    A.label("rt_sc2")
    # YANTRA_RECORD_EVENTS=<log>: the clock log, yantra-run --record-events's format; rec_fd = fd + 1
    adr(0, "env_rec")
    A.bl("rt_getenv")
    A.cbz(0, "rt_s1b")
    if mac:
        A.movi(1, 0x601)                       # O_WRONLY | O_CREAT | O_TRUNC
        A.movz(2, 420)
        sysc(0, 5)
        A.bcond(HS, "rt_no_rec")
    else:
        A.mov(1, 0)
        A.movn(0, 99)                          # AT_FDCWD
        A.movi(2, 0x241)
        A.movz(3, 420)
        A.movz(8, 56)
        A.svc()
        A.cmp_i(0, 0)
        A.bcond(LT, "rt_no_rec")
    A.add_i(9, 0, 1)
    A.mem("str", 9, SPR, FR["rec_fd"])
    adr(1, "rec_hdr")
    A.movi(2, REC_HEADER_LEN)
    sysc(64, 4)
    A.label("rt_s1b")
    # the input file: openat + fstat for its length
    A.movn(9, 0)
    A.mem("str", 9, SPR, FR["in_fd"])
    A.mem("str", ZR, SPR, FR["in_len"])
    A.mem("str", ZR, SPR, FR["name_len"])
    adr(0, "env_input")
    A.bl("rt_getenv")
    A.cbz(0, "rt_s2")
    if mac == "win":
        wcall("w_open")                        # CreateFileW: a handle, or -1
        A.cmp_i(0, 0)
        A.bcond(LT, "rt_no_input")
    elif mac:
        A.movz(1, 0)                           # open(path, O_RDONLY)
        A.movz(2, 0)
        sysc(0, 5)
        A.bcond(HS, "rt_no_input")
    else:
        A.mov(1, 0)
        A.movn(0, 99)                          # AT_FDCWD
        A.movz(2, 0)
        A.movz(3, 0)
        A.movz(8, 56)
        A.svc()
        A.cmp_i(0, 0)
        A.bcond(LT, "rt_no_input")
    A.mem("str", 0, SPR, FR["in_fd"])
    if mac == "win":
        wcall("w_size")                        # GetFileSizeEx
        A.mem("str", 0, SPR, FR["in_len"])
    else:
        A.add_i(1, SPR, FR["statbuf"])
        sysc(80, 339)
        A.mem("ldr", 9, SPR, FR["statbuf"] + (96 if mac else 48))   # st_size
        A.mem("str", 9, SPR, FR["in_len"])
    # YANTRA_INPUT_ENTRY="<module> <routine>" (yantra-run): the name run is module NUL routine
    adr(0, "env_entry")
    A.bl("rt_getenv")
    A.cbnz(0, "rt_entry")
    adr(0, "env_name")
    A.bl("rt_getenv")
    A.mem("str", 0, SPR, FR["name_ptr"])
    A.cbz(0, "rt_no_name")                    # neither a name nor an entry: refused, as yantra-run
    A.bl("rt_strlen")
    A.mem("str", 0, SPR, FR["name_len"])
    A.label("rt_s2")
    # the arguments: arg0 (yantra's image path) and argv[1..], joined by zero octets
    A.bl("rt_args_len")
    A.mem("str", 0, SPR, FR["args_len"])
    # mmap(base, ram + in_len + name_len + args_len + 64, RW, PRIVATE|ANON|FIXED_NOREPLACE)
    A.mem("ldr", 1, SPR, FR["ram"])
    A.mem("ldr", 9, SPR, FR["in_len"])
    A.rrr("add", 1, 1, 9)
    A.mem("ldr", 9, SPR, FR["name_len"])
    A.rrr("add", 1, 1, 9)
    A.mem("ldr", 9, SPR, FR["args_len"])
    A.rrr("add", 1, 1, 9)
    A.add_i(1, 1, 64)
    A.add_i(1, 1, 0xFFF)
    A.and_l(1, 1, LOG_NOT4095)
    if mac == "win":
        wcall("w_alloc")                       # VirtualAlloc anywhere: an address, or 0
        A.cbz(0, "rt_no_ram")
        A.mem("str", 0, SPR, FR["hbase"])
        A.movi(9, base)
        A.rrr("sub", 9, 0, 9)
        A.mem("str", 9, SPR, FR["delta"])
    elif mac:
        # anywhere: a hard 4 GiB __PAGEZERO covers base on Darwin arm64
        A.movz(0, 0)
        A.movz(2, 3)
        A.movi(3, 0x1000 | 0x2)                # MAP_ANON | MAP_PRIVATE
        A.movn(4, 0)
        A.movz(5, 0)
        sysc(0, 197)
        A.bcond(HS, "rt_no_ram")
        A.mem("str", 0, SPR, FR["hbase"])
        A.movi(9, base)
        A.rrr("sub", 9, 0, 9)
        A.mem("str", 9, SPR, FR["delta"])
    else:
        A.movi(0, base)
        A.movz(2, 3)
        A.movi(3, 0x22 | 0x100000)
        A.movn(4, 0)
        A.movz(5, 0)
        A.movz(8, 222)
        A.svc()
        A.movi(9, base)
        A.cmp(0, 9)
        A.bcond(NE, "rt_no_ram")
    # copy the image
    hbase(0)
    adr(1, "image")
    A.movi(2, filesz)
    A.label("rt_cp")
    A.cbz(2, "rt_cpd")
    A.mem("ldrb", 9, 1, 0)
    A.mem("strb", 9, 0, 0)
    A.add_i(0, 0, 1)
    A.add_i(1, 1, 1)
    A.add_i(2, 2, -1)
    A.b_("rt_cp")
    A.label("rt_cpd")
    # top = ram; inject the input (yantra input.rs order: input, then arguments)
    A.mem("ldr", 9, SPR, FR["ram"])
    A.mem("str", 9, SPR, FR["top"])
    A.mem("ldr", 9, SPR, FR["in_fd"])
    A.cmp_i(9, 0)
    A.bcond(LT, "rt_s3")
    for tag, reg in ((0x5455_504E_4953_4153, 19), (0x454D_414E_4953_4153, 20), (0x4543_4152_5453_4153, 21)):
        A.movi(0, tag)
        A.bl("rt_find_tag")
        A.cbz(0, "rt_no_tag")
        A.mov(reg, 0)
    # the source run: its storage, then read() the file straight into it
    A.bl("rt_run_open")
    A.mov(22, 0)                               # storage
    A.mem("ldr", 9, SPR, FR["in_len"])
    A.mem("str", 9, 22, -8)
    A.mov(1, 22)
    A.label("rt_read")
    A.mem("ldr", 2, SPR, FR["in_len"])
    A.rrr("add", 2, 2, 22)                     # end = storage + len
    A.rrr("sub", 2, 2, 1)
    A.cbz(2, "rt_read_done")
    A.mem("ldr", 0, SPR, FR["in_fd"])
    A.mov(23, 1)
    sysc(63, 3)
    if darwin:
        A.bcond(HS, "rt_no_input")
    A.mov(1, 23)
    A.cmp_i(0, 0)
    A.bcond(LE, "rt_no_input")
    A.rrr("add", 1, 1, 0)
    A.b_("rt_read")
    A.label("rt_read_done")
    A.mem("ldr", 9, SPR, FR["in_len"])
    A.rrr("add", 9, 9, 22)
    hbase(10)
    A.rrr("sub", 9, 9, 10)
    A.mem("str", 9, SPR, FR["top"])
    guest_word(22, 19)                         # the input tag's word
    # the name run
    A.bl("rt_run_open")
    A.mov(22, 0)
    A.mem("ldr", 2, SPR, FR["name_len"])
    A.mem("str", 2, 22, -8)
    A.mem("ldr", 1, SPR, FR["name_ptr"])
    A.mov(10, 22)
    A.label("rt_nm")
    A.cbz(2, "rt_nmd")
    A.mem("ldrb", 9, 1, 0)
    A.mem("strb", 9, 10, 0)
    A.add_i(10, 10, 1)
    A.add_i(1, 1, 1)
    A.add_i(2, 2, -1)
    A.b_("rt_nm")
    A.label("rt_nmd")
    hbase(11)
    A.rrr("sub", 9, 10, 11)
    A.mem("str", 9, SPR, FR["top"])
    guest_word(22, 20)                         # the name tag's word
    # the trace word: YANTRA_INPUT_TRACE or 0
    adr(0, "env_trace")
    A.bl("rt_getenv")
    A.cbz(0, "rt_s4")
    A.bl("rt_atou")
    A.label("rt_s4")
    A.mem("str", 0, 21, 8)
    A.label("rt_s3")
    # arguments, when the image declares both tags
    A.movi(0, 0x0056_4752_4153_4153)
    A.bl("rt_find_tag")
    A.cbz(0, "rt_s5")
    A.mov(19, 0)
    A.movi(0, 0x0043_4752_4153_4153)
    A.bl("rt_find_tag")
    A.cbz(0, "rt_s5")
    A.mov(20, 0)
    A.bl("rt_run_open")
    A.mov(22, 0)
    A.mov(10, 0)
    # arg0, then argv[1..]
    adr(1, "arg0")
    A.bl("rt_copyz")
    A.movz(24, 1)
    A.label("rt_argl")
    A.mem("ldr", 9, SPR, FR["argc"])
    A.cmp(24, 9)
    A.bcond(HS, "rt_arge")
    A.mem("strb", ZR, 10, 0)
    A.add_i(10, 10, 1)
    A.mem("ldr", 9, SPR, FR["argv"])
    A.memr("ldr", 1, 9, 24, 1)                 # argv[x24]
    A.bl("rt_copyz")
    A.add_i(24, 24, 1)
    A.b_("rt_argl")
    A.label("rt_arge")
    A.rrr("sub", 9, 10, 22)
    A.mem("str", 9, 22, -8)
    hbase(11)
    A.rrr("sub", 9, 10, 11)
    A.mem("str", 9, SPR, FR["top"])
    guest_word(22, 19)                         # the argv tag's word
    A.mem("ldr", 9, SPR, FR["argc"])
    A.mem("str", 9, 20, 8)                     # the argc tag's word
    A.label("rt_s5")
    # the event slot: the SASEVENT tag's address if it occurs exactly once (0 otherwise)
    A.movi(0, EVENT_TAG)
    A.bl("rt_find_tag")
    A.mem("str", 0, SPR, FR["ev_slot"])
    # a threaded image (SASTHRDS anywhere) is no clock wait: yantra's thread records answer it
    A.movi(0, THREADS_TAG)
    A.bl("rt_find_tag")
    A.cbnz(12, "rt_threaded")                  # a threaded image is refused before it runs (see x86-64)
    # limits: a load may reach top (the slab), a store only ram (yantra's store_limit);
    # each guard pair is (lo, lim): the access is in bounds when (addr - lo) <u lim
    A.movi(9, base + prot_size)                # code_end bounds stores: the protected code span only
    A.mem("str", 9, SPR, FR["code_end"])
    for width in (1, 2, 4, 8):
        A.movi(10, base)
        A.mem("ldr", 9, SPR, FR["top"])
        A.add_i(9, 9, -(width - 1))
        A.stp(10, 9, SPR, FR["g_l%d" % width])
        A.mem("ldr", 10, SPR, FR["code_end"])
        A.mem("ldr", 9, SPR, FR["ram"])
        A.movi(11, prot_size + width - 1)
        A.rrr("sub", 9, 9, 11)
        A.stp(10, 9, SPR, FR["g_s%d" % width])
    # the sp window for FAST blocks: sp in [code_end + 2048, base + ram - 2056] makes every
    # sp+imm (|imm| <= 2048, width <= 8) a RAM access outside the code span; span = hi - lo + 1
    A.movi(10, base + code_size + 2048)
    A.mem("ldr", 9, SPR, FR["ram"])
    A.movi(11, code_size + 4103)
    A.rrr("subs", 9, 9, 11)
    A.bcond(PL, "rt_spw")
    A.movz(9, 0)
    A.label("rt_spw")
    A.stp(10, 9, SPR, FR["sp_win"])
    adr(9, "jt")
    A.mem("str", 9, SPR, FR["jt_addr"])
    # the guest: every register zero, pc = entry
    for n in sorted(MAP):
        A.movz(MAP[n], 0)
    if mac:
        A.mem("ldr", DELTA, SPR, FR["delta"])
    A.movi(T0, entry)
    A.b_("rt_indirect")

    # ---- jalr: T0 = target
    A.label("rt_indirect")
    A.movi(T2, base)
    A.rrr("sub", T1, T0, T2)
    A.movi(T2, code_size)
    A.cmp(T1, T2)
    A.bcond(HS, "rt_bad_target")
    A.tst_l(T1, LOG_3)
    A.bcond(NE, "rt_bad_target")
    A.lsl_i(T1, T1, 1)
    A.mem("ldr", T2, SPR, FR["jt_addr"])
    if mac:
        A.memr("ldr", T1, T2, T1)              # the entry is an offset from the table
        A.rrr("add", T2, T2, T1)
    else:
        A.memr("ldr", T2, T2, T1)
    A.br(T2)

    # ---- a store that missed RAM: T0 = addr, T1 = value, fault_off = pc - base; bl'd
    A.label("rt_store_slow")
    A.movi(T2, UART)
    A.cmp(T0, T2)
    A.bcond(NE, "rt_st2")
    A.mem("ldr", T2, SPR, FR["outlen"])
    A.add_i(T0, SPR, FR["outbuf"])
    A.memr("strb", T1, T0, T2)
    A.add_i(T2, T2, 1)
    A.mem("str", T2, SPR, FR["outlen"])
    A.cmp_i12(T2, 16)                          # 65536
    A.bcond(LO, "rt_st1")
    A.mem("str", 30, SPR, FR["lr_slow"])
    A.bl("rt_flush")
    A.mem("ldr", 30, SPR, FR["lr_slow"])
    A.label("rt_st1")
    A.ret()
    A.label("rt_st2")
    A.movi(T2, WAIT)
    A.cmp(T0, T2)
    A.bcond(NE, "rt_st3")
    A.b_("rt_wait")                            # returns to the cold stub's caller
    A.label("rt_st3")
    A.movi(T2, FINISHER)
    A.cmp(T0, T2)
    A.bcond(NE, "rt_st_win")
    # FINISHER: flush, print yantra's halt line, exit 0 on 0x5555, else 1
    A.mem("str", T1, SPR, FR["fin_val"])
    A.bl("rt_flush")
    A.bl("m_fin1")
    A.mem("ldr", 9, SPR, FR["fin_val"])
    A.bl("rt_putu")
    A.mem("ldr", 9, SPR, FR["fin_val"])
    A.ubfm(10, 9, 0, 15)                       # value & 0xFFFF
    A.movi(11, 0x5555)
    A.cmp(10, 11)
    A.bcond(EQ, "rt_fin_ok")
    A.movi(11, 0x3333)
    A.cmp(10, 11)
    A.bcond(NE, "rt_fin_none")
    A.bl("m_fin2")
    A.mem("ldr", 9, SPR, FR["fin_val"])
    A.lsr_i(9, 9, 16)
    A.bl("rt_putu")
    A.bl("m_fin3")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_fin_ok")
    A.bl("m_fin2")
    A.movz(9, 0)
    A.bl("rt_putu")
    A.bl("m_fin3")
    A.movz(0, 0)
    A.b_("rt_exit")
    A.label("rt_fin_none")
    A.bl("m_fin4")
    A.movz(0, 1)
    A.b_("rt_exit")
    runtime_window_a64(A, FR, base, prot_size, mac, sysc, wcall, adr)
    runtime_spawn_a64(A, FR, base, mac, sysc, wcall, adr)
    # a store that missed RAM and is not UART/FINISHER: T0 = addr
    A.label("rt_beyond")
    A.mem("str", T0, SPR, FR["fault_addr"])
    A.bl("rt_flush")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.movi(10, 1 << 31)
    A.cmp(9, 10)
    A.bcond(LO, "rt_store_bad")                # below 2^31, no device
    A.mem("ldr", 10, SPR, FR["code_end"])
    A.cmp(9, 10)
    A.bcond(LO, "rt_store_code")               # in the code span
    A.bl("m_beyondstore")
    A.b_("rt_fault_tail")
    A.label("rt_store_bad")
    A.bl("m_badstore")
    A.b_("rt_fault_tail")
    A.label("rt_store_code")
    A.bl("m_codestore")
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    A.bl("m_addr")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.bl("rt_puthex")
    A.bl("m_codestore2")
    A.movz(0, 1)
    A.b_("rt_exit")
    # a load that missed RAM: T0 = addr
    A.label("rt_load_slow")
    A.mem("str", T0, SPR, FR["fault_addr"])
    A.bl("rt_flush")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.movi(10, 1 << 31)
    A.cmp(9, 10)
    A.bcond(LO, "rt_load_bad")                 # below 2^31 (a device): not in milestone 1
    A.bl("m_beyondload")
    A.b_("rt_fault_tail")
    A.label("rt_load_bad")
    # yantra's load answers BadAccess below base except at its load devices (runtime(): LOAD_DEVICES)
    for lo, n in LOAD_DEVICES:
        A.movi(10, lo)
        A.rrr("sub", 10, 9, 10)
        A.movi(11, n)
        A.cmp(10, 11)
        A.bcond(LO, "rt_load_dev")
    A.bl("m_bada1")
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    A.bl("m_bada2")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.bl("rt_putu")
    A.bl("m_bada3")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_load_dev")
    A.bl("m_badload")
    A.label("rt_fault_tail")                   # prints pc (decimal) then addr (hex)
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    A.bl("m_addr")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.bl("rt_puthex")
    A.bl("m_nl")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_bad_insn")                     # pc only (no address)
    A.bl("rt_flush")
    A.bl("m_insn")
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    A.bl("m_nl")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_bad_target")                   # T0 = the untranslated target
    A.mem("str", T0, SPR, FR["fault_addr"])
    A.bl("rt_flush")
    A.bl("m_jump1")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.bl("rt_puthex")
    A.bl("m_jump2")
    A.movz(0, 1)
    A.b_("rt_exit")
    # ---- YANTRA_INPUT_ENTRY: x0 = its value (see the x86-64 runtime for the rules)
    A.label("rt_entry")
    A.mem("str", 0, SPR, FR["name_ptr"])
    adr(0, "env_name")
    A.bl("rt_getenv")
    A.cbnz(0, "rt_entry_both")
    A.mem("ldr", 0, SPR, FR["name_ptr"])
    A.bl("rt_strlen")
    A.mem("str", 0, SPR, FR["name_len"])
    # over 4096 octets, or not UTF-8: refused before the shape is looked at (yantra-run)
    A.movz(9, 4096)
    A.cmp(0, 9)
    A.bcond(HI, "rt_entry_long")
    A.mem("ldr", 11, SPR, FR["name_ptr"])
    A.label("rt_eu1")
    A.mem("ldrb", 9, 11, 0)
    A.cbz(9, "rt_eu2")
    A.bl("rt_u8")
    A.cbnz(15, "rt_entry_bad8")
    A.rrr("add", 11, 11, 14)
    A.b_("rt_eu1")
    A.label("rt_eu2")
    A.mem("ldr", 0, SPR, FR["name_len"])
    A.mov(10, 0)                               # n
    A.mem("ldr", 11, SPR, FR["name_ptr"])      # s
    A.movz(12, 0)                              # the first space's index
    A.label("rt_en1")
    A.cmp(12, 10)
    A.bcond(HS, "rt_entry_bad")
    A.memr("ldrb", 13, 11, 12)
    A.cmp_i(13, 32)
    A.bcond(EQ, "rt_en2")
    A.add_i(12, 12, 1)
    A.b_("rt_en1")
    A.label("rt_en2")
    A.cbz(12, "rt_entry_bad")
    A.add_i(13, 12, 1)
    A.cmp(13, 10)
    A.bcond(HS, "rt_entry_bad")
    A.movz(14, 0)
    A.label("rt_ew1")
    A.cmp(14, 10)
    A.bcond(HS, "rt_ew_ok")
    A.cmp(14, 12)
    A.bcond(EQ, "rt_ew_next")
    A.rrr("add", 13, 11, 14)
    A.mem("ldrb", 9, 13, 0)
    A.cmp_i(9, 32)
    A.bcond(EQ, "rt_entry_bad")
    A.cmp_i(9, 9)
    A.bcond(LO, "rt_ew_hi")
    A.cmp_i(9, 13)
    A.bcond(LS, "rt_entry_bad")
    A.label("rt_ew_hi")
    A.cmp_i(9, 0xC2)                           # U+0085 U+00A0
    A.bcond(NE, "rt_ew_e1")
    A.mem("ldrb", 9, 13, 1)
    A.cmp_i(9, 0x85)
    A.bcond(EQ, "rt_entry_bad")
    A.cmp_i(9, 0xA0)
    A.bcond(EQ, "rt_entry_bad")
    A.b_("rt_ew_next")
    A.label("rt_ew_e1")
    A.cmp_i(9, 0xE1)                           # U+1680
    A.bcond(NE, "rt_ew_e2")
    A.mem("ldrb", 9, 13, 1)
    A.cmp_i(9, 0x9A)
    A.bcond(NE, "rt_ew_next")
    A.mem("ldrb", 9, 13, 2)
    A.cmp_i(9, 0x80)
    A.bcond(EQ, "rt_entry_bad")
    A.b_("rt_ew_next")
    A.label("rt_ew_e2")
    A.cmp_i(9, 0xE2)                           # U+2000..200A 2028 2029 202F 205F
    A.bcond(NE, "rt_ew_e3")
    A.mem("ldrb", 9, 13, 1)
    A.cmp_i(9, 0x81)
    A.bcond(EQ, "rt_ew_e2b")
    A.cmp_i(9, 0x80)
    A.bcond(NE, "rt_ew_next")
    A.mem("ldrb", 9, 13, 2)
    A.add_i(9, 9, -0x80)
    A.cmp_i(9, 10)
    A.bcond(LS, "rt_entry_bad")
    for c in (0x28, 0x29, 0x2F):
        A.cmp_i(9, c)
        A.bcond(EQ, "rt_entry_bad")
    A.b_("rt_ew_next")
    A.label("rt_ew_e2b")
    A.mem("ldrb", 9, 13, 2)
    A.cmp_i(9, 0x9F)
    A.bcond(EQ, "rt_entry_bad")
    A.b_("rt_ew_next")
    A.label("rt_ew_e3")
    A.cmp_i(9, 0xE3)                           # U+3000
    A.bcond(NE, "rt_ew_next")
    A.mem("ldrb", 9, 13, 1)
    A.cmp_i(9, 0x80)
    A.bcond(NE, "rt_ew_next")
    A.mem("ldrb", 9, 13, 2)
    A.cmp_i(9, 0x80)
    A.bcond(EQ, "rt_entry_bad")
    A.label("rt_ew_next")
    A.add_i(14, 14, 1)
    A.b_("rt_ew1")
    A.label("rt_ew_ok")
    A.memr("strb", ZR, 11, 12)                 # module NUL routine
    A.b_("rt_s2")
    A.label("rt_no_name")
    A.bl("m_noname")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_entry_both")
    A.bl("m_entry_both")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_entry_long")
    A.bl("m_long1")
    A.mem("ldr", 9, SPR, FR["name_len"])
    A.bl("rt_putu")
    A.bl("m_long2")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_entry_bad8")
    A.bl("m_entry1")
    A.bl("rt_putq")
    A.bl("m_bad8")
    A.movz(0, 1)
    A.b_("rt_exit")
    # ---- u8: x11 = a position in a zero-ended string (see the x86-64 runtime): x15 = 0 and
    # x14 = the valid character's length, or x15 = the invalid sequence's length (1..3).
    # Uses x9 x10 x16 x17; a leaf.
    A.label("rt_u8")
    A.mem("ldrb", 9, 11, 0)
    for c, lab in ((0x80, "rt_u8_1"), (0xC2, "rt_u8_bad1"), (0xE0, "rt_u8_2"), (0xF0, "rt_u8_3"), (0xF5, "rt_u8_4")):
        A.cmp_i(9, c)
        A.bcond(LO, lab)
    for n in (1, 2, 3):
        A.label("rt_u8_bad%d" % n)
        A.movz(15, n)
        A.ret()
    A.label("rt_u8_1")
    A.movz(15, 0)
    A.movz(14, 1)
    A.ret()
    A.label("rt_u8_2")
    A.mem("ldrb", 10, 11, 1)
    A.add_i(10, 10, -0x80)
    A.cmp_i(10, 0x3F)
    A.bcond(HI, "rt_u8_bad1")
    A.movz(15, 0)
    A.movz(14, 2)
    A.ret()
    for n, lead_lo, lo, lead_hi, hi in ((3, 0xE0, 0xA0, 0xED, 0x9F), (4, 0xF0, 0x90, 0xF4, 0x8F)):
        A.label("rt_u8_%d" % n)
        A.movz(16, 0x80)
        A.movz(17, 0xBF)
        A.cmp_i(9, lead_lo)
        A.bcond(NE, "rt_u8_%da" % n)
        A.movz(16, lo)
        A.label("rt_u8_%da" % n)
        A.cmp_i(9, lead_hi)
        A.bcond(NE, "rt_u8_%db" % n)
        A.movz(17, hi)
        A.label("rt_u8_%db" % n)
        A.mem("ldrb", 10, 11, 1)
        A.cmp(10, 16)
        A.bcond(LO, "rt_u8_bad1")
        A.cmp(10, 17)
        A.bcond(HI, "rt_u8_bad1")
        for k in range(2, n):
            A.mem("ldrb", 10, 11, k)
            A.add_i(10, 10, -0x80)
            A.cmp_i(10, 0x3F)
            A.bcond(HI, "rt_u8_bad%d" % k)
        A.movz(15, 0)
        A.movz(14, n)
        A.ret()
    A.label("rt_entry_bad")
    A.bl("m_entry1")
    A.bl("rt_putq")
    A.bl("m_entry2")
    A.movz(0, 1)
    A.b_("rt_exit")

    # ---- putq: the entry value as Rust's {:?} writes it (see the x86-64 runtime), to stderr
    def put_c(c):
        A.movz(10, c)
        A.mem("strb", 10, 12, 0)
        A.add_i(12, 12, 1)
    A.label("rt_putq")
    A.mov(26, 30)                              # it calls rt_u8
    A.mem("ldr", 11, SPR, FR["name_ptr"])
    A.add_i(12, SPR, FR["outbuf"])
    put_c(0x22)
    A.label("rt_q1")
    A.mem("ldrb", 9, 11, 0)
    A.cbz(9, "rt_qend")
    # an invalid UTF-8 sequence goes out as \\xHH per octet (Rust's OsStr {:?})
    A.bl("rt_u8")
    A.cbnz(15, "rt_q_x")
    A.mov(25, 14)                              # the valid character's octets
    A.mem("ldrb", 9, 11, 0)
    for c, lab in ((0x22, "rt_q_esc"), (0x5C, "rt_q_esc"), (9, "rt_q_t"), (10, "rt_q_n"), (13, "rt_q_r")):
        A.cmp_i(9, c)
        A.bcond(EQ, lab)
    A.cmp_i(9, 0x20)
    A.bcond(LO, "rt_q_u1")
    A.cmp_i(9, 0x7F)
    A.bcond(EQ, "rt_q_u1")
    A.cmp_i(9, 0xC2)
    A.bcond(LO, "rt_q_plain")
    A.cmp_i(9, 0xE0)
    A.bcond(LO, "rt_q_2")
    A.cmp_i(9, 0xF0)
    A.bcond(HS, "rt_q_plain")
    # three octets: both continuations, or the lead goes out as it is
    A.mem("ldrb", 10, 11, 1)
    A.ubfm(16, 10, 6, 7)                       # the top two bits
    A.cmp_i(16, 2)
    A.bcond(NE, "rt_q_plain")
    A.mem("ldrb", 13, 11, 2)
    A.ubfm(16, 13, 6, 7)
    A.cmp_i(16, 2)
    A.bcond(NE, "rt_q_plain")
    A.ubfm(13, 13, 0, 5)
    A.ubfm(10, 10, 0, 5)
    A.lsl_i(10, 10, 6)
    A.rrr("add", 13, 13, 10)
    A.ubfm(10, 9, 0, 3)
    A.lsl_i(10, 10, 12)
    A.rrr("add", 13, 13, 10)
    A.movz(14, 3)
    A.b_("rt_q_tab")
    A.label("rt_q_2")
    A.mem("ldrb", 10, 11, 1)
    A.ubfm(16, 10, 6, 7)
    A.cmp_i(16, 2)
    A.bcond(NE, "rt_q_plain")
    A.ubfm(13, 10, 0, 5)
    A.ubfm(10, 9, 0, 4)
    A.lsl_i(10, 10, 6)
    A.rrr("add", 13, 13, 10)
    A.movz(14, 2)
    A.label("rt_q_tab")
    adr(16, "rt_esctab")
    A.label("rt_qt1")
    A.mem("ldrw", 10, 16, 0)
    A.mem("ldrw", 15, 16, 4)
    A.cbz(15, "rt_q_plain")
    A.cmp(13, 10)
    A.bcond(LO, "rt_qt2")
    A.cmp(13, 15)
    A.bcond(LS, "rt_q_u")
    A.label("rt_qt2")
    A.add_i(16, 16, 8)
    A.b_("rt_qt1")
    A.label("rt_q_plain")
    A.mem("strb", 9, 12, 0)
    A.add_i(12, 12, 1)
    A.add_i(11, 11, 1)
    A.label("rt_q_rest")                       # the character's other octets, as they are
    A.add_i(25, 25, -1)
    A.cbz(25, "rt_q1")
    A.mem("ldrb", 9, 11, 0)
    A.mem("strb", 9, 12, 0)
    A.add_i(12, 12, 1)
    A.add_i(11, 11, 1)
    A.b_("rt_q_rest")
    A.label("rt_q_esc")
    put_c(0x5C)
    A.b_("rt_q_plain")
    for lab, c in (("rt_q_t", 0x74), ("rt_q_n", 0x6E), ("rt_q_r", 0x72)):
        A.label(lab)
        A.movz(9, c)
        A.b_("rt_q_esc")
    A.label("rt_q_x")
    A.mem("ldrb", 9, 11, 0)
    put_c(0x5C)
    put_c(0x78)
    for lo_bit, lab in ((4, "rt_qxa"), (0, "rt_qxb")):
        A.ubfm(10, 9, lo_bit, lo_bit + 3)
        A.cmp_i(10, 10)
        A.bcond(LO, lab)
        A.add_i(10, 10, 7)
        A.label(lab)
        A.add_i(10, 10, 48)
        A.mem("strb", 10, 12, 0)
        A.add_i(12, 12, 1)
    A.add_i(11, 11, 1)
    A.add_i(15, 15, -1)
    A.cbnz(15, "rt_q_x")
    A.b_("rt_q1")
    A.label("rt_q_u1")
    A.mov(13, 9)                               # cp
    A.movz(14, 1)                              # its octets
    A.label("rt_q_u")                          # \\u{cp}: x13 = cp (< 0x10000), x14 = its octets
    for c in (0x5C, 0x75, 0x7B):
        put_c(c)
    A.movz(17, 0)                              # a digit shown yet
    A.movz(15, 12)                             # the shift
    A.label("rt_qh")
    A.dp2("lsrv", 10, 13, 15)
    A.ubfm(10, 10, 0, 3)
    A.cbnz(10, "rt_qhd")
    A.cbnz(17, "rt_qhd")
    A.cbnz(15, "rt_qhs")
    A.label("rt_qhd")
    A.movz(17, 1)
    A.cmp_i(10, 10)
    A.bcond(LO, "rt_qhx")
    A.add_i(10, 10, 39)
    A.label("rt_qhx")
    A.add_i(10, 10, 48)
    A.mem("strb", 10, 12, 0)
    A.add_i(12, 12, 1)
    A.label("rt_qhs")
    A.cbz(15, "rt_qhe")
    A.add_i(15, 15, -4)
    A.b_("rt_qh")
    A.label("rt_qhe")
    put_c(0x7D)
    A.rrr("add", 11, 11, 14)
    A.b_("rt_q1")
    A.label("rt_qend")
    put_c(0x22)
    A.add_i(1, SPR, FR["outbuf"])
    A.rrr("sub", 2, 12, 1)
    A.movz(0, 2)
    sysc(64, 4)
    A.mov(30, 26)
    A.ret()
    A.label("rt_no_input")
    A.bl("m_noinput")
    A.movz(0, 2)
    A.b_("rt_exit")
    A.label("rt_no_ram")
    A.bl("m_noram")
    A.movz(0, 2)
    A.b_("rt_exit")
    # "{arg0}: segment at {vaddr:#x} needs {need} bytes and RAM is {ram} — raise it", exit 1
    A.label("rt_seg_fail")                     # x19 = the segment's (vaddr, need)
    adr(0, "arg0")
    A.bl("rt_strlen")
    A.mov(2, 0)
    adr(1, "arg0")
    A.movz(0, 2)
    sysc(64, 4)
    A.bl("m_seg0")
    A.mem("ldr", 9, 19, 0)
    A.bl("rt_puthex")
    A.bl("m_seg1")
    A.mem("ldr", 9, 19, 8)
    A.bl("rt_putu")
    A.bl("m_seg2")
    A.mem("ldr", 9, SPR, FR["ram"])
    A.bl("rt_putu")
    A.bl("m_seg3")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_no_rec")
    A.bl("m_norec")
    A.movz(0, 2)
    A.b_("rt_exit")
    A.label("rt_threaded")
    A.bl("m_threaded")
    A.movz(0, 2)
    A.b_("rt_exit")
    A.label("rt_wt_notag")
    A.bl("rt_flush")
    A.bl("m_waitnotag")
    A.movz(0, 2)
    A.b_("rt_exit")
    A.label("rt_wt_plain")
    A.bl("rt_flush")
    A.bl("m_waitplain")
    A.movz(0, 2)
    A.b_("rt_exit")
    A.label("rt_no_tag")
    A.bl("m_notag")
    A.movz(0, 2)
    A.label("rt_exit")
    sysc(93, 1)

    # ---- flush: write(1, outbuf, outlen); saves x0 x1 x2 x8, uses T0..T2 not at all
    A.label("rt_flush")
    A.stp(0, 1, SPR, FR["sv01"])
    A.stp(2, 8, SPR, FR["sv28"])
    A.add_i(8 if darwin else 1, SPR, FR["outbuf"])     # Darwin's svc writes x1: the cursor is x8
    A.mem("ldr", 2, SPR, FR["outlen"])
    A.label("rt_fl1")
    A.cbz(2, "rt_fl2")
    A.movz(0, 1)
    if darwin:
        A.mov(1, 8)
    sysc(64, 4)
    if darwin:
        A.bcond(HS, "rt_fl2")
    A.cmp_i(0, 0)
    A.bcond(LE, "rt_fl2")
    A.rrr("add", 8 if darwin else 1, 8 if darwin else 1, 0)
    A.rrr("sub", 2, 2, 0)
    A.b_("rt_fl1")
    A.label("rt_fl2")
    A.mem("str", ZR, SPR, FR["outlen"])
    A.ldp(0, 1, SPR, FR["sv01"])
    A.ldp(2, 8, SPR, FR["sv28"])
    A.ret()

    # ---- getenv: x0 = "NAME=" (zero-ended); answer x0 = value pointer or 0
    A.label("rt_getenv")
    A.mem("ldr", 9, SPR, FR["envp"])
    A.label("rt_ge1")
    A.mem("ldr", 10, 9, 0)
    A.cbz(10, "rt_ge_no")
    A.mov(11, 0)
    A.label("rt_ge2")
    A.mem("ldrb", 12, 11, 0)
    A.cbz(12, "rt_ge_yes")
    A.mem("ldrb", 13, 10, 0)
    A.cmp(12, 13)
    A.bcond(NE, "rt_ge_next")
    A.add_i(11, 11, 1)
    A.add_i(10, 10, 1)
    A.b_("rt_ge2")
    A.label("rt_ge_next")
    A.add_i(9, 9, 8)
    A.b_("rt_ge1")
    A.label("rt_ge_yes")
    A.mov(0, 10)
    A.ret()
    A.label("rt_ge_no")
    A.movz(0, 0)
    A.ret()

    # ---- atou: x0 = decimal digits; x0 = value
    A.label("rt_atou")
    A.mov(9, 0)
    A.movz(0, 0)
    A.movz(11, 10)
    A.label("rt_at1")
    A.mem("ldrb", 10, 9, 0)
    A.add_i(10, 10, -48)
    A.cmp_i(10, 9)
    A.bcond(HI, "rt_at2")
    A.madd(0, 0, 11, 10)
    A.add_i(9, 9, 1)
    A.b_("rt_at1")
    A.label("rt_at2")
    A.ret()

    # ---- atou_ram: x0 = YANTRA_RAM's value; x0 = it as Rust's usize::from_str reads it (an
    # optional '+', then one or more decimal digits, nothing else, at most 2^64-1); x1 = 0, or 1
    # when it is not one (x9..x12 clobbered)
    A.label("rt_atou_ram")
    A.mov(9, 0)
    A.movz(0, 0)
    A.movz(11, 10)
    A.mem("ldrb", 10, 9, 0)
    A.cmp_i(10, 43)                            # '+'
    A.bcond(NE, "rt_ar0")
    A.add_i(9, 9, 1)
    A.label("rt_ar0")
    A.mem("ldrb", 10, 9, 0)
    A.cbz(10, "rt_ar_bad")                     # no digit
    A.label("rt_ar1")
    A.mem("ldrb", 10, 9, 0)
    A.cbz(10, "rt_ar_ok")
    A.add_i(10, 10, -48)
    A.cmp_i(10, 9)
    A.bcond(HI, "rt_ar_bad")
    A.umulh(12, 0, 11)
    A.cbnz(12, "rt_ar_bad")                    # the product leaves 64 bits
    A.madd(0, 0, 11, 10)
    A.cmp(0, 10)
    A.bcond(LO, "rt_ar_bad")                   # the sum wrapped
    A.add_i(9, 9, 1)
    A.b_("rt_ar1")
    A.label("rt_ar_ok")
    A.movz(1, 0)
    A.ret()
    A.label("rt_ar_bad")
    A.movz(1, 1)
    A.ret()

    # ---- strlen: x0; x0 = length
    A.label("rt_strlen")
    A.mov(9, 0)
    A.movz(0, 0)
    A.label("rt_sl1")
    A.memr("ldrb", 10, 9, 0)
    A.cbz(10, "rt_sl2")
    A.add_i(0, 0, 1)
    A.b_("rt_sl1")
    A.label("rt_sl2")
    A.ret()

    # ---- args_len: strlen(arg0) + 1 + sum(strlen(argv[i]) + 1)
    A.label("rt_args_len")
    A.mov(27, 30)
    adr(0, "arg0")
    A.bl("rt_strlen")
    A.add_i(25, 0, 1)
    A.movz(24, 1)
    A.label("rt_al1")
    A.mem("ldr", 9, SPR, FR["argc"])
    A.cmp(24, 9)
    A.bcond(HS, "rt_al2")
    A.mem("ldr", 9, SPR, FR["argv"])
    A.memr("ldr", 0, 9, 24, 1)
    A.bl("rt_strlen")
    A.add_i(25, 25, 1)
    A.rrr("add", 25, 25, 0)
    A.add_i(24, 24, 1)
    A.b_("rt_al1")
    A.label("rt_al2")
    A.mov(0, 25)
    A.mov(30, 27)
    A.ret()

    # ---- copyz: copy x1's zero-ended string to x10 (no terminator); x10 advances
    A.label("rt_copyz")
    A.mem("ldrb", 9, 1, 0)
    A.cbz(9, "rt_cz2")
    A.mem("strb", 9, 10, 0)
    A.add_i(10, 10, 1)
    A.add_i(1, 1, 1)
    A.b_("rt_copyz")
    A.label("rt_cz2")
    A.ret()

    # ---- run_open: storage = align16(top + 8) above base; x0 = its address
    A.label("rt_run_open")
    A.mem("ldr", 0, SPR, FR["top"])
    A.add_i(0, 0, 8 + 15)
    A.and_l(0, 0, LOG_NOT15)
    hbase(9)
    A.rrr("add", 0, 0, 9)
    A.ret()

    # ---- find_tag: x0 = the 8-octet tag; answer x0 = its address if it occurs EXACTLY
    # once in the file-backed image, else 0 (yantra scans every offset)
    A.label("rt_find_tag")
    hbase(9)
    if mac:
        A.movi(10, filesz - 7)
        A.rrr("add", 10, 10, 9)
    else:
        A.movi(10, base + filesz - 7)
    A.movz(11, 0)                              # hit
    A.movz(12, 0)                              # count
    A.label("rt_ft1")
    A.cmp(9, 10)
    A.bcond(HS, "rt_ft3")
    A.mem("ldr", 13, 9, 0)
    A.cmp(0, 13)
    A.bcond(NE, "rt_ft2")
    A.mov(11, 9)
    A.add_i(12, 12, 1)
    A.label("rt_ft2")
    A.add_i(9, 9, 1)
    A.b_("rt_ft1")
    A.label("rt_ft3")
    A.cmp_i(12, 1)
    A.bcond(NE, "rt_ft4")
    A.mov(0, 11)
    A.ret()
    A.label("rt_ft4")
    A.movz(0, 0)
    A.ret()

    # ---- clock: x0 = the wall time now in ns since the Unix epoch (clobbers x1, x8, T0..T2 and
    # `num`; rt_wait saved them). Linux: clock_gettime(CLOCK_REALTIME); Darwin: gettimeofday
    # (microseconds, scaled), the BSD syscall, not the commpage.
    A.label("rt_clock")
    if mac:
        A.add_i(0, SPR, FR["num"])
        A.movz(1, 0)
        A.movz(2, 0)
        sysc(0, 116)
        A.add_i(1, SPR, FR["num"])
        A.mem("ldr", T0, 1, 0)
        A.mem("ldrw", T1, 1, 8)                # tv_usec (32 bits)
        A.movi(T2, 1000)
        A.madd(T1, T1, T2, ZR)
    else:
        A.movz(0, 0)                           # CLOCK_REALTIME
        A.add_i(1, SPR, FR["num"])
        sysc(113, 0)
        A.add_i(1, SPR, FR["num"])
        A.mem("ldr", T0, 1, 0)
        A.mem("ldr", T1, 1, 8)
    A.movi(T2, 1000000000)
    A.madd(0, T0, T2, T1)
    A.ret()

    # ---- THE WAIT: the guest stored to WAIT. Preserves every guest register (x0 x1 x2 x8 and
    # x30 in the frame, T0..T2 are scratch). If the word two after the SASEVENT tag (the
    # deadline hint, wall ns) is later than now, sleep until it (Linux nanosleep; Darwin poll
    # with no descriptors, whole milliseconds, rounded up); then write the wall time at
    # tag + 8 and append `t=<ns>` to the log (rec_fd), if any.
    A.label("rt_wait")
    A.mem("str", 30, SPR, FR["lr_slow"])
    A.stp(0, 1, SPR, FR["sv01"])
    A.stp(2, 8, SPR, FR["sv28"])
    A.mem("ldr", T0, SPR, FR["ev_slot"])
    A.cbz(T0, "rt_wt_notag")
    A.mem("ldr", T1, SPR, FR["rec_fd"])        # no YANTRA_RECORD_EVENTS: yantra halts here
    A.cbz(T1, "rt_wt_plain")
    A.mem("ldr", T1, T0, 16)                   # T1 = the hint
    A.cbz(T1, "rt_wt3")
    A.mem("str", T1, SPR, FR["wt_tmp"])
    A.bl("rt_clock")
    A.mem("ldr", T1, SPR, FR["wt_tmp"])
    A.cmp(T1, 0)
    A.bcond(LS, "rt_wt3")                      # the hint has passed
    A.rrr("sub", T1, T1, 0)                    # T1 = ns to sleep
    # a hint more than about an hour (838 * 2^32 ns) ahead is IGNORED, never slept (see x86-64)
    A.lsr_i(T2, T1, 32)
    A.cmp_i(T2, 838)
    A.bcond(HI, "rt_wt3")
    if mac:
        A.movi(T2, 999999)
        A.rrr("add", T1, T1, T2)
        A.movi(T2, 1000000)
        A.dp2("udiv", 0, T1, T2)               # x0 = ms, rounded up
        A.movi(T2, 0x7FFFFFFF)
        A.cmp(0, T2)
        A.bcond(LS, "rt_wt2")
        A.mov(0, T2)
        A.label("rt_wt2")
        A.mov(2, 0)                            # poll(NULL, 0, ms)
        A.movz(0, 0)
        A.movz(1, 0)
        sysc(0, 230)
    else:
        A.movi(T2, 1000000000)
        A.dp2("udiv", 0, T1, T2)               # x0 = seconds
        A.msub(T1, 0, T2, T1)                  # T1 = ns
        A.add_i(1, SPR, FR["num"])
        A.mem("str", 0, 1, 0)
        A.mem("str", T1, 1, 8)
        A.mov(0, 1)                            # nanosleep(&ts, NULL)
        A.movz(1, 0)
        sysc(101, 0)
    A.label("rt_wt3")
    A.bl("rt_clock")
    A.mem("ldr", T0, SPR, FR["ev_slot"])
    A.mem("str", 0, T0, 8)
    A.mem("ldr", T1, SPR, FR["rec_fd"])
    A.cbz(T1, "rt_wt4")
    A.add_i(1, SPR, FR["num"] + 31)            # "t=<digits>\n", built backwards
    A.movz(T2, 10)
    A.mem("strb", T2, 1, 0)
    A.label("rt_wt5")
    A.dp2("udiv", T0, 0, T2)
    A.msub(T1, T0, T2, 0)
    A.add_i(T1, T1, 48)
    A.add_i(1, 1, -1)
    A.mem("strb", T1, 1, 0)
    A.mov(0, T0)
    A.cbnz(0, "rt_wt5")
    A.add_i(1, 1, -1)
    A.movz(T2, 0x3D)                           # '='
    A.mem("strb", T2, 1, 0)
    A.add_i(1, 1, -1)
    A.movz(T2, 0x74)                           # 't'
    A.mem("strb", T2, 1, 0)
    A.add_i(2, SPR, FR["num"] + 32)
    A.rrr("sub", 2, 2, 1)
    A.mem("ldr", 0, SPR, FR["rec_fd"])
    A.add_i(0, 0, -1)
    sysc(64, 4)
    A.label("rt_wt4")
    A.mem("ldr", 30, SPR, FR["lr_slow"])
    A.ldp(0, 1, SPR, FR["sv01"])
    A.ldp(2, 8, SPR, FR["sv28"])
    A.ret()

    # ---- putu: x9 as decimal to stderr
    A.label("rt_putu")
    A.add_i(1, SPR, FR["num"] + 31)
    A.movz(10, 10)
    A.label("rt_pu1")
    A.dp2("udiv", 11, 9, 10)
    A.msub(12, 11, 10, 9)
    A.add_i(12, 12, 48)
    A.add_i(1, 1, -1)
    A.mem("strb", 12, 1, 0)
    A.mov(9, 11)
    A.cbnz(9, "rt_pu1")
    A.add_i(2, SPR, FR["num"] + 31)
    A.rrr("sub", 2, 2, 1)
    A.movz(0, 2)
    sysc(64, 4)
    A.ret()

    # ---- puthex: x9 as 0x… hex (minimal digits) to stderr
    A.label("rt_puthex")
    A.add_i(1, SPR, FR["num"] + 31)
    A.label("rt_px1")
    A.ubfm(10, 9, 0, 3)                        # x9 & 15
    A.cmp_i(10, 10)
    A.bcond(LO, "rt_px2")
    A.add_i(10, 10, 39)                        # 'a'..'f'
    A.label("rt_px2")
    A.add_i(10, 10, 48)
    A.add_i(1, 1, -1)
    A.mem("strb", 10, 1, 0)
    A.lsr_i(9, 9, 4)
    A.cbnz(9, "rt_px1")
    A.movz(10, 0x78)                           # 'x'
    A.mem("strb", 10, 1, -1)
    A.movz(10, 0x30)                           # '0'
    A.mem("strb", 10, 1, -2)
    A.add_i(1, 1, -2)
    A.add_i(2, SPR, FR["num"] + 31)
    A.rrr("sub", 2, 2, 1)
    A.movz(0, 2)
    sysc(64, 4)
    A.ret()

    msg("m_fin1", b"halt: Finisher { value: ")
    msg("m_fin2", b", status: Some(")
    msg("m_fin3", b") }\n")
    msg("m_fin4", b", status: None }\n")
    msg("m_addr", b" addr ")
    msg("m_beyondstore", b"halt: BeyondRam (a store past RAM) at pc ")
    msg("m_badstore", b"halt: BadAccess (a store below 2^31 to no device) at pc ")
    msg("m_codestore", b"native: refused a store into the code span at pc ")
    msg("m_codestore2", b" (yantra would allow it; self-modifying code is not translated)\n")
    msg("m_beyondload", b"halt: BeyondRam (a load past RAM) at pc ")
    msg("m_badload", b"halt: BadAccess (a load below 2^31; MMIO loads are not in milestone 1) at pc ")
    msg("m_bada1", b"halt: BadAccess { pc: ")
    msg("m_bada2", b", addr: ")
    msg("m_bada3", b" }\n")
    msg("m_seg0", b": segment at ")
    msg("m_seg1", b" needs ")
    msg("m_seg2", b" bytes and RAM is ")
    msg("m_seg3", b" \xe2\x80\x94 raise it\n")
    msg("m_insn", b"native: not in milestone 1 (an instruction the translator does not lower) at pc ")
    msg("m_jump1", b"native: jump to ")
    msg("m_jump2", b", outside the translated code (yantra would execute it)\n")
    msg("m_nl", b"\n")
    msg("m_noinput", b"native: YANTRA_INPUT could not be read\n")
    msg("m_noram", b"native: guest RAM could not be mapped at its own address\n")
    msg("m_norec", b"native: YANTRA_RECORD_EVENTS could not be opened for writing\n")
    msg("m_waitnotag", b"native: not in milestone 1 (a WAIT that is no plain clock wait: the SASEVENT tag is missing or duplicated; run it under yantra-run)\n")
    msg("m_waitplain", b"native: not in milestone 1 (a WAIT with no YANTRA_RECORD_EVENTS: yantra-run halts there with exit 75, W-370; run it under yantra-run)\n")
    msg("m_notag", b"native: the image lacks an input tag, or has one twice\n")
    msg("m_threaded", b"native: not in milestone 1 (a threaded image: it carries the SASTHRDS tag, and yantra-run's threads are not native; run it under yantra-run)\n")
    msg("m_entry_both", b"input: YANTRA_INPUT_ENTRY and YANTRA_INPUT_NAME are both set \xe2\x80\x94 the entry form builds the name itself (module NUL routine); set one\n")
    msg("m_entry1", b"input: YANTRA_INPUT_ENTRY ")
    msg("m_long1", b"input: YANTRA_INPUT_ENTRY is ")
    msg("m_long2", b" octets, over the 4096 limit\n")
    msg("m_bad8", b" is not UTF-8 \xe2\x80\x94 module and routine names are UTF-8\n")
    msg("m_noname", b"input: YANTRA_INPUT needs YANTRA_INPUT_NAME (or YANTRA_INPUT_ENTRY) \xe2\x80\x94 the module name is passed, never guessed\n")
    msg("m_entry2", b" is malformed \xe2\x80\x94 it must be \"<module> <routine>\": exactly one ASCII space, both halves non-empty and without whitespace\n")
    if mac == "win":
        runtime_win_a64(A, FR)
        runtime_win_spawn_a64(A, FR)
    for lab, text in strings + [("env_ram", b"YANTRA_RAM=\0"), ("env_input", b"YANTRA_INPUT=\0"),
                                ("env_grant", b"YANTRA_GRANT_ENV=\0"), ("env_files", b"YANTRA_FILES=\0"),
                                ("env_name", b"YANTRA_INPUT_NAME=\0"), ("env_trace", b"YANTRA_INPUT_TRACE=\0"),
                                ("env_entry", b"YANTRA_INPUT_ENTRY=\0"),
                                ("env_rec", b"YANTRA_RECORD_EVENTS=\0"), ("rec_hdr", REC_HEADER)]:
        A.label(lab)
        A.raw(text)
    # the code points Rust's {:?} writes as \\u{..} that putq knows (frozen; (lo, hi) as u32
    # pairs, (0, 0) ends): C1, NBSP, soft hyphen, combining diacritics, the Devanagari marks,
    # Unicode spaces and format controls
    A.label("rt_esctab")
    A.raw(b"\x80\x00\x00\x00\xa0\x00\x00\x00\xad\x00\x00\x00\xad\x00\x00\x00\x00\x03\x00\x00\x6f\x03\x00\x00\x00\x09\x00\x00\x02\x09\x00\x00\x3a\x09\x00\x00\x3a\x09\x00\x00\x3c\x09\x00\x00\x3c\x09\x00\x00\x41\x09\x00\x00\x48\x09\x00\x00\x4d\x09\x00\x00\x4d\x09\x00\x00\x51\x09\x00\x00\x57\x09\x00\x00\x62\x09\x00\x00\x63\x09\x00\x00\x80\x16\x00\x00\x80\x16\x00\x00\x00\x20\x00\x00\x0f\x20\x00\x00\x28\x20\x00\x00\x2f\x20\x00\x00\x5f\x20\x00\x00\x6f\x20\x00\x00\x00\x30\x00\x00\x00\x30\x00\x00\xff\xfe\x00\x00\xff\xfe\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00")


def runtime_window_a64(A, FR, base, prot_size, mac, sysc, wcall, adr):
    """runtime_window for A64: T0 = addr, T1 = value, bl'd; every register but T0..T2 comes
    back as it went in (x0..x14 and x30 are kept in the frame's win_sv). Guest addresses are
    checked, then made host addresses by adding delta (Darwin, Windows)."""
    darwin = mac and mac != "win"
    SAVE = list(range(15)) + [30]

    def host(r):
        if mac:
            A.mem("ldr", T0, SPR, FR["delta"])
            A.rrr("add", r, r, T0)

    A.label("rt_st_win")
    A.movi(T2, WINDOW)
    A.rrr("sub", T2, T0, T2)
    A.cmp_i(T2, 0x80)
    A.bcond(HS, "rt_beyond")
    for off, lab in ((WIN_PATH, "rt_w_path"), (WIN_BUFFER, "rt_w_buf"), (WIN_STDIN, "rt_w_stdin"),
                     (WIN_ENV, "rt_w_env"), (WIN_PARAM, "rt_w_param"), (WIN_SPAWN, "rt_w_spawn")) + \
            tuple((o, "rt_w_files") for o in WIN_FILE_OPS):
        A.cmp_i(T2, off)
        A.bcond(EQ, lab)
    A.b_("rt_beyond")
    A.label("rt_w_path")
    A.mem("str", T1, SPR, FR["win_path"])
    A.ret()
    A.label("rt_w_param")
    A.mem("str", T1, SPR, FR["win_param"])
    A.ret()
    A.label("rt_w_buf")
    A.mem("str", T1, SPR, FR["win_buf"])
    A.ret()

    def save():
        for k, r in enumerate(SAVE):
            A.mem("str", r, SPR, FR["win_sv"] + 8 * k)
        A.mem("str", T1, SPR, FR["win_st"])

    # ---- the file operations (files_engine): T2 = the store's offset, the answer in "st"
    P = PwA64(A, FR, mac, adr)
    A.label("rt_w_files")
    save()
    A.mem("str", T2, SPR, P.off("op"))
    A.mem("str", ZR, SPR, P.off("lrsp"))
    A.bl("pw_main")
    A.mem("ldr", 0, SPR, P.off("st"))
    A.mem("ldr", 9, SPR, P.off("keep"))
    A.cbnz(9, "rt_w_keep")
    A.b_("rt_w_answer")
    # ---- read stdin: fill BUFFER from fd 0 (Windows: STD_INPUT_HANDLE); the count, 0 = end
    A.label("rt_w_stdin")
    save()
    A.bl("rt_flush")                           # what was printed is seen before the read blocks
    A.mem("ldr", 1, SPR, FR["win_buf"])
    A.bl("rt_w_hdr")
    A.cbnz(2, "rt_w_bad")
    A.bl("rt_w_wr")
    A.cbnz(2, "rt_w_bad")
    host(1)
    A.mov(11, 1)
    A.mov(12, 0)
    A.movz(13, 0)
    if mac == "win":
        wcall("w_stdin")
        A.mov(14, 0)
    A.label("rt_w_si1")
    A.cmp(13, 12)
    A.bcond(HS, "rt_w_si2")
    if mac == "win":
        A.mov(0, 14)
    else:
        A.movz(0, 0)
    A.rrr("add", 1, 11, 13)
    A.rrr("sub", 2, 12, 13)
    sysc(63, 3)
    if darwin:
        A.bcond(HS, "rt_w_si2")
    A.cmp_i(0, 0)
    A.bcond(LE, "rt_w_si2")                    # end of input, or an error (a broken pipe) as its end
    A.rrr("add", 13, 13, 0)
    A.b_("rt_w_si1")
    A.label("rt_w_si2")
    A.mov(0, 13)
    A.b_("rt_w_answer")
    # ---- an environment variable: PATH names it, BUFFER receives the value
    A.label("rt_w_env")
    save()
    adr(0, "env_grant")
    A.bl("rt_getenv")
    A.cbz(0, "rt_w_ngr")
    A.mem("ldrb", 9, 0, 0)
    A.cmp_i(9, 0x31)
    A.bcond(NE, "rt_w_ngr")
    A.mem("ldrb", 9, 0, 1)
    A.cbnz(9, "rt_w_ngr")                      # granted only by exactly "1"
    A.mem("ldr", 1, SPR, FR["win_path"])
    A.bl("rt_w_hdr")
    A.cbnz(2, "rt_w_bad")
    A.bl("rt_w_rd")
    A.cbnz(2, "rt_w_bad")
    host(1)
    A.mov(5, 1)                                # the name
    A.mov(6, 0)                                # its length
    A.mem("ldr", 1, SPR, FR["win_buf"])
    A.bl("rt_w_hdr")
    A.cbnz(2, "rt_w_bad")
    A.mov(7, 0)                                # the buffer's capacity
    # a name that is empty or holds "=" or a zero octet names no variable
    A.cbz(6, "rt_w_nf")
    A.movz(9, 0)
    A.label("rt_w_ev1")
    A.cmp(9, 6)
    A.bcond(HS, "rt_w_ev2")
    A.memr("ldrb", 10, 5, 9)
    A.cbz(10, "rt_w_nf")
    A.cmp_i(10, 0x3D)
    A.bcond(EQ, "rt_w_nf")
    A.add_i(9, 9, 1)
    A.b_("rt_w_ev1")
    A.label("rt_w_ev2")
    A.mem("ldr", 8, SPR, FR["envp"])           # the first "NAME=" entry, matched exactly
    A.label("rt_w_ee1")
    A.mem("ldr", 11, 8, 0)
    A.cbz(11, "rt_w_nf")
    A.movz(9, 0)
    A.label("rt_w_ee2")
    A.cmp(9, 6)
    A.bcond(HS, "rt_w_ee3")
    A.memr("ldrb", 10, 5, 9)
    A.memr("ldrb", 12, 11, 9)
    A.cmp(10, 12)
    A.bcond(NE, "rt_w_ee4")
    A.add_i(9, 9, 1)
    A.b_("rt_w_ee2")
    A.label("rt_w_ee3")
    A.rrr("add", 11, 11, 6)
    A.mem("ldrb", 10, 11, 0)
    A.cmp_i(10, 0x3D)
    A.bcond(EQ, "rt_w_ee5")
    A.label("rt_w_ee4")
    A.add_i(8, 8, 8)
    A.b_("rt_w_ee1")
    A.label("rt_w_ee5")
    A.add_i(0, 11, 1)                          # the value
    A.mov(13, 0)
    A.bl("rt_strlen")
    A.cmp(0, 7)
    A.bcond(HI, "rt_w_big")                    # the buffer is left untouched
    A.mem("ldr", 1, SPR, FR["win_buf"])
    A.bl("rt_w_wr")
    A.cbnz(2, "rt_w_bad")
    host(1)
    A.movz(9, 0)
    A.label("rt_w_cp")
    A.cmp(9, 0)
    A.bcond(HS, "rt_w_answer")
    A.memr("ldrb", 10, 13, 9)
    A.memr("strb", 10, 1, 9)
    A.add_i(9, 9, 1)
    A.b_("rt_w_cp")
    for lab, word in (("rt_w_ngr", ST_NOT_GRANTED), ("rt_w_nf", ST_NOT_FOUND), ("rt_w_big", ST_TOO_LARGE),
                      ("rt_w_bad", ST_BAD_ADDRESS)):
        A.label(lab)
        A.movi(0, word)
        if lab != "rt_w_bad":
            A.b_("rt_w_answer")
    # x0 = the word: into STATUS[0] when that is writable; PATH and BUFFER are forgotten
    A.label("rt_w_answer")
    A.mov(3, 0)
    A.mem("ldr", 1, SPR, FR["win_st"])
    A.movz(0, 8)
    A.bl("rt_w_wr")
    A.cbnz(2, "rt_w_an1")
    host(1)
    A.mem("str", 3, 1, 0)
    A.label("rt_w_an1")
    A.mem("str", ZR, SPR, FR["win_path"])
    A.mem("str", ZR, SPR, FR["win_buf"])
    A.mem("str", ZR, SPR, FR["win_param"])
    A.label("rt_w_an2")
    for k, r in enumerate(SAVE):
        A.mem("ldr", r, SPR, FR["win_sv"] + 8 * k)
    A.ret()
    # x1 = a run (guest): x2 = 0 and x0 = its length when its header is readable, else x2 = 1
    A.label("rt_w_hdr")
    A.movi(9, base + 8)
    A.cmp(1, 9)
    A.bcond(LO, "rt_w_no")
    A.movi(9, base)
    A.rrr("sub", 10, 1, 9)
    A.mem("ldr", 9, SPR, FR["top"])
    A.cmp(10, 9)
    A.bcond(HI, "rt_w_no")
    A.mov(10, 1)
    host(10)
    A.mem("ldr", 0, 10, -8)
    A.movz(2, 0)
    A.ret()
    # x1 (guest), x0 = n: x2 = 0 when [x1, x1 + n) is readable (rt_w_rd) or writable (rt_w_wr)
    for lab, lo, hi in (("rt_w_rd", base, "top"), ("rt_w_wr", base + prot_size, "ram")):
        A.label(lab)
        A.movi(9, lo)
        A.rrr("subs", 10, 1, 9)
        A.bcond(LO, "rt_w_no")
        A.mem("ldr", 9, SPR, FR[hi])
        if lo != base:
            A.movi(2, lo - base)
            A.rrr("sub", 9, 9, 2)
        A.cmp(10, 9)
        A.bcond(HI, "rt_w_no")
        A.rrr("sub", 9, 9, 10)
        A.cmp(0, 9)
        A.bcond(HI, "rt_w_no")
        A.movz(2, 0)
        A.ret()
    A.label("rt_w_no")
    A.movz(2, 1)
    A.ret()
    # x0 = the word, as rt_w_answer, the remembered runs KEPT (yantra's GO / PUT with a run missing)
    A.label("rt_w_keep")
    A.mov(3, 0)
    A.mem("ldr", 1, SPR, FR["win_st"])
    A.movz(0, 8)
    A.bl("rt_w_wr")
    A.cbnz(2, "rt_w_an2")
    if mac:
        A.mem("ldr", T0, SPR, FR["delta"])
        A.rrr("add", 1, 1, T0)
    A.mem("str", 3, 1, 0)
    A.b_("rt_w_an2")
    files_engine(P, "win" if mac == "win" else ("mac" if mac else "linux"), "a64", base, prot_size)


def _spn_dest_a64(A, label, F, base_reg):
    """dest for A64: x3 = the stream; x1 = where its next octets go, x2 = how many fit (as
    spn_dest on x86-64). Uses x9. base_reg holds the frame's address (sp, or x19 in a shim)."""
    A.label(label)
    A.mem("ldr", 1, base_reg, F("s"))
    A.mem("ldr", 2, base_reg, F("c0"))
    A.cbz(3, label + "_1")
    A.mem("ldr", 9, base_reg, F("cap"))
    A.rrr("add", 1, 1, 9)
    A.mem("ldr", 2, base_reg, F("c1"))
    A.label(label + "_1")
    A.mem("ldr", 9, base_reg, F("cap"))
    A.cmp(2, 9)
    A.bcond(HS, label + "_2")
    A.rrr("add", 1, 1, 2)
    A.rrr("sub", 2, 9, 2)
    A.ret()
    A.label(label + "_2")
    A.mem("ldr", 1, base_reg, F("s"))
    A.rrr("add", 1, 1, 9)
    A.rrr("add", 1, 1, 9)
    A.movz(2, 4096)
    A.ret()


def runtime_spawn_a64(A, FR, base, mac, sysc, wcall, adr):
    """runtime_spawn for A64 (the same answers, in the same order): rt_w_spawn is an arm of
    rt_st_win (T1 = the STATUS run). x0..x14 and x30 are kept in win_sv (rt_w_answer restores
    them), x19..x29 in spn_sv; guest addresses become host ones by adding delta (Darwin,
    Windows)."""
    darwin = mac and mac != "win"
    win = mac == "win"

    def F(n, add=0):
        return FR["spn"] + SPN[n] + add

    def ld(r, n, add=0):
        A.mem("ldr", r, SPR, F(n, add))

    def st(r, n, add=0):
        A.mem("str", r, SPR, F(n, add))

    def host(r):
        if mac:
            A.mem("ldr", T0, SPR, FR["delta"])
            A.rrr("add", r, r, T0)

    def onfail(label):
        """after a syscall: to label on an error (Darwin: the carry; Linux: negative)"""
        if darwin:
            A.bcond(HS, label)
        else:
            A.cmp_i(0, 0)
            A.bcond(LT, label)

    def word(n, v):
        A.movi(9, v)
        st(9, n)

    A.label("rt_w_spawn")
    for k, r in enumerate(list(range(15)) + [30]):     # runtime_window_a64's save()
        A.mem("str", r, SPR, FR["win_sv"] + 8 * k)
    A.mem("str", T1, SPR, FR["win_st"])
    for k, r in enumerate(range(19, 30)):
        A.mem("str", r, SPR, FR["spn_sv"] + 8 * k)
    A.bl("spn_main")
    for k, r in enumerate(range(19, 30)):
        A.mem("ldr", r, SPR, FR["spn_sv"] + 8 * k)
    A.b_("rt_w_answer")

    _spn_dest_a64(A, "spn_dest", F, SPR)
    # ---- copy: x11 octets from x10 to x9 (both advance); uses x12
    A.label("spn_copy")
    A.cbz(11, "spn_cp2")
    A.mem("ldrb", 12, 10, 0)
    A.mem("strb", 12, 9, 0)
    A.add_i(10, 10, 1)
    A.add_i(9, 9, 1)
    A.add_i(11, 11, -1)
    A.b_("spn_copy")
    A.label("spn_cp2")
    A.ret()

    # ---- main: x0 = STATUS[0] (words 1 and 2 written on success)
    A.label("spn_main")
    st(30, "lr")
    word("ans", ST_NOT_GRANTED)
    adr(0, "env_spawn")
    A.bl("rt_getenv")
    A.cbz(0, "spn_one")
    A.mem("ldrb", 9, 0, 0)
    A.cmp_i(9, 0x31)
    A.bcond(NE, "spn_one")
    A.mem("ldrb", 9, 0, 1)
    A.cbnz(9, "spn_one")                               # granted only by exactly "1"
    word("ans", ST_BAD_ADDRESS)
    A.mem("ldr", 1, SPR, FR["win_st"])
    A.movz(0, 24)
    A.bl("rt_w_wr")
    A.cbnz(2, "spn_one")
    host(1)
    st(1, "st")
    for sym, ptr, ln, chk in (("win_path", "path", "lp", "rt_w_rd"), ("win_param", "param", "lq", "rt_w_rd"),
                              ("win_buf", "buf", "cap", "rt_w_wr")):
        A.mem("ldr", 1, SPR, FR[sym])
        A.bl("rt_w_hdr")
        A.cbnz(2, "spn_one")
        A.bl(chk)
        A.cbnz(2, "spn_one")
        host(1)
        st(1, ptr)
        st(0, ln)
    # the scratch, as on x86-64: 2 cap + 8 KiB + 32 (lp + lq), in pages
    word("ans", ST_SPAWN_FAILED)
    ld(9, "lp")
    ld(10, "lq")
    A.rrr("add", 9, 9, 10)
    A.lsl_i(9, 9, 5)
    ld(10, "cap")
    A.rrr("add", 9, 9, 10)
    A.rrr("add", 9, 9, 10)
    A.movi(10, 8192 + 4095)
    A.rrr("add", 9, 9, 10)
    A.and_l(9, 9, LOG_NOT4095)
    st(9, "size")
    if win:
        A.mov(1, 9)
        wcall("w_alloc")                               # VirtualAlloc anywhere: an address, or 0
        A.cbz(0, "spn_one")
    else:
        A.movz(0, 0)
        A.mov(1, 9)
        A.movz(2, 3)
        A.movi(3, 0x1002 if darwin else 0x22)
        A.movn(4, 0)
        A.movz(5, 0)
        sysc(222, 197)
        onfail("spn_one")
    st(0, "s")
    ld(10, "cap")
    A.rrr("add", 9, 0, 10)
    A.rrr("add", 9, 9, 10)
    A.add_i12(9, 9, 1)                                 # + 4096
    st(9, "z")
    for ptr, ln in (("path", "lp"), ("param", "lq")):
        ld(10, ptr)
        ld(11, ln)
        A.bl("spn_copy")
        A.mem("strb", ZR, 9, 0)
        A.add_i(9, 9, 1)
        if ptr == "path":
            st(9, "zq")
    A.mov(19, 9)                                       # x19 = the copies' end
    # BadPath: a zero inside the path, a copy that is not UTF-8, or a quote in argv[0]
    word("ans", ST_BAD_PATH)
    ld(10, "z")
    ld(11, "lp")
    A.label("spn_pz")
    A.cbz(11, "spn_pz2")
    A.mem("ldrb", 12, 10, 0)
    A.cbz(12, "spn_free1")
    A.add_i(10, 10, 1)
    A.add_i(11, 11, -1)
    A.b_("spn_pz")
    A.label("spn_pz2")
    ld(11, "z")
    A.label("spn_u8")
    A.cmp(11, 19)
    A.bcond(HS, "spn_u8d")
    A.mem("ldrb", 12, 11, 0)
    A.cbnz(12, "spn_u8c")
    A.add_i(11, 11, 1)
    A.b_("spn_u8")
    A.label("spn_u8c")
    A.bl("rt_u8")
    A.cbnz(15, "spn_free1")
    A.rrr("add", 11, 11, 14)
    A.b_("spn_u8")
    A.label("spn_u8d")
    ld(10, "zq")
    A.label("spn_q0")
    A.mem("ldrb", 12, 10, 0)
    A.cbz(12, "spn_q0d")
    A.cmp_i(12, 0x22)
    A.bcond(EQ, "spn_free1")
    A.add_i(10, 10, 1)
    A.b_("spn_q0")
    A.label("spn_q0d")
    # argv: one pointer per zero-separated argument, then 0
    A.add_i(20, 19, 7)
    A.and_l(20, 20, LOG_NOT7)
    st(20, "argv")
    ld(10, "zq")
    A.mem("str", 10, 20, 0)
    A.add_i(21, 20, 8)
    A.movz(22, 1)
    ld(11, "lq")
    A.label("spn_av")
    A.cbz(11, "spn_avd")
    A.mem("ldrb", 12, 10, 0)
    A.add_i(10, 10, 1)
    A.add_i(11, 11, -1)
    A.cbnz(12, "spn_av")
    A.mem("str", 10, 21, 0)
    A.add_i(21, 21, 8)
    A.add_i(22, 22, 1)
    A.b_("spn_av")
    A.label("spn_avd")
    A.mem("str", ZR, 21, 0)
    A.add_i(21, 21, 8)
    st(22, "n")
    if win:
        st(21, "cmd")
        A.mov(10, 21)
        A.bl("spn_cmdline")
        ld(9, "cmd")
        A.rrr("sub", 9, 10, 9)
        st(9, "clen")
    for k in ("c0", "c1", "exit"):
        st(ZR, k)
    A.movz(9, 1)
    st(9, "fail")
    if win:
        wcall("w_spawn")
    else:
        A.bl("spn_launch")
    word("ans", ST_SPAWN_FAILED)
    ld(9, "fail")
    A.cbnz(9, "spn_free1")
    word("ans", ST_TOO_LARGE)
    ld(9, "c0")
    ld(10, "c1")
    A.rrr("add", 9, 9, 10)
    ld(10, "cap")
    A.cmp(9, 10)
    A.bcond(HI, "spn_free1")
    # it fits: stdout then stderr into BUFFER, then words 1 and 2; word 0 by rt_w_answer
    ld(9, "buf")
    ld(10, "s")
    ld(11, "c0")
    A.bl("spn_copy")
    ld(10, "s")
    ld(11, "cap")
    A.rrr("add", 10, 10, 11)
    ld(11, "c1")
    A.bl("spn_copy")
    ld(9, "st")
    ld(10, "c0")
    A.mem("str", 10, 9, 8)
    ld(10, "c1")
    A.mem("str", 10, 9, 16)
    ld(10, "exit")
    st(10, "ans")
    A.label("spn_free1")
    ld(0, "s")
    if win:
        wcall("w_free")
    else:
        ld(1, "size")
        sysc(215, 73)                                  # munmap
    A.label("spn_one")
    ld(0, "ans")
    ld(30, "lr")
    A.ret()

    if win:
        # ---- cmdline: argv as one Windows command line at x10, zero-ended (x10 ends at the
        # zero); win_join is the reference. Uses x1 x9 x11 x12 x20..x24.
        def bs(lab):
            A.label(lab)
            A.cbz(22, lab + "_d")
            A.movz(12, 0x5C)
            A.mem("strb", 12, 10, 0)
            A.add_i(10, 10, 1)
            A.add_i(22, 22, -1)
            A.b_(lab)
            A.label(lab + "_d")

        def put(c):
            A.movz(12, c)
            A.mem("strb", 12, 10, 0)
            A.add_i(10, 10, 1)

        A.label("spn_cmdline")
        A.mov(24, 30)
        ld(20, "argv")
        ld(23, "n")
        A.mem("ldr", 1, 20, 0)
        A.mem("ldrb", 9, 1, 0)
        A.cbz(9, "spn_c0q")
        A.mov(11, 1)
        A.label("spn_c0s")
        A.mem("ldrb", 9, 11, 0)
        A.cbz(9, "spn_c0p")
        for c in (0x20, 0x09):
            A.cmp_i(9, c)
            A.bcond(EQ, "spn_c0q")
        A.add_i(11, 11, 1)
        A.b_("spn_c0s")
        A.label("spn_c0p")
        A.bl("rt_copyz")
        A.b_("spn_cargs")
        A.label("spn_c0q")
        put(0x22)
        A.bl("rt_copyz")
        put(0x22)
        A.label("spn_cargs")
        A.movz(21, 1)
        A.label("spn_ca")
        A.cmp(21, 23)
        A.bcond(HS, "spn_cend")
        put(0x20)
        A.memr("ldr", 1, 20, 21, 1)
        A.add_i(21, 21, 1)
        A.mem("ldrb", 9, 1, 0)
        A.cbz(9, "spn_cq")
        A.mov(11, 1)
        A.label("spn_cs")
        A.mem("ldrb", 9, 11, 0)
        A.cbz(9, "spn_cp")
        for c in (0x20, 0x09, 0x22):
            A.cmp_i(9, c)
            A.bcond(EQ, "spn_cq")
        A.add_i(11, 11, 1)
        A.b_("spn_cs")
        A.label("spn_cp")
        A.bl("rt_copyz")
        A.b_("spn_ca")
        A.label("spn_cq")
        put(0x22)
        A.movz(22, 0)                                  # backslashes pending
        A.label("spn_cql")
        A.mem("ldrb", 9, 1, 0)
        A.cbz(9, "spn_cqe")
        A.add_i(1, 1, 1)
        A.cmp_i(9, 0x5C)
        A.bcond(NE, "spn_cqn")
        A.add_i(22, 22, 1)
        A.b_("spn_cql")
        A.label("spn_cqn")
        A.cmp_i(9, 0x22)
        A.bcond(NE, "spn_cqc")
        A.rrr("add", 22, 22, 22)
        A.add_i(22, 22, 1)                             # 2b + 1 before a quote
        A.label("spn_cqc")
        bs("spn_bs1")
        A.mem("strb", 9, 10, 0)
        A.add_i(10, 10, 1)
        A.b_("spn_cql")
        A.label("spn_cqe")
        A.rrr("add", 22, 22, 22)                       # 2b before the closing quote
        bs("spn_bs2")
        put(0x22)
        A.b_("spn_ca")
        A.label("spn_cend")
        A.mem("strb", ZR, 10, 0)
        A.mov(30, 24)
        A.ret()
    else:
        # ---- launch (Linux, Darwin): as runtime_spawn's spn_launch
        def close_x0():
            sysc(57, 6)

        A.label("spn_launch")
        st(30, "lr2")
        A.movn(9, 0)
        for k in range(3):
            st(9, "fds%d" % k)
        for k in range(3):
            if darwin:
                sysc(0, 42)                            # pipe: x0 the read end, x1 the write end
                onfail("spn_lf")
                A.mem("strw", 0, SPR, F("fds%d" % k))
                A.mem("strw", 1, SPR, F("fds%d" % k, 4))
                for half in (0, 4):
                    A.mem("ldrsw", 0, SPR, F("fds%d" % k, half))
                    A.movz(1, 2)                       # F_SETFD
                    A.movz(2, 1)                       # FD_CLOEXEC
                    sysc(0, 92)
            else:
                A.add_i(0, SPR, F("fds%d" % k))
                A.movz(1, 8, 1)                        # O_CLOEXEC = 0x80000
                sysc(59, 0)                            # pipe2
                onfail("spn_lf")
        if darwin:
            sysc(0, 2)                                 # fork: x1 = 1 in the child
            onfail("spn_lf")
            A.cbnz(1, "spn_child")
        else:
            A.movz(0, 17)                              # clone(SIGCHLD): a fork
            for r in (1, 2, 3, 4):
                A.movz(r, 0)
            sysc(220, 0)
            onfail("spn_lf")
            A.cbz(0, "spn_child")
        st(0, "pid")
        st(ZR, "fail")
        for k in range(3):
            A.mem("ldrsw", 0, SPR, F("fds%d" % k, 4))
            close_x0()
            A.mem("ldrsw", 9, SPR, F("fds%d" % k))
            A.mem("strw", 9, SPR, F("pfd%d" % k))
            A.movz(9, 1)                               # events POLLIN, revents 0
            A.mem("strw", 9, SPR, F("pfd%d" % k, 4))
        A.label("spn_poll")
        A.mem("ldrsw", 9, SPR, F("pfd0"))
        for k in (1, 2):
            A.mem("ldrsw", 10, SPR, F("pfd%d" % k))
            A.rrr("and", 9, 9, 10)
        A.cmp_i(9, 0)
        A.bcond(LT, "spn_wait")                        # all three closed
        A.add_i(0, SPR, F("pfd0"))
        A.movz(1, 3)
        if darwin:
            A.movn(2, 0)
            sysc(0, 230)                               # poll(fds, 3, -1)
            A.bcond(LO, "spn_pok")
            A.cmp_i(0, 4)                              # EINTR
        else:
            for r in (2, 3, 4):
                A.movz(r, 0)
            sysc(73, 0)                                # ppoll(fds, 3, NULL, NULL, 0)
            A.cmp_i(0, 0)
            A.bcond(GE, "spn_pok")
            A.cmp_i(0, -4)
        A.bcond(EQ, "spn_poll")
        A.b_("spn_wait")
        A.label("spn_pok")
        for k in range(3):
            nxt = "spn_pn%d" % k
            A.mem("ldrsw", 9, SPR, F("pfd%d" % k))
            A.cmp_i(9, 0)
            A.bcond(LT, nxt)
            A.mem("ldrh", 9, SPR, F("pfd%d" % k, 6))
            A.cbz(9, nxt)
            if k < 2:
                A.movz(3, k)
                A.bl("spn_dest")
            else:
                A.add_i(1, SPR, F("one"))
                A.movz(2, 8)
            A.mem("ldrsw", 0, SPR, F("pfd%d" % k))
            sysc(63, 3)
            if darwin:
                A.bcond(HS, "spn_pc%d" % k)
            A.cmp_i(0, 0)
            A.bcond(LE, "spn_pc%d" % k)
            if k < 2:
                ld(9, "c%d" % k)
                A.rrr("add", 9, 9, 0)
                st(9, "c%d" % k)
            else:
                A.movz(9, 1)
                st(9, "fail")
            A.b_(nxt)
            A.label("spn_pc%d" % k)
            A.mem("ldrsw", 0, SPR, F("pfd%d" % k))
            close_x0()
            A.movn(9, 0)
            A.mem("strw", 9, SPR, F("pfd%d" % k))
            A.label(nxt)
        A.b_("spn_poll")
        A.label("spn_wait")
        ld(0, "pid")
        A.add_i(1, SPR, F("ws"))
        A.movz(2, 0)
        A.movz(3, 0)
        sysc(260, 7)                                   # wait4
        if darwin:
            A.bcond(LO, "spn_wok")
            A.cmp_i(0, 4)
        else:
            A.cmp_i(0, 0)
            A.bcond(GE, "spn_wok")
            A.cmp_i(0, -4)
        A.bcond(EQ, "spn_wait")
        A.label("spn_wok")
        # exited: (ws >> 8) & 255; killed by signal s: 128 + s
        A.mem("ldrw", 9, SPR, F("ws"))
        A.and_l(10, 9, 1 << 12 | 6)                    # & 0x7F
        A.cbnz(10, "spn_sig")
        A.ubfm(9, 9, 8, 15)                            # (ws >> 8) & 0xFF
        st(9, "exit")
        ld(30, "lr2")
        A.ret()
        A.label("spn_sig")
        A.add_i(10, 10, 128)
        st(10, "exit")
        ld(30, "lr2")
        A.ret()
        A.label("spn_lf")                              # a pipe or the fork failed: close what is open
        for k in range(3):
            for half in (0, 4):
                lab = "spn_lf%d%d" % (k, half)
                A.mem("ldrsw", 0, SPR, F("fds%d" % k, half))
                A.cmp_i(0, 0)
                A.bcond(LT, lab)
                close_x0()
                A.label(lab)
        ld(30, "lr2")
        A.ret()
        # the child: nothing here returns
        def dup(fd):
            A.movz(1, fd)
            if darwin:
                sysc(0, 90)                            # dup2
            else:
                A.movz(2, 0)
                sysc(24, 0)                            # dup3(old, new, 0)

        A.label("spn_child")
        for k, fd in ((0, 1), (1, 2)):
            A.mem("ldrsw", 0, SPR, F("fds%d" % k, 4))
            dup(fd)
        if darwin:
            adr(0, "s_devnull")
            A.movz(1, 0)
            A.movz(2, 0)
            sysc(0, 5)
        else:
            A.movn(0, 99)                              # AT_FDCWD
            adr(1, "s_devnull")
            A.movz(2, 0)
            A.movz(3, 0)
            sysc(56, 0)
        dup(0)
        # the environment WITHOUT YANTRA_* (as runtime_spawn): compacted in the child's copy
        A.mem("ldr", 9, SPR, FR["envp"])
        A.mov(10, 9)
        A.label("spn_ev")
        A.mem("ldr", 11, 9, 0)
        A.cbz(11, "spn_evd")
        A.add_i(9, 9, 8)
        for k, ch in enumerate(b"YANTRA_"):
            A.mem("ldrb", 12, 11, k)
            A.cmp_i(12, ch)
            A.bcond(NE, "spn_evk")
        A.b_("spn_ev")
        A.label("spn_evk")
        A.mem("str", 11, 10, 0)
        A.add_i(10, 10, 8)
        A.b_("spn_ev")
        A.label("spn_evd")
        A.mem("str", ZR, 10, 0)
        ld(0, "z")
        ld(1, "argv")
        A.mem("ldr", 2, SPR, FR["envp"])
        sysc(221, 59)                                  # execve
        A.mem("ldrsw", 0, SPR, F("fds2", 4))           # it failed: one octet says so
        A.add_i(1, SPR, F("one"))
        A.movz(2, 1)
        sysc(64, 4)
        A.movz(0, 127)
        sysc(93, 1)
    A.label("env_spawn")
    A.raw(b"YANTRA_GRANT_SPAWN=\0")
    A.label("s_devnull")
    A.raw(b"/dev/null\0")
    A.align(4)


def runtime_win_spawn_a64(A, FR):
    """aarch64-windows: w_spawn and w_free, as runtime_win_spawn (x86-64) does them. bl'd
    through wcall with runtime_win_a64's contract; inside, x19 holds the frame's address
    (sp is the thread's own stack), and x19..x22 are free (rt_w_spawn saved them)."""
    SAVED = list(range(1, 18)) + [30]

    def F(n, add=0):
        return FR["spn"] + SPN[n] + add

    def ld(r, n, add=0):
        A.mem("ldr", r, 19, F(n, add))

    def st(r, n, add=0):
        A.mem("str", r, 19, F(n, add))

    def at(r, n, add=0):
        A.add_i(r, 19, F(n, add))

    def enter(label):
        A.label(label)
        for k, r in enumerate(SAVED):
            A.mem("str", r, SPR, FR["wsave"] + 8 * k)
        A.mem("ldr", 9, SPR, FR["os_sp"])
        A.sp_from(9)
        A.add_i(SPR, SPR, -64)

    def leave():
        A.adrl(9, "frame")
        A.sp_from(9)
        for k, r in enumerate(SAVED):
            A.mem("ldr", r, SPR, FR["wsave"] + 8 * k)
        A.ret()

    def imp(name):
        A.adrl(16, "iat_" + name)
        A.mem("ldr", 16, 16, 0)
        A.blr(16)

    def close(h, lab):
        ld(0, h)
        A.cbz(0, lab)
        imp("CloseHandle")
        st(ZR, h)
        A.label(lab)

    _spn_dest_a64(A, "w_spn_dest", F, 19)
    enter("w_spawn")
    A.adrl(19, "frame")
    for h in ("hnul", "h0r", "h0w", "h1r", "h1w", "hpr", "hth"):
        st(ZR, h)
    ld(9, "cmd")
    ld(10, "clen")
    A.rrr("add", 9, 9, 10)
    A.add_i(9, 9, 8)
    A.and_l(9, 9, LOG_NOT7)
    st(9, "wp")
    ld(10, "lp")
    A.add_i(10, 10, 8)
    A.rrr("add", 9, 9, 10)
    A.rrr("add", 9, 9, 10)
    A.and_l(9, 9, LOG_NOT7)
    st(9, "wc")
    for src, dst, ln in (("z", "wp", "lp"), ("cmd", "wc", "clen")):
        A.movi(0, 65001)                               # CP_UTF8
        A.movz(1, 0)
        ld(2, src)
        A.movn(3, 0)
        ld(4, dst)
        ld(5, ln)
        A.add_i(5, 5, 1)
        imp("MultiByteToWideChar")
        A.cbz(0, "w_sp_fail", 0)
    A.movz(9, 24)                                      # SECURITY_ATTRIBUTES: inherited
    st(9, "sa0")
    st(ZR, "sa1")
    A.movz(9, 1)
    st(9, "sa2")
    for k in range(2):
        at(0, "h%dr" % k)
        at(1, "h%dw" % k)
        at(2, "sa0")
        A.movz(3, 0)
        imp("CreatePipe")
        A.cbz(0, "w_sp_fail", 0)
        ld(0, "h%dr" % k)
        A.movz(1, 1)                                   # HANDLE_FLAG_INHERIT
        A.movz(2, 0)
        imp("SetHandleInformation")
    A.adrl(0, "w_nul")
    A.movi(1, 0x8000_0000)                             # GENERIC_READ
    A.movz(2, 3)                                       # FILE_SHARE_READ | FILE_SHARE_WRITE
    at(3, "sa0")
    A.movz(4, 3)                                       # OPEN_EXISTING
    A.movz(5, 0)
    A.movz(6, 0)
    imp("CreateFileW")
    A.cmp_i(0, -1)
    A.bcond(EQ, "w_sp_fail")
    st(0, "hnul")
    for i in range(13):
        st(ZR, "si", 8 * i)
    A.movz(9, 104)
    A.mem("strw", 9, 19, F("si"))                      # cb
    A.movz(9, 0x100)
    A.mem("strw", 9, 19, F("si", 60))                  # STARTF_USESTDHANDLES
    for off, h in ((80, "hnul"), (88, "h0w"), (96, "h1w")):
        ld(9, h)
        st(9, "si", off)
    ld(0, "wp")
    ld(1, "wc")
    A.movz(2, 0)
    A.movz(3, 0)
    A.movz(4, 1)                                       # bInheritHandles
    for r in (5, 6, 7):
        A.movz(r, 0)
    at(9, "si")
    A.mem("str", 9, SPR, 0)
    at(9, "pi0")
    A.mem("str", 9, SPR, 8)
    imp("CreateProcessW")
    A.cbz(0, "w_sp_fail", 0)
    ld(9, "pi0")
    st(9, "hpr")
    ld(9, "pi1")
    st(9, "hth")
    for h in ("h0w", "h1w", "hnul", "hth"):
        close(h, "w_sp_cl_" + h)
    st(ZR, "fail")
    A.label("w_sp_loop")
    A.movz(20, 0)                                      # progress
    for k in range(2):
        nxt, cl = "w_sp_n%d" % k, "w_sp_c%d" % k
        ld(0, "h%dr" % k)
        A.cbz(0, nxt)
        for r in (1, 2, 3):
            A.movz(r, 0)
        st(ZR, "avail")
        at(4, "avail")
        A.movz(5, 0)
        imp("PeekNamedPipe")
        A.cbz(0, cl, 0)                                # broken: the child's ends are all closed
        ld(9, "avail")
        A.cbz(9, nxt)
        A.movz(3, k)
        A.bl("w_spn_dest")
        ld(9, "avail")
        A.cmp(2, 9)
        A.csel(2, 9, 2, HI)
        ld(0, "h%dr" % k)
        st(ZR, "got")
        at(3, "got")
        A.movz(4, 0)
        imp("ReadFile")
        A.cbz(0, cl, 0)
        ld(9, "got")
        ld(10, "c%d" % k)
        A.rrr("add", 10, 10, 9)
        st(10, "c%d" % k)
        A.movz(20, 1)
        A.b_(nxt)
        A.label(cl)
        close("h%dr" % k, "w_sp_cr%d" % k)
        A.label(nxt)
    ld(9, "h0r")
    ld(10, "h1r")
    A.rrr("orr", 9, 9, 10)
    A.cbz(9, "w_sp_done")
    A.cbnz(20, "w_sp_loop")
    ld(0, "hpr")
    A.movz(1, 1)
    imp("WaitForSingleObject")
    A.b_("w_sp_loop")
    A.label("w_sp_done")
    ld(0, "hpr")
    A.movn(1, 0)                                       # INFINITE
    imp("WaitForSingleObject")
    st(ZR, "exit")
    ld(0, "hpr")
    at(1, "exit")
    imp("GetExitCodeProcess")
    close("hpr", "w_sp_cl_hpr")
    leave()
    A.label("w_sp_fail")
    for h in ("h0r", "h0w", "h1r", "h1w", "hnul"):
        close(h, "w_sp_f_" + h)
    leave()
    # ---- free(x0 = an address from w_alloc)
    enter("w_free")
    A.movz(1, 0)
    A.movz(2, 0x8000)                                  # MEM_RELEASE
    imp("VirtualFree")
    leave()
    A.align(2)
    A.label("w_nul")
    A.raw("NUL\0".encode("utf-16-le"))
    A.align(4)


def runtime_win_a64(A, FR):
    """The Windows shims. Each is bl'd with sp = the frame and Linux's register contract:
    x0..x2 are the arguments, x0 the answer (a count or a handle, -1 on failure), and every
    other register comes back as it went in. A shim saves x1..x17 and x30 in the frame,
    moves sp to the thread's own stack (os_sp, 64 octets of locals below it), calls
    kernel32 through the import table (x0..x7; x18, the TEB, untouched), and moves back."""
    SAVED = list(range(1, 18)) + [30]

    def enter(label):
        A.label(label)
        for k, r in enumerate(SAVED):
            A.mem("str", r, SPR, FR["wsave"] + 8 * k)
        A.mem("ldr", 9, SPR, FR["os_sp"])
        A.sp_from(9)
        A.add_i(SPR, SPR, -64)

    def leave():
        A.adrl(9, "frame")
        A.sp_from(9)
        for k, r in enumerate(SAVED):
            A.mem("ldr", r, SPR, FR["wsave"] + 8 * k)
        A.ret()

    def imp(name):
        A.adrl(16, "iat_" + name)
        A.mem("ldr", 16, 16, 0)
        A.blr(16)

    def ok_or_minus1(fail, done):
        """after a BOOL call: x0 = the DWORD at [sp, #32], or -1"""
        A.cbz(0, fail, 0)
        A.mem("ldrw", 0, SPR, 32)
        A.b_(done)
        A.label(fail)
        A.movn(0, 0)
        A.label(done)

    # ---- write(fd 1 or 2, buf, n): GetStdHandle, WriteFile
    enter("w_sys64")
    A.mem("str", 1, SPR, 16)
    A.mem("str", 2, SPR, 24)
    A.cmp_i(0, 2)
    A.movn(0, 10)                              # STD_OUTPUT_HANDLE = -11
    A.bskip(NE, 2)
    A.movn(0, 11)                              # STD_ERROR_HANDLE = -12
    imp("GetStdHandle")
    A.mem("ldr", 1, SPR, 16)
    A.mem("ldr", 2, SPR, 24)
    A.mem("str", ZR, SPR, 32)
    A.add_i(3, SPR, 32)
    A.movz(4, 0)
    imp("WriteFile")
    ok_or_minus1("w_wr_f", "w_wr_d")
    leave()
    # ---- read(handle, buf, n): ReadFile, n capped at 2^30 (a DWORD count; the caller loops)
    enter("w_sys63")
    A.movi(9, 0x4000_0000)
    A.cmp(2, 9)
    A.csel(2, 9, 2, HI)
    A.mem("str", ZR, SPR, 32)
    A.add_i(3, SPR, 32)
    A.movz(4, 0)
    imp("ReadFile")
    ok_or_minus1("w_rd_f", "w_rd_d")
    leave()
    # ---- the stdin handle: GetStdHandle(STD_INPUT_HANDLE)
    enter("w_stdin")
    A.movn(0, 9)                               # STD_INPUT_HANDLE = -10
    imp("GetStdHandle")
    leave()
    # ---- exit(code)
    enter("w_sys93")
    imp("ExitProcess")
    # ---- open(x0 = a UTF-8 path): MultiByteToWideChar into a fresh 128 KiB buffer, CreateFileW
    enter("w_open")
    A.mem("str", 0, SPR, 16)
    A.movz(0, 0)
    A.movi(1, 0x20000)
    A.movi(2, 0x3000)                          # MEM_COMMIT | MEM_RESERVE
    A.movz(3, 4)                               # PAGE_READWRITE
    imp("VirtualAlloc")
    A.cbz(0, "w_op_f")
    A.mem("str", 0, SPR, 24)
    A.movi(0, 65001)                           # CP_UTF8
    A.movz(1, 0)
    A.mem("ldr", 2, SPR, 16)
    A.movn(3, 0)                               # zero-ended
    A.mem("ldr", 4, SPR, 24)
    A.movi(5, 0x8000)
    imp("MultiByteToWideChar")
    A.cbz(0, "w_op_f", 0)
    A.mem("ldr", 0, SPR, 24)
    A.movi(1, 0x8000_0000)                     # GENERIC_READ
    A.movz(2, 1)                               # FILE_SHARE_READ
    A.movz(3, 0)
    A.movz(4, 3)                               # OPEN_EXISTING
    A.movz(5, 0x80)                            # FILE_ATTRIBUTE_NORMAL
    A.movz(6, 0)
    imp("CreateFileW")                         # INVALID_HANDLE_VALUE is -1 already
    A.b_("w_op_d")
    A.label("w_op_f")
    A.movn(0, 0)
    A.label("w_op_d")
    leave()
    # ---- size(handle): GetFileSizeEx
    enter("w_size")
    A.mem("str", ZR, SPR, 32)
    A.add_i(1, SPR, 32)
    imp("GetFileSizeEx")
    A.cbz(0, "w_sz_f", 0)
    A.mem("ldr", 0, SPR, 32)
    A.b_("w_sz_d")
    A.label("w_sz_f")
    A.movn(0, 0)
    A.label("w_sz_d")
    leave()
    # ---- alloc(x1 = size): VirtualAlloc anywhere, committed; 0 on failure
    enter("w_alloc")
    A.movz(0, 0)
    A.movi(2, 0x3000)
    A.movz(3, 4)
    imp("VirtualAlloc")
    leave()
    # ---- init: argc, argv, envp as Linux hands them over, in UTF-8. The environment block
    # (GetEnvironmentStringsW) and the command line (GetCommandLineW) are converted whole;
    # the command line is split by the Microsoft rules (arg0: to the next space or tab, or
    # quoted without escapes; then 2n backslashes + a quote = n backslashes and a toggle,
    # 2n+1 = n and a literal quote, "" inside quotes = a literal quote, still inside).
    # x0 = 1, or 0 if there is no memory. x19..x28 are free here: the guest has not started
    # and this never returns to Windows.
    enter("w_init")
    imp("GetEnvironmentStringsW")
    A.mov(19, 0)
    A.mov(9, 19)
    A.label("w_in_e1")
    A.mem("ldrh", 10, 9, 0)
    A.cbz(10, "w_in_e3")
    A.label("w_in_e2")
    A.mem("ldrh", 10, 9, 0)
    A.add_i(9, 9, 2)
    A.cbnz(10, "w_in_e2")
    A.b_("w_in_e1")
    A.label("w_in_e3")
    A.add_i(9, 9, 2)
    A.rrr("sub", 20, 9, 19)
    A.lsr_i(20, 20, 1)                         # n: units, the final zero included
    imp("GetCommandLineW")
    A.mov(21, 0)
    A.mov(9, 21)
    A.label("w_in_c1")
    A.mem("ldrh", 10, 9, 0)
    A.add_i(9, 9, 2)
    A.cbnz(10, "w_in_c1")
    A.rrr("sub", 22, 9, 21)
    A.lsr_i(22, 22, 1)                         # m: units, the zero included
    A.movz(9, 11)
    A.movz(10, 14)
    A.madd(1, 20, 9, ZR)
    A.madd(1, 22, 10, 1)
    A.add_i(1, 1, 72)
    A.add_i(1, 1, 0xFFF)
    A.and_l(1, 1, LOG_NOT4095)                 # 11n + 14m + 72, in pages
    A.movz(0, 0)
    A.movi(2, 0x3000)
    A.movz(3, 4)
    imp("VirtualAlloc")
    A.cbz(0, "w_in_d")
    A.mov(23, 0)
    A.movz(9, 3)
    A.madd(24, 20, 9, ZR)                      # 3n: the UTF-8 room
    A.movi(0, 65001)
    A.movz(1, 0)
    A.mov(2, 19)
    A.mov(3, 20)
    A.mov(4, 23)
    A.mov(5, 24)
    A.movz(6, 0)
    A.movz(7, 0)
    imp("WideCharToMultiByte")
    A.rrr("add", 24, 23, 24)
    A.add_i(24, 24, 7)
    A.and_l(24, 24, LOG_NOT7)                  # envp[]
    A.mov(9, 23)
    A.mov(10, 24)
    A.label("w_in_v1")
    A.mem("ldrb", 11, 9, 0)
    A.cbz(11, "w_in_v3")
    A.mem("str", 9, 10, 0)
    A.add_i(10, 10, 8)
    A.label("w_in_v2")
    A.mem("ldrb", 11, 9, 0)
    A.add_i(9, 9, 1)
    A.cbnz(11, "w_in_v2")
    A.b_("w_in_v1")
    A.label("w_in_v3")
    A.mem("str", ZR, 10, 0)
    A.add_i(25, 10, 8)                         # the command line, UTF-8
    A.adrl(9, "frame")
    A.mem("str", 24, 9, FR["envp"])
    A.movi(0, 65001)
    A.movz(1, 0)
    A.mov(2, 21)
    A.mov(3, 22)
    A.mov(4, 25)
    A.movz(9, 3)
    A.madd(5, 22, 9, ZR)
    A.movz(6, 0)
    A.movz(7, 0)
    imp("WideCharToMultiByte")
    A.movz(9, 3)
    A.madd(26, 22, 9, 25)                      # the split strings
    A.madd(27, 22, 9, 26)
    A.add_i(27, 27, 15)
    A.and_l(27, 27, LOG_NOT7)                  # argv[]
    A.adrl(9, "frame")
    A.mem("str", 27, 9, FR["argv"])
    A.movz(28, 0)                              # argc
    # x9 in, x10 out, x13 the argv cursor, x11 inside quotes, x12 backslashes, x14 the octet
    A.mov(9, 25)
    A.mov(10, 26)
    A.mov(13, 27)
    A.mem("str", 10, 13, 0)
    A.add_i(13, 13, 8)
    A.add_i(28, 28, 1)
    A.mem("ldrb", 14, 9, 0)
    A.cmp_i(14, 0x22)
    A.bcond(NE, "w_a0b")
    A.add_i(9, 9, 1)
    A.label("w_a0a")
    A.mem("ldrb", 14, 9, 0)
    A.cbz(14, "w_a0e")
    A.add_i(9, 9, 1)
    A.cmp_i(14, 0x22)
    A.bcond(EQ, "w_a0e")
    A.mem("strb", 14, 10, 0)
    A.add_i(10, 10, 1)
    A.b_("w_a0a")
    A.label("w_a0b")
    A.mem("ldrb", 14, 9, 0)
    A.cbz(14, "w_a0e")
    A.cmp_i(14, 0x20)
    A.bcond(EQ, "w_a0e")
    A.cmp_i(14, 0x09)
    A.bcond(EQ, "w_a0e")
    A.mem("strb", 14, 10, 0)
    A.add_i(10, 10, 1)
    A.add_i(9, 9, 1)
    A.b_("w_a0b")
    A.label("w_a0e")
    A.mem("strb", ZR, 10, 0)
    A.add_i(10, 10, 1)
    A.label("w_p0")                            # between arguments
    A.mem("ldrb", 14, 9, 0)
    A.cbz(14, "w_pd")
    A.cmp_i(14, 0x20)
    A.bcond(EQ, "w_p0s")
    A.cmp_i(14, 0x09)
    A.bcond(NE, "w_pa")
    A.label("w_p0s")
    A.add_i(9, 9, 1)
    A.b_("w_p0")
    A.label("w_pa")                            # an argument starts
    A.mem("str", 10, 13, 0)
    A.add_i(13, 13, 8)
    A.add_i(28, 28, 1)
    A.movz(11, 0)
    A.label("w_p1")
    A.movz(12, 0)
    A.label("w_p1b")
    A.mem("ldrb", 14, 9, 0)
    A.cmp_i(14, 0x5C)
    A.bcond(NE, "w_p1c")
    A.add_i(12, 12, 1)
    A.add_i(9, 9, 1)
    A.b_("w_p1b")
    A.label("w_p1c")
    A.cmp_i(14, 0x22)
    A.bcond(NE, "w_p1n")
    A.tst_l(12, LOG_1)                         # odd: n backslashes and a literal quote
    A.lsr_i(12, 12, 1)
    A.bl("w_bs")
    A.bcond(EQ, "w_p1e")
    A.mem("strb", 14, 10, 0)
    A.add_i(10, 10, 1)
    A.add_i(9, 9, 1)
    A.b_("w_p1")
    A.label("w_p1e")
    A.cbz(11, "w_p1t")
    A.mem("ldrb", 15, 9, 1)
    A.cmp_i(15, 0x22)
    A.bcond(NE, "w_p1t")
    A.mem("strb", 14, 10, 0)                   # "" inside quotes: one quote, still inside
    A.add_i(10, 10, 1)
    A.add_i(9, 9, 2)
    A.b_("w_p1")
    A.label("w_p1t")
    A.movz(15, 1)
    A.rrr("eor", 11, 11, 15)
    A.add_i(9, 9, 1)
    A.b_("w_p1")
    A.label("w_p1n")
    A.bl("w_bs")
    A.cbz(14, "w_pe")
    A.cbnz(11, "w_p1k")
    A.cmp_i(14, 0x20)
    A.bcond(EQ, "w_pe")
    A.cmp_i(14, 0x09)
    A.bcond(EQ, "w_pe")
    A.label("w_p1k")
    A.mem("strb", 14, 10, 0)
    A.add_i(10, 10, 1)
    A.add_i(9, 9, 1)
    A.b_("w_p1")
    A.label("w_pe")
    A.mem("strb", ZR, 10, 0)
    A.add_i(10, 10, 1)
    A.b_("w_p0")
    A.label("w_pd")
    A.mem("str", ZR, 13, 0)
    A.adrl(9, "frame")
    A.mem("str", 28, 9, FR["argc"])
    A.movz(0, 1)
    A.label("w_in_d")
    leave()
    # x12 backslashes to x10; leaves the flags alone; a leaf (w_init's own x30 is in wsave)
    A.label("w_bs")
    A.movz(15, 0x5C)
    A.label("w_bs1")
    A.cbz(12, "w_bs2")
    A.mem("strb", 15, 10, 0)
    A.add_i(10, 10, 1)
    A.add_i(12, 12, -1)
    A.b_("w_bs1")
    A.label("w_bs2")
    A.ret()


def win_split(cmd):
    """The command-line rule w_init implements, in Python (for tests): argv as a list"""
    out, i = [], 0
    a = ""
    if cmd[:1] == '"':
        i = 1
        while i < len(cmd) and cmd[i] != '"':
            a += cmd[i]
            i += 1
        i += 1
    else:
        while i < len(cmd) and cmd[i] not in " \t":
            a += cmd[i]
            i += 1
    out.append(a)
    while True:
        while i < len(cmd) and cmd[i] in " \t":
            i += 1
        if i >= len(cmd):
            return out
        a, inq = "", False
        while True:
            b = 0
            while i < len(cmd) and cmd[i] == "\\":
                b += 1
                i += 1
            c = cmd[i] if i < len(cmd) else ""
            if c == '"':
                a += "\\" * (b // 2)
                if b % 2:
                    a += '"'
                    i += 1
                elif inq and cmd[i + 1:i + 2] == '"':
                    a += '"'
                    i += 2
                else:
                    inq = not inq
                    i += 1
                continue
            a += "\\" * b
            if c == "" or (c in " \t" and not inq):
                break
            a += c
            i += 1
        out.append(a)


def win_join(args):
    """The exact inverse of win_split: the command line the spawn store hands CreateProcessW
    (spn_cmdline implements it). argv[0] follows the program-name rule (quoted, no escapes),
    so it cannot carry a '"': None for such a list (the store answers BadPath on every
    target). Every later argument is written bare when it is non-empty and holds no space,
    tab or quote; otherwise quoted, a run of n backslashes doubled when a quote or the
    closing quote follows it, and each quote escaped as \\"."""
    if not args or '"' in args[0]:
        return None
    a0 = args[0]
    out = '"' + a0 + '"' if a0 == "" or " " in a0 or "\t" in a0 else a0
    for a in args[1:]:
        out += " "
        if a and not any(c in a for c in ' \t"'):
            out += a
            continue
        out += '"'
        b = 0
        for c in a:
            if c == "\\":
                b += 1
                continue
            out += "\\" * (2 * b + 1 if c == '"' else b) + c
            b = 0
        out += "\\" * (2 * b) + '"'
    return out


def pe_imports(rva):
    """kernel32.dll's import tables placed at `rva`: the bytes, and each label's offset
    (iat_<name> for every slot the code calls through)"""
    k = len(WIN_IMPORTS)
    ilt = 40
    iat = ilt + 8 * (k + 1)
    names = iat + 8 * (k + 1)
    hn, offs = b"", []
    for n in WIN_IMPORTS:
        offs.append(names + len(hn))
        e = b"\0\0" + n.encode() + b"\0"
        hn += e + b"\0" * (len(e) % 2)
    dll = names + len(hn)
    thunks = b"".join(struct.pack("<Q", rva + o) for o in offs) + bytes(8)
    b = (struct.pack("<IIIII", rva + ilt, 0, 0, rva + dll, rva + iat) + bytes(20)
         + thunks + thunks + hn + b"KERNEL32.dll\0")
    labs = {"iat_" + n: iat + 8 * i for i, n in enumerate(WIN_IMPORTS)}
    labs["pe_iat"], labs["pe_iat_end"] = iat, iat + 8 * (k + 1)
    return b + bytes(-len(b) % 8), labs


def pe_a64(A, imp, data_vsize, iat, iat_size):
    """PE32+, IMAGE_FILE_MACHINE_ARM64, a console program: headers in the first page, then
    .text (code + table + image, R X) at RVA 0x1000, .data (the import tables, then the
    frame as zero fill, R W), .reloc (one empty block: the code is PC-relative, so a slide
    changes nothing, and DYNAMICBASE wants the directory). Nothing in it depends on the
    clock: TimeDateStamp and CheckSum are 0, so the output is a function of the input."""
    return pe(A, imp, data_vsize, WBASE, 0xAA64, iat, iat_size)


def pe(A, imp, data_vsize, ibase, machine, iat, iat_size, dynamic=True):
    """dynamic=False (x86_64-windows): a fixed-base image, its relocations stripped, no .reloc;
    the x86-64 code addresses its data absolutely, as on Linux"""
    text = bytearray(0x1000) + A.b
    trva_end = (len(text) + 0xFFF) & ~0xFFF
    text += bytes(trva_end - len(text))
    drva = trva_end
    draw = imp + bytes(-len(imp) % 0x1000)
    dvsize = (data_vsize + 0xFFF) & ~0xFFF
    rrva = drva + dvsize
    reloc = struct.pack("<IIHH", 0x1000, 12, 0, 0) if dynamic else b""
    rraw = reloc + bytes(-len(reloc) % 0x1000)
    image_size = rrva + len(rraw)
    labs = A.labels
    dirs = [(0, 0)] * 16
    dirs[1] = (drva, 40)                       # imports
    if dynamic:
        dirs[5] = (rrva, len(reloc))           # base relocations
    dirs[12] = (iat, iat_size)                 # the IAT
    opt = struct.pack("<HBBIIIIIQIIHHHHHHIIIIHHQQQQII", 0x20B, 14, 0, trva_end - 0x1000,
                      len(draw) + len(rraw), dvsize - len(draw), labs["rt_start"] - ibase, 0x1000,
                      ibase, 0x1000, 0x1000, 6, 2, 0, 0, 6, 2, 0, image_size, 0x1000, 0, 3,
                      0x8160 if dynamic else 0x8100, 0x100000, 0x1000, 0x100000, 0x1000, 0, 16)
    opt += b"".join(struct.pack("<II", *d) for d in dirs)
    assert len(opt) == 240, len(opt)

    def sect(name, vsize, rva, rawsize, rawptr, ch):
        return struct.pack("<8sIIIIIIHHI", name, vsize, rva, rawsize, rawptr, 0, 0, 0, 0, ch)
    sects = (sect(b".text", trva_end - 0x1000, 0x1000, trva_end - 0x1000, 0x1000, 0x60000020)
             + sect(b".data", dvsize, drva, len(draw), trva_end, 0xC0000040))
    if dynamic:
        sects += sect(b".reloc", len(reloc), rrva, len(rraw), trva_end + len(draw), 0x42000040)
    coff = struct.pack("<HHIIIHH", machine, 3 if dynamic else 2, 0, 0, 0, 240, 0x0022 if dynamic else 0x0023)
    hdr = b"MZ" + bytes(0x3A) + struct.pack("<I", 0x40) + b"PE\0\0" + coff + opt + sects
    text[0:len(hdr)] = hdr
    return bytes(text) + draw + rraw


# ---------------------------------------------------------------- the F / D runtime (A64)
FPB = 15                    # the engine's base register: x15 = sp + the F/D data offset
FP_A64_DATA = [("fcsr", 8), ("lrsp", 8), ("lrstk", 256), ("fsf", 8 * FSF_SLOTS), ("fsen", 8)]


def fp_a64_off(name):
    o = 0
    for n, sz in FP_A64_DATA:
        if n == name:
            return o
        o += sz
    raise KeyError(name)


FP_A64_SIZE = sum(sz for _, sz in FP_A64_DATA)


class SoftA64:
    """softfp.engine's backend for A64: each variable is a quadword at [x15 + fsf + 8 slot]; temps x16 x17 only (the
    guest's registers are never touched). A call pushes x30 on a software stack in the F/D data (x30 is a guest
    register, and the engine nests)."""
    CCS = {"eq": EQ, "ne": NE, "ltu": LO, "leu": LS, "gtu": HI, "geu": HS, "lts": LT, "les": LE, "gts": GT, "ges": GE}

    def __init__(self, A):
        self.A, self.slots, self.n = A, {}, 0

    def lab(self):
        self.n += 1
        return "sfl%d" % self.n

    def off(self, name):
        if name not in self.slots:
            assert len(self.slots) < FSF_SLOTS, name
            self.slots[name] = len(self.slots)
        return fp_a64_off("fsf") + 8 * self.slots[name]

    def ld(self, r, x):
        if isinstance(x, int):
            self.A.movi(r, x & M64)
        else:
            self.A.mem("ldr", r, FPB, self.off(x))

    def st(self, d, r):
        self.A.mem("str", r, FPB, self.off(d))

    def label(self, n):
        self.A.label(n)

    def jmp(self, n):
        self.A.b_(n)

    def call(self, n):
        A = self.A
        A.mem("ldr", T0, FPB, fp_a64_off("lrsp"))
        A.rrr("add", T1, FPB, T0)
        A.mem("str", 30, T1, fp_a64_off("lrstk"))
        A.add_i(T0, T0, 8)
        A.mem("str", T0, FPB, fp_a64_off("lrsp"))
        A.bl(n)
        A.mem("ldr", T0, FPB, fp_a64_off("lrsp"))
        A.add_i(T0, T0, -8)
        A.mem("str", T0, FPB, fp_a64_off("lrsp"))
        A.rrr("add", T1, FPB, T0)
        A.mem("ldr", 30, T1, fp_a64_off("lrstk"))

    def ret(self):
        self.A.ret()

    def set(self, d, imm):
        self.ld(T0, imm)
        self.st(d, T0)

    def mov(self, d, a):
        self.ld(T0, a)
        self.st(d, T0)

    def _rrr(self, op, d, a, b):
        self.ld(T0, a)
        self.ld(T1, b)
        self.A.rrr(op, T0, T0, T1)
        self.st(d, T0)

    def add(self, d, a, b):
        self._rrr("add", d, a, b)

    def sub(self, d, a, b):
        self._rrr("sub", d, a, b)

    def and_(self, d, a, b):
        self._rrr("and", d, a, b)

    def or_(self, d, a, b):
        self._rrr("orr", d, a, b)

    def xor(self, d, a, b):
        self._rrr("eor", d, a, b)

    def mul(self, d, a, b):
        self.ld(T0, a)
        self.ld(T1, b)
        self.A.madd(T0, T0, T1, ZR)
        self.st(d, T0)

    def umulh(self, d, a, b):
        self.ld(T0, a)
        self.ld(T1, b)
        self.A.umulh(T0, T0, T1)
        self.st(d, T0)

    def _shift(self, name, d, a, b):
        A = self.A
        self.ld(T0, a)
        if isinstance(b, int):
            if b & M64 >= 64:
                if name == "sar":
                    A.asr_i(T0, T0, 63)
                else:
                    A.movz(T0, 0)
            else:
                {"shl": A.lsl_i, "shr": A.lsr_i, "sar": A.asr_i}[name](T0, T0, b)
        else:
            assert name != "sar", "a variable sar count"
            self.ld(T1, b)
            A.dp2("lslv" if name == "shl" else "lsrv", T0, T0, T1)
            A.cmp_i(T1, 64)
            A.csel(T0, T0, ZR, LO)
        self.st(d, T0)

    def shl(self, d, a, b):
        self._shift("shl", d, a, b)

    def shr(self, d, a, b):
        self._shift("shr", d, a, b)

    def sar(self, d, a, b):
        self._shift("sar", d, a, b)

    def clz(self, d, a):
        self.ld(T0, a)
        self.A.clz(T0, T0)
        self.st(d, T0)

    def br(self, cond, a, b, lab):
        A = self.A
        self.ld(T0, a)
        if isinstance(b, int) and 0 <= b & M64 < 4096:
            A.cmp_i(T0, b & M64)
        else:
            self.ld(T1, b)
            A.cmp(T0, T1)
        A.bcond(self.CCS[cond], lab)



class PwX86:
    """files_engine's backend for x86-64: each variable is a quadword of `pwv`; rax rcx rdx are the
    temporaries (the window arm has saved rbx rsi rdi r8..r11, which the host calls clobber)"""
    CC = SoftX86.CC

    def __init__(self, A, osn):
        self.A, self.osn, self.slots, self.n = A, osn, {}, 0

    def lab(self):
        self.n += 1
        return "pwl%d" % self.n

    def slot(self, name):
        if name not in self.slots:
            assert len(self.slots) < PW_SLOTS, name
            self.slots[name] = len(self.slots)
        return ("abs", "pwv", 8 * self.slots[name])

    def ld(self, r, x):
        if isinstance(x, int):
            v = x & M64
            sv = v - (1 << 64) if v >> 63 else v
            if -(1 << 31) <= sv < (1 << 31):
                self.A.mov_rm_imm32(("r", r), sv)
            else:
                self.A.mov_r_imm64(r, v)
        else:
            self.A.mov_r_rm(r, self.slot(x))

    def st(self, d, r):
        self.A.mov_rm_r(self.slot(d), r)

    def label(self, n):
        self.A.label(n)

    def jmp(self, n):
        self.A.jmp(n)

    def call(self, n):
        self.A.call(n)

    def ret(self):
        self.A.ret()

    def set(self, d, imm):
        self.ld(RAX, imm)
        self.st(d, RAX)

    def mov(self, d, a):
        self.ld(RAX, a)
        self.st(d, RAX)

    def _alu(self, op, d, a, b):
        self.ld(RAX, a)
        self.ld(RCX, b)
        self.A.alu_r_rm(op, RAX, ("r", RCX))
        self.st(d, RAX)

    def add(self, d, a, b):
        self._alu("add", d, a, b)

    def sub(self, d, a, b):
        self._alu("sub", d, a, b)

    def and_(self, d, a, b):
        self._alu("and", d, a, b)

    def or_(self, d, a, b):
        self._alu("or", d, a, b)

    def xor(self, d, a, b):
        self._alu("xor", d, a, b)

    def shl(self, d, a, n):
        self.ld(RAX, a)
        self.A.shift_imm("shl", ("r", RAX), n)
        self.st(d, RAX)

    def shr(self, d, a, n):
        self.ld(RAX, a)
        self.A.shift_imm("shr", ("r", RAX), n)
        self.st(d, RAX)

    def mul(self, d, a, b):
        self.ld(RAX, a)
        self.ld(RCX, b)
        self.A.imul_r_rm(RAX, ("r", RCX))
        self.st(d, RAX)

    def udiv(self, d, a, b):
        self.ld(RAX, a)
        self.ld(RCX, b)
        self.A.alu_r_rm("xor", RDX, ("r", RDX), 0)
        self.A.f7(6, ("r", RCX))
        self.st(d, RAX)

    def br(self, cond, a, b, lab):
        self.ld(RAX, a)
        self.ld(RCX, b)
        self.A.alu_r_rm("cmp", RAX, ("r", RCX))
        self.A.jcc(self.CC[cond], lab)

    def ldm(self, d, p, off, w):
        A = self.A
        self.ld(RCX, p)
        if w == 1 or w == 2:
            A.movzx(RAX, ("m", RCX, off), w)
        elif w == 4:
            A.mov_r_rm(RAX, ("m", RCX, off), 0)
        else:
            A.mov_r_rm(RAX, ("m", RCX, off))
        self.st(d, RAX)

    def stm(self, p, v, off, w):
        self.ld(RCX, p)
        self.ld(RAX, v)
        self.A.mov_rm_r(("m", RCX, off), RAX, width=w)

    def buf(self, d, name):
        self.A.lea(RAX, ("abs", "pwb", PWB[name][0]))
        self.st(d, RAX)

    def lea(self, d, label):
        self.A.lea(RAX, ("abs", label, 0))
        self.st(d, RAX)

    def rt(self, d, name):
        if name == "delta":
            self.set(d, 0)
            return
        self.A.mov_r_rm(RAX, ("abs", name, 0))
        self.st(d, RAX)

    def g2h(self, d, a):
        self.mov(d, a)

    def sys(self, n):
        A = self.A
        for r, a in zip((RDI, RSI, RDX, R10, R8, R9), ("a0", "a1", "a2", "a3", "a4", "a5")):
            self.ld(r, a)
        if self.osn == "mac":
            A.mov_rm_imm32(("r", RAX), 0x2000000 + n)
            A.syscall()
            ok = self.lab()
            A.jcc("ae", ok)                          # carry clear
            A.op(b"\xf7", 3, ("r", RAX))             # neg rax: -errno, as Linux answers
            A.label(ok)
        else:
            A.mov_rm_imm32(("r", RAX), n)
            A.syscall()
        self.st("r", RAX)

    def win(self, name, n, ret32):
        A = self.A
        A.mov_rm_r(self.slot("rsp"), RSP)
        A.alu_rm_imm("and", ("r", RSP), -16)
        A.alu_rm_imm("sub", ("r", RSP), 96)
        A.op(b"\x0f\xae", 3, ("m", RSP, 88), 0)      # stmxcsr: Win64 calls may clear its sticky flags
        for i in range(4, n):
            self.ld(RAX, "a%d" % i)
            A.mov_rm_r(("m", RSP, 32 + 8 * (i - 4)), RAX)
        for r, i in zip((RCX, RDX, R8, R9), range(n)):
            self.ld(r, "a%d" % i)
        A.mov_r_rm(RAX, ("abs", "iat_" + name, 0))
        A.raw(0x40, 0xFF, 0xD0)                       # call rax
        A.op(b"\x0f\xae", 2, ("m", RSP, 88), 0)      # ldmxcsr
        A.mov_r_rm(RSP, self.slot("rsp"))
        if ret32:
            A.mov_r_rm(RAX, ("r", RAX), 0)            # a BOOL / DWORD answer: eax, zero-extended
        self.st("r", RAX)


class PwA64:
    """files_engine's backend for A64: each variable is a quadword at [sp + pwv + 8 slot]; temporaries
    x16 x17 (and x15 through A64.mem); a call pushes x30 on the software stack pwlr (x30 is a guest
    register). The window arm has saved x0..x14 and x30, which the host calls clobber."""
    CCS = SoftA64.CCS

    def __init__(self, A, FR, mac, adr):
        self.A, self.FR, self.mac, self.adr, self.slots, self.n = A, FR, mac, adr, {}, 0

    def lab(self):
        self.n += 1
        return "pwl%d" % self.n

    def off(self, name):
        if name not in self.slots:
            assert len(self.slots) < PW_SLOTS, name
            self.slots[name] = len(self.slots)
        return self.FR["pwv"] + 8 * self.slots[name]

    def ld(self, r, x):
        if isinstance(x, str):                         # a variable's slot
            self.A.mem("ldr", r, SPR, self.off(x))
        else:                                          # a value: an int, or rta2t1's run-time Sym
            self.A.movi(r, x & M64 if isinstance(x, int) else x)

    def st(self, d, r):
        self.A.mem("str", r, SPR, self.off(d))

    def label(self, n):
        self.A.label(n)

    def jmp(self, n):
        self.A.b_(n)

    def call(self, n):
        A = self.A
        A.mem("ldr", T0, SPR, self.off("lrsp"))
        A.add_i(T1, SPR, 0)
        A.rrr("add", T1, T1, T0)
        A.mem("str", 30, T1, self.FR["pwlr"])
        A.add_i(T0, T0, 8)
        A.mem("str", T0, SPR, self.off("lrsp"))
        A.bl(n)
        A.mem("ldr", T0, SPR, self.off("lrsp"))
        A.add_i(T0, T0, -8)
        A.mem("str", T0, SPR, self.off("lrsp"))
        A.add_i(T1, SPR, 0)
        A.rrr("add", T1, T1, T0)
        A.mem("ldr", 30, T1, self.FR["pwlr"])

    def ret(self):
        self.A.ret()

    def set(self, d, imm):
        self.ld(T0, imm)
        self.st(d, T0)

    def mov(self, d, a):
        self.ld(T0, a)
        self.st(d, T0)

    def _rrr(self, op, d, a, b):
        self.ld(T0, a)
        self.ld(T1, b)
        self.A.rrr(op, T0, T0, T1)
        self.st(d, T0)

    def add(self, d, a, b):
        self._rrr("add", d, a, b)

    def sub(self, d, a, b):
        self._rrr("sub", d, a, b)

    def and_(self, d, a, b):
        self._rrr("and", d, a, b)

    def or_(self, d, a, b):
        self._rrr("orr", d, a, b)

    def xor(self, d, a, b):
        self._rrr("eor", d, a, b)

    def shl(self, d, a, n):
        self.ld(T0, a)
        self.A.lsl_i(T0, T0, n)
        self.st(d, T0)

    def shr(self, d, a, n):
        self.ld(T0, a)
        self.A.lsr_i(T0, T0, n)
        self.st(d, T0)

    def mul(self, d, a, b):
        self.ld(T0, a)
        self.ld(T1, b)
        self.A.madd(T0, T0, T1, ZR)
        self.st(d, T0)

    def udiv(self, d, a, b):
        self.ld(T0, a)
        self.ld(T1, b)
        self.A.dp2("udiv", T0, T0, T1)
        self.st(d, T0)

    def br(self, cond, a, b, lab):
        A = self.A
        self.ld(T0, a)
        if isinstance(b, int) and 0 <= b & M64 < 4096:
            A.cmp_i(T0, b & M64)
        else:
            self.ld(T1, b)
            A.cmp(T0, T1)
        A.bcond(self.CCS[cond], lab)

    def ldm(self, d, p, off, w):
        self.ld(T1, p)
        self.A.mem({1: "ldrb", 2: "ldrh", 4: "ldrw", 8: "ldr"}[w], T0, T1, off)
        self.st(d, T0)

    def stm(self, p, v, off, w):
        self.ld(T1, p)
        self.ld(T0, v)
        self.A.mem({1: "strb", 2: "strh", 4: "strw", 8: "str"}[w], T0, T1, off)

    def buf(self, d, name):
        o = self.FR["pwb"] + PWB[name][0]
        self.A.add_i12(T0, SPR, o >> 12)
        self.A.add_i(T0, T0, o & 0xFFF)
        self.st(d, T0)

    def lea(self, d, label):
        self.adr(T0, label)
        self.st(d, T0)

    def rt(self, d, name):
        if name == "delta" and not self.mac:
            self.set(d, 0)
            return
        self.A.mem("ldr", T0, SPR, self.FR[name])
        self.st(d, T0)

    def g2h(self, d, a):
        self.ld(T0, a)
        if self.mac:
            self.A.mem("ldr", T1, SPR, self.FR["delta"])
            self.A.rrr("add", T0, T0, T1)
        self.st(d, T0)

    def sys(self, n):
        A = self.A
        for i in range(6):
            self.ld(i, "a%d" % i)
        if self.mac:
            A.movi(16, n)
            A.svc80()
            A.bskip(LO, 2)                           # carry clear: the answer
            A.rrr("sub", 0, ZR, 0)                   # carry set: x0 = errno, made -errno
        else:
            A.movi(8, n)
            A.svc()
        self.st("r", 0)

    def win(self, name, n, ret32):
        A = self.A
        A.mem("str", 30, SPR, self.off("lrw"))
        for i in range(n):
            self.ld(i, "a%d" % i)
        A.mem("ldr", 9, SPR, self.FR["os_sp"])
        A.sp_from(9)
        A.add_i(SPR, SPR, -64)
        A.adrl(16, "iat_" + name)
        A.mem("ldr", 16, 16, 0)
        A.blr(16)
        A.adrl(9, "frame")
        A.sp_from(9)
        if ret32:
            A.rrr("orr", 0, ZR, 0, sf=0)              # a BOOL / DWORD answer: w0, zero-extended
        self.st("r", 0)
        A.mem("ldr", 30, SPR, self.off("lrw"))


def logimm(ones, rot=0):
    """a 64-bit logical immediate: `ones` consecutive one bits, rotated left by rot (N:immr:imms)"""
    return 1 << 12 | ((64 - rot) % 64) << 6 | (ones - 1)


def runtime_fp_a64(A, frame_size, soft_all=False, mac=False):
    """The F / D helpers for aarch64 (the lowering calls `bl fph_<id>` with the f sources in d0 d1 d2 (raw 64 bits),
    the integer source in x16, the rm field in x17; the answer is d0 (a single NaN-boxed) or x16). Only x15 x16
    x17 and d0-d7 are touched (x30 is saved by the caller). The F/D data (fcsr, the engine's frame) sits at
    sp + frame_size. IEEE arithmetic is the FPU's under FPCR (FZ = 0, DN = 1: every NaN result is the canonical one,
    0x7FF8.. / 0x7FC00000) with the divergences fixed: NaN-boxing, the saturating conversions (NaN gives the
    maximum), fmin / fmax / fclass / sign injection as bit logic, and the cases the FPU decides differently from RISC-V
    (RMM; underflow detected BEFORE rounding: a result of exactly the smallest normal with UFC raised is redone by the
    software engine, softfp.py)."""
    n = [0]
    stubs = []
    SF = SoftA64(A)
    LOG_3, LOG_7, LOG_1F, LOG_1, LOG_8 = logimm(2), logimm(3), logimm(5), logimm(1), logimm(1, 3)

    def lab():
        n[0] += 1
        return "fpm%d" % n[0]

    def fpbase(r):
        A.add_i12(r, SPR, frame_size >> 12)
        A.add_i(r, r, frame_size & 0xFFF)

    def fmov_xd(x, d):                        # x <- d (64 bits)
        A.fcvt_int(1, 1, 0, 6, x, d)

    def fmov_dx(d, x):
        A.fcvt_int(1, 1, 0, 7, d, x)

    def fmov_ws(w, sv):
        A.fcvt_int(0, 0, 0, 6, w, sv)

    def fmov_sw(sv, w):
        A.fcvt_int(0, 0, 0, 7, sv, w)

    def rm_resolve():                         # x17 = the rm field -> the effective mode 0..4; reserved: illegal
        ok = lab()
        A.cmp_i(17, 7)
        A.bcond(NE, ok)
        fpbase(15)
        A.mem("ldr", 15, 15, 0)
        A.lsr_i(15, 15, 5)
        A.and_l(15, 15, LOG_7)
        A.mov(17, 15)
        A.label(ok)
        A.cmp_i(17, 4)
        A.bcond(HI, "fpx_bad")

    def set_fpcr():                           # x17 = 0..3: FPCR = DN | RMode(x17), FPSR cleared
        A.lsl_i(17, 17, 1)
        A.movz(15, 0x6C)
        A.dp2("lsrv", 15, 15, 17)
        A.and_l(15, 15, LOG_3)
        A.lsl_i(15, 15, 22)
        A.movz(17, 0x200, 1)
        A.rrr("orr", 15, 15, 17)
        A.msr_fpcr(15)
        A.msr_fpsr(ZR)

    def merge_flags(keep16):                  # fcsr |= fflags(FPSR)
        A.mrs_fpsr(15)
        A.and_l(15, 15, LOG_1F)
        A.rbit(15, 15)
        A.lsr_i(15, 15, 59)
        if keep16:
            fmov_dx(7, 16)
        fpbase(17)
        A.mem("ldr", 16, 17, 0)
        A.rrr("orr", 16, 16, 15)
        A.mem("str", 16, 17, 0)
        if keep16:
            fmov_xd(16, 7)

    def box0():
        fmov_ws(16, 0)
        A.movi(15, BOX_HI)
        A.rrr("orr", 16, 16, 15)
        fmov_dx(0, 16)

    def unbox(i):
        ok = lab()
        fmov_xd(16, i)
        A.lsr_i(15, 16, 32)
        A.cmn_i(15, 1, 0)
        A.bcond(EQ, ok)
        A.movz(16, 0x7FC0, 1)
        A.label(ok)
        fmov_sw(i, 16)

    def soft_branch(*kind):
        name = "fps_%d" % len(stubs)
        stubs.append((name, kind))
        if soft_all:
            A.b_(name)
        else:
            A.cmp_i(17, 4)
            A.bcond(EQ, name)
        return name

    def helper(name):
        A.label("fph_%d" % FP_ID[name])

    A.label("fpx_bad")
    A.b_("rt_bad_insn")
    # ---- sstatus.FS off: yantra's halt for an illegal instruction (no handler: Undelivered, cause 2)
    FRM = a64_frame(mac)
    A.label("rt_illegal")
    A.bl("rt_flush")
    A.bl("fs_m1")
    A.mem("ldr", 9, SPR, FRM["fault_off"])
    A.movi(10, 0x8000_0000)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    A.bl("fs_m2")
    A.movz(0, 1)
    A.b_("rt_exit")
    for nm_, txt in (("fs_m1", b"halt: Undelivered { pc: "), ("fs_m2", b", cause: 2 }\n")):
        A.label(nm_)
        (A.adrl if mac else A.movl)(1, nm_ + "_s")
        A.movi(2, len(txt))
        A.movz(0, 2)
        if mac == "win":
            A.mem("str", 30, SPR, FRM["lr_sys"])
            A.bl("w_sys64")
            A.mem("ldr", 30, SPR, FRM["lr_sys"])
        elif mac:
            A.movz(16, 4)
            A.svc80()
        else:
            A.movz(8, 64)
            A.svc()
        A.ret()
        A.label(nm_ + "_s")
        A.raw(txt + bytes(-len(txt) % 4))
    A.label("fpx_done_d")
    merge_flags(False)
    A.ret()
    A.label("fpx_done_s")
    merge_flags(False)
    box0()
    A.ret()
    A.label("fpx_box_s")
    box0()
    A.ret()

    def uf_check(sd, name, nin):
        """UFC raised and the result exactly the smallest normal: tininess before rounding differs from RISC-V's
        (after): redo it in software with the original operands (d4 d5 d6) and the mode (d7)"""
        no = lab()
        A.mrs_fpsr(15)
        A.and_l(15, 15, LOG_8)
        A.cbz(15, no)
        if sd == "d":
            fmov_xd(16, 0)
            A.lsl_i(16, 16, 1)
            A.movi(15, 0x0020000000000000)
            A.cmp(16, 15)
        else:
            fmov_ws(16, 0)
            A.lsl_i(16, 16, 1, 0)
            A.movi(15, 0x01000000)
            A.cmp(16, 15, 0)
        A.bcond(NE, no)
        for i in range(nin):
            A.fmov_v(1, i, 4 + i)
        fmov_xd(17, 7)
        A.b_(name)
        A.label(no)

    # ---- fadd fsub fmul fdiv
    for nm in ("fadd", "fsub", "fmul", "fdiv"):
        for sd in "sd":
            helper("%s.%s" % (nm, sd))
            if sd == "s":
                unbox(0)
                unbox(1)
            rm_resolve()
            st = soft_branch("bin", {"fadd": "sf_e_add", "fsub": "sf_e_sub", "fmul": "sf_e_mul", "fdiv": "sf_e_div"}[nm], sd)
            fmov_dx(7, 17)
            A.fmov_v(1, 4, 0)
            A.fmov_v(1, 5, 1)
            set_fpcr()
            A.fp2(nm, 1 if sd == "d" else 0, 0, 0, 1)
            uf_check(sd, st, 2)
            A.b_("fpx_done_" + sd)
    for sd in "sd":
        helper("fsqrt." + sd)
        if sd == "s":
            unbox(0)
        rm_resolve()
        soft_branch("sqrt", sd)
        set_fpcr()
        A.fsqrt(1 if sd == "d" else 0, 0, 0)
        A.b_("fpx_done_" + sd)
    helper("fcvt.s.d")
    rm_resolve()
    st = soft_branch("cvtds")
    fmov_dx(7, 17)
    A.fmov_v(1, 4, 0)
    set_fpcr()
    A.fcvt_ff(0, 0, 0)
    uf_check("s", st, 1)
    A.b_("fpx_done_s")
    helper("fcvt.d.s")
    unbox(0)
    rm_resolve()
    ok = lab()
    A.cmp_i(17, 4)
    A.bcond(NE, ok)
    A.movz(17, 0)
    A.label(ok)
    set_fpcr()
    A.fcvt_ff(1, 0, 0)
    A.b_("fpx_done_d")
    # ---- the fused multiply-adds: RV fmadd = FMADD, fmsub = FNMSUB, fnmsub = FMSUB, fnmadd = FNMADD
    for nm, a64 in (("fmadd", "fmadd"), ("fmsub", "fnmsub"), ("fnmsub", "fmsub"), ("fnmadd", "fnmadd")):
        for sd in "sd":
            helper("%s.%s" % (nm, sd))
            if sd == "s":
                unbox(0)
                unbox(1)
                unbox(2)
            rm_resolve()
            st = soft_branch("fma", sd, 1 if nm in ("fnmsub", "fnmadd") else 0, 1 if nm in ("fmsub", "fnmadd") else 0)
            fmov_dx(7, 17)
            A.fmov_v(1, 4, 0)
            A.fmov_v(1, 5, 1)
            A.fmov_v(1, 6, 2)
            set_fpcr()
            A.fp3(a64, 1 if sd == "d" else 0, 0, 0, 1, 2)
            uf_check(sd, st, 3)
            A.b_("fpx_done_" + sd)
    # ---- sign injection: res = a ^ ((a ^ b [^ S]) & S)
    for nm in ("fsgnj", "fsgnjn", "fsgnjx"):
        for sd in "sd":
            helper("%s.%s" % (nm, sd))
            if sd == "d":
                fmov_xd(16, 0)
                fmov_xd(15, 1)
                A.movi(17, 1 << 63)
            else:
                fmov_ws(16, 0)
                fmov_ws(15, 1)
                A.movi(17, 1 << 31)
            if nm == "fsgnjx":
                A.rrr("and", 15, 15, 17)
            else:
                A.rrr("eor", 15, 15, 16)
                if nm == "fsgnjn":
                    A.rrr("eor", 15, 15, 17)
                A.rrr("and", 15, 15, 17)
            A.rrr("eor", 16, 16, 15)
            if sd == "d":
                fmov_dx(0, 16)
                A.ret()
            else:
                fmov_sw(0, 16)
                A.b_("fpx_box_s")
    # ---- fmin / fmax
    for nm, is_max in (("fmin", 0), ("fmax", 1)):
        for sd in "sd":
            helper("%s.%s" % (nm, sd))
            if sd == "s":
                unbox(0)
                unbox(1)
            S = 1 if sd == "d" else 0
            A.movz(15, 0x200, 1)
            A.msr_fpcr(15)
            A.msr_fpsr(ZR)
            A.fcmp(S, 0, 1)                           # IOC only for a signalling NaN
            unordered, equal, aless, a_nan, b_only = lab(), lab(), lab(), lab(), lab()
            fin = "fpx_fin_" + sd
            A.bcond(VS, unordered)
            A.bcond(EQ, equal)
            A.bcond(MI, aless)
            if not is_max:                                # a > b: min is b
                A.fmov_v(S, 0, 1)
            A.b_(fin)
            A.label(aless)
            if is_max:
                A.fmov_v(S, 0, 1)
            A.b_(fin)
            A.label(equal)                                # equal: only +0 / -0 differ in bits; min is the negative
            fmov_xd(16, 0)
            fmov_xd(15, 1)
            A.rrr("and" if is_max else "orr", 16, 16, 15)
            fmov_dx(0, 16)
            A.b_(fin)
            A.label(unordered)
            A.fcmp(S, 0, 0)
            A.bcond(VS, a_nan)
            A.b_(fin)                                     # only b is a NaN: the answer is a
            A.label(a_nan)
            A.fcmp(S, 1, 1)
            A.bcond(VC, b_only)
            if sd == "d":
                A.movi(16, CAN_D)
                fmov_dx(0, 16)
            else:
                A.movz(16, 0x7FC0, 1)
                fmov_sw(0, 16)
            A.b_(fin)
            A.label(b_only)
            A.fmov_v(S, 0, 1)
            A.b_(fin)
    A.label("fpx_fin_d")
    A.b_("fpx_done_d")
    A.label("fpx_fin_s")
    A.b_("fpx_done_s")
    # ---- comparisons: feq (quiet), flt fle (signalling)
    for nm in ("feq", "flt", "fle"):
        for sd in "sd":
            helper("%s.%s" % (nm, sd))
            if sd == "s":
                unbox(0)
                unbox(1)
            A.movz(15, 0x200, 1)
            A.msr_fpcr(15)
            A.msr_fpsr(ZR)
            A.fcmp(1 if sd == "d" else 0, 0, 1, 0 if nm == "feq" else 1)
            A.cset(16, {"feq": EQ, "flt": MI, "fle": LS}[nm])
            merge_flags(True)
            A.ret()
    # ---- fclass, fmv.x.*, fmv.*.x
    for sd in "sd":
        helper("fclass." + sd)
        if sd == "s":
            unbox(0)
            fmov_ws(16, 0)
            A.lsr_i(15, 16, 31, 0)
            A.rrr("add", 16, 16, 16, 0)                    # drop the sign: x << 1 then >> 1
            A.lsr_i(16, 16, 1, 0)
            expm, quiet, hidden, sf = 0x7F800000, 1 << 22, 1 << 23, 0
        else:
            fmov_xd(16, 0)
            A.lsr_i(15, 16, 63)
            A.lsl_i(16, 16, 1)
            A.lsr_i(16, 16, 1)
            expm, quiet, hidden, sf = 0x7FF0000000000000, 1 << 51, 1 << 52, 1
        isnan, isinf, subz = lab(), lab(), lab()
        A.movi(17, expm)
        A.cmp(16, 17, sf)
        A.bcond(HI, isnan)
        A.bcond(EQ, isinf)
        A.movi(17, hidden)
        A.cmp(16, 17, sf)
        A.bcond(LO, subz)

        def pick(pos, neg):
            nn = lab()
            A.movz(16, 1 << pos)
            A.cbz(15, nn, 0)
            A.movz(16, 1 << neg)
            A.label(nn)
            A.ret()
        pick(6, 1)
        A.label(subz)
        zero = lab()
        A.cbz(16, zero, sf)
        pick(5, 2)
        A.label(zero)
        pick(4, 3)
        A.label(isinf)
        pick(7, 0)
        A.label(isnan)
        A.movi(17, quiet)
        A.rrr("and", 16, 16, 17, sf)
        qn = lab()
        A.cbnz(16, qn, sf)
        A.movz(16, 1 << 8)
        A.ret()
        A.label(qn)
        A.movz(16, 1 << 9)
        A.ret()
    helper("fmv.x.w")
    fmov_ws(16, 0)
    A.sxtw(16, 16)
    A.ret()
    helper("fmv.x.d")
    fmov_xd(16, 0)
    A.ret()
    helper("fmv.w.x")
    fmov_sw(0, 16)
    A.b_("fpx_box_s")
    helper("fmv.d.x")
    fmov_dx(0, 16)
    A.ret()
    # ---- integer -> float
    for sd in "sd":
        for t, tn in enumerate(("w", "wu", "l", "lu")):
            helper("fcvt.%s.%s" % (sd, tn))
            rm_resolve()
            soft_branch("fromint", sd, tn)
            set_fpcr()
            A.fcvt_int(1 if t >= 2 else 0, 1 if sd == "d" else 0, 0, 2 if tn in ("w", "l") else 3, 0, 16)
            A.b_("fpx_done_" + sd)
    # ---- float -> integer: the instruction picks the mode (n z m p a); NaN gives the maximum
    for sd in "sd":
        for t, tn in enumerate(("w", "wu", "l", "lu")):
            helper("fcvt.%s.%s" % (tn, sd))
            if sd == "s":
                unbox(0)
            rm_resolve()
            A.msr_fpsr(ZR)
            tail = lab()
            for mode, (rmode, opc) in enumerate(((0, 0), (3, 0), (2, 0), (1, 0), (0, 4))):
                nxt = lab()
                if mode < 4:
                    A.cmp_i(17, mode)
                    A.bcond(NE, nxt)
                A.fcvt_int(1 if t >= 2 else 0, 1 if sd == "d" else 0, rmode, opc + (1 if tn in ("wu", "lu") else 0), 16, 0)
                A.b_(tail)
                A.label(nxt)
            A.label(tail)
            A.fcmp(1 if sd == "d" else 0, 0, 0)
            ordered, done = lab(), lab()
            A.bcond(VC, ordered)
            A.movi(16, (0x7FFFFFFF, M64, 0x7FFFFFFFFFFFFFFF, M64)[t])
            A.b_(done)
            A.label(ordered)
            if t < 2:
                A.sxtw(16, 16)
            A.label(done)
            merge_flags(True)
            A.ret()
    # ---- the float CSRs: x16 = the source, x17 = csr | kind << 4; x16 = old
    A.label("fph_csr")
    fmov_dx(4, 17)
    fmov_dx(5, 16)
    fpbase(15)
    A.mem("ldr", 16, 15, 0)
    fmov_dx(3, 16)
    fmov_xd(17, 4)
    A.and_l(17, 17, logimm(4))
    c2, c3, st_ = lab(), lab(), lab()
    A.cmp_i(17, 1)
    A.bcond(NE, c2)
    A.movz(15, 0)
    A.movz(16, 0x1F)
    A.b_(st_)
    A.label(c2)
    A.cmp_i(17, 2)
    A.bcond(NE, c3)
    A.movz(15, 5)
    A.movz(16, 7)
    A.b_(st_)
    A.label(c3)
    A.movz(15, 0)
    A.movz(16, 0xFF)
    A.label(st_)
    fmov_dx(6, 15)
    fmov_dx(7, 16)
    fmov_xd(15, 3)
    fmov_xd(16, 6)
    A.dp2("lsrv", 15, 15, 16)
    fmov_xd(16, 7)
    A.rrr("and", 15, 15, 16)
    fmov_dx(3, 15)                                     # old
    fmov_xd(17, 4)
    A.lsr_i(17, 17, 4)
    fmov_xd(16, 5)
    knew, k3 = lab(), lab()
    A.cmp_i(17, 1)
    A.bcond(EQ, knew)
    fmov_xd(15, 3)
    A.cmp_i(17, 2)
    A.bcond(NE, k3)
    A.rrr("orr", 16, 16, 15)
    A.b_(knew)
    A.label(k3)
    A.w(0x8A200000 | 16 << 16 | 15 << 5 | 16)           # bic x16, x15, x16: old & ~source
    A.label(knew)
    fmov_xd(15, 7)
    A.rrr("and", 16, 16, 15)
    fmov_xd(15, 6)
    A.dp2("lslv", 16, 16, 15)
    fmov_xd(17, 7)
    A.dp2("lslv", 17, 17, 15)
    fpbase(15)
    fmov_dx(5, 16)
    A.mem("ldr", 16, 15, 0)
    A.w(0x8A200000 | 17 << 16 | 16 << 5 | 16)           # bic x16, x16, x17
    fmov_xd(17, 5)
    A.rrr("orr", 16, 16, 17)
    A.mem("str", 16, 15, 0)
    fmov_xd(16, 3)
    A.ret()
    # ---- the RMM paths (and the underflow corner): the software engine
    def sv(name):
        return SF.off(name)

    def put(name, r):
        A.mem("str", r, FPB, sv(name))

    for name, kind in stubs:
        A.label(name)
        k = kind[0]
        fpbase(FPB)
        put("rp_rm", 17)
        if k == "bin":
            fn, sd = kind[1], kind[2]
            fmov_xd(16, 0)
            put("sf_a", 16)
            fmov_xd(16, 1)
            put("sf_b", 16)
        elif k == "sqrt":
            fn, sd = "sf_e_sqrt", kind[1]
            fmov_xd(16, 0)
            put("sf_a", 16)
        elif k == "cvtds":
            fn, sd = "sf_e_cvt_ds", "s"
            fmov_xd(16, 0)
            put("sf_a", 16)
        elif k == "fma":
            fn, sd = "sf_e_fma", kind[1]
            for i, v in enumerate(("sf_a", "sf_b", "sf_c")):
                fmov_xd(16, i)
                put(v, 16)
            A.movz(16, kind[2])
            put("ng_p", 16)
            A.movz(16, kind[3])
            put("ng_a", 16)
        else:                                           # fromint: x16 = the integer
            fn, sd, tn = "sf_e_fromint", kind[1], kind[2]
            if tn == "w":
                A.sxtw(16, 16)
            elif tn == "wu":
                A.rrr("orr", 16, ZR, 16, 0)
            if tn in ("w", "l"):
                A.asr_i(17, 16, 63)
                A.rrr("eor", 16, 16, 17)
                A.rrr("sub", 16, 16, 17)
                A.and_l(17, 17, LOG_1)
                put("fi_neg", 17)
            else:
                put("fi_neg", ZR)
            put("fi_mag", 16)
        put("fl", ZR)
        if k != "cvtds":
            SF.call("sf_fmt_" + sd)
        SF.call(fn)
        A.b_("fpx_stail_" + sd)
    for sd in "ds":
        A.label("fpx_stail_" + sd)
        A.mem("ldr", 16, FPB, sv("fl"))
        A.mem("ldr", 17, FPB, fp_a64_off("fcsr"))
        A.rrr("orr", 17, 17, 16)
        A.mem("str", 17, FPB, fp_a64_off("fcsr"))
        A.mem("ldr", 16, FPB, sv("sf_res"))
        fmov_dx(0, 16)
        if sd == "d":
            A.ret()
        else:
            A.b_("fpx_box_s")
    softfp.engine(SF)

def elf_a64(A, data_end):
    """ELF64, AArch64 (EM_AARCH64 = 183), ET_EXEC: PT_LOAD text (headers + code + table +
    image, R X) at TEXT, PT_LOAD frame (bss only, R W) after it; 64 KiB alignment, so any
    kernel page size (4, 16, 64 KiB) maps it."""
    text = bytearray(0x1000) + A.b
    tsize = len(text)
    dstart = (TEXT + tsize + 0xFFFF) & ~0xFFFF
    hdr = struct.pack("<4sBBBBB7xHHIQQQIHHHHHH", b"\x7fELF", 2, 1, 1, 0, 0, 2, 183, 1,
                      A.labels["rt_start"], 64, 0, 0, 64, 56, 2, 64, 0, 0)
    ph1 = struct.pack("<IIQQQQQQ", 1, 5, 0, TEXT, TEXT, tsize, tsize, 0x10000)
    ph2 = struct.pack("<IIQQQQQQ", 1, 6, 0, dstart, dstart, 0, data_end - dstart, 0x10000)
    text[0:len(hdr)] = hdr
    text[64:64 + 56] = ph1
    text[120:120 + 56] = ph2
    return bytes(text)


def macho_a64(A, frame_size):
    return macho(A, frame_size, TBASE_MAC, 0x4000, 0x0100000C, 0, 0x200005, 0x1_0000_0000, b"anuvada-aarch64m")


def macho_x86(A, frame_size):
    """x86-64 Darwin keeps the old rules: no PIE and a one-page __PAGEZERO, so the image sits
    at TEXT like the Linux one (guest RAM at its own address, abs32 data) on 4 KiB pages"""
    return macho(A, frame_size, TEXT, 0x1000, 0x01000007, 3, 0x5, 0x1000, b"anuvada-x86-64-m")


def macho(A, frame_size, tbase, page, cpu, cpusub, flags, pagezero, uuid):
    """Mach-O 64, MH_EXECUTE | NOUNDEFS | DYLDLINK (| PIE on arm64): __PAGEZERO,
    __TEXT at tbase (headers + code + table + image, R X, one __text section),
    __DATA (the frame, zero fill), __LINKEDIT (the signature only); LC_LOAD_DYLINKER,
    LC_UUID (a constant: the output stays a function of the input), LC_MAIN, an empty
    LC_SYMTAB + LC_DYSYMTAB (macOS 15's dyld needs them), and LC_CODE_SIGNATURE: an ad hoc SuperBlob holding one CodeDirectory (v 0x20400, SHA-256
    over 4 KiB pages of everything before it, exec segment = __TEXT, main binary)."""
    text = bytearray(0x1000) + A.b
    tsize = (len(text) + page - 1) & -page
    text += bytes(tsize - len(text))
    data_vm = tbase + tsize
    data_size = (frame_size + page - 1) & -page
    le_vm = data_vm + data_size
    npages = tsize // 4096
    ident = b"anuvada\0"
    cd_len = 88 + len(ident) + 32 * npages
    sb_len = 20 + cd_len
    sigsize = (sb_len + 15) & ~15

    def seg(name, vmaddr, vmsize, fileoff, filesize, prot, sects=b"", nsects=0):
        return struct.pack("<II16sQQQQIIII", 0x19, 72 + 80 * nsects, name, vmaddr, vmsize, fileoff,
                           filesize, prot, prot, nsects, 0) + sects
    sect = struct.pack("<16s16sQQIIIIIIII", b"__text", b"__TEXT", tbase + 0x1000, len(A.b), 0x1000, 2,
                       0, 0, 0x80000400, 0, 0, 0)
    cmds = (seg(b"__PAGEZERO", 0, pagezero, 0, 0, 0)
            + seg(b"__TEXT", tbase, tsize, 0, tsize, 5, sect, 1)
            + seg(b"__DATA", data_vm, data_size, tsize, 0, 3)
            + seg(b"__LINKEDIT", le_vm, (sigsize + page - 1) & -page, tsize, sigsize, 1)
            + struct.pack("<III", 0xE, 32, 12) + b"/usr/lib/dyld" + bytes(7)
            + struct.pack("<II", 0x1B, 24) + uuid
            + struct.pack("<IIQQ", 0x80000028, 24, A.labels["rt_start"] - tbase, 0)
            # an empty LC_SYMTAB and an all-zero LC_DYSYMTAB: macOS 15's dyld walks classic
            # relocations through LC_DYSYMTAB and faults without it (an Apple M1 host, 15.4.1);
            # LC_DYSYMTAB without LC_SYMTAB is refused by name
            + struct.pack("<IIIIII", 0x2, 24, tsize, 0, tsize, 0)
            + struct.pack("<II18I", 0xB, 80, *([0] * 18))
            + struct.pack("<IIII", 0x1D, 16, tsize, sigsize))
    hdr = struct.pack("<IIIIIIII", 0xFEEDFACF, cpu, cpusub, 2, 10, len(cmds), flags, 0)
    text[0:len(hdr) + len(cmds)] = hdr + cmds
    cd = struct.pack(">IIIIIIIIIBBBBIIIIQQQQ", 0xFADE0C02, cd_len, 0x20400, 0x2, 88 + len(ident), 88, 0, npages,
                     tsize, 32, 2, 0, 12, 0, 0, 0, 0, 0, 0, tsize, 1) + ident
    for i in range(npages):
        cd += hashlib.sha256(bytes(text[i * 4096:(i + 1) * 4096])).digest()
    sb = struct.pack(">IIIII", 0xFADE0CC0, sb_len, 1, 0, 20) + cd
    return bytes(text) + sb + bytes(sigsize - len(sb))


# ================================================================ wasm32 target
# The fifth target (owner order: retire Rust, run Sassembly in the browser). Output: a .wasm
# module built by our own encoder. DESIGN:
#  * wasm 1.0 + mutable globals (exported) + the i64<->BigInt JS API; no sign-ext, no bulk memory,
#    no threads, no GC. 5-octet padded LEB128 sizes (valid, and they make back-patching trivial).
#  * Guest RAM is linear memory at RAMOFF + (guest_addr - base). Below RAMOFF: the UART buffer, f0..f31,
#    the jump table (block index per code word), the block -> segment table.
#  * RV x1..x31 are i64 locals of a SEGMENT function (a run of blocks, ~SEGW words), spilled to
#    globals when control leaves it. A segment is one `loop` whose body is a br_table dispatch on the
#    block index; the trampoline `go` calls the segment of g_blk through a funcref table.
#  * Blocks are cut as in the FAST/SAFE design minus the sp window (every access is guarded):
#    leaders = entry, direct targets, the word after a branch/jal/jalr/undecodable word, and every statically
#    materialised code address (auipc/lui+addi pairs, 8-octet data words).
#    jalr maps pc -> block through the table; a non-leader target is refused by name (kind 11).
#  * Step budget (plain): each block opens with `slice -= n` (n = its words). A slice is at most SLICE_I
#    instructions; the trampoline refills it from the rest of the budget and polls deadline() (the run is
#    resumable); when the rest is empty a short slice is the StepLimit, block-granular (the pc of the block). A
#    halt refunds the instructions it did not begin, so the retired count (budget - left()) is exact at every
#    halt. ATTEST mode (--target wasm32-attest): an exact per-instruction budget (StepLimit at the pc of the
#    instruction that would be the budget+1th), one tick per block for the deadline poll, and yantra's store
#    high-water mark (max RAM offset + width of a RAM store) in global `hw`.
#  * The halts are yantra's/the oracle's: h_kind/h_pc/h_addr/h_val are exported; the host prints.
# STYLE. Everything from here to wasm_translate() is written in a RESTRICTED subset of Python, so that
# t1/authoring/py2t1.py can port it to anuvada.t1 mechanically (the .t1 is generated, never edited):
# integer variables only; module-level state declared with an int (or [] for an array); `global` in
# every function that assigns module state; if/elif/else, while, for-in-range, return; one comparison
# per test (and/or/not between tests); no closures, tuples, dicts or strings except the literals in
# E.o("op") / E.blk("block") / E.str("name"). E is the byte writer (class WB below).
WOPC = {
    "unreachable": 0x00, "nop": 0x01, "return": 0x0F, "drop": 0x1A, "select": 0x1B, "end": 0x0B, "else": 0x05,
    "i32.eqz": 0x45, "i32.eq": 0x46, "i32.ne": 0x47, "i32.lt_s": 0x48, "i32.lt_u": 0x49, "i32.ge_s": 0x4E, "i32.ge_u": 0x4F,
    "i64.eqz": 0x50, "i64.eq": 0x51, "i64.ne": 0x52, "i64.lt_s": 0x53, "i64.lt_u": 0x54, "i64.gt_u": 0x56,
    "i64.ge_s": 0x59, "i64.ge_u": 0x5A,
    "i32.add": 0x6A, "i32.sub": 0x6B, "i32.mul": 0x6C, "i32.div_s": 0x6D, "i32.div_u": 0x6E, "i32.rem_s": 0x6F,
    "i32.rem_u": 0x70, "i32.and": 0x71, "i32.or": 0x72, "i32.xor": 0x73, "i32.shl": 0x74, "i32.shr_s": 0x75, "i32.shr_u": 0x76,
    "i64.add": 0x7C, "i64.sub": 0x7D, "i64.mul": 0x7E, "i64.div_s": 0x7F, "i64.div_u": 0x80, "i64.rem_s": 0x81,
    "i64.rem_u": 0x82, "i64.and": 0x83, "i64.or": 0x84, "i64.xor": 0x85, "i64.shl": 0x86, "i64.shr_s": 0x87, "i64.shr_u": 0x88,
    "i32.wrap_i64": 0xA7, "i64.extend_i32_s": 0xAC, "i64.extend_i32_u": 0xAD, "memory.size": 0x3F,
}
WLOAD = {"i32.load": (0x28, 2), "i64.load": (0x29, 3), "i64.load8_s": (0x30, 0), "i64.load8_u": (0x31, 0),
         "i64.load16_s": (0x32, 1), "i64.load16_u": (0x33, 1), "i64.load32_s": (0x34, 2), "i64.load32_u": (0x35, 2),
         "i32.load16_u": (0x2F, 1), "i32.store": (0x36, 2), "i64.store": (0x37, 3), "i32.store8": (0x3A, 0),
         "i64.store8": (0x3C, 0), "i64.store16": (0x3D, 1), "i64.store32": (0x3E, 2)}
WBLK = {"block": 0x02, "loop": 0x03, "if": 0x04}
WVOID, WI32, WI64 = 0x40, 0x7F, 0x7E
# opcodes chosen at run time by the lowering
I64_EQ, I64_NE, I64_LT_S, I64_LT_U, I64_GE_S, I64_GE_U = 0x51, 0x52, 0x53, 0x54, 0x59, 0x5A
I64_ADD, I64_SUB, I64_MUL, I64_AND, I64_OR, I64_XOR, I64_SHL, I64_SHR_S, I64_SHR_U = 0x7C, 0x7D, 0x7E, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88
I32_ADD, I32_SUB, I32_MUL, I32_SHL, I32_SHR_S, I32_SHR_U = 0x6A, 0x6B, 0x6C, 0x74, 0x75, 0x76
SEGW = 4096              # words per segment function (cut at the first block end past it)
SLICE = 65536            # attest: blocks between two deadline() polls
SLICE_I = 1048576        # plain: instructions between two deadline() polls (and the final slice ends at the budget)
# globals: x1..x31 = 0..30
G_TOT, G_TICKS, G_BLK = 31, 32, 33
G_LL, G_LS = 34, 38            # lim_l1,2,4,8 / lim_s1,2,4,8
G_OUTLEN, G_KIND, G_PC, G_ADDR, G_VAL, G_HW = 42, 43, 44, 45, 46, 47
G_RAMOFF, G_BASE, G_IMGLEN, G_RAMDEF, G_ATTEST = 48, 49, 50, 51, 52
G_FINAL = 53                    # plain mode: 1 when no budget is left beyond the current slice
# function indices
F_WRITE, F_FINISH, F_DEADLINE, F_FLUSH, F_HALT, F_STSLOW, F_MULHU = 0, 1, 2, 3, 4, 5, 6
F_DIV, F_REM, F_DIVU, F_REMU, F_DIVW, F_REMW, F_DIVUW, F_REMUW = 7, 8, 9, 10, 11, 12, 13, 14
F_GO, F_RUN, F_SETUP, F_LEFT, F_SEG0 = 15, 16, 17, 18, 19
# types
T_WRITE, T_FIN, T_I32, T_RUN, T_SETUP, T_I64, T_BIN, T_HALT, T_VOID = range(9)
LOC_TOT, LOC_TICKS, LOC_EA, LOC_OFF, LOC_V, LOC_T, LOC_BLK, LOC_RET, LOC_K = 31, 32, 33, 34, 35, 36, 37, 38, 39
WL_FREGS = 65536
WL_JT = 65536 + 256
WBASE = 0x8000_0000
# the decoder's operation numbers (decode.t1a: the same list)
K_LUI, K_AUIPC, K_JAL, K_JALR, K_BEQ, K_BNE, K_BLT, K_BGE, K_BLTU, K_BGEU = 1, 2, 3, 4, 5, 6, 7, 8, 9, 10
K_LB, K_LH, K_LW, K_LD, K_LBU, K_LHU, K_LWU, K_FLD = 11, 12, 13, 14, 15, 16, 17, 18
K_SB, K_SH, K_SW, K_SD, K_FSD = 19, 20, 21, 22, 23
K_ADDI, K_SLTI, K_SLTIU, K_XORI, K_ORI, K_ANDI, K_SLLI, K_SRLI, K_SRAI = 24, 25, 26, 27, 28, 29, 30, 31, 32
K_ADDIW, K_SLLIW, K_SRLIW, K_SRAIW = 33, 34, 35, 36
K_ADD, K_SLL, K_SLT, K_SLTU, K_XOR, K_SRL, K_OR, K_AND, K_SUB, K_SRA = 37, 38, 39, 40, 41, 42, 43, 44, 45, 46
K_MUL, K_MULH, K_MULHSU, K_MULHU, K_DIV, K_DIVU, K_REM, K_REMU = 47, 48, 49, 50, 51, 52, 53, 54
K_ADDW, K_SLLW, K_SRLW, K_SUBW, K_SRAW, K_MULW, K_DIVW, K_DIVUW, K_REMW, K_REMUW = 55, 56, 57, 58, 59, 60, 61, 62, 63, 64
K_FENCE, K_CSRS = 65, 66
KNAMES = ["", "lui", "auipc", "jal", "jalr", "beq", "bne", "blt", "bge", "bltu", "bgeu", "lb", "lh", "lw", "ld", "lbu", "lhu",
          "lwu", "fld", "sb", "sh", "sw", "sd", "fsd", "addi", "slti", "sltiu", "xori", "ori", "andi", "slli", "srli", "srai",
          "addiw", "slliw", "srliw", "sraiw", "add", "sll", "slt", "sltu", "xor", "srl", "or", "and", "sub", "sra", "mul",
          "mulh", "mulhsu", "mulhu", "div", "divu", "rem", "remu", "addw", "sllw", "srlw", "subw", "sraw", "mulw", "divw",
          "divuw", "remw", "remuw", "fence", "csrs_sstatus"]


class WB:
    """the byte writer E: every emitter of the wasm encoder. py2t1.py ports each method to a .t1 routine."""

    def __init__(self):
        self.b = bytearray()

    def here(self):
        return len(self.b)

    def byte(self, v):
        self.b.append(v & 255)

    def u(self, v):
        while True:
            c = v & 0x7F
            v >>= 7
            if v:
                self.b.append(c | 0x80)
            else:
                self.b.append(c)
                return

    def s(self, v):
        v = sx(v, 64)
        while True:
            c = v & 0x7F
            v >>= 7
            if (v == 0 and not c & 0x40) or (v == -1 and c & 0x40):
                self.b.append(c)
                return
            self.b.append(c | 0x80)

    def pad5(self, v):
        for i in range(4):
            self.b.append((v >> (7 * i)) & 0x7F | 0x80)
        self.b.append((v >> 28) & 0x7F)

    def patch5(self, pos, v):
        for i in range(4):
            self.b[pos + i] = (v >> (7 * i)) & 0x7F | 0x80
        self.b[pos + 4] = (v >> 28) & 0x7F

    def o(self, name):
        self.b.append(WOPC[name])
        if name == "memory.size":
            self.b.append(0)

    def mem(self, name, off):
        op, al = WLOAD[name]
        self.b.append(op)
        self.u(al)
        self.u(off)

    def i32c(self, v):
        self.b.append(0x41)
        self.s(v)

    def i64c(self, v):
        self.b.append(0x42)
        self.s(v)

    def idx(self, op, i):
        self.b.append(op)
        self.u(i)

    def lget(self, i): self.idx(0x20, i)
    def lset(self, i): self.idx(0x21, i)
    def ltee(self, i): self.idx(0x22, i)
    def gget(self, i): self.idx(0x23, i)
    def gset(self, i): self.idx(0x24, i)
    def call(self, i): self.idx(0x10, i)
    def br(self, d): self.idx(0x0C, d)
    def br_if(self, d): self.idx(0x0D, d)

    def call_indirect(self, ty):
        self.b.append(0x11)
        self.u(ty)
        self.u(0)

    def blk(self, kind, t=WVOID):
        self.b.append(WBLK[kind])
        self.b.append(t)

    def str(self, s):
        self.u(len(s))
        self.b += s.encode()


E = WB()
# ---- the translation's state (module level; the .t1 keeps the same names as globals)
WORDS, OPS, RD, RS1, RS2, IMM, IMG = [], [], [], [], [], [], []   # the decoded image (anuvada.t1: decode.t1a's arrays)
NWORDS = 0
CODE_SIZE = 0
PROT_SIZE = 0
ENTRY = 0
IMGLEN = 0
EXTENT = 0
ATTEST = 0
LEADER, BLKOF, STARTS, BLEN, SEGA, SEGB, SEGOF = [], [], [], [], [], [], []
NBLK = 0
NSEG = 0
RAMOFF = 0
SEGOFAT = 0
CTXCNT = 0
CTXNEST = 0


def grow(a, n):
    """(Python only; py2t1.py ports a call of it as the T1 store that sizes a run)"""
    a[:] = [0] * n


def begin_body():
    pos = E.here()
    E.pad5(0)
    return pos


def end_body(pos):
    E.patch5(pos, E.here() - pos - 5)


# ---------------------------------------------------------------- the fixed functions
def h_flush():
    E.u(0)
    E.gget(G_OUTLEN)
    E.blk("if")
    E.i32c(1)
    E.i32c(0)
    E.gget(G_OUTLEN)
    E.call(F_WRITE)
    E.i32c(0)
    E.gset(G_OUTLEN)
    E.o("end")
    E.o("end")


def h_halt():
    E.u(0)
    E.lget(0)
    E.gset(G_KIND)
    E.lget(1)
    E.gset(G_PC)
    E.lget(2)
    E.gset(G_ADDR)
    E.lget(3)
    E.gset(G_VAL)
    E.call(F_FLUSH)
    E.o("end")


def h_stslow():
    E.u(0)
    E.lget(0)
    E.i64c(0x1000_0000)
    E.o("i64.eq")
    E.blk("if")
    E.gget(G_OUTLEN)
    E.lget(1)
    E.o("i32.wrap_i64")
    E.mem("i32.store8", 0)
    E.gget(G_OUTLEN)
    E.i32c(1)
    E.o("i32.add")
    E.gset(G_OUTLEN)
    E.gget(G_OUTLEN)
    E.i32c(65536)
    E.o("i32.ge_u")
    E.blk("if")
    E.call(F_FLUSH)
    E.o("end")
    E.i32c(0)
    E.o("return")
    E.o("end")
    E.lget(0)
    E.i64c(0x0010_0000)
    E.o("i64.eq")
    E.blk("if")
    E.call(F_FLUSH)
    E.lget(1)
    E.o("i32.wrap_i64")
    E.lget(1)
    E.i64c(32)
    E.o("i64.shr_u")
    E.o("i32.wrap_i64")
    E.call(F_FINISH)
    E.i32c(1)
    E.o("return")
    E.o("end")
    E.lget(0)
    E.i64c(1 << 31)
    E.o("i64.lt_u")
    E.blk("if")
    E.i32c(4)
    E.o("return")
    E.o("end")
    E.lget(0)
    E.i64c(WBASE + PROT_SIZE)
    E.o("i64.lt_u")
    E.blk("if")
    E.i32c(6)
    E.o("return")
    E.o("end")
    E.i32c(5)
    E.o("end")


def h_mulhu_split(dst, src, hi):
    E.lget(src)
    if hi:
        E.i64c(32)
        E.o("i64.shr_u")
    else:
        E.i64c(0xFFFFFFFF)
        E.o("i64.and")
    E.lset(dst)


def h_mulhu():
    E.u(1)
    E.u(7)
    E.byte(WI64)
    h_mulhu_split(2, 0, 0)
    h_mulhu_split(3, 0, 1)
    h_mulhu_split(4, 1, 0)
    h_mulhu_split(5, 1, 1)
    E.lget(2)
    E.lget(4)
    E.o("i64.mul")
    E.i64c(32)
    E.o("i64.shr_u")
    E.lset(6)
    E.lget(3)
    E.lget(4)
    E.o("i64.mul")
    E.lget(6)
    E.o("i64.add")
    E.lset(6)
    E.lget(6)
    E.i64c(0xFFFFFFFF)
    E.o("i64.and")
    E.lset(7)
    E.lget(6)
    E.i64c(32)
    E.o("i64.shr_u")
    E.lset(8)
    E.lget(2)
    E.lget(5)
    E.o("i64.mul")
    E.lget(7)
    E.o("i64.add")
    E.i64c(32)
    E.o("i64.shr_u")
    E.lset(6)
    E.lget(3)
    E.lget(5)
    E.o("i64.mul")
    E.lget(8)
    E.o("i64.add")
    E.lget(6)
    E.o("i64.add")
    E.o("end")


def hd_arg(n, wide):
    E.lget(n)
    if wide == 0:
        E.o("i32.wrap_i64")


def hd_ext(wide):
    if wide == 0:
        E.o("i64.extend_i32_s")


def hd_const(v, wide):
    if wide:
        E.i64c(v)
    else:
        E.i32c(v)


def h_divop(wide, signed, rem):
    E.u(0)
    hd_arg(1, wide)
    if wide:
        E.o("i64.eqz")
    else:
        E.o("i32.eqz")
    E.blk("if", WI64)
    if rem:
        hd_arg(0, wide)
        hd_ext(wide)
    else:
        E.i64c(-1)
    E.o("else")
    if signed:
        hd_arg(1, wide)
        hd_const(-1, wide)
        if wide:
            E.o("i64.eq")
        else:
            E.o("i32.eq")
        E.blk("if", WI64)
        if rem:
            E.i64c(0)
        else:
            hd_const(0, wide)
            hd_arg(0, wide)
            if wide:
                E.o("i64.sub")
            else:
                E.o("i32.sub")
            hd_ext(wide)
        E.o("else")
    hd_arg(0, wide)
    hd_arg(1, wide)
    op = 0x6D
    if wide:
        op = 0x7F
    if rem:
        op = op + 2
    if signed == 0:
        op = op + 1
    E.byte(op)
    hd_ext(wide)
    if signed:
        E.o("end")
    E.o("end")
    E.o("end")


def h_go():
    E.u(2)
    E.u(1)
    E.byte(WI32)
    E.u(1)
    E.byte(WI64)
    E.blk("loop")
    E.gget(G_BLK)
    E.i32c(1)
    E.o("i32.shl")
    E.mem("i32.load16_u", SEGOFAT)
    E.call_indirect(T_I32)
    E.lset(0)
    E.lget(0)
    E.o("i32.eqz")
    E.br_if(0)
    E.lget(0)
    E.i32c(1)
    E.o("i32.eq")
    E.blk("if")
    E.gget(G_KIND)
    E.o("return")
    E.o("end")
    if ATTEST:
        E.i64c(SLICE)
        E.gset(G_TICKS)
    else:
        h_take(1, G_TOT)
        E.gget(G_TICKS)
        E.lget(1)
        E.o("i64.add")
        E.gset(G_TICKS)
        E.gget(G_TOT)
        E.lget(1)
        E.o("i64.sub")
        E.gset(G_TOT)
        E.gget(G_TOT)
        E.o("i64.eqz")
        E.gset(G_FINAL)
    E.call(F_DEADLINE)
    E.blk("if")
    E.call(F_FLUSH)
    E.i32c(10)
    E.gset(G_KIND)
    E.i32c(10)
    E.o("return")
    E.o("end")
    E.br(0)
    E.o("end")
    E.i32c(0)
    E.o("end")


def h_take(loc, src):
    """local loc = min(global src, SLICE_I)"""
    E.gget(src)
    E.i64c(SLICE_I)
    E.gget(src)
    E.i64c(SLICE_I)
    E.o("i64.lt_u")
    E.o("select")
    E.lset(loc)


def h_run():
    if ATTEST:
        E.u(0)
        E.lget(0)
        E.gset(G_TOT)
        E.i64c(SLICE)
        E.gset(G_TICKS)
    else:
        E.u(1)
        E.u(1)
        E.byte(WI64)
        E.lget(0)
        E.gset(G_TOT)
        h_take(1, G_TOT)
        E.lget(1)
        E.gset(G_TICKS)
        E.gget(G_TOT)
        E.lget(1)
        E.o("i64.sub")
        E.gset(G_TOT)
        E.gget(G_TOT)
        E.o("i64.eqz")
        E.gset(G_FINAL)
    E.call(F_GO)
    E.o("end")


def h_setup_lim(j, wd):
    E.lget(1)
    E.i64c(wd - 1)
    E.o("i64.sub")
    E.gset(G_LL + j)
    E.lget(0)
    E.i64c(PROT_SIZE + wd - 1)
    E.o("i64.sub")
    E.gset(G_LS + j)


def h_setup():
    E.u(0)
    E.i64c(RAMOFF)
    E.lget(1)
    E.o("i64.add")
    E.o("memory.size")
    E.o("i64.extend_i32_u")
    E.i64c(65536)
    E.o("i64.mul")
    E.o("i64.gt_u")
    E.blk("if")
    E.i32c(1)
    E.o("return")
    E.o("end")
    h_setup_lim(0, 1)
    h_setup_lim(1, 2)
    h_setup_lim(2, 4)
    h_setup_lim(3, 8)
    E.i32c(0)
    E.o("end")


def h_left():
    E.u(0)
    E.gget(G_TOT)
    if ATTEST == 0:
        E.gget(G_TICKS)
        E.o("i64.add")
    E.o("end")


def h_type(i):
    t = T_BIN
    if i == F_FLUSH:
        t = T_VOID
    if i == F_HALT:
        t = T_HALT
    if i == F_STSLOW:
        t = T_SETUP
    if i == F_GO:
        t = T_I32
    if i == F_RUN:
        t = T_RUN
    if i == F_SETUP:
        t = T_SETUP
    if i == F_LEFT:
        t = T_I64
    return t


def h_emit(i):
    if i == F_FLUSH:
        h_flush()
    if i == F_HALT:
        h_halt()
    if i == F_STSLOW:
        h_stslow()
    if i == F_MULHU:
        h_mulhu()
    if i == F_DIV:
        h_divop(1, 1, 0)
    if i == F_REM:
        h_divop(1, 1, 1)
    if i == F_DIVU:
        h_divop(1, 0, 0)
    if i == F_REMU:
        h_divop(1, 0, 1)
    if i == F_DIVW:
        h_divop(0, 1, 0)
    if i == F_REMW:
        h_divop(0, 1, 1)
    if i == F_DIVUW:
        h_divop(0, 0, 0)
    if i == F_REMUW:
        h_divop(0, 0, 1)
    if i == F_GO:
        h_go()
    if i == F_RUN:
        h_run()
    if i == F_SETUP:
        h_setup()
    if i == F_LEFT:
        h_left()


# ---------------------------------------------------------------- blocks and segments
def wasm_plan():
    """leaders, blocks, segments and the low-memory layout"""
    global NBLK, NSEG, RAMOFF, SEGOFAT
    for k in range(NWORDS):
        LEADER[k] = 0
        BLKOF[k] = 0
    LEADER[(ENTRY - WBASE) // 4] = 1
    for k in range(NWORDS):
        op = OPS[k]
        if op == 0 or op == K_JAL or op == K_JALR or (op >= K_BEQ and op <= K_BGEU):
            if k + 1 < NWORDS:
                LEADER[k + 1] = 1
        if op == K_JAL or (op >= K_BEQ and op <= K_BGEU):
            t = k + IMM[k] // 4
            if t >= 0 and t < NWORDS and IMM[k] % 4 == 0:
                LEADER[t] = 1
    # every statically materialised code address begins a block (a routine reached only through a pointer):
    # auipc+addi, auipc+jalr and lui+addi pairs whose value lands in the code, and 8-octet data words
    # (address tables) that are 4-aligned code addresses
    for k in range(NWORDS - 1):
        op = OPS[k]
        if op == K_AUIPC or op == K_LUI:
            if RD[k] != 0 and RS1[k + 1] == RD[k] and (OPS[k + 1] == K_ADDI or (op == K_AUIPC and OPS[k + 1] == K_JALR)):
                v = IMM[k] + IMM[k + 1]
                if op == K_AUIPC:
                    v = v + WBASE + 4 * k
                v = v & 0xFFFFFFFF
                wt = (v - WBASE) // 4
                if v >= WBASE and wt < NWORDS and v % 4 == 0:
                    LEADER[wt] = 1
    o = CODE_SIZE
    while o + 8 <= IMGLEN:
        hi = IMG[o + 4] | (IMG[o + 5] << 8) | (IMG[o + 6] << 16) | (IMG[o + 7] << 24)
        if hi == 0:
            v = IMG[o] | (IMG[o + 1] << 8) | (IMG[o + 2] << 16) | (IMG[o + 3] << 24)
            wt = (v - WBASE) // 4
            if v >= WBASE and wt < NWORDS and v % 4 == 0:
                LEADER[wt] = 1
        o += 4
    NBLK = 0
    for k in range(NWORDS):
        if LEADER[k]:
            STARTS[NBLK] = k
            BLKOF[k] = NBLK
            NBLK += 1
    for j in range(NBLK):
        nxt = NWORDS
        if j + 1 < NBLK:
            nxt = STARTS[j + 1]
        BLEN[j] = nxt - STARTS[j]
    NSEG = 0
    i = 0
    while i < NBLK:
        j = i
        acc = 0
        while j < NBLK and acc < SEGW:
            acc += BLEN[j]
            j += 1
        SEGA[NSEG] = i
        SEGB[NSEG] = j
        for q in range(i, j):
            SEGOF[q] = NSEG
        NSEG += 1
        i = j
    SEGOFAT = (WL_JT + 4 * NWORDS + 15) // 16 * 16
    RAMOFF = (SEGOFAT + 2 * NBLK + 0xFFFF) // 0x10000 * 0x10000


def rget(n):
    if n == 0:
        E.i64c(0)
    else:
        E.lget(n - 1)


def rput(n):
    if n:
        E.lset(n - 1)
    else:
        E.o("drop")


def fault(kind, pc, aloc, aval, vloc, vval, refund):
    """halt(kind, pc, addr, val), refund the unstarted tail, leave the block. kind < 0: the kind is in LOC_K"""
    if kind < 0:
        E.lget(LOC_K)
    else:
        E.i32c(kind)
    E.i64c(pc)
    if aloc:
        E.lget(aval)
    else:
        E.i64c(aval)
    if vloc:
        E.lget(vval)
    else:
        E.i64c(vval)
    E.call(F_HALT)
    if refund and ATTEST == 0:
        E.lget(LOC_TICKS)
        E.i64c(refund)
        E.o("i64.add")
        E.lset(LOC_TICKS)
    E.br(CTXCNT + CTXNEST)


def goto_blk(bi):
    E.i32c(bi)
    E.lset(LOC_BLK)
    E.br(CTXCNT + CTXNEST + 2)


def direct_target(pc, imm):
    """the block of a direct branch/jal target, or -1 when it is outside the code or misaligned"""
    t = pc + imm
    r = -1
    if t >= WBASE and t < WBASE + 4 * NWORDS and t % 4 == 0:
        r = BLKOF[(t - WBASE) // 4]
    return r


def lower_ctl(k, pc, refund):
    """lui auipc jal jalr and the branches"""
    global CTXNEST
    op = OPS[k]
    rd = RD[k]
    rs1 = RS1[k]
    rs2 = RS2[k]
    imm = IMM[k]
    if op == K_LUI:
        if rd:
            E.i64c(imm)
            rput(rd)
    if op == K_AUIPC:
        if rd:
            E.i64c(pc + imm)
            rput(rd)
    if op == K_JAL:
        if rd:
            E.i64c(pc + 4)
            rput(rd)
        tb = direct_target(pc, imm)
        if tb >= 0:
            goto_blk(tb)
        else:
            fault(8, pc, 0, pc + imm, 0, 0, refund)
    if op == K_JALR:
        rget(rs1)
        E.i64c(imm)
        E.o("i64.add")
        E.i64c(-2)
        E.o("i64.and")
        E.lset(LOC_T)
        if rd:
            E.i64c(pc + 4)
            rput(rd)
        E.lget(LOC_T)
        E.i64c(WBASE)
        E.o("i64.sub")
        E.ltee(LOC_OFF)
        E.i64c(CODE_SIZE)
        E.o("i64.ge_u")
        E.lget(LOC_OFF)
        E.o("i32.wrap_i64")
        E.i32c(3)
        E.o("i32.and")
        E.o("i32.or")
        E.blk("if")
        CTXNEST = 1
        fault(8, pc, 1, LOC_T, 0, 0, refund)
        CTXNEST = 0
        E.o("end")
        E.lget(LOC_OFF)
        E.o("i32.wrap_i64")
        E.mem("i32.load", WL_JT)
        E.ltee(LOC_BLK)
        E.i32c(0)
        E.o("i32.lt_s")
        E.blk("if")
        CTXNEST = 1
        fault(11, pc, 1, LOC_T, 0, 0, refund)
        CTXNEST = 0
        E.o("end")
        E.br(CTXCNT + CTXNEST + 2)
    if op >= K_BEQ and op <= K_BGEU:
        rget(rs1)
        rget(rs2)
        c = I64_EQ
        if op == K_BNE:
            c = I64_NE
        if op == K_BLT:
            c = I64_LT_S
        if op == K_BGE:
            c = I64_GE_S
        if op == K_BLTU:
            c = I64_LT_U
        if op == K_BGEU:
            c = I64_GE_U
        E.byte(c)
        E.blk("if")
        CTXNEST = 1
        tb = direct_target(pc, imm)
        if tb >= 0:
            goto_blk(tb)
        else:
            fault(8, pc, 0, pc + imm, 0, 0, refund)
        CTXNEST = 0
        E.o("end")


def lim_index(wd):
    r = 0
    if wd == 2:
        r = 1
    if wd == 4:
        r = 2
    if wd == 8:
        r = 3
    return r


def lower_load(k, pc, refund):
    global CTXNEST
    op = OPS[k]
    rd = RD[k]
    wd = 8
    ld = 0x29
    if op == K_LB:
        wd = 1
        ld = 0x30
    if op == K_LBU:
        wd = 1
        ld = 0x31
    if op == K_LH:
        wd = 2
        ld = 0x32
    if op == K_LHU:
        wd = 2
        ld = 0x33
    if op == K_LW:
        wd = 4
        ld = 0x34
    if op == K_LWU:
        wd = 4
        ld = 0x35
    rget(RS1[k])
    E.i64c(IMM[k])
    E.o("i64.add")
    E.lset(LOC_EA)
    E.lget(LOC_EA)
    E.i64c(WBASE)
    E.o("i64.sub")
    E.ltee(LOC_OFF)
    E.gget(G_LL + lim_index(wd))
    E.o("i64.ge_u")
    E.blk("if")
    CTXNEST = 1
    E.i32c(2)
    E.i32c(3)
    E.lget(LOC_EA)
    E.i64c(1 << 31)
    E.o("i64.lt_u")
    E.o("select")
    E.lset(LOC_K)
    fault(-1, pc, 1, LOC_EA, 0, 0, refund)
    CTXNEST = 0
    E.o("end")
    if op == K_FLD:
        E.i32c(WL_FREGS + 8 * rd)
        E.lget(LOC_OFF)
        E.o("i32.wrap_i64")
        E.byte(ld)
        E.u(3)
        E.u(RAMOFF)
        E.mem("i64.store", 0)
    else:
        E.lget(LOC_OFF)
        E.o("i32.wrap_i64")
        E.byte(ld)
        E.u(wd_align(wd))
        E.u(RAMOFF)
        rput(rd)


def wd_align(wd):
    r = 0
    if wd == 2:
        r = 1
    if wd == 4:
        r = 2
    if wd == 8:
        r = 3
    return r


def lower_store(k, pc, refund):
    global CTXNEST
    op = OPS[k]
    wd = 8
    st = 0x37
    if op == K_SB:
        wd = 1
        st = 0x3C
    if op == K_SH:
        wd = 2
        st = 0x3D
    if op == K_SW:
        wd = 4
        st = 0x3E
    rget(RS1[k])
    E.i64c(IMM[k])
    E.o("i64.add")
    E.lset(LOC_EA)
    if op == K_FSD:
        E.i32c(0)
        E.mem("i64.load", WL_FREGS + 8 * RS2[k])
    else:
        rget(RS2[k])
    E.lset(LOC_V)
    E.lget(LOC_EA)
    E.i64c(WBASE + PROT_SIZE)
    E.o("i64.sub")
    E.gget(G_LS + lim_index(wd))
    E.o("i64.ge_u")
    E.blk("if")
    CTXNEST = 1
    E.lget(LOC_EA)
    E.lget(LOC_V)
    E.call(F_STSLOW)
    E.ltee(LOC_K)
    E.blk("if")
    CTXNEST = 2
    fault(-1, pc, 1, LOC_EA, 1, LOC_V, refund)
    CTXNEST = 1
    E.o("end")
    E.o("else")
    E.lget(LOC_EA)
    E.i64c(WBASE)
    E.o("i64.sub")
    E.o("i32.wrap_i64")
    E.lget(LOC_V)
    E.byte(st)
    E.u(wd_align(wd))
    E.u(RAMOFF)
    if ATTEST:
        E.lget(LOC_EA)
        E.i64c(WBASE - wd)
        E.o("i64.sub")
        E.ltee(LOC_T)
        E.gget(G_HW)
        E.o("i64.gt_u")
        E.blk("if")
        E.lget(LOC_T)
        E.gset(G_HW)
        E.o("end")
    CTXNEST = 0
    E.o("end")


def lower_alu(k, pc):
    op = OPS[k]
    rd = RD[k]
    rs1 = RS1[k]
    rs2 = RS2[k]
    imm = IMM[k]
    if rd == 0:
        return 0
    if op == K_ADDI or op == K_XORI or op == K_ORI or op == K_ANDI:
        if rs1 == 0 and op == K_ADDI:
            E.i64c(imm)
        else:
            rget(rs1)
            E.i64c(imm)
            c = I64_ADD
            if op == K_XORI:
                c = I64_XOR
            if op == K_ORI:
                c = I64_OR
            if op == K_ANDI:
                c = I64_AND
            E.byte(c)
        rput(rd)
    if op == K_SLTI or op == K_SLTIU:
        rget(rs1)
        E.i64c(imm)
        if op == K_SLTI:
            E.o("i64.lt_s")
        else:
            E.o("i64.lt_u")
        E.o("i64.extend_i32_u")
        rput(rd)
    if op == K_SLLI or op == K_SRLI or op == K_SRAI:
        rget(rs1)
        E.i64c(imm)
        c = I64_SHL
        if op == K_SRLI:
            c = I64_SHR_U
        if op == K_SRAI:
            c = I64_SHR_S
        E.byte(c)
        rput(rd)
    if op == K_ADDIW:
        rget(rs1)
        E.o("i32.wrap_i64")
        E.i32c(imm)
        E.o("i32.add")
        E.o("i64.extend_i32_s")
        rput(rd)
    if op == K_SLLIW or op == K_SRLIW or op == K_SRAIW:
        rget(rs1)
        E.o("i32.wrap_i64")
        E.i32c(imm)
        c = I32_SHL
        if op == K_SRLIW:
            c = I32_SHR_U
        if op == K_SRAIW:
            c = I32_SHR_S
        E.byte(c)
        E.o("i64.extend_i32_s")
        rput(rd)
    if op == K_ADD or op == K_SUB or op == K_AND or op == K_OR or op == K_XOR or op == K_MUL or op == K_SLL or op == K_SRL or op == K_SRA:
        rget(rs1)
        rget(rs2)
        c = I64_ADD
        if op == K_SUB:
            c = I64_SUB
        if op == K_AND:
            c = I64_AND
        if op == K_OR:
            c = I64_OR
        if op == K_XOR:
            c = I64_XOR
        if op == K_MUL:
            c = I64_MUL
        if op == K_SLL:
            c = I64_SHL
        if op == K_SRL:
            c = I64_SHR_U
        if op == K_SRA:
            c = I64_SHR_S
        E.byte(c)
        rput(rd)
    if op == K_SLT or op == K_SLTU:
        rget(rs1)
        rget(rs2)
        if op == K_SLT:
            E.o("i64.lt_s")
        else:
            E.o("i64.lt_u")
        E.o("i64.extend_i32_u")
        rput(rd)
    if op == K_ADDW or op == K_SUBW or op == K_MULW or op == K_SLLW or op == K_SRLW or op == K_SRAW:
        rget(rs1)
        E.o("i32.wrap_i64")
        rget(rs2)
        E.o("i32.wrap_i64")
        c = I32_ADD
        if op == K_SUBW:
            c = I32_SUB
        if op == K_MULW:
            c = I32_MUL
        if op == K_SLLW:
            c = I32_SHL
        if op == K_SRLW:
            c = I32_SHR_U
        if op == K_SRAW:
            c = I32_SHR_S
        E.byte(c)
        E.o("i64.extend_i32_s")
        rput(rd)
    if op == K_MULHU:
        rget(rs1)
        rget(rs2)
        E.call(F_MULHU)
        rput(rd)
    if op == K_MULH or op == K_MULHSU:
        rget(rs1)
        rget(rs2)
        E.call(F_MULHU)
        rget(rs1)
        E.i64c(63)
        E.o("i64.shr_s")
        rget(rs2)
        E.o("i64.and")
        E.o("i64.sub")
        if op == K_MULH:
            rget(rs2)
            E.i64c(63)
            E.o("i64.shr_s")
            rget(rs1)
            E.o("i64.and")
            E.o("i64.sub")
        rput(rd)
    if op == K_DIV or op == K_REM or op == K_DIVU or op == K_REMU or op == K_DIVW or op == K_REMW or op == K_DIVUW or op == K_REMUW:
        rget(rs1)
        rget(rs2)
        f = F_DIV
        if op == K_REM:
            f = F_REM
        if op == K_DIVU:
            f = F_DIVU
        if op == K_REMU:
            f = F_REMU
        if op == K_DIVW:
            f = F_DIVW
        if op == K_REMW:
            f = F_REMW
        if op == K_DIVUW:
            f = F_DIVUW
        if op == K_REMUW:
            f = F_REMUW
        E.call(f)
        rput(rd)
    return 0


def lower_one(k, pc, refund):
    op = OPS[k]
    if op == 0:
        fault(7, pc, 0, 0, 0, WORDS[k], refund)
    if op == K_LUI or op == K_AUIPC or op == K_JAL or op == K_JALR or (op >= K_BEQ and op <= K_BGEU):
        lower_ctl(k, pc, refund)
    if (op >= K_LB and op <= K_LWU) or op == K_FLD:
        lower_load(k, pc, refund)
    if (op >= K_SB and op <= K_SD) or op == K_FSD:
        lower_store(k, pc, refund)
    if op >= K_ADDI and op <= K_REMUW:
        lower_alu(k, pc)


def seg_func(si):
    """one segment: load the state, loop { br_table on the block index }, spill and return"""
    global CTXCNT, CTXNEST
    a = SEGA[si]
    b = SEGB[si]
    n = b - a
    pos = begin_body()
    E.u(2)
    E.u(37)
    E.byte(WI64)
    E.u(3)
    E.byte(WI32)
    for r in range(1, 32):
        E.gget(r - 1)
        E.lset(r - 1)
    if ATTEST:
        E.gget(G_TOT)
        E.lset(LOC_TOT)
    E.gget(G_TICKS)
    E.lset(LOC_TICKS)
    E.gget(G_BLK)
    E.lset(LOC_BLK)
    E.blk("loop")
    E.blk("block")
    E.blk("block")
    for j in range(n):
        E.blk("block")
    E.lget(LOC_BLK)
    E.i32c(a)
    E.o("i32.sub")
    E.byte(0x0E)
    E.u(n)
    for j in range(n):
        E.u(j)
    E.u(n + 1)
    CTXNEST = 0
    for j in range(n):
        E.o("end")
        CTXCNT = n - 1 - j
        bi = a + j
        k0 = STARTS[bi]
        ln = BLEN[bi]
        bpc = WBASE + 4 * k0
        if ATTEST:
            # the deadline tick: one per block
            E.lget(LOC_TICKS)
            E.i64c(1)
            E.o("i64.sub")
            E.ltee(LOC_TICKS)
            E.i64c(0)
            E.o("i64.lt_s")
            E.blk("if")
            CTXNEST = 1
            E.i32c(bi)
            E.lset(LOC_BLK)
            E.i32c(2)
            E.lset(LOC_RET)
            E.br(CTXCNT + CTXNEST + 1)
            CTXNEST = 0
            E.o("end")
        else:
            # the block's steps: the slice counter falls by n; short, it is the budget's end (StepLimit
            # at this block, already refunded) or a refill and a deadline poll in the trampoline
            E.lget(LOC_TICKS)
            E.i64c(ln)
            E.o("i64.sub")
            E.ltee(LOC_TICKS)
            E.i64c(0)
            E.o("i64.lt_s")
            E.blk("if")
            CTXNEST = 1
            E.lget(LOC_TICKS)
            E.i64c(ln)
            E.o("i64.add")
            E.lset(LOC_TICKS)
            E.i32c(bi)
            E.lset(LOC_BLK)
            E.gget(G_FINAL)
            E.blk("if")
            CTXNEST = 2
            fault(9, bpc, 0, 0, 0, 0, 0)
            CTXNEST = 1
            E.o("end")
            E.i32c(2)
            E.lset(LOC_RET)
            E.br(CTXCNT + CTXNEST + 1)
            CTXNEST = 0
            E.o("end")
        for ii in range(ln):
            kk = k0 + ii
            pc = WBASE + 4 * kk
            if ATTEST:
                E.lget(LOC_TOT)
                E.o("i64.eqz")
                E.blk("if")
                CTXNEST = 1
                fault(9, pc, 0, 0, 0, 0, 0)
                CTXNEST = 0
                E.o("end")
                E.lget(LOC_TOT)
                E.i64c(1)
                E.o("i64.sub")
                E.lset(LOC_TOT)
            lower_one(kk, pc, ln - ii - 1)
    # fell off the segment's last block
    CTXCNT = 0
    CTXNEST = 0
    if b < NBLK:
        E.i32c(b)
        E.lset(LOC_BLK)
        E.br(CTXCNT + CTXNEST + 1)
    else:
        fault(8, WBASE + 4 * NWORDS, 0, WBASE + 4 * NWORDS, 0, 0, 0)
    E.o("end")
    E.i32c(1)
    E.lset(LOC_RET)
    E.o("end")
    for r in range(1, 32):
        E.lget(r - 1)
        E.gset(r - 1)
    if ATTEST:
        E.lget(LOC_TOT)
        E.gset(G_TOT)
    E.lget(LOC_TICKS)
    E.gset(G_TICKS)
    E.lget(LOC_BLK)
    E.gset(G_BLK)
    E.lget(LOC_RET)
    E.o("return")
    E.o("end")
    E.i32c(0)
    E.o("end")
    end_body(pos)


# ---------------------------------------------------------------- the module
def sec_begin(sid):
    E.byte(sid)
    return begin_body()


def ft_params(n32, n64):
    E.byte(0x60)
    E.u(n32 + n64)
    for i in range(n32):
        E.byte(WI32)
    for i in range(n64):
        E.byte(WI64)


def ft_none():
    E.u(0)


def sec_types():
    pos = sec_begin(1)
    E.u(9)
    ft_params(3, 0)
    ft_none()
    ft_params(2, 0)
    ft_none()
    ft_params(0, 0)
    E.u(1)
    E.byte(WI32)
    ft_params(0, 1)
    E.u(1)
    E.byte(WI32)
    ft_params(0, 2)
    E.u(1)
    E.byte(WI32)
    ft_params(0, 0)
    E.u(1)
    E.byte(WI64)
    ft_params(0, 2)
    E.u(1)
    E.byte(WI64)
    ft_params(1, 3)
    ft_none()
    ft_params(0, 0)
    ft_none()
    end_body(pos)


def sec_imports():
    pos = sec_begin(2)
    E.u(3)
    E.str("env")
    E.str("write")
    E.byte(0)
    E.u(T_WRITE)
    E.str("env")
    E.str("finish")
    E.byte(0)
    E.u(T_FIN)
    E.str("env")
    E.str("deadline")
    E.byte(0)
    E.u(T_I32)
    end_body(pos)


def glob(ty, mut, v):
    E.byte(ty)
    E.byte(mut)
    if ty == WI32:
        E.i32c(v)
    else:
        E.i64c(v)
    E.o("end")


def sec_globals(attest):
    pos = sec_begin(6)
    E.u(54)
    for i in range(33):
        glob(WI64, 1, 0)
    glob(WI32, 1, BLKOF[(ENTRY - WBASE) // 4])
    for i in range(8):
        glob(WI64, 1, 0)
    glob(WI32, 1, 0)
    glob(WI32, 1, 0)
    for i in range(4):
        glob(WI64, 1, 0)
    glob(WI32, 0, RAMOFF)
    glob(WI64, 0, WBASE)
    glob(WI64, 0, IMGLEN)
    ramdef = EXTENT + 16777216
    if ramdef < 20971520:
        ramdef = 20971520
    glob(WI64, 0, ramdef)
    glob(WI32, 0, attest)
    glob(WI32, 1, 0)
    end_body(pos)


def ex_func(i):
    E.byte(0)
    E.u(i)


def ex_global(i):
    E.byte(3)
    E.u(i)


def sec_exports():
    pos = sec_begin(7)
    E.u(14)
    E.str("memory")
    E.byte(2)
    E.u(0)
    E.str("run")
    ex_func(F_RUN)
    E.str("setup")
    ex_func(F_SETUP)
    E.str("left")
    ex_func(F_LEFT)
    E.str("h_kind")
    ex_global(G_KIND)
    E.str("h_pc")
    ex_global(G_PC)
    E.str("h_addr")
    ex_global(G_ADDR)
    E.str("h_val")
    ex_global(G_VAL)
    E.str("hw")
    ex_global(G_HW)
    E.str("ram_off")
    ex_global(G_RAMOFF)
    E.str("guest_base")
    ex_global(G_BASE)
    E.str("image_len")
    ex_global(G_IMGLEN)
    E.str("ram_default")
    ex_global(G_RAMDEF)
    E.str("attest")
    ex_global(G_ATTEST)
    end_body(pos)


def sec_data():
    pos = sec_begin(11)
    E.u(2)
    # the tables: the jump table (a block index per code word), then the block -> segment table
    E.byte(0)
    E.i32c(WL_JT)
    E.o("end")
    E.u(RAMOFF - WL_JT)
    for k in range(NWORDS):
        v = 0xFFFFFFFF
        if LEADER[k]:
            v = BLKOF[k]
        E.byte(v)
        E.byte(v >> 8)
        E.byte(v >> 16)
        E.byte(v >> 24)
    for j in range(RAMOFF - WL_JT - 4 * NWORDS):
        z = 0
        q = WL_JT + 4 * NWORDS + j - SEGOFAT
        if q >= 0 and q < 2 * NBLK:
            if q % 2 == 0:
                z = SEGOF[q // 2] & 255
            else:
                z = SEGOF[q // 2] >> 8
        E.byte(z)
    # the guest image at RAMOFF
    E.byte(0)
    E.i32c(RAMOFF)
    E.o("end")
    E.u(IMGLEN)
    for i in range(IMGLEN):
        E.byte(IMG[i])
    end_body(pos)


def wasm_module(attest):
    E.byte(0)
    E.byte(0x61)
    E.byte(0x73)
    E.byte(0x6D)
    E.byte(1)
    E.byte(0)
    E.byte(0)
    E.byte(0)
    sec_types()
    sec_imports()
    pos = sec_begin(3)
    E.u(16 + NSEG)
    for i in range(F_FLUSH, F_LEFT + 1):
        E.u(h_type(i))
    for i in range(NSEG):
        E.u(T_I32)
    end_body(pos)
    pos = sec_begin(4)
    E.u(1)
    E.byte(0x70)
    E.byte(0)
    E.u(NSEG)
    end_body(pos)
    pos = sec_begin(5)
    E.u(1)
    E.byte(1)
    E.u((RAMOFF + IMGLEN + 0xFFFF) // 0x10000)
    E.u(65536)
    end_body(pos)
    sec_globals(attest)
    sec_exports()
    pos = sec_begin(9)
    E.u(1)
    E.byte(0)
    E.i32c(0)
    E.o("end")
    E.u(NSEG)
    for i in range(NSEG):
        E.u(F_SEG0 + i)
    end_body(pos)
    pos = sec_begin(10)
    E.u(16 + NSEG)
    for i in range(F_FLUSH, F_LEFT + 1):
        p2 = begin_body()
        h_emit(i)
        end_body(p2)
    for i in range(NSEG):
        seg_func(i)
    end_body(pos)
    sec_data()
    return 0


def wasm_sizes():
    """size every run by its length first (T1 runs grow by a store at the last index)"""
    grow(LEADER, NWORDS)
    grow(BLKOF, NWORDS)
    grow(STARTS, NWORDS)
    grow(BLEN, NWORDS)
    grow(SEGA, NWORDS)
    grow(SEGB, NWORDS)
    grow(SEGOF, NWORDS)


def wasm_entry():
    wasm_sizes()
    wasm_plan()
    wasm_module(ATTEST)


def translate_wasm(img, arg0, attest=False):
    """the oracle's entry: parse, decode into the module state, run the restricted code"""
    global E, WORDS, OPS, RD, RS1, RS2, IMM, IMG, NWORDS, CODE_SIZE, PROT_SIZE, ENTRY, IMGLEN, EXTENT, ATTEST
    global LEADER, BLKOF, STARTS, BLEN, SEGA, SEGB, SEGOF
    entry, base, span, extent, code_size, prot_size = parse_elf(img)
    assert base == 0x8000_0000, "milestone 1: images load at 0x8000_0000"
    NWORDS = code_size // 4
    WORDS = list(struct.unpack_from("<%dI" % NWORDS, span))
    OPS, RD, RS1, RS2, IMM = [], [], [], [], []
    for w in WORDS:
        d = decode(w)
        if d is None:
            d = ("", 0, 0, 0, 0)
        OPS.append(KNAMES.index(d[0]) if d[0] else 0)
        RD.append(d[1])
        RS1.append(d[2])
        RS2.append(d[3])
        IMM.append(d[4])
    IMG = list(span)
    CODE_SIZE, PROT_SIZE, ENTRY, IMGLEN, EXTENT = code_size, prot_size, entry, len(span), extent
    ATTEST = int(attest)
    E = WB()
    wasm_entry()
    return bytes(E.b), {"segments": NSEG, "blocks": NBLK, "ramoff": RAMOFF}



if __name__ == "__main__":
    av = sys.argv[1:]
    target = "x86_64-linux"
    if av and av[0] == "--target":
        target = av[1]
        av = av[2:]
    assert target in ("x86_64-linux", "aarch64-linux", "aarch64-macos", "x86_64-macos", "aarch64-windows",
                      "x86_64-windows", "wasm32", "wasm32-attest"), target
    img = open(av[0], "rb").read()
    arg0 = av[2] if len(av) > 2 else av[0]
    if target.startswith("wasm32"):
        out, M = translate_wasm(img, arg0, target == "wasm32-attest")
    elif target.startswith("x86_64"):
        out, M = translate(img, arg0, {"x86_64-macos": True, "x86_64-windows": "win"}.get(target, False))
    else:
        out, M = translate_a64(img, arg0, {"aarch64-macos": True, "aarch64-windows": "win"}.get(target, False))
    open(av[1], "wb").write(out)
    import os
    os.chmod(av[1], 0o755)
    if target.startswith("wasm32"):
        sys.stderr.write("anuvada: %d octets, %s\n" % (len(out), M))
    else:
        sys.stderr.write("anuvada: %d octets, mapped %s\n" % (len(out), {("x%d" % r): h for r, h in M.items()}))
