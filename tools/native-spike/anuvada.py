#!/usr/bin/env python3
# ROLE: ORACLE for the .t1 translator (native route B; owner: "lowering ok for x86_64").
# anuvāda = translation. RV64 image -> a STATIC x86-64 Linux ELF, BYTE FOR BYTE: its own
# x86-64 encoder, its own runtime written as machine code (Linux syscalls only, no libc),
# its own ELF writer. No assembler, no compiler, no C. The .t1 translator must emit the
# SAME OCTETS as this file for the same image; this file must, in turn, agree with yantra
# (byte-identical program output; the compiler's Stage 2 reproduces itself).
# Lowering: tools/native-spike/LOWERING-x86-64.md (owner-approved 2026-10-08).
# MILESTONE 1 scope: RV64 I + M, fld/fsd as moves, the sstatus CSR write, UART, FINISHER.
# Every other instruction lowers to a refusal that names its pc and word at run time.
# Usage: anuvada.py [--target aarch64-linux] IMAGE.elf OUT [ARG0]   (ARG0: what the program
#        sees as argument 0, yantra's image path; default: IMAGE as given). The default target
#        is x86_64-linux; aarch64-linux is the second target (translate_a64, at the end),
#        aarch64-macos the third (translate_a64 with mac=True, macho_a64), x86_64-macos the
#        fourth (translate with mac=True, macho_x86), wasm32 (and wasm32-attest) the fifth: a .wasm
#        module, translate_wasm at the end (WASM32.md; run it with wasm-host.mjs).
import hashlib
import os
import struct
import sys

import softfp

TEXT = 0x400000          # the host image's text segment
HOST = [3, 5, 12, 13, 14, 15, 6, 7, 8, 9, 10, 11]   # rbx rbp r12..r15 rsi rdi r8..r11
RAX, RCX, RDX, RBX, RSP, RBP, RSI, RDI = range(8)
R8, R9, R10, R11, R12, R13, R14, R15 = 8, 9, 10, 11, 12, 13, 14, 15
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
# the addresses below base where yantra's Machine::store answers a device by its state, not
# BadAccess: the socket (0x1000_0110..0x1000_0120) and the virtio-mmio slots. Not native yet.
STORE_DEVICES = ((0x1000_0110, 0x10), (0x1000_1000, 0x8000))
# virtio-mmio slot 1 (0x1000_2000..0x1000_3000) holds the file window: yantra answers every load
# there, and every store but its four patra stores, with this Device halt whatever its state
# (virtio_mmio.rs check()). Tested BEFORE LOAD_DEVICES / STORE_DEVICES, which contain it.
SLOT1 = (0x1000_2000, 0x1000)
SLOT1_WHY = (b"virtio-mmio slot 1 overlaps the file window (patra::PATRA_PATH is its base); "
             b"only the four patra stores are answered here and no virtio device sits in it")
# THE SOCKET (W-377, yantra's socket; SOCKET.md): NEXT (store 0) pops the queue into the RX
# latch, RX (load) answers it, TX (store) sends an octet, +0xc (CLOSE) is refused. Live only
# (YANTRA_LISTEN=127.0.0.1:PORT with YANTRA_RECORD_EVENTS=<log>), Linux only; the words below are
# socket's, copied by script (tools/check-anuvada-words.sh checks them against the source).
SOCK, SOCK_LEN = 0x1000_0110, 16
SOCK_EMPTY, SOCK_END, SOCK_UNREAD = 0x100, 0x101, 0x102
SOCK_RECORD_CAP, SOCK_SESSION_CAP = 4096, 64 << 20
SOCK_WHY = (
    ("WHY_NO_DEVICE", b'no socket device: pass `--listen` or a socket log (W-377)'),
    ("WHY_WIDTH", b"the socket device's registers are 32 bits wide and aligned: a four-octet access at NEXT, RX or TX (W-377)"),
    ("WHY_LOAD_STORE_ONLY", b'NEXT and TX are store-only registers: read the socket at RX (W-377)'),
    ("WHY_STORE_LOAD_ONLY", b'RX is a load-only register: store 0 to NEXT to pop the queue into it (W-377)'),
    ("WHY_NEXT_VALUE", b'NEXT takes the value 0 only: other values are reserved for a connection id (W-377)'),
    ("WHY_TX_VALUE", b'TX sends one octet: a value above 0xff is refused, never truncated (W-377)'),
    ("WHY_CLOSE", b'0x1000_011c is reserved for CLOSE, which is deferred: the program cannot close the connection in this row (W-377)'),
)
SOCKET_LIMIT_EXCEEDED = b'\xe0\xa4\xb8\xe0\xa4\x99\xe0\xa5\x8d\xe0\xa4\x95\xe0\xa5\x87\xe0\xa4\xa4\xe0\xa4\xae\xe0\xa4\xbe\xe0\xa4\xa4\xe0\xa5\x8d\xe0\xa4\xb0\xe0\xa4\xbe\xe0\xa4\xaa\xe0\xa5\x8d\xe0\xa4\xb0\xe0\xa4\xa4\xe0\xa4\xbf\xe0\xa4\xb7\xe0\xa5\x87\xe0\xa4\xa7\xe0\xa4\x83'
SOCKET_LOG_HEADER = b'# yantra-run --record-events --listen (W-377): one socket record per wait, s=<hex octets> or s=end; replay with --events\n'
FILES_GRANT_LINE = b"# yantra-run --files: this run was granted a file root; replay it with --files DIR\n"
# the socket's state block `sk` (both runtimes): fds are stored + 1 (0 = none); the queue is one
# mmap'ed run of SESSION_CAP + RECORD_CAP octets that only grows (head <= tail = octets delivered)
(SK_ON, SK_LFD, SK_CFD, SK_Q, SK_HEAD, SK_TAIL, SK_REC, SK_SENT, SK_END, SK_LATCH, SK_SFD, SK_FAIL,
 SK_SAID, SK_T0, SK_T1, SK_T2, SK_SA, SK_LEN, SK_POLL, SK_B, SK_LR, SK_TS) = (
    0, 8, 16, 24, 32, 40, 48, 56, 64, 72, 80, 88, 96, 104, 112, 120, 128, 144, 152, 160, 168, 176)
SK_SIZE, SK_LINE = 192, 2 + 2 * 4096 + 8 + 8
SK_RP = 184   # replay flag: a socket LOG (YANTRA_SOCK_REPLAY) feeds the waits; LFD = its cursor, POLL = its end
# the socket's syscalls on Darwin (BSD numbers), keyed by the Linux number each runtime spells:
# x86-64 (read write open close poll mmap lseek socket sendto accept bind listen getsockname
# setsockopt ftruncate) and A64 (read write openat close ppoll mmap lseek socket sendto accept bind
# listen getsockname setsockopt ftruncate); Darwin's poll (230) stands for ppoll, open (5) for openat
SOCK_DARWIN_X86 = {0: 3, 1: 4, 2: 5, 3: 6, 7: 230, 9: 197, 8: 199, 41: 97, 44: 133, 43: 30, 49: 104,
                   50: 106, 51: 32, 54: 105, 77: 201}
SOCK_DARWIN_A64 = {63: 3, 64: 4, 56: 5, 57: 6, 73: 230, 222: 197, 62: 199, 198: 97, 206: 133, 202: 30,
                   200: 104, 201: 106, 204: 32, 208: 105, 46: 201}
M64 = (1 << 64) - 1
# THE HALTS YANTRA MAKES, NATIVELY (NATIVE-GAPS A1/A2). yantra (crates/yantra/src/lib.rs, step_inner)
# executes the major opcodes below and halts `Unimplemented { pc, word, opcode }` on every other word,
# compressed encodings included (IALIGN 32: a word whose low two bits are not 11 is one of them).
# Bit i of YANTRA_OPS is the opcode (i << 2) | 3. A native run is always in S-mode with stvec = 0 and
# sepc = 0 (stvec and sepc are never written: only sstatus, sscratch and the float three are lowered, and no trap is delivered),
# so ebreak halts `Breakpoint { pc }`, sret returns to pc 0 (a fetch there: `BadAccess { pc: 0, addr: 0 }`)
# and sfence.vma is a no-op, exactly as yantra.
# A CSR instruction on a number outside yantra's csr_read set halts `Csr { pc, csr, write }` (H5), as
# yantra does; one inside it that the translator does not lower is refused (t1-run falls back).
YANTRA_CSRS = (0x100, 0x104, 0x144, 0x105, 0x140, 0x141, 0x142, 0x143, 0x180, 0xC01, 0x009, 0x00A, 0x00F, 0x008,
               0xC20, 0xC21, 0xC22, 0x001, 0x002, 0x003)
# sstatus as yantra keeps it (crates/yantra/src/lib.rs): stored masked to SSTATUS_MASK (SIE SPIE SPP SUM MXR FS VS);
# a read adds UXL (2 << 32) and SD (bit 63) when FS or VS reads 3. FS lives in fsen and VS in vsen when the image
# carries them (they gate the float and vector words); everything else in `sst`. sscratch is plain storage (H8).
SSTATUS_MASK, SSTATUS_UXL, SSTATUS_FS, SSTATUS_VS = 0xC6722, 2 << 32, 3 << 13, 3 << 9
CSR_DATA = [("sst", 8), ("sscr", 8)]
YANTRA_OPS = sum(1 << (o >> 2) for o in (0x03, 0x07, 0x0F, 0x13, 0x17, 0x1B, 0x23, 0x27, 0x2F, 0x33, 0x37,
                                         0x3B, 0x43, 0x47, 0x4B, 0x4F, 0x53, 0x57, 0x63, 0x67, 0x6F, 0x73))
# the fetch windows yantra's `load` answers below RAM (a jump there reads a device): SOCK's 16 octets
# and the virtio-mmio window's 8 slots of 0x1000; any other address below RAM is BadAccess
SOCK, SOCK_LEN, VIRTIO_LO, VIRTIO_LEN = 0x1000_0110, 16, 0x1000_1000, 8 * 0x1000


def yantra_static(w):
    """what yantra does with a word decode() does not lower: 0 something the native code does not
    emulate (the run is refused: t1-run falls back), 1 Unimplemented, 2 Breakpoint, 3 sret, 4 sfence.vma,
    5 Csr (a CSR instruction on a number yantra does not implement)"""
    if w & 3 != 3 or not (YANTRA_OPS >> ((w & 0x7F) >> 2)) & 1:
        return 1
    if w & 0x7F == 0x2F:
        if (w >> 12) & 7 in (2, 3):
            return 0      # a decodable A word the wasm32 path blanks (translate_wasm): refused, kind 7 (not Unimplemented)
        return 1          # an A word decode() did not lower: funct3 is not 2 or 3, yantra's Unimplemented
    if w & 0x7F != 0x73:
        return 0
    f3 = (w >> 12) & 7
    if f3 == 4:
        return 1
    if f3:
        return 0 if (w >> 20) in YANTRA_CSRS else 5
    if w & 0xFE007FFF == 0x12000073:
        return 4
    if w in (0x00100073, 0x10200073):
        return 2 if w == 0x00100073 else 3
    return 0 if w == 0x73 else 1


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
        self.fs = None         # FORK-JOIN (FORKJOIN.md): data labels addressed fs-relative, per thread
        self.fjsys = False     # FORK-JOIN: syscall() is a call of rt_fj_sys

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
        if kind == "mabs":     # [base + label + addend], disp32 (FORK-JOIN: a thread's copy of a data label)
            base = op[1]
            self.raw(0x80 | (reg & 7) << 3 | (base & 7))
            if base & 7 == 4:
                self.raw(0x24)
            self.abs32(op[2], op[3])
            return r, 0, base >> 3
        if kind == "sib2":
            idx = op[1]
            self.raw(0x04 | (reg & 7) << 3, 0x40 | (idx & 7) << 3 | 5)
            self.abs32(op[2])
            return r, idx >> 3, 0
        raise ValueError(op)

    def fsrel(self, rm):
        """FORK-JOIN images only: a data label is per thread, read through fs (fs base 0 in the main thread)"""
        return self.fs is not None and rm[0] == "abs" and rm[1] in self.fs

    def op(self, opcode, reg, rm, w=1, p66=False):
        """[66] [64] REX opcode modrm…  — REX is ALWAYS emitted (fixed lengths; 0x40 when empty).
        FORK-JOIN: the fs override goes last among the legacy prefixes, right before REX"""
        if p66:
            self.raw(0x66)
        if self.fsrel(rm):
            self.raw(0x64)
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
        if self.fsrel(rm):     # lea ignores the segment: the thread's address is fj_delta + the label
            self.mov_r_rm(r, ("abs", "fj_delta", 0))
            self.op(b"\x8d", r, ("mabs", r, rm[1], rm[2]))
            return
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
        if self.fjsys:                 # FORK-JOIN: a worker's host calls are diverted (FORKJOIN.md)
            self.call("rt_fj_sys")
            return
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
    """OP-FP (0x53). An rm field decodes whatever it holds: a reserved one (static 5 / 6, or DYN while frm
    holds 5..7) is yantra's illegal instruction, found in ONE place, the helpers' shared rm entry (fpx_rm /
    rm_resolve), after the FS check as yantra orders them (H7)."""
    fmt = f7 & 3
    if fmt > 1:
        return None
    sd = ".d" if fmt else ".s"
    op5 = f7 >> 2
    if op5 in (0, 1, 2, 3):
        return ("fadd fsub fmul fdiv".split()[op5] + sd, rd, rs1, rs2, f3)
    if op5 == 0x0B:
        if rs2 != 0:
            return None
        return ("fsqrt" + sd, rd, rs1, 0, f3)
    if op5 == 8:                              # fcvt.s.d (fmt 0, rs2 1) / fcvt.d.s (fmt 1, rs2 0)
        if rs2 != (0 if fmt else 1):
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
        if rs2 > 3:
            return None
        return ("fcvt.%s%s" % (("w", "wu", "l", "lu")[rs2], sd), rd, rs1, 0, f3)
    if op5 == 0x1A:
        if rs2 > 3:
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
            return ("slli", rd, rs1, 0, sh)    # yantra reads no funct7 bit of SLLI (L31)
        if f3 == 5:
            # yantra reads only bit 30 of funct7 (SRAI vs SRLI); the other reserved bits are ignored (L31)
            return ("srai" if (w >> 30) & 1 else "srli", rd, rs1, 0, sh)
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
    if op == 0x2F and f3 in (2, 3):
        # A: lr / sc / amo*, .w (f3 2) or .d (f3 3): imm = funct5 | width bit << 5 (rt_amo reads the word itself)
        return ("amo", rd, rs1, rs2, (w >> 27) | (f3 & 1) << 5)
    if op == 0x57 or (op in (0x07, 0x27) and f3 in (0, 5, 6, 7)):
        # the V extension: the word itself rides in imm (rt_vec decodes it); only vset* writes an x register
        return ("vec", rd if op == 0x57 and f3 == 7 else 0, rs1, rs2, w)
    if op == 0x53:
        return decode_fp(w, rd, f3, rs1, rs2, f7)
    if op in (0x43, 0x47, 0x4B, 0x4F) and (w >> 25) & 3 < 2:
        return ("fmadd fmsub fnmsub fnmadd".split()[(op >> 2) & 3] + (".d" if (w >> 25) & 3 else ".s"),
                rd, rs1, rs2, f3 | (w >> 27) << 3)
    if op == 0x73 and f3 in (1, 2, 3, 5, 6, 7) and (w >> 20) in (1, 2, 3):
        return (CSR_OPS[f3 - 1 if f3 < 4 else f3 - 2], rd, rs1, 0, w >> 20)
    if op == 0x73 and f3 in (1, 2, 3, 5, 6, 7) and (w >> 20) == 0x100:
        # sstatus: the operand rides in imm as f3 | rs1 << 3 (rs1 is the register or the 5-bit immediate). An image
        # whose sstatus words never read it (rd 0) nor clear with a register keeps the startup's lowering (only FS and
        # VS, as fsen / vsen, and only when the image carries them); any other is exact (csr_plan: the full form)
        return ("csrs_sstatus", rd, 0, 0, f3 | rs1 << 3)
    if op == 0x73 and f3 in (1, 2, 3, 5, 6, 7) and (w >> 20) == 0x140:
        return ("csr_sscratch", rd, 0, 0, f3 | rs1 << 3)      # sscratch: plain storage, the operand as sstatus's
    return None


def csr_plan(decs):
    """(sst_full, csr_data): sst_full when an sstatus word reads it (rd != 0) or clears with a register (csrrc,
    rs1 != 0; a clear can turn a Dirty FS into a Clean one that the startup form would turn off), so that sstatus
    is kept exactly (sst, FS dirtied by every float write); csr_data when the image needs the `sst` / `sscr` words"""
    full = any(d and d[0] == "csrs_sstatus" and (d[1] != 0 or (d[4] & 7 == 3 and d[4] >> 3 != 0)) for d in decs)
    return full, full or any(d and d[0] == "csr_sscratch" for d in decs)


# ---------------------------------------------------------------- the translator
BOOT_BASE = 0x8000_0000          # a boot-proof's link base (ADR-0015); the supervisor's gigabyte
IMAGE_BASE = BOOT_BASE           # THE base cell: the image's link base, set by require_base()


def image_bases():
    """the bases an image may be linked at: the boot base and every row of spec/application-load.tsv
    (the one table of application load addresses; nothing is copied from it by hand)"""
    ok = [BOOT_BASE]
    tsv = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "spec", "application-load.tsv")
    for ln in open(tsv, encoding="utf-8"):
        f = ln.split("\t")
        if not ln.startswith("#") and f[0].startswith("0x"):
            ok.append(int(f[0].replace("_", ""), 16))
    return ok


def require_base(base):
    """refusal E-BASE: the image's lowest PT_LOAD vaddr is not an accepted link base; else sets the base cell"""
    global IMAGE_BASE
    if base != BOOT_BASE and base not in image_bases():
        raise SystemExit("native: E-BASE: image base %#x is not %#x or a row of spec/application-load.tsv; this layout is not translated\n"
                         % (base, BOOT_BASE))
    IMAGE_BASE = base


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


# ---------------------------------------------------------------- FORK-JOIN (FORKJOIN.md)
# The compiler's region marker routine (ir.t1 `समान्तरवृत्तिसंज्ञा`, yantrotsarjana.t1
# `यन्त्रसमान्तरचिह्नोत्सर्जनम्`) is FIXED TEXT: `lui x0, 0xF0A11` (a hint), `mv a0, a1`, `ret`,
# called as (phase, lo, hi, out, body). It answers its second argument: the begin call's value
# is where the serial loop starts, so a runtime that ran [lo, hi) on threads answers hi.
FJ_HINT = (0xF0A11037, 0x00058513, 0x00008067)
FJ_STACK = 65536            # yantrotsarjana.t1 यन्त्रस्तूपाष्टकाः: the stack's octets, below the entry's sp
FJ_MAX = 64                 # threads at most (SASSEMBLY_NATIVE_THREADS is clamped to it)
FJ_SLICE = 81920            # a worker's guest stack slice (>= the serial stack + 4 KiB + 2 x 2048)
FJ_CAP = 16384              # a worker's capture of what it would write (records), per region
FJ_HSTACK = 65536           # a worker's host stack
FJ_SENTINEL = 0x7FFF_FFF0   # the return address a worker gives the body: outside the code, even
FJ_K = 8192                 # the small-region threshold: T = min(N, (hi - lo) // K) threads, serial below 2
                            # (SASSEMBLY_NATIVE_FJ_MIN=K overrides it; 0 = no threshold). Measured: FORKJOIN.md


# the x86-64 fork-join data, after x86_data() and the F/D data (always present in a fork-join image,
# so every offset below is one constant): per thread (each worker has a copy; fs = its delta):
# fj_worker 1 in a worker; fj_delta its block - the main block; fj_i / fj_hi its chunk; fj_bout /
# fj_bbody the region's output run and body; fj_sp the body's sp; fj_kind how it ended (1 done,
# 2 exit with fj_code, 3 a host call or device it may not use, 4 its stack slice overrun); fj_cap
# its records (fd, length, octets) of what it would have written. Main only: fj_n the switch,
# fj_hsave the host registers at the marker (HOST order), fj_g the marker's a0..a4 and sp (from
# the image's stub rt_fj_args, with fj_sbx = SB + 2056, fj_sp0, fj_ext = the extent's page end),
# fj_tids the CLONE_CHILD_CLEARTID words, the region's numbers and the pool of worker blocks.
FJ_X86_DATA = [("fj_worker", 8), ("fj_delta", 8), ("fj_i", 8), ("fj_hi", 8), ("fj_bout", 8), ("fj_bbody", 8),
               ("fj_sp", 8), ("fj_kind", 8), ("fj_code", 8), ("fj_caplen", 8), ("fj_capover", 8),
               ("fj_n", 8), ("fj_k", 8), ("fj_hsave", 8 * 12), ("fj_g", 8 * 6), ("fj_sbx", 8), ("fj_sp0", 8), ("fj_ext", 8),
               ("fj_t", 8), ("fj_lo", 8), ("fj_hiall", 8),
               ("fj_q", 8), ("fj_r", 8), ("fj_out", 8), ("fj_body", 8), ("fj_depth", 8), ("fj_area", 8),
               ("fj_pool", 8), ("fj_c", 8), ("fj_stp", 8), ("fj_tids", 8 * FJ_MAX), ("fj_cap", FJ_CAP),
               ("fj_pmax", 8), ("fj_regions", 8), ("fj_spawned", 8), ("fj_att", 8), ("fj_abuf", 192)]


def fj_x86_layout():
    """the fork-join image's data list and each symbol's offset from regs"""
    dl = x86_data() + FP_DATA + FJ_X86_DATA
    off, o = {}, 0
    for n, sz in dl:
        off[n] = o
        o += sz
    return dl, off, o


def fj_plan(words, decs, base, entry, nwords):
    """The fork-join plan of an image, or None (then the image is translated exactly as before):
    the word indices of every marker routine that some `jal ra` calls, and the stack's bottom
    word SB (the canary's address: the entry sets sp = SB + 64 KiB and arms SB)."""
    hints = []
    for k in range(nwords - 2):
        if tuple(words[k:k + 3]) == FJ_HINT:
            called = any(d and d[0] == "jal" and d[1] == 1 and j + d[4] // 4 == k for j, d in enumerate(decs))
            if called:
                hints.append(k)
    if not hints:
        return None
    e = (entry - base) // 4
    d0, d1 = decs[e], decs[e + 1]
    if not (d0 and d1 and d0[0] == "auipc" and d0[1] == 2 and d1[0] == "addi" and d1[1] == 2 and d1[2] == 2):
        return None
    sp0 = entry + d0[4] + d1[4]
    sb = sp0 - FJ_STACK
    # the canary: auipc t5, addi t5 t5 = SB, then ld t4, 0(t5) (yantrotsarjana.t1, V-009 finding 6)
    for k in range(e + 2, min(e + 16, nwords - 2)):
        a, b, c = decs[k], decs[k + 1], decs[k + 2]
        if a and b and c and a[0] == "auipc" and b[0] == "addi" and b[1] == b[2] == a[1] and c[0] == "ld" \
                and c[2] == a[1] and c[4] == 0:
            if base + 4 * k + a[4] + b[4] == sb:
                return {"hints": set(hints), "sb": sb, "sp0": sp0}
            return None
    return None


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


def elf_writable(img):
    """(p_vaddr, p_filesz if PF_W else 0) of every PT_LOAD, in header order: the only bytes a tag may live in"""
    _entry, phoff = struct.unpack_from("<QQ", img, 24)
    phentsize, phnum = struct.unpack_from("<HH", img, 54)
    out = []
    for i in range(phnum):
        t, fl, _off, va, _pa, fsz, _msz, _al = struct.unpack_from("<IIQQQQQQ", img, phoff + i * phentsize)
        if t == 1:
            out.append((va, fsz if fl & 2 else 0))
    return out


def seg_tab(A, img):
    """"seg_tab", 8-aligned: u64 count, then (p_vaddr, need) u64 pairs in header order (elf_loads)"""
    loads = elf_loads(img)
    A.raw(b"\0" * (-len(A.b) % 8))
    A.label("seg_tab")
    A.raw(struct.pack("<Q", len(loads)) + b"".join(struct.pack("<QQ", va, need) for va, need in loads))
    # "wr_tab" (GAP-TAGSCAN): the same count, then (p_vaddr, writable p_filesz) pairs: p_filesz when the
    # PT_LOAD has PF_W, else 0. The tag scan (rt_find_tag) reads only these file-backed writable bytes.
    A.label("wr_tab")
    A.raw(struct.pack("<Q", len(loads)) + b"".join(struct.pack("<QQ", va, wf) for va, wf in elf_writable(img)))


# ---- THE STEP CEILING (L18/H1): yantra-run stops a run at DEFAULT_STEPS = 1_000_000 instructions
# (or YANTRA_STEPS) with `halt: StepLimit { pc }`, pc the first instruction NOT executed. Native code
# charges in two ways. The FAST region charges a block's words when the block is entered (the FB_
# leader). A charge that would cross the limit goes to S_k, which gives the words back and enters the
# SAFE region at the block's first word; the SAFE region charges ONE word before it runs it, so the
# limit is met at the exact word: the halt is native, whatever the words before it did (a load, a
# store, a call), and it names the exact pc. A jalr into the middle of a block lands in the SAFE
# region, so it needs no charge of its own. A fork-join worker that meets the limit makes the run
# fall back to yantra-run (fj_kind 3), and the join charges the workers' words to the main thread.
STEP_BUDGET = 1000000


def step_plan(decs, leader):
    """per word: the words from it to the end of its block"""
    n = len(decs)
    rem = [0] * n
    for k in range(n - 1, -1, -1):
        last = k + 1 == n or leader[k + 1]
        rem[k] = 1 + (0 if last else rem[k + 1])
    return rem


def x86_data():
    """the x86-64 data symbols in order, with their sizes (t1/authoring/mk.sh writes main.t1a's
    layout from this list, so the two cannot drift)"""
    return [("regs", 256), ("fregs", 256), ("lim_l1", 8), ("lim_l2", 8), ("lim_l4", 8),
            ("lim_l8", 8), ("lim_s1", 8), ("lim_s2", 8), ("lim_s4", 8), ("lim_s8", 8),
            ("fault_off", 8), ("fault_word", 8), ("fault_addr", 8), ("resv", 8), ("code_end", 8), ("sp_lo", 8), ("sp_span", 8),
            ("steps", 8), ("steps0", 8), ("outlen", 8), ("argc", 8), ("argv", 8),
            ("envp", 8), ("ram", 8), ("top", 8), ("in_fd", 8), ("in_len", 8), ("name_ptr", 8),
            ("name_len", 8), ("args_len", 8), ("ev_slot", 8), ("rec_fd", 8), ("rep_fd", 8), ("win_path", 8), ("win_buf", 8), ("win_st", 8),
            ("win_param", 8), ("pwv", 8 * PW_SLOTS), ("spn", SPN_SIZE), ("num", 32), ("statbuf", 144),
            ("outbuf", 65536), ("pwb", PWB_SIZE), ("sk", SK_SIZE), ("sk_line", SK_LINE),
            # SASTHRDS threads (runtime(): the threads block), last so every symbol above keeps its place
            ("th_n", 8), ("th_cur", 8), ("th_ctx", 8), ("th_id", 8), ("th_stk", 8), ("th_slim", 8),
            ("th_bos", 8), ("th_bosk", 8), ("th_cap", 8), ("th_capn", 8), ("th_failk", 8), ("th_failv", 8),
            ("th_lastv", 8), ("th_rc", 8), ("th_pre", 8), ("th_ended", 8), ("th_len", 8), ("th_e1", 8),
            ("th_e2", 8)]


VD_BUF, VD_OUT = 16384, 20480
VD_DATA = [("vd_st", 8), ("vd_lim", 8), ("vd_fd", 8), ("vd_sv", 8), ("vd_n", 8), ("vd_buf", VD_BUF), ("vd_out", VD_OUT)]
VD_STEPS = 1_000_000     # yantra's DEFAULT_STEPS


def wm_x86(A, reg, disp, width, lab):
    """the store high-water note (YANTRA_WATERMARK, `wm`): hw = max(hw, addr + width) from the address
    [reg + disp] of a store that is already known to be in RAM. hw is the guest's ABSOLUTE end address (0: no
    store yet), as on aarch64; rt_wm_report subtracts the base, so no constant is folded into the displacement
    (a disp below -width would overflow the int32 of a folded one). Clobbers rdx and the flags"""
    A.lea(RDX, ("m", reg, disp + width))
    A.alu_r_rm("cmp", RDX, ("abs", "hw", 0))
    A.jcc("be", lab)
    A.mov_rm_r(("abs", "hw", 0), RDX)
    A.label(lab)


def translate(img, arg0, mac=False, wm=False, vd=False):
    """wm: the store high-water mark is kept (the data symbol hw) and said at exit when YANTRA_WATERMARK
    is set, as yantra-run does; Linux, no fork-join. Off, the output is what it always was.
    vd (implies wm): yantra-run's own accounting (L33): the executed-instruction count (vd_st), the step
    ceiling YANTRA_STEPS (StepLimit), `jal .` (SpinForever), the `steps:` line and the YANTRA_VERDICT file"""
    wm = wm or vd
    entry, base, span, extent, code_size, prot_size = parse_elf(img)
    require_base(base)
    assert not wm or not mac, "the watermark is Linux only"
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
    # an image with a vector word carries rt_vec and its data, and the F/D runtime (its float ops, fsen, rt_illegal)
    vec_used = any(d and d[0] == "vec" for d in decs)
    fp_used = fp_used or vec_used
    amo_used = any(d and d[0] == "amo" for d in decs)       # lr / sc / amo*: rt_amo (a fork-join image is refused)
    sst_full, csr_data = csr_plan(decs)
    fs_dirty = sst_full and fp_used           # FS is observable: every float write makes it Dirty (yantra's mark_fp_dirty)
    csrh_used = any(d is None and yantra_static(w) == 5 for d, w in zip(decs, words))

    A = Asm(TEXT + 0x1000)   # code starts one page into the text segment (headers first)
    DATA = {}                # data symbols, laid out after the text
    # FORK-JOIN: only when asked at translation (ANUVADA_FORKJOIN=1), on Linux, with FAST blocks,
    # and for an image that calls the marker routine; every other translation is octet for octet
    # what it was. The run itself is then switched by SASSEMBLY_NATIVE_THREADS (0 = serial).
    fj = None
    net = os.environ.get("ANUVADA_NET") == "1" and not mac     # the frames device (runtime_net)
    assert not (vd and net), "the verdict is not offered with the frames device"
    scan = os.environ.get("ANUVADA_SC") == "1" and not mac and not wm   # slot 0's GPU (scanout_dev.py)
    scanp = None
    if scan:
        import scanout_dev
        scanp = scanout_dev.build(base)
    # YANTRA_VIRTIO_DEFER (L33): with the frames device, a translation that ticks yantra's deferred-completion
    # clock before every guest word (SAFE blocks only); without ANUVADA_VD=1 the output is what it always was
    vdf = net and os.environ.get("ANUVADA_VD") == "1"
    if os.environ.get("ANUVADA_FORKJOIN") == "1" and not mac and not net and not scan and 2 in M and not vec_used and not amo_used and not csr_data and not wm:
        fj = fj_plan(words, decs, base, entry, nwords)
    DLIST = x86_data()
    if wm:
        DLIST += [("hw", 8)]
    if vd:
        DLIST += VD_DATA
    if fp_used:
        DLIST += FP_DATA
    if vec_used:
        DLIST += VEC_DATA
    if csr_data:
        DLIST += CSR_DATA
    if net:
        DLIST += [("nt", NT_SIZE + (NTV_SIZE if vdf else 0))]
    if scan:
        DLIST += [("gp", scanp.size())]
    if fj:
        DLIST, fj["off"], fj["dsize"] = fj_x86_layout()
        A.fs = set(n for n, _ in DLIST)
        A.fjsys = True                 # every syscall goes through rt_fj_sys

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
    fast_ok = 2 in M and not vdf
    rem = step_plan(decs, leader)
    stubs, stubs1 = set(), set()

    def charge(k):
        """a FAST block's words off the budget; borrow: the limit falls inside the block (S_k: SAFE, word by word)"""
        if vd:                                    # vd mode counts its own words (vd_st, vd_lim)
            return
        A.alu_rm_imm("sub", ("abs", "steps", 0), rem[k])
        A.jcc("b", "S_%x" % (base + 4 * k))
        stubs.add(k)

    def charge1(k):
        """the SAFE region: one word off the budget before it runs; borrow: the limit is met at this word"""
        if vd:
            return
        A.alu_rm_imm("sub", ("abs", "steps", 0), 1)
        A.jcc("b", "T_%x" % (base + 4 * k))
        stubs1.add(k)

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

        if vd:
            # yantra's clock (L33): one tick per instruction begun, the step ceiling checked with it (StepLimit
            # names this word's pc); rdx and the flags are scratch at a word's start
            A.mov_r_rm(RDX, ("abs", "vd_st", 0))
            A.alu_rm_imm("add", ("r", RDX), 1)
            A.mov_rm_r(("abs", "vd_st", 0), RDX)
            A.alu_r_rm("cmp", RDX, ("abs", "vd_lim", 0))
            A.raw(0x0F, 0x86, 15, 0, 0, 0)    # jbe over the next 15 octets (mov r64, imm64 + jmp rel32)
            A.mov_r_imm64(RAX, pc)
            A.jmp("rt_vd_slim")
        d = decs[k]
        if d is None and words[k] == 0x73:
            # ecall from S-mode is an SBI call (yantra is the firmware), yantra's whole model: a7 = 1
            # putchar(a0) and a7 = 0 set_timer, each a0 = 0; every other eid halts in rt_ecall (8 Shutdown,
            # else Sbi { eid, fid }). set_timer arms yantra's timecmp, which only sip, rdtime and a taken
            # interrupt read; natively sie stays 0 (its write is refused), so none is taken (yantra's
            # interrupt(): nothing enabled, nothing taken) and sip / time reads are refused: the deadline
            # has no native reader, and arming it is a0 = 0
            get(17)
            A.alu_rm_imm("cmp", ("r", RAX), 0)
            A.jcc("e", "T%s_%x" % (V, pc))
            A.alu_rm_imm("cmp", ("r", RAX), 1)
            A.jcc("ne", "E%s_%x" % (V, pc))
            get(10, RCX)
            A.mov_r_imm64(RAX, UART)
            A.call("rt_store_slow")
            A.label("T%s_%x" % (V, pc))
            A.alu_r_rm("xor", RAX, ("r", RAX), 0)
            put(10)
            if fast:
                forget_reg(10)
            A.jmp("D%s_%x" % (V, pc))
            A.label("E%s_%x" % (V, pc))
            A.mov_rm_imm32(("abs", "fault_off", 0), 4 * k, 0)
            get(16, RCX)
            A.jmp("rt_ecall")
            A.label("D%s_%x" % (V, pc))
            return
        if d is None:
            kind = yantra_static(words[k])
            if kind == 4:                     # sfence.vma: a no-op in S-mode
                return
            A.mov_rm_imm32(("abs", "fault_off", 0), 4 * k, 0)
            A.mov_rm_imm32(("abs", "fault_word", 0), sx(words[k], 32), 0)
            A.jmp(("rt_bad_insn", "rt_unimpl", "rt_break", "rt_sret", "", "rt_csrh")[kind])
            return
        op, rd, rs1, rs2, imm = d
        if op == "fence":
            return
        if op == "csrs_sstatus" and sst_full:
            # sstatus, exactly (H8): rdx = the source; rax = yantra's read; rcx = the new value; rd = rax
            f3, r = imm & 7, imm >> 3
            if f3 < 4:
                get(r, RDX)
            else:
                A.mov_rm_imm32(("r", RDX), r, 0)
            A.mov_r_rm(RAX, ("abs", "sst", 0))
            if fp_used:                       # FS is fsen's
                A.alu_rm_imm("and", ("r", RAX), ~SSTATUS_FS)
                A.mov_r_rm(RCX, ("abs", "fsen", 0), 0)
                A.shift_imm("shl", ("r", RCX), 13)
                A.alu_r_rm("or", RAX, ("r", RCX))
            if vec_used:                      # VS is vsen's
                A.alu_rm_imm("and", ("r", RAX), ~SSTATUS_VS)
                A.mov_r_rm(RCX, ("abs", "vsen", 0), 0)
                A.shift_imm("shl", ("r", RCX), 9)
                A.alu_r_rm("or", RAX, ("r", RCX))
            A.mov_r_rm(RCX, ("r", RAX))       # SD = (FS == 3 or VS == 3) << 63: bit 13 / bit 9 of s & s >> 1
            A.shift_imm("shr", ("r", RCX), 1)
            A.alu_r_rm("and", RCX, ("r", RAX))
            A.alu_rm_imm("and", ("r", RCX), 0x2200)
            A.f7(3, ("r", RCX))               # neg: bit 63 set exactly when one was
            A.shift_imm("shr", ("r", RCX), 63)
            A.shift_imm("shl", ("r", RCX), 63)
            A.alu_r_rm("or", RAX, ("r", RCX))
            A.mov_r_imm64(RCX, SSTATUS_UXL)
            A.alu_r_rm("or", RAX, ("r", RCX))
            if f3 & 3 == 1:
                A.mov_r_rm(RCX, ("r", RDX))
            elif f3 & 3 == 2:
                A.mov_r_rm(RCX, ("r", RAX))
                A.alu_r_rm("or", RCX, ("r", RDX))
            else:
                A.mov_r_rm(RCX, ("r", RDX))
                A.f7(2, ("r", RCX))           # not
                A.alu_r_rm("and", RCX, ("r", RAX))
            A.mov_r_rm(RDX, ("r", RCX))
            A.alu_rm_imm("and", ("r", RDX), SSTATUS_MASK)
            A.mov_rm_r(("abs", "sst", 0), RDX)
            if fp_used:
                A.mov_r_rm(RDX, ("r", RCX))
                A.shift_imm("shr", ("r", RDX), 13)
                A.alu_rm_imm("and", ("r", RDX), 3, 0)
                A.mov_rm_r(("abs", "fsen", 0), RDX, width=4)
            if vec_used:
                A.mov_r_rm(RDX, ("r", RCX))
                A.shift_imm("shr", ("r", RDX), 9)
                A.alu_rm_imm("and", ("r", RDX), 3, 0)
                A.mov_rm_r(("abs", "vsen", 0), RDX, width=4)
            if rd:
                if fast:
                    forget_reg(rd)
                put(rd)
            return
        if op == "csr_sscratch":
            # sscratch (H8): rdx = the source, rax = the old value, rcx = the new one
            f3, r = imm & 7, imm >> 3
            if f3 < 4:
                get(r, RDX)
            else:
                A.mov_rm_imm32(("r", RDX), r, 0)
            A.mov_r_rm(RAX, ("abs", "sscr", 0))
            if f3 & 3 == 1:
                A.mov_rm_r(("abs", "sscr", 0), RDX)
            elif f3 & 3 == 2:
                A.mov_r_rm(RCX, ("r", RAX))
                A.alu_r_rm("or", RCX, ("r", RDX))
                A.mov_rm_r(("abs", "sscr", 0), RCX)
            else:
                A.mov_r_rm(RCX, ("r", RDX))
                A.f7(2, ("r", RCX))
                A.alu_r_rm("and", RCX, ("r", RAX))
                A.mov_rm_r(("abs", "sscr", 0), RCX)
            if rd:
                if fast:
                    forget_reg(rd)
                put(rd)
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
            if vec_used:                      # and sstatus.VS (bits 10:9) every vector instruction
                f3, r = imm & 7, imm >> 3
                if f3 in (1, 2, 3) and (f3 == 1 or r):
                    get(r)
                    A.shift_imm("shr", ("r", RAX), 9)
                    A.alu_rm_imm("and", ("r", RAX), 3, 0)
                    if f3 == 1:
                        A.mov_rm_r(("abs", "vsen", 0), RAX, width=4)
                    else:
                        A.mov_r_rm(RCX, ("abs", "vsen", 0), 0)
                        if f3 == 2:
                            A.alu_r_rm("or", RCX, ("r", RAX), 0)
                        else:
                            A.f7(2, ("r", RAX), 0)
                            A.alu_r_rm("and", RCX, ("r", RAX), 0)
                        A.mov_rm_r(("abs", "vsen", 0), RCX, width=4)
                elif f3 == 5:
                    A.mov_rm_imm32(("abs", "vsen", 0), 0, 0)
            return
        if op == "amo":
            A.mov_rm_imm32(("abs", "fault_off", 0), 4 * k, 0)
            A.mov_rm_imm32(("abs", "fault_word", 0), sx(words[k], 32), 0)
            get(rs1)
            get(rs2, RDX)
            A.call("rt_amo")
            if fast:
                slots.clear()                      # an AMO may write the stack
                forget_reg(rd)
            put(rd)
            return
        if op == "vec":
            A.mov_rm_imm32(("abs", "fault_off", 0), 4 * k, 0)
            A.mov_rm_imm32(("abs", "fault_word", 0), sx(words[k], 32), 0)
            get(rs1)
            get(rs2, RCX)
            A.call("rt_vec")
            if fast:
                slots.clear()                      # a vector store may write the stack
            put(rd)
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
            elif fj and k in fj["hints"]:
                A.call("rt_fj")                # the marker: a region may run on threads here
                if fast:
                    slots.clear()
            return
        if op == "auipc":
            if rd:
                if mapped(rd):
                    A.mov_r_imm64(M[rd], pc + imm)
                else:
                    A.mov_r_imm64(RAX, pc + imm)
                    put(rd)
            return
        if op == "jal" and imm == 0:
            # `jal rd, .`: yantra's SpinForever { pc } (H2), exit 1, the idiom a bare program parks with
            A.mov_rm_imm32(("abs", "fault_off", 0), 4 * k, 0)
            A.jmp("rt_vd_spin" if vd else "rt_spin")     # vd mode prints its own verdict line
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
                if fs_dirty:
                    A.mov_rm_imm32(("abs", "fsen", 0), 3, 0)      # FS Dirty, as yantra's mark_fp_dirty
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
                if wm:
                    wm_x86(A, M[2], imm, width, "W%s_%x" % (V, pc))
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
            if wm:
                wm_x86(A, RAX, 0, width, "W%s_%x" % (V, pc))
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
                if fs_dirty and (kind == 1 or rs1):
                    # a write that happens (csrrw always, set / clear with a non-zero mask) makes FS Dirty
                    if kind != 1 and CSR_OPS.index(op) < 3:
                        A.test_rm_r(("r", RAX), RAX)
                        A.raw(0x74, 0)
                        at = len(A.b)
                        A.mov_rm_imm32(("abs", "fsen", 0), 3, 0)
                        A.b[at - 1] = len(A.b) - at
                    else:
                        A.mov_rm_imm32(("abs", "fsen", 0), 3, 0)
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
                if imm & 7 >= 5:       # reserved (5, 6) or DYN (frm may be): the helper's rm entry names this word
                    A.mov_rm_imm32(("abs", "fault_off", 0), 4 * k, 0)
                    A.mov_rm_imm32(("abs", "fault_word", 0), sx(words[k], 32), 0)
                A.mov_rm_imm32(("r", RCX), imm & 7, 0)
            A.call("fph_%d" % FP_ID[op])
            if fd:
                A.sse(0x66, b"\x0f\xd6", 0, ("abs", "fregs", 8 * rd), 0)        # movq [fregs + 8 rd], xmm0
                if fs_dirty:
                    A.mov_rm_imm32(("abs", "fsen", 0), 3, 0)
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
                A.jcc("ae", ("XF_%x" if fj else "G_%x") % pc)
                A.label("FB_%x" % pc)
                charge(k)
            lower(k, True)
        A.mov_r_imm64(RAX, base + 4 * nwords)   # fell off the end: name the pc past the last word
        A.jmp("rt_bad_target")
    # ---------------- the SAFE region: every word, every access guarded
    vdcold = []
    for k in range(nwords):
        A.label("G_%x" % (base + 4 * k))
        if vdf:                                     # one tick of the deferred-completion clock per guest word
            A.alu_rm_imm("cmp", ("abs", "nt", NT["vwait"]), 0)
            A.jcc("ne", "VT_%x" % (base + 4 * k))
            A.label("VB_%x" % (base + 4 * k))
            vdcold.append(base + 4 * k)
        charge1(k)
        lower(k, False)
        d = decs[k]
        if fj and fast_ok and k + 1 < nwords and d and d[1] == 2 and d[0] not in BR + MEM_S:
            A.jmp("F_%x" % (base + 4 * k + 4))     # FORK-JOIN: a new sp is checked before it is used
    A.mov_r_imm64(RAX, base + 4 * nwords)
    A.jmp("rt_bad_target")
    if fj:
        # a FAST check that failed: the main thread runs the block SAFE; a worker has left its slice
        for k in range(nwords):
            if leader[k]:
                pc = base + 4 * k
                A.label("XF_%x" % pc)
                A.alu_rm_imm("cmp", ("abs", "fj_worker", 0), 0)
                A.jcc("e", "G_%x" % pc)
                A.jmp("rt_fj_stack")
        # the image's part of rt_fj (it knows which guest registers live in host registers):
        # the marker's a0..a4 and sp, and the image's stack and extent, to fj_g / fj_sbx / fj_sp0 / fj_ext
        A.label("rt_fj_args")
        for i, n in enumerate((10, 11, 12, 13, 14, 2)):
            get(n)
            A.mov_rm_r(("abs", "fj_g", 8 * i), RAX)
        for name, v in (("fj_sbx", fj["sb"] + 8 + 2048), ("fj_sp0", fj["sp0"]),
                        ("fj_ext", (base + extent + 0xFFF) & ~0xFFF)):
            A.mov_r_imm64(RAX, v)
            A.mov_rm_r(("abs", name, 0), RAX)
        A.ret()
        # a1 = rax, in the marker's saved registers (rt_fj restores them)
        A.label("rt_fj_seta1")
        if 11 in M:
            A.mov_rm_r(("abs", "fj_hsave", 8 * HOST.index(M[11])), RAX)
        else:
            A.mov_rm_r(("abs", "regs", 8 * 11), RAX)
        A.ret()
        # a worker's body call: a0 = its index, a1 = the output run, ra = the sentinel, sp = its slice
        A.label("rt_fj_wregs")
        A.mov_r_rm(RAX, ("abs", "fj_i", 0))
        put(10)
        A.mov_r_rm(RAX, ("abs", "fj_bout", 0))
        put(11)
        A.mov_rm_imm32(("r", RAX), FJ_SENTINEL)
        put(1)
        A.mov_r_rm(RAX, ("abs", "fj_sp", 0))
        put(2)
        A.ret()
    # ---------------- cold paths: a guard that failed names its site, then the runtime decides
    for V, pc, kind, width in cold:
        A.label("C%s_%x" % (V, pc))
        A.mov_rm_imm32(("abs", "fault_off", 0), pc - base, 0)
        A.mov_rm_imm32(("r", RDX), width, 0)
        if kind == "s":
            A.call("rt_store_slow")      # returns only for the UART (and the socket's NEXT / TX)
            A.jmp("B%s_%x" % (V, pc))
        else:
            A.call("rt_load_slow")       # returns only for the socket's RX: rax = the value's address
            A.jmp("B%s_%x" % (V, pc))
    # ---------------- the threads' image-dependent helpers (runtime()'s threads block): where sp lives
    # (a host register or regs[2]) and whether fcsr exists depend on the image, so they are written here
    A.label("rt_th_sp")                  # x2 = rax
    put(2)
    A.ret()
    A.label("rt_th_fget")                # rax = fcsr (0 in an image without the F/D runtime)
    if fp_used:
        A.mov_r_rm(RAX, ("abs", "fcsr", 0))
    else:
        A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.ret()
    A.label("rt_th_fput")                # fcsr = rax
    if fp_used:
        A.mov_rm_r(("abs", "fcsr", 0), RAX)
    A.ret()
    for k in sorted(stubs):                       # the limit falls in block k: its words back, then SAFE, word by word
        A.label("S_%x" % (base + 4 * k))
        A.alu_rm_imm("add", ("abs", "steps", 0), rem[k])
        if fj:
            A.alu_rm_imm("cmp", ("abs", "fj_worker", 0), 0)
            A.jcc("ne", "rt_fj_stepw")
        A.jmp("G_%x" % (base + 4 * k))
    for k in sorted(stubs1):                      # the limit is met at word k: rdx its offset
        A.label("T_%x" % (base + 4 * k))
        A.mov_rm_imm32(("r", RDX), 4 * k, 0)
        A.jmp("rt_stepcross")
    for vpc in vdcold:                  # YANTRA_VIRTIO_DEFER: one tick of the completion clock, then on
        A.label("VT_%x" % vpc)
        A.mov_rm_imm32(("abs", "fault_off", 0), vpc - base, 0)
        A.call("rt_nt_tick")
        A.jmp("VB_%x" % vpc)
    runtime(A, M, base, entry, extent, len(span), code_size, prot_size, mac, fj, wm, net, vd, scanp, vdf)
    if fp_used:
        runtime_fp(A, os.environ.get("ANUVADA_SOFT_ALL") == "1", mac, os.environ.get("ANUVADA_NO_FMA3") == "1")
    if vec_used:
        runtime_vec(A, wm)
    if amo_used:
        runtime_amo(A, wm)
    if csrh_used:
        runtime_csrh(A, base, mac)
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
    for name, size in DLIST:
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


def runtime_csrh(A, base, mac=False):
    """yantra's halt for a CSR it does not implement (H5): `halt: Csr { pc: P, csr: C, write: W }`, status 1.
    fault_off and fault_word name the site; W is yantra's rule (csrrw/csrrwi, or an rs1 field that is not 0).
    Only in an image that has such a word; the text goes out through rt_puts (rsi, rdx: write(2, ...))."""
    A.label("rt_csrh")
    A.call("rt_flush")
    A.lea(RSI, ("abs", "csrh_s1", 0))
    A.mov_rm_imm32(("r", RDX), len(CSRH_TEXT[0]))
    A.call("rt_puts")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.lea(RSI, ("abs", "csrh_s2", 0))
    A.mov_rm_imm32(("r", RDX), len(CSRH_TEXT[1]))
    A.call("rt_puts")
    A.mov_r_rm(RAX, ("abs", "fault_word", 0), 0)
    A.shift_imm("shr", ("r", RAX), 20, 0)
    A.call("rt_putu")
    A.mov_r_rm(RAX, ("abs", "fault_word", 0), 0)
    A.lea(RSI, ("abs", "csrh_s3", 0))                  # write: true
    A.mov_rm_imm32(("r", RDX), len(CSRH_TEXT[2]))
    A.mov_r_rm(RCX, ("r", RAX), 0)
    A.shift_imm("shr", ("r", RCX), 15, 0)
    A.alu_rm_imm("and", ("r", RCX), 31, 0)
    A.jcc("ne", "rt_csrh_w")
    A.shift_imm("shr", ("r", RAX), 12, 0)
    A.alu_rm_imm("and", ("r", RAX), 3, 0)
    A.alu_rm_imm("cmp", ("r", RAX), 1, 0)
    A.jcc("e", "rt_csrh_w")
    A.lea(RSI, ("abs", "csrh_s4", 0))                  # write: false
    A.mov_rm_imm32(("r", RDX), len(CSRH_TEXT[3]))
    A.label("rt_csrh_w")
    A.call("rt_puts")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_puts")
    A.mov_rm_imm32(("r", RDI), 2)
    x86_sys(A, 1, mac)
    A.ret()
    for i, t in enumerate(CSRH_TEXT):
        A.label("csrh_s%d" % (i + 1))
        A.raw(t)


CSRH_TEXT = (b"halt: Csr { pc: ", b", csr: ", b", write: true }\n", b", write: false }\n")


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
# every status word by name (patra.rs Status::word, then runtime-completeness-design.md §5)
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
    base = Wide(base) if isinstance(base, int) else base
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
    # header near 2^64 would wrap the scratch size to one page (5f B1, reproduced as SIGSEGV)
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


# ---- spawn (agent/rtc-spawn): PARAM (0x20) is remembered like PATH and BUFFER; the store to
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
        # process's, never its child's (design §3; 5f M1). The child's own copy of envp is
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
    # the child's environment: this process's, without the YANTRA_ entries (M1; the Unix spawns
    # drop them too). GetEnvironmentStringsW's block is walked once for its size, then copied
    # into a VirtualAlloc'd block (at [rsp+88]) that is freed after CreateProcessW.
    def wch(d):                                        # movzx eax, word [rsi+d]
        A.raw(0x0F, 0xB7, 0x46, d)
    imp("GetEnvironmentStringsW")
    A.test_rm_r(("r", RAX), RAX, 0)
    A.jcc("e", "w_sp_fail")
    A.mov_r_rm(RSI, ("r", RAX))
    A.mov_r_rm(RBX, ("r", RAX))
    A.label("w_se_n")
    wch(0)
    A.test_rm_r(("r", RAX), RAX, 0)
    A.jcc("e", "w_se_sized")
    A.label("w_se_n1")
    wch(0)
    A.alu_rm_imm("add", ("r", RSI), 2)
    A.test_rm_r(("r", RAX), RAX, 0)
    A.jcc("ne", "w_se_n1")
    A.jmp("w_se_n")
    A.label("w_se_sized")
    A.mov_r_rm(RDX, ("r", RSI))
    A.alu_r_rm("sub", RDX, ("r", RBX))
    A.alu_rm_imm("add", ("r", RDX), 8)
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.mov_rm_imm32(("r", R8), 0x3000)                  # MEM_COMMIT | MEM_RESERVE
    A.mov_rm_imm32(("r", R9), 4)                       # PAGE_READWRITE
    imp("VirtualAlloc")
    A.test_rm_r(("r", RAX), RAX, 0)
    A.jcc("e", "w_sp_fail")
    A.mov_rm_r(("m", RSP, 88), RAX)
    A.mov_r_rm(RDI, ("r", RAX))
    A.mov_r_rm(RSI, ("r", RBX))
    A.label("w_se_c")
    wch(0)
    A.test_rm_r(("r", RAX), RAX, 0)
    A.jcc("e", "w_se_end")
    for i, c in enumerate("YANTRA_"):                  # stops at the entry's NUL: never reads past it
        wch(2 * i)
        A.alu_rm_imm("cmp", ("r", RAX), ord(c), 0)
        A.jcc("ne", "w_se_keep")
    A.label("w_se_skip")
    wch(0)
    A.alu_rm_imm("add", ("r", RSI), 2)
    A.test_rm_r(("r", RAX), RAX, 0)
    A.jcc("ne", "w_se_skip")
    A.jmp("w_se_c")
    A.label("w_se_keep")
    wch(0)
    A.raw(0x66, 0x89, 0x07)                            # mov [rdi], ax
    A.alu_rm_imm("add", ("r", RSI), 2)
    A.alu_rm_imm("add", ("r", RDI), 2)
    A.test_rm_r(("r", RAX), RAX, 0)
    A.jcc("ne", "w_se_keep")
    A.jmp("w_se_c")
    A.label("w_se_end")
    A.mov_rm_imm32(("m", RDI, 0), 0, 0)                # two NULs: an empty block is valid too
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
    arg(5, ("i", 0x400))                               # CREATE_UNICODE_ENVIRONMENT
    A.mov_r_rm(RAX, ("m", RSP, 88))
    arg(6, RAX)                                        # the filtered environment
    arg(7, ("i", 0))
    A.lea(RAX, S("si"))
    arg(8, RAX)
    A.lea(RAX, S("pi0"))
    arg(9, RAX)
    imp("CreateProcessW")
    A.mov_r_rm(RBX, ("r", RAX))
    A.mov_r_rm(RCX, ("m", RSP, 88))
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.mov_rm_imm32(("r", R8), 0x8000)                  # MEM_RELEASE
    imp("VirtualFree")
    A.test_rm_r(("r", RBX), RBX, 0)
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


VD_PFX = (("fin", b"Finisher { value: ", "rt_vd_isfin"), ("wt", b"Wait", None), ("sl", b"StepLimit", None),
          ("sp", b"SpinForever", None))
VD_LITS = [b'fault ', b'\n', b'finisher ', b' ', b'-', b'steps ', b'verdict: ', b': ', b'Unknown error ', b' (os error ', b')\n', b'steps: ', b' executed instructions\n', b'No such file or directory', b'Permission denied', b'Is a directory', b'Not a directory', b'Read-only file system', b'No space left on device', b'File name too long', b'wait\n', b'steplimit\n', b'spin\n']
VD_LABEL = {t: "vd_l%d" % (k + 1) for k, t in enumerate(VD_LITS)}


def runtime_vd(A):
    """L33 YANTRA_VERDICT, x86-64 Linux: the exit-funnel routine and its helpers (recorded verbatim by rt2t1.py)."""
    # ---- L33 YANTRA_VERDICT. stderr was captured from the start (a memfd under fd 2); this runs once, at the
    # exit funnel, with rdi = the exit code (kept): replay the capture, putting yantra's own `steps:` line right
    # after the halt line, and write the verdict file. r12 = the end of the halt line, r13 = the end of the
    # capture, r14 = the output cursor, r15 = the newline of the halt line
    def lit(text):
        A.lea(RSI, ("abs", VD_LABEL[text], 0))
        A.mov_rm_imm32(("r", RCX), len(text))
        A.call("rt_vd_cpy")

    def sys_(n):
        A.mov_rm_imm32(("r", RAX), n)
        A.syscall()

    def vd_at(label):                       # rsi = the text of the halt line (after `halt: `)
        A.lea(RSI, ("abs", "vd_buf", 0))
        A.alu_rm_imm("add", ("r", RSI), 6)

    A.label("rt_vd_fin")
    A.alu_rm_imm("cmp", ("abs", "vd_fd", 0), 0)
    A.jcc("e", "rt_vd_ret")
    A.push(RDI)
    A.mov_r_rm(RDI, ("abs", "vd_fd", 0))
    A.lea(RSI, ("abs", "vd_buf", 0))
    A.mov_rm_imm32(("r", RDX), VD_BUF)
    A.alu_r_rm("xor", R10, ("r", R10), 0)
    sys_(17)                                # pread64(fd, buf, size, 0)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ns", "rt_vd_f1")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.label("rt_vd_f1")
    A.lea(R13, ("abs", "vd_buf", 0))
    A.alu_r_rm("add", R13, ("r", RAX))
    # leading `args:` lines (the argument report, before the run) are not part of the halt verdict: replay them
    # to the real stderr, then re-read the capture past them, so the halt line leads what is parsed
    A.alu_r_rm("xor", R12, ("r", R12), 0)
    A.label("rt_vd_sk")
    A.lea(RSI, ("abs", "vd_buf", 0))
    A.mov_r_rm(RAX, ("r", R13))
    A.alu_r_rm("sub", RAX, ("r", RSI))
    A.alu_rm_imm("cmp", ("r", RAX), 5)
    A.jcc("l", "rt_vd_sk9")
    A.alu_rm_imm("cmp", ("m", RSI, 0), 0x73677261, 0)
    A.jcc("ne", "rt_vd_sk9")
    A.movzx(RAX, ("m", RSI, 4), 1)
    A.alu_rm_imm("cmp", ("r", RAX), 0x3a, 0)
    A.jcc("ne", "rt_vd_sk9")
    A.mov_r_rm(R15, ("r", RSI))
    A.label("rt_vd_sk1")
    A.alu_r_rm("cmp", R15, ("r", R13))
    A.jcc("ae", "rt_vd_sk9")
    A.movzx(RAX, ("m", R15, 0), 1)
    A.alu_rm_imm("cmp", ("r", RAX), 10, 0)
    A.jcc("e", "rt_vd_sk2")
    A.alu_rm_imm("add", ("r", R15), 1)
    A.jmp("rt_vd_sk1")
    A.label("rt_vd_sk2")
    A.alu_rm_imm("add", ("r", R15), 1)
    A.mov_r_rm(RDI, ("abs", "vd_sv", 0))
    A.lea(RSI, ("abs", "vd_buf", 0))
    A.mov_r_rm(RDX, ("r", R15))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    A.alu_r_rm("add", R12, ("r", RDX))
    sys_(1)                                 # write(real stderr, the args line)
    A.mov_r_rm(RDI, ("abs", "vd_fd", 0))
    A.lea(RSI, ("abs", "vd_buf", 0))
    A.mov_rm_imm32(("r", RDX), VD_BUF)
    A.mov_r_rm(R10, ("r", R12))
    sys_(17)                                # pread64(fd, buf, size, past the args lines)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ns", "rt_vd_sk3")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.label("rt_vd_sk3")
    A.lea(R13, ("abs", "vd_buf", 0))
    A.alu_r_rm("add", R13, ("r", RAX))
    A.jmp("rt_vd_sk")
    A.label("rt_vd_sk9")
    A.mov_r_rm(RDI, ("abs", "vd_fd", 0))
    sys_(3)                                 # close the capture
    A.mov_r_rm(RDI, ("abs", "vd_sv", 0))
    A.mov_rm_imm32(("r", RSI), 2)
    sys_(33)                                # dup2(real stderr, 2)
    A.mov_r_rm(RDI, ("abs", "vd_sv", 0))
    sys_(3)
    A.mov_rm_imm32(("abs", "vd_fd", 0), 0)
    # a halt line leads the capture, else it is a refusal yantra would not have made: replay, no verdict
    A.lea(RSI, ("abs", "vd_buf", 0))
    A.mov_r_rm(RAX, ("r", R13))
    A.alu_r_rm("sub", RAX, ("r", RSI))
    A.alu_rm_imm("cmp", ("r", RAX), 6)
    A.jcc("l", "rt_vd_raw")
    A.alu_rm_imm("cmp", ("m", RSI, 0), 0x746c6168, 0)
    A.jcc("ne", "rt_vd_raw")
    A.movzx(RAX, ("m", RSI, 4), 2)
    A.alu_rm_imm("cmp", ("r", RAX), 0x203a, 0)
    A.jcc("ne", "rt_vd_raw")
    A.alu_rm_imm("add", ("r", RSI), 6)
    A.mov_r_rm(R15, ("r", RSI))
    A.label("rt_vd_nl")
    A.alu_r_rm("cmp", R15, ("r", R13))
    A.jcc("ae", "rt_vd_nl2")
    A.movzx(RAX, ("m", R15, 0), 1)
    A.alu_rm_imm("cmp", ("r", RAX), 10, 0)
    A.jcc("e", "rt_vd_nl2")
    A.alu_rm_imm("add", ("r", R15), 1)
    A.jmp("rt_vd_nl")
    A.label("rt_vd_nl2")
    A.lea(R12, ("m", R15, 1))
    A.alu_r_rm("cmp", R12, ("r", R13))
    A.jcc("be", "rt_vd_nl3")
    A.mov_r_rm(R12, ("r", R13))
    A.label("rt_vd_nl3")
    A.mov_rm_imm32(("r", RDI), 2)           # the halt line, as it stands
    A.lea(RSI, ("abs", "vd_buf", 0))
    A.mov_r_rm(RDX, ("r", R12))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    sys_(1)
    # <first>
    A.lea(R14, ("abs", "vd_out", 0))
    for name, text, jl in VD_PFX:
        vd_at(name)
        A.lea(R8, ("abs", "vd_p" + name, 0))
        A.mov_rm_imm32(("r", RCX), len(text))
        A.call("rt_vd_pfx")
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("ne", jl or "rt_vd_is" + name)
    lit(b"fault ")
    vd_at("flt")
    A.mov_r_rm(RCX, ("r", R15))
    A.alu_r_rm("sub", RCX, ("r", RSI))
    A.call("rt_vd_cpy")
    lit(b"\n")
    A.jmp("rt_vd_steps")
    for name, word in (("wt", b"wait\n"), ("sl", b"steplimit\n"), ("sp", b"spin\n")):
        A.label("rt_vd_is" + name)
        lit(word)
        A.jmp("rt_vd_steps")
    A.label("rt_vd_isfin")
    lit(b"finisher ")
    vd_at("fin")
    A.alu_rm_imm("add", ("r", RSI), 18)
    A.call("rt_vd_dig")
    A.push(RSI)
    lit(b" ")
    A.pop(RSI)
    A.alu_rm_imm("add", ("r", RSI), 10)     # `, status: `
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.alu_rm_imm("cmp", ("r", RAX), 78, 0)  # N: None
    A.jcc("ne", "rt_vd_some")
    lit(b"-")
    A.jmp("rt_vd_fnl")
    A.label("rt_vd_some")
    A.alu_rm_imm("add", ("r", RSI), 5)      # `Some(`
    A.call("rt_vd_dig")
    A.label("rt_vd_fnl")
    lit(b"\n")
    A.label("rt_vd_steps")
    lit(b"steps ")
    A.mov_r_rm(RAX, ("abs", "vd_st", 0))
    A.call("rt_vd_dec")
    lit(b"\n")
    A.lea(RDI, ("abs", "env_vd", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_vd_sl")
    A.mov_r_rm(R15, ("r", RAX))             # the path
    A.mov_r_rm(RDI, ("r", R15))
    A.mov_rm_imm32(("r", RSI), 0x241)       # O_WRONLY | O_CREAT | O_TRUNC
    A.mov_rm_imm32(("r", RDX), 438)         # 0666
    sys_(2)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_vd_err")
    A.mov_r_rm(R9, ("r", RAX))
    A.mov_r_rm(RDI, ("r", R9))
    A.lea(RSI, ("abs", "vd_out", 0))
    A.mov_r_rm(RDX, ("r", R14))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    sys_(1)
    A.mov_r_rm(R10, ("r", RAX))
    A.mov_r_rm(RDI, ("r", R9))
    sys_(3)
    A.mov_r_rm(RAX, ("r", R10))
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ns", "rt_vd_sl")
    A.label("rt_vd_err")                    # `verdict: <path>: <strerror> (os error N)`, then exit 1 at once
    A.alu_r_rm("xor", R10, ("r", R10), 0)
    A.alu_r_rm("sub", R10, ("r", RAX))
    A.lea(R14, ("abs", "vd_out", 0))
    lit(b"verdict: ")
    A.mov_r_rm(RSI, ("r", R15))
    A.call("rt_vd_cz")
    lit(b": ")
    for no, text in ((2, b"No such file or directory"), (13, b"Permission denied"), (21, b"Is a directory"),
                     (20, b"Not a directory"), (30, b"Read-only file system"),
                     (28, b"No space left on device"), (36, b"File name too long")):
        A.alu_rm_imm("cmp", ("r", R10), no)
        A.jcc("ne", "rt_vd_en%d" % no)
        lit(text)
        A.jmp("rt_vd_e2")
        A.label("rt_vd_en%d" % no)
    lit(b"Unknown error ")
    A.mov_r_rm(RAX, ("r", R10))
    A.call("rt_vd_dec")
    A.label("rt_vd_e2")
    lit(b" (os error ")
    A.mov_r_rm(RAX, ("r", R10))
    A.call("rt_vd_dec")
    lit(b")\n")
    A.mov_rm_imm32(("r", RDI), 2)
    A.lea(RSI, ("abs", "vd_out", 0))
    A.mov_r_rm(RDX, ("r", R14))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    sys_(1)
    A.mov_rm_imm32(("r", RDI), 1)
    sys_(60)
    A.label("rt_vd_sl")                     # steps: N executed instructions
    A.lea(R14, ("abs", "vd_out", 0))
    lit(b"steps: ")
    A.mov_r_rm(RAX, ("abs", "vd_st", 0))
    A.call("rt_vd_dec")
    lit(b" executed instructions\n")
    A.mov_rm_imm32(("r", RDI), 2)
    A.lea(RSI, ("abs", "vd_out", 0))
    A.mov_r_rm(RDX, ("r", R14))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    sys_(1)
    A.mov_rm_imm32(("r", RDI), 2)           # whatever followed the halt line
    A.mov_r_rm(RSI, ("r", R12))
    A.mov_r_rm(RDX, ("r", R13))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    sys_(1)
    A.jmp("rt_vd_done")
    A.label("rt_vd_raw")
    A.mov_rm_imm32(("r", RDI), 2)
    A.lea(RSI, ("abs", "vd_buf", 0))
    A.mov_r_rm(RDX, ("r", R13))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    sys_(1)
    A.label("rt_vd_done")
    A.pop(RDI)
    A.label("rt_vd_ret")
    A.ret()
    # rsi = source, rcx = octets, r14 = destination (advanced)
    A.label("rt_vd_cpy")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("e", "rt_vd_cpy2")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.mov_rm_r(("m", R14, 0), RAX, width=1)
    A.alu_rm_imm("add", ("r", RSI), 1)
    A.alu_rm_imm("add", ("r", R14), 1)
    A.alu_rm_imm("sub", ("r", RCX), 1)
    A.jmp("rt_vd_cpy")
    A.label("rt_vd_cpy2")
    A.ret()
    # rsi = a zero-ended string, copied to r14
    A.label("rt_vd_cz")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_vd_cz2")
    A.mov_rm_r(("m", R14, 0), RAX, width=1)
    A.alu_rm_imm("add", ("r", RSI), 1)
    A.alu_rm_imm("add", ("r", R14), 1)
    A.jmp("rt_vd_cz")
    A.label("rt_vd_cz2")
    A.ret()
    # rsi = text, r8 = literal, rcx = its length: rax = 1 when the text begins with it (rsi, rcx kept)
    A.label("rt_vd_pfx")
    A.push(RSI)
    A.push(RCX)
    A.label("rt_vd_pf1")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("e", "rt_vd_pf2")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.movzx(RDX, ("m", R8, 0), 1)
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    A.jcc("ne", "rt_vd_pf3")
    A.alu_rm_imm("add", ("r", RSI), 1)
    A.alu_rm_imm("add", ("r", R8), 1)
    A.alu_rm_imm("sub", ("r", RCX), 1)
    A.jmp("rt_vd_pf1")
    A.label("rt_vd_pf2")
    A.mov_rm_imm32(("r", RAX), 1)
    A.jmp("rt_vd_pf4")
    A.label("rt_vd_pf3")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.label("rt_vd_pf4")
    A.pop(RCX)
    A.pop(RSI)
    A.ret()
    # the decimal digits at rsi to r14 (rsi advanced)
    A.label("rt_vd_dig")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.lea(RDX, ("m", RAX, -48))
    A.alu_rm_imm("cmp", ("r", RDX), 9)
    A.jcc("a", "rt_vd_dig2")
    A.mov_rm_r(("m", R14, 0), RAX, width=1)
    A.alu_rm_imm("add", ("r", RSI), 1)
    A.alu_rm_imm("add", ("r", R14), 1)
    A.jmp("rt_vd_dig")
    A.label("rt_vd_dig2")
    A.ret()
    # rax as decimal digits to r14 (advanced)
    A.label("rt_vd_dec")
    A.lea(RSI, ("abs", "num", 31))
    A.mov_rm_imm32(("r", RCX), 10)
    A.label("rt_vd_dc1")
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.f7(6, ("r", RCX))
    A.lea(RDX, ("m", RDX, 48))
    A.lea(RSI, ("m", RSI, -1))
    A.raw(0x88, 0x16)                       # mov [rsi], dl
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_vd_dc1")
    A.lea(RCX, ("abs", "num", 31))
    A.alu_r_rm("sub", RCX, ("r", RSI))
    A.jmp("rt_vd_cpy")
    # StepLimit: the ceiling was passed at pc = rax; yantra's clock reads the limit
    A.label("rt_vd_slim")
    A.mov_r_rm(RCX, ("abs", "vd_lim", 0))
    A.mov_rm_r(("abs", "vd_st", 0), RCX)
    A.mov_rm_r(("abs", "fault_addr", 0), RAX)
    A.call("rt_flush")
    A.call("m_vsl")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.call("rt_putu")
    A.call("m_un4")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    # SpinForever: a `jal` to its own pc (fault_off names it): exit 1
    A.label("rt_vd_spin")
    A.call("rt_flush")
    A.call("m_vsp")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, IMAGE_BASE)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("m_un4")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")


def runtime(A, M, base, entry, extent, filesz, code_size, prot_size, mac=False, fj=None, wm=False, net=False, vd=False, scanp=None, vdf=False):
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
        A.jmp("rt_ewr")                         # stderr, or a thread's halt held back (the threads block)
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
    if fj:
        # SASSEMBLY_NATIVE_THREADS=N: each fork-join region on N threads (0: serial); unset: the CPUs this
        # process may run on (sched_getaffinity), at most FJ_MAX
        A.lea(RDI, ("abs", "env_fj", 0))
        A.call("rt_getenv")
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "rt_s1fc")
        A.mov_r_rm(RSI, ("r", RAX))
        A.call("rt_atou")
        A.jmp("rt_s1fm")
        A.label("rt_s1fc")
        A.alu_r_rm("xor", RDI, ("r", RDI), 0)
        A.mov_rm_imm32(("r", RSI), 1024)
        A.lea(RDX, ("abs", "fj_cap", 0))
        A.mov_rm_imm32(("r", RAX), 204)
        A.syscall()
        A.mov_r_rm(R9, ("r", RAX))                  # octets of the mask (or a negative error: 0 CPUs)
        A.lea(RSI, ("abs", "fj_cap", 0))
        A.alu_r_rm("xor", RAX, ("r", RAX), 0)
        A.label("rt_s1fb")
        A.alu_rm_imm("cmp", ("r", R9), 0)
        A.jcc("le", "rt_s1fm")
        A.movzx(R8, ("m", RSI, 0), 1)
        A.label("rt_s1fp")
        A.test_rm_r(("r", R8), R8)
        A.jcc("e", "rt_s1fq")
        A.lea(RCX, ("m", R8, -1))
        A.alu_r_rm("and", R8, ("r", RCX))
        A.lea(RAX, ("m", RAX, 1))
        A.jmp("rt_s1fp")
        A.label("rt_s1fq")
        A.lea(RSI, ("m", RSI, 1))
        A.alu_rm_imm("sub", ("r", R9), 1)
        A.jmp("rt_s1fb")
        A.label("rt_s1fm")
        A.alu_rm_imm("cmp", ("r", RAX), FJ_MAX)
        A.jcc("be", "rt_s1fn")
        A.mov_rm_imm32(("r", RAX), FJ_MAX)
        A.label("rt_s1fn")
        A.mov_rm_r(("abs", "fj_n", 0), RAX)
        # SASSEMBLY_NATIVE_FJ_MIN=K: the small-region threshold (default FJ_K)
        A.mov_rm_imm32(("abs", "fj_k", 0), FJ_K)
        A.lea(RDI, ("abs", "env_fjk", 0))
        A.call("rt_getenv")
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "rt_s1fj")
        A.mov_r_rm(RSI, ("r", RAX))
        A.call("rt_atou")
        A.mov_rm_r(("abs", "fj_k", 0), RAX)
        A.label("rt_s1fj")
        # SASSEMBLY_NATIVE_FJATTEST=<path>: the attestation file (S1); unset = 0 = nothing written
        A.lea(RDI, ("abs", "env_fja", 0))
        A.call("rt_getenv")
        A.mov_rm_r(("abs", "fj_att", 0), RAX)
    if vd:
        # L33 YANTRA_VERDICT: the step ceiling (YANTRA_STEPS, an unparsable value is the default, as yantra-run) and
        # stderr into a memfd (dup of the real one kept), so rt_vd_fin can place the `steps:` line and read the halt
        A.mov_r_imm64(RAX, VD_STEPS)
        A.mov_rm_r(("abs", "vd_lim", 0), RAX)
        A.lea(RDI, ("abs", "env_steps", 0))
        A.call("rt_getenv")
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "rt_vd_s1")
        A.mov_r_rm(RSI, ("r", RAX))
        A.call("rt_atou_ram")
        A.test_rm_r(("r", RCX), RCX)
        A.jcc("ne", "rt_vd_s1")
        A.mov_rm_r(("abs", "vd_lim", 0), RAX)
        A.label("rt_vd_s1")
        A.lea(RDI, ("abs", "vd_nm", 0))
        A.alu_r_rm("xor", RSI, ("r", RSI), 0)
        sys_(319)                               # memfd_create("vd", 0)
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("s", "rt_vd_s3")
        A.mov_rm_r(("abs", "vd_fd", 0), RAX)
        A.mov_rm_imm32(("r", RDI), 2)
        sys_(32)                                # dup(2)
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("s", "rt_vd_s2")
        A.mov_rm_r(("abs", "vd_sv", 0), RAX)
        A.mov_r_rm(RDI, ("abs", "vd_fd", 0))
        A.mov_rm_imm32(("r", RSI), 2)
        sys_(33)                                # dup2(memfd, 2)
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("ns", "rt_vd_s3")
        A.label("rt_vd_s2")
        A.mov_rm_imm32(("abs", "vd_fd", 0), 0)   # no capture: no steps line, no verdict
        A.label("rt_vd_s3")
    # a RAM no host can map (2^47 and up; the sizes added to it below would wrap): refused as
    # unmappable, which t1-run reruns under yantra-run (whose allocator then answers, 8f low)
    A.mov_r_rm(RAX, ("abs", "ram", 0))
    A.shift_imm("shr", ("r", RAX), 47)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_no_ram")
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
    if win:
        A.jmp("rt_win_ev")                                  # the clock is not ported to Windows
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
    # YANTRA_REPLAY_EVENTS=<file>: an --events replay's records, 8 octets each, little-endian (t1-run
    # writes them from a log it has checked as yantra-run does); rep_fd = fd + 1. A wait then reads
    # the next record in place of the clock, sleeps for no hint, and logs it to rec_fd
    A.lea(RDI, ("abs", "env_rep", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_s1c")
    if win:
        A.jmp("rt_win_ev")
    A.mov_r_rm(RDI, ("r", RAX))
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)                 # O_RDONLY
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    sys_(2)
    if mac:
        A.jcc("b", "rt_no_rep")
    else:
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("s", "rt_no_rep")
    A.lea(RDI, ("m", RAX, 1))
    A.mov_rm_r(("abs", "rep_fd", 0), RDI)
    A.label("rt_s1c")
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
    A.alu_rm_imm("add", ("r", RSI), 64)
    A.call("rt_th_pre")                           # SASTHRDS: room for the stacks and the contexts
    A.alu_rm_imm("add", ("r", RSI), 0xFFF)
    A.alu_rm_imm("and", ("r", RSI), -0x1000)
    A.mov_rm_r(("abs", "th_len", 0), RSI)
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
    A.call("rt_th_detect")                        # SASTHRDS: checked, stacks made, BEFORE the input (yantra's order)
    A.mov_r_rm(RAX, ("abs", "in_fd", 0))
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_s3")
    for k, tag in enumerate((0x5455_504E_4953_4153, 0x454D_414E_4953_4153, 0x4543_4152_5453_4153)):
        A.mov_r_imm64(RAX, tag)
        A.call("rt_find_tag")
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "rt_nt%d" % k if win else "rt_no_tag")
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
    A.jcc("e", "rt_s5v")
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
    # the args report (success line too) follows the replay line: printed at rt_s5r
    A.jmp("rt_s5")
    # a tag missing: yantra-run's refusal line, and the run goes on; a tag twice: not native
    A.label("rt_s5p")
    A.pop(RAX)
    A.alu_rm_imm("cmp", ("r", R8), 1)
    A.jcc("a", "rt_argsdup")
    A.jmp("rt_s5")
    A.label("rt_s5v")
    A.alu_rm_imm("cmp", ("r", R8), 1)
    A.jcc("a", "rt_argsdup")
    A.label("rt_s5")
    # the event slot: the SASEVENT tag's address if it occurs exactly once (0 otherwise)
    A.mov_r_imm64(RAX, EVENT_TAG)
    A.call("rt_find_tag")
    A.mov_rm_r(("abs", "ev_slot", 0), RAX)
    # AN --events REPLAY (YANTRA_REPLAY_EVENTS) names the slot it delivers into, as an offset in RAM
    # (t1-run prints yantra-run's load line from it); with the tag missing or duplicated it is
    # refused HERE, before anything runs, and t1-run reruns the image under yantra-run, which
    # refuses it at load as it refuses such a log
    A.alu_rm_imm("cmp", ("abs", "rep_fd", 0), 0)
    A.jcc("e", "rt_s5r")
    A.alu_rm_imm("cmp", ("abs", "ev_slot", 0), 0)
    A.jcc("e", "rt_rep_notag")
    A.call("m_reptag")
    A.mov_r_rm(RAX, ("abs", "ev_slot", 0))
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("sub", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("m_nl")
    A.label("rt_s5r")
    # yantra-run's refusal line for a missing argument tag comes AFTER the replay line (its load
    # line "events:" precedes "args:"), so it is printed here, not where the tags are looked up;
    # a tag found twice was refused above, so absent here means absent
    A.mov_r_imm64(RAX, 0x0056_4752_4153_4153)
    A.call("rt_find_tag")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_s5a")
    A.call("m_noargv")
    A.jmp("rt_s5b")
    A.label("rt_s5a")
    A.push(RAX)                                 # the argv tag
    A.mov_r_imm64(RAX, 0x0043_4752_4153_4153)
    A.call("rt_find_tag")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_s5c")
    A.pop(RAX)
    A.call("m_noargc")
    A.jmp("rt_s5b")
    A.label("rt_s5c")
    # both tags: yantra-run's line: args: N handed over, tags at [argv, argc], run at 0x…
    A.mov_r_rm(RCX, ("r", RAX))
    A.pop(RDX)
    A.push(RCX)
    A.push(RDX)
    A.call("m_args1")
    A.mov_r_rm(RAX, ("abs", "argc", 0))
    A.call("rt_putu")
    A.call("m_args2")
    A.mov_r_rm(RAX, ("m", RSP, 0))
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("sub", RAX, ("r", RCX))
    A.call("rt_puthexn")
    A.call("m_args3")
    A.mov_r_rm(RAX, ("m", RSP, 8))
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("sub", RAX, ("r", RCX))
    A.call("rt_puthexn")
    A.call("m_args4")
    A.pop(RDX)
    A.pop(RCX)
    A.mov_r_rm(RAX, ("m", RDX, 8))
    A.call("rt_puthex")
    A.call("m_nl")
    A.label("rt_s5b")
    if not win:
        A.call("rt_sk_init")                    # YANTRA_LISTEN: the socket's listener (runtime_sock)
    if net:
        A.call("rt_nt_init")                    # YANTRA_NET_PEER: the frames device's peer (runtime_net)
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
    # the step ceiling: yantra's DEFAULT_STEPS, or YANTRA_STEPS when it is a decimal usize (as yantra-run reads it)
    A.mov_rm_imm32(("abs", "steps", 0), 1000000)
    A.lea(RDI, ("abs", "env_steps", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_stp")
    A.mov_r_rm(RSI, ("r", RAX))
    A.call("rt_atou_ram")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("ne", "rt_stp")
    A.mov_rm_r(("abs", "steps", 0), RAX)
    A.label("rt_stp")
    A.mov_r_rm(RAX, ("abs", "steps", 0))
    A.mov_rm_r(("abs", "steps0", 0), RAX)   # the effective limit, for the StepLimit report
    # the guest: every register zero, pc = entry; a threaded image's first decision picks thread 0
    A.alu_rm_imm("cmp", ("abs", "th_n", 0), 0)
    A.jcc("ne", "rt_th_next")
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
    # a target in the middle of a block runs in the SAFE region (G_), which charges word by word
    A.label("rt_ind_go")
    A.jmp_rm(("sib2", RDX, "jt"))

    # ---- the step ceiling (L18/H1): rdx = the offset of the word the limit is met at (the first not executed)
    A.label("rt_stepcross")
    if fj:
        A.alu_rm_imm("cmp", ("abs", "fj_worker", 0), 0)
        A.jcc("ne", "rt_fj_stepw")
    A.mov_rm_r(("abs", "fault_off", 0), RDX)
    A.call("rt_flush")
    A.call("m_steplim")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("m_un4")
    A.call("m_steps1")
    A.mov_r_rm(RAX, ("abs", "steps0", 0))
    A.call("rt_putu")
    A.call("m_steps1b")
    A.call("m_stop1")
    A.mov_r_rm(RAX, ("abs", "steps0", 0))
    A.call("rt_putu")
    A.call("m_stop1b")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_puthex")
    A.call("m_stop2")
    A.mov_r_rm(RAX, ("abs", "steps0", 0))
    A.call("rt_putu")
    A.call("m_stop2b")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")

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
    A.movzx(RCX, ("r", RCX), 1)                    # a newline flushes: yantra-run's stdout is line-buffered (K1 H12)
    A.alu_rm_imm("cmp", ("r", RCX), 10)
    A.jcc("e", "rt_st_f")
    A.alu_rm_imm("cmp", ("r", RDX), 65536)
    A.jcc("b", "rt_st1")
    A.label("rt_st_f")
    if vdf:                                     # YANTRA_VIRTIO_DEFER: a full UART buffer is no halt (no settle)
        A.call("rt_flush_u")
    else:
        A.call("rt_flush")
    A.label("rt_st1")
    A.ret()
    A.label("rt_st2")
    if net:                                        # the width (edx) survives for slot 2's stores
        A.push(RDX)
    A.mov_rm_imm32(("r", RDX), WAIT)
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    if net:
        A.pop(RDX)
    A.jcc("ne", "rt_st3")
    A.jmp("rt_wait")                               # returns to the cold stub's caller
    A.label("rt_st3")
    if fj:                                 # a worker: the socket is a main-thread device
        A.alu_rm_imm("cmp", ("abs", "fj_worker", 0), 0)
        A.jcc("ne", "rt_st3s")
    A.push(RAX)
    A.alu_rm_imm("sub", ("r", RAX), 0x1000_0110)   # SOCK..SOCK+16: the socket (runtime_sock)
    A.alu_rm_imm("cmp", ("r", RAX), 16)
    A.pop(RAX)
    A.jcc("b", "rt_sk_st")
    if fj:
        A.label("rt_st3s")
    if scanp:                                  # slot 0: the GPU (scanout_dev.py), at 0x1000_1000 + 0x1000
        A.push(RAX)
        A.alu_rm_imm("sub", ("r", RAX), 0x1000_1000)
        A.alu_rm_imm("cmp", ("r", RAX), 0x1000)
        A.pop(RAX)
        A.jcc("b", "rt_sc_st")
    if net:                                    # slot 2: the frames device, when its peer is up
        A.alu_rm_imm("cmp", ("abs", "nt", NT["on"]), 0)
        A.jcc("e", "rt_st3n")
        A.push(RAX)
        A.alu_rm_imm("sub", ("r", RAX), NET_LO)
        A.alu_rm_imm("cmp", ("r", RAX), 0x1000)
        A.pop(RAX)
        A.jcc("b", "rt_st3nt")
        A.label("rt_st3n")
    A.mov_rm_imm32(("r", RDX), FINISHER)
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    A.jcc("ne", "rt_st_fjdev" if fj else "rt_st_win")
    A.alu_rm_imm("cmp", ("abs", "th_n", 0), 0)
    A.jcc("ne", "rt_th_fin")                      # a thread's finisher ends the thread (the threads block)
    # FINISHER: flush, print yantra's halt line, exit 0 on 0x5555, else 1 (rcx = the value)
    A.label("rt_fin_print")
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
    # 0x3333 with a zero status (value >> 16 == 0) is yantra's `status: Some(0)`: exit 0 (vd0013)
    A.mov_r_rm(RAX, ("r", RCX))
    A.shift_imm("shr", ("r", RAX), 16)
    A.alu_rm_imm("cmp", ("r", RAX), 0)
    A.jcc("e", "rt_fin_ok")
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
    # FORK-JOIN: the device engines run only in the main thread (rt_st_fjdev refuses a worker every device
    # but the UART, WAIT and FINISHER), so they are the plain code even in a fork-join image
    fj_fs, fj_sys = A.fs, A.fjsys
    A.fs, A.fjsys = None, False
    runtime_window(A, base, prot_size, win, mac)
    runtime_spawn(A, base, win, mac)
    if not win:
        runtime_sock(A, base, mac, wm)
    else:
        runtime_sock_win(A, base)
    if net:
        runtime_net(A, base, vdf)
        A.label("rt_st3nt")                    # the store hook's door: rt_nt_st returns to the stub
        A.jmp("rt_nt_st")
    if scanp:
        import scanout_dev as sd
        sd.emit_x86(A, scanp, base, RAX, RCX, RDX, RBX, RSI, RDI, R8, R9, R10, R11, scanp.size())
    A.fs, A.fjsys = fj_fs, fj_sys
    # a store that missed RAM and is not UART/FINISHER: rax = addr
    A.label("rt_beyond")
    A.alu_rm_imm("cmp", ("abs", "th_n", 0), 0)
    A.jcc("ne", "rt_th_beyond")                   # a thread's stack store (the threads block)
    A.label("rt_beyond0")
    A.mov_rm_r(("abs", "fault_addr", 0), RAX)
    A.call("rt_flush")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.mov_r_imm64(RCX, 1 << 31)
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("b", "rt_store_bad")            # below 2^31, no device
    A.mov_r_rm(RCX, ("abs", "code_end", 0))
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("b", "rt_store_code")           # in the code span
    A.mov_r_rm(RAX, ("abs", "th_slim", 0))  # threaded: stores stop above the stacks
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_br_tail")
    A.mov_r_rm(RAX, ("abs", "ram", 0))    # yantra: BeyondRam { pc, addr, ram: the writable span }
    A.jmp("rt_br_tail")
    A.label("rt_store_bad")
    # yantra's store answers BadAccess below base except at its devices: slot 1 halts Device by
    # name; the socket and the other virtio-mmio slots are not native yet (t1-run reruns)
    A.mov_r_rm(RCX, ("r", RAX))
    A.mov_r_imm64(RDX, SLOT1[0])
    A.alu_r_rm("sub", RCX, ("r", RDX))
    A.alu_rm_imm("cmp", ("r", RCX), SLOT1[1])
    A.jcc("b", "rt_slot1")
    for lo, n in STORE_DEVICES:
        A.mov_r_rm(RCX, ("r", RAX))
        A.mov_r_imm64(RDX, lo)
        A.alu_r_rm("sub", RCX, ("r", RDX))
        A.alu_rm_imm("cmp", ("r", RCX), n)
        A.jcc("b", "rt_store_dev")
    A.jmp("rt_bada")
    A.label("rt_store_dev")
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
    # a load that missed RAM: rax = addr, edx = width; called from the cold stub, which loads
    # through rax at its B label if this returns (only the socket's RX does)
    A.label("rt_load_slow")
    if fj:                                 # a worker: the socket is a main-thread device
        A.alu_rm_imm("cmp", ("abs", "fj_worker", 0), 0)
        A.jcc("ne", "rt_ls_nosk")
    A.mov_r_rm(RCX, ("r", RAX))
    A.alu_rm_imm("sub", ("r", RCX), 0x1000_0110)   # SOCK..SOCK+16: the socket (runtime_sock)
    A.alu_rm_imm("cmp", ("r", RCX), 16)
    A.jcc("b", "rt_sk_ld")
    if fj:
        A.label("rt_ls_nosk")
    if scanp:
        A.mov_r_rm(RCX, ("r", RAX))
        A.alu_rm_imm("sub", ("r", RCX), 0x1000_1000)   # slot 0: the GPU (scanout_dev.py)
        A.alu_rm_imm("cmp", ("r", RCX), 0x1000)
        A.jcc("b", "rt_sc_ld")
    if net:
        A.alu_rm_imm("cmp", ("abs", "nt", NT["on"]), 0)
        A.jcc("e", "rt_ls_nont")
        A.mov_r_rm(RCX, ("r", RAX))
        A.alu_rm_imm("sub", ("r", RCX), NET_LO)
        A.alu_rm_imm("cmp", ("r", RCX), 0x1000)
        A.jcc("b", "rt_nt_ld")
        A.mov_r_rm(RCX, ("r", RAX))
        A.alu_rm_imm("sub", ("r", RCX), 0x1000_1000)   # slot 0: the GPU's identity, as yantra answers
        A.alu_rm_imm("cmp", ("r", RCX), 0x1000)
        A.jcc("b", "rt_nt_s0")
        A.label("rt_ls_nont")
    A.mov_rm_r(("abs", "fault_addr", 0), RAX)
    A.call("rt_flush")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.mov_r_imm64(RCX, 1 << 31)
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("b", "rt_load_bad")             # below 2^31 (a device): not in milestone 1
    A.mov_r_rm(RAX, ("abs", "top", 0))    # yantra: BeyondRam { pc, addr, ram: the loads' limit }
    A.label("rt_br_tail")                 # rax = the limit; fault_off, fault_addr: the site and the address
    A.mov_rm_r(("abs", "fault_word", 0), RAX)
    A.call("m_br1")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("m_ba2")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.call("rt_putu")
    A.call("m_br2")
    A.mov_r_rm(RAX, ("abs", "fault_word", 0))
    A.call("rt_putu")
    A.call("m_un4")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_load_bad")
    # yantra's load answers BadAccess below base except at its load devices: the counter and the
    # socket (0x1000_0108..0x1000_0120) and the virtio-mmio slots (0x1000_1000..0x1000_9000).
    # Those are not in milestone 1, but slot 1 (SLOT1), whose answer is fixed; every other
    # address halts as yantra-run prints it.
    A.mov_r_rm(RCX, ("r", RAX))
    A.mov_r_imm64(RDX, SLOT1[0])
    A.alu_r_rm("sub", RCX, ("r", RDX))
    A.alu_rm_imm("cmp", ("r", RCX), SLOT1[1])
    A.jcc("b", "rt_slot1")
    for lo, n in LOAD_DEVICES:
        A.mov_r_rm(RCX, ("r", RAX))
        A.mov_r_imm64(RDX, lo)
        A.alu_r_rm("sub", RCX, ("r", RDX))
        A.alu_rm_imm("cmp", ("r", RCX), n)
        A.jcc("b", "rt_load_dev")
    A.label("rt_bada")                    # yantra's "halt: BadAccess { pc: P, addr: A }"
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
    A.label("rt_slot1")                   # yantra's "halt: Device { pc: P, addr: A, why: SLOT1_WHY }"
    A.call("m_dev1")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("m_bada2")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.call("rt_putu")
    A.call("m_dev3")
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
    # ---- yantra's halts for a word it does not execute (fault_off, fault_word: the site and the word)
    A.label("rt_unimpl")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.mov_rm_r(("abs", "fault_addr", 0), RAX)
    A.label("rt_unimpl_at")               # fault_addr = the pc, fault_word's low half = the word
    A.call("rt_flush")
    A.call("m_un1")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.call("rt_putu")
    A.call("m_un2")
    A.mov_r_rm(RAX, ("abs", "fault_word", 0), 0)
    A.call("rt_putu")
    A.call("m_un3")
    A.mov_r_rm(RAX, ("abs", "fault_word", 0), 0)
    A.alu_rm_imm("and", ("r", RAX), 0x7F)
    A.call("rt_putu")
    A.call("m_un4")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_break")                   # ebreak, no stvec: Breakpoint { pc }
    A.call("rt_flush")
    A.call("m_brk")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("m_un4")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_ecall")                   # an SBI call but putchar / set_timer: rax = a7 (eid), rcx = a6 (fid)
    A.push(RCX)
    A.push(RAX)
    A.call("rt_flush")
    A.alu_rm_imm("cmp", ("m", RSP, 0), 8)
    A.jcc("ne", "rt_ec_sbi")
    A.call("m_shut")
    A.jmp("rt_ec_pc")
    A.label("rt_ec_sbi")
    A.call("m_sbi1")
    A.label("rt_ec_pc")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.alu_rm_imm("cmp", ("m", RSP, 0), 8)
    A.jcc("e", "rt_ec_end")
    A.call("m_sbi2")
    A.mov_r_rm(RAX, ("m", RSP, 0))
    A.call("rt_putu")
    A.call("m_sbi3")
    A.mov_r_rm(RAX, ("m", RSP, 8))
    A.call("rt_putu")
    A.label("rt_ec_end")
    A.call("m_un4")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_spin")                    # `jal rd, .`: SpinForever { pc }, exit 1
    A.call("rt_flush")
    A.call("m_spin")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("m_un4")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_sret")                    # sret: to sepc, which is 0 in every native run
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    # ---- a jump to a pc with no translation: rax = the target. yantra FETCHES it: below RAM that is
    # BadAccess (but for the SOCK and virtio windows, which answer a load), past the loads' limit
    # (top) BeyondRam, and in RAM the word there, which halts Unimplemented when its opcode is none
    # yantra executes. Anything yantra would execute is refused by name (t1-run falls back).
    A.label("rt_bad_target")
    if fj:
        # a worker's body returned to the sentinel return address: the next index
        A.alu_rm_imm("cmp", ("abs", "fj_worker", 0), 0)
        A.jcc("e", "rt_bt_main")
        A.mov_rm_imm32(("r", RDX), FJ_SENTINEL)
        A.alu_r_rm("cmp", RAX, ("r", RDX))
        A.jcc("e", "rt_fj_ret")
        A.label("rt_bt_main")
    if vd:
        A.alu_rm_imm("add", ("abs", "vd_st", 0), 1)       # yantra counts the fetch that faults
    A.mov_rm_r(("abs", "fault_addr", 0), RAX)
    A.call("rt_flush")
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("b", "rt_bt_low")
    A.mov_r_rm(RDX, ("r", RAX))
    A.alu_r_rm("sub", RDX, ("r", RCX))
    A.mov_r_rm(RCX, ("abs", "top", 0))
    A.alu_rm_imm("sub", ("r", RCX), 4)
    A.alu_r_rm("cmp", RDX, ("r", RCX))
    A.jcc("a", "rt_bt_beyond")
    A.mov_r_rm(RCX, ("m", RAX, 0), 0)     # the word yantra fetches
    A.mov_rm_r(("abs", "fault_word", 0), RCX, width=4)
    A.mov_r_rm(RDX, ("r", RCX), 0)
    A.alu_rm_imm("and", ("r", RDX), 3, 0)
    A.alu_rm_imm("cmp", ("r", RDX), 3, 0)
    A.jcc("ne", "rt_unimpl_at")           # a compressed encoding
    A.shift_imm("shr", ("r", RCX), 2, 0)
    A.alu_rm_imm("and", ("r", RCX), 31, 0)
    A.mov_rm_imm32(("r", RDX), YANTRA_OPS, 0)
    A.op(b"\xd3", 5, ("r", RDX), 0)       # shr edx, cl
    A.alu_rm_imm("and", ("r", RDX), 1, 0)
    A.jcc("e", "rt_unimpl_at")            # an opcode yantra does not execute
    A.label("rt_bt_refuse")
    A.call("m_jump1")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.call("rt_puthex")
    A.call("m_jump2")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_bt_low")
    A.mov_r_rm(RDX, ("r", RAX))
    A.alu_rm_imm("sub", ("r", RDX), SOCK)
    A.alu_rm_imm("cmp", ("r", RDX), SOCK_LEN)
    A.jcc("b", "rt_bt_refuse")
    A.mov_r_rm(RDX, ("r", RAX))
    A.alu_rm_imm("sub", ("r", RDX), VIRTIO_LO)
    A.alu_rm_imm("cmp", ("r", RDX), VIRTIO_LEN)
    A.jcc("b", "rt_bt_refuse")
    A.call("m_ba1")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.call("rt_putu")
    A.call("m_ba2")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.call("rt_putu")
    A.call("m_un4")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_bt_beyond")
    A.call("m_br1")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.call("rt_putu")
    A.call("m_ba2")
    A.mov_r_rm(RAX, ("abs", "fault_addr", 0))
    A.call("rt_putu")
    A.call("m_br2")
    A.mov_r_rm(RAX, ("abs", "top", 0))
    A.call("rt_putu")
    A.call("m_un4")
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
    A.label("rt_no_rep")
    A.call("m_norep")
    A.mov_rm_imm32(("r", RDI), 2)
    A.jmp("rt_exit")
    if win:
        A.label("rt_win_ev")
        A.call("m_winev")
        A.mov_rm_imm32(("r", RDI), 2)
        A.jmp("rt_exit")
    A.label("rt_rep_notag")
    A.call("m_repnotag")
    A.mov_rm_imm32(("r", RDI), 2)
    A.jmp("rt_exit")
    A.label("rt_threaded")
    A.call("m_threaded")
    A.mov_rm_imm32(("r", RDI), 2)
    A.jmp("rt_exit")
    A.label("rt_argsdup")
    A.call("m_argsdup")
    A.mov_rm_imm32(("r", RDI), 2)
    A.jmp("rt_exit")
    A.label("rt_wt_notag")
    A.call("rt_flush")
    A.call("m_waitnotag")
    A.mov_rm_imm32(("r", RDI), 2)
    A.jmp("rt_exit")
    A.label("rt_wt_halt")                         # yantra-run with no event source: halt Wait, exit 75
    A.call("rt_flush")
    A.call("m_wait")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("m_un4")
    A.mov_rm_imm32(("r", RDI), 75)
    A.jmp("rt_exit")
    if win:
        # x86_64-windows: yantra-run's refusal for a tag found NOWHERE (input.rs inject), one per tag;
        # rt_find_tag leaves its count in r8, and a tag found twice or more keeps the native text below
        for k in range(3):
            A.label("rt_nt%d" % k)
            A.alu_rm_imm("cmp", ("r", R8), 0)
            A.jcc("ne", "rt_no_tag")
            A.call("m_nt%d" % k)
            A.mov_rm_imm32(("r", RDI), 1)
            A.jmp("rt_exit")
    A.label("rt_no_tag")
    A.call("m_notag")
    A.mov_rm_imm32(("r", RDI), 1 if win else 2)    # x86_64-windows: 1, as yantra-run and the A64 runtimes
    A.label("rt_exit")
    A.alu_rm_imm("cmp", ("abs", "th_cap", 0), 0)
    A.jcc("ne", "rt_th_exit")                     # a thread's halt: yantra's thread line first
    if not win:
        A.alu_rm_imm("cmp", ("abs", "sk", 0), 0)
        A.jcc("e", "rt_exit2")
        A.call("rt_sk_report")                  # the socket's account, after the halt line
        A.label("rt_exit2")
    if net:
        A.alu_rm_imm("cmp", ("abs", "nt", NT["on"]), 0)
        A.jcc("e", "rt_exit3")
        A.call("rt_nt_report")                  # the frames device's account, after the halt line
        A.label("rt_exit3")
    if wm:
        A.call("rt_wm_report")                  # YANTRA_WATERMARK: the store high-water line, last
        A.label("rt_exit_nw")
    if vd:
        A.call("rt_vd_fin")                     # L33: the captured stderr, the `steps:` line, the verdict file
    if fj:
        A.alu_rm_imm("cmp", ("abs", "fj_att", 0), 0)
        A.jcc("e", "rt_exit_fa")
        A.call("rt_fja_rep")                    # S1: the fork-join attestation line, into the file only
        A.label("rt_exit_fa")
    sys_(60)
    if wm:
        # ---- ram: high water N of M octets, as yantra-run, when YANTRA_WATERMARK is set; keeps rdi
        A.label("rt_wm_report")
        A.push(RDI)
        A.lea(RDI, ("abs", "env_wm", 0))
        A.call("rt_getenv")
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "rt_wm_no")
        A.call("m_wm1")
        A.mov_r_rm(RAX, ("abs", "hw", 0))         # the absolute end address, or 0: no store
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "rt_wm_z")
        A.alu_rm_imm("add", ("r", RAX), -base)    # minus the base
        A.label("rt_wm_z")
        A.call("rt_putu")
        A.call("m_wm2")
        A.mov_r_rm(RAX, ("abs", "ram", 0))
        A.call("rt_putu")
        A.call("m_wm3")
        A.label("rt_wm_no")
        A.pop(RDI)
        A.ret()

    if vd:
        runtime_vd(A)

    # ---- flush: write(1, outbuf, outlen); saves every register the syscall touches
    A.label("rt_flush")
    if vdf:                                     # YANTRA_VIRTIO_DEFER: every halt serves a pending TX first
        A.alu_rm_imm("cmp", ("abs", "nt", NT["on"]), 0)
        A.jcc("e", "rt_flush_u")
        A.call("rt_nt_settle")
        A.label("rt_flush_u")                  # the UART buffer's flush is no halt: it enters here
    if net:                                    # the TX outbox goes out at every halt (and here)
        A.alu_rm_imm("cmp", ("abs", "nt", NT["on"]), 0)
        A.jcc("e", "rt_fl0")
        A.call("rt_nt_flush")
        A.label("rt_fl0")
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

    # ---- next_rec: the next record of an --events replay (YANTRA_REPLAY_EVENTS: 16 octets each,
    # little-endian, kind then value; t1-run writes them from a log it has read as yantra-run reads
    # it). Answers rax = the kind (0 a value or t= record, 1 `@N` run thread N, 2 the log is spent),
    # rdx = the value. Clobbers rcx, rsi, rdi, r11 and `num`. The one reader of the file (K3: the
    # thread scheduler takes its `@N` here too).
    A.label("rt_next_rec")
    A.mov_r_rm(RDI, ("abs", "rep_fd", 0))
    A.alu_rm_imm("sub", ("r", RDI), 1)
    A.lea(RSI, ("abs", "num", 0))
    A.mov_rm_imm32(("r", RDX), 16)
    A.mov_rm_imm32(("r", RAX), sc(0))
    A.syscall()
    A.alu_rm_imm("cmp", ("r", RAX), 16)
    A.jcc("ne", "rt_nr1")
    A.mov_r_rm(RAX, ("abs", "num", 0))
    A.mov_r_rm(RDX, ("abs", "num", 8))
    A.ret()
    A.label("rt_nr1")
    A.mov_rm_imm32(("r", RAX), 2)
    A.ret()

    # ---- find_tag: rax = the 8-octet tag; answer rax = its address if it occurs EXACTLY
    # once in the file-backed image, else 0 (yantra scans every offset)
    A.label("rt_find_tag")
    A.mov_r_imm64(RSI, base)
    A.label("rt_find_tag_at")                     # rsi = where the image's octets start, i.e. the host address of guest `base` (rt_th_pre: the file's)
    # only the file bytes of PF_W PT_LOADs (wr_tab) are scanned; each hit lies wholly inside one of them
    A.push(RBX)
    A.push(12)
    A.push(13)
    A.mov_r_rm(13, ("r", RSI))
    A.lea(RBX, ("abs", "wr_tab", 0))
    A.mov_r_rm(12, ("m", RBX, 0))
    A.lea(RBX, ("m", RBX, 8))
    A.alu_r_rm("xor", RDI, ("r", RDI), 0)          # hit
    A.alu_r_rm("xor", R8, ("r", R8), 0)            # count
    A.label("rt_ft0")
    A.test_rm_r(("r", 12), 12)
    A.jcc("e", "rt_ft3")
    A.mov_r_rm(RSI, ("m", RBX, 0))
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("sub", RSI, ("r", RCX))
    A.alu_r_rm("add", RSI, ("r", 13))             # the segment's host start
    A.mov_r_rm(RCX, ("m", RBX, 8))
    A.alu_r_rm("add", RCX, ("r", RSI))
    A.alu_rm_imm("sub", ("r", RCX), 7)
    A.label("rt_ft1")
    A.alu_r_rm("cmp", RSI, ("r", RCX))
    A.jcc("ae", "rt_ft5")
    A.alu_r_rm("cmp", RAX, ("m", RSI, 0))
    A.jcc("ne", "rt_ft2")
    A.mov_r_rm(RDI, ("r", RSI))
    A.lea(R8, ("m", R8, 1))
    A.label("rt_ft2")
    A.lea(RSI, ("m", RSI, 1))
    A.jmp("rt_ft1")
    A.label("rt_ft5")
    A.lea(RBX, ("m", RBX, 16))
    A.alu_rm_imm("sub", ("r", 12), 1)
    A.jmp("rt_ft0")
    A.label("rt_ft3")
    A.pop(13)
    A.pop(12)
    A.pop(RBX)
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
    A.alu_rm_imm("cmp", ("abs", "th_n", 0), 0)
    A.jcc("ne", "rt_th_wait")                     # a thread's wait is a switch (the threads block)
    for reg in (RSI, RDI, R8, R9, R10, R11):
        A.push(reg)
    if not win:
        A.alu_rm_imm("cmp", ("abs", "sk", 0), 0)
        A.jcc("ne", "rt_sk_wait")                 # a socket run's wait (runtime_sock)
    if net:
        A.alu_rm_imm("cmp", ("abs", "nt", NT["on"]), 0)
        A.jcc("ne", "rt_nt_wait")                 # a frames run's wait (runtime_net)
    A.mov_r_rm(RAX, ("abs", "ev_slot", 0))             # (a threaded image never runs: refused at startup)
    A.mov_r_rm(RCX, ("abs", "rec_fd", 0))              # no YANTRA_RECORD_EVENTS: yantra halts here,
    A.test_rm_r(("r", RCX), RCX)                       # tagged or not (yantra-run with no --events)
    A.jcc("e", "rt_wt_halt")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_wt_notag")
    A.alu_rm_imm("cmp", ("abs", "rep_fd", 0), 0)      # a replay: the next value record, or yantra's
    A.jcc("e", "rt_wt1")                               # halt at a wait the log has no record for
    A.call("rt_next_rec")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_wt_halt")
    A.mov_r_rm(RAX, ("r", RDX))
    A.jmp("rt_wt6")
    A.label("rt_wt1")
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
    A.label("rt_wt6")
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
    A.jmp("rt_ewr")

    # ---- puthex: rax as 0x… hex (minimal digits) to stderr
    A.label("rt_puthexn")                         # the same, no 0x: yantra-run's {:x?}
    A.lea(RSI, ("abs", "num", 31))
    A.label("rt_pn1")
    A.mov_r_rm(RCX, ("r", RAX))
    A.alu_rm_imm("and", ("r", RCX), 15)
    A.alu_rm_imm("cmp", ("r", RCX), 10)
    A.jcc("b", "rt_pn2")
    A.alu_rm_imm("add", ("r", RCX), 39)
    A.label("rt_pn2")
    A.alu_rm_imm("add", ("r", RCX), 48)
    A.lea(RSI, ("m", RSI, -1))
    A.raw(0x88, 0x0E)                          # mov [rsi], cl
    A.shift_imm("shr", ("r", RAX), 4)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_pn1")
    A.lea(RDX, ("abs", "num", 31))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    A.mov_rm_imm32(("r", RDI), 2)
    sys_(1)
    A.ret()

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
    A.jmp("rt_ewr")

    # ---------------------------------------------------------------- SASTHRDS threads (x86-64), SL6 / K3
    # runtime_threads_a64's design (its comment holds yantra's W-376 rules) on x86-64 Linux and Darwin:
    # a context (TH_CTX_X86) is +0 state (0 Fresh 1 Waiting 2 Running 3 Ended), +8 the resume address
    # (the cold stub's return), +24 fcsr, +32 regs and fregs (one 512-octet copy), +544 the host
    # registers by number; the capture (TH_CAP) first. Written flat in runtime() (rt2t1.py ports it);
    # rt_th_sp / rt_th_fget / rt_th_fput are translate()'s (they depend on the image).
    # ---- ctx: rax = k -> rax = its context (clobbers rcx, rdx)
    A.label("rt_th_ctx")
    A.mov_rm_imm32(("r", RCX), TH_CTX_X86)
    A.f7(4, ("r", RCX))
    A.alu_r_rm("add", RAX, ("abs", "th_ctx", 0))
    A.alu_rm_imm("add", ("r", RAX), TH_CAP)
    A.ret()
    # ---- pre: rsi = the mapping's length so far (kept); a threaded image's stacks and contexts are added
    A.label("rt_th_pre")
    A.push(RSI)
    A.lea(RSI, ("abs", "image", 0))
    A.mov_r_imm64(RAX, THREADS_TAG)
    A.call("rt_find_tag_at")
    A.mov_rm_r(("abs", "th_pre", 0), R8)
    A.lea(RSI, ("abs", "image", 0))
    A.mov_r_imm64(RAX, THREAD_ID_TAG)
    A.call("rt_find_tag_at")
    A.alu_r_rm("add", R8, ("abs", "th_pre", 0))
    A.mov_rm_r(("abs", "th_pre", 0), R8)
    A.pop(RSI)
    A.test_rm_r(("r", R8), R8)
    A.jcc("e", "rt_thp1")
    A.alu_rm_imm("add", ("r", RSI), 63 * 65536 + 256 + TH_AREA_X86)
    A.label("rt_thp1")
    A.ret()
    # ---- detect: after the image is copied, before the input (yantra's order)
    A.label("rt_th_detect")
    A.alu_rm_imm("cmp", ("abs", "th_pre", 0), 0)
    A.jcc("ne", "rt_thd0")
    A.ret()
    A.label("rt_thd0")
    if win:
        A.jmp("rt_threaded")                 # no Windows threads yet
    if fj:
        A.jmp("rt_threaded")                 # a fork-join image: yantra-run
    A.mov_r_imm64(RAX, THREADS_TAG)          # anything yantra might refuse goes to yantra-run whole
    A.call("rt_find_tag")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_threaded")
    A.mov_r_rm(RBX, ("r", RAX))
    A.mov_r_imm64(RAX, THREAD_ID_TAG)
    A.call("rt_find_tag")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_threaded")
    A.mov_rm_r(("abs", "th_id", 0), RAX)
    A.mov_r_rm(RBX, ("m", RBX, 8))           # the count, 1..64
    A.mov_r_rm(RAX, ("r", RBX))
    A.alu_rm_imm("sub", ("r", RAX), 1)
    A.alu_rm_imm("cmp", ("r", RAX), 64)
    A.jcc("ae", "rt_threaded")
    A.mov_r_imm64(RAX, EVENT_TAG)
    A.call("rt_find_tag")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_threaded")
    A.mov_r_imm64(RAX, entry)                # the entry: auipc sp / addi sp (thread k starts after them)
    A.mov_r_rm(RCX, ("m", RAX, 0), 0)
    A.alu_rm_imm("and", ("r", RCX), 0xFFF)
    A.alu_rm_imm("cmp", ("r", RCX), 0x117)
    A.jcc("ne", "rt_threaded")
    A.mov_r_rm(RCX, ("m", RAX, 4), 0)
    A.alu_rm_imm("and", ("r", RCX), 0xFFFFF)
    A.alu_rm_imm("cmp", ("r", RCX), 0x10113)
    A.jcc("ne", "rt_threaded")
    A.lea(RDI, ("abs", "th_env_listen", 0))  # a socket run: yantra refuses it
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_threaded")
    A.mov_rm_r(("abs", "th_n", 0), RBX)
    A.call("m_th1")
    A.mov_r_rm(RAX, ("r", RBX))
    A.call("rt_putu")
    A.call("m_nl")
    A.mov_r_rm(RAX, ("abs", "rec_fd", 0))
    A.alu_r_rm("or", RAX, ("abs", "rep_fd", 0))
    A.jcc("e", "rt_th_nolog")
    A.mov_r_rm(RAX, ("abs", "ram", 0))
    A.alu_rm_imm("add", ("r", RAX), 15)
    A.alu_rm_imm("and", ("r", RAX), -16)
    A.mov_rm_r(("abs", "th_stk", 0), RAX)
    A.mov_r_rm(RCX, ("r", RBX))
    A.alu_rm_imm("sub", ("r", RCX), 1)
    A.shift_imm("shl", ("r", RCX), 16)
    A.alu_r_rm("add", RCX, ("r", RAX))
    A.mov_rm_r(("abs", "th_slim", 0), RCX)
    A.mov_rm_r(("abs", "top", 0), RCX)
    A.call("m_th1")
    A.mov_r_rm(RAX, ("r", RBX))
    A.call("rt_putu")
    A.call("m_th2")
    A.mov_r_rm(RAX, ("r", RBX))
    A.alu_rm_imm("sub", ("r", RAX), 1)
    A.call("rt_putu")
    A.call("m_th3")
    A.mov_r_rm(RAX, ("abs", "th_stk", 0))
    A.call("rt_puthex")
    A.call("m_th4")
    A.mov_r_rm(RAX, ("abs", "th_slim", 0))
    A.call("rt_putu")
    A.call("m_nl")
    A.mov_r_rm(RAX, ("abs", "th_len", 0))
    A.alu_rm_imm("sub", ("r", RAX), TH_AREA_X86)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.mov_rm_r(("abs", "th_ctx", 0), RAX)
    A.mov_r_rm(RAX, ("r", RBX))
    A.alu_rm_imm("sub", ("r", RAX), 1)
    A.mov_rm_r(("abs", "th_cur", 0), RAX)    # so the first pick is thread 0
    if not win:
        # the log is a thread log: yantra's thread header in place of the clock one
        A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
        A.test_rm_r(("r", RDI), RDI)
        A.jcc("e", "rt_thd9")
        A.alu_rm_imm("sub", ("r", RDI), 1)
        A.alu_r_rm("xor", RSI, ("r", RSI), 0)
        if mac:
            A.mov_rm_imm32(("r", RAX), 0x2000000 + 201)
        else:
            A.mov_rm_imm32(("r", RAX), 77)       # ftruncate(fd, 0)
        A.syscall()
        A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
        A.alu_rm_imm("sub", ("r", RDI), 1)
        A.alu_r_rm("xor", RSI, ("r", RSI), 0)
        A.alu_r_rm("xor", RDX, ("r", RDX), 0)
        if mac:
            A.mov_rm_imm32(("r", RAX), 0x2000000 + 199)
        else:
            A.mov_rm_imm32(("r", RAX), 8)        # lseek(fd, 0, SEEK_SET)
        A.syscall()
        A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
        A.alu_rm_imm("sub", ("r", RDI), 1)
        A.lea(RSI, ("abs", "th_hdr", 0))
        A.mov_rm_imm32(("r", RDX), TH_HDR_LEN)
        sys_(1)
        A.label("rt_thd9")
    A.ret()
    A.label("rt_th_nolog")
    A.call("m_thnolog")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")

    # ---- the decision point: at the start, at every wait, at every end (all guest state saved; rsp
    # at the guest's level). rbx = k, rbp = its context, r12 the count left, r13 a resuming value
    A.label("rt_th_next")
    A.mov_r_rm(RAX, ("abs", "th_ended", 0))
    A.alu_r_rm("cmp", RAX, ("abs", "th_n", 0))
    A.jcc("ae", "rt_thn_end")
    A.alu_rm_imm("cmp", ("abs", "rep_fd", 0), 0)
    A.jcc("ne", "rt_thn_rep")
    A.mov_r_rm(RBX, ("abs", "th_cur", 0))    # live: round-robin from the one after the current
    A.mov_r_rm(R12, ("abs", "th_n", 0))
    A.label("rt_thn1")
    A.alu_rm_imm("add", ("r", RBX), 1)
    A.alu_r_rm("cmp", RBX, ("abs", "th_n", 0))
    A.jcc("b", "rt_thn2")
    A.alu_r_rm("xor", RBX, ("r", RBX), 0)
    A.label("rt_thn2")
    A.mov_r_rm(RAX, ("r", RBX))
    A.call("rt_th_ctx")
    A.mov_r_rm(RBP, ("r", RAX))
    A.alu_rm_imm("cmp", ("m", RBP, 0), 3)
    A.jcc("ne", "rt_thn3")
    A.alu_rm_imm("sub", ("r", R12), 1)
    A.jcc("ne", "rt_thn1")
    A.jmp("rt_thn_end")
    A.label("rt_thn3")
    A.mov_r_rm(RAX, ("r", RBX))
    A.mov_rm_imm32(("r", RDX), 0x40)         # "@k"
    A.mov_rm_imm32(("r", RCX), 1)
    A.call("rt_th_wr")
    A.alu_rm_imm("cmp", ("m", RBP, 0), 1)
    A.jcc("ne", "rt_th_in")
    A.call("rt_clock")
    A.mov_r_rm(R13, ("r", RAX))
    A.label("rt_thn_val")                    # a resuming thread's value, delivered at the resume
    A.mov_r_rm(RCX, ("abs", "ev_slot", 0))
    A.mov_rm_r(("m", RCX, 8), R13)
    A.mov_r_rm(RAX, ("r", R13))
    A.mov_rm_imm32(("r", RDX), 0x3D74)       # "t="
    A.mov_rm_imm32(("r", RCX), 2)
    A.call("rt_th_wr")
    A.jmp("rt_th_in")
    A.label("rt_thn_rep")                    # replay: the next record is `@k`, k a thread not ended
    A.call("rt_next_rec")
    A.alu_rm_imm("cmp", ("r", RAX), 1)
    A.jcc("ne", "rt_th_bad")
    A.alu_r_rm("cmp", RDX, ("abs", "th_n", 0))
    A.jcc("ae", "rt_th_bad")
    A.mov_r_rm(RBX, ("r", RDX))
    A.mov_r_rm(RAX, ("r", RBX))
    A.call("rt_th_ctx")
    A.mov_r_rm(RBP, ("r", RAX))
    A.alu_rm_imm("cmp", ("m", RBP, 0), 3)
    A.jcc("e", "rt_th_bad")
    A.mov_r_rm(RAX, ("r", RBX))
    A.mov_rm_imm32(("r", RDX), 0x40)
    A.mov_rm_imm32(("r", RCX), 1)
    A.call("rt_th_wr")
    A.alu_rm_imm("cmp", ("m", RBP, 0), 1)
    A.jcc("ne", "rt_th_in")
    A.call("rt_next_rec")                    # a resuming thread: then a value record
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_th_bad")
    A.mov_r_rm(R13, ("r", RDX))
    A.jmp("rt_thn_val")
    A.label("rt_thn_end")                    # every thread ended: a replay's log must be spent too
    A.alu_rm_imm("cmp", ("abs", "rep_fd", 0), 0)
    A.jcc("e", "rt_th_end")
    A.call("rt_next_rec")
    A.alu_rm_imm("cmp", ("r", RAX), 2)
    A.jcc("ne", "rt_th_bad")
    A.jmp("rt_th_end")
    A.label("rt_th_bad")
    A.mov_rm_imm32(("abs", "th_cap", 0), 0)
    A.call("rt_flush")
    A.call("m_thbad")
    A.mov_rm_imm32(("r", RDI), 2)
    A.jmp("rt_exit")

    # ---- wr: "<prefix><rax decimal>\n" to the log (dx the prefix's octets, rcx its length), if any
    A.label("rt_th_wr")
    A.alu_rm_imm("cmp", ("abs", "rec_fd", 0), 0)
    A.jcc("e", "rt_thw9")
    A.mov_r_rm(R8, ("r", RDX))
    A.mov_r_rm(R9, ("r", RCX))
    A.lea(RSI, ("abs", "num", 31))
    A.raw(0xC6, 0x06, 0x0A)                  # mov byte [rsi], 10
    A.mov_rm_imm32(("r", RCX), 10)
    A.label("rt_thw1")
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.f7(6, ("r", RCX))
    A.lea(RDX, ("m", RDX, 48))
    A.lea(RSI, ("m", RSI, -1))
    A.raw(0x88, 0x16)                        # mov [rsi], dl
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_thw1")
    A.alu_r_rm("sub", RSI, ("r", R9))
    A.alu_rm_imm("cmp", ("r", R9), 1)
    A.jcc("ne", "rt_thw2")
    A.mov_rm_r(("m", RSI, 0), R8, width=1)
    A.jmp("rt_thw3")
    A.label("rt_thw2")
    A.mov_rm_r(("m", RSI, 0), R8, width=2)
    A.label("rt_thw3")
    A.lea(RDX, ("abs", "num", 32))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
    A.alu_rm_imm("sub", ("r", RDI), 1)
    sys_(1)
    A.label("rt_thw9")
    A.ret()

    # ---- switch in: rbx = k, rbp = its context
    A.label("rt_th_in")
    A.mov_rm_r(("abs", "th_cur", 0), RBX)
    A.mov_r_rm(RAX, ("abs", "th_id", 0))
    A.mov_rm_r(("m", RAX, 8), RBX)           # SASTHRID's word = k, at every switch-in
    A.mov_rm_imm32(("abs", "th_cap", 0), 1)
    A.mov_rm_imm32(("abs", "th_capn", 0), 0)
    A.mov_r_rm(RAX, ("m", RBP, 0))
    A.mov_rm_imm32(("m", RBP, 0), 2)
    A.alu_rm_imm("cmp", ("r", RAX), 1)
    A.jcc("e", "rt_thi_res")
    A.lea(RDI, ("abs", "regs", 0))           # FRESH: every register 0, sp its stack, pc entry (+8)
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.mov_rm_imm32(("r", RCX), 512)
    A.raw(0xF3, 0xAA)                        # rep stosb: regs and fregs
    A.call("rt_th_fput")
    for n, h in M.items():
        A.alu_r_rm("xor", h, ("r", h), 0)
    A.mov_r_rm(RAX, ("abs", "th_cur", 0))
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_thi2")
    A.shift_imm("shl", ("r", RAX), 16)
    A.alu_r_rm("add", RAX, ("abs", "th_stk", 0))
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_th_sp")
    A.mov_r_imm64(RAX, entry)
    A.alu_rm_imm("add", ("r", RAX), 8)
    A.jmp("rt_indirect")
    A.label("rt_thi2")
    A.mov_r_imm64(RAX, entry)
    A.jmp("rt_indirect")
    A.label("rt_thi_res")                    # WAITING: its state back, then into its stub
    A.mov_r_rm(RAX, ("m", RBP, 24))
    A.call("rt_th_fput")
    A.lea(RSI, ("m", RBP, 32))
    A.lea(RDI, ("abs", "regs", 0))
    A.mov_rm_imm32(("r", RCX), 512)
    A.raw(0xF3, 0xA4)                        # rep movsb
    A.mov_r_rm(RAX, ("r", RBP))
    for n, h in M.items():
        A.mov_r_rm(h, ("m", RAX, 544 + 8 * h))
    A.jmp_rm(("m", RAX, 8))

    # ---- a thread's wait (from rt_store_slow; [rsp] = the cold stub's return): park it
    A.label("rt_th_wait")
    A.alu_rm_imm("cmp", ("abs", "th_bos", 0), 0)
    A.jcc("ne", "rt_wt_halt")                # the segment that stored past its stack ends here
    A.mov_r_rm(RAX, ("abs", "th_cur", 0))
    A.call("rt_th_ctx")
    for n, h in M.items():
        A.mov_rm_r(("m", RAX, 544 + 8 * h), h)
    A.mov_r_rm(RBX, ("r", RAX))
    A.pop(RCX)
    A.mov_rm_r(("m", RBX, 8), RCX)
    A.lea(RDI, ("m", RBX, 32))
    A.lea(RSI, ("abs", "regs", 0))
    A.mov_rm_imm32(("r", RCX), 512)
    A.raw(0xF3, 0xA4)                        # rep movsb
    A.call("rt_th_fget")
    A.mov_rm_r(("m", RBX, 24), RAX)
    A.mov_rm_imm32(("m", RBX, 0), 1)
    A.jmp("rt_th_next")

    # ---- a thread's finisher (rcx = the value; [rsp] = the stub's return): the thread ends
    A.label("rt_th_fin")
    A.alu_rm_imm("cmp", ("abs", "th_bos", 0), 0)
    A.jcc("ne", "rt_fin_print")
    A.mov_rm_r(("abs", "th_lastv", 0), RCX)
    A.mov_r_rm(RAX, ("r", RCX))
    A.alu_rm_imm("and", ("r", RAX), 0xFFFF)
    A.alu_rm_imm("cmp", ("r", RAX), 0x5555)
    A.jcc("e", "rt_thf_ok")
    A.alu_rm_imm("cmp", ("r", RAX), 0x3333)
    A.jcc("ne", "rt_thf_bad")
    A.mov_r_rm(RAX, ("r", RCX))
    A.shift_imm("shr", ("r", RAX), 16)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_thf_ok")
    A.label("rt_thf_bad")
    A.alu_rm_imm("cmp", ("abs", "th_failk", 0), 0)
    A.jcc("ne", "rt_thf_ok")
    A.mov_r_rm(RAX, ("abs", "th_cur", 0))
    A.alu_rm_imm("add", ("r", RAX), 1)
    A.mov_rm_r(("abs", "th_failk", 0), RAX)
    A.mov_rm_r(("abs", "th_failv", 0), RCX)
    A.label("rt_thf_ok")
    A.pop(RAX)                               # the stub's return: this thread never resumes
    A.mov_r_rm(RAX, ("abs", "th_cur", 0))
    A.call("rt_th_ctx")
    A.mov_rm_imm32(("m", RAX, 0), 3)
    A.alu_rm_imm("add", ("abs", "th_ended", 0), 1)
    A.jmp("rt_th_next")

    # ---- the end: every thread ended
    A.label("rt_th_end")
    A.mov_rm_imm32(("abs", "th_cap", 0), 0)
    A.alu_rm_imm("cmp", ("abs", "th_failk", 0), 0)
    A.jcc("e", "rt_the_ok")
    A.call("rt_flush")
    A.call("m_thf1")
    A.mov_r_rm(RAX, ("abs", "th_failk", 0))
    A.alu_rm_imm("sub", ("r", RAX), 1)
    A.call("rt_putu")
    A.call("m_thf2")
    A.mov_r_rm(RAX, ("abs", "th_failv", 0))
    A.alu_rm_imm("and", ("r", RAX), 0xFFFF)
    A.alu_rm_imm("cmp", ("r", RAX), 0x3333)
    A.jcc("ne", "rt_the_u")
    A.mov_r_rm(RAX, ("abs", "th_failv", 0))
    A.shift_imm("shr", ("r", RAX), 16)
    A.call("rt_putu")
    A.jmp("rt_the_t")
    A.label("rt_the_u")
    A.call("m_thf3")
    A.mov_r_rm(RAX, ("abs", "th_failv", 0))
    A.call("rt_puthex")
    A.call("m_thf4")
    A.label("rt_the_t")
    A.call("m_thf5")
    A.mov_r_rm(RCX, ("abs", "th_failv", 0))
    A.jmp("rt_fin_print")
    A.label("rt_the_ok")
    A.mov_r_rm(RCX, ("abs", "th_lastv", 0))
    A.jmp("rt_fin_print")

    # ---- a store past RAM (rax = addr, rcx = value, [rsp] = the stub's return): a stack's lands
    A.label("rt_th_beyond")
    A.push(RSI)
    A.push(RDI)
    A.mov_r_rm(RSI, ("r", RAX))
    A.mov_r_imm64(RDI, base)
    A.alu_r_rm("sub", RSI, ("r", RDI))       # rsi = the offset
    A.alu_r_rm("cmp", RSI, ("abs", "ram", 0))
    A.jcc("b", "rt_thb_out")
    A.alu_r_rm("cmp", RSI, ("abs", "th_slim", 0))
    A.jcc("ae", "rt_thb_out")
    A.mov_r_rm(RDX, ("abs", "fault_off", 0), 0)   # the width, from the store's word
    A.alu_r_rm("add", RDX, ("r", RDI))
    A.mov_r_rm(RDX, ("m", RDX, 0), 0)
    A.shift_imm("shr", ("r", RDX), 12, 0)
    A.alu_rm_imm("and", ("r", RDX), 3, 0)
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "rt_thb_b")
    A.alu_rm_imm("cmp", ("r", RDX), 1)
    A.jcc("e", "rt_thb_h")
    A.alu_rm_imm("cmp", ("r", RDX), 2)
    A.jcc("e", "rt_thb_w")
    A.mov_rm_r(("m", RAX, 0), RCX)
    A.mov_rm_imm32(("r", RDX), 8)
    A.jmp("rt_thb_chk")
    A.label("rt_thb_b")
    A.mov_rm_r(("m", RAX, 0), RCX, width=1)
    A.mov_rm_imm32(("r", RDX), 1)
    A.jmp("rt_thb_chk")
    A.label("rt_thb_h")
    A.mov_rm_r(("m", RAX, 0), RCX, width=2)
    A.mov_rm_imm32(("r", RDX), 2)
    A.jmp("rt_thb_chk")
    A.label("rt_thb_w")
    A.mov_rm_r(("m", RAX, 0), RCX, width=4)
    A.mov_rm_imm32(("r", RDX), 4)
    A.label("rt_thb_chk")                    # wholly inside its own stack? (thread 0 has none)
    A.mov_r_rm(RDI, ("abs", "th_cur", 0))
    A.test_rm_r(("r", RDI), RDI)
    A.jcc("e", "rt_thb_bos")
    A.alu_rm_imm("sub", ("r", RDI), 1)
    A.shift_imm("shl", ("r", RDI), 16)
    A.alu_r_rm("add", RDI, ("abs", "th_stk", 0))
    A.alu_r_rm("cmp", RSI, ("r", RDI))
    A.jcc("b", "rt_thb_bos")
    A.alu_r_rm("add", RSI, ("r", RDX))
    A.alu_rm_imm("add", ("r", RDI), 65536)
    A.alu_r_rm("cmp", RSI, ("r", RDI))
    A.jcc("be", "rt_thb_ret")
    A.label("rt_thb_bos")
    A.alu_rm_imm("cmp", ("abs", "th_bos", 0), 0)
    A.jcc("ne", "rt_thb_ret")
    A.mov_rm_r(("abs", "th_bos", 0), RAX)
    A.mov_r_rm(RDI, ("abs", "th_cur", 0))
    A.mov_rm_r(("abs", "th_bosk", 0), RDI)
    A.label("rt_thb_ret")
    A.pop(RDI)
    A.pop(RSI)
    A.ret()
    A.label("rt_thb_out")
    A.pop(RDI)
    A.pop(RSI)
    A.jmp("rt_beyond0")

    # ---- a thread's halt (rdi = the code; its lines are in the capture): yantra's thread line first.
    # rbx = the capture, r12 its length, r13 the end of the held halt line
    A.label("rt_th_exit")
    A.mov_rm_r(("abs", "th_rc", 0), RDI)
    A.mov_rm_imm32(("abs", "th_cap", 0), 0)
    A.mov_r_rm(RBX, ("abs", "th_ctx", 0))
    A.mov_r_rm(R12, ("abs", "th_capn", 0))
    A.alu_rm_imm("cmp", ("r", R12), 6)
    A.jcc("b", "rt_thx_raw")
    A.mov_r_rm(RAX, ("m", RBX, 0), 0)
    A.alu_rm_imm("cmp", ("r", RAX), 0x746C6168, 0)    # "halt"
    A.jcc("ne", "rt_thx_raw")
    A.movzx(RAX, ("m", RBX, 4), 2)
    A.alu_rm_imm("cmp", ("r", RAX), 0x203A)           # ": "
    A.jcc("ne", "rt_thx_raw")
    A.mov_rm_imm32(("r", R13), 6)
    A.label("rt_thx1")
    A.alu_r_rm("cmp", R13, ("r", R12))
    A.jcc("ae", "rt_thx2")
    A.mov_r_rm(RAX, ("r", RBX))
    A.alu_r_rm("add", RAX, ("r", R13))
    A.movzx(RAX, ("m", RAX, 0), 1)
    A.alu_rm_imm("cmp", ("r", RAX), 10)
    A.jcc("e", "rt_thx2")
    A.alu_rm_imm("add", ("r", R13), 1)
    A.jmp("rt_thx1")
    A.label("rt_thx2")
    # two passes over the why: r15 = 0 yantra's `threads:` line (before the held lines), 1 yantra-run's
    # last line `yantra-run: the threaded run failed — <why>` (a BeyondOwnStack, or a SpinForever halt)
    A.alu_r_rm("xor", R15, ("r", R15), 0)
    A.label("rt_thx_line")
    A.alu_rm_imm("cmp", ("abs", "th_bos", 0), 0)
    A.jcc("ne", "rt_thx_bos")
    A.test_rm_r(("r", R15), R15)
    A.jcc("ne", "rt_thx_h1")
    A.call("m_thh0")
    A.jmp("rt_thx_h2")
    A.label("rt_thx_h1")
    A.call("m_thfail")
    A.label("rt_thx_h2")
    A.call("m_thw1")
    A.mov_r_rm(RAX, ("abs", "th_cur", 0))
    A.call("rt_putu")
    A.call("m_thh2")
    A.call("rt_thx_text")
    A.call("m_thh3")
    A.jmp("rt_thx_next")
    A.label("rt_thx_bos")
    A.test_rm_r(("r", R15), R15)
    A.jcc("ne", "rt_thx_b1")
    A.call("m_thb1")
    A.jmp("rt_thx_b2")
    A.label("rt_thx_b1")
    A.call("m_thfail")
    A.label("rt_thx_b2")
    A.call("m_thw1")
    A.mov_r_rm(RAX, ("abs", "th_bosk", 0))
    A.call("rt_putu")
    A.call("m_thb2")
    A.mov_r_rm(RAX, ("abs", "th_bos", 0))
    A.call("rt_puthex")
    A.call("m_thb3")
    A.mov_r_rm(RAX, ("abs", "ram", 0))
    A.call("rt_putu")
    A.call("m_thb4")
    A.mov_r_rm(R14, ("abs", "th_bosk", 0))
    A.test_rm_r(("r", R14), R14)
    A.jcc("ne", "rt_thx_own")
    A.call("m_thb5")
    A.jmp("rt_thx_b8")
    A.label("rt_thx_own")
    A.call("m_thb6")
    A.alu_rm_imm("sub", ("r", R14), 1)
    A.shift_imm("shl", ("r", R14), 16)
    A.alu_r_rm("add", R14, ("abs", "th_stk", 0))
    A.mov_r_imm64(RAX, base)
    A.alu_r_rm("add", R14, ("r", RAX))
    A.mov_r_rm(RAX, ("r", R14))
    A.call("rt_puthex")
    A.call("m_thb7")
    A.lea(RAX, ("m", R14, 65536))
    A.call("rt_puthex")
    A.label("rt_thx_b8")
    A.call("m_thb8")
    A.call("rt_thx_text")
    A.call("m_thb9")
    A.mov_rm_imm32(("abs", "th_rc", 0), 1)   # BeyondOwnStack always fails
    A.label("rt_thx_next")
    A.test_rm_r(("r", R15), R15)
    A.jcc("ne", "rt_thx_end")
    A.mov_r_rm(RSI, ("r", RBX))              # the held lines
    A.mov_r_rm(RDX, ("r", R12))
    A.mov_rm_imm32(("r", RDI), 2)
    sys_(1)
    A.alu_rm_imm("cmp", ("abs", "th_bos", 0), 0)
    A.jcc("ne", "rt_thx_fail")
    A.alu_rm_imm("cmp", ("r", R13), 6 + 11)  # "SpinForever"
    A.jcc("b", "rt_thx_end")
    A.mov_r_rm(RAX, ("m", RBX, 6), 0)
    A.alu_rm_imm("cmp", ("r", RAX), 0x6E697053, 0)    # "Spin"
    A.jcc("ne", "rt_thx_end")
    A.mov_r_rm(RAX, ("m", RBX, 10), 0)
    A.alu_rm_imm("cmp", ("r", RAX), 0x65726F46, 0)    # "Fore"
    A.jcc("ne", "rt_thx_end")
    A.mov_rm_imm32(("abs", "th_rc", 0), 1)   # a thread's SpinForever fails the run (yantra-run)
    A.label("rt_thx_fail")
    A.mov_rm_imm32(("r", R15), 1)
    A.jmp("rt_thx_line")
    A.label("rt_thx_end")
    A.mov_r_rm(RDI, ("abs", "th_rc", 0))
    A.jmp("rt_exit")
    A.label("rt_thx_raw")
    A.mov_r_rm(RSI, ("r", RBX))
    A.mov_r_rm(RDX, ("r", R12))
    A.mov_rm_imm32(("r", RDI), 2)
    sys_(1)
    A.mov_r_rm(RDI, ("abs", "th_rc", 0))
    A.jmp("rt_exit")
    A.label("rt_thx_text")                   # the held halt line's text after "halt: "
    A.lea(RSI, ("m", RBX, 6))
    A.mov_r_rm(RDX, ("r", R13))
    A.alu_rm_imm("sub", ("r", RDX), 6)
    A.mov_rm_imm32(("r", RDI), 2)
    sys_(1)
    A.ret()

    # ---- ewr: rsi, rdx to stderr, or into the capture while a thread runs (clobbers rax rcx rdx rsi rdi r11)
    A.label("rt_ewr")
    A.alu_rm_imm("cmp", ("abs", "th_cap", 0), 0)
    A.jcc("e", "rt_ewr9")
    A.mov_r_rm(RAX, ("abs", "th_capn", 0))
    A.mov_r_rm(RCX, ("r", RAX))
    A.alu_r_rm("add", RCX, ("r", RDX))
    A.alu_rm_imm("cmp", ("r", RCX), TH_CAP)
    A.jcc("a", "rt_ewr_ov")
    A.mov_rm_r(("abs", "th_capn", 0), RCX)
    A.mov_r_rm(RDI, ("abs", "th_ctx", 0))
    A.alu_r_rm("add", RDI, ("r", RAX))
    A.mov_r_rm(RCX, ("r", RDX))
    A.raw(0xF3, 0xA4)                        # rep movsb
    A.ret()
    A.label("rt_ewr_ov")                     # more than the capture holds: out as it is
    A.mov_rm_r(("abs", "th_e1", 0), RSI)
    A.mov_rm_r(("abs", "th_e2", 0), RDX)
    A.mov_rm_imm32(("abs", "th_cap", 0), 0)
    A.mov_r_rm(RSI, ("abs", "th_ctx", 0))
    A.mov_r_rm(RDX, ("abs", "th_capn", 0))
    A.mov_rm_imm32(("r", RDI), 2)
    sys_(1)
    A.mov_r_rm(RSI, ("abs", "th_e1", 0))
    A.mov_r_rm(RDX, ("abs", "th_e2", 0))
    A.label("rt_ewr9")
    A.mov_rm_imm32(("r", RDI), 2)
    sys_(1)
    A.ret()

    msg("m_th1", b"threads: ")
    msg("m_th2", b" contexts; ")
    msg("m_th3", b" host stacks of 65536 octets from RAM offset ")
    msg("m_th4", b", stores now stop at ")
    msg("m_thnolog", b"threads: refused \xe2\x80\x94 a threaded image needs --events or --record-events: which "
        b"thread runs first is a decision the log makes, and there is no default (refused at load, exit 1; "
        b"nothing ran)\n")
    msg("m_thbad", b"native: not in milestone 1 (a threaded replay whose log does not fit this run: "
        b"yantra-run judges it)\n")
    msg("m_thf1", b"threads: thread ")
    msg("m_thf2", b" ended with status ")
    msg("m_thf3", b"undefined (finisher value ")
    msg("m_thf4", b")")
    msg("m_thf5", b" \xe2\x80\x94 the first non-zero; every thread ran to its end and the run FAILS\n")
    msg("m_thh0", b"threads: ")
    msg("m_thw1", b"thread ")
    msg("m_thfail", b"yantra-run: the threaded run failed \xe2\x80\x94 ")
    msg("m_thh2", b" halted ")
    msg("m_thh3", b", not at its finisher \xe2\x80\x94 that ends the WHOLE run, because RAM is shared\n")
    msg("m_thb1", b"threads: BeyondOwnStack \xe2\x80\x94 ")
    msg("m_thb2", b" stored at ")
    msg("m_thb3", b", at or above the ")
    msg("m_thb4", b" octets of RAM the image was given and outside its own stack \xe2\x80\x94 ")
    msg("m_thb5", b"it has no host stack (thread 0's is the image's own)")
    msg("m_thb6", b"its own stack is ")
    msg("m_thb7", b"..")
    msg("m_thb8", b"; a heap past RAM or a stack overflowing into another thread's. The WHOLE run ends "
        b"(the segment ended ")
    msg("m_thb9", b")\n")

    msg("m_fin1", b"halt: Finisher { value: ")
    msg("m_fin2", b", status: Some(")
    msg("m_fin3", b") }\n")
    msg("m_fin4", b", status: None }\n")
    msg("m_addr", b" addr ")
    msg("m_badstore", b"halt: BadAccess (a store to the socket or a virtio-mmio slot; not in milestone 1) at pc ")
    msg("m_codestore", b"native: refused a store into the code span at pc ")
    msg("m_codestore2", b" (yantra would allow it; self-modifying code is not translated)\n")
    msg("m_badload", b"halt: BadAccess (a load below 2^31; MMIO loads are not in milestone 1) at pc ")
    msg("m_bada1", b"halt: BadAccess { pc: ")
    msg("m_bada2", b", addr: ")
    msg("m_bada3", b" }\n")
    msg("m_dev1", b"halt: Device { pc: ")
    msg("m_dev3", b", why: \"" + SLOT1_WHY + b"\" }\n")
    msg("m_seg0", b": segment at ")
    msg("m_seg1", b" needs ")
    msg("m_seg2", b" bytes and RAM is ")
    msg("m_seg3", b" \xe2\x80\x94 raise it\n")
    msg("m_insn", b"native: not in milestone 1 (an instruction the translator does not lower) at pc ")
    msg("m_steplim", b"halt: StepLimit { pc: ")
    msg("m_steps1", b"steps: ")
    msg("m_steps1b", b" executed instructions\n")
    msg("m_stop1", b"yantra-run: STOPPED AT THE STEP LIMIT of ")
    msg("m_stop1b", b" instructions at pc ")
    msg("m_stop2", b" \xe2\x80\x94 the program did not finish. Raise YANTRA_STEPS. This is a budget exhaustion, NOT a program fault, and the ")
    msg("m_stop2b", b" instructions above are a LOWER BOUND on what it needed.\n")
    msg("m_jump1", b"native: jump to ")
    msg("m_jump2", b", outside the translated code: executing data is yantra-only (no native lowering, by ruling); run it under yantra-run\n")
    msg("m_un1", b"halt: Unimplemented { pc: ")
    msg("m_un2", b", word: ")
    msg("m_un3", b", opcode: ")
    msg("m_un4", b" }\n")
    msg("m_brk", b"halt: Breakpoint { pc: ")
    msg("m_shut", b"halt: Shutdown { pc: ")
    msg("m_sbi1", b"halt: Sbi { pc: ")
    msg("m_sbi2", b", eid: ")
    msg("m_sbi3", b", fid: ")
    msg("m_spin", b"halt: SpinForever { pc: ")
    msg("m_ba1", b"halt: BadAccess { pc: ")
    msg("m_ba2", b", addr: ")
    msg("m_br1", b"halt: BeyondRam { pc: ")
    msg("m_br2", b", ram: ")
    msg("m_wait", b"halt: Wait { pc: ")
    msg("m_nl", b"\n")
    msg("m_args1", b"args: ")
    msg("m_args2", b" handed over, tags at [")
    msg("m_args3", b", ")
    msg("m_args4", b"], run at ")
    msg("m_noargv", "args: not handed over \u2014 no argument interface in this image: the SASARGV tag (0x56475241534153) appears at no word. It was built without the argument globals \u2014 declare them, or do not pass arguments\n".encode())
    msg("m_noargc", "args: not handed over \u2014 no argument interface in this image: the SASARGC tag (0x43475241534153) appears at no word. It was built without the argument globals \u2014 declare them, or do not pass arguments\n".encode())
    msg("m_argsdup", b"native: not in milestone 1 (an argument tag occurs more than once; yantra-run names every offset; run it under yantra-run)\n")
    if vd:
        msg("m_vsl", b"halt: StepLimit { pc: ")
        msg("m_vsp", b"halt: SpinForever { pc: ")
    if wm:
        msg("m_wm1", b"ram: high water ")
        msg("m_wm2", b" of ")
        msg("m_wm3", b" octets\n")
    msg("m_noinput", b"native: YANTRA_INPUT could not be read\n")
    msg("m_noram", b"native: guest RAM could not be mapped at its own address\n")
    msg("m_norec", b"native: YANTRA_RECORD_EVENTS could not be opened for writing\n")
    msg("m_norep", b"native: YANTRA_REPLAY_EVENTS could not be opened for reading\n")
    if win:
        msg("m_winev", b"native: not in milestone 1 (YANTRA_RECORD_EVENTS or YANTRA_REPLAY_EVENTS on Windows: the clock is not ported there; run it under yantra-run)\n")
    msg("m_repnotag", b"native: not in milestone 1 (an --events replay (YANTRA_REPLAY_EVENTS) with the SASEVENT tag missing or duplicated; run it under yantra-run)\n")
    msg("m_reptag", b"native: replay: SASEVENT tag at RAM offset ")
    msg("m_waitnotag", b"native: not in milestone 1 (a recorded WAIT (YANTRA_RECORD_EVENTS) with the SASEVENT tag missing or duplicated; run it under yantra-run)\n")
    msg("m_notag", b"native: the image lacks an input tag, or has one twice\n")
    if win:
        for k, (what, tag) in enumerate((("SASINPUT", 0x5455_504e_4953_4153), ("SASINAME", 0x454d_414e_4953_4153),
                                         ("SASTRACE", 0x4543_4152_5453_4153))):
            msg("m_nt%d" % k, ("input: refused \u2014 no input interface in this image: the %s tag (%#x) appears at no "
                               "word. It was built without the `\u0928\u093f\u0935\u0947\u0936\u2026` globals "
                               "\u2014 run the entry that declares them, or do not pass an input\n" % (what, tag)).encode())
    msg("m_threaded", b"native: not in milestone 1 (a threaded image this runtime does not run: its tags or startup are not the regular form, a socket run, Windows or fork-join; run it under yantra-run)\n")
    msg("m_entry_both", b"input: YANTRA_INPUT_ENTRY and YANTRA_INPUT_NAME are both set \xe2\x80\x94 the entry form builds the name itself (module NUL routine); set one\n")
    msg("m_entry1", b"input: YANTRA_INPUT_ENTRY ")
    msg("m_long1", b"input: YANTRA_INPUT_ENTRY is ")
    msg("m_long2", b" octets, over the 4096 limit\n")
    msg("m_bad8", b" is not UTF-8 \xe2\x80\x94 module and routine names are UTF-8\n")
    msg("m_noname", b"input: YANTRA_INPUT needs YANTRA_INPUT_NAME (or YANTRA_INPUT_ENTRY) \xe2\x80\x94 the module name is passed, never guessed\n")
    msg("m_entry2", b" is malformed \xe2\x80\x94 it must be \"<module> <routine>\": exactly one ASCII space, both halves non-empty and without whitespace\n")
    if fj:
        runtime_fj(A)
        strings.append(("env_fj", b"SASSEMBLY_NATIVE_THREADS=\0"))
        strings.append(("env_fjk", b"SASSEMBLY_NATIVE_FJ_MIN=\0"))
        strings.append(("env_fja", b"SASSEMBLY_NATIVE_FJATTEST=\0"))
        strings.append(("fja_0", b'fj_attest v1 p_req='))
        strings.append(("fja_1", b' fj_p_max='))
        strings.append(("fja_2", b' B='))
        strings.append(("fja_3", b' K='))
        strings.append(("fja_4", b' K0='))
        strings.append(("fja_5", b' spawned='))
        strings.append(("fja_6", b'\n'))
    if win:
        runtime_win(A)
        runtime_win_spawn(A)
    if wm:
        strings.append(("env_wm", b"YANTRA_WATERMARK=\0"))
    if vd:
        strings.append(("env_vd", b"YANTRA_VERDICT=\0"))
        strings.append(("vd_nm", b"vd\0"))
        strings.append(("vd_l1", b'fault '))
        strings.append(("vd_l2", b'\n'))
        strings.append(("vd_l3", b'finisher '))
        strings.append(("vd_l4", b' '))
        strings.append(("vd_l5", b'-'))
        strings.append(("vd_l6", b'steps '))
        strings.append(("vd_l7", b'verdict: '))
        strings.append(("vd_l8", b': '))
        strings.append(("vd_l9", b'Unknown error '))
        strings.append(("vd_l10", b' (os error '))
        strings.append(("vd_l11", b')\n'))
        strings.append(("vd_l12", b'steps: '))
        strings.append(("vd_l13", b' executed instructions\n'))
        strings.append(("vd_l14", b'No such file or directory'))
        strings.append(("vd_l15", b'Permission denied'))
        strings.append(("vd_l16", b'Is a directory'))
        strings.append(("vd_l17", b'Not a directory'))
        strings.append(("vd_l18", b'Read-only file system'))
        strings.append(("vd_l19", b'No space left on device'))
        strings.append(("vd_l20", b'File name too long'))
        strings.append(("vd_l21", b'wait\n'))
        strings.append(("vd_l22", b'steplimit\n'))
        strings.append(("vd_l23", b'spin\n'))
        strings.append(("vd_pfin", b'Finisher { value: '))
        strings.append(("vd_pwt", b'Wait'))
        strings.append(("vd_psl", b'StepLimit'))
        strings.append(("vd_psp", b'SpinForever'))
    for lab, text in strings + [("env_ram", b"YANTRA_RAM=\0"), ("env_steps", b"YANTRA_STEPS=\0"), ("env_input", b"YANTRA_INPUT=\0"),
                                ("env_grant", b"YANTRA_GRANT_ENV=\0"), ("env_files", b"YANTRA_FILES=\0"),
                                ("env_name", b"YANTRA_INPUT_NAME=\0"), ("env_trace", b"YANTRA_INPUT_TRACE=\0"),
                                ("env_entry", b"YANTRA_INPUT_ENTRY=\0"),
                                ("env_rec", b"YANTRA_RECORD_EVENTS=\0"), ("rec_hdr", REC_HEADER),
                                ("env_rep", b"YANTRA_REPLAY_EVENTS=\0"),
                                ("th_env_listen", b"YANTRA_LISTEN=\0"), ("th_hdr", TH_HDR)]:
        A.label(lab)
        A.raw(text)
    # the code points Rust's {:?} writes as \\u{..} that putq knows (frozen; (lo, hi) as u32
    # pairs, (0, 0) ends): C1, NBSP, soft hyphen, combining diacritics, the Devanagari marks,
    # Unicode spaces and format controls
    A.label("rt_esctab")
    A.raw(b"\x80\x00\x00\x00\xa0\x00\x00\x00\xad\x00\x00\x00\xad\x00\x00\x00\x00\x03\x00\x00\x6f\x03\x00\x00\x00\x09\x00\x00\x02\x09\x00\x00\x3a\x09\x00\x00\x3a\x09\x00\x00\x3c\x09\x00\x00\x3c\x09\x00\x00\x41\x09\x00\x00\x48\x09\x00\x00\x4d\x09\x00\x00\x4d\x09\x00\x00\x51\x09\x00\x00\x57\x09\x00\x00\x62\x09\x00\x00\x63\x09\x00\x00\x80\x16\x00\x00\x80\x16\x00\x00\x00\x20\x00\x00\x0f\x20\x00\x00\x28\x20\x00\x00\x2f\x20\x00\x00\x5f\x20\x00\x00\x6f\x20\x00\x00\x00\x30\x00\x00\x00\x30\x00\x00\xff\xfe\x00\x00\xff\xfe\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00")



def runtime_sock(A, base, mac=False, wm=False):
    """THE SOCKET DEVICE, x86-64 Linux, and Darwin when mac (SOCKET.md; yantra's socket and
    yantra-run's serve_live). Darwin: BSD numbers (+0x2000000), an error sets the carry (made -errno
    here, as Linux answers), sockaddr_in leads with sin_len, SO_NOSIGPIPE stands for MSG_NOSIGNAL
    and ECONNRESET is 54.
    Entered from runtime(): rt_sk_init (load time), rt_sk_ld / rt_sk_st (the slow paths: rax = addr,
    rcx = value, edx = width; the RX load returns rax = the latch's address, which the guest's own
    load at its B label reads), rt_sk_wait (from rt_wait, the guest's rsi rdi r8..r11 pushed; it
    leaves through rt_wt4) and rt_sk_report (from rt_exit). Once the guest runs, every register
    but rax rcx rdx is the guest's; a path that needs more saves it."""
    strs = []

    def sk(off):
        return ("abs", "sk", off)

    def say(text):
        lab = "sk_s%d" % len(strs)
        strs.append((lab, text))
        A.lea(RSI, ("abs", lab, 0))
        A.mov_rm_imm32(("r", RDX), len(text))
        A.mov_rm_imm32(("r", RDI), 2)
        sysn(1)

    ncarry = [0]

    def sysn(n):
        A.mov_rm_imm32(("r", RAX), 0x2000000 + SOCK_DARWIN_X86[n] if mac else n)
        A.syscall()
        if mac:                               # carry set: rax = errno, made -errno as Linux answers
            ncarry[0] += 1
            A.jcc("ae", "rt_skc%d" % ncarry[0])
            A.f7(3, ("r", RAX))
            A.label("rt_skc%d" % ncarry[0])

    def fd(r, off):                       # r = the descriptor stored + 1 at sk+off
        A.mov_r_rm(r, sk(off))
        A.alu_rm_imm("sub", ("r", r), 1)

    def pc_to_rax():
        A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
        A.mov_r_imm64(RCX, base)
        A.alu_r_rm("add", RAX, ("r", RCX))

    def native_refuse(lab, text):          # before the guest runs, so t1-run.sh reruns it whole
        A.label(lab)
        say(b"native: not in milestone 1 (" + text + b"; run it under yantra-run)\n")
        A.mov_rm_imm32(("r", RDI), 2)
        A.jmp("rt_exit")

    def nib(lab):                          # rcx = a nibble -> its lowercase hex digit
        A.alu_rm_imm("cmp", ("r", RCX), 10)
        A.jcc("b", lab)
        A.alu_rm_imm("add", ("r", RCX), 39)
        A.label(lab)
        A.alu_rm_imm("add", ("r", RCX), 48)

    # ---- rt_sk_init: YANTRA_LISTEN=127.0.0.1:PORT binds the listener (yantra-run --listen) and
    # rewrites the log's header to the socket log's; with no YANTRA_LISTEN nothing changes
    A.label("rt_sk_init")
    A.lea(RDI, ("abs", "sk_env_listen", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_ski1")
    A.lea(RDI, ("abs", "sk_env_replay", 0))     # YANTRA_SOCK_REPLAY=<socket log>: replay it (phase 2)
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_skrp")
    A.ret()
    A.label("rt_ski1")
    A.alu_rm_imm("cmp", ("abs", "rec_fd", 0), 0)
    A.jcc("e", "rt_ski_norec")
    A.alu_rm_imm("cmp", ("abs", "ev_slot", 0), 0)
    A.jcc("e", "rt_ski_noslot")
    A.mov_r_rm(RSI, ("r", RAX))
    A.lea(RDI, ("abs", "sk_pfx", 0))
    A.mov_rm_imm32(("r", RCX), 10)
    A.label("rt_ski2")                        # "127.0.0.1:"
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.movzx(RDX, ("m", RDI, 0), 1)
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    A.jcc("ne", "rt_ski_addr")
    A.lea(RSI, ("m", RSI, 1))
    A.lea(RDI, ("m", RDI, 1))
    A.alu_rm_imm("sub", ("r", RCX), 1)
    A.jcc("ne", "rt_ski2")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.alu_r_rm("xor", R8, ("r", R8), 0)
    A.label("rt_ski3")                        # the port: 1..5 decimal digits, at most 65535
    A.movzx(RCX, ("m", RSI, 0), 1)
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("e", "rt_ski4")
    A.alu_rm_imm("sub", ("r", RCX), 48)
    A.alu_rm_imm("cmp", ("r", RCX), 9)
    A.jcc("a", "rt_ski_addr")
    A.raw(0x48, 0x6B, 0xC0, 0x0A)                  # imul rax, rax, 10
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.lea(RSI, ("m", RSI, 1))
    A.lea(R8, ("m", R8, 1))
    A.alu_rm_imm("cmp", ("r", R8), 5)
    A.jcc("a", "rt_ski_addr")
    A.jmp("rt_ski3")
    A.label("rt_ski4")
    A.test_rm_r(("r", R8), R8)
    A.jcc("e", "rt_ski_addr")
    A.alu_rm_imm("cmp", ("r", RAX), 65535)
    A.jcc("a", "rt_ski_addr")
    # sockaddr_in: AF_INET, the port big-endian, 127.0.0.1
    A.mov_r_rm(RDX, ("r", RAX))
    A.shift_imm("shr", ("r", RDX), 8)
    A.alu_rm_imm("and", ("r", RAX), 0xFF)
    A.shift_imm("shl", ("r", RAX), 8)
    A.alu_r_rm("or", RAX, ("r", RDX))
    A.shift_imm("shl", ("r", RAX), 16)
    A.alu_rm_imm("or", ("r", RAX), 0x0210 if mac else 2)   # Darwin: sin_len 16, then sin_family
    A.mov_rm_r(sk(SK_SA), RAX, width=4)
    A.mov_rm_imm32(sk(SK_SA + 4), 0x0100007F, 0)
    A.mov_rm_imm32(sk(SK_SA + 8), 0)
    # socket, SO_REUSEADDR (as Rust's TcpListener::bind sets it), bind, listen 128, getsockname
    A.mov_rm_imm32(("r", RDI), 2)
    A.mov_rm_imm32(("r", RSI), 1)
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    sysn(41)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_ski_bind")
    A.lea(RCX, ("m", RAX, 1))
    A.mov_rm_r(sk(SK_LFD), RCX)
    A.mov_rm_imm32(sk(SK_T1), 1)
    fd(RDI, SK_LFD)
    A.mov_rm_imm32(("r", RSI), 0xFFFF if mac else 1)  # SOL_SOCKET, SO_REUSEADDR
    A.mov_rm_imm32(("r", RDX), 4 if mac else 2)
    A.lea(R10, sk(SK_T1))
    A.mov_rm_imm32(("r", R8), 4)
    sysn(54)
    fd(RDI, SK_LFD)
    A.lea(RSI, sk(SK_SA))
    A.mov_rm_imm32(("r", RDX), 16)
    sysn(49)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_ski_bind")
    fd(RDI, SK_LFD)
    A.mov_rm_imm32(("r", RSI), 128)
    sysn(50)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_ski_bind")
    A.mov_rm_imm32(sk(SK_LEN), 16)
    fd(RDI, SK_LFD)
    A.lea(RSI, sk(SK_SA))
    A.lea(RDX, sk(SK_LEN))
    sysn(51)
    # the queue: SESSION_CAP + RECORD_CAP octets, reserved and committed only as written
    A.alu_r_rm("xor", RDI, ("r", RDI), 0)
    A.mov_r_imm64(RSI, SOCK_SESSION_CAP + SOCK_RECORD_CAP)
    A.mov_rm_imm32(("r", RDX), 3)
    A.mov_rm_imm32(("r", R10), 0x1002 if mac else 0x4022)   # MAP_PRIVATE | MAP_ANON(YMOUS) | (Linux) MAP_NORESERVE
    A.mov_rm_imm32(("r", R8), -1)
    A.alu_r_rm("xor", R9, ("r", R9), 0)
    sysn(9)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_ski_bind")
    A.mov_rm_r(sk(SK_Q), RAX)
    # the log: the clock header truncated away, the socket log's written (and the files grant line)
    A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
    A.alu_rm_imm("sub", ("r", RDI), 1)
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)
    sysn(77)                                        # ftruncate(fd, 0)
    A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
    A.alu_rm_imm("sub", ("r", RDI), 1)
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    sysn(8)                                         # lseek(fd, 0, SEEK_SET)
    A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
    A.alu_rm_imm("sub", ("r", RDI), 1)
    A.lea(RSI, ("abs", "sk_hdr", 0))
    A.mov_rm_imm32(("r", RDX), len(SOCKET_LOG_HEADER))
    sysn(1)
    A.lea(RDI, ("abs", "env_files", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_ski5")
    A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
    A.alu_rm_imm("sub", ("r", RDI), 1)
    A.lea(RSI, ("abs", "sk_grant", 0))
    A.mov_rm_imm32(("r", RDX), len(FILES_GRANT_LINE))
    sysn(1)
    A.label("rt_ski5")
    # YANTRA_SOCK_SENT=<file>: every TX octet is appended there; t1-run.sh puts its sha256 in the report
    A.lea(RDI, ("abs", "sk_env_sent", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_ski6")
    A.mov_r_rm(RDI, ("r", RAX))
    A.mov_rm_imm32(("r", RSI), 0x601 if mac else 0x241)   # O_WRONLY | O_CREAT | O_TRUNC
    A.mov_rm_imm32(("r", RDX), 420)
    sysn(2)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_ski_sent")                  # no side file, no honest digest: refused (8f low)
    A.lea(RAX, ("m", RAX, 1))
    A.mov_rm_r(sk(SK_SFD), RAX)
    A.label("rt_ski6")
    A.mov_rm_imm32(sk(SK_LATCH), SOCK_UNREAD)
    A.mov_rm_imm32(sk(SK_ON), 1)
    A.alu_rm_imm("cmp", sk(SK_RP), 0)
    A.jcc("ne", "rt_ski7")                          # a replay listens on nothing
    say(b"socket: listening on 127.0.0.1:")
    A.call("rt_sk_port")
    say(b"\n")
    A.label("rt_ski7")
    A.ret()
    # replay: the log is mapped read-only (cursor LFD, end POLL); the queue is the live one; no client
    # exists (SAID set so a TX never says so); records are decoded at each wait
    A.label("rt_skrp")
    A.alu_rm_imm("cmp", ("abs", "ev_slot", 0), 0)
    A.jcc("e", "rt_ski_noslot")
    A.mov_r_rm(RDI, ("r", RAX))
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    sysn(2)                                         # open(log, O_RDONLY)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_ski_rplog")
    A.mov_rm_r(sk(SK_T0), RAX)
    A.mov_r_rm(RDI, ("r", RAX))
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)
    A.mov_rm_imm32(("r", RDX), 2)
    sysn(8)                                         # lseek(fd, 0, SEEK_END): the length
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_ski_rplog")
    A.jcc("e", "rt_ski_rplog")
    A.mov_rm_r(sk(SK_POLL), RAX)
    A.mov_r_rm(RSI, ("r", RAX))
    A.alu_r_rm("xor", RDI, ("r", RDI), 0)
    A.mov_rm_imm32(("r", RDX), 1)
    A.mov_rm_imm32(("r", R10), 2)                   # PROT_READ, MAP_PRIVATE
    A.mov_r_rm(R8, sk(SK_T0))
    A.alu_r_rm("xor", R9, ("r", R9), 0)
    sysn(9)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_ski_rplog")
    A.mov_rm_r(sk(SK_LFD), RAX)
    A.alu_r_rm("add", RAX, sk(SK_POLL))
    A.mov_rm_r(sk(SK_POLL), RAX)                    # the end address
    A.mov_r_rm(RDI, sk(SK_T0))
    sysn(3)
    A.alu_r_rm("xor", RDI, ("r", RDI), 0)
    A.mov_r_imm64(RSI, SOCK_SESSION_CAP + SOCK_RECORD_CAP)
    A.mov_rm_imm32(("r", RDX), 3)
    A.mov_rm_imm32(("r", R10), 0x1002 if mac else 0x4022)
    A.mov_rm_imm32(("r", R8), -1)
    A.alu_r_rm("xor", R9, ("r", R9), 0)
    sysn(9)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_ski_rplog")
    A.mov_rm_r(sk(SK_Q), RAX)
    A.mov_rm_imm32(sk(SK_RP), 1)
    A.mov_rm_imm32(sk(SK_SAID), 1)
    A.jmp("rt_ski5")
    native_refuse("rt_ski_rplog", b"YANTRA_SOCK_REPLAY could not be opened or mapped")
    native_refuse("rt_ski_norec", b"YANTRA_LISTEN without YANTRA_RECORD_EVENTS: yantra-run takes --listen only right after --record-events <log>")
    native_refuse("rt_ski_noslot", b"a socket run on an image whose SASEVENT tag is missing or duplicated, or that is threaded")
    native_refuse("rt_ski_addr", b"YANTRA_LISTEN is not 127.0.0.1:PORT")
    native_refuse("rt_ski_bind", b"the socket listener could not be bound")
    native_refuse("rt_ski_sent", b"YANTRA_SOCK_SENT could not be opened (the sent digest would be a lie)")

    # ---- the port at sk+SA+2 (big-endian), as decimal
    A.label("rt_sk_port")
    A.movzx(RAX, sk(SK_SA + 2), 1)
    A.shift_imm("shl", ("r", RAX), 8)
    A.movzx(RCX, sk(SK_SA + 3), 1)
    A.alu_r_rm("or", RAX, ("r", RCX))
    A.jmp("rt_putu")

    # ---- the report line, at every exit once the listener is bound (yantra's socket::report_line);
    # the digest is t1-run.sh's (the sha256 of YANTRA_SOCK_SENT): the line carries a marker for it
    A.label("rt_sk_report")
    A.push(RDI)
    A.mov_rm_imm32(sk(SK_ON), 0)
    say(b"socket: ")
    A.mov_r_rm(RAX, sk(SK_TAIL))
    A.call("rt_putu")
    say(b" octets received in ")
    A.mov_r_rm(RAX, sk(SK_REC))
    A.call("rt_putu")
    say(b" records, ")
    A.mov_r_rm(RAX, sk(SK_SENT))
    A.call("rt_putu")
    say(b" sent, sent sha256 @YANTRA_SOCK_SENT@\n")
    A.pop(RDI)
    A.ret()

    # ---- a refusal of the window, by name: halt Device { pc, addr, why } (yantra's Debug), exit 1
    for name, text in SOCK_WHY:
        A.label("rt_sk_" + name)
        A.lea(RCX, ("abs", "sk_" + name, 0))
        A.mov_rm_imm32(("r", RDX), len(text))
        A.jmp("rt_sk_dev")
    A.label("rt_sk_dev")                            # rax = addr, rcx = why, rdx = its length
    A.mov_rm_r(sk(SK_T0), RAX)
    A.mov_rm_r(sk(SK_T1), RCX)
    A.mov_rm_r(sk(SK_T2), RDX)
    A.call("rt_flush")
    say(b"halt: Device { pc: ")
    pc_to_rax()
    A.call("rt_putu")
    say(b", addr: ")
    A.mov_r_rm(RAX, sk(SK_T0))
    A.call("rt_putu")
    say(b", why: \"")
    A.call("rt_sk_why")
    say(b"\" }\n")
    A.alu_rm_imm("cmp", sk(SK_ON), 0)
    A.jcc("ne", "rt_skd1")
    say(b"socket: touched at ")                    # yantra-run's line when the run has no device
    A.mov_r_rm(RAX, sk(SK_T0))
    A.call("rt_puthex")
    say(b" with no socket device \xe2\x80\x94 ")
    A.call("rt_sk_why")
    say(b"\n")
    A.label("rt_skd1")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_sk_why")
    A.mov_r_rm(RSI, sk(SK_T1))
    A.mov_r_rm(RDX, sk(SK_T2))
    A.mov_rm_imm32(("r", RDI), 2)
    sysn(1)
    A.ret()

    # ---- a load in the window: rax = addr, edx = width; RX answers rax = &latch (the order of
    # socket::load: no device, then width and alignment, then the register)
    A.label("rt_sk_ld")
    A.alu_rm_imm("cmp", sk(SK_ON), 0)
    A.jcc("e", "rt_sk_WHY_NO_DEVICE")
    A.alu_rm_imm("cmp", ("r", RDX), 4, 0)
    A.jcc("ne", "rt_sk_WHY_WIDTH")
    A.mov_r_rm(RCX, ("r", RAX))
    A.alu_rm_imm("sub", ("r", RCX), SOCK)
    A.op(b"\xf6", 0, ("r", RCX), 0)
    A.raw(3)                                        # test cl, 3
    A.jcc("ne", "rt_sk_WHY_WIDTH")
    A.alu_rm_imm("cmp", ("r", RCX), 4)
    A.jcc("ne", "rt_skl1")
    A.lea(RAX, sk(SK_LATCH))
    A.ret()
    A.label("rt_skl1")
    A.alu_rm_imm("cmp", ("r", RCX), 12)
    A.jcc("e", "rt_sk_WHY_CLOSE")
    A.jmp("rt_sk_WHY_LOAD_STORE_ONLY")

    # ---- a store in the window: rax = addr, rcx = value (socket::store's order); rt_store_slow
    # has spent edx, so the width is the guest's own word's (funct3 & 3: 2 is four octets)
    A.label("rt_sk_st")
    A.alu_rm_imm("cmp", sk(SK_ON), 0)
    A.jcc("e", "rt_sk_WHY_NO_DEVICE")
    A.push(RCX)
    A.mov_r_rm(RDX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RDX, ("r", RCX))
    A.mov_r_rm(RDX, ("m", RDX, 0), 0)
    A.shift_imm("shr", ("r", RDX), 12, 0)
    A.alu_rm_imm("and", ("r", RDX), 3, 0)
    A.alu_rm_imm("cmp", ("r", RDX), 2, 0)
    A.pop(RCX)
    A.jcc("ne", "rt_sk_WHY_WIDTH")
    A.mov_r_rm(RDX, ("r", RAX))
    A.alu_rm_imm("sub", ("r", RDX), SOCK)
    A.op(b"\xf6", 0, ("r", RDX), 0)
    A.raw(3)                                        # test dl, 3
    A.jcc("ne", "rt_sk_WHY_WIDTH")
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "rt_sk_next")
    A.alu_rm_imm("cmp", ("r", RDX), 8)
    A.jcc("e", "rt_sk_tx")
    A.alu_rm_imm("cmp", ("r", RDX), 12)
    A.jcc("e", "rt_sk_WHY_CLOSE")
    A.jmp("rt_sk_WHY_STORE_LOAD_ONLY")
    # NEXT: the low 32 bits must be 0; the queue's head into the latch, else EMPTY or END
    A.label("rt_sk_next")
    A.test_rm_r(("r", RCX), RCX, 0)
    A.jcc("ne", "rt_sk_WHY_NEXT_VALUE")
    A.mov_r_rm(RDX, sk(SK_HEAD))
    A.alu_r_rm("cmp", RDX, sk(SK_TAIL))
    A.jcc("ae", "rt_skn1")
    A.mov_r_rm(RAX, sk(SK_Q))
    A.alu_r_rm("add", RAX, ("r", RDX))
    A.movzx(RAX, ("m", RAX, 0), 1)
    A.lea(RDX, ("m", RDX, 1))
    A.mov_rm_r(sk(SK_HEAD), RDX)
    A.mov_rm_r(sk(SK_LATCH), RAX)
    A.ret()
    A.label("rt_skn1")
    A.mov_rm_imm32(("r", RAX), SOCK_EMPTY)
    A.alu_r_rm("add", RAX, sk(SK_END))             # END = EMPTY + 1, sticky
    A.mov_rm_r(sk(SK_LATCH), RAX)
    A.ret()
    # TX: the low 32 bits at most 0xff; counted, appended to YANTRA_SOCK_SENT, written to the client
    # at once (MSG_NOSIGNAL: a closed peer is EPIPE, as Rust, which ignores SIGPIPE); a failure is
    # said once and never reaches the program
    A.label("rt_sk_tx")
    A.alu_rm_imm("cmp", ("r", RCX), 0xFF, 0)
    A.jcc("a", "rt_sk_WHY_TX_VALUE")
    A.mov_rm_r(sk(SK_B), RCX, width=1)
    for r in (RSI, RDI, R8, R9, R10, R11):
        A.push(r)
    A.mov_r_rm(RAX, sk(SK_SENT))
    A.mov_rm_r(sk(SK_T0), RAX)                      # before: the octets sent before this one
    A.alu_rm_imm("add", sk(SK_SENT), 1)
    A.alu_rm_imm("cmp", sk(SK_SFD), 0)
    A.jcc("e", "rt_skt1")
    fd(RDI, SK_SFD)
    A.lea(RSI, sk(SK_B))
    A.mov_rm_imm32(("r", RDX), 1)
    sysn(1)
    A.label("rt_skt1")
    A.alu_rm_imm("cmp", sk(SK_CFD), 0)
    A.jcc("ne", "rt_skt2")
    A.alu_rm_imm("cmp", sk(SK_SAID), 0)
    A.jcc("ne", "rt_skt9")
    A.mov_rm_imm32(sk(SK_SAID), 1)
    say(b"socket: send failed after ")
    A.mov_r_rm(RAX, sk(SK_T0))
    A.call("rt_putu")
    say(b" octets \xe2\x80\x94 no client has connected yet (one is accepted at the first wait); "
        b"the program is not told (W-377)\n")
    A.jmp("rt_skt9")
    A.label("rt_skt2")
    A.alu_rm_imm("cmp", sk(SK_FAIL), 0)
    A.jcc("ne", "rt_skt9")
    A.label("rt_skt3")
    fd(RDI, SK_CFD)
    A.lea(RSI, sk(SK_B))
    A.mov_rm_imm32(("r", RDX), 1)
    A.mov_rm_imm32(("r", R10), 0 if mac else 0x4000)   # MSG_NOSIGNAL (Darwin: SO_NOSIGPIPE at accept)
    A.alu_r_rm("xor", R8, ("r", R8), 0)
    A.alu_r_rm("xor", R9, ("r", R9), 0)
    sysn(44)
    A.alu_rm_imm("cmp", ("r", RAX), -4)            # EINTR: again, as write_all
    A.jcc("e", "rt_skt3")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ns", "rt_skt9")
    A.mov_rm_imm32(sk(SK_FAIL), 1)
    A.f7(3, ("r", RAX))                             # neg rax: the errno
    A.mov_rm_r(sk(SK_T1), RAX)
    say(b"socket: send failed after ")
    A.mov_r_rm(RAX, sk(SK_T0))
    A.call("rt_putu")
    say(b" octets \xe2\x80\x94 ")
    A.call("rt_sk_err")
    say(b"; nothing more is written, and the program is not told (W-377)\n")
    A.label("rt_skt9")
    for r in (R11, R10, R9, R8, RDI, RSI):
        A.pop(r)
    A.ret()

    # ---- an errno (sk+T1) as Rust's io::Error says it: "<strerror> (os error N)" for the two a
    # connection meets (EPIPE, ECONNRESET), "os error N" for any other
    A.label("rt_sk_err")
    for n, text in ((32, b"Broken pipe"), (54 if mac else 104, b"Connection reset by peer")):
        A.alu_rm_imm("cmp", sk(SK_T1), n)
        A.jcc("ne", "rt_ske%d" % n)
        say(text + b" (")
        A.label("rt_ske%d" % n)
    say(b"os error ")
    A.mov_r_rm(RAX, sk(SK_T1))
    A.call("rt_putu")
    for n, text in ((32, b"Broken pipe"), (54 if mac else 104, b"Connection reset by peer")):
        A.alu_rm_imm("cmp", sk(SK_T1), n)
        A.jcc("ne", "rt_skf%d" % n)
        say(b")")
        A.label("rt_skf%d" % n)
    A.ret()

    # ---- THE WAIT, a socket run's (yantra-run's serve_live): a wait after END is refused; the first
    # wait accepts ONE client within 30 s and closes the listener; every wait reads once (up to
    # RECORD_CAP octets, within 30 s), checks the session cap, logs `s=<hex>` or `s=end`, appends
    # to the queue and writes the octet count at tag + 8
    A.label("rt_sk_wait")
    A.alu_rm_imm("cmp", sk(SK_END), 0)
    A.jcc("ne", "rt_skr_end")
    A.alu_rm_imm("cmp", sk(SK_RP), 0)
    A.jcc("ne", "rt_skrw")
    A.alu_rm_imm("cmp", sk(SK_CFD), 0)
    A.jcc("ne", "rt_skw2")
    A.mov_r_rm(RAX, sk(SK_LFD))
    A.alu_rm_imm("sub", ("r", RAX), 1)
    A.mov_rm_r(sk(SK_POLL), RAX, width=4)
    A.mov_rm_imm32(sk(SK_POLL + 4), 1, 0)          # POLLIN
    A.label("rt_skw1")
    A.lea(RDI, sk(SK_POLL))
    A.mov_rm_imm32(("r", RSI), 1)
    A.mov_rm_imm32(("r", RDX), 30000)
    sysn(7)
    A.alu_rm_imm("cmp", ("r", RAX), -4)
    A.jcc("e", "rt_skw1")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_skr_idle1")
    A.jcc("s", "rt_skr_accept")
    A.mov_rm_imm32(sk(SK_LEN), 16)
    fd(RDI, SK_LFD)
    A.lea(RSI, sk(SK_SA))
    A.lea(RDX, sk(SK_LEN))
    sysn(43)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_skr_accept")
    A.lea(RCX, ("m", RAX, 1))
    A.mov_rm_r(sk(SK_CFD), RCX)
    A.mov_rm_imm32(sk(SK_T1), 1)
    A.mov_r_rm(RDI, ("r", RAX))
    A.mov_rm_imm32(("r", RSI), 6)                  # IPPROTO_TCP, TCP_NODELAY (as yantra-run)
    A.mov_rm_imm32(("r", RDX), 1)
    A.lea(R10, sk(SK_T1))
    A.mov_rm_imm32(("r", R8), 4)
    sysn(54)
    if mac:                                         # SO_NOSIGPIPE: a closed peer is EPIPE, not SIGPIPE
        fd(RDI, SK_CFD)
        A.mov_rm_imm32(("r", RSI), 0xFFFF)
        A.mov_rm_imm32(("r", RDX), 0x1022)
        A.lea(R10, sk(SK_T1))
        A.mov_rm_imm32(("r", R8), 4)
        sysn(54)
    say(b"socket: accepted ")
    for i in range(4):
        A.movzx(RAX, sk(SK_SA + 4 + i), 1)
        A.call("rt_putu")
        say(b"." if i < 3 else b":")
    A.call("rt_sk_port")
    say(b"\nsocket: the listener is closed; a second client is refused at connect\n")
    fd(RDI, SK_LFD)
    sysn(3)
    A.mov_rm_imm32(sk(SK_LFD), 0)
    A.label("rt_skw2")
    A.mov_r_rm(RAX, sk(SK_CFD))
    A.alu_rm_imm("sub", ("r", RAX), 1)
    A.mov_rm_r(sk(SK_POLL), RAX, width=4)
    A.mov_rm_imm32(sk(SK_POLL + 4), 1, 0)
    A.label("rt_skw3")
    A.lea(RDI, sk(SK_POLL))
    A.mov_rm_imm32(("r", RSI), 1)
    A.mov_rm_imm32(("r", RDX), 30000)
    sysn(7)
    A.alu_rm_imm("cmp", ("r", RAX), -4)
    A.jcc("e", "rt_skw3")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_skr_idle")
    A.jcc("s", "rt_skr_read")
    A.label("rt_skw4")
    fd(RDI, SK_CFD)
    A.mov_r_rm(RSI, sk(SK_Q))
    A.alu_r_rm("add", RSI, sk(SK_TAIL))
    A.mov_rm_imm32(("r", RDX), SOCK_RECORD_CAP)
    sysn(0)
    A.alu_rm_imm("cmp", ("r", RAX), -4)
    A.jcc("e", "rt_skw4")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_skr_read")
    A.jcc("e", "rt_skw_end")
    # the session cap, before the record is logged or applied
    A.mov_r_rm(RCX, sk(SK_TAIL))
    A.alu_r_rm("add", RCX, ("r", RAX))
    A.mov_rm_r(sk(SK_T0), RAX)
    A.mov_rm_r(sk(SK_T1), RCX)
    A.mov_rm_imm32(("r", RDX), SOCK_SESSION_CAP)
    A.alu_r_rm("cmp", RCX, ("r", RDX))
    A.jcc("a", "rt_skr_cap")
    # the record's line: "s=" + two lowercase hex digits per octet + "\n", one write
    A.mov_r_rm(R8, ("r", RAX))
    A.mov_r_rm(RSI, sk(SK_Q))
    A.alu_r_rm("add", RSI, sk(SK_TAIL))
    A.lea(RDI, ("abs", "sk_line", 0))
    A.mov_rm_imm32(("m", RDI, 0), 0x3D73, 0)        # "s="
    A.lea(RDI, ("m", RDI, 2))
    A.label("rt_skh1")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.mov_r_rm(RCX, ("r", RAX))
    A.shift_imm("shr", ("r", RCX), 4)
    nib("rt_skh2")
    A.mov_rm_r(("m", RDI, 0), RCX, width=1)
    A.mov_r_rm(RCX, ("r", RAX))
    A.alu_rm_imm("and", ("r", RCX), 15)
    nib("rt_skh3")
    A.mov_rm_r(("m", RDI, 1), RCX, width=1)
    A.lea(RDI, ("m", RDI, 2))
    A.lea(RSI, ("m", RSI, 1))
    A.alu_rm_imm("sub", ("r", R8), 1)
    A.jcc("ne", "rt_skh1")
    A.mov_rm_imm32(("r", RCX), 10)
    A.mov_rm_r(("m", RDI, 0), RCX, width=1)
    A.lea(RDX, ("m", RDI, 1))
    A.lea(RSI, ("abs", "sk_line", 0))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
    A.alu_rm_imm("sub", ("r", RDI), 1)
    sysn(1)
    A.mov_r_rm(RAX, sk(SK_T0))
    A.mov_r_rm(RCX, sk(SK_T1))
    A.mov_rm_r(sk(SK_TAIL), RCX)
    A.jmp("rt_skw_dl")
    A.label("rt_skw_end")
    A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
    A.alu_rm_imm("sub", ("r", RDI), 1)
    A.lea(RSI, ("abs", "sk_send", 0))
    A.mov_rm_imm32(("r", RDX), 6)
    sysn(1)
    A.mov_rm_imm32(sk(SK_END), 1)
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.label("rt_skw_dl")                            # deliver: the count at tag + 8
    A.mov_r_rm(RCX, ("abs", "ev_slot", 0))
    A.mov_rm_r(("m", RCX, 8), RAX)
    A.alu_rm_imm("add", sk(SK_REC), 1)
    A.jmp("rt_wt4")

    # ---- the replayed wait: the next record of the mapped socket log (t1-run.sh has validated it).
    # `#` and blank lines are skipped; the end of the log is the SHORTER refusal; `s=end` is END (a
    # count of 0); `s=<hex>` is decoded into the queue after the session-cap check
    A.label("rt_skrw")
    A.mov_r_rm(RSI, sk(SK_LFD))
    A.label("rt_skrw1")
    A.alu_r_rm("cmp", RSI, sk(SK_POLL))
    A.jcc("ae", "rt_skr_short")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.alu_rm_imm("cmp", ("r", RAX), 10)
    A.jcc("e", "rt_skrw2")
    A.alu_rm_imm("cmp", ("r", RAX), 35)             # '#'
    A.jcc("ne", "rt_skrw4")
    A.label("rt_skrw3")                             # skip to the end of the line
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.lea(RSI, ("m", RSI, 1))
    A.alu_rm_imm("cmp", ("r", RAX), 10)
    A.jcc("ne", "rt_skrw3")
    A.jmp("rt_skrw1")
    A.label("rt_skrw2")
    A.lea(RSI, ("m", RSI, 1))
    A.jmp("rt_skrw1")
    A.label("rt_skrw4")
    A.movzx(RAX, ("m", RSI, 3), 1)
    A.alu_rm_imm("cmp", ("r", RAX), 110)            # "s=en": s=end (n is no hex digit)
    A.jcc("ne", "rt_skrw5")
    A.lea(RSI, ("m", RSI, 6))
    A.jmp("rt_skrw9")
    A.label("rt_skrw5")
    A.lea(RSI, ("m", RSI, 2))
    A.mov_r_rm(RDI, sk(SK_Q))
    A.alu_r_rm("add", RDI, sk(SK_TAIL))
    A.alu_r_rm("xor", R8, ("r", R8), 0)             # the octets of this record
    A.label("rt_skrw6")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.alu_rm_imm("cmp", ("r", RAX), 10)
    A.jcc("e", "rt_skrw8")
    A.movzx(RCX, ("m", RSI, 1), 1)
    for r, lab in ((RAX, "rt_skrwa"), (RCX, "rt_skrwb")):
        A.alu_rm_imm("sub", ("r", r), 48)
        A.alu_rm_imm("cmp", ("r", r), 9)
        A.jcc("be", lab)
        A.alu_rm_imm("sub", ("r", r), 39)
        A.label(lab)
    A.shift_imm("shl", ("r", RAX), 4)
    A.alu_r_rm("or", RAX, ("r", RCX))
    A.mov_rm_r(("m", RDI, 0), RAX, width=1)
    A.lea(RDI, ("m", RDI, 1))
    A.lea(RSI, ("m", RSI, 2))
    A.lea(R8, ("m", R8, 1))
    A.jmp("rt_skrw6")
    A.label("rt_skrw8")
    A.lea(RSI, ("m", RSI, 1))
    A.mov_r_rm(RCX, sk(SK_TAIL))
    A.alu_r_rm("add", RCX, ("r", R8))
    A.mov_rm_r(sk(SK_T0), R8)
    A.mov_rm_r(sk(SK_T1), RCX)
    A.mov_rm_r(sk(SK_LFD), RSI)
    A.mov_rm_imm32(("r", RDX), SOCK_SESSION_CAP)
    A.alu_r_rm("cmp", RCX, ("r", RDX))
    A.jcc("a", "rt_skr_cap")
    A.mov_rm_r(sk(SK_TAIL), RCX)
    A.mov_r_rm(RAX, sk(SK_T0))
    A.jmp("rt_skw_dl")
    A.label("rt_skrw9")                             # s=end: the peer closed
    A.mov_rm_r(sk(SK_LFD), RSI)
    A.mov_rm_imm32(sk(SK_END), 1)
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.jmp("rt_skw_dl")

    # ---- the wait's refusals (yantra-run's words): halt Wait { pc }, the report line, then
    # "events: REFUSED — <why>", exit 1. A refusal after the listener was bound is TERMINAL.
    def refuse_head(lab):
        A.label(lab)
        A.call("rt_sk_rhead")

    def wait_at():                          # "wait index N (the store at pc 0x…)"
        say(b"wait index ")
        A.mov_r_rm(RAX, sk(SK_REC))
        A.call("rt_putu")
        say(b" (the store at pc ")
        pc_to_rax()
        A.call("rt_puthex")
        say(b")")
    A.label("rt_sk_rhead")
    A.call("rt_flush")
    say(b"halt: Wait { pc: ")
    pc_to_rax()
    A.call("rt_putu")
    say(b" }\n")
    A.call("rt_sk_report")
    if wm:
        A.call("rt_wm_report")              # yantra-run says `ram: high water` BEFORE `events: REFUSED`
    say(b"events: REFUSED \xe2\x80\x94 ")
    A.ret()
    refuse_head("rt_skr_end")
    say(b"a WAIT after END: ")
    wait_at()
    say(b" waited after the peer closed (s=end), and there is no other source to wait for (W-377)")
    A.jmp("rt_skr_tail")
    refuse_head("rt_skr_short")
    say(b"the socket log is SHORTER than the run: ")
    wait_at()
    say(b" has no record; the log holds ")
    A.mov_r_rm(RAX, sk(SK_REC))
    A.call("rt_putu")
    say(b" \xe2\x80\x94 the program was NOT resumed, rather than let it read octets nobody delivered")
    A.jmp("rt_skr_tail")
    refuse_head("rt_skr_idle1")
    say(b"the socket was IDLE for more than 30 s at the first wait: no client connected (W-377)")
    A.jmp("rt_skr_tail")
    A.label("rt_skr_accept")
    A.f7(3, ("r", RAX))
    A.mov_rm_r(sk(SK_T1), RAX)
    A.call("rt_sk_rhead")
    say(b"accepting the client failed: ")
    A.call("rt_sk_err")
    say(b" (W-377)")
    A.jmp("rt_skr_tail")
    refuse_head("rt_skr_idle")
    say(b"the socket was IDLE for more than 30 s at ")
    wait_at()
    say(b": nothing arrived and the peer did not close (W-377)")
    A.jmp("rt_skr_tail")
    A.label("rt_skr_read")
    A.f7(3, ("r", RAX))
    A.mov_rm_r(sk(SK_T1), RAX)
    A.call("rt_sk_rhead")
    say(b"the read at ")
    wait_at()
    say(b" failed: ")
    A.call("rt_sk_err")
    say(b" (W-377)")
    A.jmp("rt_skr_tail")
    refuse_head("rt_skr_cap")
    say(SOCKET_LIMIT_EXCEEDED + b" (SOCKET_LIMIT_EXCEEDED): record ")
    A.mov_r_rm(RAX, sk(SK_REC))
    A.call("rt_putu")
    say(b" (")
    A.mov_r_rm(RAX, sk(SK_T0))
    A.call("rt_putu")
    say(b" octets) would take the session's delivered records past the cap of %d octets (64 MiB), to "
        % SOCK_SESSION_CAP)
    A.mov_r_rm(RAX, sk(SK_T1))
    A.call("rt_putu")
    say(b" \xe2\x80\x94 refused, not applied and not logged (W-377)")
    A.label("rt_skr_tail")
    say(b"\n")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit_nw" if wm else "rt_exit")   # the watermark line is already out (rt_sk_rhead)

    for lab, text in strs + [("sk_" + n, t) for n, t in SOCK_WHY] + [
            ("sk_env_listen", b"YANTRA_LISTEN=\0"), ("sk_env_sent", b"YANTRA_SOCK_SENT=\0"),
            ("sk_env_replay", b"YANTRA_SOCK_REPLAY=\0"),
            ("sk_pfx", b"127.0.0.1:"), ("sk_hdr", SOCKET_LOG_HEADER), ("sk_grant", FILES_GRANT_LINE),
            ("sk_send", b"s=end\n")]:
        A.label(lab)
        A.raw(text)


# ---------------------------------------------------------------- THE FRAMES DEVICE (NET.md)
# yantra's virtio-net at slot 2 (netdev, virtqueue) with yantra-run's --net-peer backend, x86-64
# Linux only, compiled in only when the translation asks (ANUVADA_NET=1: t1-run's net branch), and live
# only when YANTRA_NET_PEER is set: with neither, every octet and every answer is what it was.
NET_LO = 0x1000_3000
R12, R13, R14, R15 = 12, 13, 14, 15
NT_FIELDS = [("on", 8), ("fd", 8), ("listen", 8), ("maxf", 8), ("maxtx", 8), ("maxrx", 8), ("filter", 8),
             ("tmo", 8), ("rpl", 8), ("mac", 8), ("cfg", 16), ("status", 8), ("hfsel", 8), ("gfsel", 8), ("dfeat", 8),
             ("page", 8), ("qsel", 8), ("isr", 8), ("q0", 64), ("q1", 64),
             ("txf", 8), ("rxseen", 8), ("rxf", 8), ("drop", 8), ("filt", 8), ("k", 8), ("recs", 8),
             ("failed", 8), ("val", 8), ("sidefd", 8), ("oblen", 8), ("addr", 8), ("head", 8), ("segn", 8),
             ("rlen", 8), ("wlen", 8), ("deadline", 8), ("tmp", 8), ("tmp2", 8), ("cbwhy", 8), ("served", 8),
             ("pathlen", 8), ("rxlen", 8), ("placed", 8), ("got", 8), ("waddr", 8), ("lenbuf", 8),
             ("dec", 32), ("pollfd", 16), ("ts", 16), ("sa", 112), ("seg", 256 * 16),
             ("rxbuf", 65536 + 32), ("line", 2 * 65536 + 128), ("outbox", 262144)]
NT = {}
NT_SIZE = 0
for _n, _s in NT_FIELDS:
    NT[_n] = NT_SIZE
    NT_SIZE += _s
NT_OUTBOX = 262144
# YANTRA_VIRTIO_DEFER (L33): the deferral's state follows the base block, so a translation without it keeps
# every offset; only a vdf translation sizes "nt" to cover it. stg: a ring of NTV_SLOTS staged frames, each
# {due, length, frame (at most 65535 octets)}
NTV_SLOTS, NTV_SLOT = 512, 16 + 65536
NTV_FIELDS = [("vd", 8), ("vclk", 8), ("ptx", 8), ("txdue", 8), ("ppc", 8), ("vwait", 8), ("stn", 8),
              ("sth", 8), ("stt", 8), ("stg", NTV_SLOTS * NTV_SLOT)]
NTV_SIZE = 0
for _n, _s in NTV_FIELDS:
    NT[_n] = NT_SIZE + NTV_SIZE
    NTV_SIZE += _s
# a queue's block in nt: QueueNum, QueueAlign, QueuePFN, the last available index seen, and the
# layout QueuePFN fixed (legacy_layout)
NQ_NUM, NQ_ALIGN, NQ_PFN, NQ_LAST, NQ_DESC, NQ_AVAIL, NQ_USED = 0, 8, 16, 24, 32, 40, 48

NET_FRAME_TOO_LARGE = "पिण्डातिदीर्घनिषेधः (0x360): a frame is longer than --net-max-frame (default 1514 octets)"
NET_TX_COUNT_EXCEEDED = ("पिण्डसङ्ख्यातिक्रमः (0x361): the run has transmitted more frames than "
                         "--net-max-tx-frames")
NET_RX_COUNT_EXCEEDED = ("पिण्डसङ्ख्यातिक्रमः (0x361): the run has received more frames than "
                         "--net-max-rx-frames")
NET_FRAME_RUNT = "पिण्डावैधरूपम् (0x363): a frame is shorter than an Ethernet header (14 octets)"
NET_TX_NO_HEADER = "पिण्डावैधरूपम् (0x363): a transmit buffer is shorter than the 10-octet virtio_net_hdr"
NET_SRC_MAC_REFUSED = ("उद्गमसम्बन्धनिषेधः (0x368): a transmitted frame's source MAC is not the "
                       "--net-mac allow-listed one")
NET_TIMEOUT = "सञ्चारकालातीतम् (0x369): no frame arrived within --net-timeout"
NET_PEER_CLOSED = "प्रतिपक्षविच्छिन्नम् (0x36a): the frame peer closed the connection"
NET_WHY = {
    "width": "virtio-net control registers take 32-bit aligned accesses only (virtio 1.2 §4.2.2); its "
             "config space takes 1, 2 or 4 octets within its 12 octets",
    "readw": "read of a write-only virtio-mmio register (virtio 1.2 §4.2.4 table, direction W)",
    "writer": "write to a read-only virtio-mmio register of the virtio-net device",
    "noreg": "no register at this virtio-mmio offset in the legacy table (virtio 1.2 §4.2.4)",
    "feature": "virtio-net GuestFeatures acknowledges a feature bit this device does not offer (it offers "
               "VIRTIO_NET_F_MTU, _MAC and _STATUS only)",
    "inuse": "virtio-net QueueNum, QueueAlign or GuestPageSize written while a queue is in use (QueuePFN is "
             "nonzero); write QueuePFN 0 first",
    "cfgro": "write to the virtio-net config space: it is read-only (virtio 1.2 §5.1.4: the driver MUST NOT "
             "write the MAC)",
    "qnmax": "virtio-net QueueNumMax read while the selected queue is in use (QueuePFN is nonzero)",
    "page": "virtio-net GuestPageSize must be a power of 2 (virtio 1.2 §4.2.4 table)",
    "qnum_noq": "virtio-net QueueNum for a queue this device does not have",
    "qnum": "virtio-net QueueNum must be a power of 2, at least 1 and at most QueueNumMax",
    "qalign_noq": "virtio-net QueueAlign for a queue this device does not have",
    "qalign": "virtio-net QueueAlign must be a power of 2 (virtio 1.2 §4.2.4 table)",
    "qpfn_noq": "virtio-net QueuePFN for a queue this device does not have",
    "qpfn_page": "virtio-net QueuePFN written before GuestPageSize",
    "qpfn_num": "virtio-net QueuePFN written before QueueNum and QueueAlign",
    "qpfn_out": "virtio-net QueuePFN places the queue's areas outside RAM",
    "notify_noq": "virtio-net QueueNotify names a queue this device does not have",
    "notify_nopfn": "virtio-net QueueNotify for a queue that is not set up (its QueuePFN is 0)",
}
# the walker's answers 2.. and the TX callback's refusals, by number
NET_VQ = [(2, "the queue has size zero"), (3, "a ring or buffer does not lie wholly in RAM"),
          (4, "the available ring offered a head outside the descriptor table"),
          (5, "a descriptor's next is outside the descriptor table"),
          (6, "the chain is longer than the queue, so it loops"),
          (7, "a device-readable descriptor follows a device-writable one"),
          (8, "an INDIRECT descriptor, a feature this device does not offer"),
          (9, "the chain's buffers add up to more than 2^32 bytes"),
          (10, "the answer is longer than the chain's writable space")]
NET_CB = [(20, NET_TX_NO_HEADER), (21, NET_FRAME_RUNT), (22, NET_FRAME_TOO_LARGE), (23, NET_TX_COUNT_EXCEEDED),
          (24, NET_SRC_MAC_REFUSED)]
NET_ESC = [(0x80, 0xa0), (0xad, 0xad), (0x300, 0x36f), (0x900, 0x902), (0x93a, 0x93a), (0x93c, 0x93c),
           (0x941, 0x948), (0x94d, 0x94d), (0x951, 0x957), (0x962, 0x963), (0x1680, 0x1680), (0x2000, 0x200f),
           (0x2028, 0x202f), (0x205f, 0x206f), (0x3000, 0x3000), (0xfeff, 0xfeff)]
NET_HDR_TAIL = (" defer=0 backend=peer — one n= record per wait, n=<wait>:<handle>:<hex frame|none>; "
                "replay with --events\n")


def rust_debug(s):
    """a &str as Rust's {:?} writes it (the halt line's why), for the code points rt_esctab knows"""
    out = []
    for ch in s:
        c = ord(ch)
        if ch == '"':
            out.append('\\"')
        elif ch == "\\":
            out.append("\\\\")
        elif ch == "\t":
            out.append("\\t")
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\r":
            out.append("\\r")
        elif ch == "\0":
            out.append("\\0")
        elif c < 0x20 or c == 0x7f or any(lo <= c <= hi for lo, hi in NET_ESC):
            out.append("\\u{%x}" % c)
        else:
            out.append(ch)
    return '"' + "".join(out) + '"'


def runtime_net(A, base, vdf=False):
    """THE FRAMES DEVICE, x86-64 Linux (NET.md; yantra's netdev + virtqueue, yantra-run's open_peer
    and record_live_net). Entered from runtime(): rt_nt_init (load time; YANTRA_NET_PEER unset: nothing),
    rt_nt_ld / rt_nt_st (the slow paths, slot 2: rax = addr, rcx = value, edx = width; a load returns
    rax = the value's address), rt_nt_wait (from rt_wait, rsi rdi r8..r11 pushed; leaves through rt_wt4),
    rt_nt_flush (from rt_flush: the TX outbox to the peer and the side file) and rt_nt_report (rt_exit).
    Once the guest runs every register but rax rcx rdx is the guest's: the store and wait paths save the
    rest (SAVE) before they use them."""
    strs = []
    cnt = [0]
    SAVE = (RBX, RBP, R12, R13, R14, R15, RSI, RDI, R8, R9, R10, R11)

    def L(p="l"):
        cnt[0] += 1
        return "rt_nt_%s%d" % (p, cnt[0])

    def f(name, add=0):
        return ("abs", "nt", NT[name] + add)

    def sysn(n):
        A.mov_rm_imm32(("r", RAX), n)
        A.syscall()

    def text(t):
        return t.encode() if isinstance(t, str) else t

    def say(t):                           # write(2, t): clobbers rax rcx rdx rsi rdi r11
        t = text(t)
        lab = "nt_s%d" % len(strs)
        strs.append((lab, t))
        A.lea(RSI, ("abs", lab, 0))
        A.mov_rm_imm32(("r", RDX), len(t))
        A.mov_rm_imm32(("r", RDI), 2)
        sysn(1)

    def cpy(t):                           # t at rdi, rdi advanced: clobbers rcx rsi
        t = text(t)
        lab = "nt_s%d" % len(strs)
        strs.append((lab, t))
        A.lea(RSI, ("abs", lab, 0))
        A.mov_rm_imm32(("r", RCX), len(t))
        A.raw(0xF3, 0xA4)                 # rep movsb

    def inc(name):
        A.mov_r_rm(RAX, f(name))
        A.alu_rm_imm("add", ("r", RAX), 1)
        A.mov_rm_r(f(name), RAX)

    def pc_rax(add=0):
        A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
        A.mov_r_imm64(RCX, base + add)
        A.alu_r_rm("add", RAX, ("r", RCX))

    def native_refuse(lab, t):            # before the guest runs: t1-run reruns it whole
        A.label(lab)
        say(b"native: not in milestone 1 (" + text(t) + b"; run it under yantra-run)\n")
        A.mov_rm_imm32(("r", RDI), 2)
        A.jmp("rt_exit")

    def nib(lab):                         # rcx = a nibble -> its lowercase hex digit
        A.alu_rm_imm("cmp", ("r", RCX), 10)
        A.jcc("b", lab)
        A.alu_rm_imm("add", ("r", RCX), 39)
        A.label(lab)
        A.alu_rm_imm("add", ("r", RCX), 48)

    def pow2_or(r, fail):                 # r nonzero and a power of 2, else jump to fail (rax clobbered)
        A.test_rm_r(("r", r), r)
        A.jcc("e", fail)
        A.lea(RAX, ("m", r, -1))
        A.test_rm_r(("r", RAX), r)
        A.jcc("ne", fail)

    def say_path():
        A.lea(RSI, f("sa", 2))
        A.mov_r_rm(RDX, f("pathlen"))
        A.mov_rm_imm32(("r", RDI), 2)
        sysn(1)

    # ================= rt_nt_init: the backend, before the first instruction (every register free)
    A.label("rt_nt_init")
    A.lea(RDI, ("abs", "nt_env_peer", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_nti1")
    # YANTRA_NET_REPLAY=<frames file>: the logged n= records replayed from a regular file (no peer, no log)
    A.lea(RDI, ("abs", "nt_env_replay", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_nti_rp0")
    A.ret()
    A.label("rt_nti_rp0")
    A.mov_rm_imm32(f("rpl"), 1)
    A.alu_rm_imm("cmp", ("abs", "ev_slot", 0), 0)
    A.jcc("e", "rt_nti_noslot")
    A.jmp("rt_nti_cfg")
    A.label("rt_nti1")
    A.mov_r_rm(RBX, ("r", RAX))
    A.alu_rm_imm("cmp", ("abs", "rec_fd", 0), 0)
    A.jcc("e", "rt_nti_norec")
    A.alu_rm_imm("cmp", ("abs", "ev_slot", 0), 0)
    A.jcc("e", "rt_nti_noslot")
    # sockaddr_un: AF_UNIX, the path (1..107 octets, NUL ended)
    A.mov_rm_imm32(f("sa"), 1, 0)
    A.lea(RDI, f("sa", 2))
    A.mov_r_rm(RSI, ("r", RBX))
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.label("rt_nti2")
    A.movzx(RAX, ("m", RSI, 0), 1)
    A.mov_rm_r(("m", RDI, 0), RAX, width=1)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_nti3")
    A.lea(RSI, ("m", RSI, 1))
    A.lea(RDI, ("m", RDI, 1))
    A.lea(RCX, ("m", RCX, 1))
    A.alu_rm_imm("cmp", ("r", RCX), 107)
    A.jcc("a", "rt_nti_path")
    A.jmp("rt_nti2")
    A.label("rt_nti3")
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("e", "rt_nti_path")
    A.mov_rm_r(f("pathlen"), RCX)
    A.label("rt_nti_cfg")
    # the MAC: 12 lowercase hex digits (t1-run checked and lowercased them)
    A.lea(RDI, ("abs", "nt_env_mac", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_nti_env")
    A.mov_r_rm(RSI, ("r", RAX))
    A.lea(RDI, f("mac"))
    A.mov_rm_imm32(("r", R9), 6)
    A.label("rt_nti4")
    for half in (0, 1):
        A.movzx(RCX, ("m", RSI, half), 1)
        A.alu_rm_imm("sub", ("r", RCX), 48)
        A.alu_rm_imm("cmp", ("r", RCX), 9)
        lab = L("hx")
        A.jcc("be", lab)
        A.alu_rm_imm("sub", ("r", RCX), 39)
        A.label(lab)
        A.alu_rm_imm("cmp", ("r", RCX), 15)
        A.jcc("a", "rt_nti_env")
        if half == 0:
            A.mov_r_rm(RAX, ("r", RCX))
            A.shift_imm("shl", ("r", RAX), 4)
        else:
            A.alu_r_rm("or", RAX, ("r", RCX))
    A.mov_rm_r(("m", RDI, 0), RAX, width=1)
    A.lea(RSI, ("m", RSI, 2))
    A.lea(RDI, ("m", RDI, 1))
    A.alu_rm_imm("sub", ("r", R9), 1)
    A.jcc("ne", "rt_nti4")
    for lab, field in (("nt_env_filter", "filter"), ("nt_env_maxf", "maxf"), ("nt_env_maxtx", "maxtx"),
                       ("nt_env_maxrx", "maxrx"), ("nt_env_tmo", "tmo")):
        A.lea(RDI, ("abs", lab, 0))
        A.call("rt_getenv")
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "rt_nti_env")
        A.mov_r_rm(RSI, ("r", RAX))
        A.call("rt_atou")
        A.mov_rm_r(f(field), RAX)
    # the config space: mac, status LINK_UP, 0 queue pairs, mtu = max-frame - 14
    A.mov_r_rm(RAX, f("mac"), 0)
    A.mov_rm_r(f("cfg"), RAX, width=4)
    A.movzx(RAX, f("mac", 4), 2)
    A.mov_rm_r(f("cfg", 4), RAX, width=2)
    A.mov_rm_imm32(("r", RAX), 1)
    A.mov_rm_r(f("cfg", 6), RAX, width=2)
    A.mov_r_rm(RAX, f("maxf"))
    A.alu_rm_imm("sub", ("r", RAX), 14)
    A.mov_rm_r(f("cfg", 10), RAX, width=2)
    # the side file t1-run hashes: every frame sent, u16 BE length + frame (yantra's note_sent)
    A.lea(RDI, ("abs", "nt_env_sent", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_nti_env")
    A.mov_r_rm(RDI, ("r", RAX))
    A.mov_rm_imm32(("r", RSI), 0x80241)
    A.mov_rm_imm32(("r", RDX), 420)
    sysn(2)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_nti_sent")
    A.mov_rm_r(f("sidefd"), RAX)
    if vdf:                                             # YANTRA_VIRTIO_DEFER: yantra-run's stderr line (n > 0)
        A.lea(RDI, ("abs", "nt_env_vd", 0))
        A.call("rt_getenv")
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "rt_nti_novd")
        A.mov_r_rm(RSI, ("r", RAX))
        A.call("rt_atou")
        A.mov_rm_r(f("vd"), RAX)
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "rt_nti_novd")
        A.lea(RDI, f("line"))
        cpy("virtio: completions deferred by ")
        A.mov_r_rm(RAX, f("vd"))
        A.call("rt_nt_dec")
        cpy(" instruction(s) (YANTRA_VIRTIO_DEFER)\n")
        A.mov_r_rm(RDX, ("r", RDI))
        A.lea(RSI, f("line"))
        A.alu_r_rm("sub", RDX, ("r", RSI))
        A.mov_rm_imm32(("r", RDI), 2)
        sysn(1)
        A.label("rt_nti_novd")
    A.alu_rm_imm("cmp", f("rpl"), 0)
    A.jcc("ne", "rt_nti_rpl")
    # the rendezvous (open_peer): connect, else bind and listen, at most 100 tries
    A.mov_rm_imm32(("r", R12), 100)
    A.label("rt_nti_try")
    A.mov_rm_imm32(("r", RDI), 1)
    A.mov_rm_imm32(("r", RSI), 0x80001)
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    sysn(41)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_nti_sock")
    A.mov_r_rm(RBX, ("r", RAX))
    A.mov_r_rm(RDI, ("r", RBX))
    A.lea(RSI, f("sa"))
    A.mov_r_rm(RDX, f("pathlen"))
    A.alu_rm_imm("add", ("r", RDX), 3)
    sysn(42)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_nti5")
    A.mov_rm_r(f("fd"), RBX)
    say("net: peer ")
    say_path()
    say(" connected\n")
    A.jmp("rt_nti_hdr")
    A.label("rt_nti5")
    A.alu_r_rm("xor", R13, ("r", R13), 0)
    A.alu_rm_imm("cmp", ("r", RAX), -111)            # ECONNREFUSED: a socket file nobody listens on
    A.jcc("ne", "rt_nti6")
    A.mov_rm_imm32(("r", R13), 1)
    A.label("rt_nti6")
    A.mov_r_rm(RDI, ("r", RBX))
    sysn(3)
    A.mov_rm_imm32(("r", RDI), 1)
    A.mov_rm_imm32(("r", RSI), 0x80801)               # SOCK_STREAM | NONBLOCK | CLOEXEC
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    sysn(41)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_nti_sock")
    A.mov_r_rm(RBX, ("r", RAX))
    A.mov_r_rm(RDI, ("r", RBX))
    A.lea(RSI, f("sa"))
    A.mov_r_rm(RDX, f("pathlen"))
    A.alu_rm_imm("add", ("r", RDX), 3)
    sysn(49)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_nti_bound")
    A.alu_rm_imm("cmp", ("r", RAX), -98)             # EADDRINUSE
    A.jcc("ne", "rt_nti_bind")
    A.mov_r_rm(RDI, ("r", RBX))
    sysn(3)
    A.test_rm_r(("r", R13), R13)
    A.jcc("e", "rt_nti7")
    A.lea(RDI, f("sa", 2))
    sysn(87)                                           # unlink the dead socket file
    A.jmp("rt_nti8")
    A.label("rt_nti7")
    A.mov_rm_imm32(f("ts"), 0)
    A.mov_rm_imm32(f("ts", 8), 50000000)
    A.lea(RDI, f("ts"))
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)
    sysn(35)
    A.label("rt_nti8")
    A.alu_rm_imm("sub", ("r", R12), 1)
    A.jcc("ne", "rt_nti_try")
    A.jmp("rt_nti_rdv")
    A.label("rt_nti_bound")
    A.mov_r_rm(RDI, ("r", RBX))
    A.mov_rm_imm32(("r", RSI), 128)
    sysn(50)
    A.mov_rm_imm32(f("listen"), 1)
    say("net: peer ")
    say_path()
    say(" listening\n")
    A.alu_r_rm("xor", R14, ("r", R14), 0)
    A.label("rt_nti_acc")
    A.mov_r_rm(RDI, ("r", RBX))
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.mov_rm_imm32(("r", R10), 0x80000)
    sysn(288)                                          # accept4(.., SOCK_CLOEXEC): a blocking stream
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_nti9")
    A.mov_rm_r(f("fd"), RAX)
    A.mov_r_rm(RDI, ("r", RBX))
    sysn(3)
    say("net: peer ")
    say_path()
    say(" connected\n")
    A.jmp("rt_nti_hdr")
    A.label("rt_nti9")
    A.alu_rm_imm("cmp", ("r", RAX), -4)
    A.jcc("e", "rt_nti_acc")
    A.alu_rm_imm("cmp", ("r", RAX), -11)
    A.jcc("ne", "rt_nti_accerr")
    A.alu_r_rm("cmp", R14, f("tmo"))
    A.jcc("ae", "rt_nti_tmo")
    A.mov_rm_imm32(f("ts"), 0)
    A.mov_rm_imm32(f("ts", 8), 10000000)
    A.lea(RDI, f("ts"))
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)
    sysn(35)
    A.alu_rm_imm("add", ("r", R14), 10)
    A.jmp("rt_nti_acc")
    A.label("rt_nti_tmo")                              # yantra-run: net: refused — NET_TIMEOUT, exit 1
    say("net: refused — " + NET_TIMEOUT + " (nothing ran)\n")
    A.lea(RDI, f("sa", 2))
    sysn(87)
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_nti_accerr")
    A.lea(RDI, f("sa", 2))
    sysn(87)
    A.jmp("rt_nti_acc2")
    # replay: the frames file is the peer; nothing is written to the log
    A.label("rt_nti_rpl")
    A.lea(RDI, ("abs", "nt_env_replay", 0))
    A.call("rt_getenv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_nti_env")
    A.mov_r_rm(RDI, ("r", RAX))
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    sysn(2)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_nti_rdv")
    A.mov_rm_r(f("fd"), RAX)
    A.mov_rm_imm32(f("on"), 1)
    A.ret()
    # the log: the clock header goes, the net header is written once the peer is there
    A.label("rt_nti_hdr")
    A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
    A.alu_rm_imm("sub", ("r", RDI), 1)
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)
    sysn(77)
    A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
    A.alu_rm_imm("sub", ("r", RDI), 1)
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    sysn(8)
    A.lea(RDI, f("line"))
    cpy("# yantra-net v1 mac=")
    A.lea(R8, f("mac"))
    A.mov_rm_imm32(("r", R9), 6)
    A.label("rt_nti10")
    A.movzx(RAX, ("m", R8, 0), 1)
    A.call("rt_nt_hex2")
    A.lea(R8, ("m", R8, 1))
    A.alu_rm_imm("sub", ("r", R9), 1)
    A.jcc("e", "rt_nti11")
    A.raw(0xC6, 0x07, 0x3A)                            # mov byte [rdi], ':'
    A.lea(RDI, ("m", RDI, 1))
    A.jmp("rt_nti10")
    A.label("rt_nti11")
    for t, field in ((" max-frame=", "maxf"), (" max-tx=", "maxtx"), (" max-rx=", "maxrx"),
                     (" filter=", "filter")):
        cpy(t)
        A.mov_r_rm(RAX, f(field))
        A.call("rt_nt_dec")
    if vdf:
        cpy(" defer=")
        A.mov_r_rm(RAX, f("vd"))
        A.call("rt_nt_dec")
        cpy(NET_HDR_TAIL[len(" defer=0"):])
    else:
        cpy(NET_HDR_TAIL)
    A.mov_r_rm(RDX, ("r", RDI))
    A.lea(RSI, f("line"))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
    A.alu_rm_imm("sub", ("r", RDI), 1)
    A.call("rt_nt_wfd")
    A.mov_rm_imm32(f("on"), 1)
    A.ret()
    if vdf:
        native_refuse("rt_ntv_stgfull", "more than 512 received frames staged under YANTRA_VIRTIO_DEFER")
    native_refuse("rt_nti_norec", "a net run without an event log")
    native_refuse("rt_nti_noslot", "a net run of an image with no SASEVENT tag")
    native_refuse("rt_nti_path", "a --net-peer path of 0 or more than 107 octets")
    native_refuse("rt_nti_env", "a YANTRA_NET_* setting missing or malformed")
    native_refuse("rt_nti_sent", "the frames side file cannot be opened")
    native_refuse("rt_nti_sock", "no AF_UNIX socket")
    native_refuse("rt_nti_bind", "the peer path cannot be bound")
    native_refuse("rt_nti_rdv", "no peer after 100 tries")
    native_refuse("rt_nti_acc2", "the peer's accept failed")

    # ================= small helpers
    # rt_nt_dec: rax as decimal at rdi, rdi advanced (rax rcx rdx rsi clobbered)
    A.label("rt_nt_dec")
    A.lea(RSI, f("dec", 31))
    A.mov_rm_imm32(("r", RCX), 10)
    A.label("rt_ntd1")
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.f7(6, ("r", RCX))
    A.lea(RDX, ("m", RDX, 48))
    A.lea(RSI, ("m", RSI, -1))
    A.raw(0x88, 0x16)                                  # mov [rsi], dl
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_ntd1")
    A.lea(RCX, f("dec", 31))
    A.alu_r_rm("sub", RCX, ("r", RSI))
    A.raw(0xF3, 0xA4)
    A.ret()
    # rt_nt_hex2: the octet al as two lowercase hex digits at rdi, rdi advanced (rax rcx clobbered)
    A.label("rt_nt_hex2")
    A.mov_r_rm(RCX, ("r", RAX))
    A.shift_imm("shr", ("r", RCX), 4)
    A.alu_rm_imm("and", ("r", RCX), 15)
    nib("rt_nth1")
    A.mov_rm_r(("m", RDI, 0), RCX, width=1)
    A.mov_r_rm(RCX, ("r", RAX))
    A.alu_rm_imm("and", ("r", RCX), 15)
    nib("rt_nth2")
    A.mov_rm_r(("m", RDI, 1), RCX, width=1)
    A.lea(RDI, ("m", RDI, 2))
    A.ret()
    # rt_nt_wfd: write(rdi, rsi, rdx) whole (EINTR retried; rax rcx rdx rsi r11 clobbered)
    A.label("rt_nt_wfd")
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "rt_ntf2")
    sysn(1)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_ntf1")
    A.alu_r_rm("add", RSI, ("r", RAX))
    A.alu_r_rm("sub", RDX, ("r", RAX))
    A.jmp("rt_nt_wfd")
    A.label("rt_ntf1")
    A.alu_rm_imm("cmp", ("r", RAX), -4)
    A.jcc("e", "rt_nt_wfd")
    A.label("rt_ntf2")
    A.ret()
    # rt_nt_mono: rax = CLOCK_MONOTONIC in ns (rcx rdx rsi rdi r11 clobbered)
    A.label("rt_nt_mono")
    A.mov_rm_imm32(("r", RDI), 1)
    A.lea(RSI, f("ts"))
    sysn(228)
    A.mov_r_rm(RAX, f("ts"))
    A.mov_rm_imm32(("r", RCX), 1000000000)
    A.f7(4, ("r", RCX))
    A.alu_r_rm("add", RAX, f("ts", 8))
    A.ret()
    # rt_nt_span: rax = addr, rcx = len: rax 0 when [addr, addr + len) lies in RAM (virtqueue::span), else 1
    A.label("rt_nt_span")
    A.mov_r_imm64(RDX, base)
    A.alu_r_rm("sub", RAX, ("r", RDX))
    A.jcc("b", "rt_nts_no")
    A.mov_r_rm(RDX, ("abs", "top", 0))
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    A.jcc("a", "rt_nts_no")
    A.alu_r_rm("sub", RDX, ("r", RAX))
    A.alu_r_rm("cmp", RDX, ("r", RCX))
    A.jcc("b", "rt_nts_no")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.ret()
    A.label("rt_nts_no")
    A.mov_rm_imm32(("r", RAX), 1)
    A.ret()

    # ================= the halts
    # a Device halt: rsi/rdx = its tail (", why: \"…\" }\n"); pc = the access's, addr = nt.addr
    A.label("rt_nt_dev")
    A.mov_rm_r(f("tmp"), RSI)
    A.mov_rm_r(f("tmp2"), RDX)
    A.call("rt_flush")
    A.call("m_dev1")
    pc_rax()
    A.call("rt_putu")
    A.call("m_bada2")
    A.mov_r_rm(RAX, f("addr"))
    A.call("rt_putu")
    A.mov_r_rm(RSI, f("tmp"))
    A.mov_r_rm(RDX, f("tmp2"))
    A.mov_rm_imm32(("r", RDI), 2)
    sysn(1)
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")

    def why_stub(lab, t):
        tail = (", why: " + rust_debug(t) + " }\n").encode()
        A.label(lab)
        A.lea(RSI, ("abs", lab + "_s", 0))
        A.mov_rm_imm32(("r", RDX), len(tail))
        A.jmp("rt_nt_dev")
        strs.append((lab + "_s", tail))

    for key, t in NET_WHY.items():
        why_stub("rt_ntw_" + key, t)
    for code, t in NET_VQ:
        why_stub("rt_ntw_vq%d" % code, "virtqueue refused: " + t)
    for code, t in NET_CB:
        why_stub("rt_ntw_cb%d" % code, t)

    def W(key):
        return "rt_ntw_" + key

    # rt_nt_check: rax = offset, edx = width: config (>= 0x100) -> cfg; a register -> ctl; else WIDTH
    def check(cfg, ctl):
        A.alu_rm_imm("cmp", ("r", RAX), 0x100)
        A.jcc("b", ctl + "c")
        A.mov_r_rm(RCX, ("r", RAX))
        A.alu_r_rm("add", RCX, ("r", RDX))
        A.alu_rm_imm("cmp", ("r", RCX), 0x10C)
        A.jcc("a", W("width"))
        for w in (1, 2, 4):
            A.alu_rm_imm("cmp", ("r", RDX), w)
            A.jcc("e", cfg)
        A.jmp(W("width"))
        A.label(ctl + "c")
        A.alu_rm_imm("cmp", ("r", RDX), 4)
        A.jcc("ne", W("width"))
        A.mov_r_rm(RCX, ("r", RAX))
        A.alu_rm_imm("and", ("r", RCX), 3)
        A.jcc("ne", W("width"))
        A.jmp(ctl)

    def selq(r, fail):                    # r = &q[QueueSel], or jump to fail (QueueSel >= 2)
        A.mov_r_rm(r, f("qsel"))
        A.alu_rm_imm("cmp", ("r", r), 2)
        A.jcc("ae", fail)
        A.shift_imm("shl", ("r", r), 6)
        A.push(RAX)
        A.lea(RAX, f("q0"))
        A.alu_r_rm("add", r, ("r", RAX))
        A.pop(RAX)

    # ================= rt_nt_ld: a load in slot 2 (rax = addr, edx = width): rax = the value's address
    A.label("rt_nt_ld")
    A.mov_rm_r(f("addr"), RAX)
    A.alu_rm_imm("sub", ("r", RAX), NET_LO)
    check("rt_ntl_cfg", "rt_ntl_ctl")
    A.label("rt_ntl_cfg")
    A.lea(RCX, f("cfg", -0x100))
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.ret()
    A.label("rt_ntl_ctl")
    consts = {0x000: 0x74726976, 0x004: 1, 0x008: 1, 0x00C: 0x534E4153}
    for off, v in consts.items():
        lab = L("lc")
        A.alu_rm_imm("cmp", ("r", RAX), off)
        A.jcc("ne", lab)
        A.mov_rm_imm32(f("val"), v)
        A.jmp("rt_ntl_ret")
        A.label(lab)
    for off, field in ((0x070, "status"), (0x060, "isr")):
        lab = L("lc")
        A.alu_rm_imm("cmp", ("r", RAX), off)
        A.jcc("ne", lab)
        A.mov_r_rm(RCX, f(field))
        A.mov_rm_r(f("val"), RCX)
        A.jmp("rt_ntl_ret")
        A.label(lab)
    lab = L("lc")
    A.alu_rm_imm("cmp", ("r", RAX), 0x010)
    A.jcc("ne", lab)
    A.mov_rm_imm32(f("val"), 0)
    A.alu_rm_imm("cmp", f("hfsel"), 0)
    A.jcc("ne", "rt_ntl_ret")
    A.mov_rm_imm32(f("val"), 0x10028)
    A.jmp("rt_ntl_ret")
    A.label(lab)
    lab = L("lc")
    A.alu_rm_imm("cmp", ("r", RAX), 0x034)
    A.jcc("ne", lab)
    A.mov_rm_imm32(f("val"), 0)
    selq(RCX, "rt_ntl_ret")
    A.alu_rm_imm("cmp", ("m", RCX, NQ_PFN), 0)
    A.jcc("ne", W("qnmax"))
    A.mov_rm_imm32(f("val"), 256)
    A.jmp("rt_ntl_ret")
    A.label(lab)
    lab = L("lc")
    A.alu_rm_imm("cmp", ("r", RAX), 0x040)
    A.jcc("ne", lab)
    A.mov_rm_imm32(f("val"), 0)
    selq(RCX, "rt_ntl_ret")
    A.mov_r_rm(RCX, ("m", RCX, NQ_PFN))
    A.mov_rm_r(f("val"), RCX)
    A.jmp("rt_ntl_ret")
    A.label(lab)
    for off in (0x14, 0x20, 0x24, 0x28, 0x30, 0x38, 0x3C, 0x50, 0x64):
        A.alu_rm_imm("cmp", ("r", RAX), off)
        A.jcc("e", W("readw"))
    A.jmp(W("noreg"))
    A.label("rt_ntl_ret")
    A.lea(RAX, f("val"))
    A.ret()

    # ================= rt_nt_st: a store in slot 2 (rax = addr, rcx = value, edx = width)
    A.label("rt_nt_st")
    for r in SAVE:
        A.push(r)
    A.mov_rm_r(f("addr"), RAX)
    A.mov_r_rm(R9, ("r", RCX), 0)                      # v = the value as u32
    A.alu_rm_imm("sub", ("r", RAX), NET_LO)
    check(W("cfgro"), "rt_nts_ctl")
    A.label("rt_nts_ctl")
    for off in (0x000, 0x004, 0x008, 0x00C, 0x010, 0x034, 0x060):
        A.alu_rm_imm("cmp", ("r", RAX), off)
        A.jcc("e", W("writer"))

    def case(off):
        lab = L("sc")
        A.alu_rm_imm("cmp", ("r", RAX), off)
        A.jcc("ne", lab)
        return lab

    nxt = case(0x070)                                  # Status: 0 resets the device, not the host's counts
    A.test_rm_r(("r", R9), R9)
    lab = L("st")
    A.jcc("e", lab)
    A.mov_rm_r(f("status"), R9)
    A.jmp("rt_nts_done")
    A.label(lab)
    for field in ("status", "hfsel", "gfsel", "dfeat", "page", "qsel", "isr"):
        A.mov_rm_imm32(f(field), 0)
    for q in ("q0", "q1"):
        for o in range(0, 56, 8):
            A.mov_rm_imm32(f(q, o), 0)
    if vdf:                                             # yantra's reset builds a fresh device: the pending TX and the staged frames go
        for field in ("ptx", "stn", "sth", "stt", "vwait"):
            A.mov_rm_imm32(f(field), 0)
    A.jmp("rt_nts_done")
    A.label(nxt)
    for off, field in ((0x14, "hfsel"), (0x24, "gfsel"), (0x30, "qsel")):
        nxt = case(off)
        A.mov_rm_r(f(field), R9)
        A.jmp("rt_nts_done")
        A.label(nxt)
    nxt = case(0x20)                                   # GuestFeatures
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.alu_rm_imm("cmp", f("gfsel"), 0)
    lab = L("gf")
    A.jcc("ne", lab)
    A.mov_rm_imm32(("r", RCX), 0x10028)
    A.label(lab)
    A.alu_rm_imm("xor", ("r", RCX), -1)
    A.test_rm_r(("r", RCX), R9)
    A.jcc("ne", W("feature"))
    A.alu_rm_imm("cmp", f("gfsel"), 0)
    A.jcc("ne", "rt_nts_done")
    A.mov_rm_r(f("dfeat"), R9)
    A.jmp("rt_nts_done")
    A.label(nxt)
    nxt = case(0x28)                                   # GuestPageSize
    pow2_or(R9, W("page"))
    A.mov_r_rm(RAX, f("q0", NQ_PFN))
    A.alu_r_rm("or", RAX, f("q1", NQ_PFN))
    A.jcc("ne", W("inuse"))
    A.mov_rm_r(f("page"), R9)
    A.jmp("rt_nts_done")
    A.label(nxt)
    nxt = case(0x38)                                   # QueueNum
    selq(RDX, W("qnum_noq"))
    A.alu_rm_imm("cmp", ("m", RDX, NQ_PFN), 0)
    A.jcc("ne", W("inuse"))
    pow2_or(R9, W("qnum"))
    A.alu_rm_imm("cmp", ("r", R9), 256)
    A.jcc("a", W("qnum"))
    A.mov_rm_r(("m", RDX, NQ_NUM), R9)
    A.jmp("rt_nts_done")
    A.label(nxt)
    nxt = case(0x3C)                                   # QueueAlign
    selq(RDX, W("qalign_noq"))
    A.alu_rm_imm("cmp", ("m", RDX, NQ_PFN), 0)
    A.jcc("ne", W("inuse"))
    pow2_or(R9, W("qalign"))
    A.mov_rm_r(("m", RDX, NQ_ALIGN), R9)
    A.jmp("rt_nts_done")
    A.label(nxt)
    nxt = case(0x40)                                   # QueuePFN
    selq(RDX, W("qpfn_noq"))
    A.test_rm_r(("r", R9), R9)
    lab = L("pf")
    A.jcc("ne", lab)
    A.mov_rm_imm32(("m", RDX, NQ_PFN), 0)
    A.mov_rm_imm32(("m", RDX, NQ_LAST), 0)
    A.jmp("rt_nts_done")
    A.label(lab)
    A.alu_rm_imm("cmp", f("page"), 0)
    A.jcc("e", W("qpfn_page"))
    A.alu_rm_imm("cmp", ("m", RDX, NQ_NUM), 0)
    A.jcc("e", W("qpfn_num"))
    A.alu_rm_imm("cmp", ("m", RDX, NQ_ALIGN), 0)
    A.jcc("e", W("qpfn_num"))
    A.mov_r_rm(R10, ("r", R9))                         # desc = pfn * page
    A.imul_r_rm(R10, f("page"))
    A.mov_r_rm(RCX, ("m", RDX, NQ_NUM))                # avail = desc + 16 num
    A.shift_imm("shl", ("r", RCX), 4)
    A.mov_r_rm(R11, ("r", R10))
    A.alu_r_rm("add", R11, ("r", RCX))
    A.mov_r_rm(RCX, ("m", RDX, NQ_NUM))                # used = align_up(avail + 2 (3 + num), align)
    A.alu_rm_imm("add", ("r", RCX), 3)
    A.shift_imm("shl", ("r", RCX), 1)
    A.mov_r_rm(RAX, ("r", R11))
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.mov_r_rm(RCX, ("m", RDX, NQ_ALIGN))
    A.alu_rm_imm("sub", ("r", RCX), 1)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.alu_rm_imm("xor", ("r", RCX), -1)
    A.alu_r_rm("and", RAX, ("r", RCX))
    A.mov_r_rm(RBX, ("r", RAX))
    A.mov_r_rm(RCX, ("m", RDX, NQ_NUM))                # end = used + 6 + 8 num
    A.shift_imm("shl", ("r", RCX), 3)
    A.alu_rm_imm("add", ("r", RCX), 6)
    A.alu_r_rm("add", RCX, ("r", RBX))
    A.mov_r_imm64(RAX, base)
    A.alu_r_rm("cmp", R10, ("r", RAX))
    A.jcc("b", W("qpfn_out"))
    A.alu_r_rm("add", RAX, ("abs", "top", 0))
    A.alu_r_rm("cmp", RCX, ("r", RAX))
    A.jcc("a", W("qpfn_out"))
    A.mov_rm_r(("m", RDX, NQ_PFN), R9)
    A.mov_rm_imm32(("m", RDX, NQ_LAST), 0)
    A.mov_rm_r(("m", RDX, NQ_DESC), R10)
    A.mov_rm_r(("m", RDX, NQ_AVAIL), R11)
    A.mov_rm_r(("m", RDX, NQ_USED), RBX)
    A.jmp("rt_nts_done")
    A.label(nxt)
    nxt = case(0x50)                                   # QueueNotify: queue 1 is walked now
    A.alu_rm_imm("cmp", ("r", R9), 2)
    A.jcc("ae", W("notify_noq"))
    A.mov_r_rm(RDX, ("r", R9))
    A.shift_imm("shl", ("r", RDX), 6)
    A.lea(RAX, f("q0"))
    A.alu_r_rm("add", RDX, ("r", RAX))
    A.alu_rm_imm("cmp", ("m", RDX, NQ_PFN), 0)
    A.jcc("e", W("notify_nopfn"))
    A.alu_rm_imm("cmp", ("r", R9), 1)
    A.jcc("ne", "rt_nts_done")
    if vdf:                                             # COALESCING: a notify while one is pending changes nothing
        A.alu_rm_imm("cmp", f("vd"), 0)
        A.jcc("e", "rt_nts_now")
        A.alu_rm_imm("cmp", f("ptx"), 0)
        A.jcc("ne", "rt_nts_done")
        A.mov_rm_imm32(f("ptx"), 1)
        A.mov_r_rm(RAX, f("vclk"))
        A.alu_r_rm("add", RAX, f("vd"))
        A.mov_rm_r(f("txdue"), RAX)
        A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
        A.mov_rm_r(f("ppc"), RAX)
        inc("vwait")
        A.jmp("rt_nts_done")
        A.label("rt_nts_now")
    A.call("rt_nt_tx")
    A.jmp("rt_nts_done")
    A.label(nxt)
    nxt = case(0x64)                                   # InterruptACK
    A.mov_r_rm(RAX, ("r", R9))
    A.alu_rm_imm("xor", ("r", RAX), -1)
    A.mov_r_rm(RCX, f("isr"))
    A.alu_r_rm("and", RCX, ("r", RAX))
    A.mov_rm_r(f("isr"), RCX)
    A.jmp("rt_nts_done")
    A.label(nxt)
    A.jmp(W("noreg"))
    A.label("rt_nts_done")
    for r in reversed(SAVE):
        A.pop(r)
    A.ret()

    # ================= rt_nt_tx: notify_now(TRANSMITQ) — every offered chain is a frame
    A.label("rt_nt_tx")
    A.mov_rm_imm32(f("served"), 0)
    A.mov_rm_imm32(f("cbwhy"), 0)
    A.label("rt_ntt_loop")
    A.lea(R8, f("q1"))
    A.call("rt_nt_walk")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_ntt_end")
    A.alu_rm_imm("cmp", ("r", RAX), 1)
    A.jcc("e", "rt_ntt_some")
    A.mov_rm_r(f("cbwhy"), RAX)
    A.jmp("rt_ntt_end")
    A.label("rt_ntt_some")

    def cbref(code):
        lab = L("cb")
        A.label(lab)
        A.mov_rm_imm32(f("cbwhy"), code)
        A.jmp("rt_ntt_cbdone")
        return lab

    A.mov_r_rm(RCX, f("rlen"))
    A.alu_rm_imm("cmp", ("r", RCX), 10)
    A.jcc("b", "rt_ntt_c20")
    A.alu_rm_imm("sub", ("r", RCX), 10)
    A.mov_rm_r(f("tmp"), RCX)                          # tmp = the frame's length
    A.alu_rm_imm("cmp", ("r", RCX), 14)
    A.jcc("b", "rt_ntt_c21")
    A.alu_r_rm("cmp", RCX, f("maxf"))
    A.jcc("a", "rt_ntt_c22")
    A.mov_r_rm(RAX, f("txf"))
    A.alu_r_rm("cmp", RAX, f("maxtx"))
    A.jcc("ae", "rt_ntt_c23")
    A.mov_r_rm(RAX, f("oblen"))                        # room for u16 + frame, else send what is there
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.alu_rm_imm("add", ("r", RAX), 2)
    A.alu_rm_imm("cmp", ("r", RAX), NT_OUTBOX)
    A.jcc("be", "rt_ntt_room")
    A.call("rt_nt_flush")
    A.label("rt_ntt_room")
    A.lea(RDI, f("outbox"))
    A.alu_r_rm("add", RDI, f("oblen"))
    A.mov_r_rm(RAX, f("tmp"))
    A.mov_r_rm(RCX, ("r", RAX))
    A.shift_imm("shr", ("r", RCX), 8)
    A.mov_rm_r(("m", RDI, 0), RCX, width=1)
    A.mov_rm_r(("m", RDI, 1), RAX, width=1)
    A.lea(RDI, ("m", RDI, 2))
    A.mov_rm_imm32(("r", R10), 10)
    A.call("rt_nt_gather")
    A.alu_rm_imm("cmp", f("filter"), 0)
    A.jcc("e", "rt_ntt_pass")
    A.lea(RSI, f("outbox"))
    A.alu_r_rm("add", RSI, f("oblen"))
    A.mov_r_rm(RAX, ("m", RSI, 8), 0)                  # frame[6..12]: the source MAC
    A.alu_r_rm("cmp", RAX, f("mac"), 0)
    A.jcc("ne", "rt_ntt_c24")
    A.movzx(RAX, ("m", RSI, 12), 2)
    A.movzx(RCX, f("mac", 4), 2)
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("ne", "rt_ntt_c24")
    A.label("rt_ntt_pass")
    A.mov_r_rm(RAX, f("oblen"))
    A.alu_r_rm("add", RAX, f("tmp"))
    A.alu_rm_imm("add", ("r", RAX), 2)
    A.mov_rm_r(f("oblen"), RAX)
    inc("txf")
    A.jmp("rt_ntt_cbdone")
    for code in (20, 21, 22, 23, 24):
        A.label("rt_ntt_c%d" % code)
        A.mov_rm_imm32(f("cbwhy"), code)
        A.jmp("rt_ntt_cbdone")
    A.label("rt_ntt_cbdone")                           # the chain is used, answered with 0 octets
    A.lea(R8, f("q1"))
    A.alu_r_rm("xor", R10, ("r", R10), 0)
    A.call("rt_nt_commit")
    A.mov_rm_imm32(f("served"), 1)
    A.alu_rm_imm("cmp", f("cbwhy"), 0)
    A.jcc("e", "rt_ntt_loop")
    A.label("rt_ntt_end")
    A.alu_rm_imm("cmp", f("served"), 0)
    A.jcc("e", "rt_ntt_e2")
    A.mov_r_rm(RAX, f("isr"))
    A.alu_rm_imm("or", ("r", RAX), 1)
    A.mov_rm_r(f("isr"), RAX)
    A.label("rt_ntt_e2")
    A.mov_r_rm(RAX, f("cbwhy"))
    for code, _ in NET_VQ:
        A.alu_rm_imm("cmp", ("r", RAX), code)
        A.jcc("e", W("vq%d" % code))
    for code, _ in NET_CB:
        A.alu_rm_imm("cmp", ("r", RAX), code)
        A.jcc("e", W("cb%d" % code))
    A.ret()

    # rt_nt_gather: the chain's readable octets after the first r10, to rdi (rax rcx rdx rsi r10 r11)
    A.label("rt_nt_gather")
    A.alu_r_rm("xor", R11, ("r", R11), 0)
    A.label("rt_ntg1")
    A.alu_r_rm("cmp", R11, f("segn"))
    A.jcc("ae", "rt_ntg9")
    A.mov_r_rm(RAX, ("r", R11))
    A.shift_imm("shl", ("r", RAX), 4)
    A.lea(RDX, f("seg"))
    A.alu_r_rm("add", RDX, ("r", RAX))
    A.mov_r_rm(RAX, ("m", RDX, 12), 0)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_ntg3")
    A.mov_r_rm(RSI, ("m", RDX, 0))
    A.mov_r_rm(RCX, ("m", RDX, 8), 0)
    A.alu_r_rm("cmp", R10, ("r", RCX))
    A.jcc("b", "rt_ntg2")
    A.alu_r_rm("sub", R10, ("r", RCX))
    A.jmp("rt_ntg3")
    A.label("rt_ntg2")
    A.alu_r_rm("add", RSI, ("r", R10))
    A.alu_r_rm("sub", RCX, ("r", R10))
    A.alu_r_rm("xor", R10, ("r", R10), 0)
    A.raw(0xF3, 0xA4)
    A.label("rt_ntg3")
    A.alu_rm_imm("add", ("r", R11), 1)
    A.jmp("rt_ntg1")
    A.label("rt_ntg9")
    A.ret()

    # ================= rt_nt_walk: virtqueue::process up to serve, r8 = &queue. rax: 0 nothing offered,
    # 1 a chain (seg/segn/rlen/wlen/head), 2.. a refusal (NET_VQ). Clobbers every register but r8.
    A.label("rt_nt_walk")
    A.mov_r_rm(R9, ("m", R8, NQ_NUM))
    A.test_rm_r(("r", R9), R9)
    A.jcc("e", "rt_ntw_r2")
    for field, sh, add in ((NQ_DESC, 4, 0), (NQ_AVAIL, 1, 4), (NQ_USED, 3, 4)):
        A.mov_r_rm(RAX, ("m", R8, field))
        A.mov_r_rm(RCX, ("r", R9))
        A.shift_imm("shl", ("r", RCX), sh)
        if add:
            A.alu_rm_imm("add", ("r", RCX), add)
        A.call("rt_nt_span")
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("ne", "rt_ntw_r3")
    A.mov_r_rm(RSI, ("m", R8, NQ_AVAIL))
    A.movzx(RAX, ("m", RSI, 2), 2)
    A.movzx(RCX, ("m", R8, NQ_LAST), 2)
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("ne", "rt_ntw1")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.ret()
    A.label("rt_ntw1")
    A.lea(RDX, ("m", R9, -1))
    A.alu_r_rm("and", RDX, ("r", RCX))
    A.shift_imm("shl", ("r", RDX), 1)
    A.alu_r_rm("add", RDX, ("r", RSI))
    A.movzx(RBX, ("m", RDX, 4), 2)
    A.alu_r_rm("cmp", RBX, ("r", R9))
    A.jcc("ae", "rt_ntw_r4")
    A.mov_rm_r(f("head"), RBX)
    for r in (R12, R13, R14, R15, R10):                # total, seen a writable, segments, readable, writable
        A.alu_r_rm("xor", r, ("r", r), 0)
    A.label("rt_ntw2")
    A.alu_r_rm("cmp", R14, ("r", R9))
    A.jcc("ae", "rt_ntw_r6")
    A.mov_r_rm(RDI, ("r", RBX))
    A.shift_imm("shl", ("r", RDI), 4)
    A.alu_r_rm("add", RDI, ("m", R8, NQ_DESC))
    A.movzx(RSI, ("m", RDI, 12), 2)                    # flags
    A.mov_r_rm(RAX, ("r", RSI))
    A.alu_rm_imm("and", ("r", RAX), 4)
    A.jcc("ne", "rt_ntw_r8")
    A.mov_r_rm(R11, ("r", RSI))
    A.alu_rm_imm("and", ("r", R11), 2)
    A.test_rm_r(("r", R13), R13)
    A.jcc("e", "rt_ntw3")
    A.test_rm_r(("r", R11), R11)
    A.jcc("e", "rt_ntw_r7")
    A.label("rt_ntw3")
    A.test_rm_r(("r", R11), R11)
    A.jcc("e", "rt_ntw4")
    A.mov_rm_imm32(("r", R13), 1)
    A.label("rt_ntw4")
    A.mov_r_rm(RCX, ("m", RDI, 8), 0)
    A.alu_r_rm("add", R12, ("r", RCX))
    A.mov_r_imm64(RAX, 0xFFFFFFFF)
    A.alu_r_rm("cmp", R12, ("r", RAX))
    A.jcc("a", "rt_ntw_r9")
    A.mov_r_rm(RAX, ("m", RDI, 0))
    A.mov_rm_r(f("waddr"), RAX)
    A.call("rt_nt_span")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_ntw_r3")
    A.mov_r_rm(RAX, ("r", R14))
    A.shift_imm("shl", ("r", RAX), 4)
    A.lea(RDX, f("seg"))
    A.alu_r_rm("add", RDX, ("r", RAX))
    A.mov_r_rm(RAX, f("waddr"))
    A.mov_rm_r(("m", RDX, 0), RAX)
    A.mov_rm_r(("m", RDX, 8), RCX, width=4)
    A.mov_rm_r(("m", RDX, 12), R11, width=4)
    A.test_rm_r(("r", R11), R11)
    A.jcc("e", "rt_ntw5")
    A.alu_r_rm("add", R10, ("r", RCX))
    A.jmp("rt_ntw6")
    A.label("rt_ntw5")
    A.alu_r_rm("add", R15, ("r", RCX))
    A.label("rt_ntw6")
    A.alu_rm_imm("add", ("r", R14), 1)
    A.mov_r_rm(RAX, ("r", RSI))
    A.alu_rm_imm("and", ("r", RAX), 1)
    A.jcc("e", "rt_ntw7")
    A.movzx(RBX, ("m", RDI, 14), 2)
    A.alu_r_rm("cmp", RBX, ("r", R9))
    A.jcc("ae", "rt_ntw_r5")
    A.jmp("rt_ntw2")
    A.label("rt_ntw7")
    A.mov_rm_r(f("segn"), R14)
    A.mov_rm_r(f("rlen"), R15)
    A.mov_rm_r(f("wlen"), R10)
    A.mov_rm_imm32(("r", RAX), 1)
    A.ret()
    for code in range(2, 10):
        A.label("rt_ntw_r%d" % code)
        A.mov_rm_imm32(("r", RAX), code)
        A.ret()

    # rt_nt_commit: the answer (r11, r10 octets) across the writables, the used element {head, r10}, the
    # used index and the last seen index advanced; r8 = &queue. rax 0, or 1 = ResponseTooLong (nothing done)
    A.label("rt_nt_commit")
    A.alu_r_rm("cmp", R10, f("wlen"))
    A.jcc("be", "rt_ntc1")
    A.mov_rm_imm32(("r", RAX), 1)
    A.ret()
    A.label("rt_ntc1")
    A.mov_r_rm(RBX, ("r", R10))
    A.alu_r_rm("xor", R9, ("r", R9), 0)
    A.label("rt_ntc2")
    A.alu_r_rm("cmp", R9, f("segn"))
    A.jcc("ae", "rt_ntc5")
    A.mov_r_rm(RAX, ("r", R9))
    A.shift_imm("shl", ("r", RAX), 4)
    A.lea(RDX, f("seg"))
    A.alu_r_rm("add", RDX, ("r", RAX))
    A.mov_r_rm(RAX, ("m", RDX, 12), 0)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_ntc4")
    A.mov_r_rm(RCX, ("m", RDX, 8), 0)
    A.alu_r_rm("cmp", RCX, ("r", R10))
    A.jcc("be", "rt_ntc3")
    A.mov_r_rm(RCX, ("r", R10))
    A.label("rt_ntc3")
    A.mov_r_rm(RDI, ("m", RDX, 0))
    A.mov_r_rm(RSI, ("r", R11))
    A.alu_r_rm("add", R11, ("r", RCX))
    A.alu_r_rm("sub", R10, ("r", RCX))
    A.raw(0xF3, 0xA4)
    A.label("rt_ntc4")
    A.alu_rm_imm("add", ("r", R9), 1)
    A.jmp("rt_ntc2")
    A.label("rt_ntc5")
    A.mov_r_rm(RSI, ("m", R8, NQ_USED))
    A.movzx(RAX, ("m", RSI, 2), 2)
    A.mov_r_rm(RCX, ("m", R8, NQ_NUM))
    A.alu_rm_imm("sub", ("r", RCX), 1)
    A.alu_r_rm("and", RCX, ("r", RAX))
    A.shift_imm("shl", ("r", RCX), 3)
    A.alu_r_rm("add", RCX, ("r", RSI))
    A.mov_r_rm(RDX, f("head"))
    A.mov_rm_r(("m", RCX, 4), RDX, width=4)
    A.mov_rm_r(("m", RCX, 8), RBX, width=4)
    A.alu_rm_imm("add", ("r", RAX), 1)
    A.mov_rm_r(("m", RSI, 2), RAX, width=2)
    A.mov_r_rm(RAX, ("m", R8, NQ_LAST))
    A.alu_rm_imm("add", ("r", RAX), 1)
    A.alu_rm_imm("and", ("r", RAX), 0xFFFF)
    A.mov_rm_r(("m", R8, NQ_LAST), RAX)
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.ret()

    if vdf:
        # ================= YANTRA_VIRTIO_DEFER (netdev tick / settle / place)
        # rt_nt_txd: the deferred serve of the transmit queue. A queue whose QueuePFN was written 0 since the
        # notify is empty for yantra: size zero when QueueNum is 0, else the ring does not lie in RAM.
        A.label("rt_nt_txd")
        A.alu_rm_imm("cmp", f("q1", NQ_PFN), 0)
        A.jcc("e", "rt_ntd_empty")
        A.jmp("rt_nt_tx")
        A.label("rt_ntd_empty")
        A.mov_rm_imm32(f("served"), 0)
        A.mov_rm_imm32(("r", RAX), 3)
        A.alu_rm_imm("cmp", f("q1", NQ_NUM), 0)
        A.jcc("ne", "rt_ntd_e")
        A.mov_rm_imm32(("r", RAX), 2)
        A.label("rt_ntd_e")
        A.mov_rm_r(f("cbwhy"), RAX)
        A.jmp("rt_ntt_e2")

        # rt_nt_tick: one guest word begins with a completion waiting (the stub set fault_off): the clock is one
        # nearer; a due TX is served (a refusal names the notify's pc), then a due staged frame is placed
        # (a refusal names this word). rax rcx rdx are scratch there; the rest is kept.
        A.label("rt_nt_tick")
        for r in SAVE:
            A.push(r)
        inc("vclk")
        A.alu_rm_imm("cmp", f("ptx"), 0)
        A.jcc("e", "rt_ntk_rx")
        A.mov_r_rm(RAX, f("vclk"))
        A.alu_r_rm("cmp", RAX, f("txdue"))
        A.jcc("ne", "rt_ntk_rx")
        A.mov_rm_imm32(f("ptx"), 0)
        A.mov_r_rm(RAX, f("vwait"))
        A.alu_rm_imm("sub", ("r", RAX), 1)
        A.mov_rm_r(f("vwait"), RAX)
        A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
        A.push(RAX)
        A.mov_r_rm(RAX, f("ppc"))
        A.mov_rm_r(("abs", "fault_off", 0), RAX, width=4)
        A.mov_rm_imm32(f("addr"), NET_LO + 0x50)
        A.call("rt_nt_txd")
        A.pop(RAX)
        A.mov_rm_r(("abs", "fault_off", 0), RAX, width=4)
        A.label("rt_ntk_rx")
        A.alu_rm_imm("cmp", f("stn"), 0)
        A.jcc("e", "rt_ntk_end")
        A.mov_r_rm(RSI, f("sth"))
        A.mov_rm_imm32(("r", RCX), NTV_SLOT)
        A.imul_r_rm(RSI, ("r", RCX))
        A.lea(RAX, f("stg"))
        A.alu_r_rm("add", RSI, ("r", RAX))
        A.mov_r_rm(RAX, ("m", RSI, 0))
        A.alu_r_rm("cmp", RAX, f("vclk"))
        A.jcc("ne", "rt_ntk_end")
        A.mov_rm_imm32(f("addr"), NET_LO + 0x50)
        A.call("rt_nt_place")
        A.mov_r_rm(RAX, f("sth"))
        A.alu_rm_imm("add", ("r", RAX), 1)
        A.alu_rm_imm("and", ("r", RAX), NTV_SLOTS - 1)
        A.mov_rm_r(f("sth"), RAX)
        A.mov_r_rm(RAX, f("stn"))
        A.alu_rm_imm("sub", ("r", RAX), 1)
        A.mov_rm_r(f("stn"), RAX)
        A.mov_r_rm(RAX, f("vwait"))
        A.alu_rm_imm("sub", ("r", RAX), 1)
        A.mov_rm_r(f("vwait"), RAX)
        A.jmp("rt_ntk_rx")
        A.label("rt_ntk_end")
        for r in reversed(SAVE):
            A.pop(r)
        A.ret()

        # rt_nt_place: the staged frame at rsi (netdev place()): into the next receive buffer, or dropped
        A.label("rt_nt_place")
        A.mov_r_rm(RCX, ("m", RSI, 8))
        A.mov_rm_r(f("rxlen"), RCX)
        A.lea(RSI, ("m", RSI, 16))
        A.lea(RDI, f("rxbuf", 10))
        A.raw(0xF3, 0xA4)
        A.alu_rm_imm("cmp", f("q0", NQ_PFN), 0)
        A.jcc("e", "rt_ntp_drop")
        A.mov_rm_imm32(f("rxbuf"), 0)
        A.alu_r_rm("xor", RAX, ("r", RAX), 0)
        A.mov_rm_r(f("rxbuf", 8), RAX, width=2)
        A.lea(R8, f("q0"))
        A.call("rt_nt_walk")
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "rt_ntp_drop")
        A.alu_rm_imm("cmp", ("r", RAX), 1)
        A.jcc("ne", "rt_ntp_err")
        A.lea(R8, f("q0"))
        A.lea(R11, f("rxbuf"))
        A.mov_r_rm(R10, f("rxlen"))
        A.alu_rm_imm("add", ("r", R10), 10)
        A.call("rt_nt_commit")
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("ne", "rt_ntp_drop")
        inc("rxf")
        A.mov_r_rm(RAX, f("isr"))
        A.alu_rm_imm("or", ("r", RAX), 1)
        A.mov_rm_r(f("isr"), RAX)
        A.ret()
        A.label("rt_ntp_drop")
        inc("drop")
        A.ret()
        A.label("rt_ntp_err")
        for code, _ in NET_VQ:
            A.alu_rm_imm("cmp", ("r", RAX), code)
            A.jcc("e", W("vq%d" % code))
        A.ret()

        # rt_nt_settle: a pending TX is served now (a wait, or any halt: rt_flush's door). Every register and
        # the halt's own scratch (fault_off, addr, tmp, tmp2, cbwhy) are kept; a refusal replaces the halt.
        A.label("rt_nt_settle")
        A.alu_rm_imm("cmp", f("ptx"), 0)
        A.jcc("ne", "rt_nst1")
        A.ret()
        A.label("rt_nst1")
        for r in (RAX, RCX, RDX) + SAVE:
            A.push(r)
        for v in (("abs", "fault_off", 0), f("addr"), f("tmp"), f("tmp2"), f("cbwhy")):
            A.mov_r_rm(RAX, v, 0 if v[1] == "fault_off" else 1)
            A.push(RAX)
        A.mov_rm_imm32(f("ptx"), 0)
        A.mov_r_rm(RAX, f("vwait"))
        A.alu_rm_imm("sub", ("r", RAX), 1)
        A.mov_rm_r(f("vwait"), RAX)
        A.mov_r_rm(RAX, f("ppc"))
        A.mov_rm_r(("abs", "fault_off", 0), RAX, width=4)
        A.mov_rm_imm32(f("addr"), NET_LO + 0x50)
        A.call("rt_nt_txd")
        for v in (f("cbwhy"), f("tmp2"), f("tmp"), f("addr"), ("abs", "fault_off", 0)):
            A.pop(RAX)
            A.mov_rm_r(v, RAX, width=4 if v[1] == "fault_off" else 8)
        for r in reversed((RAX, RCX, RDX) + SAVE):
            A.pop(r)
        A.ret()

    # ================= rt_nt_flush: the outbox (u16 BE length + frame, each) to the side file and the
    # peer; every register kept. A failed send is said once and nothing more is sent (ADR-0047).
    A.label("rt_nt_flush")
    for r in (RAX, RCX, RDX, RSI, RDI, R8, R9, R10, R11):
        A.push(r)
    A.mov_r_rm(RDX, f("oblen"))
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "rt_ntx9")
    A.mov_r_rm(RDI, f("sidefd"))
    A.lea(RSI, f("outbox"))
    A.call("rt_nt_wfd")
    A.alu_rm_imm("cmp", f("failed"), 0)
    A.jcc("ne", "rt_ntx8")
    A.lea(RSI, f("outbox"))
    A.mov_r_rm(RDX, f("oblen"))
    A.label("rt_ntx1")
    A.test_rm_r(("r", RDX), RDX)
    A.jcc("e", "rt_ntx8")
    A.mov_r_rm(RDI, f("fd"))
    A.mov_rm_imm32(("r", R10), 0x4000)                 # MSG_NOSIGNAL: EPIPE, not SIGPIPE
    A.alu_r_rm("xor", R8, ("r", R8), 0)
    A.alu_r_rm("xor", R9, ("r", R9), 0)
    sysn(44)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_ntx2")
    A.alu_r_rm("add", RSI, ("r", RAX))
    A.alu_r_rm("sub", RDX, ("r", RAX))
    A.jmp("rt_ntx1")
    A.label("rt_ntx2")
    A.alu_rm_imm("cmp", ("r", RAX), -4)
    A.jcc("e", "rt_ntx1")
    A.mov_rm_imm32(f("failed"), 1)
    A.mov_r_rm(R8, ("r", RAX))
    say("net: send failed — ")
    A.alu_rm_imm("cmp", ("r", R8), -32)
    A.jcc("ne", "rt_ntx3")
    say("Broken pipe (os error 32)")
    A.jmp("rt_ntx5")
    A.label("rt_ntx3")
    A.alu_rm_imm("cmp", ("r", R8), -104)
    A.jcc("ne", "rt_ntx4")
    say("Connection reset by peer (os error 104)")
    A.jmp("rt_ntx5")
    A.label("rt_ntx4")
    say("error (os error ")
    A.mov_r_rm(RAX, ("r", R8))
    A.f7(3, ("r", RAX))
    A.call("rt_putu")
    say(")")
    A.label("rt_ntx5")
    say("; nothing more is sent, and the program is not told (ADR-0047)\n")
    A.label("rt_ntx8")
    A.mov_rm_imm32(f("oblen"), 0)
    A.label("rt_ntx9")
    for r in (R11, R10, R9, R8, RDI, RSI, RDX, RCX, RAX):
        A.pop(r)
    A.ret()

    # ================= rt_nt_report: yantra-run's net: line (after the halt line); the sha256 is t1-run's
    A.label("rt_nt_report")
    A.push(RDI)
    A.call("rt_nt_flush")
    A.mov_rm_imm32(f("on"), 0)
    for t, field in (("net: ", "txf"), (" frames transmitted, ", "rxf"), (" received (", "drop"),
                     (" dropped, ", "filt"), (" filtered), ", "recs")):
        say(t)
        A.mov_r_rm(RAX, f(field))
        A.call("rt_putu")
    say(" wait records; sent sha256 @YANTRA_NET_SENT@\n")
    A.alu_rm_imm("cmp", f("listen"), 0)
    A.jcc("e", "rt_ntr1")
    A.lea(RDI, f("sa", 2))
    sysn(87)
    A.label("rt_ntr1")
    A.pop(RDI)
    A.ret()

    # ================= rt_nt_wait: record_live_net's wait (rsi rdi r8..r11 pushed by rt_wait)
    A.label("rt_nt_wait")
    for r in (RBX, RBP, R12, R13, R14, R15):
        A.push(r)
    if vdf:
        A.call("rt_nt_settle")                         # a pending TX is served at every wait
    A.call("rt_nt_flush")
    A.mov_rm_imm32(f("placed"), 0)
    A.mov_rm_imm32(f("got"), 0)
    A.lea(R8, f("q0"))                                 # rx_ready: a posted buffer not yet used
    A.alu_rm_imm("cmp", ("m", R8, NQ_PFN), 0)
    A.jcc("e", "rt_ntv_rec")
    A.mov_r_rm(RAX, ("m", R8, NQ_AVAIL))
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("sub", RAX, ("r", RCX))
    A.alu_rm_imm("add", ("r", RAX), 4)
    A.alu_r_rm("cmp", RAX, ("abs", "top", 0))
    A.jcc("a", "rt_ntv_rec")
    A.mov_r_rm(RSI, ("m", R8, NQ_AVAIL))
    A.movzx(RAX, ("m", RSI, 2), 2)
    A.alu_r_rm("sub", RAX, ("m", R8, NQ_LAST))
    A.alu_rm_imm("and", ("r", RAX), 0xFFFF)
    if vdf:                                             # a buffer no staged frame has claimed
        A.alu_r_rm("cmp", RAX, f("stn"))
        A.jcc("be", "rt_ntv_rec")
    else:
        A.jcc("e", "rt_ntv_rec")
    A.call("rt_nt_recv")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_ntv1")
    A.alu_rm_imm("cmp", ("r", RAX), 1)
    A.jcc("e", "rt_ntv_tmo")
    A.jmp("rt_ntv_closed")
    A.label("rt_ntv1")
    A.mov_rm_imm32(f("got"), 1)
    A.mov_r_rm(RCX, f("rxlen"))
    A.alu_rm_imm("cmp", ("r", RCX), 14)
    A.jcc("b", "rt_ntv_runt")
    A.alu_r_rm("cmp", RCX, f("maxf"))
    A.jcc("a", "rt_ntv_large")
    A.mov_r_rm(RAX, f("rxseen"))
    A.alu_r_rm("cmp", RAX, f("maxrx"))
    A.jcc("ae", "rt_ntv_count")
    inc("rxseen")
    A.alu_rm_imm("cmp", f("filter"), 0)
    A.jcc("e", "rt_ntv3")
    A.movzx(RAX, f("rxbuf", 10), 1)
    A.alu_rm_imm("and", ("r", RAX), 1)
    A.jcc("ne", "rt_ntv3")                             # multicast / broadcast: never filtered
    A.mov_r_rm(RAX, f("rxbuf", 10), 0)
    A.alu_r_rm("cmp", RAX, f("mac"), 0)
    A.jcc("ne", "rt_ntv2")
    A.movzx(RAX, f("rxbuf", 14), 2)
    A.movzx(RCX, f("mac", 4), 2)
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("e", "rt_ntv3")
    A.label("rt_ntv2")
    inc("filt")
    A.jmp("rt_ntv_rec")
    A.label("rt_ntv3")
    if vdf:
        A.alu_rm_imm("cmp", f("vd"), 0)
        A.jcc("e", "rt_ntv3n")
        A.alu_rm_imm("cmp", f("q0", NQ_PFN), 0)
        A.jcc("ne", "rt_ntv3s")
        A.jmp("rt_ntv_drop")                           # no receive queue: dropped now, 0 octets placed
        A.label("rt_ntv3s")                            # stage it: the event word is the frame's length
        A.alu_rm_imm("cmp", f("stn"), NTV_SLOTS)
        A.jcc("ae", "rt_ntv_stgfull")
        A.mov_r_rm(RAX, f("stt"))
        A.mov_rm_imm32(("r", RCX), NTV_SLOT)
        A.imul_r_rm(RAX, ("r", RCX))
        A.lea(RDI, f("stg"))
        A.alu_r_rm("add", RDI, ("r", RAX))
        A.mov_r_rm(RAX, f("vclk"))
        A.alu_r_rm("add", RAX, f("vd"))
        A.mov_rm_r(("m", RDI, 0), RAX)
        A.mov_r_rm(RCX, f("rxlen"))
        A.mov_rm_r(("m", RDI, 8), RCX)
        A.lea(RSI, f("rxbuf", 10))
        A.lea(RDI, ("m", RDI, 16))
        A.raw(0xF3, 0xA4)
        A.mov_r_rm(RAX, f("stt"))
        A.alu_rm_imm("add", ("r", RAX), 1)
        A.alu_rm_imm("and", ("r", RAX), NTV_SLOTS - 1)
        A.mov_rm_r(f("stt"), RAX)
        inc("stn")
        inc("vwait")
        A.mov_r_rm(RAX, f("rxlen"))
        A.mov_rm_r(f("placed"), RAX)
        A.jmp("rt_ntv_rec")
        A.label("rt_ntv3n")
    A.mov_rm_imm32(f("rxbuf"), 0)                      # the 10-octet virtio_net_hdr, all zero
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.mov_rm_r(f("rxbuf", 8), RAX, width=2)
    A.lea(R8, f("q0"))
    A.call("rt_nt_walk")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_ntv_drop")
    A.alu_rm_imm("cmp", ("r", RAX), 1)
    A.jcc("ne", "rt_ntv_vq")
    A.lea(R8, f("q0"))
    A.lea(R11, f("rxbuf"))
    A.mov_r_rm(R10, f("rxlen"))
    A.alu_rm_imm("add", ("r", R10), 10)
    A.call("rt_nt_commit")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_ntv_drop")                         # ResponseTooLong: dropped, as yantra
    inc("rxf")
    A.mov_r_rm(RAX, f("isr"))
    A.alu_rm_imm("or", ("r", RAX), 1)
    A.mov_rm_r(f("isr"), RAX)
    A.mov_r_rm(RAX, f("rxlen"))
    A.mov_rm_r(f("placed"), RAX)
    A.jmp("rt_ntv_rec")
    A.label("rt_ntv_drop")
    inc("drop")
    A.label("rt_ntv_rec")                              # deliver, then the record n=K:0:<hex|none>
    A.mov_r_rm(RCX, ("abs", "ev_slot", 0))
    A.mov_r_rm(RAX, f("placed"))
    A.mov_rm_r(("m", RCX, 8), RAX)
    A.lea(RDI, f("line"))
    cpy("n=")
    A.mov_r_rm(RAX, f("k"))
    A.call("rt_nt_dec")
    cpy(":0:")
    A.alu_rm_imm("cmp", f("got"), 0)
    A.jcc("ne", "rt_ntv4")
    cpy("none")
    A.jmp("rt_ntv6")
    A.label("rt_ntv4")
    A.lea(R12, f("rxbuf", 10))
    A.mov_r_rm(R13, f("rxlen"))
    A.label("rt_ntv5")
    A.test_rm_r(("r", R13), R13)
    A.jcc("e", "rt_ntv6")
    A.movzx(RAX, ("m", R12, 0), 1)
    A.call("rt_nt_hex2")
    A.lea(R12, ("m", R12, 1))
    A.alu_rm_imm("sub", ("r", R13), 1)
    A.jmp("rt_ntv5")
    A.label("rt_ntv6")
    A.raw(0xC6, 0x07, 0x0A)                            # mov byte [rdi], 10
    A.lea(RDI, ("m", RDI, 1))
    A.mov_r_rm(RDX, ("r", RDI))
    A.lea(RSI, f("line"))
    A.alu_r_rm("sub", RDX, ("r", RSI))
    A.alu_rm_imm("cmp", f("rpl"), 0)
    A.jcc("ne", "rt_ntv_nolog")
    A.mov_r_rm(RDI, ("abs", "rec_fd", 0))
    A.alu_rm_imm("sub", ("r", RDI), 1)
    A.call("rt_nt_wfd")
    A.label("rt_ntv_nolog")
    inc("k")
    A.mov_rm_r(f("recs"), RAX)
    for r in (R15, R14, R13, R12, RBP, RBX):
        A.pop(r)
    A.jmp("rt_wt4")

    # a wait refused (record_live_net's Err): halt Wait at the next pc, the net: line with 0 records,
    # then events: REFUSED — wait index K (pc 0x…): why
    A.label("rt_nt_wref")
    A.call("rt_flush")
    A.call("m_wait")
    pc_rax(4)
    A.call("rt_putu")
    A.call("m_un4")
    A.mov_rm_imm32(f("recs"), 0)
    A.call("rt_nt_report")
    say("events: REFUSED — wait index ")
    A.mov_r_rm(RAX, f("k"))
    A.call("rt_putu")
    say(" (pc ")
    pc_rax()
    A.call("rt_puthex")
    say("): ")
    A.ret()

    def wref(lab, why):
        A.label(lab)
        A.call("rt_nt_wref")
        say(why)

    def wend():
        say("\n")
        A.mov_rm_imm32(("r", RDI), 1)
        A.jmp("rt_exit")

    wref("rt_ntv_tmo", NET_TIMEOUT)
    wend()
    wref("rt_ntv_closed", NET_PEER_CLOSED)
    wend()
    wref("rt_ntv_runt", NET_FRAME_RUNT + " (")
    A.mov_r_rm(RAX, f("rxlen"))
    A.call("rt_putu")
    say(" octets)")
    wend()
    wref("rt_ntv_large", NET_FRAME_TOO_LARGE + " (")
    A.mov_r_rm(RAX, f("rxlen"))
    A.call("rt_putu")
    say(" octets against ")
    A.mov_r_rm(RAX, f("maxf"))
    A.call("rt_putu")
    say(")")
    wend()
    wref("rt_ntv_count", NET_RX_COUNT_EXCEEDED + " (the cap is ")
    A.mov_r_rm(RAX, f("maxrx"))
    A.call("rt_putu")
    say(")")
    wend()
    A.label("rt_ntv_vq")                               # a malformed receive ring: its refusal by name
    A.mov_rm_r(f("cbwhy"), RAX)
    A.call("rt_nt_wref")
    say("virtqueue refused: ")
    A.mov_r_rm(RAX, f("cbwhy"))
    for code, t in NET_VQ:
        lab = L("vq")
        A.alu_rm_imm("cmp", ("r", RAX), code)
        A.jcc("ne", lab)
        say(t)
        wend()
        A.label(lab)
    wend()

    # rt_nt_recv: one frame from the peer (u16 BE length, then the frame) into rxbuf + 10, rxlen = its
    # length, within --net-timeout of now. rax 0, 1 = NET_TIMEOUT, 2 = NET_PEER_CLOSED.
    A.label("rt_nt_recv")
    A.call("rt_nt_mono")
    A.mov_r_rm(RCX, f("tmo"))
    A.mov_rm_imm32(("r", RDX), 1000000)
    A.imul_r_rm(RCX, ("r", RDX))
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.mov_rm_r(f("deadline"), RAX)
    A.lea(R12, f("lenbuf"))
    A.mov_rm_imm32(("r", R13), 2)
    A.call("rt_nt_readx")
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_ntq9")
    A.movzx(RAX, f("lenbuf"), 1)
    A.shift_imm("shl", ("r", RAX), 8)
    A.movzx(RCX, f("lenbuf", 1), 1)
    A.alu_r_rm("or", RAX, ("r", RCX))
    A.mov_rm_r(f("rxlen"), RAX)
    A.lea(R12, f("rxbuf", 10))
    A.mov_r_rm(R13, ("r", RAX))
    A.call("rt_nt_readx")
    A.label("rt_ntq9")
    A.ret()
    # rt_nt_readx: r13 octets to r12 by the deadline (poll, then read)
    A.label("rt_nt_readx")
    A.test_rm_r(("r", R13), R13)
    A.jcc("ne", "rt_ntr2")
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.ret()
    A.label("rt_ntr2")
    A.call("rt_nt_mono")
    A.mov_r_rm(R14, f("deadline"))
    A.alu_r_rm("sub", R14, ("r", RAX))
    A.jcc("a", "rt_ntr3")
    A.alu_r_rm("xor", R14, ("r", R14), 0)             # the deadline has passed: one poll that does not wait
    A.label("rt_ntr3")
    A.mov_r_rm(RAX, ("r", R14))
    A.alu_rm_imm("add", ("r", RAX), 999999)
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.mov_rm_imm32(("r", RCX), 1000000)
    A.f7(6, ("r", RCX))
    A.mov_rm_imm32(("r", RCX), 0x7FFFFFFF)
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("be", "rt_ntr4")
    A.mov_r_rm(RAX, ("r", RCX))
    A.label("rt_ntr4")
    A.mov_r_rm(RDX, ("r", RAX))
    A.mov_r_rm(RAX, f("fd"))
    A.mov_rm_r(f("pollfd"), RAX, width=4)
    A.mov_rm_imm32(f("pollfd", 4), 1, 0)               # POLLIN, revents 0
    A.lea(RDI, f("pollfd"))
    A.mov_rm_imm32(("r", RSI), 1)
    sysn(7)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("g", "rt_ntr6")
    A.jcc("e", "rt_ntr5")
    A.alu_rm_imm("cmp", ("r", RAX), -4)
    A.jcc("e", "rt_nt_readx")
    A.mov_rm_imm32(("r", RAX), 2)
    A.ret()
    A.label("rt_ntr5")                                 # nothing arrived: expired, or poll woke early
    A.test_rm_r(("r", R14), R14)
    A.jcc("ne", "rt_nt_readx")
    A.mov_rm_imm32(("r", RAX), 1)
    A.ret()
    A.label("rt_ntr6")
    A.mov_r_rm(RDI, f("fd"))
    A.mov_r_rm(RSI, ("r", R12))
    A.mov_r_rm(RDX, ("r", R13))
    sysn(0)
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("g", "rt_ntr7")
    A.jcc("e", "rt_ntr8")
    A.alu_rm_imm("cmp", ("r", RAX), -4)
    A.jcc("e", "rt_nt_readx")
    A.alu_rm_imm("cmp", ("r", RAX), -11)
    A.jcc("e", "rt_nt_readx")
    A.label("rt_ntr8")
    A.mov_rm_imm32(("r", RAX), 2)
    A.ret()
    A.label("rt_ntr7")
    A.alu_r_rm("add", R12, ("r", RAX))
    A.alu_r_rm("sub", R13, ("r", RAX))
    A.jmp("rt_nt_readx")

    # ================= rt_nt_s0: a load in slot 0 (rcx = offset, edx = width). yantra's GPU sits
    # there; a driver scanning for the net device reads its identity (MagicValue, Version,
    # DeviceID, VendorID) and Status (0: native refuses every slot-0 store). Anything else in
    # slot 0 is what this phase does not answer: the load refusal, as before.
    A.label("rt_nt_s0")
    A.alu_rm_imm("cmp", ("r", RDX), 4)
    A.jcc("ne", "rt_ls_nont")
    A.alu_rm_imm("cmp", ("r", RCX), 0x070)
    A.jcc("e", "rt_nts0z")
    A.alu_rm_imm("cmp", ("r", RCX), 0x00C)
    A.jcc("a", "rt_ls_nont")
    A.push(RCX)
    A.alu_rm_imm("and", ("r", RCX), 3)
    A.pop(RCX)                                         # pop keeps the flags
    A.jcc("ne", "rt_ls_nont")
    A.lea(RAX, ("abs", "nt_s0tab", 0))
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.ret()
    A.label("rt_nts0z")
    A.lea(RAX, ("abs", "nt_s0tab", 16))
    A.ret()

    for lab, t in strs + [("nt_s0tab", struct.pack("<5I", 0x74726976, 1, 16, 0x534E4153, 0)),
                          ("nt_env_peer", b"YANTRA_NET_PEER=\0"), ("nt_env_mac", b"YANTRA_NET_MAC=\0"),
                          ("nt_env_filter", b"YANTRA_NET_FILTER=\0"), ("nt_env_maxf", b"YANTRA_NET_MAX_FRAME=\0"),
                          ("nt_env_maxtx", b"YANTRA_NET_MAX_TX=\0"), ("nt_env_maxrx", b"YANTRA_NET_MAX_RX=\0"),
                          ("nt_env_tmo", b"YANTRA_NET_TIMEOUT=\0"), ("nt_env_sent", b"YANTRA_NET_SENT=\0"),
                          ("nt_env_replay", b"YANTRA_NET_REPLAY=\0")] + \
            ([("nt_env_vd", b"YANTRA_VIRTIO_DEFER=\0")] if vdf else []):
        A.label(lab)
        A.raw(t)


def runtime_sock_win(A, base):
    """x86_64-windows: the socket window with NO device (there is no --listen here). Any load or
    store in SOCK..SOCK+16 halts Device { pc, addr, why: WHY_NO_DEVICE } and prints yantra-run's
    'socket: touched at' line, exactly as runtime_sock's rt_sk_dev does for a run without a
    listener (socket::load / store test the device first, so width and register never matter).
    rt_sk_ld / rt_sk_st: rax = addr; the write goes to fd 2 through the w_sys1 shim."""
    strs = []

    def say(text):
        lab = "skw_s%d" % len(strs)
        strs.append((lab, text))
        A.lea(RSI, ("abs", lab, 0))
        A.mov_rm_imm32(("r", RDX), len(text))
        A.mov_rm_imm32(("r", RDI), 2)
        A.call("w_sys1")

    why = dict(SOCK_WHY)["WHY_NO_DEVICE"]
    A.label("rt_sk_ld")
    A.label("rt_sk_st")
    A.mov_rm_r(("abs", "sk", SK_T0), RAX)
    A.call("rt_flush")
    say(b"halt: Device { pc: ")
    A.mov_r_rm(RAX, ("abs", "fault_off", 0), 0)
    A.mov_r_imm64(RCX, base)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    say(b", addr: ")
    A.mov_r_rm(RAX, ("abs", "sk", SK_T0))
    A.call("rt_putu")
    say(b", why: \"")
    A.call("rt_skw_why")
    say(b"\" }\n")
    say(b"socket: touched at ")
    A.mov_r_rm(RAX, ("abs", "sk", SK_T0))
    A.call("rt_puthex")
    say(b" with no socket device \xe2\x80\x94 ")
    A.call("rt_skw_why")
    say(b"\n")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    A.label("rt_skw_why")
    A.lea(RSI, ("abs", "skw_why", 0))
    A.mov_rm_imm32(("r", RDX), len(why))
    A.mov_rm_imm32(("r", RDI), 2)
    A.call("w_sys1")
    A.ret()
    for lab, text in strs + [("skw_why", why)]:
        A.label(lab)
        A.raw(text)


FJA_LITS = [19, 10, 3, 3, 4, 9, 1]                 # byte lengths of the strings fja_0..6 (registered with env_fj)


def runtime_fj(A):
    """FORK-JOIN on x86-64 Linux (FORKJOIN.md). rt_fj is called by the marker routine's first word
    with every guest register live. Phase 1, SASSEMBLY_NATIVE_THREADS = N > 0, lo < hi, the
    caller's sp inside the entry's stack and the slices' RAM untouched: [lo, hi) is cut into
    T = min(N, hi - lo) contiguous chunks, each run in order by one raw clone() thread with its
    own copy of the data block (fs = its delta: the guest registers not in host registers, the
    F/D state, the UART buffer, the fault words, the FAST sp window) and its own guest stack
    slice at the top of RAM. A worker's writes and its exit are CAPTURED (rt_fj_sys); the main
    thread joins the chunks IN INDEX ORDER (CLONE_CHILD_CLEARTID + futex, then mfence) and
    replays each one's records, so the first chunk that halted speaks exactly as the serial
    run would have, and later chunks are never waited for. Then a1 = hi: `mv a0, a1` answers
    hi and the serial loop runs nothing. Any other case returns at once: the serial loop runs."""
    R12, R13, R14, R15 = 12, 13, 14, 15

    def SYSCALL():                             # the real one (A.syscall() is rt_fj_sys here)
        A.raw(0x0F, 0x05)
    base = IMAGE_BASE                          # the one base cell
    _, off, dsize = fj_x86_layout()
    BLK = ((dsize + 0xFFF) & ~0xFFF) + FJ_HSTACK
    flags = 0x100 | 0x200 | 0x400 | 0x800 | 0x10000 | 0x40000 | 0x80000 | 0x200000   # VM FS FILES SIGHAND THREAD SYSVSEM SETTLS CHILD_CLEARTID

    def D(name, add=0):
        return ("abs", name, add)

    def B(reg, name, add=0):                   # a field of the block whose delta is in reg
        return ("mabs", reg, name, add)

    def gload(r, n):                           # main: guest xn as it was at the marker (rt_fj_args)
        A.mov_r_rm(r, D("fj_g", 8 * (10, 11, 12, 13, 14, 2).index(n)))

    def mfence():
        A.raw(0x0F, 0xAE, 0xF0)

    def block(rb, rd, creg):                   # rb = the block of chunk creg, rd = its delta
        A.mov_r_rm(rb, ("r", creg))
        A.mov_rm_imm32(("r", RDX), BLK)
        A.imul_r_rm(rb, ("r", RDX))
        A.alu_r_rm("add", rb, D("fj_pool"))
        A.lea(RAX, D("regs"))
        A.mov_r_rm(rd, ("r", rb))
        A.alu_r_rm("sub", rd, ("r", RAX))

    # ---------------- S1: the attestation line, at the exit funnel (rdi = the exit code, kept). One line to the
    # file SASSEMBLY_NATIVE_FJATTEST names (never stdout/stderr): p_req fj_p_max B K K0 spawned
    def fja_lit(k):
        A.lea(RSI, ("abs", "fja_%d" % k, 0))
        A.mov_rm_imm32(("r", RCX), FJA_LITS[k])
        A.raw(0xF3, 0xA4)                      # rep movsb

    def fja_num(k, name):
        fja_lit(k)
        A.mov_r_rm(RAX, D(name))
        A.call("rt_fja_dec")

    A.label("rt_fja_rep")
    A.push(RDI)
    A.lea(RDI, D("fj_abuf"))
    fja_num(0, "fj_n")
    fja_num(1, "fj_pmax")
    fja_num(2, "fj_q")
    fja_num(3, "fj_regions")
    fja_num(4, "fj_k")
    fja_num(5, "fj_spawned")
    fja_lit(6)
    A.lea(RSI, D("fj_abuf"))
    A.mov_r_rm(R8, ("r", RDI))
    A.alu_r_rm("sub", R8, ("r", RSI))          # the line's length
    A.mov_r_rm(RDI, D("fj_att"))
    A.mov_rm_imm32(("r", RSI), 0x241)          # O_WRONLY | O_CREAT | O_TRUNC
    A.mov_rm_imm32(("r", RDX), 0o644)
    A.mov_rm_imm32(("r", RAX), 2)
    SYSCALL()
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("s", "rt_fja_done")
    A.mov_r_rm(R9, ("r", RAX))
    A.mov_r_rm(RDI, ("r", R9))
    A.lea(RSI, D("fj_abuf"))
    A.mov_r_rm(RDX, ("r", R8))
    A.mov_rm_imm32(("r", RAX), 1)
    SYSCALL()
    A.mov_r_rm(RDI, ("r", R9))
    A.mov_rm_imm32(("r", RAX), 3)
    SYSCALL()
    A.label("rt_fja_done")
    A.pop(RDI)
    A.ret()
    A.label("rt_fja_dec")                      # rax as decimal at [rdi], rdi advanced
    A.lea(RSI, ("abs", "num", 31))
    A.mov_rm_imm32(("r", RCX), 10)
    A.label("rt_fja_d1")
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.f7(6, ("r", RCX))
    A.lea(RDX, ("m", RDX, 48))
    A.lea(RSI, ("m", RSI, -1))
    A.raw(0x88, 0x16)                          # mov [rsi], dl
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_fja_d1")
    A.lea(RCX, ("abs", "num", 31))
    A.alu_r_rm("sub", RCX, ("r", RSI))
    A.raw(0xF3, 0xA4)                          # rep movsb
    A.ret()

    # ---------------- the marker, main thread
    A.label("rt_fj")
    A.alu_rm_imm("cmp", D("fj_worker"), 0)
    A.jcc("ne", "rt_fj_r0")                    # in a worker (a nested region): serial
    for i, h in enumerate(HOST):
        A.mov_rm_r(D("fj_hsave", 8 * i), h)
    A.call("rt_fj_args")
    gload(RAX, 10)
    A.alu_rm_imm("cmp", ("r", RAX), 1)
    A.jcc("ne", "rt_fj_out")
    A.mov_r_rm(RCX, D("fj_n"))
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("e", "rt_fj_out")
    gload(R8, 11)
    gload(R9, 12)
    A.alu_r_rm("cmp", R8, ("r", R9))
    A.jcc("ae", "rt_fj_out")                   # empty, or lo above hi: the serial loop runs nothing
    A.mov_r_rm(R10, ("r", R9))
    A.alu_r_rm("sub", R10, ("r", R8))          # count
    A.alu_r_rm("cmp", RCX, ("r", R10))
    A.jcc("be", "rt_fj_t")
    A.mov_r_rm(RCX, ("r", R10))
    A.label("rt_fj_t")
    # the small-region threshold: at most count // K threads, and serial below two
    A.mov_r_rm(R11, D("fj_k"))
    A.test_rm_r(("r", R11), R11)
    A.jcc("e", "rt_fj_tk")
    A.mov_r_rm(RAX, ("r", R10))
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.f7(6, ("r", R11))
    A.alu_r_rm("cmp", RAX, ("r", RCX))
    A.jcc("ae", "rt_fj_tk2")
    A.mov_r_rm(RCX, ("r", RAX))
    A.label("rt_fj_tk2")
    A.alu_rm_imm("cmp", ("r", RCX), 2)
    A.jcc("b", "rt_fj_out")
    A.label("rt_fj_tk")
    A.mov_rm_r(D("fj_t"), RCX)
    A.mov_rm_r(D("fj_lo"), R8)
    A.mov_rm_r(D("fj_hiall"), R9)
    A.mov_r_rm(RAX, ("r", R10))
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.f7(6, ("r", RCX))                        # q, r = count / T
    A.mov_rm_r(D("fj_q"), RAX)
    A.mov_rm_r(D("fj_r"), RDX)
    gload(RAX, 13)
    A.mov_rm_r(D("fj_out"), RAX)
    gload(RAX, 14)
    A.mov_rm_r(D("fj_body"), RAX)
    # the caller's sp in (SB + 2056, sp0]: depth = sp - SB - 2056, the deepest a worker may go
    gload(RAX, 2)
    A.alu_r_rm("cmp", RAX, D("fj_sp0"))
    A.jcc("a", "rt_fj_out")
    A.mov_r_rm(RDX, D("fj_sbx"))
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    A.jcc("be", "rt_fj_out")
    A.alu_r_rm("sub", RAX, ("r", RDX))
    A.mov_rm_r(D("fj_depth"), RAX)
    # the slices: T x FJ_SLICE at the top of RAM, above the image's extent, all zero
    A.mov_r_rm(RAX, D("fj_t"))
    A.mov_rm_imm32(("r", RDX), FJ_SLICE)
    A.imul_r_rm(RAX, ("r", RDX))
    A.mov_r_imm64(RDX, base)
    A.alu_r_rm("add", RDX, D("ram"))
    A.alu_r_rm("sub", RDX, ("r", RAX))
    A.alu_rm_imm("and", ("r", RDX), -4096)
    A.alu_r_rm("cmp", RDX, D("fj_ext"))
    A.jcc("b", "rt_fj_out")
    A.mov_rm_r(D("fj_area"), RDX)
    A.mov_r_rm(RDI, ("r", RDX))
    A.mov_r_rm(RAX, D("fj_t"))
    A.mov_rm_imm32(("r", RCX), FJ_SLICE // 8)
    A.imul_r_rm(RCX, ("r", RAX))
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.raw(0xF3, 0x48, 0xAF)                    # repe scasq
    A.jcc("ne", "rt_fj_out")                   # something lives there: serial
    # the pool of worker blocks (data copy + host stack), mapped once
    A.mov_r_rm(RAX, D("fj_pool"))
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("ne", "rt_fj_have")
    A.alu_r_rm("xor", RDI, ("r", RDI), 0)
    A.mov_r_imm64(RSI, FJ_MAX * BLK)
    A.mov_rm_imm32(("r", RDX), 3)
    A.mov_rm_imm32(("r", R10), 0x22)
    A.mov_rm_imm32(("r", R8), -1)
    A.alu_r_rm("xor", R9, ("r", R9), 0)
    A.mov_rm_imm32(("r", RAX), 9)
    SYSCALL()
    A.alu_rm_imm("cmp", ("r", RAX), -4096)
    A.jcc("a", "rt_fj_out")
    A.mov_rm_r(D("fj_pool"), RAX)
    A.label("rt_fj_have")
    A.call("rt_flush")                         # the main thread's output so far goes first
    A.mov_r_rm(RAX, D("steps"))                # the budget at the fork: every worker starts with it
    A.mov_rm_r(D("fj_stp"), RAX)
    # ---------------- spawn chunk c = 0 .. T-1
    A.mov_rm_imm32(D("fj_c"), 0)
    A.label("rt_fj_sp")
    A.mov_r_rm(RCX, D("fj_c"))
    A.alu_r_rm("cmp", RCX, D("fj_t"))
    A.jcc("ae", "rt_fj_join")
    block(RBX, RBP, RCX)
    # the data block, but for the UART buffer and the capture: [regs, outbuf), [outbuf + 64 KiB, fj_cap)
    A.mov_r_rm(RDI, ("r", RBX))
    A.lea(RSI, D("regs"))
    A.mov_rm_imm32(("r", RCX), off["outbuf"])
    A.raw(0xF3, 0xA4)
    A.lea(RDI, ("m", RBX, off["outbuf"] + 65536))
    A.lea(RSI, D("outbuf", 65536))
    A.mov_rm_imm32(("r", RCX), off["fj_cap"] - off["outbuf"] - 65536)
    A.raw(0xF3, 0xA4)
    A.mov_rm_imm32(B(RBP, "fj_worker"), 1)
    A.mov_rm_r(B(RBP, "fj_delta"), RBP)
    for f in ("fj_kind", "fj_code", "fj_caplen", "fj_capover", "outlen"):
        A.mov_rm_imm32(B(RBP, f), 0)
    # the chunk: lo + c q + min(c, r), q + (c < r) indices
    A.mov_r_rm(RAX, D("fj_c"))
    A.mov_r_rm(RCX, ("r", RAX))
    A.imul_r_rm(RCX, D("fj_q"))
    A.alu_r_rm("add", RCX, D("fj_lo"))
    A.mov_r_rm(RDX, D("fj_r"))
    A.alu_r_rm("cmp", RAX, ("r", RDX))
    A.jcc("ae", "rt_fj_c1")
    A.alu_r_rm("add", RCX, ("r", RAX))
    A.jmp("rt_fj_c2")
    A.label("rt_fj_c1")
    A.alu_r_rm("add", RCX, ("r", RDX))
    A.label("rt_fj_c2")
    A.mov_rm_r(B(RBP, "fj_i"), RCX)
    A.alu_r_rm("add", RCX, D("fj_q"))
    A.alu_r_rm("cmp", RAX, D("fj_r"))
    A.jcc("ae", "rt_fj_c3")
    A.alu_rm_imm("add", ("r", RCX), 1)
    A.label("rt_fj_c3")
    A.mov_rm_r(B(RBP, "fj_hi"), RCX)
    A.mov_r_rm(RCX, D("fj_out"))
    A.mov_rm_r(B(RBP, "fj_bout"), RCX)
    A.mov_r_rm(RCX, D("fj_body"))
    A.mov_rm_r(B(RBP, "fj_bbody"), RCX)
    # its slice [S, S + FJ_SLICE): the body starts at S + FJ_SLICE - 4096 and its FAST window
    # is [start - depth, start + 8]: inside it no sp+imm access leaves the slice, and the serial
    # run's at the same depth stays above the canary word SB
    A.mov_rm_imm32(("r", RDX), FJ_SLICE)
    A.imul_r_rm(RAX, ("r", RDX))
    A.alu_r_rm("add", RAX, D("fj_area"))
    A.alu_rm_imm("add", ("r", RAX), FJ_SLICE - 4096)
    A.mov_rm_r(B(RBP, "fj_sp"), RAX)
    A.alu_r_rm("sub", RAX, D("fj_depth"))
    A.mov_rm_r(B(RBP, "sp_lo"), RAX)
    A.mov_r_rm(RAX, D("fj_depth"))
    A.alu_rm_imm("add", ("r", RAX), 9)
    A.mov_rm_r(B(RBP, "sp_span"), RAX)
    # its CLEARTID word = 1; clone(flags, stack top, 0, &word, tls = delta)
    A.mov_r_rm(RAX, D("fj_c"))
    A.lea(R10, D("fj_tids"))
    A.shift_imm("shl", ("r", RAX), 3)
    A.alu_r_rm("add", R10, ("r", RAX))
    A.mov_rm_imm32(("m", R10, 0), 1)
    A.mov_rm_imm32(("r", RDI), flags)
    A.lea(RSI, ("m", RBX, BLK))
    A.alu_r_rm("xor", RDX, ("r", RDX), 0)
    A.mov_r_rm(R8, ("r", RBP))
    A.mov_rm_imm32(("r", RAX), 56)
    SYSCALL()
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_fj_worker")
    A.jcc("s", "rt_fj_nospawn")
    A.alu_rm_imm("add", D("fj_spawned"), 1)     # S1 attestation: one chunk really spawned
    A.mov_r_rm(RAX, D("fj_c"))
    A.alu_rm_imm("add", ("r", RAX), 1)
    A.mov_rm_r(D("fj_c"), RAX)
    A.jmp("rt_fj_sp")
    A.label("rt_fj_nospawn")
    A.alu_rm_imm("cmp", D("fj_c"), 0)
    A.jcc("e", "rt_fj_out")                    # no thread at all: serial
    A.jmp("rt_fj_fall")
    # ---------------- join chunk c = 0 .. T-1, in order
    A.label("rt_fj_join")
    A.mov_rm_imm32(D("fj_c"), 0)
    A.label("rt_fj_jl")
    A.mov_r_rm(RCX, D("fj_c"))
    A.alu_r_rm("cmp", RCX, D("fj_t"))
    A.jcc("ae", "rt_fj_all")
    A.lea(RDI, D("fj_tids"))
    A.shift_imm("shl", ("r", RCX), 3)
    A.alu_r_rm("add", RDI, ("r", RCX))
    A.label("rt_fj_jw")
    A.mov_r_rm(RDX, ("m", RDI, 0), 0)
    A.test_rm_r(("r", RDX), RDX, 0)
    A.jcc("e", "rt_fj_jd")
    A.push(RDI)
    A.alu_r_rm("xor", RSI, ("r", RSI), 0)      # FUTEX_WAIT (shared: CLEARTID wakes shared)
    A.alu_r_rm("xor", R10, ("r", R10), 0)
    A.mov_rm_imm32(("r", RAX), 202)
    SYSCALL()
    A.pop(RDI)
    A.jmp("rt_fj_jw")
    A.label("rt_fj_jd")
    mfence()
    A.mov_r_rm(RCX, D("fj_c"))
    block(RBX, RBP, RCX)
    A.alu_rm_imm("cmp", B(RBP, "fj_capover"), 0)
    A.jcc("ne", "rt_fj_fall")
    A.mov_r_rm(RAX, B(RBP, "fj_kind"))
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("e", "rt_fj_fall")
    A.alu_rm_imm("cmp", ("r", RAX), 3)
    A.jcc("ae", "rt_fj_fall")
    # the words it ran are the serial run's: off the main thread's budget; a borrow means the limit is
    # in this region, so the serial run (yantra-run) names the exact step
    A.mov_r_rm(RAX, D("fj_stp"))
    A.alu_r_rm("sub", RAX, B(RBP, "steps"))
    A.op(b"\x29", RAX, D("steps"), 1)           # sub [steps], rax
    A.jcc("b", "rt_fj_fall")
    # replay its records: (fd, n, n octets padded to 8)
    A.op(b"\x8d", R9, B(RBP, "fj_cap"))
    A.mov_r_rm(R12, ("r", R9))
    A.alu_r_rm("add", R12, B(RBP, "fj_caplen"))
    A.label("rt_fj_rl")
    A.alu_r_rm("cmp", R9, ("r", R12))
    A.jcc("ae", "rt_fj_rd")
    A.mov_r_rm(R15, ("m", R9, 0))
    A.mov_r_rm(R13, ("m", R9, 8))
    A.lea(R14, ("m", R9, 16))
    A.mov_r_rm(RAX, ("r", R13))
    A.alu_rm_imm("add", ("r", RAX), 16 + 7)
    A.alu_rm_imm("and", ("r", RAX), -8)
    A.alu_r_rm("add", R9, ("r", RAX))
    A.label("rt_fj_rw")
    A.test_rm_r(("r", R13), R13)
    A.jcc("e", "rt_fj_rl")
    A.mov_r_rm(RDI, ("r", R15))
    A.mov_r_rm(RSI, ("r", R14))
    A.mov_r_rm(RDX, ("r", R13))
    A.mov_rm_imm32(("r", RAX), 1)
    SYSCALL()
    A.test_rm_r(("r", RAX), RAX)
    A.jcc("le", "rt_fj_rl")
    A.alu_r_rm("add", R14, ("r", RAX))
    A.alu_r_rm("sub", R13, ("r", RAX))
    A.jmp("rt_fj_rw")
    A.label("rt_fj_rd")
    A.alu_rm_imm("cmp", B(RBP, "fj_kind"), 2)
    A.jcc("ne", "rt_fj_k1")
    A.mov_r_rm(RDI, B(RBP, "fj_code"))         # it halted: the run ends as the serial run ends
    A.mov_rm_imm32(("r", RAX), 231)
    SYSCALL()
    A.label("rt_fj_k1")
    if True:
        # fflags are sticky: OR the chunk's into the main thread's; a changed frm or FS is not merged
        A.mov_r_rm(RAX, B(RBP, "fcsr"))
        A.mov_r_rm(RCX, D("fcsr"))
        A.mov_r_rm(RDX, ("r", RAX))
        A.shift_imm("shr", ("r", RDX), 5)
        A.mov_r_rm(R8, ("r", RCX))
        A.shift_imm("shr", ("r", R8), 5)
        A.alu_r_rm("cmp", RDX, ("r", R8))
        A.jcc("ne", "rt_fj_fall")
        A.alu_rm_imm("and", ("r", RAX), 31)
        A.alu_r_rm("or", RCX, ("r", RAX))
        A.mov_rm_r(D("fcsr"), RCX)
        A.mov_r_rm(RAX, B(RBP, "fsen"))
        A.alu_r_rm("cmp", RAX, D("fsen"))
        A.jcc("ne", "rt_fj_fall")
    A.mov_r_rm(RAX, D("fj_c"))
    A.alu_rm_imm("add", ("r", RAX), 1)
    A.mov_rm_r(D("fj_c"), RAX)
    A.jmp("rt_fj_jl")
    A.label("rt_fj_all")
    # S1 attestation, only here (after the join; the fallback path never gets here): fj_pmax = max(fj_pmax, T), K += 1
    A.alu_rm_imm("add", D("fj_regions"), 1)
    A.mov_r_rm(RAX, D("fj_t"))
    A.alu_r_rm("cmp", RAX, D("fj_pmax"))
    A.jcc("be", "rt_fj_pm")
    A.mov_rm_r(D("fj_pmax"), RAX)
    A.label("rt_fj_pm")
    # every chunk done: the slices back to zero (RAM as the serial run left it there), a1 = hi
    A.mov_r_rm(RDI, D("fj_area"))
    A.mov_r_rm(RAX, D("fj_t"))
    A.mov_rm_imm32(("r", RCX), FJ_SLICE)
    A.imul_r_rm(RCX, ("r", RAX))
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.raw(0xF3, 0xAA)                          # rep stosb
    A.mov_r_rm(RAX, D("fj_hiall"))
    A.call("rt_fj_seta1")
    A.label("rt_fj_out")
    for i, h in enumerate(HOST):
        A.mov_r_rm(h, D("fj_hsave", 8 * i))
    A.label("rt_fj_r0")
    A.ret()
    A.label("rt_fj_fall")                      # what the threads did is not the serial run's: yantra
    A.call("m_fjfall")
    A.mov_rm_imm32(("r", RDI), 1)
    A.mov_rm_imm32(("r", RAX), 231)
    SYSCALL()
    text = (b"native: not in milestone 1 (a fork-join worker made a host call, left its stack slice, "
            b"overflowed its capture or changed the rounding mode; run it under yantra-run)\n")
    A.label("m_fjfall")
    A.lea(RSI, ("abs", "m_fjfall_s", 0))
    A.mov_rm_imm32(("r", RDX), len(text))
    A.mov_rm_imm32(("r", RDI), 2)
    A.mov_rm_imm32(("r", RAX), 1)
    SYSCALL()
    A.ret()

    # ---------------- a worker (fs = its delta, rsp = its host stack)
    A.label("rt_fj_worker")
    for i, h in enumerate(HOST):
        A.mov_r_rm(h, D("fj_hsave", 8 * i))
    A.label("rt_fj_next")
    A.mov_r_rm(RAX, D("fj_i"))
    A.alu_r_rm("cmp", RAX, D("fj_hi"))
    A.jcc("ae", "rt_fj_fin")
    A.call("rt_fj_wregs")
    A.mov_r_rm(RAX, D("fj_bbody"))
    A.jmp("rt_indirect")
    A.label("rt_fj_ret")                       # rt_bad_target: the body returned
    A.mov_r_rm(RAX, D("fj_i"))
    A.alu_rm_imm("add", ("r", RAX), 1)
    A.mov_rm_r(D("fj_i"), RAX)
    A.jmp("rt_fj_next")
    A.label("rt_fj_fin")
    A.call("rt_flush")
    A.mov_rm_imm32(D("fj_kind"), 1)
    A.jmp("rt_fj_wdone")
    A.label("rt_fj_stepw")                     # the step limit is met in a worker: the serial run (yantra) speaks
    A.mov_rm_imm32(D("fj_kind"), 3)
    A.jmp("rt_fj_wdone")
    A.label("rt_fj_stack")                     # XF_: its sp left the FAST window of its slice
    A.mov_rm_imm32(D("fj_kind"), 4)
    A.label("rt_fj_wdone")
    mfence()
    A.alu_r_rm("xor", RDI, ("r", RDI), 0)
    A.mov_rm_imm32(("r", RAX), 60)             # exit (this thread); the kernel clears its word and wakes
    SYSCALL()

    # ---------------- every host call: the main thread's as it was (exit is exit_group now that
    # threads exist); a worker's write is captured, its exit recorded, anything else refused
    A.label("rt_fj_sys")
    A.alu_rm_imm("cmp", D("fj_worker"), 0)
    A.jcc("ne", "rt_fj_sw")
    A.alu_rm_imm("cmp", ("r", RAX), 60)
    A.jcc("ne", "rt_fj_s1")
    A.mov_rm_imm32(("r", RAX), 231)
    A.label("rt_fj_s1")
    SYSCALL()
    A.ret()
    A.label("rt_fj_sw")
    A.alu_rm_imm("cmp", ("r", RAX), 1)
    A.jcc("e", "rt_fj_ww")
    A.alu_rm_imm("cmp", ("r", RAX), 60)
    A.jcc("e", "rt_fj_we")
    A.mov_rm_imm32(D("fj_kind"), 3)
    A.jmp("rt_fj_wdone")
    A.label("rt_fj_we")
    A.mov_rm_r(D("fj_code"), RDI)
    A.mov_rm_imm32(D("fj_kind"), 2)
    A.jmp("rt_fj_wdone")
    A.label("rt_fj_ww")
    for reg in (RSI, RDI, RCX, RDX, R8):
        A.push(reg)
    A.lea(R8, ("m", RDX, 16 + 7))
    A.alu_rm_imm("and", ("r", R8), -8)
    A.mov_r_rm(RAX, D("fj_caplen"))
    A.mov_r_rm(RCX, ("r", RAX))
    A.alu_r_rm("add", RCX, ("r", R8))
    A.alu_rm_imm("cmp", ("r", RCX), FJ_CAP)
    A.jcc("a", "rt_fj_wo")
    A.mov_rm_r(D("fj_caplen"), RCX)
    A.lea(RCX, D("fj_cap"))
    A.alu_r_rm("add", RCX, ("r", RAX))
    A.mov_rm_r(("m", RCX, 0), RDI)
    A.mov_rm_r(("m", RCX, 8), RDX)
    A.lea(RDI, ("m", RCX, 16))
    A.mov_r_rm(RCX, ("r", RDX))
    A.raw(0xF3, 0xA4)                          # rep movsb
    A.jmp("rt_fj_wx")
    A.label("rt_fj_wo")
    A.mov_rm_imm32(D("fj_capover"), 1)
    A.label("rt_fj_wx")
    for reg in (R8, RDX, RCX, RDI, RSI):
        A.pop(reg)
    A.mov_r_rm(RAX, ("r", RDX))
    A.ret()
    # a store to a device other than the UART, WAIT and FINISHER (files, spawn, ...): not from a worker
    A.label("rt_st_fjdev")
    A.alu_rm_imm("cmp", D("fj_worker"), 0)
    A.jcc("e", "rt_st_win")
    A.mov_rm_imm32(D("fj_kind"), 3)
    A.jmp("rt_fj_wdone")
    A.label("m_fjfall_s")
    A.raw(text)


class Wide(int):
    """a guest-base-derived constant: loaded in its 64-bit form whatever its value, so the octets of a
    runtime have the same shape for every base (the recorders diff them); a difference of two is plain"""
    def __add__(self, o):
        return Wide(int(self) + int(o))
    __radd__ = __add__

    def __sub__(self, o):
        return int(self) - int(o) if isinstance(o, Wide) else Wide(int(self) - int(o))


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
            if -(1 << 31) <= sv < (1 << 31) and not isinstance(x, Wide):
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


# ---------------------------------------------------------------- the V extension (V-007's executed subset, e64)
# yantra's vector unit (crates/yantra/src/vector.rs): VLEN 128, 32 registers of 2 x 64-bit lanes, element i of the
# group at reg is the u64 at vregs + 16 reg + 8 i. vt holds vtype ^ VILL, so the zeroed data is vill (yantra's reset);
# vsen is sstatus.VS (0 off; 3 once dirtied, as yantra marks it). Every vector word lowers to ONE call of rt_vec,
# which decodes the word (fault_word) at run time: what yantra makes an illegal instruction halts as yantra does
# (rt_illegal); what yantra refuses (Unimplemented) or an element outside the plain RAM window is rt_bad_insn,
# so t1-run reruns the whole program under yantra-run, which says it exactly.
VEC_DATA = [("vregs", 512), ("vl", 8), ("vt", 8), ("vstart", 8), ("vsen", 8)]
VILL = 1 << 63
VEC_INT = {(0, 0x00): ("add", "v"), (0, 0x02): ("sub", "v"), (4, 0x00): ("add", "x"), (4, 0x02): ("sub", "x"),
           (2, 0x25): ("mul", "v"), (6, 0x25): ("mul", "x")}
VEC_FP = {(1, 0x00): ("fadd.d", "v"), (1, 0x02): ("fsub.d", "v"), (1, 0x24): ("fmul.d", "v"), (1, 0x20): ("fdiv.d", "v"),
          (5, 0x00): ("fadd.d", "f"), (5, 0x02): ("fsub.d", "f"), (5, 0x24): ("fmul.d", "f"), (5, 0x20): ("fdiv.d", "f")}


def runtime_amo(A, wm=False):
    """rt_amo (RV64A): called from the lowering with fault_off / fault_word = the site and the word, rax = x[rs1]
    (the address, no immediate), rdx = x[rs2]; answers rax = the new x[rd]. Clobbers rax rcx rdx (rsi rdi are
    kept). yantra's order: alignment (BadAccess), then sc (the reservation: resv, 0 = none), then lr's rs2 check,
    then the load, then lr / the funct5 switch (any other: Unimplemented), then the store."""
    A.label("rt_amo")
    A.push(RSI)
    A.push(RDI)
    A.mov_r_rm(RDI, ("abs", "fault_word", 0), 0)
    A.mov_r_rm(RSI, ("r", RDI), 0)
    A.shift_imm("shr", ("r", RSI), 27, 0)               # esi = funct5
    A.shift_imm("shr", ("r", RDI), 12, 0)
    A.alu_rm_imm("and", ("r", RDI), 1, 0)               # edi = 0 (.w) / 1 (.d)
    A.mov_r_rm(RCX, ("r", RDI), 0)
    A.shift_imm("shl", ("r", RCX), 2, 0)
    A.alu_rm_imm("or", ("r", RCX), 3, 0)                # ecx = width - 1
    A.test_rm_r(("r", RAX), RCX)
    A.jcc("ne", "ra_mis")
    A.test_rm_r(("r", RDI), RDI, 0)
    A.jcc("ne", "ra_w0")
    A.movsx(RDX, ("r", RDX), 4)                         # .w: the operand sign-extended (min / max compare it)
    A.label("ra_w0")
    A.alu_rm_imm("cmp", ("r", RSI), 3, 0)
    A.jcc("e", "ra_sc")
    A.alu_rm_imm("cmp", ("r", RSI), 2, 0)
    A.jcc("ne", "ra_ld")
    A.mov_r_rm(RCX, ("abs", "fault_word", 0), 0)
    A.shift_imm("shr", ("r", RCX), 20, 0)
    A.alu_rm_imm("and", ("r", RCX), 31, 0)
    A.jcc("ne", "rt_unimpl")                            # lr with rs2 != 0
    A.label("ra_ld")
    A.lea(RCX, ("m", RAX, -IMAGE_BASE))
    A.test_rm_r(("r", RDI), RDI, 0)
    A.jcc("ne", "ra_l8")
    A.alu_r_rm("cmp", RCX, ("abs", "lim_l4", 0))
    A.jcc("ae", "ra_lc4")
    A.movsx(RCX, ("m", RAX, 0), 4)
    A.jmp("ra_lok")
    A.label("ra_l8")
    A.alu_r_rm("cmp", RCX, ("abs", "lim_l8", 0))
    A.jcc("ae", "ra_lc8")
    A.mov_r_rm(RCX, ("m", RAX, 0))
    A.label("ra_lok")                                   # rcx = the old value
    A.alu_rm_imm("cmp", ("r", RSI), 2, 0)
    A.jcc("e", "ra_lr")
    for f5, lab in ((0, "ra_add"), (1, "ra_st"), (4, "ra_xor"), (8, "ra_or"), (0xC, "ra_and"),
                    (0x10, "ra_min"), (0x14, "ra_max"), (0x18, "ra_minu"), (0x1C, "ra_maxu")):
        A.alu_rm_imm("cmp", ("r", RSI), f5, 0)
        A.jcc("e", lab)
    A.jmp("rt_unimpl")
    for lab, nm in (("ra_add", "add"), ("ra_xor", "xor"), ("ra_or", "or"), ("ra_and", "and")):
        A.label(lab)
        A.alu_r_rm(nm, RDX, ("r", RCX))
        A.jmp("ra_st")
    for lab, keep in (("ra_min", "ge"), ("ra_max", "le"), ("ra_minu", "ae"), ("ra_maxu", "be")):
        A.label(lab)                                    # rdx = the new value: the old one when it wins
        A.alu_r_rm("cmp", RCX, ("r", RDX))
        A.jcc(keep, "ra_st")
        A.mov_r_rm(RDX, ("r", RCX))
        A.jmp("ra_st")
    A.label("ra_st")                                    # store rdx at rax; the answer is the old value (rcx)
    A.mov_r_rm(RSI, ("r", RAX))
    A.alu_r_rm("sub", RSI, ("abs", "code_end", 0))
    A.test_rm_r(("r", RDI), RDI, 0)
    A.jcc("ne", "ra_s8")
    A.alu_r_rm("cmp", RSI, ("abs", "lim_s4", 0))
    A.jcc("ae", "ra_sc4")
    if wm:
        A.lea(RSI, ("m", RAX, 4))
        A.alu_r_rm("cmp", RSI, ("abs", "hw", 0))
        A.jcc("be", "ra_h4")
        A.mov_rm_r(("abs", "hw", 0), RSI)
        A.label("ra_h4")
    A.mov_rm_r(("m", RAX, 0), RDX, width=4)
    A.jmp("ra_done")
    A.label("ra_s8")
    A.alu_r_rm("cmp", RSI, ("abs", "lim_s8", 0))
    A.jcc("ae", "ra_sc8")
    if wm:
        A.lea(RSI, ("m", RAX, 8))
        A.alu_r_rm("cmp", RSI, ("abs", "hw", 0))
        A.jcc("be", "ra_h8")
        A.mov_rm_r(("abs", "hw", 0), RSI)
        A.label("ra_h8")
    A.mov_rm_r(("m", RAX, 0), RDX)
    A.label("ra_done")
    A.mov_r_rm(RAX, ("r", RCX))
    A.pop(RDI)
    A.pop(RSI)
    A.ret()
    A.label("ra_lr")                                    # lr: the reservation is the address; rd = the old value
    A.mov_rm_r(("abs", "resv", 0), RAX)
    A.jmp("ra_done")
    A.label("ra_sc")                                    # sc: the reservation is spent either way; rd = 0 on success
    A.mov_r_rm(RCX, ("abs", "resv", 0))
    A.mov_rm_imm32(("abs", "resv", 0), 0)
    A.test_rm_r(("r", RCX), RCX)
    A.jcc("e", "ra_fail")
    A.alu_r_rm("cmp", RCX, ("r", RAX))
    A.jcc("ne", "ra_fail")
    A.alu_r_rm("xor", RCX, ("r", RCX), 0)
    A.jmp("ra_st")
    A.label("ra_fail")
    A.mov_rm_imm32(("r", RAX), 1, 0)
    A.pop(RDI)
    A.pop(RSI)
    A.ret()
    A.label("ra_mis")
    A.mov_rm_r(("abs", "fault_addr", 0), RAX)
    A.call("rt_flush")
    A.jmp("rt_bada")
    for lab, w in (("ra_lc4", 4), ("ra_lc8", 8)):       # a load that missed RAM: the runtime's load halts
        A.label(lab)
        A.mov_rm_imm32(("r", RDX), w, 0)
        A.call("rt_load_slow")
        A.jmp("rt_bad_insn")                            # (only the socket's RX returns: refused)
    for lab, w in (("ra_sc4", 4), ("ra_sc8", 8)):       # a store that missed RAM
        A.label(lab)
        A.mov_r_rm(RCX, ("r", RDX))
        A.mov_rm_imm32(("r", RDX), w, 0)
        A.call("rt_store_slow")
        A.jmp("rt_bad_insn")


def runtime_vec(A, wm=False):
    """rt_vec: called from the lowering with fault_off / fault_word = the site and the word, rax = x[rs1],
    rcx = x[rs2]; answers rax = the new x[rd] (vsetvli / vsetivli / vsetvl). Every host register but rax rcx rdx
    is kept (the F/D helpers it calls touch rax rcx rdx xmm0-7)."""
    SAVE = (RBX, RBP, RSI, RDI, R8, R9, R10, R11)
    n = [0]

    def lab():
        n[0] += 1
        return "rvl%d" % n[0]

    def fld(r, sh, mask):                       # r = (word >> sh) & mask
        A.mov_r_rm(r, ("r", RBX), 0)
        if sh:
            A.shift_imm("shr", ("r", r), sh, 0)
        A.alu_rm_imm("and", ("r", r), mask, 0)

    def cmp32(r, v):
        A.alu_rm_imm("cmp", ("r", r), v, 0)

    def group(r, sh):                           # r = &vregs + 16 * (the register field at sh)
        fld(r, sh, 31)
        A.shift_imm("shl", ("r", r), 4)
        A.lea(RAX, ("abs", "vregs", 0))
        A.alu_r_rm("add", r, ("r", RAX))

    def at_start(r):                            # r += 8 * vstart
        A.mov_r_rm(RAX, ("abs", "vstart", 0))
        A.shift_imm("shl", ("r", RAX), 3)
        A.alu_r_rm("add", r, ("r", RAX))

    def executes(fields):
        """vill: illegal; a vtype the engine does not compute at: refused; a register field (shift) not a multiple
        of LMUL: illegal. Leaves rdx = LMUL - 1."""
        A.mov_r_rm(RAX, ("abs", "vt", 0))
        A.test_rm_r(("r", RAX), RAX)
        A.jcc("e", "rv_ill")
        A.mov_r_rm(RDX, ("r", RAX), 0)          # vtype's low half (vtype < 256)
        A.alu_rm_imm("and", ("r", RDX), 0x3C, 0)
        cmp32(RDX, 0x18)                        # vsew 3 (e64) and vlmul 0..3
        A.jcc("ne", "rv_bad")
        A.mov_r_rm(RCX, ("r", RAX), 0)
        A.alu_rm_imm("and", ("r", RCX), 3, 0)
        A.mov_rm_imm32(("r", RDX), 1, 0)
        A.shift_cl("shl", ("r", RDX), 0)
        A.alu_rm_imm("sub", ("r", RDX), 1, 0)
        for sh in fields:
            fld(RCX, sh, 31)
            A.test_rm_r(("r", RCX), RDX, 0)
            A.jcc("ne", "rv_ill")

    def frm(r):                                 # r = frm; a reserved one (5..7) is illegal
        A.mov_r_rm(r, ("abs", "fcsr", 0), 0)
        A.shift_imm("shr", ("r", r), 5, 0)
        A.alu_rm_imm("and", ("r", r), 7, 0)
        cmp32(r, 4)
        A.jcc("a", "rv_ill")

    def movq_xm(x, m):
        A.sse(0xF3, b"\x0f\x7e", x, m, 0)

    def movq_mx(m, x):
        A.sse(0x66, b"\x0f\xd6", x, m, 0)

    def loop(body, step=(R10, R11, RBP)):
        """for i in vstart..vl: body(); every pointer in step advances 8"""
        top, out = lab(), lab()
        A.mov_r_rm(R8, ("abs", "vstart", 0))
        A.mov_r_rm(R9, ("abs", "vl", 0))
        A.label(top)
        A.alu_r_rm("cmp", R8, ("r", R9))
        A.jcc("ae", out)
        body()
        for r in step:
            A.alu_rm_imm("add", ("r", r), 8)
        A.alu_rm_imm("add", ("r", R8), 1)
        A.jmp(top)
        A.label(out)

    def done(dirty=True):
        if dirty:
            A.mov_rm_imm32(("abs", "vsen", 0), 3, 0)
        A.mov_rm_imm32(("abs", "vstart", 0), 0)
        A.jmp("rv_ret")

    A.label("rt_vec")
    for r in SAVE:
        A.push(r)
    A.mov_r_rm(RSI, ("r", RAX))
    A.mov_r_rm(RDI, ("r", RCX))
    A.mov_r_rm(RBX, ("abs", "fault_word", 0), 0)
    A.alu_rm_imm("cmp", ("abs", "vsen", 0), 0, 0)       # sstatus.VS off: illegal (yantra's step_inner)
    A.jcc("e", "rv_ill")
    fld(RDX, 0, 0x7F)
    cmp32(RDX, 0x57)
    A.jcc("ne", "rv_mem")
    fld(R10, 12, 7)
    fs_, nofs = lab(), lab()
    cmp32(R10, 1)
    A.jcc("e", fs_)
    cmp32(R10, 5)
    A.jcc("ne", nofs)
    A.label(fs_)
    A.alu_rm_imm("cmp", ("abs", "fsen", 0), 0, 0)       # a float source with FS off: illegal
    A.jcc("e", "rv_ill")
    A.label(nofs)
    cmp32(R10, 7)
    A.jcc("e", "rv_set")
    fld(RDX, 25, 1)                                     # masked: outside the subset
    A.jcc("e", "rv_err")
    fld(R11, 26, 63)
    A.shift_imm("shl", ("r", R10), 8, 0)
    A.alu_r_rm("or", R10, ("r", R11), 0)                # r10 = funct3 << 8 | funct6
    for key in list(VEC_INT) + list(VEC_FP) + [(1, 0x03)]:
        cmp32(R10, key[0] << 8 | key[1])
        A.jcc("e", "rv_%d_%d" % key)
    A.label("rv_err")                                   # decode refused it: illegal under vill, else refused
    A.alu_rm_imm("cmp", ("abs", "vt", 0), 0)
    A.jcc("e", "rv_ill")
    A.jmp("rv_bad")
    # ---- integer .vv / .vx
    for key, (nm, src) in VEC_INT.items():
        A.label("rv_%d_%d" % key)
        executes((7, 20) + ((15,) if src == "v" else ()))
        group(R10, 7)
        group(R11, 20)
        at_start(R10)
        at_start(R11)
        if src == "v":
            group(RBP, 15)
            at_start(RBP)

        def body(nm=nm, src=src):
            A.mov_r_rm(RAX, ("m", R11, 0))
            A.mov_r_rm(RCX, ("m", RBP, 0) if src == "v" else ("r", RSI))
            if nm == "mul":
                A.imul_r_rm(RAX, ("r", RCX))
            else:
                A.alu_r_rm(nm, RAX, ("r", RCX))
            A.mov_rm_r(("m", R10, 0), RAX)
        loop(body)
        done()
    # ---- float .vv / .vf: element by element through the scalar helper, rm = frm (flags accrue as yantra's)
    for key, (nm, src) in VEC_FP.items():
        A.label("rv_%d_%d" % key)
        executes((7, 20) + ((15,) if src == "v" else ()))
        group(R10, 7)
        group(R11, 20)
        if src == "v":
            group(RBP, 15)
            at_start(RBP)
        else:
            fld(RBP, 15, 31)
            A.shift_imm("shl", ("r", RBP), 3)
            A.lea(RAX, ("abs", "fregs", 0))
            A.alu_r_rm("add", RBP, ("r", RAX))
        at_start(R10)
        at_start(R11)
        frm(RBX)                                        # (rbx is the word until here: the fields first)

        def body(nm=nm):
            movq_xm(0, ("m", R11, 0))
            movq_xm(1, ("m", RBP, 0))
            A.mov_r_rm(RCX, ("r", RBX), 0)
            A.call("fph_%d" % FP_ID[nm])
            movq_mx(("m", R10, 0), 0)
        loop(body, (R10, R11, RBP) if src == "v" else (R10, R11))
        done()
    # ---- vfredosum.vs: vd[0] = vs1[0] + vs2[0] + ... in order (vl 0 writes nothing)
    A.label("rv_1_3")
    executes((20,))
    A.alu_rm_imm("cmp", ("abs", "vstart", 0), 0)
    A.jcc("ne", "rv_ill")
    group(R10, 7)
    group(R11, 20)
    group(RBP, 15)
    frm(RBX)
    none_ = lab()
    A.alu_rm_imm("cmp", ("abs", "vl", 0), 0)
    A.jcc("e", none_)
    movq_xm(0, ("m", RBP, 0))

    def red():
        movq_xm(1, ("m", R11, 0))
        A.mov_r_rm(RCX, ("r", RBX), 0)
        A.call("fph_%d" % FP_ID["fadd.d"])
    loop(red, (R11,))
    movq_mx(("m", R10, 0), 0)
    done()
    A.label(none_)
    done(False)
    # ---- vsetvli / vsetivli / vsetvl: r8 = the AVL, r9 = its kind (0 a number, 1 VLMAX, 2 keep vl), rcx = vtype
    A.label("rv_set")
    vli, ivli, vl_, rule, have = lab(), lab(), lab(), lab(), lab()
    fld(RDX, 30, 3)
    cmp32(RDX, 1)
    A.jcc("be", vli)
    cmp32(RDX, 3)
    A.jcc("e", ivli)
    fld(RDX, 25, 0x7F)
    cmp32(RDX, 0x40)
    A.jcc("ne", "rv_ill")                               # the reserved vsetvl space: illegal under any vtype
    A.mov_r_rm(RCX, ("r", RDI))
    A.jmp(rule)
    A.label(vli)
    fld(RCX, 20, 0x7FF)
    A.jmp(rule)
    A.label(ivli)
    fld(RCX, 20, 0x3FF)
    fld(R8, 15, 31)
    A.alu_r_rm("xor", R9, ("r", R9), 0)
    A.jmp(have)
    A.label(rule)                                       # rs1 != x0: x[rs1]; else rd != x0: VLMAX; else keep vl
    A.mov_r_rm(R8, ("r", RSI))
    A.alu_r_rm("xor", R9, ("r", R9), 0)
    fld(RDX, 15, 31)
    A.jcc("ne", have)
    A.mov_rm_imm32(("r", R9), 1, 0)
    fld(RDX, 7, 31)
    A.jcc("ne", have)
    A.mov_rm_imm32(("r", R9), 2, 0)
    A.label(have)
    ill_ = lab()
    A.mov_r_rm(RAX, ("r", RCX))
    A.shift_imm("shr", ("r", RAX), 8)
    A.jcc("ne", ill_)                                   # a bit above 7
    A.mov_r_rm(R10, ("r", RCX), 0)
    A.alu_rm_imm("and", ("r", R10), 7, 0)               # vlmul
    cmp32(R10, 4)
    A.jcc("e", ill_)
    A.mov_r_rm(R11, ("r", RCX), 0)
    A.shift_imm("shr", ("r", R11), 3, 0)
    A.alu_rm_imm("and", ("r", R11), 7, 0)               # vsew
    cmp32(R11, 3)
    A.jcc("a", ill_)
    A.mov_r_rm(RDX, ("r", RCX))                         # rdx = vtype (rcx is the shift count from here)
    frac, sh = lab(), lab()
    A.mov_rm_imm32(("r", RAX), 128, 0)
    A.mov_r_rm(RCX, ("r", R10), 0)
    cmp32(R10, 4)
    A.jcc("a", frac)
    A.shift_cl("shl", ("r", RAX))
    A.jmp(sh)
    A.label(frac)
    A.mov_rm_imm32(("r", RCX), 8, 0)
    A.alu_r_rm("sub", RCX, ("r", R10), 0)
    A.shift_cl("shr", ("r", RAX))
    A.label(sh)
    A.mov_r_rm(RCX, ("r", R11), 0)
    A.alu_rm_imm("add", ("r", RCX), 3, 0)
    A.shift_cl("shr", ("r", RAX))                       # rax = VLMAX
    A.jcc("e", ill_)
    keep, mx, got = lab(), lab(), lab()
    cmp32(R9, 1)
    A.jcc("e", got)                                     # VLMAX
    A.jcc("a", keep)
    A.alu_r_rm("cmp", R8, ("r", RAX))
    A.jcc("ae", got)
    A.mov_r_rm(RAX, ("r", R8))
    A.jmp(got)
    A.label(keep)
    A.mov_r_rm(R8, ("abs", "vl", 0))
    A.alu_r_rm("cmp", R8, ("r", RAX))
    A.jcc("ae", got)
    A.mov_r_rm(RAX, ("r", R8))
    A.label(got)
    A.mov_rm_r(("abs", "vl", 0), RAX)
    A.mov_r_imm64(RCX, VILL)
    A.alu_r_rm("or", RDX, ("r", RCX))
    A.mov_rm_r(("abs", "vt", 0), RDX)
    done_set = lab()
    A.jmp(done_set)
    A.label(ill_)                                       # vill: vl = 0, x[rd] = 0, no trap
    A.alu_r_rm("xor", RAX, ("r", RAX), 0)
    A.mov_rm_r(("abs", "vl", 0), RAX)
    A.mov_rm_r(("abs", "vt", 0), RAX)
    A.label(done_set)
    A.mov_rm_imm32(("abs", "vsen", 0), 3, 0)
    A.mov_rm_imm32(("abs", "vstart", 0), 0)
    A.jmp("rv_ret")
    # ---- vle64.v / vse64.v, unit-stride or strided (LOAD-FP / STORE-FP with a vector width)
    A.label("rv_mem")
    werr, whole, merr = lab(), lab(), lab()
    fld(RCX, 25, 1)
    A.jcc("e", merr)                                    # masked
    fld(RCX, 26, 3)                                     # mop
    unit, strided, fields = lab(), lab(), lab()
    A.jcc("e", unit)
    cmp32(RCX, 2)
    A.jcc("e", strided)
    A.jmp(merr)
    A.label(unit)
    fld(RCX, 20, 31)
    A.jcc("ne", merr)
    A.mov_rm_imm32(("r", RDI), 8)                       # the step
    A.jmp(fields)
    A.label(strided)                                    # rdi = x[rs2], the step
    A.label(fields)
    fld(RCX, 28, 15)                                    # mew, nf
    A.jcc("ne", merr)
    fld(RCX, 12, 7)
    cmp32(RCX, 7)
    A.jcc("ne", merr)
    executes((7,))
    group(R10, 7)
    at_start(R10)
    fld(RDX, 0, 0x7F)
    load_ = lab()
    cmp32(RDX, 0x07)
    A.jcc("e", load_)
    for kind in "sl":
        if kind == "l":
            A.label(load_)

        def body(kind=kind):
            A.mov_r_rm(RAX, ("r", RDI))
            A.imul_r_rm(RAX, ("r", R8))
            A.alu_r_rm("add", RAX, ("r", RSI))
            if kind == "l":
                A.lea(RDX, ("m", RAX, -IMAGE_BASE))
                A.alu_r_rm("cmp", RDX, ("abs", "lim_l8", 0))
                A.jcc("ae", "rv_bad")
                A.mov_r_rm(RCX, ("m", RAX, 0))
                A.mov_rm_r(("m", R10, 0), RCX)
            else:
                A.mov_r_rm(RDX, ("r", RAX))
                A.alu_r_rm("sub", RDX, ("abs", "code_end", 0))
                A.alu_r_rm("cmp", RDX, ("abs", "lim_s8", 0))
                A.jcc("ae", "rv_bad")
                A.mov_r_rm(RCX, ("m", R10, 0))
                if wm:
                    wm_x86(A, RAX, 0, 8, lab())
                A.mov_rm_r(("m", RAX, 0), RCX)
        loop(body, (R10,))
        done(kind == "l")
    A.label(merr)                                       # a whole-register access is refused even under vill
    fld(RCX, 26, 3)
    A.jcc("ne", werr)
    fld(RCX, 20, 31)
    cmp32(RCX, 8)
    A.jcc("e", "rv_bad")
    A.label(werr)
    A.jmp("rv_err")
    # ---- out: back to the lowering, or (the call's frame dropped) to the runtime's halts
    A.label("rv_ret")
    for r in reversed(SAVE):
        A.pop(r)
    A.ret()
    for nm, to in (("rv_ill", "rt_illegal"), ("rv_bad", "rt_bad_insn")):
        A.label(nm)
        for r in reversed(SAVE):
            A.pop(r)
        A.alu_rm_imm("add", ("r", RSP), 8)
        A.jmp(to)


def runtime_fp(A, soft_all=False, mac=False, no_fma3=False):
    """The F / D helpers. One entry per instruction, fph_<id> (FP_ID), called from the lowering with
         xmm0 xmm1 xmm2 = the raw 64-bit f sources (frs1 frs2 frs3)    rax = the integer source
         ecx = the rounding-mode field (0..7, 7 = DYN)   (the fault site is armed when it is 5..7)
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
    A.mov_r_imm64(RCX, IMAGE_BASE)
    A.alu_r_rm("add", RAX, ("r", RCX))
    A.call("rt_putu")
    A.call("fs_m2")
    A.mov_rm_imm32(("r", RDI), 1)
    A.jmp("rt_exit")
    for nm_, txt in (("fs_m1", L1), ("fs_m2", L2)):
        A.label(nm_)
        A.lea(RSI, ("abs", nm_ + "_s", 0))
        A.mov_rm_imm32(("r", RDX), len(txt))
        A.jmp("rt_ewr")                        # stderr, or a thread's halt held back (runtime()'s threads block)
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
    ok = lab()                             # (yantra's rm_reserved: static 5 / 6, or DYN with frm 5..7; cause 2)
    cmp32(RCX, 7)
    A.jcc("ne", ok)
    A.mov_r_rm(RCX, ("abs", "fcsr", 0), 0)
    A.shift_imm("shr", ("r", RCX), 5, 0)
    A.alu_rm_imm("and", ("r", RCX), 7, 0)
    A.label(ok)
    cmp32(RCX, 4)
    A.jcc("a", "rt_illegal")
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
# ELF writer (EM_AARCH64). Lowering: LOWERING-x86-64.md's semantics, with these A64 forms:
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
                  ("steps", 8), ("steps0", 8), ("fin_val", 8), ("lr_cold", 8), ("lr_slow", 8), ("ev_slot", 8), ("rec_fd", 8),
                  ("rep_fd", 8), ("wt_tmp", 8), ("resv", 8), ("win_path", 8), ("win_buf", 8),
                  ("win_st", 8), ("win_sv", 8 * 16), ("win_param", 8), ("pwv", 8 * PW_SLOTS),
                  ("pwlr", 8 * PW_LR), ("spn", SPN_SIZE), ("spn_sv", 8 * 11), ("sk", SK_SIZE),
                  ("sk_sv", 8 * 20),
                  # SASTHRDS threads (runtime_threads_a64); scalars here, below `extra`, so every
                  # target has them at one offset
                  ("th_n", 8), ("th_cur", 8), ("th_ctx", 8), ("th_id", 8), ("th_stk", 8), ("th_slim", 8),
                  ("th_bos", 8), ("th_bosk", 8), ("th_cap", 8), ("th_capn", 8), ("th_failk", 8),
                  ("th_failv", 8), ("th_lastv", 8), ("th_rc", 8), ("th_pre", 8), ("th_ended", 8),
                  ("th_s0", 8), ("th_s1", 8), ("th_lr", 8), ("th_len", 8), ("th_e1", 8), ("th_e2", 8)] + extra + [
                  ("num", 32), ("statbuf", 144), ("outbuf", 65536), ("pwb", PWB_SIZE), ("sk_line", SK_LINE)]:
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
               # spawn (agent/rtc-spawn), appended; CloseHandle and VirtualFree are above
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

    def subs_i(self, rd, rn, imm):
        """rd = rn - imm (0 <= imm < 4096), flags set"""
        assert 0 <= imm < 4096, imm
        self.w(0xF1000000 | imm << 10 | rn << 5 | rd)

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


# FORK-JOIN on AArch64 (FORKJOIN.md): the per-thread area after the frame and the F/D data, at sp + fjoff;
# x18 (no guest register on Linux) addresses it. fj_hsave holds host x0..x30 at the marker (by host number).
FJ64, _o = {}, 0
for _n, _sz in [("fj_worker", 8), ("fj_i", 8), ("fj_hi", 8), ("fj_bout", 8), ("fj_bbody", 8), ("fj_sp", 8),
                ("fj_kind", 8), ("fj_code", 8), ("fj_caplen", 8), ("fj_capover", 8), ("fj_n", 8), ("fj_k", 8), ("fj_t", 8),
                ("fj_lo", 8), ("fj_hiall", 8), ("fj_q", 8), ("fj_r", 8), ("fj_out", 8), ("fj_body", 8),
                ("fj_depth", 8), ("fj_area", 8), ("fj_pool", 8), ("fj_c", 8), ("fj_stp", 8), ("fj_ret", 8), ("fj_sv", 40),
                ("fj_pmax", 8), ("fj_regions", 8), ("fj_spawned", 8), ("fj_att", 8), ("fj_ec", 8), ("fj_lr", 8), ("fj_lr2", 8), ("fj_abuf", 256),
                ("fj_hsave", 8 * 32), ("fj_tids", 8 * FJ_MAX), ("fj_cap", FJ_CAP)]:
    FJ64[_n] = _o
    _o += _sz
FJ64_SIZE = _o
LOG_NOT7 = 1 << 12 | 61 << 6 | 60        # ~7
LOG_31 = 1 << 12 | 0 << 6 | 4             # 31


def fj_base_a64(A, fj):
    """x18 = sp + fjoff: this thread's fork-join area"""
    A.add_i12(18, SPR, fj["fjoff"] >> 12)
    A.add_i(18, 18, fj["fjoff"] & 0xFFF)


def runtime_fj_a64(A, base, fj, msg, FR, MAP):
    """FORK-JOIN on AArch64 Linux: runtime_fj's design (see there) with A64's model. A worker's state
    is its FRAME: sp points at a copy of the main thread's frame, F/D data and fork-join area (raw
    clone() with the copy as the child's stack: the frame is never pushed). Its host calls go through
    rt_fj_sw (sysc), its body returns to the sentinel through rt_bad_target. Join: CLEARTID + futex,
    then dmb ish."""
    FO = fj["fjoff"]
    DS = FO + FJ64["fj_cap"]                 # what a worker's block copies (but outbuf)
    BLK = (FO + FJ64_SIZE + 0xFFF) & ~0xFFF
    sb, sp0, ext_end = fj["sb"], fj["sp0"], fj["ext_end"]
    flags = 0x100 | 0x200 | 0x400 | 0x800 | 0x10000 | 0x40000 | 0x200000   # VM FS FILES SIGHAND THREAD SYSVSEM CHILD_CLEARTID
    X = 18
    F = FJ64
    saved = [h for h in range(31) if h not in (15, 16, 17, 18)]

    def ld(r, f):
        A.mem("ldr", r, X, F[f])

    def st(r, f):
        A.mem("str", r, X, F[f])

    def gload(r, n):
        if n in MAP:
            A.mem("ldr", r, X, F["fj_hsave"] + 8 * MAP[n])
        else:
            A.mem("ldr", r, SPR, FR["regs"] + 8 * n)

    def gstore(n, r):
        if n in MAP:
            A.mem("str", r, X, F["fj_hsave"] + 8 * MAP[n])
        else:
            A.mem("str", r, SPR, FR["regs"] + 8 * n)

    def addbig(rd, rn, v):                     # rd = rn + v, v < 2^24
        A.add_i12(rd, rn, v >> 12)
        A.add_i(rd, rd, v & 0xFFF)

    def dmb():
        A.w(0xD5033BBF)                        # dmb ish

    def copy(dst, src, n, tag):                # n octets (a multiple of 8), x9 x10 x11
        A.movz(9, 0)
        A.movi(11, n)
        A.label(tag)
        A.cmp(9, 11)
        A.bcond(HS, tag + "d")
        A.memr("ldr", 10, src, 9)
        A.memr("str", 10, dst, 9)
        A.add_i(9, 9, 8)
        A.b_(tag)
        A.label(tag + "d")

    def block(c):                              # x20 = the block of chunk c, x21 = its fork-join area
        A.movi(9, BLK)
        ld(20, "fj_pool")
        A.madd(20, c, 9, 20)
        addbig(21, 20, FO)

    # ---------------- the marker, main thread (bl'd; x30 = the return)
    A.label("rt_fj")
    fj_base_a64(A, fj)
    ld(16, "fj_worker")
    A.cbnz(16, "rt_fj_r0")                     # in a worker (a nested region): serial
    for h in saved:
        A.mem("str", h, X, F["fj_hsave"] + 8 * h)
    gload(9, 10)
    A.cmp_i(9, 1)
    A.bcond(NE, "rt_fj_out")
    ld(10, "fj_n")
    A.cbz(10, "rt_fj_out")
    gload(11, 11)
    gload(12, 12)
    A.cmp(11, 12)
    A.bcond(HS, "rt_fj_out")                   # empty, or lo above hi: the serial loop runs nothing
    A.rrr("sub", 13, 12, 11)                   # count
    A.cmp(10, 13)
    A.bcond(LS, "rt_fj_t")
    A.mov(10, 13)
    A.label("rt_fj_t")
    # the small-region threshold: at most count // K threads, and serial below two
    ld(9, "fj_k")
    A.cbz(9, "rt_fj_tk")
    A.dp2("udiv", 14, 13, 9)
    A.cmp(14, 10)
    A.bcond(HS, "rt_fj_tk2")
    A.mov(10, 14)
    A.label("rt_fj_tk2")
    A.cmp_i(10, 2)
    A.bcond(LO, "rt_fj_out")
    A.label("rt_fj_tk")
    st(10, "fj_t")
    st(11, "fj_lo")
    st(12, "fj_hiall")
    A.dp2("udiv", 14, 13, 10)
    A.msub(9, 14, 10, 13)
    st(14, "fj_q")
    st(9, "fj_r")
    gload(9, 13)
    st(9, "fj_out")
    gload(9, 14)
    st(9, "fj_body")
    # the caller's sp in (SB + 2056, sp0]: depth = sp - SB - 2056
    gload(9, 2)
    A.movi(10, sp0)
    A.cmp(9, 10)
    A.bcond(HI, "rt_fj_out")
    A.movi(10, sb + 8 + 2048)
    A.cmp(9, 10)
    A.bcond(LS, "rt_fj_out")
    A.rrr("sub", 9, 9, 10)
    st(9, "fj_depth")
    # the slices: T x FJ_SLICE at the top of RAM, above the image's extent, all zero
    ld(10, "fj_t")
    A.movi(11, FJ_SLICE)
    A.madd(9, 10, 11, ZR)
    A.movi(12, base)
    A.mem("ldr", 13, SPR, FR["ram"])
    A.rrr("add", 12, 12, 13)
    A.rrr("sub", 12, 12, 9)
    A.and_l(12, 12, LOG_NOT4095)
    A.movi(13, ext_end)
    A.cmp(12, 13)
    A.bcond(LO, "rt_fj_out")
    st(12, "fj_area")
    A.label("rt_fj_zs")
    A.cbz(9, "rt_fj_zok")
    A.mem("ldr", 13, 12, 0)
    A.cbnz(13, "rt_fj_out")                    # something lives there: serial
    A.add_i(12, 12, 8)
    A.add_i(9, 9, -8)
    A.b_("rt_fj_zs")
    A.label("rt_fj_zok")
    # the pool of worker blocks, mapped once
    ld(9, "fj_pool")
    A.cbnz(9, "rt_fj_have")
    A.movz(0, 0)
    A.movi(1, FJ_MAX * BLK)
    A.movz(2, 3)
    A.movi(3, 0x22)
    A.movn(4, 0)
    A.movz(5, 0)
    A.movz(8, 222)
    A.svc()
    A.movn(9, 4095)                            # -4096
    A.cmp(0, 9)
    A.bcond(HI, "rt_fj_out")
    st(0, "fj_pool")
    A.label("rt_fj_have")
    A.bl("rt_flush")                           # the main thread's output so far goes first
    fj_base_a64(A, fj)
    A.mem("ldr", 9, SPR, FR["steps"])          # the budget at the fork: every worker starts with it
    st(9, "fj_stp")
    # ---------------- spawn chunk c = 0 .. T-1
    st(ZR, "fj_c")
    A.label("rt_fj_sp")
    ld(19, "fj_c")
    ld(9, "fj_t")
    A.cmp(19, 9)
    A.bcond(HS, "rt_fj_join")
    block(19)
    copy(20, SPR, FR["outbuf"], "rt_fj_cp1")   # the frame but its UART buffer, then the F/D data and the area
    addbig(12, SPR, A64_FRAME_SIZE)
    addbig(13, 20, A64_FRAME_SIZE)
    copy(13, 12, DS - A64_FRAME_SIZE, "rt_fj_cp2")
    A.movz(9, 1)
    A.mem("str", 9, 21, F["fj_worker"])
    for f in ("fj_kind", "fj_code", "fj_caplen", "fj_capover"):
        A.mem("str", ZR, 21, F[f])
    A.mem("str", ZR, 20, FR["outlen"])
    # the chunk: lo + c q + min(c, r), q + (c < r) indices
    ld(9, "fj_q")
    ld(10, "fj_lo")
    A.madd(11, 19, 9, 10)
    ld(12, "fj_r")
    A.cmp(19, 12)
    A.csel(13, 19, 12, LO)
    A.rrr("add", 11, 11, 13)
    A.mem("str", 11, 21, F["fj_i"])
    A.rrr("add", 11, 11, 9)
    A.cmp(19, 12)
    A.cset(13, LO)
    A.rrr("add", 11, 11, 13)
    A.mem("str", 11, 21, F["fj_hi"])
    ld(9, "fj_out")
    A.mem("str", 9, 21, F["fj_bout"])
    ld(9, "fj_body")
    A.mem("str", 9, 21, F["fj_bbody"])
    # its slice: the body starts at S + FJ_SLICE - 4096, FAST window [start - depth, start + 8]
    A.movi(9, FJ_SLICE)
    ld(10, "fj_area")
    A.madd(11, 19, 9, 10)
    A.movi(9, FJ_SLICE - 4096)
    A.rrr("add", 11, 11, 9)
    A.mem("str", 11, 21, F["fj_sp"])
    ld(10, "fj_depth")
    A.rrr("sub", 12, 11, 10)
    A.add_i(13, 10, 9)
    A.stp(12, 13, 20, FR["sp_win"])
    # its CLEARTID word = 1; clone(flags, the block as its stack, 0, 0, &word)
    A.add_i(22, X, F["fj_tids"])
    A.rrr("add", 22, 22, 19, 1, 0, 3)
    A.movz(9, 1)
    A.mem("str", 9, 22, 0)
    A.movi(0, flags)
    A.mov(1, 20)
    A.movz(2, 0)
    A.movz(3, 0)
    A.mov(4, 22)
    A.movz(8, 220)
    A.svc()
    A.cbz(0, "rt_fj_worker")
    A.cmp_i(0, 0)
    A.bcond(LT, "rt_fj_nospawn")
    fj_base_a64(A, fj)
    ld(9, "fj_spawned")                        # S1 attestation: one chunk really spawned
    A.add_i(9, 9, 1)
    st(9, "fj_spawned")
    ld(9, "fj_c")
    A.add_i(9, 9, 1)
    st(9, "fj_c")
    A.b_("rt_fj_sp")
    A.label("rt_fj_nospawn")
    fj_base_a64(A, fj)
    ld(9, "fj_c")
    A.cbz(9, "rt_fj_out")                      # no thread at all: serial
    A.b_("rt_fj_fall")
    # ---------------- join chunk c = 0 .. T-1, in order
    A.label("rt_fj_join")
    st(ZR, "fj_c")
    A.label("rt_fj_jl")
    ld(19, "fj_c")
    ld(9, "fj_t")
    A.cmp(19, 9)
    A.bcond(HS, "rt_fj_all")
    A.add_i(22, X, F["fj_tids"])
    A.rrr("add", 22, 22, 19, 1, 0, 3)
    A.label("rt_fj_jw")
    A.mem("ldrw", 2, 22, 0)
    A.cbz(2, "rt_fj_jd", 0)
    A.mov(0, 22)
    A.movz(1, 0)                               # FUTEX_WAIT (shared: CLEARTID wakes shared)
    A.movz(3, 0)
    A.movz(8, 98)
    A.svc()
    A.b_("rt_fj_jw")
    A.label("rt_fj_jd")
    dmb()
    block(19)
    A.mem("ldr", 9, 21, F["fj_capover"])
    A.cbnz(9, "rt_fj_fall")
    A.mem("ldr", 9, 21, F["fj_kind"])
    A.cbz(9, "rt_fj_fall")
    A.cmp_i(9, 3)
    A.bcond(HS, "rt_fj_fall")
    # the words it ran are the serial run's: off the main thread's budget; a borrow means the limit is
    # in this region, so the serial run (yantra) names the exact step
    ld(9, "fj_stp")
    A.mem("ldr", 10, 20, FR["steps"])
    A.rrr("sub", 9, 9, 10)
    A.mem("ldr", 10, SPR, FR["steps"])
    A.rrr("subs", 10, 10, 9)
    A.bcond(LO, "rt_fj_fall")
    A.mem("str", 10, SPR, FR["steps"])
    # replay its records: (fd, n, n octets padded to 8)
    A.add_i(23, 21, F["fj_cap"] & 0xFFF) if F["fj_cap"] < 4096 else addbig(23, 21, F["fj_cap"])
    A.mem("ldr", 9, 21, F["fj_caplen"])
    A.rrr("add", 24, 23, 9)
    A.label("rt_fj_rl")
    A.cmp(23, 24)
    A.bcond(HS, "rt_fj_rd")
    A.mem("ldr", 25, 23, 0)
    A.mem("ldr", 26, 23, 8)
    A.add_i(27, 23, 16)
    A.add_i(9, 26, 16 + 7)
    A.and_l(9, 9, LOG_NOT7)
    A.rrr("add", 23, 23, 9)
    A.label("rt_fj_rw")
    A.cbz(26, "rt_fj_rl")
    A.mov(0, 25)
    A.mov(1, 27)
    A.mov(2, 26)
    A.movz(8, 64)
    A.svc()
    A.cmp_i(0, 0)
    A.bcond(LE, "rt_fj_rl")
    A.rrr("add", 27, 27, 0)
    A.rrr("sub", 26, 26, 0)
    A.b_("rt_fj_rw")
    A.label("rt_fj_rd")
    A.mem("ldr", 9, 21, F["fj_kind"])
    A.cmp_i(9, 2)
    A.bcond(NE, "rt_fj_k1")
    A.mem("ldr", 0, 21, F["fj_code"])          # it halted: the run ends as the serial run ends
    A.movz(8, 94)
    A.svc()
    A.label("rt_fj_k1")
    fj_base_a64(A, fj)
    if True:
        # fflags are sticky: OR the chunk's into the main thread's; a changed frm or FS is not merged
        FPO = A64_FRAME_SIZE
        addbig(12, SPR, FPO)
        addbig(13, 20, FPO)
        A.mem("ldr", 9, 13, fp_a64_off("fcsr"))
        A.mem("ldr", 10, 12, fp_a64_off("fcsr"))
        A.lsr_i(11, 9, 5)
        A.lsr_i(14, 10, 5)
        A.cmp(11, 14)
        A.bcond(NE, "rt_fj_fall")
        A.and_l(9, 9, LOG_31)
        A.rrr("orr", 10, 10, 9)
        A.mem("str", 10, 12, fp_a64_off("fcsr"))
        A.mem("ldr", 9, 13, fp_a64_off("fsen"))
        A.mem("ldr", 10, 12, fp_a64_off("fsen"))
        A.cmp(9, 10)
        A.bcond(NE, "rt_fj_fall")
    ld(9, "fj_c")
    A.add_i(9, 9, 1)
    st(9, "fj_c")
    A.b_("rt_fj_jl")
    A.label("rt_fj_all")
    # S1 attestation, only here (after the join; the fallback never gets here): fj_pmax = max(fj_pmax, T), K += 1
    ld(9, "fj_regions")
    A.add_i(9, 9, 1)
    st(9, "fj_regions")
    ld(9, "fj_t")
    ld(10, "fj_pmax")
    A.cmp(9, 10)
    A.bcond(LS, "rt_fj_pm")
    st(9, "fj_pmax")
    A.label("rt_fj_pm")
    # every chunk done: the slices back to zero, a1 = hi
    ld(12, "fj_area")
    ld(10, "fj_t")
    A.movi(11, FJ_SLICE)
    A.madd(9, 10, 11, ZR)
    A.label("rt_fj_zz")
    A.cbz(9, "rt_fj_zd")
    A.mem("str", ZR, 12, 0)
    A.add_i(12, 12, 8)
    A.add_i(9, 9, -8)
    A.b_("rt_fj_zz")
    A.label("rt_fj_zd")
    ld(9, "fj_hiall")
    gstore(11, 9)
    A.label("rt_fj_out")
    fj_base_a64(A, fj)
    for h in saved:
        A.mem("ldr", h, X, F["fj_hsave"] + 8 * h)
    A.label("rt_fj_r0")
    A.ret()
    # ---- S1: the attestation line, entered by bl from the exit funnel (x18 = the area, the exit code kept by the caller)
    A.label("rt_fja_cpy")                      # x1 = dst, x2 = src, x3 = octets
    A.cbz(3, "rt_fja_c9")
    A.mem("ldrb", 4, 2, 0)
    A.mem("strb", 4, 1, 0)
    A.add_i(2, 2, 1)
    A.add_i(1, 1, 1)
    A.add_i(3, 3, -1)
    A.b_("rt_fja_cpy")
    A.label("rt_fja_c9")
    A.ret()
    A.label("rt_fja_dec")                      # x0 as decimal at x1, x1 advanced
    A.add_i(5, 18, F["fj_abuf"] + 224)
    A.mov(2, 5)
    A.movz(7, 10)
    A.label("rt_fja_d1")
    A.dp2("udiv", 6, 0, 7)
    A.msub(8, 6, 7, 0)
    A.add_i(8, 8, 48)
    A.add_i(5, 5, -1)
    A.mem("strb", 8, 5, 0)
    A.mov(0, 6)
    A.cbnz(0, "rt_fja_d1")
    A.rrr("sub", 3, 2, 5)
    A.mov(2, 5)
    st(30, "fj_lr2")
    A.bl("rt_fja_cpy")
    ld(30, "fj_lr2")
    A.ret()
    A.label("rt_fja_rep")
    st(30, "fj_lr")
    A.add_i(1, 18, F["fj_abuf"])
    for k, (ln, name) in enumerate([(19, "fj_n"), (10, "fj_pmax"), (3, "fj_q"), (3, "fj_regions"), (4, "fj_k"), (9, "fj_spawned")]):
        A.movl(2, "fja_%d" % k)
        A.movz(3, ln)
        A.bl("rt_fja_cpy")
        ld(0, name)
        A.bl("rt_fja_dec")
    A.movl(2, "fja_6")
    A.movz(3, 1)
    A.bl("rt_fja_cpy")
    A.add_i(9, 18, F["fj_abuf"])
    A.rrr("sub", 12, 1, 9)                     # the line's length
    A.movn(0, 99)                              # AT_FDCWD
    ld(1, "fj_att")
    A.movi(2, 0x241)                           # O_WRONLY | O_CREAT | O_TRUNC
    A.movi(3, 0o644)
    A.movz(8, 56)
    A.svc()
    A.cmp_i(0, 0)
    A.bcond(LT, "rt_fja_done")
    A.mov(13, 0)
    A.add_i(1, 18, F["fj_abuf"])
    A.mov(2, 12)
    A.movz(8, 64)
    A.svc()
    A.mov(0, 13)
    A.movz(8, 57)
    A.svc()
    A.label("rt_fja_done")
    ld(30, "fj_lr")
    A.ret()
    A.label("rt_fj_fall")                      # what the threads did is not the serial run's: yantra
    A.bl("m_fjfall")
    A.movz(0, 1)
    A.movz(8, 94)
    A.svc()
    msg("m_fjfall", b"native: not in milestone 1 (a fork-join worker made a host call, left its stack slice, "
                    b"overflowed its capture or changed the rounding mode; run it under yantra-run)\n")

    # ---------------- a worker (sp = its block)
    A.label("rt_fj_worker")
    fj_base_a64(A, fj)
    for h in saved:
        A.mem("ldr", h, X, F["fj_hsave"] + 8 * h)
    A.label("rt_fj_next")
    fj_base_a64(A, fj)
    ld(T0, "fj_i")
    ld(T1, "fj_hi")
    A.cmp(T0, T1)
    A.bcond(HS, "rt_fj_fin")
    A.mov(MAP[10], T0)
    ld(MAP[11], "fj_bout")
    A.movi(MAP[1], FJ_SENTINEL)
    ld(MAP[2], "fj_sp")
    ld(T0, "fj_bbody")
    A.b_("rt_indirect")
    A.label("rt_fj_ret")                       # rt_bad_target: the body returned
    fj_base_a64(A, fj)
    ld(T0, "fj_i")
    A.add_i(T0, T0, 1)
    st(T0, "fj_i")
    A.b_("rt_fj_next")
    A.label("rt_fj_fin")
    A.bl("rt_flush")
    fj_base_a64(A, fj)
    A.movz(T0, 1)
    st(T0, "fj_kind")
    A.b_("rt_fj_wdone")
    A.label("rt_fj_stepw")                     # the step limit is met in a worker: the serial run (yantra) speaks
    fj_base_a64(A, fj)
    A.movz(T0, 3)
    st(T0, "fj_kind")
    A.b_("rt_fj_wdone")
    A.label("rt_fj_stack")                     # XF_: its sp left the FAST window of its slice
    fj_base_a64(A, fj)
    A.movz(T0, 4)
    st(T0, "fj_kind")
    A.label("rt_fj_wdone")
    dmb()
    A.movz(0, 0)
    A.movz(8, 93)                              # exit (this thread); the kernel clears its word and wakes
    A.svc()

    # ---------------- a worker's host call (sysc): x8 = the number, x18 = where to come back.
    # A write is captured (x0 = its length, nothing else changes), exit recorded, anything else refused.
    A.label("rt_fj_sw")
    A.mem("str", 18, SPR, FR["wt_tmp"])
    fj_base_a64(A, fj)
    A.stp(T0, T1, X, F["fj_sv"])
    A.stp(T2, 9, X, F["fj_sv"] + 16)
    A.mem("str", 10, X, F["fj_sv"] + 32)
    A.cmp_i(8, 64)
    A.bcond(EQ, "rt_fj_ww")
    A.cmp_i(8, 94)
    A.bcond(EQ, "rt_fj_we")
    A.cmp_i(8, 93)
    A.bcond(EQ, "rt_fj_we")
    A.movz(T0, 3)
    st(T0, "fj_kind")
    A.b_("rt_fj_wdone")
    A.label("rt_fj_we")
    st(0, "fj_code")
    A.movz(T0, 2)
    st(T0, "fj_kind")
    A.b_("rt_fj_wdone")
    A.label("rt_fj_ww")
    A.add_i(T0, 2, 16 + 7)
    A.and_l(T0, T0, LOG_NOT7)
    ld(T1, "fj_caplen")
    A.rrr("add", T2, T1, T0)
    A.movi(9, FJ_CAP)
    A.cmp(T2, 9)
    A.bcond(HI, "rt_fj_wo")
    st(T2, "fj_caplen")
    addbig(9, X, F["fj_cap"])
    A.rrr("add", 9, 9, T1)
    A.mem("str", 0, 9, 0)
    A.mem("str", 2, 9, 8)
    A.add_i(9, 9, 16)
    A.movz(T0, 0)
    A.label("rt_fj_wc")
    A.cmp(T0, 2)
    A.bcond(HS, "rt_fj_wx")
    A.memr("ldrb", 10, 1, T0)
    A.memr("strb", 10, 9, T0)
    A.add_i(T0, T0, 1)
    A.b_("rt_fj_wc")
    A.label("rt_fj_wo")
    A.movz(T0, 1)
    st(T0, "fj_capover")
    A.label("rt_fj_wx")
    A.mov(0, 2)
    A.ldp(T0, T1, X, F["fj_sv"])
    A.ldp(T2, 9, X, F["fj_sv"] + 16)
    A.mem("ldr", 10, X, F["fj_sv"] + 32)
    A.mem("ldr", 18, SPR, FR["wt_tmp"])
    A.br(18)
    # a store to a device other than the UART, WAIT and FINISHER (files, spawn, ...): not from a worker
    A.label("rt_st_fjdev")
    fj_base_a64(A, fj)
    A.mem("ldr", 18, 18, F["fj_worker"])
    A.cbz(18, "rt_st_win")
    A.movz(T0, 3)
    fj_base_a64(A, fj)
    A.mem("str", T0, X, F["fj_kind"])
    A.b_("rt_fj_wdone")


def wm_a64(A, rn, off):
    """the store high-water note (YANTRA_WATERMARK, `wm`): hw = max(hw, rn + off) at [sp, #-16], the guest's
    ABSOLUTE end address (the report subtracts the base); clobbers T1 T2 and the flags"""
    A.add_i(T1, rn, off)
    A.mem("ldr", T2, SPR, WM_HW)
    A.cmp(T1, T2)
    A.bskip(LS, 2)
    A.mem("str", T1, SPR, WM_HW)


WM_HW = -16          # wm: the frame (sp) starts 16 octets into the bss; hw sits before it, so no frame offset moves
VD_ST, VD_LIM, VD_FD, VD_SV = -24, -32, -40, -48       # vd: 64 octets before the frame; the buffers follow the bss


def translate_a64(img, arg0, mac=False, wm=False, vd=False):
    """ANUVADA_SC=1 (Linux, no wm): slot 0's GPU (scanout_dev.py). Its state block is appended to the frame."""
    global A64_FRAME, A64_FRAME_SIZE
    if os.environ.get("ANUVADA_SC") != "1" or mac or wm or vd:
        return _translate_a64(img, arg0, mac, wm, vd)
    import scanout_dev
    scanp = scanout_dev.build(IMAGE_BASE)
    saved = (A64_FRAME, A64_FRAME_SIZE)
    at = (A64_FRAME_SIZE + 15) & ~15
    A64_FRAME = dict(A64_FRAME, scs=at)
    A64_FRAME_SIZE = at + scanp.size() + 16 * 8
    try:
        return _translate_a64(img, arg0, mac, wm, vd, scanp)
    finally:
        A64_FRAME, A64_FRAME_SIZE = saved


def _translate_a64(img, arg0, mac=False, wm=False, vd=False, scanp=None):
    """wm: as translate(): the store high-water mark is kept and said at exit when YANTRA_WATERMARK is set;
    Linux, no fork-join. Off, the output is what it always was."""
    entry, base, span, extent, code_size, prot_size = parse_elf(img)
    require_base(base)
    wm = wm or vd
    assert not wm or not mac, "the watermark is Linux only"
    nwords = code_size // 4
    words = struct.unpack_from("<%dI" % nwords, span)
    decs = [decode(w) for w in words]
    M = A64_MAP_MAC if mac else A64_MAP
    FR = a64_frame(mac)
    A = A64((WBASE if mac == "win" else TBASE_MAC if mac else TEXT) + 0x1000)
    # FORK-JOIN (FORKJOIN.md): as on x86-64, only with ANUVADA_FORKJOIN=1, on Linux, for an image that
    # calls the marker routine; the per-thread state is the frame itself (a worker's sp is its copy)
    fj = None
    if os.environ.get("ANUVADA_FORKJOIN") == "1" and not mac and not wm and not scanp:
        fj = fj_plan(words, decs, base, entry, nwords)

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
    vec_used = any(d and d[0] == "vec" for d in decs)
    fp_used = fp_used or vec_used
    amo_used = any(d and d[0] == "amo" for d in decs)
    if vec_used or amo_used:
        fj = None                             # the vector state and the reservation are not a thread's
    sst_full, csr_data = csr_plan(decs)
    fs_dirty = sst_full and fp_used
    csrh_used = any(d is None and yantra_static(w) == 5 for d, w in zip(decs, words))
    if csr_data:
        fj = None                             # nor sstatus / sscratch
    # sst and sscr sit after the F/D and V data (fs_addr's base + CSRO)
    CSRO = (FP_A64_SIZE if fp_used else 0) + (VEC_A64_SIZE if vec_used else 0)
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
    rem = step_plan(decs, leader)                 # the step ceiling (see the x86-64 translate)
    stubs, stubs1 = set(), set()

    def charge(k):
        """a FAST block's words off the budget; borrow (LO): the limit falls inside the block (S_k: SAFE, word by word)"""
        if vd:                                  # vd mode counts its own words (vd_st, vd_lim)
            return
        A.mem("ldr", T1, SPR, FR["steps"])
        if rem[k] < 4096:
            A.subs_i(T1, T1, rem[k])
        else:
            A.movi(T2, rem[k])
            A.rrr("subs", T1, T1, T2)
        A.mem("str", T1, SPR, FR["steps"])
        A.bskip(HS, 2)                          # the stub lies after all code: b.cond reaches +-1 MiB, b +-128 MiB
        A.b_("S_%x" % (base + 4 * k))
        stubs.add(k)

    def charge1(k):
        """the SAFE region: one word off the budget before it runs; borrow: the limit is met at this word"""
        if vd:
            return
        A.mem("ldr", T1, SPR, FR["steps"])
        A.subs_i(T1, T1, 1)
        A.mem("str", T1, SPR, FR["steps"])
        A.bskip(HS, 2)                          # the stub lies after all code: b.cond reaches +-1 MiB, b +-128 MiB
        A.b_("T_%x" % (base + 4 * k))
        stubs1.add(k)

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

        if vd:
            # yantra's clock (L33): one tick per instruction begun, the step ceiling checked with it
            A.mem("ldr", T1, SPR, VD_ST)
            A.add_i(T1, T1, 1)
            A.mem("str", T1, SPR, VD_ST)
            A.mem("ldr", T2, SPR, VD_LIM)
            A.cmp(T1, T2)
            A.bskip(LS, 4)                    # over movz, movk, b
            A.movz(T0, pc & 0xFFFF)
            A.movk(T0, pc >> 16 & 0xFFFF, 1)
            A.b_("rt_vd_slim")
        d = decs[k]
        if d is None and words[k] == 0x73:
            # ecall from S-mode: an SBI call (see the x86-64 lowering); a7 = 1 putchar(a0), a7 = 0 set_timer, a0 = 0
            a7 = src(17, T0)
            A.cmp_i(a7, 0)
            A.bcond(EQ, "T%s_%x" % (V, pc))
            A.cmp_i(a7, 1)
            A.bcond(NE, "E%s_%x" % (V, pc))
            a0 = src(10, T1)
            if a0 != T1:
                A.mov(T1, a0)
            A.movi(T0, UART)
            A.mem("str", 30, SPR, FR["lr_cold"])
            A.bl("rt_store_slow")
            A.mem("ldr", 30, SPR, FR["lr_cold"])
            A.label("T%s_%x" % (V, pc))
            if 10 in M:
                A.movz(M[10], 0)
            else:
                A.mem("str", ZR, SPR, FR["regs"] + 8 * 10)
            if fast:
                forget_reg(10)
            A.b_("D%s_%x" % (V, pc))
            A.label("E%s_%x" % (V, pc))
            A.mem("str", src(17, T0), SPR, FR["fault_addr"])     # eid
            A.mem("str", src(16, T1), SPR, FR["fault_word"])     # fid
            A.movi(T2, 4 * k)
            A.mem("str", T2, SPR, FR["fault_off"])
            A.b_("rt_ecall")
            A.label("D%s_%x" % (V, pc))
            return
        if d is None:
            kind = yantra_static(words[k])
            if kind == 4:                     # sfence.vma: a no-op in S-mode
                return
            A.movi(T2, 4 * k)
            A.mem("str", T2, SPR, FR["fault_off"])
            A.movi(T2, sx(words[k], 32))
            A.mem("str", T2, SPR, FR["fault_word"])
            A.b_(("rt_bad_insn", "rt_unimpl", "rt_break", "rt_sret", "", "rt_csrh")[kind])
            return
        op, rd, rs1, rs2, imm = d
        if op == "fence":
            return
        FPO, FSEN = a64_frame_size(mac), fp_a64_off("fsen")

        def fs_addr(r):
            A.add_i12(r, SPR, FPO >> 12)
            A.add_i(r, r, FPO & 0xFFF)
        if op == "csrs_sstatus" and sst_full:
            # sstatus, exactly (H8): the source to wt_tmp; x16 = yantra's read; x17 = the new value; rd = x16
            f3, r = imm & 7, imm >> 3
            VSEN = FP_A64_SIZE + vec_a64_off("vsen")
            if f3 < 4:
                r0 = srcn(r, T0)
            else:
                A.movz(T0, r)
                r0 = T0
            A.mem("str", r0, SPR, FR["wt_tmp"])
            fs_addr(T2)
            A.mem("ldr", T0, T2, CSRO)
            if fp_used:                       # FS is fsen's
                A.and_l(T0, T0, logimm(62, 15))
                A.mem("ldr", T1, T2, FSEN)
                A.rrr("orr", T0, T0, T1, 1, 0, 13)
            if vec_used:                      # VS is vsen's
                A.and_l(T0, T0, logimm(62, 11))
                A.mem("ldr", T1, T2, VSEN)
                A.rrr("orr", T0, T0, T1, 1, 0, 9)
            A.lsr_i(T1, T0, 1)                # SD = (FS == 3 or VS == 3) << 63: bit 13 / bit 9 of s & s >> 1
            A.rrr("and", T1, T1, T0)
            A.movi(T2, 0x2200)
            A.rrr("and", T1, T1, T2)
            A.rrr("sub", T1, ZR, T1)          # neg: bit 63 set exactly when one was
            A.lsr_i(T1, T1, 63)
            A.rrr("orr", T0, T0, T1, 1, 0, 63)
            A.w(0xB2000000 | logimm(1, 33) << 10 | T0 << 5 | T0)      # orr x16, x16, #UXL (1 << 33)
            A.mem("ldr", T1, SPR, FR["wt_tmp"])
            if f3 & 3 == 2:
                A.rrr("orr", T1, T0, T1)
            elif f3 & 3 == 3:
                A.w(0x8A200000 | T1 << 16 | T0 << 5 | T1)      # bic x17, x16, x17
            if rd:
                if fast:
                    forget_reg(rd)
                rr = dst(rd)
                if rr != T0:
                    A.mov(rr, T0)
                fin(rd, rr)
            fs_addr(T2)
            A.movi(T0, SSTATUS_MASK)
            A.rrr("and", T0, T0, T1)
            A.mem("str", T0, T2, CSRO)
            if fp_used:
                A.lsr_i(T0, T1, 13)
                A.and_l(T0, T0, LOG_3)
                A.mem("str", T0, T2, FSEN)
            if vec_used:
                A.lsr_i(T0, T1, 9)
                A.and_l(T0, T0, LOG_3)
                A.mem("str", T0, T2, VSEN)
            return
        if op == "csr_sscratch":
            # sscratch (H8): x16 = the source, x17 = the old value, then the new one to sscr
            f3, r = imm & 7, imm >> 3
            if f3 < 4:
                r0 = srcn(r, T0)
                if r0 != T0:
                    A.mov(T0, r0)
            else:
                A.movz(T0, r)
            fs_addr(T2)
            A.mem("ldr", T1, T2, CSRO + 8)
            if f3 & 3 == 2:
                A.rrr("orr", T0, T0, T1)
            elif f3 & 3 == 3:
                A.w(0x8A200000 | T0 << 16 | T1 << 5 | T0)      # bic x16, x17, x16
            A.mem("str", T0, T2, CSRO + 8)
            if rd:
                if fast:
                    forget_reg(rd)
                rr = dst(rd)
                if rr != T1:
                    A.mov(rr, T1)
                fin(rd, rr)
            return
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
            if vec_used:                      # and sstatus.VS (bits 10:9) every vector instruction
                f3, r = imm & 7, imm >> 3
                VSEN = FP_A64_SIZE + vec_a64_off("vsen")
                if f3 in (1, 2, 3) and (f3 == 1 or r):
                    r0 = srcn(r, T0)
                    A.lsr_i(T0, r0, 9)
                    A.and_l(T0, T0, LOG_3)
                    fs_addr(T2)
                    if f3 == 1:
                        A.mem("str", T0, T2, VSEN)
                    else:
                        A.mem("ldr", T1, T2, VSEN)
                        if f3 == 2:
                            A.rrr("orr", T1, T1, T0)
                        else:
                            A.w(0x8A200000 | T0 << 16 | T1 << 5 | T1)      # bic x17, x17, x16
                        A.mem("str", T1, T2, VSEN)
                elif f3 == 5:
                    fs_addr(T2)
                    A.mem("str", ZR, T2, VSEN)
            return
        if op == "amo":                       # one call of rt_amo: x16 = address, x17 = x[rs2] in, x16 = x[rd] out
            A.movi(T2, 4 * k)
            A.mem("str", T2, SPR, FR["fault_off"])
            A.movi(T2, sx(words[k], 32))
            A.mem("str", T2, SPR, FR["fault_word"])
            r0 = src(rs1, T0)
            if r0 != T0:
                A.mov(T0, r0)
            r0 = src(rs2, T1)
            if r0 != T1:
                A.mov(T1, r0)
            A.mem("str", 30, SPR, FR["lr_cold"])
            A.bl("rt_amo")
            A.mem("ldr", 30, SPR, FR["lr_cold"])
            if fast:
                slots.clear()                 # an AMO may write the stack
                forget_reg(rd)
            if rd:
                r = dst(rd)
                if r != T0:
                    A.mov(r, T0)
                fin(rd, r)
            return
        if op == "vec":                       # one call of rt_vec: x16 = x[rs1], x17 = x[rs2] in, x16 = x[rd] out
            A.movi(T2, 4 * k)
            A.mem("str", T2, SPR, FR["fault_off"])
            A.movi(T2, sx(words[k], 32))
            A.mem("str", T2, SPR, FR["fault_word"])
            r0 = src(rs1, T0)
            if r0 != T0:
                A.mov(T0, r0)
            r0 = src(rs2, T1)
            if r0 != T1:
                A.mov(T1, r0)
            A.mem("str", 30, SPR, FR["lr_cold"])
            A.bl("rt_vec")
            A.mem("ldr", 30, SPR, FR["lr_cold"])
            if fast:
                slots.clear()                 # a vector store may write the stack
            if rd:
                r = dst(rd)
                if r != T0:
                    A.mov(r, T0)
                fin(rd, r)
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
            elif fj and op == "lui" and k in fj["hints"]:
                A.mem("str", 30, SPR, FR["lr_cold"])   # the marker: a region may run on threads here
                A.bl("rt_fj")
                A.mem("ldr", 30, SPR, FR["lr_cold"])
                if fast:
                    slots.clear()
            return
        if op == "jal" and imm == 0:
            # `jal rd, .`: yantra's SpinForever { pc } (H2), exit 1
            A.movi(T2, 4 * k)
            A.mem("str", T2, SPR, FR["fault_off"])
            A.b_("rt_vd_spin" if vd else "rt_spin")      # vd mode prints its own verdict line
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
                A.label("B%s_%x" % (V, pc))        # the cold stub's way back (the socket's RX)
                gmem(kind, r, T0, 0)
            if op in ("fld", "flw"):
                if op == "flw":
                    A.movi(T2, 0xFFFFFFFF00000000)         # a single is BOXED: the upper 32 bits all ones
                    A.rrr("orr", T1, T1, T2)
                A.mem("str", T1, SPR, FR["fregs"] + 8 * rd)
                if fs_dirty:                       # FS Dirty, as yantra's mark_fp_dirty
                    fs_addr(T2)
                    A.movz(T1, 3)
                    A.mem("str", T1, T2, FSEN)
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
                if wm:
                    wm_a64(A, M[2], imm + width)
                gmem(kind, value(), M[2], imm)
                forget_range(imm, width)
                if op == "sd" and rs2 and rs2 in M:
                    slots[imm] = rs2
                return
            if fast:
                slots.clear()
            addr(rs1, imm)
            guard(width, "s", val)
            if wm:
                wm_a64(A, T0, width)
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
                if fs_dirty and (kind == 1 or rs1):
                    # a write that happens (csrrw always, set / clear with a non-zero mask) makes FS Dirty
                    at = None
                    if kind != 1 and CSR_OPS.index(op) < 3:
                        at = len(A.b)
                        A.w(0)
                    fs_addr(T2)
                    A.movz(T1, 3)
                    A.mem("str", T1, T2, FSEN)
                    if at is not None:
                        struct.pack_into("<I", A.b, at, 0xB4000000 | ((len(A.b) - at) >> 2) << 5 | T0)   # cbz x16, here
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
                if imm & 7 >= 5:
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
                if fs_dirty:
                    fs_addr(T2)
                    A.movz(T1, 3)
                    A.mem("str", T1, T2, FSEN)
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
            A.b_(("XF_%x" if fj else "G_%x") % pc)
            A.label("FB_%x" % pc)
            charge(k)
        lower(k, True)
    A.movi(T0, base + 4 * nwords)
    A.b_("rt_bad_target")
    # ---------------- the SAFE region: every word, every access guarded
    for k in range(nwords):
        A.label("G_%x" % (base + 4 * k))
        charge1(k)
        lower(k, False)
        d = decs[k]
        if fj and k + 1 < nwords and d and d[1] == 2 and d[0] not in BR + MEM_S:
            A.b_("F_%x" % (base + 4 * k + 4))      # FORK-JOIN: a new sp is checked before it is used
    A.movi(T0, base + 4 * nwords)
    A.b_("rt_bad_target")
    if fj:
        fj["fjoff"] = FJ64_OFF                    # the F/D data is always reserved in a fork-join image
        fj["ext_end"] = (base + extent + 0xFFF) & ~0xFFF
        # a FAST check that failed: the main thread runs the block SAFE; a worker has left its slice
        for k in range(nwords):
            if leader[k]:
                pc = base + 4 * k
                A.label("XF_%x" % pc)
                fj_base_a64(A, fj)
                A.mem("ldr", 18, 18, FJ64["fj_worker"])
                A.cbnz(18, "XFW_%x" % pc)
                A.b_("G_%x" % pc)
                A.label("XFW_%x" % pc)
                A.b_("rt_fj_stack")
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
            A.bl("rt_store_slow")              # returns only for the UART (and the socket's NEXT / TX)
            A.mem("ldr", 30, SPR, FR["lr_cold"])
            A.b_("B%s_%x" % (V, pc))
        else:
            A.mem("str", 30, SPR, FR["lr_cold"])
            A.bl("rt_load_slow")               # returns only for the socket's RX: T0 = the value's address
            A.mem("ldr", 30, SPR, FR["lr_cold"])
            A.b_("B%s_%x" % (V, pc))
    for k in sorted(stubs):                       # the limit falls in block k: its words back (T1 holds the borrowed count), then SAFE
        A.label("S_%x" % (base + 4 * k))
        if rem[k] < 4096:
            A.add_i(T1, T1, rem[k])
        else:
            A.movi(T2, rem[k])
            A.rrr("add", T1, T1, T2)
        A.mem("str", T1, SPR, FR["steps"])
        if fj:
            fj_base_a64(A, fj)
            A.mem("ldr", 18, 18, FJ64["fj_worker"])
            A.cbnz(18, "rt_fj_stepw")
        A.b_("G_%x" % (base + 4 * k))
    for k in sorted(stubs1):                      # the limit is met at word k: fault_off its offset
        A.label("T_%x" % (base + 4 * k))
        A.movi(T0, 4 * k)
        A.mem("str", T0, SPR, FR["fault_off"])
        A.b_("rt_stepcross")
    runtime_a64(A, base, entry, max(DEFAULT_RAM, extent + RAM_HEADROOM), len(span), code_size, prot_size, mac, fj, wm, vd, scanp,
                fp=fp_used)
    if fp_used:
        A.raw(b"\0" * (-len(A.b) % 4))              # the runtime's strings end unaligned; the F/D code is words
        runtime_fp_a64(A, a64_frame_size(mac), os.environ.get("ANUVADA_SOFT_ALL") == "1", mac)
    if vec_used:
        runtime_vec_a64(A, a64_frame_size(mac), mac, wm)
    if amo_used:
        A.raw(b"\0" * (-len(A.b) % 4))              # the runtime's strings end unaligned; this code is words
        runtime_amo_a64(A, mac, wm)
    if csrh_used:
        A.raw(b"\0" * (-len(A.b) % 4))
        runtime_csrh_a64(A, base, mac)
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
        return pe_a64(A, imp, frame - data + A64_FRAME_WIN_SIZE + (FP_A64_SIZE if fp_used else 0) + (VEC_A64_SIZE if vec_used else 0) + (16 if csr_data else 0),
                      labs["pe_iat"] - WBASE, labs["pe_iat_end"] - labs["pe_iat"]), M
    if mac:
        # the frame (bss) is __DATA, right after __TEXT (16 KiB pages); the table holds offsets
        frame = (A.here() + 0x3FFF) & ~0x3FFF
        A.resolve({"frame": frame})
        for k in range(nwords):
            struct.pack_into("<q", A.b, jt_at + 8 * k, A.labels[tgt(base + 4 * k)] - A.labels["jt"])
        return macho_a64(A, A64_FRAME_MAC_SIZE + (FP_A64_SIZE if fp_used else 0) + (VEC_A64_SIZE if vec_used else 0) + (16 if csr_data else 0)), M
    # ---------------- the frame (bss), 64 KiB aligned after the text (any page size)
    frame = (A.here() + 0xFFFF) & ~0xFFFF
    if wm:
        frame += 64 if vd else 16                    # hw (and the vd words) are the octets before the frame
    bss_end = frame + A64_FRAME_SIZE + (FP_A64_SIZE if fp_used else 0) + (VEC_A64_SIZE if vec_used else 0) + (16 if csr_data else 0)
    vd_buf = bss_end
    A.resolve({"frame": frame, "vd_buf": vd_buf, "vd_out": vd_buf + VD_BUF})
    for k in range(nwords):
        struct.pack_into("<Q", A.b, jt_at + 8 * k, A.labels[tgt(base + 4 * k)])
    if fj:
        return elf_a64(A, frame + fj["fjoff"] + FJ64_SIZE), M
    return elf_a64(A, vd_buf + VD_BUF + VD_OUT if vd else bss_end), M


# ---------------------------------------------------------------- SASTHRDS threads (A64), SL6 / K3
# yantra's W-376 threads (crates/yantra/src/threads.rs, yantra-run.rs) on ONE host thread: N guest
# contexts in a host area at the end of the RAM mapping, switched only at a WAIT store or a thread's
# finisher. Thread 0 starts at entry; thread k >= 1 at entry + 8 with sp = base + stacks_at + k*64K and
# every other register 0. stacks_at = align16(ram), stores stop at stacks_at + (n-1)*64K (made before
# the input is injected, so the input sits above them). [SASTHRID + 8] = k at every switch-in. Live: the
# next thread is round-robin from the one after the current, `@k` is logged and a resuming thread gets
# `t=<now ns>` in [SASEVENT + 8]; replay (YANTRA_REPLAY_EVENTS, rt_next_rec): the log decides, and a
# log that does not fit goes to yantra-run (m_thbad, exit 2). The end rule: the first non-zero status
# in end order is the run's. Any other halt ends the whole run, its halt line held back (th_cap) so
# yantra's thread line comes first.
TH_CTX, TH_CAP = 576, 1024                 # a context: +0 state (0 Fresh 1 Waiting 2 Running 3 Ended),
TH_AREA = TH_CAP + 64 * TH_CTX             # +8 resume, +24 fcsr, +32 x[32], +288 f[32]; the capture first
THREAD_ID_TAG = 0x4449_5248_5453_4153
TH_HDR = (b"# yantra-run --record-events (W-376): a threaded run; @N ran thread N at each decision point, "
          b"round-robin, and t=<host time in ns since the Unix epoch, UTC> is the value a resuming thread "
          b"was given; replay with --events\n")
TH_HDR_LEN = len(TH_HDR)
TH_CTX_X86 = 672                           # x86-64 (runtime()'s threads block): +0 state, +8 resume, +24 fcsr,
TH_AREA_X86 = TH_CAP + 64 * TH_CTX_X86     # +32 regs and fregs (512), +544 the host registers by number


def runtime_threads_a64(A, FR, base, entry, mac, sysc, adr, hbase, msg, strings, MAP, fp, fj):
    UNM = [rv for rv in range(1, 32) if rv not in MAP]
    RV30 = [rv for rv in MAP if MAP[rv] == 30][0]    # the guest register host x30 carries (lr_cold)
    FCSR = a64_frame_size(mac) + fp_a64_off("fcsr")
    EM = b"\xe2\x80\x94"

    def fcsr_at(r):
        A.add_i12(r, SPR, FCSR >> 12)
        A.add_i(r, r, FCSR & 0xFFF)

    def ctx_of(rd, rk, s1, s2):            # rd = the context of thread rk
        A.movi(s1, TH_CTX)
        A.mem("ldr", s2, SPR, FR["th_ctx"])
        A.madd(rd, rk, s1, s2)
        A.add_i(rd, rd, TH_CAP)

    # ---- pre: x1 = the mapping's length so far; a threaded image's stacks and contexts are added
    A.label("rt_th_pre")
    A.mem("str", 30, SPR, FR["th_lr"])
    A.mem("str", 1, SPR, FR["th_s1"])
    adr(15, "image")
    A.movi(0, THREADS_TAG)
    A.bl("rt_find_tag_at")
    A.mem("str", 12, SPR, FR["th_pre"])
    adr(15, "image")
    A.movi(0, THREAD_ID_TAG)
    A.bl("rt_find_tag_at")
    A.mem("ldr", 9, SPR, FR["th_pre"])
    A.rrr("add", 9, 9, 12)
    A.mem("str", 9, SPR, FR["th_pre"])
    A.mem("ldr", 1, SPR, FR["th_s1"])
    A.mem("ldr", 30, SPR, FR["th_lr"])
    A.cbz(9, "rt_thp1")
    A.movi(9, 63 * 65536 + 256 + TH_AREA)
    A.rrr("add", 1, 1, 9)
    A.label("rt_thp1")
    A.ret()

    # ---- detect: after the image is copied, before the input (yantra's order)
    A.label("rt_th_detect")
    A.mem("ldr", 9, SPR, FR["th_pre"])
    A.cbnz(9, "rt_thd0")
    A.ret()
    A.label("rt_thd0")
    if mac == "win" or fj:
        A.b_("rt_threaded")                # no Windows threads yet; a fork-join image: yantra-run
    A.mem("str", 30, SPR, FR["th_lr"])
    A.movi(0, THREADS_TAG)                 # anything yantra might refuse goes to yantra-run whole
    A.bl("rt_find_tag")
    A.cbz(0, "rt_threaded")
    A.mov(19, 0)
    A.movi(0, THREAD_ID_TAG)
    A.bl("rt_find_tag")
    A.cbz(0, "rt_threaded")
    A.mem("str", 0, SPR, FR["th_id"])
    A.mem("ldr", 23, 19, 8)                # the count, 1..64
    A.add_i(9, 23, -1)
    A.cmp_i(9, 64)
    A.bcond(HS, "rt_threaded")
    A.movi(0, EVENT_TAG)
    A.bl("rt_find_tag")
    A.cbz(0, "rt_threaded")
    hbase(9)                               # the entry: auipc sp / addi sp (thread k starts after them)
    A.movi(10, entry - base)
    A.rrr("add", 9, 9, 10)
    A.mem("ldrw", 10, 9, 0)
    A.ubfm(10, 10, 0, 11)
    A.movi(11, 0x117)
    A.cmp(10, 11)
    A.bcond(NE, "rt_threaded")
    A.mem("ldrw", 10, 9, 4)
    A.ubfm(10, 10, 0, 19)
    A.movi(11, 0x10113)
    A.cmp(10, 11)
    A.bcond(NE, "rt_threaded")
    adr(0, "th_env_listen")                # a socket run: yantra refuses it
    A.bl("rt_getenv")
    A.cbnz(0, "rt_threaded")
    A.mem("str", 23, SPR, FR["th_n"])
    A.bl("m_th1")
    A.mov(9, 23)
    A.bl("rt_putu")
    A.bl("m_nl")
    A.mem("ldr", 9, SPR, FR["rec_fd"])
    A.mem("ldr", 10, SPR, FR["rep_fd"])
    A.rrr("orr", 9, 9, 10)
    A.cbz(9, "rt_th_nolog")
    A.mem("ldr", 9, SPR, FR["ram"])
    A.add_i(9, 9, 15)
    A.and_l(9, 9, LOG_NOT15)
    A.mem("str", 9, SPR, FR["th_stk"])
    A.add_i(10, 23, -1)
    A.lsl_i(10, 10, 16)
    A.rrr("add", 10, 10, 9)
    A.mem("str", 10, SPR, FR["th_slim"])
    A.mem("str", 10, SPR, FR["top"])
    A.bl("m_th1")
    A.mov(9, 23)
    A.bl("rt_putu")
    A.bl("m_th2")
    A.add_i(9, 23, -1)
    A.bl("rt_putu")
    A.bl("m_th3")
    A.mem("ldr", 9, SPR, FR["th_stk"])
    A.bl("rt_puthex")
    A.bl("m_th4")
    A.mem("ldr", 9, SPR, FR["th_slim"])
    A.bl("rt_putu")
    A.bl("m_nl")
    A.mem("ldr", 9, SPR, FR["th_len"])
    A.movi(10, TH_AREA)
    A.rrr("sub", 9, 9, 10)
    hbase(10)
    A.rrr("add", 9, 9, 10)
    A.mem("str", 9, SPR, FR["th_ctx"])
    A.add_i(9, 23, -1)
    A.mem("str", 9, SPR, FR["th_cur"])     # so the first pick is thread 0
    if mac != "win":
        # the log is a thread log: yantra's thread header in place of the clock one
        A.mem("ldr", 9, SPR, FR["rec_fd"])
        A.cbz(9, "rt_thd9")
        A.add_i(0, 9, -1)
        A.movz(1, 0)
        sysc(46, 201)                      # ftruncate(fd, 0)
        A.mem("ldr", 0, SPR, FR["rec_fd"])
        A.add_i(0, 0, -1)
        A.movz(1, 0)
        A.movz(2, 0)
        sysc(62, 199)                      # lseek(fd, 0, SEEK_SET)
        A.mem("ldr", 0, SPR, FR["rec_fd"])
        A.add_i(0, 0, -1)
        adr(1, "th_hdr")
        A.movi(2, len(TH_HDR))
        sysc(64, 4)
        A.label("rt_thd9")
    A.mem("ldr", 30, SPR, FR["th_lr"])
    A.ret()
    A.label("rt_th_nolog")
    A.bl("m_thnolog")
    A.movz(0, 1)
    A.b_("rt_exit")

    # ---- the decision point: at the start, at every wait, at every end (all guest state saved)
    A.label("rt_th_next")
    A.mem("ldr", 9, SPR, FR["th_ended"])
    A.mem("ldr", 23, SPR, FR["th_n"])
    A.cmp(9, 23)
    A.bcond(HS, "rt_thn_end")
    A.mem("ldr", 9, SPR, FR["rep_fd"])
    A.cbnz(9, "rt_thn_rep")
    A.mem("ldr", 19, SPR, FR["th_cur"])    # live: round-robin from the one after the current
    A.mov(21, 23)
    A.label("rt_thn1")
    A.add_i(19, 19, 1)
    A.cmp(19, 23)
    A.bcond(LO, "rt_thn2")
    A.movz(19, 0)
    A.label("rt_thn2")
    ctx_of(20, 19, 9, 10)
    A.mem("ldr", 9, 20, 0)
    A.cmp_i(9, 3)
    A.bcond(NE, "rt_thn3")
    A.add_i(21, 21, -1)
    A.cbnz(21, "rt_thn1")
    A.b_("rt_thn_end")
    A.label("rt_thn3")
    A.mov(9, 19)
    A.movz(10, 0x40)                       # "@k"
    A.movz(11, 1)
    A.bl("rt_th_wr")
    A.mem("ldr", 9, 20, 0)
    A.cmp_i(9, 1)
    A.bcond(NE, "rt_th_in")
    A.bl("rt_clock")
    A.mov(22, 0)
    A.label("rt_thn_val")                  # a resuming thread's value, delivered at the resume
    A.mem("ldr", 9, SPR, FR["ev_slot"])
    A.mem("str", 22, 9, 8)
    A.mov(9, 22)
    A.movz(10, 0x3D74)                     # "t="
    A.movz(11, 2)
    A.bl("rt_th_wr")
    A.b_("rt_th_in")
    A.label("rt_thn_rep")                  # replay: the next record is `@k`, k a thread not ended
    A.bl("rt_next_rec")
    A.cmp_i(0, 1)
    A.bcond(NE, "rt_th_bad")
    A.cmp(1, 23)
    A.bcond(HS, "rt_th_bad")
    A.mov(19, 1)
    ctx_of(20, 19, 9, 10)
    A.mem("ldr", 9, 20, 0)
    A.cmp_i(9, 3)
    A.bcond(EQ, "rt_th_bad")
    A.mov(9, 19)
    A.movz(10, 0x40)
    A.movz(11, 1)
    A.bl("rt_th_wr")
    A.mem("ldr", 9, 20, 0)
    A.cmp_i(9, 1)
    A.bcond(NE, "rt_th_in")
    A.bl("rt_next_rec")                    # a resuming thread: then a value record
    A.cbnz(0, "rt_th_bad")
    A.mov(22, 1)
    A.b_("rt_thn_val")
    A.label("rt_thn_end")                  # every thread ended: a replay's log must be spent too
    A.mem("ldr", 9, SPR, FR["rep_fd"])
    A.cbz(9, "rt_th_end")
    A.bl("rt_next_rec")
    A.cmp_i(0, 2)
    A.bcond(NE, "rt_th_bad")
    A.b_("rt_th_end")
    A.label("rt_th_bad")
    A.mem("str", ZR, SPR, FR["th_cap"])
    A.bl("rt_flush")
    A.bl("m_thbad")
    A.movz(0, 2)
    A.b_("rt_exit")

    # ---- wr: "<prefix><x9 decimal>\n" to the log (x10 the prefix's octets, x11 its length), if any
    A.label("rt_th_wr")
    A.mem("ldr", 12, SPR, FR["rec_fd"])
    A.cbz(12, "rt_thw9")
    A.add_i(1, SPR, FR["num"] + 31)
    A.movz(2, 10)
    A.mem("strb", 2, 1, 0)
    A.label("rt_thw1")
    A.dp2("udiv", 13, 9, 2)
    A.msub(12, 13, 2, 9)
    A.add_i(12, 12, 48)
    A.add_i(1, 1, -1)
    A.mem("strb", 12, 1, 0)
    A.mov(9, 13)
    A.cbnz(9, "rt_thw1")
    A.rrr("sub", 1, 1, 11)
    A.cmp_i(11, 1)
    A.bcond(NE, "rt_thw2")
    A.mem("strb", 10, 1, 0)
    A.b_("rt_thw3")
    A.label("rt_thw2")
    A.mem("strh", 10, 1, 0)
    A.label("rt_thw3")
    A.add_i(2, SPR, FR["num"] + 32)
    A.rrr("sub", 2, 2, 1)
    A.mem("ldr", 0, SPR, FR["rec_fd"])
    A.add_i(0, 0, -1)
    sysc(64, 4)
    A.label("rt_thw9")
    A.ret()

    # ---- switch in: x19 = k, x20 = its context
    A.label("rt_th_in")
    A.mem("str", 19, SPR, FR["th_cur"])
    A.mem("ldr", 9, SPR, FR["th_id"])
    A.mem("str", 19, 9, 8)                 # SASTHRID's word = k, at every switch-in
    A.movz(9, 1)
    A.mem("str", 9, SPR, FR["th_cap"])
    A.mem("str", ZR, SPR, FR["th_capn"])
    A.mem("ldr", 9, 20, 0)
    A.movz(10, 2)
    A.mem("str", 10, 20, 0)
    A.cmp_i(9, 1)
    A.bcond(EQ, "rt_thi_res")
    for rv in UNM:                          # FRESH: every register 0, sp its stack, pc entry (+8)
        A.mem("str", ZR, SPR, FR["regs"] + 8 * rv)
    for i in range(32):
        A.mem("str", ZR, SPR, FR["fregs"] + 8 * i)
    A.movi(9, fp)
    A.cbz(9, "rt_thi1")
    fcsr_at(10)
    A.mem("str", ZR, 10, 0)
    A.label("rt_thi1")
    A.movi(T0, entry)
    A.movz(T1, 0)
    A.cbz(19, "rt_thi2")
    A.add_i(T0, T0, 8)
    A.mem("ldr", 9, SPR, FR["th_stk"])
    A.lsl_i(10, 19, 16)
    A.rrr("add", 9, 9, 10)
    A.movi(10, base)
    A.rrr("add", T1, 9, 10)
    A.label("rt_thi2")
    for n in sorted(MAP):
        A.movz(MAP[n], 0)
    A.mov(MAP[2], T1)
    if mac:
        A.mem("ldr", DELTA, SPR, FR["delta"])
    A.b_("rt_indirect")
    A.label("rt_thi_res")                  # WAITING: its registers back, then return into its stub
    A.mov(T0, 20)
    for rv in UNM:
        A.mem("ldr", T1, T0, 32 + 8 * rv)
        A.mem("str", T1, SPR, FR["regs"] + 8 * rv)
    for i in range(32):
        A.mem("ldr", T1, T0, 288 + 8 * i)
        A.mem("str", T1, SPR, FR["fregs"] + 8 * i)
    A.movi(T1, fp)
    A.cbz(T1, "rt_thi3")
    A.mem("ldr", T1, T0, 24)
    fcsr_at(T2)
    A.mem("str", T1, T2, 0)
    A.label("rt_thi3")
    A.mem("ldr", T1, T0, 32 + 8 * RV30)
    A.mem("str", T1, SPR, FR["lr_cold"])
    A.mem("ldr", 30, T0, 8)
    for rv in sorted(MAP):
        if MAP[rv] != 30:
            A.mem("ldr", MAP[rv], T0, 32 + 8 * rv)
    if mac:
        A.mem("ldr", DELTA, SPR, FR["delta"])
    A.ret()

    # ---- a thread's wait (from rt_wait: x30 in lr_slow, x0 x1 in sv01, x2 x8 in sv28): park it
    A.label("rt_th_wait")
    A.mem("ldr", T0, SPR, FR["th_bos"])
    A.cbnz(T0, "rt_wt_halt")               # the segment that stored past its stack ends here
    A.mem("ldr", T1, SPR, FR["th_cur"])
    ctx_of(T0, T1, T2, T0)
    for rv in sorted(MAP):
        if MAP[rv] not in (0, 1, 2, 8, 30):
            A.mem("str", MAP[rv], T0, 32 + 8 * rv)
    A.ldp(0, 1, SPR, FR["sv01"])
    A.ldp(2, 8, SPR, FR["sv28"])
    for rv in sorted(MAP):
        if MAP[rv] in (0, 1, 2, 8):
            A.mem("str", MAP[rv], T0, 32 + 8 * rv)
    A.mem("ldr", T1, SPR, FR["lr_cold"])
    A.mem("str", T1, T0, 32 + 8 * RV30)
    for rv in UNM:
        A.mem("ldr", T1, SPR, FR["regs"] + 8 * rv)
        A.mem("str", T1, T0, 32 + 8 * rv)
    for i in range(32):
        A.mem("ldr", T1, SPR, FR["fregs"] + 8 * i)
        A.mem("str", T1, T0, 288 + 8 * i)
    A.movi(T1, fp)
    A.cbz(T1, "rt_thw_nf")
    fcsr_at(T2)
    A.mem("ldr", T1, T2, 0)
    A.mem("str", T1, T0, 24)
    A.label("rt_thw_nf")
    A.mem("ldr", T1, SPR, FR["lr_slow"])
    A.mem("str", T1, T0, 8)
    A.movz(T1, 1)
    A.mem("str", T1, T0, 0)
    A.b_("rt_th_next")

    # ---- a thread's finisher (T1 = the value): the thread ends; the first failure is kept
    A.label("rt_th_fin")
    A.mem("ldr", T2, SPR, FR["th_bos"])
    A.cbnz(T2, "rt_fin_print")
    A.mem("str", T1, SPR, FR["th_lastv"])
    A.ubfm(T2, T1, 0, 15)
    A.movi(T0, 0x5555)
    A.cmp(T2, T0)
    A.bcond(EQ, "rt_thf_ok")
    A.movi(T0, 0x3333)
    A.cmp(T2, T0)
    A.bcond(NE, "rt_thf_bad")
    A.lsr_i(T2, T1, 16)
    A.cbz(T2, "rt_thf_ok")
    A.label("rt_thf_bad")
    A.mem("ldr", T2, SPR, FR["th_failk"])
    A.cbnz(T2, "rt_thf_ok")
    A.mem("ldr", T2, SPR, FR["th_cur"])
    A.add_i(T2, T2, 1)
    A.mem("str", T2, SPR, FR["th_failk"])
    A.mem("str", T1, SPR, FR["th_failv"])
    A.label("rt_thf_ok")
    A.mem("ldr", 19, SPR, FR["th_cur"])
    ctx_of(20, 19, 9, 10)
    A.movz(9, 3)
    A.mem("str", 9, 20, 0)
    A.mem("ldr", 9, SPR, FR["th_ended"])
    A.add_i(9, 9, 1)
    A.mem("str", 9, SPR, FR["th_ended"])
    A.b_("rt_th_next")

    # ---- the end: every thread ended
    A.label("rt_th_end")
    A.mem("str", ZR, SPR, FR["th_cap"])
    A.mem("ldr", 9, SPR, FR["th_failk"])
    A.cbz(9, "rt_the_ok")
    A.bl("rt_flush")
    A.bl("m_thf1")
    A.mem("ldr", 9, SPR, FR["th_failk"])
    A.add_i(9, 9, -1)
    A.bl("rt_putu")
    A.bl("m_thf2")
    A.mem("ldr", 9, SPR, FR["th_failv"])
    A.ubfm(10, 9, 0, 15)
    A.movi(11, 0x3333)
    A.cmp(10, 11)
    A.bcond(NE, "rt_the_u")
    A.lsr_i(9, 9, 16)
    A.bl("rt_putu")
    A.b_("rt_the_t")
    A.label("rt_the_u")
    A.bl("m_thf3")
    A.mem("ldr", 9, SPR, FR["th_failv"])
    A.bl("rt_puthex")
    A.bl("m_thf4")
    A.label("rt_the_t")
    A.bl("m_thf5")
    A.mem("ldr", T1, SPR, FR["th_failv"])
    A.b_("rt_fin_print")
    A.label("rt_the_ok")
    A.mem("ldr", T1, SPR, FR["th_lastv"])
    A.b_("rt_fin_print")

    # ---- a store past RAM (T0 = guest addr, T1 = value; x30 = the stub's return): a stack's lands
    A.label("rt_th_beyond")
    A.mem("str", 0, SPR, FR["th_s0"])
    A.mem("str", 1, SPR, FR["th_s1"])
    A.movi(0, base)
    A.rrr("sub", 0, T0, 0)                 # x0 = the offset
    A.mem("ldr", 1, SPR, FR["ram"])
    A.cmp(0, 1)
    A.bcond(LO, "rt_thb_out")
    A.mem("ldr", 1, SPR, FR["th_slim"])
    A.cmp(0, 1)
    A.bcond(HS, "rt_thb_out")
    A.mem("ldr", T2, SPR, FR["fault_off"])  # the width, from the store's word
    hbase(1)
    A.memr("ldrw", T2, 1, T2)
    A.ubfm(T2, T2, 12, 13)
    A.rrr("add", 1, 1, 0)
    A.cbz(T2, "rt_thb_b")
    A.cmp_i(T2, 1)
    A.bcond(EQ, "rt_thb_h")
    A.cmp_i(T2, 2)
    A.bcond(EQ, "rt_thb_w")
    A.mem("str", T1, 1, 0)
    A.movz(T0, 8)
    A.b_("rt_thb_chk")
    A.label("rt_thb_b")
    A.mem("strb", T1, 1, 0)
    A.movz(T0, 1)
    A.b_("rt_thb_chk")
    A.label("rt_thb_h")
    A.mem("strh", T1, 1, 0)
    A.movz(T0, 2)
    A.b_("rt_thb_chk")
    A.label("rt_thb_w")
    A.mem("strw", T1, 1, 0)
    A.movz(T0, 4)
    A.label("rt_thb_chk")                  # wholly inside its own stack? (thread 0 has none)
    A.mem("ldr", T2, SPR, FR["th_cur"])
    A.cbz(T2, "rt_thb_bos")
    A.add_i(T2, T2, -1)
    A.lsl_i(T2, T2, 16)
    A.mem("ldr", 1, SPR, FR["th_stk"])
    A.rrr("add", T2, T2, 1)
    A.cmp(0, T2)
    A.bcond(LO, "rt_thb_bos")
    A.rrr("add", 1, 0, T0)
    A.add_i12(T2, T2, 16)
    A.cmp(1, T2)
    A.bcond(LS, "rt_thb_ret")
    A.label("rt_thb_bos")
    A.mem("ldr", T2, SPR, FR["th_bos"])
    A.cbnz(T2, "rt_thb_ret")
    A.movi(1, base)
    A.rrr("add", 1, 0, 1)
    A.mem("str", 1, SPR, FR["th_bos"])
    A.mem("ldr", T2, SPR, FR["th_cur"])
    A.mem("str", T2, SPR, FR["th_bosk"])
    A.label("rt_thb_ret")
    A.mem("ldr", 0, SPR, FR["th_s0"])
    A.mem("ldr", 1, SPR, FR["th_s1"])
    A.ret()
    A.label("rt_thb_out")
    A.mem("ldr", 0, SPR, FR["th_s0"])
    A.mem("ldr", 1, SPR, FR["th_s1"])
    A.b_("rt_beyond0")

    # ---- a thread's halt (x0 = the code; its lines are in the capture): yantra's thread line first
    A.label("rt_th_exit")
    A.mem("str", 0, SPR, FR["th_rc"])
    A.mem("str", ZR, SPR, FR["th_cap"])
    A.mem("ldr", 19, SPR, FR["th_ctx"])
    A.mem("ldr", 20, SPR, FR["th_capn"])
    A.cmp_i(20, 6)
    A.bcond(LO, "rt_thx_raw")
    A.mem("ldrw", 9, 19, 0)
    A.movi(10, 0x746C6168)                 # "halt"
    A.cmp(9, 10)
    A.bcond(NE, "rt_thx_raw")
    A.mem("ldrh", 9, 19, 4)
    A.movi(10, 0x203A)                     # ": "
    A.cmp(9, 10)
    A.bcond(NE, "rt_thx_raw")
    A.movz(21, 6)
    A.label("rt_thx1")
    A.cmp(21, 20)
    A.bcond(HS, "rt_thx2")
    A.memr("ldrb", 9, 19, 21)
    A.cmp_i(9, 10)
    A.bcond(EQ, "rt_thx2")
    A.add_i(21, 21, 1)
    A.b_("rt_thx1")
    A.label("rt_thx2")
    # two passes over the why: x23 = 0 yantra's `threads:` line (before the held lines), 1 yantra-run's
    # last line `yantra-run: the threaded run failed — <why>` (a BeyondOwnStack, or a SpinForever halt)
    A.movz(23, 0)
    A.label("rt_thx_line")
    A.mem("ldr", 9, SPR, FR["th_bos"])
    A.cbnz(9, "rt_thx_bos")
    A.cbnz(23, "rt_thx_h1")
    A.bl("m_thh0")
    A.b_("rt_thx_h2")
    A.label("rt_thx_h1")
    A.bl("m_thfail")
    A.label("rt_thx_h2")
    A.bl("m_thw1")
    A.mem("ldr", 9, SPR, FR["th_cur"])
    A.bl("rt_putu")
    A.bl("m_thh2")
    A.bl("rt_thx_text")
    A.bl("m_thh3")
    A.b_("rt_thx_next")
    A.label("rt_thx_bos")
    A.cbnz(23, "rt_thx_b1")
    A.bl("m_thb1")
    A.b_("rt_thx_b2")
    A.label("rt_thx_b1")
    A.bl("m_thfail")
    A.label("rt_thx_b2")
    A.bl("m_thw1")
    A.mem("ldr", 9, SPR, FR["th_bosk"])
    A.bl("rt_putu")
    A.bl("m_thb2")
    A.mem("ldr", 9, SPR, FR["th_bos"])
    A.bl("rt_puthex")
    A.bl("m_thb3")
    A.mem("ldr", 9, SPR, FR["ram"])
    A.bl("rt_putu")
    A.bl("m_thb4")
    A.mem("ldr", 22, SPR, FR["th_bosk"])
    A.cbnz(22, "rt_thx_own")
    A.bl("m_thb5")
    A.b_("rt_thx_b8")
    A.label("rt_thx_own")
    A.bl("m_thb6")
    A.add_i(22, 22, -1)
    A.lsl_i(22, 22, 16)
    A.mem("ldr", 9, SPR, FR["th_stk"])
    A.rrr("add", 22, 22, 9)
    A.movi(9, base)
    A.rrr("add", 22, 22, 9)
    A.mov(9, 22)
    A.bl("rt_puthex")
    A.bl("m_thb7")
    A.add_i12(9, 22, 16)
    A.bl("rt_puthex")
    A.label("rt_thx_b8")
    A.bl("m_thb8")
    A.bl("rt_thx_text")
    A.bl("m_thb9")
    A.movz(9, 1)                           # BeyondOwnStack always fails
    A.mem("str", 9, SPR, FR["th_rc"])
    A.label("rt_thx_next")
    A.cbnz(23, "rt_thx_end")
    A.movz(0, 2)                           # the held lines
    A.mov(1, 19)
    A.mov(2, 20)
    sysc(64, 4)
    A.mem("ldr", 9, SPR, FR["th_bos"])
    A.cbnz(9, "rt_thx_fail")
    A.cmp_i(21, 6 + 11)                    # "SpinForever"
    A.bcond(LO, "rt_thx_end")
    A.mem("ldrw", 9, 19, 6)
    A.movi(10, 0x6E697053)                 # "Spin"
    A.cmp(9, 10)
    A.bcond(NE, "rt_thx_end")
    A.mem("ldrw", 9, 19, 10)
    A.movi(10, 0x65726F46)                 # "Fore"
    A.cmp(9, 10)
    A.bcond(NE, "rt_thx_end")
    A.movz(9, 1)                           # a thread's SpinForever fails the run (yantra-run)
    A.mem("str", 9, SPR, FR["th_rc"])
    A.label("rt_thx_fail")
    A.movz(23, 1)
    A.b_("rt_thx_line")
    A.label("rt_thx_end")
    A.mem("ldr", 0, SPR, FR["th_rc"])
    A.b_("rt_exit")
    A.label("rt_thx_raw")
    A.movz(0, 2)
    A.mov(1, 19)
    A.mov(2, 20)
    sysc(64, 4)
    A.mem("ldr", 0, SPR, FR["th_rc"])
    A.b_("rt_exit")
    A.label("rt_thx_text")                 # the held halt line's text after "halt: "
    A.movz(0, 2)
    A.add_i(1, 19, 6)
    A.add_i(2, 21, -6)
    sysc(64, 4)
    A.ret()

    # ---- ewr: x1, x2 to stderr, or into the capture while a thread runs (clobbers x0 x1 x2 x8)
    A.label("rt_ewr")
    A.mem("ldr", 0, SPR, FR["th_cap"])
    A.cbz(0, "rt_ewr9")
    A.mem("ldr", 0, SPR, FR["th_capn"])
    A.rrr("add", 8, 0, 2)
    A.cmp_i(8, TH_CAP)
    A.bcond(HI, "rt_ewr_ov")
    A.mem("ldr", 8, SPR, FR["th_ctx"])
    A.rrr("add", 0, 0, 8)
    A.label("rt_ewr1")
    A.cbz(2, "rt_ewr2")
    A.mem("ldrb", 8, 1, 0)
    A.mem("strb", 8, 0, 0)
    A.add_i(0, 0, 1)
    A.add_i(1, 1, 1)
    A.add_i(2, 2, -1)
    A.b_("rt_ewr1")
    A.label("rt_ewr2")
    A.mem("ldr", 8, SPR, FR["th_ctx"])
    A.rrr("sub", 0, 0, 8)
    A.mem("str", 0, SPR, FR["th_capn"])
    A.ret()
    A.label("rt_ewr_ov")                   # more than the capture holds: out as it is
    A.mem("str", 1, SPR, FR["th_e1"])
    A.mem("str", 2, SPR, FR["th_e2"])
    A.mem("str", ZR, SPR, FR["th_cap"])
    A.movz(0, 2)
    A.mem("ldr", 1, SPR, FR["th_ctx"])
    A.mem("ldr", 2, SPR, FR["th_capn"])
    sysc(64, 4)
    A.mem("ldr", 1, SPR, FR["th_e1"])
    A.mem("ldr", 2, SPR, FR["th_e2"])
    A.label("rt_ewr9")
    A.movz(0, 2)
    sysc(64, 4)
    A.ret()

    msg("m_th1", b"threads: ")
    msg("m_th2", b" contexts; ")
    msg("m_th3", b" host stacks of 65536 octets from RAM offset ")
    msg("m_th4", b", stores now stop at ")
    msg("m_thnolog", b"threads: refused " + EM + b" a threaded image needs --events or --record-events: which "
        b"thread runs first is a decision the log makes, and there is no default (refused at load, exit 1; "
        b"nothing ran)\n")
    msg("m_thbad", b"native: not in milestone 1 (a threaded replay whose log does not fit this run: "
        b"yantra-run judges it)\n")
    msg("m_thf1", b"threads: thread ")
    msg("m_thf2", b" ended with status ")
    msg("m_thf3", b"undefined (finisher value ")
    msg("m_thf4", b")")
    msg("m_thf5", b" " + EM + b" the first non-zero; every thread ran to its end and the run FAILS\n")
    msg("m_thh0", b"threads: ")
    msg("m_thw1", b"thread ")
    msg("m_thfail", b"yantra-run: the threaded run failed " + EM + b" ")
    msg("m_thh2", b" halted ")
    msg("m_thh3", b", not at its finisher " + EM + b" that ends the WHOLE run, because RAM is shared\n")
    msg("m_thb1", b"threads: BeyondOwnStack " + EM + b" ")
    msg("m_thb2", b" stored at ")
    msg("m_thb3", b", at or above the ")
    msg("m_thb4", b" octets of RAM the image was given and outside its own stack " + EM + b" ")
    msg("m_thb5", b"it has no host stack (thread 0's is the image's own)")
    msg("m_thb6", b"its own stack is ")
    msg("m_thb7", b"..")
    msg("m_thb8", b"; a heap past RAM or a stack overflowing into another thread's. The WHOLE run ends "
        b"(the segment ended ")
    msg("m_thb9", b")\n")
    strings.append(("th_env_listen", b"YANTRA_LISTEN=\0"))
    strings.append(("th_hdr", TH_HDR))


def runtime_a64(A, base, entry, ram_default, filesz, code_size, prot_size, mac=False, fj=None, wm=False, vd=False, scanp=None, fp=False):
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
        elif fj:
            # FORK-JOIN: a worker's host call goes to rt_fj_sw (x18 = where to come back); the main
            # thread's is the svc itself. exit is exit_group: threads may exist.
            A.movz(8, 94 if n == 93 else n)
            fj_base_a64(A, fj)
            A.mem("ldr", 18, 18, FJ64["fj_worker"])
            A.w(0xB4000000 | 3 << 5 | 18)          # cbz x18, the svc
            A.w(0x10000000 | 3 << 5 | 18)          # adr x18, after the svc
            A.b_("rt_fj_sw")
            A.svc()
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
        A.b_("rt_ewr")                         # stderr, or a thread's halt held back (runtime_threads_a64)
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
    if vd:
        # L33 YANTRA_VERDICT: the step ceiling (YANTRA_STEPS) and stderr into a memfd (the real one kept by dup)
        A.movi(9, VD_STEPS)
        A.mem("str", 9, SPR, VD_LIM)
        adr(0, "env_steps")
        A.bl("rt_getenv")
        A.cbz(0, "rt_vd_s1")
        A.bl("rt_atou_ram")
        A.cbnz(1, "rt_vd_s1")
        A.mem("str", 0, SPR, VD_LIM)
        A.label("rt_vd_s1")
        adr(0, "vd_nm")
        A.movz(1, 0)
        A.movz(8, 279)
        A.svc()                                  # memfd_create("vd", 0)
        A.cmp_i(0, 0)
        A.bcond(LT, "rt_vd_s3")
        A.mem("str", 0, SPR, VD_FD)
        A.movz(0, 2)
        A.movz(8, 23)
        A.svc()                                  # dup(2)
        A.cmp_i(0, 0)
        A.bcond(LT, "rt_vd_s2")
        A.mem("str", 0, SPR, VD_SV)
        A.mem("ldr", 0, SPR, VD_FD)
        A.movz(1, 2)
        A.movz(2, 0)
        A.movz(8, 24)
        A.svc()                                  # dup3(memfd, 2, 0)
        A.cmp_i(0, 0)
        A.bcond(GE, "rt_vd_s3")
        A.label("rt_vd_s2")
        A.movz(9, 0)
        A.mem("str", 9, SPR, VD_FD)              # no capture: no steps line, no verdict
        A.label("rt_vd_s3")
    if fj:
        # SASSEMBLY_NATIVE_THREADS=N (0: serial); unset: sched_getaffinity's CPUs, at most FJ_MAX
        adr(0, "env_fj")
        A.bl("rt_getenv")
        A.cbz(0, "rt_s1fc")
        A.bl("rt_atou")
        A.b_("rt_s1fm")
        A.label("rt_s1fc")
        fj_base_a64(A, fj)
        A.add_i(2, 18, FJ64["fj_cap"])
        A.movz(0, 0)
        A.movz(1, 1024)
        A.movz(8, 123)
        A.svc()
        A.mov(9, 0)                                # octets of the mask (or a negative error: 0 CPUs)
        A.add_i(10, 18, FJ64["fj_cap"])
        A.movz(0, 0)
        A.label("rt_s1fb")
        A.cmp_i(9, 0)
        A.bcond(LE, "rt_s1fm")
        A.mem("ldrb", 11, 10, 0)
        A.label("rt_s1fp")
        A.cbz(11, "rt_s1fq")
        A.add_i(12, 11, -1)
        A.rrr("and", 11, 11, 12)
        A.add_i(0, 0, 1)
        A.b_("rt_s1fp")
        A.label("rt_s1fq")
        A.add_i(10, 10, 1)
        A.add_i(9, 9, -1)
        A.b_("rt_s1fb")
        A.label("rt_s1fm")
        A.cmp_i(0, FJ_MAX)
        A.bcond(LS, "rt_s1fn")
        A.movz(0, FJ_MAX)
        A.label("rt_s1fn")
        fj_base_a64(A, fj)
        A.mem("str", 0, 18, FJ64["fj_n"])
        # SASSEMBLY_NATIVE_FJ_MIN=K: the small-region threshold (default FJ_K)
        A.movz(9, FJ_K)
        A.mem("str", 9, 18, FJ64["fj_k"])
        adr(0, "env_fjk")
        A.bl("rt_getenv")
        A.cbz(0, "rt_s1fj")
        A.bl("rt_atou")
        fj_base_a64(A, fj)
        A.mem("str", 0, 18, FJ64["fj_k"])
        A.label("rt_s1fj")
        # SASSEMBLY_NATIVE_FJATTEST=<path>: the attestation file (S1); unset = 0 = nothing written
        adr(0, "env_fja")
        A.bl("rt_getenv")
        fj_base_a64(A, fj)
        A.mem("str", 0, 18, FJ64["fj_att"])
    # a RAM no host can map (2^47 and up): unmappable, rerun under yantra-run (as runtime())
    A.mem("ldr", 9, SPR, FR["ram"])
    A.lsr_i(9, 9, 47)
    A.cbnz(9, "rt_no_ram")
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
    if mac == "win":
        A.b_("rt_win_ev")                      # the clock is not ported to Windows
    elif mac:
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
    # YANTRA_REPLAY_EVENTS=<file>: an --events replay's records (see x86-64); rep_fd = fd + 1
    adr(0, "env_rep")
    A.bl("rt_getenv")
    A.cbz(0, "rt_s1c")
    if mac == "win":
        A.b_("rt_win_ev")
    elif mac:
        A.movz(1, 0)                           # O_RDONLY
        A.movz(2, 0)
        sysc(0, 5)
        A.bcond(HS, "rt_no_rep")
    else:
        A.mov(1, 0)
        A.movn(0, 99)                          # AT_FDCWD
        A.movz(2, 0)
        A.movz(3, 0)
        A.movz(8, 56)
        A.svc()
        A.cmp_i(0, 0)
        A.bcond(LT, "rt_no_rep")
    A.add_i(9, 0, 1)
    A.mem("str", 9, SPR, FR["rep_fd"])
    A.label("rt_s1c")
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
    A.bl("rt_th_pre")                          # SASTHRDS: room for the stacks and the contexts
    A.add_i(1, 1, 0xFFF)
    A.and_l(1, 1, LOG_NOT4095)
    A.mem("str", 1, SPR, FR["th_len"])
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
    A.bl("rt_th_detect")                       # SASTHRDS: checked, stacks made, BEFORE the input (yantra's order)
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
    A.cbz(0, "rt_s5v")
    A.mov(19, 0)
    A.movi(0, 0x0043_4752_4153_4153)
    A.bl("rt_find_tag")
    A.cbz(0, "rt_s5p")
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
    # the args report (success line too) follows the replay line: printed at rt_s5r
    A.b_("rt_s5")
    # a tag missing: yantra-run's refusal line, and the run goes on; a tag twice: not native
    A.label("rt_s5p")
    A.cmp_i(12, 1)
    A.bcond(HI, "rt_argsdup")
    A.b_("rt_s5")
    A.label("rt_s5v")
    A.cmp_i(12, 1)
    A.bcond(HI, "rt_argsdup")
    A.label("rt_s5")
    # the event slot: the SASEVENT tag's address if it occurs exactly once (0 otherwise)
    A.movi(0, EVENT_TAG)
    A.bl("rt_find_tag")
    A.mem("str", 0, SPR, FR["ev_slot"])
    # a threaded image was checked by rt_th_detect (runtime_threads_a64): it runs natively, or not at all
    # an --events replay names its slot, or is refused with no unique tag (see x86-64)
    A.mem("ldr", 9, SPR, FR["rep_fd"])
    A.cbz(9, "rt_s5r")
    A.mem("ldr", 9, SPR, FR["ev_slot"])
    A.cbz(9, "rt_rep_notag")
    A.bl("m_reptag")
    A.mem("ldr", 9, SPR, FR["ev_slot"])
    hbase(10)
    A.rrr("sub", 9, 9, 10)
    A.bl("rt_putu")
    A.bl("m_nl")
    A.label("rt_s5r")
    # a missing argument tag's refusal line follows the replay line (see x86-64)
    A.movi(0, 0x0056_4752_4153_4153)
    A.bl("rt_find_tag")
    A.cbnz(0, "rt_s5a")
    A.bl("m_noargv")
    A.b_("rt_s5b")
    A.label("rt_s5a")
    A.mov(19, 0)
    A.movi(0, 0x0043_4752_4153_4153)
    A.bl("rt_find_tag")
    A.cbnz(0, "rt_s5c")
    A.bl("m_noargc")
    A.b_("rt_s5b")
    A.label("rt_s5c")
    A.mov(20, 0)
    # both tags: yantra-run's line: args: N handed over, tags at [argv, argc], run at 0x…
    A.bl("m_args1")
    A.mem("ldr", 9, SPR, FR["argc"])
    A.bl("rt_putu")
    A.bl("m_args2")
    hbase(11)
    A.rrr("sub", 9, 19, 11)
    A.bl("rt_puthexn")
    A.bl("m_args3")
    hbase(11)
    A.rrr("sub", 9, 20, 11)
    A.bl("rt_puthexn")
    A.bl("m_args4")
    A.mem("ldr", 9, 19, 8)                     # the guest address of the run
    A.bl("rt_puthex")
    A.bl("m_nl")
    A.label("rt_s5b")
    if mac != "win":
        A.bl("rt_sk_init")                     # YANTRA_LISTEN: the socket's listener (runtime_sock_a64)
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
    # the step ceiling: yantra's DEFAULT_STEPS, or YANTRA_STEPS when it is a decimal usize (as yantra-run reads it)
    A.movi(9, 1000000)
    A.mem("str", 9, SPR, FR["steps"])
    adr(0, "env_steps")
    A.bl("rt_getenv")
    A.cbz(0, "rt_stp")
    A.bl("rt_atou_ram")
    A.cbnz(1, "rt_stp")
    A.mem("str", 0, SPR, FR["steps"])
    A.label("rt_stp")
    A.mem("ldr", 9, SPR, FR["steps"])
    A.mem("str", 9, SPR, FR["steps0"])   # the effective limit, for the StepLimit report
    # the guest: every register zero, pc = entry; a threaded image's first decision picks thread 0
    A.mem("ldr", 9, SPR, FR["th_n"])
    A.cbnz(9, "rt_th_next")
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
    # a target in the middle of a block runs in the SAFE region (G_), which charges word by word
    A.label("rt_ind_go")
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
    A.memr("ldrb", T1, T0, T2)                 # a newline flushes (yantra-run's stdout is line-buffered)
    A.add_i(T2, T2, 1)
    A.mem("str", T2, SPR, FR["outlen"])
    A.cmp_i(T1, 10)
    A.bcond(EQ, "rt_st_f")
    A.cmp_i12(T2, 16)                          # 65536
    A.bcond(LO, "rt_st1")
    A.label("rt_st_f")
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
    if mac != "win":
        if fj:                                 # a worker: the socket is a main-thread device
            fj_base_a64(A, fj)
            A.mem("ldr", 18, 18, FJ64["fj_worker"])
            A.cbnz(18, "rt_st3s")
        A.movi(T2, SOCK)                       # SOCK..SOCK+16: the socket (runtime_sock_a64)
        A.rrr("sub", T2, T0, T2)
        A.cmp_i(T2, 16)
        A.bcond(LO, "rt_sk_st")
        if fj:
            A.label("rt_st3s")
    if scanp:                                  # slot 0: the GPU (scanout_dev.py), at 0x1000_1000 + 0x1000
        A.movi(T2, 0x1000_1000)
        A.rrr("sub", T2, T0, T2)
        A.cmp_i12(T2, 1)
        A.bcond(LO, "rt_sc_st")
    A.movi(T2, FINISHER)
    A.cmp(T0, T2)
    A.bcond(NE, "rt_st_fjdev" if fj else "rt_st_win")
    A.mem("ldr", T2, SPR, FR["th_n"])
    A.cbnz(T2, "rt_th_fin")                    # a thread's finisher ends the thread (runtime_threads_a64)
    # FINISHER: flush, print yantra's halt line, exit 0 on 0x5555, else 1
    A.label("rt_fin_print")
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
    # 0x3333 with a zero status (value >> 16 == 0) is yantra's `status: Some(0)`: exit 0 (vd0013)
    A.lsr_i(10, 9, 16)
    A.cmp_i(10, 0)
    A.bcond(EQ, "rt_fin_ok")
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
    if scanp:
        import scanout_dev as sd
        sd.emit_a64(A, scanp, base, FR, {"eq": EQ, "ne": NE, "ltu": LO, "geu": HS, "gtu": HI, "leu": LS})
    runtime_window_a64(A, FR, base, prot_size, mac, sysc, wcall, adr)
    runtime_spawn_a64(A, FR, base, mac, sysc, wcall, adr)
    if mac != "win":
        strings += runtime_sock_a64(A, FR, base, darwin, wm)
    # a store that missed RAM and is not UART/FINISHER: T0 = addr
    A.label("rt_beyond")
    A.mem("ldr", T2, SPR, FR["th_n"])
    A.cbnz(T2, "rt_th_beyond")                 # a thread's stack store (runtime_threads_a64)
    A.label("rt_beyond0")
    A.mem("str", T0, SPR, FR["fault_addr"])
    A.bl("rt_flush")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.movi(10, 1 << 31)
    A.cmp(9, 10)
    A.bcond(LO, "rt_store_bad")                # below 2^31, no device
    A.mem("ldr", 10, SPR, FR["code_end"])
    A.cmp(9, 10)
    A.bcond(LO, "rt_store_code")               # in the code span
    A.mem("ldr", 9, SPR, FR["th_slim"])        # threaded: stores stop above the stacks
    A.cbnz(9, "rt_br_tail")
    A.mem("ldr", 9, SPR, FR["ram"])            # yantra: BeyondRam { pc, addr, ram: the writable span }
    A.b_("rt_br_tail")
    A.label("rt_store_bad")
    # yantra's store answers BadAccess below base except at its devices (runtime(): SLOT1, STORE_DEVICES)
    A.movi(10, SLOT1[0])
    A.rrr("sub", 10, 9, 10)
    A.movi(11, SLOT1[1])
    A.cmp(10, 11)
    A.bcond(LO, "rt_slot1")
    for lo, n in STORE_DEVICES:
        A.movi(10, lo)
        A.rrr("sub", 10, 9, 10)
        A.movi(11, n)
        A.cmp(10, 11)
        A.bcond(LO, "rt_store_dev")
    A.b_("rt_bada")
    A.label("rt_store_dev")
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
    # a load that missed RAM: T0 = addr; bl'd from the cold stub, which loads through T0 at its
    # B label if this returns (only the socket's RX does)
    A.label("rt_load_slow")
    if mac != "win":
        if fj:                                 # a worker: the socket is a main-thread device
            fj_base_a64(A, fj)
            A.mem("ldr", 18, 18, FJ64["fj_worker"])
            A.cbnz(18, "rt_ls_nosk")
        A.movi(T1, SOCK)                       # SOCK..SOCK+16: the socket (runtime_sock_a64)
        A.rrr("sub", T1, T0, T1)
        A.cmp_i(T1, 16)
        A.bcond(LO, "rt_sk_ld")
        if fj:
            A.label("rt_ls_nosk")
    if scanp:
        A.movi(T1, 0x1000_1000)                # slot 0: the GPU (scanout_dev.py)
        A.rrr("sub", T1, T0, T1)
        A.cmp_i12(T1, 1)
        A.bcond(LO, "rt_sc_ld")
    A.mem("str", T0, SPR, FR["fault_addr"])
    A.bl("rt_flush")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.movi(10, 1 << 31)
    A.cmp(9, 10)
    A.bcond(LO, "rt_load_bad")                 # below 2^31 (a device): not in milestone 1
    A.mem("ldr", 9, SPR, FR["top"])            # yantra: BeyondRam { pc, addr, ram: the loads' limit }
    A.label("rt_br_tail")                      # x9 = the limit; fault_off, fault_addr: the site and the address
    A.mem("str", 9, SPR, FR["fault_word"])
    A.bl("m_br1")
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    A.bl("m_ba2")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.bl("rt_putu")
    A.bl("m_br2")
    A.mem("ldr", 9, SPR, FR["fault_word"])
    A.bl("rt_putu")
    A.bl("m_un4")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_load_bad")
    # yantra's load answers BadAccess below base except at its load devices (runtime(): SLOT1, LOAD_DEVICES)
    A.movi(10, SLOT1[0])
    A.rrr("sub", 10, 9, 10)
    A.movi(11, SLOT1[1])
    A.cmp(10, 11)
    A.bcond(LO, "rt_slot1")
    for lo, n in LOAD_DEVICES:
        A.movi(10, lo)
        A.rrr("sub", 10, 9, 10)
        A.movi(11, n)
        A.cmp(10, 11)
        A.bcond(LO, "rt_load_dev")
    A.label("rt_bada")                         # yantra's "halt: BadAccess { pc: P, addr: A }"
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
    A.label("rt_slot1")                        # yantra's "halt: Device { pc: P, addr: A, why: SLOT1_WHY }"
    A.bl("m_dev1")
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    A.bl("m_bada2")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.bl("rt_putu")
    A.bl("m_dev3")
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
    # ---- yantra's halts for a word it does not execute (see the x86-64 runtime)
    A.label("rt_unimpl")
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.mem("str", 9, SPR, FR["fault_addr"])
    A.label("rt_unimpl_at")                    # fault_addr = the pc, fault_word's low half = the word
    A.bl("rt_flush")
    A.bl("m_un1")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.bl("rt_putu")
    A.bl("m_un2")
    A.mem("ldrw", 9, SPR, FR["fault_word"])
    A.bl("rt_putu")
    A.bl("m_un3")
    A.mem("ldrw", 9, SPR, FR["fault_word"])
    A.ubfm(9, 9, 0, 6)                         # & 0x7F
    A.bl("rt_putu")
    A.bl("m_un4")
    A.movz(0, 1)
    A.b_("rt_exit")
    # ---- the step ceiling (L18/H1): fault_off = the offset of the word the limit is met at (the first not executed)
    A.label("rt_stepcross")
    if fj:
        fj_base_a64(A, fj)
        A.mem("ldr", 18, 18, FJ64["fj_worker"])
        A.cbnz(18, "rt_fj_stepw")
    A.bl("rt_flush")
    A.bl("m_steplim")
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    A.bl("m_un4")
    A.bl("m_steps1")
    A.mem("ldr", 9, SPR, FR["steps0"])
    A.bl("rt_putu")
    A.bl("m_steps1b")
    A.bl("m_stop1")
    A.mem("ldr", 9, SPR, FR["steps0"])
    A.bl("rt_putu")
    A.bl("m_stop1b")
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_puthex")
    A.bl("m_stop2")
    A.mem("ldr", 9, SPR, FR["steps0"])
    A.bl("rt_putu")
    A.bl("m_stop2b")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_break")                        # ebreak, no stvec: Breakpoint { pc }
    A.bl("rt_flush")
    A.bl("m_brk")
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    A.bl("m_un4")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_ecall")                        # an SBI call but putchar / set_timer: fault_addr = eid, fault_word = fid
    A.bl("rt_flush")
    A.mem("ldr", T0, SPR, FR["fault_addr"])
    A.cmp_i(T0, 8)
    A.bcond(NE, "rt_ec_sbi")
    A.bl("m_shut")
    A.b_("rt_ec_pc")
    A.label("rt_ec_sbi")
    A.bl("m_sbi1")
    A.label("rt_ec_pc")
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    A.mem("ldr", T0, SPR, FR["fault_addr"])
    A.cmp_i(T0, 8)
    A.bcond(EQ, "rt_ec_end")
    A.bl("m_sbi2")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.bl("rt_putu")
    A.bl("m_sbi3")
    A.mem("ldr", 9, SPR, FR["fault_word"])
    A.bl("rt_putu")
    A.label("rt_ec_end")
    A.bl("m_un4")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_spin")                         # `jal rd, .`: SpinForever { pc }, exit 1
    A.bl("rt_flush")
    A.bl("m_spin")
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    A.bl("m_un4")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_sret")                         # sret: to sepc, which is 0 in every native run
    A.movz(T0, 0)
    # ---- a jump to a pc with no translation: T0 = the target; yantra fetches it (see the x86-64 runtime)
    A.label("rt_bad_target")
    if fj:
        # a worker's body returned to the sentinel return address: the next index
        fj_base_a64(A, fj)
        A.mem("ldr", 18, 18, FJ64["fj_worker"])
        A.cbz(18, "rt_bt_main")
        A.movi(T1, FJ_SENTINEL)
        A.cmp(T0, T1)
        A.bcond(EQ, "rt_fj_ret")
        A.label("rt_bt_main")
    if vd:
        A.mem("ldr", 9, SPR, VD_ST)               # yantra counts the fetch that faults
        A.add_i(9, 9, 1)
        A.mem("str", 9, SPR, VD_ST)
    A.mem("str", T0, SPR, FR["fault_addr"])
    A.bl("rt_flush")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.movi(10, base)
    A.cmp(9, 10)
    A.bcond(LO, "rt_bt_low")
    A.rrr("sub", 11, 9, 10)
    A.mem("ldr", 12, SPR, FR["top"])
    A.add_i(12, 12, -4)
    A.cmp(11, 12)
    A.bcond(HI, "rt_bt_beyond")
    if mac:
        A.mem("ldr", 10, SPR, FR["delta"])
        A.rrr("add", 9, 9, 10)
    A.mem("ldrw", 11, 9, 0)                    # the word yantra fetches
    A.mem("strw", 11, SPR, FR["fault_word"])
    A.ubfm(12, 11, 0, 1)
    A.cmp_i(12, 3)
    A.bcond(NE, "rt_unimpl_at")                # a compressed encoding
    A.ubfm(12, 11, 2, 6)                       # (word >> 2) & 31
    A.movi(10, YANTRA_OPS)
    A.dp2("lsrv", 10, 10, 12)
    A.tst_l(10, LOG_1)
    A.bcond(EQ, "rt_unimpl_at")                # an opcode yantra does not execute
    A.label("rt_bt_refuse")
    A.bl("m_jump1")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.bl("rt_puthex")
    A.bl("m_jump2")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_bt_low")
    A.movi(10, SOCK)
    A.rrr("sub", 11, 9, 10)
    A.cmp_i(11, SOCK_LEN)
    A.bcond(LO, "rt_bt_refuse")
    A.movi(10, VIRTIO_LO)
    A.rrr("sub", 11, 9, 10)
    A.movi(10, VIRTIO_LEN)
    A.cmp(11, 10)
    A.bcond(LO, "rt_bt_refuse")
    A.bl("m_ba1")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.bl("rt_putu")
    A.bl("m_ba2")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.bl("rt_putu")
    A.bl("m_un4")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_bt_beyond")
    A.bl("m_br1")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.bl("rt_putu")
    A.bl("m_ba2")
    A.mem("ldr", 9, SPR, FR["fault_addr"])
    A.bl("rt_putu")
    A.bl("m_br2")
    A.mem("ldr", 9, SPR, FR["top"])
    A.bl("rt_putu")
    A.bl("m_un4")
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
    A.label("rt_no_rep")
    A.bl("m_norep")
    A.movz(0, 2)
    A.b_("rt_exit")
    if mac == "win":
        A.label("rt_win_ev")
        A.bl("m_winev")
        A.movz(0, 2)
        A.b_("rt_exit")
    A.label("rt_rep_notag")
    A.bl("m_repnotag")
    A.movz(0, 2)
    A.b_("rt_exit")
    A.label("rt_threaded")
    A.bl("m_threaded")
    A.movz(0, 2)
    A.b_("rt_exit")
    A.label("rt_argsdup")
    A.bl("m_argsdup")
    A.movz(0, 2)
    A.b_("rt_exit")
    A.label("rt_wt_notag")
    A.bl("rt_flush")
    A.bl("m_waitnotag")
    A.movz(0, 2)
    A.b_("rt_exit")
    A.label("rt_wt_halt")                      # yantra-run with no event source: halt Wait, exit 75
    A.bl("rt_flush")
    A.bl("m_wait")
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    A.bl("m_un4")
    A.movz(0, 75)
    A.b_("rt_exit")
    A.label("rt_no_tag")
    A.bl("m_notag")
    A.movz(0, 1 if mac == "win" else 2)
    A.label("rt_exit")
    A.mem("ldr", T0, SPR, FR["th_cap"])
    A.cbnz(T0, "rt_th_exit")                   # a thread's halt: yantra's thread line first
    if mac != "win":
        A.mem("ldr", T0, SPR, FR["sk"])
        A.cbz(T0, "rt_exit2")
        A.b_("rt_sk_exit")                     # the socket's account, after the halt line
        A.label("rt_exit2")
    if wm:
        A.mem("str", 0, SPR, FR["wt_tmp"])
        A.bl("rt_wm_report")                   # YANTRA_WATERMARK: the store high-water line, after the socket's
        A.mem("ldr", 0, SPR, FR["wt_tmp"])
        A.label("rt_exit_nw")
    if fj:
        fj_base_a64(A, fj)
        A.mem("ldr", 9, 18, FJ64["fj_att"])
        A.cbz(9, "rt_exit_fa")
        A.mem("str", 0, 18, FJ64["fj_ec"])
        A.bl("rt_fja_rep")                     # S1: the fork-join attestation line, into the file only
        fj_base_a64(A, fj)
        A.mem("ldr", 0, 18, FJ64["fj_ec"])
        A.label("rt_exit_fa")
    if vd:
        A.b_("rt_vd_fin")                      # L33: the captured stderr, the `steps:` line, the verdict file; exits
    sysc(93, 1)
    if wm:
        # ---- ram: high water N of M octets, as yantra-run, when YANTRA_WATERMARK is set
        A.label("rt_wm_report")
        A.mem("str", 30, SPR, FR["lr_cold"])
        adr(0, "env_wm")
        A.bl("rt_getenv")
        A.cbz(0, "rt_wm_no")
        A.bl("m_wm1")
        A.mem("ldr", 9, SPR, WM_HW)             # the absolute end address, or 0: no store
        A.cbz(9, "rt_wm_z")
        A.movi(10, base)
        A.rrr("sub", 9, 9, 10)
        A.label("rt_wm_z")
        A.bl("rt_putu")
        A.bl("m_wm2")
        A.mem("ldr", 9, SPR, FR["ram"])
        A.bl("rt_putu")
        A.bl("m_wm3")
        A.label("rt_wm_no")
        A.mem("ldr", 30, SPR, FR["lr_cold"])
        A.ret()

    if vd:
        # ---- L33 YANTRA_VERDICT (see the x86-64 runtime): entered by a branch at the exit funnel, x0 = the exit code.
        # x19 = the exit code, x20 = the end of the halt line, x21 = the end of the capture, x22 = the output cursor,
        # x23 = the newline of the halt line, x24 = the verdict path, x25 = a kept pointer, x26 = errno
        nlit = [0]

        def lit(text):
            nlit[0] += 1
            lab = "vd_l%d" % nlit[0]
            strings.append((lab, text))
            adr(1, lab)
            A.movi(2, len(text))
            A.bl("rt_vd_cpy")

        def wr(buf_reg_setup, end_reg, fd=None):    # write(fd or 2, buf, end - buf)
            buf_reg_setup()
            A.rrr("sub", 2, end_reg, 1)
            if fd is None:
                A.movz(0, 2)
            else:
                A.mov(0, fd)
            A.movz(8, 64)
            A.svc()

        def vd_buf1():
            adr(1, "vd_buf")

        def vd_out1():
            adr(1, "vd_out")

        def at_text(extra=0):
            adr(1, "vd_buf")
            A.add_i(1, 1, 6 + extra)

        A.label("rt_vd_fin")
        A.mov(19, 0)
        A.mem("ldr", 9, SPR, VD_FD)
        A.cbz(9, "rt_vd_exit")
        A.mov(0, 9)
        adr(1, "vd_buf")
        A.movi(2, VD_BUF)
        A.movz(3, 0)
        A.movz(8, 67)
        A.svc()                                  # pread64(fd, buf, size, 0)
        A.cmp_i(0, 0)
        A.bcond(GE, "rt_vd_f1")
        A.movz(0, 0)
        A.label("rt_vd_f1")
        adr(9, "vd_buf")
        A.rrr("add", 21, 9, 0)
        # leading `args:` lines are replayed to the real stderr and skipped (see x86-64)
        A.movz(24, 0)
        A.label("rt_vd_sk")
        adr(1, "vd_buf")
        A.rrr("sub", 9, 21, 1)
        A.cmp_i(9, 5)
        A.bcond(LT, "rt_vd_sk9")
        A.mem("ldrw", 10, 1, 0)
        A.movi(11, 0x73677261)
        A.cmp(10, 11)
        A.bcond(NE, "rt_vd_sk9")
        A.mem("ldrb", 10, 1, 4)
        A.cmp_i(10, 0x3a)
        A.bcond(NE, "rt_vd_sk9")
        A.mov(23, 1)
        A.label("rt_vd_sk1")
        A.cmp(23, 21)
        A.bcond(HS, "rt_vd_sk9")
        A.mem("ldrb", 9, 23, 0)
        A.cmp_i(9, 10)
        A.bcond(EQ, "rt_vd_sk2")
        A.add_i(23, 23, 1)
        A.b_("rt_vd_sk1")
        A.label("rt_vd_sk2")
        A.add_i(23, 23, 1)
        adr(1, "vd_buf")
        A.rrr("sub", 2, 23, 1)
        A.rrr("add", 24, 24, 2)
        A.mem("ldr", 0, SPR, VD_SV)
        A.movz(8, 64)
        A.svc()                                  # write(real stderr, the args line)
        A.mem("ldr", 0, SPR, VD_FD)
        adr(1, "vd_buf")
        A.movi(2, VD_BUF)
        A.mov(3, 24)
        A.movz(8, 67)
        A.svc()                                  # pread64(fd, buf, size, past the args lines)
        A.cmp_i(0, 0)
        A.bcond(GE, "rt_vd_sk3")
        A.movz(0, 0)
        A.label("rt_vd_sk3")
        adr(9, "vd_buf")
        A.rrr("add", 21, 9, 0)
        A.b_("rt_vd_sk")
        A.label("rt_vd_sk9")
        A.mem("ldr", 0, SPR, VD_FD)
        A.movz(8, 57)
        A.svc()                                  # close the capture
        A.mem("ldr", 0, SPR, VD_SV)
        A.movz(1, 2)
        A.movz(2, 0)
        A.movz(8, 24)
        A.svc()                                  # dup3(real stderr, 2, 0)
        A.mem("ldr", 0, SPR, VD_SV)
        A.movz(8, 57)
        A.svc()
        A.movz(9, 0)
        A.mem("str", 9, SPR, VD_FD)
        adr(1, "vd_buf")
        A.rrr("sub", 9, 21, 1)
        A.cmp_i(9, 6)
        A.bcond(LT, "rt_vd_raw")
        A.mem("ldrw", 10, 1, 0)
        A.movi(11, 0x746c6168)
        A.cmp(10, 11)
        A.bcond(NE, "rt_vd_raw")
        A.mem("ldrh", 10, 1, 4)
        A.movi(11, 0x203a)
        A.cmp(10, 11)
        A.bcond(NE, "rt_vd_raw")
        A.add_i(1, 1, 6)
        A.mov(23, 1)
        A.label("rt_vd_nl")
        A.cmp(23, 21)
        A.bcond(HS, "rt_vd_nl2")
        A.mem("ldrb", 9, 23, 0)
        A.cmp_i(9, 10)
        A.bcond(EQ, "rt_vd_nl2")
        A.add_i(23, 23, 1)
        A.b_("rt_vd_nl")
        A.label("rt_vd_nl2")
        A.add_i(20, 23, 1)
        A.cmp(20, 21)
        A.bcond(LS, "rt_vd_nl3")
        A.mov(20, 21)
        A.label("rt_vd_nl3")
        wr(vd_buf1, 20)                          # the halt line, as it stands
        adr(22, "vd_out")
        for name, text, jl in (("fin", b"Finisher { value: ", "rt_vd_isfin"), ("wt", b"Wait", None),
                               ("sl", b"StepLimit", None), ("sp", b"SpinForever", None)):
            at_text()
            adr(3, "vd_p" + name)
            A.movi(2, len(text))
            A.bl("rt_vd_pfx")
            A.cbnz(0, jl or "rt_vd_is" + name)
            strings.append(("vd_p" + name, text))
        lit(b"fault ")
        at_text()
        A.rrr("sub", 2, 23, 1)
        A.bl("rt_vd_cpy")
        lit(b"\n")
        A.b_("rt_vd_steps")
        for name, word in (("wt", b"wait\n"), ("sl", b"steplimit\n"), ("sp", b"spin\n")):
            A.label("rt_vd_is" + name)
            lit(word)
            A.b_("rt_vd_steps")
        A.label("rt_vd_isfin")
        lit(b"finisher ")
        at_text(18)
        A.bl("rt_vd_dig")
        A.mov(25, 1)
        lit(b" ")
        A.add_i(1, 25, 10)                       # `, status: `
        A.mem("ldrb", 9, 1, 0)
        A.cmp_i(9, 78)                           # N: None
        A.bcond(NE, "rt_vd_some")
        lit(b"-")
        A.b_("rt_vd_fnl")
        A.label("rt_vd_some")
        A.add_i(1, 1, 5)                         # `Some(`
        A.bl("rt_vd_dig")
        A.label("rt_vd_fnl")
        lit(b"\n")
        A.label("rt_vd_steps")
        lit(b"steps ")
        A.mem("ldr", 0, SPR, VD_ST)
        A.bl("rt_vd_dec")
        lit(b"\n")
        adr(0, "env_vd")
        A.bl("rt_getenv")
        A.cbz(0, "rt_vd_sl")
        A.mov(24, 0)
        A.movi(0, -100)                          # AT_FDCWD
        A.mov(1, 24)
        A.movi(2, 0x241)                         # O_WRONLY | O_CREAT | O_TRUNC
        A.movi(3, 438)                           # 0666
        A.movz(8, 56)
        A.svc()                                  # openat
        A.cmp_i(0, 0)
        A.bcond(LT, "rt_vd_err")
        A.mov(25, 0)
        wr(vd_out1, 22, 25)
        A.mov(26, 0)
        A.mov(0, 25)
        A.movz(8, 57)
        A.svc()
        A.mov(0, 26)
        A.cmp_i(0, 0)
        A.bcond(GE, "rt_vd_sl")
        A.label("rt_vd_err")                     # `verdict: <path>: <strerror> (os error N)`, then exit 1 at once
        A.rrr("sub", 26, ZR, 0)
        adr(22, "vd_out")
        lit(b"verdict: ")
        A.mov(1, 24)
        A.bl("rt_vd_cz")
        lit(b": ")
        for no, text in ((2, b"No such file or directory"), (13, b"Permission denied"), (21, b"Is a directory"),
                         (20, b"Not a directory"), (30, b"Read-only file system"),
                         (28, b"No space left on device"), (36, b"File name too long")):
            A.cmp_i(26, no)
            A.bcond(NE, "rt_vd_en%d" % no)
            lit(text)
            A.b_("rt_vd_e2")
            A.label("rt_vd_en%d" % no)
        lit(b"Unknown error ")
        A.mov(0, 26)
        A.bl("rt_vd_dec")
        A.label("rt_vd_e2")
        lit(b" (os error ")
        A.mov(0, 26)
        A.bl("rt_vd_dec")
        lit(b")\n")
        wr(vd_out1, 22)
        A.movz(19, 1)
        A.b_("rt_vd_exit")
        A.label("rt_vd_sl")                      # steps: N executed instructions
        adr(22, "vd_out")
        lit(b"steps: ")
        A.mem("ldr", 0, SPR, VD_ST)
        A.bl("rt_vd_dec")
        lit(b" executed instructions\n")
        wr(vd_out1, 22)
        A.movz(0, 2)                             # whatever followed the halt line
        A.mov(1, 20)
        A.rrr("sub", 2, 21, 20)
        A.movz(8, 64)
        A.svc()
        A.b_("rt_vd_exit")
        A.label("rt_vd_raw")
        wr(vd_buf1, 21)
        A.label("rt_vd_exit")
        A.mov(0, 19)
        sysc(93, 1)
        # x1 = source, x2 = octets, x22 = destination (advanced)
        A.label("rt_vd_cpy")
        A.cbz(2, "rt_vd_cpy2")
        A.mem("ldrb", 9, 1, 0)
        A.mem("strb", 9, 22, 0)
        A.add_i(1, 1, 1)
        A.add_i(22, 22, 1)
        A.add_i(2, 2, -1)
        A.b_("rt_vd_cpy")
        A.label("rt_vd_cpy2")
        A.ret()
        # x1 = a zero-ended string, copied to x22
        A.label("rt_vd_cz")
        A.mem("ldrb", 9, 1, 0)
        A.cbz(9, "rt_vd_cz2")
        A.mem("strb", 9, 22, 0)
        A.add_i(1, 1, 1)
        A.add_i(22, 22, 1)
        A.b_("rt_vd_cz")
        A.label("rt_vd_cz2")
        A.ret()
        # x1 = text, x3 = literal, x2 = its length: x0 = 1 when the text begins with it
        A.label("rt_vd_pfx")
        A.mov(9, 1)
        A.mov(10, 3)
        A.mov(11, 2)
        A.label("rt_vd_pf1")
        A.cbz(11, "rt_vd_pf2")
        A.mem("ldrb", 12, 9, 0)
        A.mem("ldrb", 13, 10, 0)
        A.cmp(12, 13)
        A.bcond(NE, "rt_vd_pf3")
        A.add_i(9, 9, 1)
        A.add_i(10, 10, 1)
        A.add_i(11, 11, -1)
        A.b_("rt_vd_pf1")
        A.label("rt_vd_pf2")
        A.movz(0, 1)
        A.ret()
        A.label("rt_vd_pf3")
        A.movz(0, 0)
        A.ret()
        # the decimal digits at x1 to x22 (x1 advanced)
        A.label("rt_vd_dig")
        A.mem("ldrb", 9, 1, 0)
        A.add_i(10, 9, -48)
        A.cmp_i(10, 9)
        A.bcond(HI, "rt_vd_dig2")
        A.mem("strb", 9, 22, 0)
        A.add_i(1, 1, 1)
        A.add_i(22, 22, 1)
        A.b_("rt_vd_dig")
        A.label("rt_vd_dig2")
        A.ret()
        # x0 as decimal digits to x22 (advanced)
        A.label("rt_vd_dec")
        A.add_i(1, SPR, FR["num"] + 31)
        A.movz(10, 10)
        A.label("rt_vd_dc1")
        A.dp2("udiv", 11, 0, 10)
        A.msub(12, 11, 10, 0)
        A.add_i(12, 12, 48)
        A.add_i(1, 1, -1)
        A.mem("strb", 12, 1, 0)
        A.mov(0, 11)
        A.cbnz(0, "rt_vd_dc1")
        A.add_i(2, SPR, FR["num"] + 31)
        A.rrr("sub", 2, 2, 1)
        A.b_("rt_vd_cpy")
        # StepLimit: the ceiling was passed at pc = T0; yantra's clock reads the limit
        A.label("rt_vd_slim")
        A.mem("ldr", 9, SPR, VD_LIM)
        A.mem("str", 9, SPR, VD_ST)
        A.mem("str", T0, SPR, FR["fault_addr"])
        A.bl("rt_flush")
        A.bl("m_vsl")
        A.mem("ldr", 9, SPR, FR["fault_addr"])
        A.bl("rt_putu")
        A.bl("m_un4")
        A.movz(0, 1)
        A.b_("rt_exit")
        # SpinForever: a `jal` to its own pc (fault_off names it): exit 1
        A.label("rt_vd_spin")
        A.bl("rt_flush")
        A.bl("m_vsp")
        A.mem("ldr", 9, SPR, FR["fault_off"])
        A.movi(10, base)
        A.rrr("add", 9, 9, 10)
        A.bl("rt_putu")
        A.bl("m_un4")
        A.movz(0, 1)
        A.b_("rt_exit")

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

    # ---- next_rec: the next record of an --events replay (see x86-64): x0 = the kind (0 value,
    # 1 `@N`, 2 spent), x1 = the value. Clobbers x2, x8, x16 and `num`; x30 kept in wt_tmp
    A.label("rt_next_rec")
    A.mem("str", 30, SPR, FR["wt_tmp"])
    A.mem("ldr", 0, SPR, FR["rep_fd"])
    A.add_i(0, 0, -1)
    A.add_i(1, SPR, FR["num"])
    A.movz(2, 16)
    sysc(63, 3)
    A.mem("ldr", 30, SPR, FR["wt_tmp"])
    A.cmp_i(0, 16)
    A.bcond(NE, "rt_nr1")
    A.mem("ldr", 0, SPR, FR["num"])
    A.mem("ldr", 1, SPR, FR["num"] + 8)
    A.ret()
    A.label("rt_nr1")
    A.movz(0, 2)
    A.ret()

    # ---- find_tag: x0 = the 8-octet tag; answer x0 = its address if it occurs EXACTLY
    # once in the file-backed image, else 0 (yantra scans every offset)
    A.label("rt_find_tag")
    hbase(15)
    A.label("rt_find_tag_at")                  # x15 = where the image's octets start, i.e. the host address of guest `base` (rt_th_pre: the file's)
    # only the file bytes of PF_W PT_LOADs (wr_tab) are scanned; each hit lies wholly inside one of them
    adr(16, "wr_tab")
    A.mem("ldr", 17, 16, 0)
    A.add_i(16, 16, 8)
    A.movz(11, 0)                              # hit
    A.movz(12, 0)                              # count
    A.label("rt_ft0")
    A.cbz(17, "rt_ft3")
    A.mov(9, 15)
    A.movi(10, base)
    A.rrr("sub", 9, 9, 10)                     # host - guest
    A.mem("ldr", 10, 16, 0)
    A.rrr("add", 9, 9, 10)                     # the segment's host start
    A.mem("ldr", 10, 16, 8)
    A.rrr("add", 10, 10, 9)
    A.add_i(10, 10, -7)                        # one past the last start of a whole word
    A.label("rt_ft1")
    A.cmp(9, 10)
    A.bcond(HS, "rt_ft5")
    A.mem("ldr", 13, 9, 0)
    A.cmp(0, 13)
    A.bcond(NE, "rt_ft2")
    A.mov(11, 9)
    A.add_i(12, 12, 1)
    A.label("rt_ft2")
    A.add_i(9, 9, 1)
    A.b_("rt_ft1")
    A.label("rt_ft5")
    A.add_i(16, 16, 16)
    A.add_i(17, 17, -1)
    A.b_("rt_ft0")
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
    if mac == "win":
        A.movz(0, 0)                           # never reached: Windows refuses a clock run at startup
    elif mac:
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
    A.mem("ldr", T0, SPR, FR["th_n"])
    A.cbnz(T0, "rt_th_wait")                   # a thread's wait is a switch (runtime_threads_a64)
    if mac != "win":
        A.mem("ldr", T0, SPR, FR["sk"])
        A.cbnz(T0, "rt_sk_wait")               # a socket run's wait (runtime_sock_a64)
    A.mem("ldr", T0, SPR, FR["ev_slot"])       # (a threaded image never runs: refused at startup)
    A.mem("ldr", T1, SPR, FR["rec_fd"])        # no YANTRA_RECORD_EVENTS: yantra halts here,
    A.cbz(T1, "rt_wt_halt")                    # tagged or not (yantra-run with no --events)
    A.cbz(T0, "rt_wt_notag")
    A.mem("ldr", 0, SPR, FR["rep_fd"])         # a replay: the next value record, or yantra's halt
    A.cbz(0, "rt_wt1")                         # at a wait the log has no record for
    A.bl("rt_next_rec")
    A.cbnz(0, "rt_wt_halt")
    A.mov(0, 1)
    A.b_("rt_wt6")
    A.label("rt_wt1")
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
    if mac == "win":
        pass                                   # never reached (see rt_clock)
    elif mac:
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
    A.label("rt_wt6")
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
    A.b_("rt_ewr")
    A.label("rt_puts")                         # x1, x2: octets to stderr (rt_csrh's text)
    A.movz(0, 2)
    sysc(64, 4)
    A.ret()

    # ---- puthex: x9 as 0x… hex (minimal digits) to stderr
    A.label("rt_puthexn")                      # the same, no 0x: yantra-run's {:x?}
    A.add_i(1, SPR, FR["num"] + 31)
    A.label("rt_pn1")
    A.ubfm(10, 9, 0, 3)
    A.cmp_i(10, 10)
    A.bcond(LO, "rt_pn2")
    A.add_i(10, 10, 39)
    A.label("rt_pn2")
    A.add_i(10, 10, 48)
    A.add_i(1, 1, -1)
    A.mem("strb", 10, 1, 0)
    A.lsr_i(9, 9, 4)
    A.cbnz(9, "rt_pn1")
    A.add_i(2, SPR, FR["num"] + 31)
    A.rrr("sub", 2, 2, 1)
    A.movz(0, 2)
    sysc(64, 4)
    A.ret()

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
    A.b_("rt_ewr")

    msg("m_fin1", b"halt: Finisher { value: ")
    msg("m_fin2", b", status: Some(")
    msg("m_fin3", b") }\n")
    msg("m_fin4", b", status: None }\n")
    msg("m_addr", b" addr ")
    msg("m_badstore", b"halt: BadAccess (a store to the socket or a virtio-mmio slot; not in milestone 1) at pc ")
    msg("m_codestore", b"native: refused a store into the code span at pc ")
    msg("m_codestore2", b" (yantra would allow it; self-modifying code is not translated)\n")
    msg("m_badload", b"halt: BadAccess (a load below 2^31; MMIO loads are not in milestone 1) at pc ")
    msg("m_bada1", b"halt: BadAccess { pc: ")
    msg("m_bada2", b", addr: ")
    msg("m_bada3", b" }\n")
    msg("m_dev1", b"halt: Device { pc: ")
    msg("m_dev3", b", why: \"" + SLOT1_WHY + b"\" }\n")
    msg("m_seg0", b": segment at ")
    msg("m_seg1", b" needs ")
    msg("m_seg2", b" bytes and RAM is ")
    msg("m_seg3", b" \xe2\x80\x94 raise it\n")
    msg("m_insn", b"native: not in milestone 1 (an instruction the translator does not lower) at pc ")
    msg("m_steplim", b"halt: StepLimit { pc: ")
    msg("m_steps1", b"steps: ")
    msg("m_steps1b", b" executed instructions\n")
    msg("m_stop1", b"yantra-run: STOPPED AT THE STEP LIMIT of ")
    msg("m_stop1b", b" instructions at pc ")
    msg("m_stop2", b" \xe2\x80\x94 the program did not finish. Raise YANTRA_STEPS. This is a budget exhaustion, NOT a program fault, and the ")
    msg("m_stop2b", b" instructions above are a LOWER BOUND on what it needed.\n")
    msg("m_jump1", b"native: jump to ")
    msg("m_jump2", b", outside the translated code: executing data is yantra-only (no native lowering, by ruling); run it under yantra-run\n")
    msg("m_un1", b"halt: Unimplemented { pc: ")
    msg("m_un2", b", word: ")
    msg("m_un3", b", opcode: ")
    msg("m_un4", b" }\n")
    msg("m_brk", b"halt: Breakpoint { pc: ")
    msg("m_shut", b"halt: Shutdown { pc: ")
    msg("m_sbi1", b"halt: Sbi { pc: ")
    msg("m_sbi2", b", eid: ")
    msg("m_sbi3", b", fid: ")
    msg("m_spin", b"halt: SpinForever { pc: ")
    msg("m_ba1", b"halt: BadAccess { pc: ")
    msg("m_ba2", b", addr: ")
    msg("m_br1", b"halt: BeyondRam { pc: ")
    msg("m_br2", b", ram: ")
    msg("m_wait", b"halt: Wait { pc: ")
    msg("m_nl", b"\n")
    msg("m_args1", b"args: ")
    msg("m_args2", b" handed over, tags at [")
    msg("m_args3", b", ")
    msg("m_args4", b"], run at ")
    msg("m_noargv", "args: not handed over \u2014 no argument interface in this image: the SASARGV tag (0x56475241534153) appears at no word. It was built without the argument globals \u2014 declare them, or do not pass arguments\n".encode())
    msg("m_noargc", "args: not handed over \u2014 no argument interface in this image: the SASARGC tag (0x43475241534153) appears at no word. It was built without the argument globals \u2014 declare them, or do not pass arguments\n".encode())
    msg("m_argsdup", b"native: not in milestone 1 (an argument tag occurs more than once; yantra-run names every offset; run it under yantra-run)\n")
    if vd:
        msg("m_vsl", b"halt: StepLimit { pc: ")
        msg("m_vsp", b"halt: SpinForever { pc: ")
    if wm:
        msg("m_wm1", b"ram: high water ")
        msg("m_wm2", b" of ")
        msg("m_wm3", b" octets\n")
    msg("m_noinput", b"native: YANTRA_INPUT could not be read\n")
    msg("m_noram", b"native: guest RAM could not be mapped at its own address\n")
    msg("m_norec", b"native: YANTRA_RECORD_EVENTS could not be opened for writing\n")
    msg("m_norep", b"native: YANTRA_REPLAY_EVENTS could not be opened for reading\n")
    if mac == "win":
        msg("m_winev", b"native: not in milestone 1 (YANTRA_RECORD_EVENTS or YANTRA_REPLAY_EVENTS on Windows: the clock is not ported there; run it under yantra-run)\n")
    msg("m_repnotag", b"native: not in milestone 1 (an --events replay (YANTRA_REPLAY_EVENTS) with the SASEVENT tag missing or duplicated; run it under yantra-run)\n")
    msg("m_reptag", b"native: replay: SASEVENT tag at RAM offset ")
    msg("m_waitnotag", b"native: not in milestone 1 (a recorded WAIT (YANTRA_RECORD_EVENTS) with the SASEVENT tag missing or duplicated; run it under yantra-run)\n")
    msg("m_notag", b"native: the image lacks an input tag, or has one twice\n")
    msg("m_threaded", b"native: not in milestone 1 (a threaded image this runtime does not run: its tags or startup are not the regular form, a socket run, Windows or fork-join; run it under yantra-run)\n")
    msg("m_entry_both", b"input: YANTRA_INPUT_ENTRY and YANTRA_INPUT_NAME are both set \xe2\x80\x94 the entry form builds the name itself (module NUL routine); set one\n")
    msg("m_entry1", b"input: YANTRA_INPUT_ENTRY ")
    msg("m_long1", b"input: YANTRA_INPUT_ENTRY is ")
    msg("m_long2", b" octets, over the 4096 limit\n")
    msg("m_bad8", b" is not UTF-8 \xe2\x80\x94 module and routine names are UTF-8\n")
    msg("m_noname", b"input: YANTRA_INPUT needs YANTRA_INPUT_NAME (or YANTRA_INPUT_ENTRY) \xe2\x80\x94 the module name is passed, never guessed\n")
    msg("m_entry2", b" is malformed \xe2\x80\x94 it must be \"<module> <routine>\": exactly one ASCII space, both halves non-empty and without whitespace\n")
    runtime_threads_a64(A, FR, base, entry, mac, sysc, adr, hbase, msg, strings, MAP, fp, fj)
    if fj:
        runtime_fj_a64(A, base, fj, msg, FR, MAP)
        strings.append(("env_fj", b"SASSEMBLY_NATIVE_THREADS=\0"))
        strings.append(("env_fjk", b"SASSEMBLY_NATIVE_FJ_MIN=\0"))
        strings.append(("env_fja", b"SASSEMBLY_NATIVE_FJATTEST=\0"))
        strings.append(("fja_0", b'fj_attest v1 p_req='))
        strings.append(("fja_1", b' fj_p_max='))
        strings.append(("fja_2", b' B='))
        strings.append(("fja_3", b' K='))
        strings.append(("fja_4", b' K0='))
        strings.append(("fja_5", b' spawned='))
        strings.append(("fja_6", b'\n'))
    if mac == "win":
        runtime_win_a64(A, FR)
        runtime_win_spawn_a64(A, FR)
    if wm:
        strings.append(("env_wm", b"YANTRA_WATERMARK=\0"))
    if vd:
        strings.append(("env_vd", b"YANTRA_VERDICT=\0"))
        strings.append(("vd_nm", b"vd\0"))
    for lab, text in strings + [("env_ram", b"YANTRA_RAM=\0"), ("env_steps", b"YANTRA_STEPS=\0"), ("env_input", b"YANTRA_INPUT=\0"),
                                ("env_grant", b"YANTRA_GRANT_ENV=\0"), ("env_files", b"YANTRA_FILES=\0"),
                                ("env_name", b"YANTRA_INPUT_NAME=\0"), ("env_trace", b"YANTRA_INPUT_TRACE=\0"),
                                ("env_entry", b"YANTRA_INPUT_ENTRY=\0"),
                                ("env_rec", b"YANTRA_RECORD_EVENTS=\0"), ("rec_hdr", REC_HEADER),
                                ("env_rep", b"YANTRA_REPLAY_EVENTS=\0")]:
        A.label(lab)
        A.raw(text)
    # the code points Rust's {:?} writes as \\u{..} that putq knows (frozen; (lo, hi) as u32
    # pairs, (0, 0) ends): C1, NBSP, soft hyphen, combining diacritics, the Devanagari marks,
    # Unicode spaces and format controls
    A.label("rt_esctab")
    A.raw(b"\x80\x00\x00\x00\xa0\x00\x00\x00\xad\x00\x00\x00\xad\x00\x00\x00\x00\x03\x00\x00\x6f\x03\x00\x00\x00\x09\x00\x00\x02\x09\x00\x00\x3a\x09\x00\x00\x3a\x09\x00\x00\x3c\x09\x00\x00\x3c\x09\x00\x00\x41\x09\x00\x00\x48\x09\x00\x00\x4d\x09\x00\x00\x4d\x09\x00\x00\x51\x09\x00\x00\x57\x09\x00\x00\x62\x09\x00\x00\x63\x09\x00\x00\x80\x16\x00\x00\x80\x16\x00\x00\x00\x20\x00\x00\x0f\x20\x00\x00\x28\x20\x00\x00\x2f\x20\x00\x00\x5f\x20\x00\x00\x6f\x20\x00\x00\x00\x30\x00\x00\x00\x30\x00\x00\xff\xfe\x00\x00\xff\xfe\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00")


def runtime_sock_a64(A, FR, base, mac=False, wm=False):
    """THE SOCKET DEVICE, aarch64 Linux, and Darwin when mac: runtime_sock's twin (the same states,
    words and order). Darwin: svc #0x80 with x16 (= T0) the BSD number, the carry an error (made
    -errno here), labels PC-relative, guest RAM at hbase (x4 = delta), poll for ppoll, open for
    openat, sin_len in sockaddr_in, SO_NOSIGPIPE for MSG_NOSIGNAL, ECONNRESET 54.
    rt_sk_ld / rt_sk_st are reached from the slow paths (T0 = addr, T1 = value; x30 = the cold
    stub's return); the RX load returns T0 = the latch's address. rt_sk_wait comes from rt_wait
    (x30 in lr_slow, x0 x1 x2 x8 in sv01/sv28) and leaves through rt_wt4. A path that runs while
    the guest lives saves x0..x13 in sk_sv first; T0..T2 are scratch."""
    strs = []
    SK, SV = FR["sk"], FR["sk_sv"]
    LR_REP, LR_HEAD, LR_ERR, X0_EXIT = SV + 8 * 16, SV + 8 * 17, SV + 8 * 18, SV + 8 * 15

    def ld(r, off):
        A.mem("ldr", r, SPR, SK + off)

    def st(r, off):
        A.mem("str", r, SPR, SK + off)

    def at(r, off):                          # r = sp + SK + off
        A.add_i(r, SPR, SK + off)

    def line_at(r):
        A.add_i12(r, SPR, FR["sk_line"] >> 12)
        A.add_i(r, r, FR["sk_line"] & 0xFFF)

    def lab_at(r, lab):                      # a label's address (Darwin: PC-relative, the image slides)
        (A.adrl if mac else A.movl)(r, lab)

    def say(text):
        lab = "sk_s%d" % len(strs)
        strs.append((lab, text))
        lab_at(1, lab)
        A.movi(2, len(text))
        A.movz(0, 2)
        sysn(64)

    def sysn(n):
        if mac:
            A.movz(16, SOCK_DARWIN_A64[n])
            A.svc80()
            A.bskip(LO, 2)                    # carry clear: the answer
            A.rrr("sub", 0, ZR, 0)            # carry set: x0 = errno, made -errno
        else:
            A.movz(8, n)
            A.svc()

    def code_word(r):                        # r = the guest's word at fault_off (T2 scratch)
        A.mem("ldr", r, SPR, FR["fault_off"])
        if mac:
            A.mem("ldr", T2, SPR, FR["hbase"])
        else:
            A.movi(T2, base)
        A.memr("ldrw", r, T2, r)

    def fd(r, off):
        ld(r, off)
        A.add_i(r, r, -1)

    def pc_to(r):
        A.mem("ldr", r, SPR, FR["fault_off"])
        A.movi(T2, base)
        A.rrr("add", r, r, T2)

    def save():                              # x0..x13 and x30 to sk_sv (T2 its base)
        A.add_i(T2, SPR, SV)
        for i in range(0, 14, 2):
            A.stp(i, i + 1, T2, 8 * i)
        A.mem("str", 30, T2, 8 * 14)

    def restore():
        A.add_i(T2, SPR, SV)
        for i in range(0, 14, 2):
            A.ldp(i, i + 1, T2, 8 * i)
        A.mem("ldr", 30, T2, 8 * 14)

    def native_refuse(lab, text):
        A.label(lab)
        say(b"native: not in milestone 1 (" + text + b"; run it under yantra-run)\n")
        A.movz(0, 2)
        A.b_("rt_exit")

    def nib(r, lab):                         # r = a nibble -> its lowercase hex digit
        A.cmp_i(r, 10)
        A.bcond(LO, lab)
        A.add_i(r, r, 39)
        A.label(lab)
        A.add_i(r, r, 48)

    # ---- rt_sk_init (before the guest: every register is free)
    A.label("rt_sk_init")
    A.mem("str", 30, SPR, SV + 8 * 14)
    lab_at(0, "sk_env_listen")
    A.bl("rt_getenv")
    A.cbnz(0, "rt_ski1")
    lab_at(0, "sk_env_replay")                # YANTRA_SOCK_REPLAY=<socket log>: the replay (no listener)
    A.bl("rt_getenv")
    A.cbnz(0, "rt_skrp")
    A.mem("ldr", 30, SPR, SV + 8 * 14)
    A.ret()
    A.label("rt_ski1")
    A.mem("ldr", 9, SPR, FR["rec_fd"])
    A.cbz(9, "rt_ski_norec")
    A.mem("ldr", 9, SPR, FR["ev_slot"])
    A.cbz(9, "rt_ski_noslot")
    lab_at(10, "sk_pfx")
    A.movz(11, 10)
    A.label("rt_ski2")                        # "127.0.0.1:"
    A.mem("ldrb", 12, 0, 0)
    A.mem("ldrb", 13, 10, 0)
    A.cmp(12, 13)
    A.bcond(NE, "rt_ski_addr")
    A.add_i(0, 0, 1)
    A.add_i(10, 10, 1)
    A.add_i(11, 11, -1)
    A.cbnz(11, "rt_ski2")
    A.movz(9, 0)                              # the port
    A.movz(11, 0)                             # its digits
    A.movz(13, 10)
    A.label("rt_ski3")
    A.mem("ldrb", 12, 0, 0)
    A.cbz(12, "rt_ski4")
    A.add_i(12, 12, -48)
    A.cmp_i(12, 9)
    A.bcond(HI, "rt_ski_addr")
    A.madd(9, 9, 13, 12)
    A.add_i(0, 0, 1)
    A.add_i(11, 11, 1)
    A.cmp_i(11, 5)
    A.bcond(HI, "rt_ski_addr")
    A.b_("rt_ski3")
    A.label("rt_ski4")
    A.cbz(11, "rt_ski_addr")
    A.movi(12, 65535)
    A.cmp(9, 12)
    A.bcond(HI, "rt_ski_addr")
    # sockaddr_in: AF_INET, the port big-endian, 127.0.0.1
    A.lsr_i(10, 9, 8)
    A.ubfm(9, 9, 0, 7)
    A.lsl_i(9, 9, 8)
    A.rrr("orr", 9, 9, 10)
    A.lsl_i(9, 9, 16)
    A.add_i(9, 9, 0x0210 if mac else 2)       # Darwin: sin_len 16, then sin_family
    A.movi(10, 0x0100007F)
    A.lsl_i(10, 10, 32)
    A.rrr("orr", 9, 9, 10)
    st(9, SK_SA)
    A.mem("str", ZR, SPR, SK + SK_SA + 8)
    A.movz(0, 2)
    A.movz(1, 1)
    A.movz(2, 0)
    sysn(198)                                 # socket
    A.cmp_i(0, 0)
    A.bcond(LT, "rt_ski_bind")
    A.add_i(9, 0, 1)
    st(9, SK_LFD)
    A.movz(9, 1)
    st(9, SK_T1)
    A.movz(1, 0xFFFF if mac else 1)           # SOL_SOCKET, SO_REUSEADDR
    A.movz(2, 4 if mac else 2)
    at(3, SK_T1)
    A.movz(4, 4)
    sysn(208)
    fd(0, SK_LFD)
    at(1, SK_SA)
    A.movz(2, 16)
    sysn(200)                                 # bind
    A.cbnz(0, "rt_ski_bind")
    fd(0, SK_LFD)
    A.movz(1, 128)
    sysn(201)                                 # listen
    A.cbnz(0, "rt_ski_bind")
    A.movz(9, 16)
    st(9, SK_LEN)
    fd(0, SK_LFD)
    at(1, SK_SA)
    at(2, SK_LEN)
    sysn(204)                                 # getsockname
    A.movz(0, 0)                              # the queue (MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE)
    A.movi(1, SOCK_SESSION_CAP + SOCK_RECORD_CAP)
    A.movz(2, 3)
    A.movi(3, 0x1002 if mac else 0x4022)     # Darwin: MAP_PRIVATE | MAP_ANON
    A.movn(4, 0)
    A.movz(5, 0)
    sysn(222)
    A.cmp_i(0, 0)
    A.bcond(LT, "rt_ski_bind")
    st(0, SK_Q)
    A.mem("ldr", 0, SPR, FR["rec_fd"])        # the log: truncated, the socket log's header
    A.add_i(0, 0, -1)
    A.movz(1, 0)
    sysn(46)                                  # ftruncate
    A.mem("ldr", 0, SPR, FR["rec_fd"])
    A.add_i(0, 0, -1)
    A.movz(1, 0)
    A.movz(2, 0)
    sysn(62)                                  # lseek
    A.mem("ldr", 0, SPR, FR["rec_fd"])
    A.add_i(0, 0, -1)
    lab_at(1, "sk_hdr")
    A.movi(2, len(SOCKET_LOG_HEADER))
    sysn(64)
    lab_at(0, "env_files")
    A.bl("rt_getenv")
    A.cbz(0, "rt_ski5")
    A.mem("ldr", 0, SPR, FR["rec_fd"])
    A.add_i(0, 0, -1)
    lab_at(1, "sk_grant")
    A.movi(2, len(FILES_GRANT_LINE))
    sysn(64)
    A.label("rt_ski5")
    lab_at(0, "sk_env_sent")
    A.bl("rt_getenv")
    A.cbz(0, "rt_ski6")
    if mac:                                   # open(path, O_WRONLY | O_CREAT | O_TRUNC, 0644)
        A.movi(1, 0x601)
        A.movz(2, 420)
    else:
        A.mov(1, 0)
        A.movn(0, 99)                         # AT_FDCWD
        A.movi(2, 0x241)
        A.movz(3, 420)
    sysn(56)
    A.cmp_i(0, 0)
    A.bcond(LT, "rt_ski_sent")                 # no side file, no honest digest: refused (8f low)
    A.add_i(0, 0, 1)
    st(0, SK_SFD)
    A.label("rt_ski6")
    A.movi(9, SOCK_UNREAD)
    st(9, SK_LATCH)
    A.movz(9, 1)
    st(9, SK_ON)
    ld(9, SK_RP)
    A.cbnz(9, "rt_ski7")                      # a replay listens on nothing
    say(b"socket: listening on 127.0.0.1:")
    A.bl("rt_sk_port")
    say(b"\n")
    A.label("rt_ski7")
    A.mem("ldr", 30, SPR, SV + 8 * 14)
    A.ret()
    # replay: the log is mapped read-only (cursor LFD, end POLL); the queue is the live one; no client
    # exists (SAID set so a TX never says so); records are decoded at each wait
    A.label("rt_skrp")
    A.mem("ldr", 9, SPR, FR["ev_slot"])
    A.cbz(9, "rt_ski_noslot")
    if mac:                                   # open(path, O_RDONLY)
        A.movz(1, 0)
        A.movz(2, 0)
    else:
        A.mov(1, 0)
        A.movn(0, 99)                         # AT_FDCWD
        A.movz(2, 0)
        A.movz(3, 0)
    sysn(56)
    A.cmp_i(0, 0)
    A.bcond(LT, "rt_ski_rplog")
    st(0, SK_T0)
    A.movz(1, 0)
    A.movz(2, 2)
    sysn(62)                                  # lseek(fd, 0, SEEK_END): the length
    A.cmp_i(0, 0)
    A.bcond(LE, "rt_ski_rplog")
    st(0, SK_POLL)
    A.mov(1, 0)
    A.movz(0, 0)
    A.movz(2, 1)                              # PROT_READ
    A.movz(3, 2)                              # MAP_PRIVATE
    ld(4, SK_T0)
    A.movz(5, 0)
    sysn(222)
    A.cmp_i(0, 0)
    A.bcond(LT, "rt_ski_rplog")
    st(0, SK_LFD)
    ld(9, SK_POLL)
    A.rrr("add", 9, 9, 0)
    st(9, SK_POLL)                            # the end address
    ld(0, SK_T0)
    sysn(57)                                  # close
    A.movz(0, 0)                              # the queue, as the live path maps it
    A.movi(1, SOCK_SESSION_CAP + SOCK_RECORD_CAP)
    A.movz(2, 3)
    A.movi(3, 0x1002 if mac else 0x4022)
    A.movn(4, 0)
    A.movz(5, 0)
    sysn(222)
    A.cmp_i(0, 0)
    A.bcond(LT, "rt_ski_rplog")
    st(0, SK_Q)
    A.movz(9, 1)
    st(9, SK_RP)
    st(9, SK_SAID)
    A.b_("rt_ski5")
    native_refuse("rt_ski_rplog", b"YANTRA_SOCK_REPLAY could not be opened or mapped")
    native_refuse("rt_ski_norec", b"YANTRA_LISTEN without YANTRA_RECORD_EVENTS: yantra-run takes --listen only right after --record-events <log>")
    native_refuse("rt_ski_noslot", b"a socket run on an image whose SASEVENT tag is missing or duplicated, or that is threaded")
    native_refuse("rt_ski_addr", b"YANTRA_LISTEN is not 127.0.0.1:PORT")
    native_refuse("rt_ski_bind", b"the socket listener could not be bound")
    native_refuse("rt_ski_sent", b"YANTRA_SOCK_SENT could not be opened (the sent digest would be a lie)")

    # ---- the port at sk+SA+2 (big-endian) as decimal: tail-calls rt_putu
    A.label("rt_sk_port")
    A.mem("ldrb", 9, SPR, SK + SK_SA + 2)
    A.mem("ldrb", 10, SPR, SK + SK_SA + 3)
    A.lsl_i(9, 9, 8)
    A.rrr("orr", 9, 9, 10)
    A.b_("rt_putu")

    # ---- the report line (yantra's socket::report_line; the digest is t1-run.sh's)
    A.label("rt_sk_report")
    A.mem("str", 30, SPR, LR_REP)
    st(ZR, SK_ON)
    say(b"socket: ")
    ld(9, SK_TAIL)
    A.bl("rt_putu")
    say(b" octets received in ")
    ld(9, SK_REC)
    A.bl("rt_putu")
    say(b" records, ")
    ld(9, SK_SENT)
    A.bl("rt_putu")
    say(b" sent, sent sha256 @YANTRA_SOCK_SENT@\n")
    A.mem("ldr", 30, SPR, LR_REP)
    A.ret()

    # ---- a refusal of the window by name: T0 = addr
    for name, text in SOCK_WHY:
        A.label("rt_sk_" + name)
        lab_at(T1, "sk_" + name)
        A.movi(T2, len(text))
        A.b_("rt_sk_dev")
    A.label("rt_sk_dev")
    st(T0, SK_T0)
    st(T1, SK_T1)
    st(T2, SK_T2)
    A.bl("rt_flush")
    say(b"halt: Device { pc: ")
    pc_to(9)
    A.bl("rt_putu")
    say(b", addr: ")
    ld(9, SK_T0)
    A.bl("rt_putu")
    say(b", why: \"")
    A.bl("rt_sk_why")
    say(b"\" }\n")
    ld(9, SK_ON)
    A.cbnz(9, "rt_skd1")
    say(b"socket: touched at ")
    ld(9, SK_T0)
    A.bl("rt_puthex")
    say(b" with no socket device \xe2\x80\x94 ")
    A.bl("rt_sk_why")
    say(b"\n")
    A.label("rt_skd1")
    A.movz(0, 1)
    A.b_("rt_exit")
    A.label("rt_sk_why")
    ld(1, SK_T1)
    ld(2, SK_T2)
    A.movz(0, 2)
    sysn(64)
    A.ret()

    # ---- a load in the window: T0 = addr; the width from the guest's own word (funct3)
    A.label("rt_sk_ld")
    ld(T1, SK_ON)
    A.cbz(T1, "rt_sk_WHY_NO_DEVICE")
    code_word(T1)                             # the load's word
    A.ubfm(T1, T1, 12, 13)                    # funct3 & 3: 2 is a four-octet load
    A.cmp_i(T1, 2)
    A.bcond(NE, "rt_sk_WHY_WIDTH")
    A.movi(T2, SOCK)
    A.rrr("sub", T1, T0, T2)
    A.tst_l(T1, LOG_3)
    A.bcond(NE, "rt_sk_WHY_WIDTH")
    A.cmp_i(T1, 4)
    A.bcond(NE, "rt_skl1")
    at(T0, SK_LATCH)
    if mac:                                   # the guest's load reads [x4 + T0]: a guest address
        A.mem("ldr", T2, SPR, FR["delta"])
        A.rrr("sub", T0, T0, T2)
    A.ret()
    A.label("rt_skl1")
    A.cmp_i(T1, 12)
    A.bcond(EQ, "rt_sk_WHY_CLOSE")
    A.b_("rt_sk_WHY_LOAD_STORE_ONLY")

    # ---- a store in the window: T0 = addr, T1 = value; the width from the guest's word
    A.label("rt_sk_st")
    ld(T2, SK_ON)
    A.cbz(T2, "rt_sk_WHY_NO_DEVICE")
    st(T1, SK_B)                              # the value, while T1 decodes the word
    code_word(T1)
    A.ubfm(T1, T1, 12, 13)
    A.cmp_i(T1, 2)
    A.bcond(NE, "rt_sk_WHY_WIDTH")
    A.movi(T2, SOCK)
    A.rrr("sub", T2, T0, T2)
    A.tst_l(T2, LOG_3)
    A.bcond(NE, "rt_sk_WHY_WIDTH")
    ld(T1, SK_B)
    A.cbz(T2, "rt_sk_next")
    A.cmp_i(T2, 8)
    A.bcond(EQ, "rt_sk_tx")
    A.cmp_i(T2, 12)
    A.bcond(EQ, "rt_sk_WHY_CLOSE")
    A.b_("rt_sk_WHY_STORE_LOAD_ONLY")
    A.label("rt_sk_next")
    A.cbnz(T1, "rt_sk_WHY_NEXT_VALUE", 0)     # the low 32 bits
    ld(T1, SK_HEAD)
    ld(T2, SK_TAIL)
    A.cmp(T1, T2)
    A.bcond(HS, "rt_skn1")
    ld(T2, SK_Q)
    A.memr("ldrb", T2, T2, T1)
    A.add_i(T1, T1, 1)
    st(T1, SK_HEAD)
    st(T2, SK_LATCH)
    A.ret()
    A.label("rt_skn1")
    ld(T1, SK_END)
    A.add_i(T1, T1, SOCK_EMPTY)               # END = EMPTY + 1, sticky
    st(T1, SK_LATCH)
    A.ret()
    A.label("rt_sk_tx")
    A.movi(T2, 0xFF)
    A.cmp(T1, T2, 0)
    A.bcond(HI, "rt_sk_WHY_TX_VALUE")
    st(T1, SK_B)
    save()
    ld(9, SK_SENT)
    st(9, SK_T0)
    A.add_i(9, 9, 1)
    st(9, SK_SENT)
    ld(0, SK_SFD)
    A.cbz(0, "rt_skt1")
    A.add_i(0, 0, -1)
    at(1, SK_B)
    A.movz(2, 1)
    sysn(64)
    A.label("rt_skt1")
    ld(9, SK_CFD)
    A.cbnz(9, "rt_skt2")
    ld(9, SK_SAID)
    A.cbnz(9, "rt_skt9")
    A.movz(9, 1)
    st(9, SK_SAID)
    say(b"socket: send failed after ")
    ld(9, SK_T0)
    A.bl("rt_putu")
    say(b" octets \xe2\x80\x94 no client has connected yet (one is accepted at the first wait); "
        b"the program is not told (W-377)\n")
    A.b_("rt_skt9")
    A.label("rt_skt2")
    ld(9, SK_FAIL)
    A.cbnz(9, "rt_skt9")
    A.label("rt_skt3")
    fd(0, SK_CFD)
    at(1, SK_B)
    A.movz(2, 1)
    A.movz(3, 0 if mac else 0x4000)           # MSG_NOSIGNAL (Darwin: SO_NOSIGPIPE at accept)
    A.movz(4, 0)
    A.movz(5, 0)
    sysn(206)                                 # sendto
    A.cmn_i(0, 4)                             # EINTR: again
    A.bcond(EQ, "rt_skt3")
    A.cmp_i(0, 0)
    A.bcond(GE, "rt_skt9")
    A.movz(9, 1)
    st(9, SK_FAIL)
    A.rrr("sub", 0, ZR, 0)
    st(0, SK_T1)
    say(b"socket: send failed after ")
    ld(9, SK_T0)
    A.bl("rt_putu")
    say(b" octets \xe2\x80\x94 ")
    A.bl("rt_sk_err")
    say(b"; nothing more is written, and the program is not told (W-377)\n")
    A.label("rt_skt9")
    restore()
    A.ret()

    # ---- an errno (sk+T1) as Rust's io::Error says it
    A.label("rt_sk_err")
    A.mem("str", 30, SPR, LR_ERR)
    for n, text in ((32, b"Broken pipe"), (54 if mac else 104, b"Connection reset by peer")):
        ld(9, SK_T1)
        A.cmp_i(9, n)
        A.bcond(NE, "rt_ske%d" % n)
        say(text + b" (")
        A.label("rt_ske%d" % n)
    say(b"os error ")
    ld(9, SK_T1)
    A.bl("rt_putu")
    for n, text in ((32, b"Broken pipe"), (54 if mac else 104, b"Connection reset by peer")):
        ld(9, SK_T1)
        A.cmp_i(9, n)
        A.bcond(NE, "rt_skf%d" % n)
        say(b")")
        A.label("rt_skf%d" % n)
    A.mem("ldr", 30, SPR, LR_ERR)
    A.ret()

    # ---- THE WAIT, a socket run's (runtime_sock's rt_sk_wait)
    def poll(fd_off, retry, idle, err):
        fd(9, fd_off)
        A.mem("strw", 9, SPR, SK + SK_POLL)
        A.movz(9, 1)                          # POLLIN
        A.mem("strw", 9, SPR, SK + SK_POLL + 4)
        A.movz(9, 30)
        st(9, SK_TS)
        st(ZR, SK_TS + 8)
        A.label(retry)
        at(0, SK_POLL)
        A.movz(1, 1)
        if mac:
            A.movi(2, 30000)                  # poll(fds, 1, 30 000 ms)
        else:
            at(2, SK_TS)
            A.movz(3, 0)
            A.movz(4, 0)
        sysn(73)                              # ppoll(fds, 1, {30 s}, NULL, 0)
        A.cmn_i(0, 4)
        A.bcond(EQ, retry)
        A.cbz(0, idle)
        A.cmp_i(0, 0)
        A.bcond(LT, err)
    A.label("rt_sk_wait")
    save()
    ld(9, SK_END)
    A.cbnz(9, "rt_skr_end")
    ld(9, SK_RP)
    A.cbnz(9, "rt_skrw")                      # the replay: the next record of the mapped log
    ld(9, SK_CFD)
    A.cbnz(9, "rt_skw2")
    poll(SK_LFD, "rt_skw1", "rt_skr_idle1", "rt_skr_accept")
    A.movz(9, 16)
    st(9, SK_LEN)
    fd(0, SK_LFD)
    at(1, SK_SA)
    at(2, SK_LEN)
    sysn(202)                                 # accept
    A.cmp_i(0, 0)
    A.bcond(LT, "rt_skr_accept")
    A.add_i(9, 0, 1)
    st(9, SK_CFD)
    A.movz(9, 1)
    st(9, SK_T1)
    A.movz(1, 6)                              # IPPROTO_TCP, TCP_NODELAY
    A.movz(2, 1)
    at(3, SK_T1)
    A.movz(4, 4)
    sysn(208)
    if mac:                                   # SO_NOSIGPIPE: a closed peer is EPIPE, not SIGPIPE
        fd(0, SK_CFD)
        A.movi(1, 0xFFFF)
        A.movi(2, 0x1022)
        at(3, SK_T1)
        A.movz(4, 4)
        sysn(208)
    say(b"socket: accepted ")
    for i in range(4):
        A.mem("ldrb", 9, SPR, SK + SK_SA + 4 + i)
        A.bl("rt_putu")
        say(b"." if i < 3 else b":")
    A.bl("rt_sk_port")
    say(b"\nsocket: the listener is closed; a second client is refused at connect\n")
    fd(0, SK_LFD)
    sysn(57)                                  # close
    st(ZR, SK_LFD)
    A.label("rt_skw2")
    poll(SK_CFD, "rt_skw3", "rt_skr_idle", "rt_skr_read")
    A.label("rt_skw4")
    fd(0, SK_CFD)
    ld(1, SK_Q)
    ld(9, SK_TAIL)
    A.rrr("add", 1, 1, 9)
    A.movi(2, SOCK_RECORD_CAP)
    sysn(63)                                  # read
    A.cmn_i(0, 4)
    A.bcond(EQ, "rt_skw4")
    A.cmp_i(0, 0)
    A.bcond(LT, "rt_skr_read")
    A.cbz(0, "rt_skw_end")
    ld(9, SK_TAIL)                            # the session cap, before the record is logged or applied
    A.rrr("add", 9, 9, 0)
    st(0, SK_T0)
    st(9, SK_T1)
    A.movi(10, SOCK_SESSION_CAP)
    A.cmp(9, 10)
    A.bcond(HI, "rt_skr_cap")
    ld(11, SK_Q)                              # the record's line: "s=" + hex + "\n"
    ld(9, SK_TAIL)
    A.rrr("add", 11, 11, 9)
    line_at(12)
    A.movz(9, 0x3D73)                         # "s="
    A.mem("strh", 9, 12, 0)
    A.add_i(12, 12, 2)
    A.mov(13, 0)
    A.label("rt_skh1")
    A.mem("ldrb", 9, 11, 0)
    A.lsr_i(10, 9, 4)
    nib(10, "rt_skh2")
    A.mem("strb", 10, 12, 0)
    A.ubfm(10, 9, 0, 3)
    nib(10, "rt_skh3")
    A.mem("strb", 10, 12, 1)
    A.add_i(12, 12, 2)
    A.add_i(11, 11, 1)
    A.add_i(13, 13, -1)
    A.cbnz(13, "rt_skh1")
    A.movz(9, 10)
    A.mem("strb", 9, 12, 0)
    A.add_i(12, 12, 1)
    line_at(1)
    A.rrr("sub", 2, 12, 1)
    A.mem("ldr", 0, SPR, FR["rec_fd"])
    A.add_i(0, 0, -1)
    sysn(64)
    ld(0, SK_T0)
    ld(9, SK_T1)
    st(9, SK_TAIL)
    A.b_("rt_skw_dl")
    A.label("rt_skw_end")
    A.mem("ldr", 0, SPR, FR["rec_fd"])
    A.add_i(0, 0, -1)
    lab_at(1, "sk_send")
    A.movz(2, 6)
    sysn(64)
    A.movz(9, 1)
    st(9, SK_END)
    A.movz(0, 0)
    A.label("rt_skw_dl")                      # deliver: the count at tag + 8
    A.mem("ldr", 9, SPR, FR["ev_slot"])
    A.mem("str", 0, 9, 8)
    ld(9, SK_REC)
    A.add_i(9, 9, 1)
    st(9, SK_REC)
    restore()
    A.b_("rt_wt4")

    # ---- the replayed wait: the next record of the mapped socket log (t1-run.sh has validated it).
    # `#` and blank lines are skipped; the end of the log is the SHORTER refusal; `s=end` is END (a
    # count of 0); `s=<hex>` is decoded into the queue after the session-cap check
    A.label("rt_skrw")
    ld(9, SK_LFD)
    ld(10, SK_POLL)
    A.label("rt_skrw1")
    A.cmp(9, 10)
    A.bcond(HS, "rt_skr_short")
    A.mem("ldrb", 0, 9, 0)
    A.cmp_i(0, 10)
    A.bcond(EQ, "rt_skrw2")
    A.cmp_i(0, 35)                            # '#'
    A.bcond(NE, "rt_skrw4")
    A.label("rt_skrw3")                       # skip to the end of the line
    A.mem("ldrb", 0, 9, 0)
    A.add_i(9, 9, 1)
    A.cmp_i(0, 10)
    A.bcond(NE, "rt_skrw3")
    A.b_("rt_skrw1")
    A.label("rt_skrw2")
    A.add_i(9, 9, 1)
    A.b_("rt_skrw1")
    A.label("rt_skrw4")
    A.mem("ldrb", 0, 9, 3)
    A.cmp_i(0, 110)                           # "s=en": s=end (n is no hex digit)
    A.bcond(NE, "rt_skrw5")
    A.add_i(9, 9, 6)
    A.b_("rt_skrw9")
    A.label("rt_skrw5")
    A.add_i(9, 9, 2)
    ld(11, SK_Q)
    ld(12, SK_TAIL)
    A.rrr("add", 11, 11, 12)
    A.movz(13, 0)                             # the octets of this record
    A.label("rt_skrw6")
    A.mem("ldrb", 0, 9, 0)
    A.cmp_i(0, 10)
    A.bcond(EQ, "rt_skrw8")
    A.mem("ldrb", 1, 9, 1)
    for r, lab in ((0, "rt_skrwa"), (1, "rt_skrwb")):
        A.add_i(r, r, -48)
        A.cmp_i(r, 10)
        A.bcond(LO, lab)
        A.add_i(r, r, -39)
        A.label(lab)
    A.lsl_i(0, 0, 4)
    A.rrr("orr", 0, 0, 1)
    A.mem("strb", 0, 11, 0)
    A.add_i(11, 11, 1)
    A.add_i(9, 9, 2)
    A.add_i(13, 13, 1)
    A.b_("rt_skrw6")
    A.label("rt_skrw8")
    A.add_i(9, 9, 1)
    ld(12, SK_TAIL)
    A.rrr("add", 12, 12, 13)
    st(13, SK_T0)
    st(12, SK_T1)
    st(9, SK_LFD)
    A.movi(10, SOCK_SESSION_CAP)
    A.cmp(12, 10)
    A.bcond(HI, "rt_skr_cap")
    st(12, SK_TAIL)
    ld(0, SK_T0)
    A.b_("rt_skw_dl")
    A.label("rt_skrw9")                       # s=end: the peer closed
    st(9, SK_LFD)
    A.movz(9, 1)
    st(9, SK_END)
    A.movz(0, 0)
    A.b_("rt_skw_dl")

    # ---- the wait's refusals: halt Wait { pc }, the report line, "events: REFUSED — <why>", exit 1
    def wait_at():
        say(b"wait index ")
        ld(9, SK_REC)
        A.bl("rt_putu")
        say(b" (the store at pc ")
        pc_to(9)
        A.bl("rt_puthex")
        say(b")")
    A.label("rt_sk_rhead")
    A.mem("str", 30, SPR, LR_HEAD)
    A.bl("rt_flush")
    say(b"halt: Wait { pc: ")
    pc_to(9)
    A.bl("rt_putu")
    say(b" }\n")
    A.bl("rt_sk_report")
    if wm:
        A.bl("rt_wm_report")                   # yantra-run says `ram: high water` BEFORE `events: REFUSED`
    say(b"events: REFUSED \xe2\x80\x94 ")
    A.mem("ldr", 30, SPR, LR_HEAD)
    A.ret()
    A.label("rt_skr_end")
    A.bl("rt_sk_rhead")
    say(b"a WAIT after END: ")
    wait_at()
    say(b" waited after the peer closed (s=end), and there is no other source to wait for (W-377)")
    A.b_("rt_skr_tail")
    A.label("rt_skr_short")
    A.bl("rt_sk_rhead")
    say(b"the socket log is SHORTER than the run: ")
    wait_at()
    say(b" has no record; the log holds ")
    ld(9, SK_REC)
    A.bl("rt_putu")
    say(b" \xe2\x80\x94 the program was NOT resumed, rather than let it read octets nobody delivered")
    A.b_("rt_skr_tail")
    A.label("rt_skr_idle1")
    A.bl("rt_sk_rhead")
    say(b"the socket was IDLE for more than 30 s at the first wait: no client connected (W-377)")
    A.b_("rt_skr_tail")
    A.label("rt_skr_accept")
    A.rrr("sub", 0, ZR, 0)
    st(0, SK_T1)
    A.bl("rt_sk_rhead")
    say(b"accepting the client failed: ")
    A.bl("rt_sk_err")
    say(b" (W-377)")
    A.b_("rt_skr_tail")
    A.label("rt_skr_idle")
    A.bl("rt_sk_rhead")
    say(b"the socket was IDLE for more than 30 s at ")
    wait_at()
    say(b": nothing arrived and the peer did not close (W-377)")
    A.b_("rt_skr_tail")
    A.label("rt_skr_read")
    A.rrr("sub", 0, ZR, 0)
    st(0, SK_T1)
    A.bl("rt_sk_rhead")
    say(b"the read at ")
    wait_at()
    say(b" failed: ")
    A.bl("rt_sk_err")
    say(b" (W-377)")
    A.b_("rt_skr_tail")
    A.label("rt_skr_cap")
    A.bl("rt_sk_rhead")
    say(SOCKET_LIMIT_EXCEEDED + b" (SOCKET_LIMIT_EXCEEDED): record ")
    ld(9, SK_REC)
    A.bl("rt_putu")
    say(b" (")
    ld(9, SK_T0)
    A.bl("rt_putu")
    say(b" octets) would take the session's delivered records past the cap of %d octets (64 MiB), to "
        % SOCK_SESSION_CAP)
    ld(9, SK_T1)
    A.bl("rt_putu")
    say(b" \xe2\x80\x94 refused, not applied and not logged (W-377)")
    A.label("rt_skr_tail")
    say(b"\n")
    A.movz(0, 1)
    A.b_("rt_exit_nw" if wm else "rt_exit")    # the watermark line is already out (rt_sk_rhead)

    # ---- rt_sk_exit: from rt_exit, x0 = the code: the report line once the listener is bound
    A.label("rt_sk_exit")
    A.mem("str", 0, SPR, X0_EXIT)
    A.bl("rt_sk_report")
    A.mem("ldr", 0, SPR, X0_EXIT)
    A.b_("rt_exit2")

    # the strings go out with the runtime's own, at its end (the code here stays words)
    return strs + [("sk_" + n, t) for n, t in SOCK_WHY] + [
        ("sk_env_listen", b"YANTRA_LISTEN=\0"), ("sk_env_sent", b"YANTRA_SOCK_SENT=\0"),
        ("sk_env_replay", b"YANTRA_SOCK_REPLAY=\0"),
        ("sk_pfx", b"127.0.0.1:"), ("sk_hdr", SOCKET_LOG_HEADER), ("sk_grant", FILES_GRANT_LINE),
        ("sk_send", b"s=end\n")]


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


# ---------------------------------------------------------------- the V subset (A64): runtime_vec's twin
# The vector data sits after the F/D data (sp + frame + FP_A64_SIZE); vsave first, so one stp reaches it.
VEC_A64_DATA = [("vsave", 128), ("vl", 8), ("vt", 8), ("vstart", 8), ("vsen", 8), ("vregs", 512)]


def vec_a64_off(name):
    o = 0
    for n, sz in VEC_A64_DATA:
        if n == name:
            return o
        o += sz
    raise KeyError(name)


VEC_A64_SIZE = sum(sz for _, sz in VEC_A64_DATA)


def runtime_csrh_a64(A, base, mac):
    """runtime_csrh's twin: `halt: Csr { pc: P, csr: C, write: W }`, status 1 (fault_off, fault_word: the site)"""
    FR = a64_frame(mac)

    def adr(rd, lab):
        (A.adrl if mac else A.movl)(rd, lab)
    A.label("rt_csrh")
    A.bl("rt_flush")
    adr(1, "csrh_s1")
    A.movi(2, len(CSRH_TEXT[0]))
    A.bl("rt_puts")
    A.mem("ldr", 9, SPR, FR["fault_off"])
    A.movi(10, base)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    adr(1, "csrh_s2")
    A.movi(2, len(CSRH_TEXT[1]))
    A.bl("rt_puts")
    A.mem("ldrw", 9, SPR, FR["fault_word"])
    A.lsr_i(9, 9, 20)
    A.bl("rt_putu")
    A.mem("ldrw", 9, SPR, FR["fault_word"])
    adr(1, "csrh_s3")                          # write: true
    A.movi(2, len(CSRH_TEXT[2]))
    A.ubfm(10, 9, 15, 19)                      # the rs1 field
    A.cbnz(10, "rt_csrh_w")
    A.ubfm(10, 9, 12, 13)                      # funct3 & 3
    A.cmp_i(10, 1)
    A.bcond(EQ, "rt_csrh_w")
    adr(1, "csrh_s4")                          # write: false
    A.movi(2, len(CSRH_TEXT[3]))
    A.label("rt_csrh_w")
    A.bl("rt_puts")
    A.movz(0, 1)
    A.b_("rt_exit")
    for i, t in enumerate(CSRH_TEXT):
        A.label("csrh_s%d" % (i + 1))
        A.raw(t)


def runtime_amo_a64(A, mac=False, wm=False):
    """rt_amo for aarch64 (see runtime_amo): x16 = the address, x17 = x[rs2] in (fault_off / fault_word set),
    x16 = the new x[rd] out; x0 x1 x2 x8 are saved in sv01 / sv28 and restored, x15 x17 are scratch.
    x0 = the old value, x1 = funct5, x15 = 0 (.w) / 1 (.d), x2 x8 = scratch. Reservation: the frame's resv (0: none)."""
    FR = a64_frame(mac)

    def gm(kind, r, rn):                        # a guest access [rn]: through delta on Darwin / Windows
        if mac:
            A.memr(kind, r, DELTA, rn)
        else:
            A.mem(kind, r, rn, 0)

    def out():                                  # x16 = x0; the saved registers back; return to the lowering
        A.mov(T0, 0)
        A.ldp(0, 1, SPR, FR["sv01"])
        A.ldp(2, 8, SPR, FR["sv28"])
        A.ret()

    A.label("rt_amo")
    A.stp(0, 1, SPR, FR["sv01"])
    A.stp(2, 8, SPR, FR["sv28"])
    A.mem("ldrw", 0, SPR, FR["fault_word"])
    A.lsr_i(1, 0, 27, 0)                        # x1 = funct5
    A.ubfm(T2, 0, 12, 12)                       # x15 = 0 (.w) / 1 (.d)
    A.and_l(2, T0, LOG_3)
    A.cbnz(2, "ra_mis")
    A.cbz(T2, "ra_al")
    A.ubfm(2, T0, 2, 2)
    A.cbnz(2, "ra_mis")
    A.label("ra_al")
    A.cbnz(T2, "ra_w0")
    A.sxtw(T1, T1)                              # .w: the operand sign-extended (min / max compare it)
    A.label("ra_w0")
    A.cmp_i(1, 3)
    A.bcond(EQ, "ra_sc")
    A.cmp_i(1, 2)
    A.bcond(NE, "ra_ld")
    A.mem("ldrw", 0, SPR, FR["fault_word"])
    A.ubfm(0, 0, 20, 24)
    A.cbnz(0, "ra_un")                          # lr with rs2 != 0
    A.label("ra_ld")
    A.cbnz(T2, "ra_l8")
    A.ldp(2, 8, SPR, FR["g_l4"])
    A.rrr("sub", 2, T0, 2)
    A.cmp(2, 8)
    A.bcond(HS, "ra_lc")
    gm("ldrsw", 0, T0)
    A.b_("ra_lok")
    A.label("ra_l8")
    A.ldp(2, 8, SPR, FR["g_l8"])
    A.rrr("sub", 2, T0, 2)
    A.cmp(2, 8)
    A.bcond(HS, "ra_lc")
    gm("ldr", 0, T0)
    A.label("ra_lok")                           # x0 = the old value
    A.cmp_i(1, 2)
    A.bcond(EQ, "ra_lr")
    for f5, lab in ((0, "ra_add"), (1, "ra_st"), (4, "ra_xor"), (8, "ra_or"), (0xC, "ra_and"),
                    (0x10, "ra_min"), (0x14, "ra_max"), (0x18, "ra_minu"), (0x1C, "ra_maxu")):
        A.cmp_i(1, f5)
        A.bcond(EQ, lab)
    A.label("ra_un")
    A.b_("rt_unimpl")
    for lab, nm in (("ra_add", "add"), ("ra_xor", "eor"), ("ra_or", "orr"), ("ra_and", "and")):
        A.label(lab)
        A.rrr(nm, T1, T1, 0)
        A.b_("ra_st")
    for lab, keep in (("ra_min", GE), ("ra_max", LE), ("ra_minu", HS), ("ra_maxu", LS)):
        A.label(lab)                            # x17 = the new value: the old one when it wins
        A.cmp(0, T1)
        A.bcond(keep, "ra_st")
        A.mov(T1, 0)
        A.b_("ra_st")
    A.label("ra_st")                            # store x17 at x16; the answer is the old value (x0)
    A.cbnz(T2, "ra_s8")
    A.ldp(2, 8, SPR, FR["g_s4"])
    A.rrr("sub", 2, T0, 2)
    A.cmp(2, 8)
    A.bcond(HS, "ra_sc4")
    if wm:
        A.add_i(2, T0, 4)
        A.mem("ldr", 8, SPR, WM_HW)
        A.cmp(2, 8)
        A.bskip(LS, 2)
        A.mem("str", 2, SPR, WM_HW)
    gm("strw", T1, T0)
    out()
    A.label("ra_s8")
    A.ldp(2, 8, SPR, FR["g_s8"])
    A.rrr("sub", 2, T0, 2)
    A.cmp(2, 8)
    A.bcond(HS, "ra_sc4")
    if wm:
        A.add_i(2, T0, 8)
        A.mem("ldr", 8, SPR, WM_HW)
        A.cmp(2, 8)
        A.bskip(LS, 2)
        A.mem("str", 2, SPR, WM_HW)
    gm("str", T1, T0)
    out()
    A.label("ra_lr")                            # lr: the reservation is the address; rd = the old value
    A.mem("str", T0, SPR, FR["resv"])
    out()
    A.label("ra_sc")                            # sc: the reservation is spent either way; rd = 0 on success
    A.mem("ldr", 0, SPR, FR["resv"])
    A.mem("str", ZR, SPR, FR["resv"])
    A.cbz(0, "ra_fail")
    A.cmp(0, T0)
    A.bcond(NE, "ra_fail")
    A.movz(0, 0)
    A.b_("ra_st")
    A.label("ra_fail")
    A.movz(0, 1)
    out()
    A.label("ra_mis")
    A.mem("str", T0, SPR, FR["fault_addr"])
    A.bl("rt_flush")
    A.b_("rt_bada")
    A.label("ra_lc")                            # a load that missed RAM: the runtime's load halts
    A.bl("rt_load_slow")
    A.b_("rt_bad_insn")                         # (only the socket's RX returns: refused)
    A.label("ra_sc4")                           # a store that missed RAM
    A.bl("rt_store_slow")
    A.b_("rt_bad_insn")


def runtime_vec_a64(A, frame_size, mac=False, wm=False):
    """rt_vec for aarch64: x16 = x[rs1], x17 = x[rs2] in (fault_off / fault_word set), x16 = the new x[rd] out.
    x0-x3 x5-x14 x30 are saved in vsave and restored (x15-x17 and d0-d7 are scratch, as for the F/D helpers);
    x4 (Darwin's delta) is read, never written. Registers: x0 = the F/D data, x3 = the vector data, x5 = x[rs1],
    x6 = x[rs2] (or the step), x7 = the word, x8 the element, x9 vl, x10 x11 x12 the group pointers."""
    FR = a64_frame(mac)
    O = vec_a64_off
    SAVED = (0, 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14)
    n = [0]

    def lab():
        n[0] += 1
        return "rva%d" % n[0]

    def ubfx(rd, rn, lsb, width):
        A.ubfm(rd, rn, lsb, lsb + width - 1)

    def fld(r, sh, width):
        ubfx(r, 7, sh, width)

    def group(r, sh):                           # r = &vregs + 16 * (the register field at sh)
        fld(r, sh, 5)
        A.rrr("add", r, 3, r, amt=4)
        A.add_i(r, r, O("vregs"))

    def at_start(r):                            # r += 8 * vstart
        A.mem("ldr", 13, 3, O("vstart"))
        A.rrr("add", r, r, 13, amt=3)

    def executes(fields):
        A.mem("ldr", 13, 3, O("vt"))
        A.cbz(13, "rva_ill")
        ubfx(14, 13, 2, 4)                      # (vtype & 0x3C) == 0x18: e64, vlmul 0..3
        A.cmp_i(14, 6)
        A.bcond(NE, "rva_bad")
        ubfx(2, 13, 0, 2)
        A.movz(14, 1)
        A.dp2("lslv", 14, 14, 2)
        A.add_i(14, 14, -1)                     # x14 = LMUL - 1
        for sh in fields:
            fld(2, sh, 5)
            A.rrr("and", 2, 2, 14)
            A.cbnz(2, "rva_ill")

    def frm(r):
        A.mem("ldr", r, 0, fp_a64_off("fcsr"))
        ubfx(r, r, 5, 3)
        A.cmp_i(r, 4)
        A.bcond(HI, "rva_ill")

    def loop(body, step=(10, 11, 12)):
        top, out = lab(), lab()
        A.mem("ldr", 8, 3, O("vstart"))
        A.mem("ldr", 9, 3, O("vl"))
        A.label(top)
        A.cmp(8, 9)
        A.bcond(HS, out)
        body()
        for r in step:
            A.add_i(r, r, 8)
        A.add_i(8, 8, 1)
        A.b_(top)
        A.label(out)

    def done(dirty=True):
        if dirty:
            A.movz(13, 3)
            A.mem("str", 13, 3, O("vsen"))
        A.mem("str", ZR, 3, O("vstart"))
        A.b_("rva_ret")

    def gmem(kind, r, a):
        if mac:
            A.memr(kind, r, DELTA, a)
        else:
            A.mem(kind, r, a, 0)

    A.label("rt_vec")
    A.add_i12(15, SPR, frame_size >> 12)
    A.add_i(15, 15, frame_size & 0xFFF)         # x15 = the F/D data
    A.add_i(15, 15, FP_A64_SIZE)                # x15 = the vector data
    for i in range(0, len(SAVED), 2):
        A.stp(SAVED[i], SAVED[i + 1], 15, O("vsave") + 8 * i)
    A.mem("str", 30, 15, O("vsave") + 8 * len(SAVED))
    A.mov(3, 15)
    A.add_i(0, 3, -FP_A64_SIZE)
    A.mov(5, T0)
    A.mov(6, T1)
    A.mem("ldrw", 7, SPR, FR["fault_word"])
    A.mem("ldr", 13, 3, O("vsen"))              # sstatus.VS off: illegal (yantra's step_inner)
    A.cbz(13, "rva_ill")
    fld(13, 0, 7)
    A.cmp_i(13, 0x57)
    A.bcond(NE, "rva_mem")
    fld(10, 12, 3)
    fs_, nofs = lab(), lab()
    A.cmp_i(10, 1)
    A.bcond(EQ, fs_)
    A.cmp_i(10, 5)
    A.bcond(NE, nofs)
    A.label(fs_)
    A.mem("ldr", 13, 0, fp_a64_off("fsen"))     # a float source with FS off: illegal
    A.cbz(13, "rva_ill")
    A.label(nofs)
    A.cmp_i(10, 7)
    A.bcond(EQ, "rva_set")
    fld(13, 25, 1)                              # masked: outside the subset
    A.cbz(13, "rva_err")
    fld(11, 26, 6)
    A.rrr("orr", 10, 11, 10, amt=8)             # x10 = funct3 << 8 | funct6
    for key in list(VEC_INT) + list(VEC_FP) + [(1, 0x03)]:
        A.cmp_i(10, key[0] << 8 | key[1])
        A.bcond(EQ, "rva_%d_%d" % key)
    A.label("rva_err")                          # decode refused it: illegal under vill, else refused
    A.mem("ldr", 13, 3, O("vt"))
    A.cbz(13, "rva_ill")
    A.b_("rva_bad")
    # ---- integer .vv / .vx
    for key, (nm, src) in VEC_INT.items():
        A.label("rva_%d_%d" % key)
        executes((7, 20) + ((15,) if src == "v" else ()))
        group(10, 7)
        group(11, 20)
        at_start(10)
        at_start(11)
        if src == "v":
            group(12, 15)
            at_start(12)

        def body(nm=nm, src=src):
            A.mem("ldr", 13, 11, 0)
            if src == "v":
                A.mem("ldr", 14, 12, 0)
            if nm == "mul":
                A.madd(13, 13, 14 if src == "v" else 5, ZR)
            else:
                A.rrr(nm, 13, 13, 14 if src == "v" else 5)
            A.mem("str", 13, 10, 0)
        loop(body)
        done()
    # ---- float .vv / .vf through the scalar helper, rm = frm
    for key, (nm, src) in VEC_FP.items():
        A.label("rva_%d_%d" % key)
        executes((7, 20) + ((15,) if src == "v" else ()))
        group(10, 7)
        group(11, 20)
        if src == "v":
            group(12, 15)
            at_start(12)
        else:
            fld(12, 15, 5)
            A.add_i(13, SPR, FR["fregs"])
            A.rrr("add", 12, 13, 12, amt=3)
        at_start(10)
        at_start(11)
        frm(7)                                  # (x7 is the word until here)

        def body(nm=nm):
            A.fldr(1, 0, 11, 0)
            A.fldr(1, 1, 12, 0)
            A.mov(17, 7)
            A.bl("fph_%d" % FP_ID[nm])
            A.fstr(1, 0, 10, 0)
        loop(body, (10, 11, 12) if src == "v" else (10, 11))
        done()
    # ---- vfredosum.vs
    A.label("rva_1_3")
    executes((20,))
    A.mem("ldr", 13, 3, O("vstart"))
    A.cbnz(13, "rva_ill")
    group(10, 7)
    group(11, 20)
    group(12, 15)
    frm(7)
    none_ = lab()
    A.mem("ldr", 13, 3, O("vl"))
    A.cbz(13, none_)
    A.fldr(1, 0, 12, 0)

    def red():
        A.fldr(1, 1, 11, 0)
        A.mov(17, 7)
        A.bl("fph_%d" % FP_ID["fadd.d"])
    loop(red, (11,))
    A.fstr(1, 0, 10, 0)
    done()
    A.label(none_)
    done(False)
    # ---- vsetvli / vsetivli / vsetvl: x8 = the AVL, x9 = its kind (0 a number, 1 VLMAX, 2 keep vl), x2 = vtype
    A.label("rva_set")
    vli, ivli, rule, have = lab(), lab(), lab(), lab()
    fld(13, 30, 2)
    A.cmp_i(13, 1)
    A.bcond(LS, vli)
    A.cmp_i(13, 3)
    A.bcond(EQ, ivli)
    fld(13, 25, 7)
    A.cmp_i(13, 0x40)
    A.bcond(NE, "rva_ill")                      # the reserved vsetvl space: illegal under any vtype
    A.mov(2, 6)
    A.b_(rule)
    A.label(vli)
    fld(2, 20, 11)
    A.b_(rule)
    A.label(ivli)
    fld(2, 20, 10)
    fld(8, 15, 5)
    A.movz(9, 0)
    A.b_(have)
    A.label(rule)                               # rs1 != x0: x[rs1]; else rd != x0: VLMAX; else keep vl
    A.mov(8, 5)
    A.movz(9, 0)
    fld(13, 15, 5)
    A.cbnz(13, have)
    A.movz(9, 1)
    fld(13, 7, 5)
    A.cbnz(13, have)
    A.movz(9, 2)
    A.label(have)
    ill_, frac, sh, keep, got, ds = lab(), lab(), lab(), lab(), lab(), lab()
    A.lsr_i(13, 2, 8)
    A.cbnz(13, ill_)                            # a bit above 7
    ubfx(10, 2, 0, 3)                           # vlmul
    A.cmp_i(10, 4)
    A.bcond(EQ, ill_)
    ubfx(11, 2, 3, 3)                           # vsew
    A.cmp_i(11, 3)
    A.bcond(HI, ill_)
    A.movz(1, 128)
    A.cmp_i(10, 4)
    A.bcond(HI, frac)
    A.dp2("lslv", 1, 1, 10)
    A.b_(sh)
    A.label(frac)
    A.movz(13, 8)
    A.rrr("sub", 13, 13, 10)
    A.dp2("lsrv", 1, 1, 13)
    A.label(sh)
    A.add_i(13, 11, 3)
    A.dp2("lsrv", 1, 1, 13)                     # x1 = VLMAX
    A.cbz(1, ill_)
    A.cmp_i(9, 1)
    A.bcond(EQ, got)
    A.bcond(HI, keep)
    A.cmp(8, 1)
    A.bcond(HS, got)
    A.mov(1, 8)
    A.b_(got)
    A.label(keep)
    A.mem("ldr", 8, 3, O("vl"))
    A.cmp(8, 1)
    A.bcond(HS, got)
    A.mov(1, 8)
    A.label(got)
    A.mem("str", 1, 3, O("vl"))
    A.movi(13, VILL)
    A.rrr("orr", 2, 2, 13)
    A.mem("str", 2, 3, O("vt"))
    A.b_(ds)
    A.label(ill_)                               # vill: vl = 0, x[rd] = 0, no trap
    A.movz(1, 0)
    A.mem("str", ZR, 3, O("vl"))
    A.mem("str", ZR, 3, O("vt"))
    A.label(ds)
    A.movz(13, 3)
    A.mem("str", 13, 3, O("vsen"))
    A.mem("str", ZR, 3, O("vstart"))
    A.mov(T0, 1)
    A.b_("rva_ret1")
    # ---- vle64.v / vse64.v, unit-stride or strided
    A.label("rva_mem")
    werr, merr, unit, strided, fields, load_ = lab(), lab(), lab(), lab(), lab(), lab()
    fld(13, 25, 1)
    A.cbz(13, merr)                             # masked
    fld(13, 26, 2)                              # mop
    A.cbz(13, unit)
    A.cmp_i(13, 2)
    A.bcond(EQ, fields)                         # strided: x6 = x[rs2], the step
    A.b_(merr)
    A.label(unit)
    fld(13, 20, 5)
    A.cbnz(13, merr)
    A.movz(6, 8)
    A.label(fields)
    fld(13, 28, 4)                              # mew, nf
    A.cbnz(13, merr)
    fld(13, 12, 3)
    A.cmp_i(13, 7)
    A.bcond(NE, merr)
    executes((7,))
    group(10, 7)
    at_start(10)
    fld(13, 0, 7)
    A.cmp_i(13, 0x07)
    A.bcond(EQ, load_)
    for kind in "sl":
        if kind == "l":
            A.label(load_)

        def body(kind=kind):
            A.madd(1, 6, 8, 5)                  # x1 = step * i + x[rs1]
            A.ldp(13, 14, SPR, FR["g_%s8" % kind])
            A.rrr("sub", 13, 1, 13)
            A.cmp(13, 14)
            A.bcond(HS, "rva_bad")
            if kind == "l":
                gmem("ldr", 13, 1)
                A.mem("str", 13, 10, 0)
            else:
                A.mem("ldr", 13, 10, 0)
                if wm:
                    A.add_i(14, 1, 8)                   # x14 = the element's end; x15 is scratch
                    A.mem("ldr", 15, SPR, WM_HW)
                    A.cmp(14, 15)
                    A.bskip(LS, 2)
                    A.mem("str", 14, SPR, WM_HW)
                gmem("str", 13, 1)
        loop(body, (10,))
        done(kind == "l")
    A.label(merr)                               # a whole-register access is refused even under vill
    fld(13, 26, 2)
    A.cbnz(13, werr)
    fld(13, 20, 5)
    A.cmp_i(13, 8)
    A.bcond(EQ, "rva_bad")
    A.label(werr)
    A.b_("rva_err")
    # ---- out
    def restore():
        A.add_i12(15, SPR, frame_size >> 12)
        A.add_i(15, 15, frame_size & 0xFFF)
        A.add_i(15, 15, FP_A64_SIZE)
        for i in range(0, len(SAVED), 2):
            A.ldp(SAVED[i], SAVED[i + 1], 15, O("vsave") + 8 * i)
        A.mem("ldr", 30, 15, O("vsave") + 8 * len(SAVED))
    A.label("rva_ret")
    A.mov(T0, 1)                                # (x1: only rva_set answers; the rest leave x[rd] alone)
    A.label("rva_ret1")
    restore()
    A.ret()
    for nm, to in (("rva_ill", "rt_illegal"), ("rva_bad", "rt_bad_insn")):
        A.label(nm)
        restore()
        A.b_(to)


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
FJ64_OFF = (A64_FRAME_SIZE + FP_A64_SIZE + 15) & ~15     # FORK-JOIN: the area after the frame and the F/D data


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
            if -(1 << 31) <= sv < (1 << 31) and not isinstance(x, Wide):
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

    A.label("fpx_bad")                         # a reserved rm (yantra's rm_reserved): illegal, cause 2
    A.b_("rt_illegal")
    # ---- sstatus.FS off: yantra's halt for an illegal instruction (no handler: Undelivered, cause 2)
    FRM = a64_frame(mac)
    A.label("rt_illegal")
    A.bl("rt_flush")
    A.bl("fs_m1")
    A.mem("ldr", 9, SPR, FRM["fault_off"])
    A.movi(10, IMAGE_BASE)
    A.rrr("add", 9, 9, 10)
    A.bl("rt_putu")
    A.bl("fs_m2")
    A.movz(0, 1)
    A.b_("rt_exit")
    for nm_, txt in (("fs_m1", b"halt: Undelivered { pc: "), ("fs_m2", b", cause: 2 }\n")):
        A.label(nm_)
        (A.adrl if mac else A.movl)(1, nm_ + "_s")
        A.movi(2, len(txt))
        A.b_("rt_ewr")                     # stderr, or a thread's halt held back (runtime_threads_a64)
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
            # relocations through LC_DYSYMTAB and faults without it (macmini1, 15.4.1);
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
# module built by our own encoder. DESIGN (documented in WASM32.md):
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
SEGW = 16384             # words per segment function (cut at the first block end past it)
SLICE = 65536            # attest: blocks between two deadline() polls
SLICE_I = 1048576        # plain: instructions between two deadline() polls (and the final slice ends at the budget)
# globals: x1..x31 = 0..30
G_TOT, G_TICKS, G_BLK = 31, 32, 33
G_LL, G_LS = 34, 38            # lim_l1,2,4,8 / lim_s1,2,4,8
G_OUTLEN, G_KIND, G_PC, G_ADDR, G_VAL, G_HW = 42, 43, 44, 45, 46, 47
G_RAMOFF, G_BASE, G_IMGLEN, G_RAMDEF, G_ATTEST = 48, 49, 50, 51, 52
G_FINAL = 53                    # plain mode: 1 when no budget is left beyond the current slice
G_MODE, G_SPAN = 54, 55         # plain mode: 0 the FAST functions run, 1 the SAFE ones; the sp window's span
G_RT = 56                       # FAST: the block a loop's dispatch is to enter (-1: none), a global so that no
                                # local is live around a loop for it
G_BUDGET = 57                   # attest mode: initial step budget, to compute retired instructions
G_SST, G_SSCR = 58, 59          # sstatus (masked) and sscratch, only in a module with CSRG (H8)
# function indices
F_WRITE, F_FINISH, F_DEADLINE, F_FLUSH, F_HALT, F_STSLOW, F_MULHU = 0, 1, 2, 3, 4, 5, 6
F_DIV, F_REM, F_DIVU, F_REMU, F_DIVW, F_REMW, F_DIVUW, F_REMUW = 7, 8, 9, 10, 11, 12, 13, 14
F_GO, F_RUN, F_SETUP, F_LEFT, F_SEG0 = 15, 16, 17, 18, 19
# types
T_WRITE, T_FIN, T_I32, T_RUN, T_SETUP, T_I64, T_BIN, T_HALT, T_VOID = range(9)
LOC_TOT, LOC_TICKS, LOC_EA, LOC_OFF, LOC_V, LOC_T, LOC_BLK, LOC_RET, LOC_K = 31, 32, 33, 34, 35, 36, 37, 38, 39
LOC_SAV = 40                    # FAST: a loop's dispatch keeps the entry block here
LOC_SPA = 41                    # FAST: sp - base as an i32 (kept with every write to sp): the sp accesses' address
SPWIN = 2048                    # the sp window: sp in [base + prot + SPWIN, base + ram - SPWIN - 8]
BIGSEG = 600                    # a FAST function of more blocks keeps only its KREG hottest registers in locals
KREG = 12                       # (and sp): V8's register allocation degrades with many locals in a huge function
WL_FREGS = 65536
WL_JT = 65536 + 256
WBASE = 0x8000_0000   # the effective aarch64-windows image base (overrides the 0x1_4000_0000 above); wasm uses WG_BASE
WG_BASE = 0x8000_0000
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
K_SSCR = 134                    # sscratch (the .t1 decoder's number; translate_wasm maps "csr_sscratch" to it)
KNAMES = ["", "lui", "auipc", "jal", "jalr", "beq", "bne", "blt", "bge", "bltu", "bgeu", "lb", "lh", "lw", "ld", "lbu", "lhu",
          "lwu", "fld", "sb", "sh", "sw", "sd", "fsd", "addi", "slti", "sltiu", "xori", "ori", "andi", "slli", "srli", "srai",
          "addiw", "slliw", "srliw", "sraiw", "add", "sll", "slt", "sltu", "xor", "srl", "or", "and", "sub", "sra", "mul",
          "mulh", "mulhsu", "mulhu", "div", "divu", "rem", "remu", "addw", "sllw", "srlw", "subw", "sraw", "mulw", "divw",
          "divuw", "remw", "remuw", "fence", "csrs_sstatus"]


# decoder names the wasm32 target does not lower (refused by name at run time): all F/D operation names of FP_GROUPS,
# flw/fsw/fmv.d.x, the Zicsr forms
WASM_REFUSED = set(["flw", "fsw", "fmv.d.x", "csrrw", "csrrs", "csrrc", "csrrwi", "csrrsi", "csrrci"])
for _g in FP_GROUPS:
    WASM_REFUSED.update(_g[0])


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
CSRG = 0                        # 1: the module keeps sstatus and sscratch (G_SST, G_SSCR): a word reads sstatus or uses sscratch
LEADER, BLKOF, STARTS, BLEN, SEGA, SEGB, SEGOF, ACC, OEND = [], [], [], [], [], [], [], [], []
NBLK = 0
NSEG = 0
RAMOFF = 0
SEGOFAT = 0
CTXCNT = 0
CTXNEST = 0
# the FAST (structured) segment functions, plain mode (WASM32.md "Speed"): loops per block, labels, depths
LEND, LPAR, INNER, LAB, DLAB, SLAB, LOOPAT, OSTK, LPATH, SEGWT = [], [], [], [], [], [], [], [], [], []
ORD, POS, RPO, RPON, LOOPH, LPARH, ISH, VIS, LSIZE, BODY, TREE = [], [], [], [], [], [], [], [], [], [], []
PSTART, PFILL, PLIST, DSTK, DIT, WSTK, RSTK, RCUR = [], [], [], [], [], [], [], []
POSAT = 0
FASTM = 0
DISPOFF = 2
CUR = 0
CURBLK = 0
EXITAT = 0
TOPAT = 0
SEGLO = 0
SEGHI = 0
SEGWORDS = 0
THREADX = 0
# FAST, within a block: which register holds each 8-octet sp slot (store-to-load forwarding). A slot is
# known when SLOTG = GEN and its register still has the version SLOTV it had when the slot was written
SLOTR, SLOTV, SLOTG, REGVER = [], [], [], []
GEN = 0
# FAST: INLOC[r] = 1 when x[r] is a local of the function, 0 when it lives at XRAT + 8r (RCNT: the weights)
INLOC, RCNT = [], []
XRAT = 0


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
    E.i64c(WG_BASE + PROT_SIZE)
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
    if ATTEST == 0:
        # plain mode: the table holds the FAST functions, then the SAFE ones
        E.gget(G_MODE)
        E.i32c(NSEG)
        E.o("i32.mul")
        E.o("i32.add")
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
        E.lget(0)
        E.gset(G_BUDGET)
        E.i64c(SLICE)
        E.gset(G_TICKS)
    else:
        E.u(1)
        E.u(1)
        E.byte(WI64)
        E.i64c(-1)
        E.gset(G_BUDGET)
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
    # the sp window's span (0: no window, every FAST entry hands over to SAFE)
    E.lget(0)
    E.i64c(PROT_SIZE + 2 * SPWIN + 7)
    E.o("i64.sub")
    E.i64c(0)
    E.lget(0)
    E.i64c(PROT_SIZE + 2 * SPWIN + 7)
    E.o("i64.ge_s")
    E.o("select")
    E.gset(G_SPAN)
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
def writes_x(op):
    """1 when the operation writes x[rd] (fld writes an f-register; fence and the sstatus write lower to nothing)"""
    r = 0
    if op >= K_LUI and op <= K_JALR:
        r = 1
    if op >= K_LB and op <= K_LWU:
        r = 1
    if op >= K_ADDI and op <= K_REMUW:
        r = 1
    if op == K_CSRS or op == K_SSCR:
        r = 1
    return r


def wasm_plan():
    """leaders, blocks, segments and the low-memory layout"""
    global NBLK, NSEG, RAMOFF, SEGOFAT, POSAT, XRAT, CSRG
    CSRG = 0
    for k in range(NWORDS):
        LEADER[k] = 0
        BLKOF[k] = 0
        if OPS[k] == K_SSCR:
            CSRG = 1
        if OPS[k] == K_CSRS and RD[k] != 0:
            CSRG = 1
        if OPS[k] == K_FLD or OPS[k] == K_FSD:
            CSRG = 1
    LEADER[(ENTRY - WG_BASE) // 4] = 1
    for k in range(NWORDS):
        op = OPS[k]
        if op == 0 or op == K_JAL or op == K_JALR or (op >= K_BEQ and op <= K_BGEU):
            if k + 1 < NWORDS:
                LEADER[k + 1] = 1
        # plain mode: a write to sp ends its block (the FAST code checks the sp window there); such a leader
        # (2) only splits the code: the budget, the StepLimit points and the jump table keep the blocks of the
        # other leaders (1), so the observable behaviour is unchanged
        if ATTEST == 0 and RD[k] == 2 and writes_x(op):
            if k + 1 < NWORDS and LEADER[k + 1] == 0:
                LEADER[k + 1] = 2
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
                    v = v + WG_BASE + 4 * k
                v = v & 0xFFFFFFFF
                wt = (v - WG_BASE) // 4
                if v >= WG_BASE and wt < NWORDS and v % 4 == 0:
                    LEADER[wt] = 1
    o = CODE_SIZE
    while o + 8 <= IMGLEN:
        hi = IMG[o + 4] | (IMG[o + 5] << 8) | (IMG[o + 6] << 16) | (IMG[o + 7] << 24)
        if hi == 0:
            v = IMG[o] | (IMG[o + 1] << 8) | (IMG[o + 2] << 16) | (IMG[o + 3] << 24)
            wt = (v - WG_BASE) // 4
            if v >= WG_BASE and wt < NWORDS and v % 4 == 0:
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
    # the accounting blocks: ACC = the words a block charges to the budget (its whole accounting block's, at the
    # block that begins it; 0 inside one), OEND = the word that ends its accounting block (halts refund to it)
    e = NWORDS
    j = NBLK - 1
    while j >= 0:
        OEND[j] = e
        ACC[j] = 0
        if LEADER[STARTS[j]] == 1:
            ACC[j] = e - STARTS[j]
            e = STARTS[j]
        j -= 1
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
    POSAT = (SEGOFAT + 2 * NBLK + 15) // 16 * 16
    XRAT = (POSAT + 4 * NBLK + 15) // 16 * 16
    RAMOFF = (XRAT + 256 + 0xFFFF) // 0x10000 * 0x10000


def rget(n):
    if n == 0:
        E.i64c(0)
    elif FASTM and INLOC[n] == 0:
        E.i32c(0)
        E.mem("i64.load", XRAT + 8 * n)
    else:
        E.lget(n - 1)


def rput(n):
    if n:
        if FASTM:
            REGVER[n] += 1
        if FASTM and n == 2:
            E.ltee(1)
            E.o("i32.wrap_i64")
            E.i32c(-2147483648)
            E.o("i32.add")
            E.lset(LOC_SPA)
        elif FASTM and INLOC[n] == 0:
            E.lset(LOC_EA)
            E.i32c(0)
            E.lget(LOC_EA)
            E.mem("i64.store", XRAT + 8 * n)
        else:
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
    if FASTM:
        fast_goto(bi)
    else:
        E.i32c(bi)
        E.lset(LOC_BLK)
        E.br(CTXCNT + CTXNEST + 2)


# ---------------------------------------------------------------- the FAST functions' helpers
def win_test():
    """i32 1 when sp lies in the window: then every sp+imm access (|imm| <= 2048, width <= 8) is RAM above the
    code span and below the store limit, so a FAST block makes it with no guard (no such access can fault)"""
    E.lget(1)
    E.i64c(WG_BASE + PROT_SIZE + SPWIN)
    E.o("i64.sub")
    E.gget(G_SPAN)
    E.o("i64.lt_u")


def to_safe(bi):
    """leave the FAST function for the SAFE one at block bi (bi < 0: the block is in LOC_BLK); RET is 0"""
    if bi >= 0:
        E.i32c(bi)
    else:
        E.lget(LOC_BLK)
    E.gset(G_BLK)
    E.i32c(1)
    E.gset(G_MODE)
    E.br(CUR + CTXNEST - EXITAT)


def win_exit(bi):
    """after a write to sp: out of the window, the SAFE function goes on at block bi"""
    global CTXNEST
    win_test()
    E.o("i32.eqz")
    E.blk("if")
    CTXNEST = CTXNEST + 1
    to_safe(bi)
    CTXNEST = CTXNEST - 1
    E.o("end")


def sp_addr():
    """the linear address of sp - base (an i32 local; the access adds RAMOFF + imm as its offset)"""
    E.lget(LOC_SPA)


def slot_forget(off, wd):
    """an sp store of wd octets at sp + off: the slots it overlaps are no longer known"""
    SLOTG[(off + 2048) // 8] = -1
    SLOTG[(off + 2048 + wd - 1) // 8] = -1


def own(p):
    """the loop whose level block p is a position of (-1: the function's top level)"""
    r = INNER[p]
    if LEND[p] >= 0:
        r = LPAR[p]
    return r


def contains(h, i):
    r = 0
    if h < 0:
        r = 1
    elif h <= i and i <= LEND[h]:
        r = 1
    return r


def child_of(h, j):
    """the position of loop h's level (h = -1: the top level) that holds block j: j, or a loop header"""
    x = j
    if LEND[j] < 0:
        x = INNER[j]
        if x == h:
            return j
    while LPAR[x] != h:
        x = LPAR[x]
    return x


def thread(t):
    """FAST: a block that is a lone `jal x0` (the .t1 compiler's loop-continue stub) is passed through to the
    block it jumps to, its words counted in THREADX (the budget stays exact); up to 8 in a chain"""
    global THREADX
    THREADX = 0
    n = 0
    while n < 8 and t >= SEGLO and t < SEGHI and BLEN[t] == 1 and OPS[STARTS[t]] == K_JAL and RD[STARTS[t]] == 0:
        k = STARTS[t]
        u = direct_target(WG_BASE + 4 * k, IMM[k])
        n += 1
        if u < 0 or u == t:
            n = 8
        else:
            THREADX += ACC[t]
            t = u
    return t


def block_target(p):
    """the threaded direct target of block p's closing branch or jal (-1: none, or outside the code)"""
    global THREADX
    THREADX = 0
    k = STARTS[p] + BLEN[p] - 1
    op = OPS[k]
    r = -1
    if op == K_JAL or (op >= K_BEQ and op <= K_BGEU):
        r = direct_target(WG_BASE + 4 * k, IMM[k])
        if r >= 0:
            r = thread(r)
    return r


def fast_goto(t):
    """a direct transfer from position CURBLK to block t, structured: a back edge is a br to its loop, a
    forward edge a br out of the block that ends at t's position; a backward edge that is no loop's goes
    through the entry's dispatch, an edge into a loop not by its header through that loop's (LOC_BLK = t);
    another segment's block leaves the function"""
    i = CURBLK
    t = thread(t)
    if THREADX > 0:
        E.lget(LOC_TICKS)
        E.i64c(THREADX)
        E.o("i64.sub")
        E.lset(LOC_TICKS)
    if t < SEGLO or t >= SEGHI:
        E.i32c(t)
        E.gset(G_BLK)
        E.br(CUR + CTXNEST - EXITAT)
    else:
        tp = POS[t]
        if tp <= i and LEND[tp] >= i:
            E.br(CUR + CTXNEST - LOOPAT[tp])
        elif tp <= i:
            E.i32c(t)
            E.lset(LOC_BLK)
            E.br(CUR + CTXNEST - TOPAT)
        elif contains(own(tp), i):
            E.br(CUR + CTXNEST - LAB[tp])
        else:
            x = tp
            if LEND[tp] < 0:
                x = INNER[tp]
            while LPAR[x] >= 0 and contains(LPAR[x], i) == 0:
                x = LPAR[x]
            E.i32c(t)
            E.gset(G_RT)
            E.br(CUR + CTXNEST - LAB[x])


def direct_target(pc, imm):
    """the block of a direct branch/jal target, or -1 when it is outside the code or misaligned"""
    t = pc + imm
    r = -1
    if t >= WG_BASE and t < WG_BASE + 4 * NWORDS and t % 4 == 0:
        r = BLKOF[(t - WG_BASE) // 4]
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
        if imm == 0:
            # `jal rd, .`: yantra's Halt::SpinForever { pc } (kind 16), the idiom a bare program parks with
            fault(16, pc, 0, 0, 0, 0, refund)
        elif tb >= 0:
            if FASTM and rd == 2:
                win_exit(tb)
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
        E.i64c(WG_BASE)
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
        if FASTM and rd == 2:
            win_exit(-1)
        E.br(CTXCNT + CTXNEST + DISPOFF)
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
    if FASTM and RS1[k] == 2:
        # FAST: sp lies in the window, so the access is RAM and cannot fault: no guard. An ld of a slot
        # whose value a register still holds (stored or loaded in this block) is that register
        si = (IMM[k] + 2048) // 8
        if op == K_LD and (IMM[k] + 2048) % 8 == 0 and SLOTG[si] == GEN and REGVER[SLOTR[si]] == SLOTV[si]:
            r = SLOTR[si]
            if rd != r and rd != 0:
                rget(r)
                rput(rd)
            return 0
        if op == K_FLD:
            E.i32c(WL_FREGS + 8 * rd)
            sp_addr()
            E.byte(ld)
            E.u(3)
            E.u(RAMOFF + IMM[k])
            E.mem("i64.store", 0)
        else:
            sp_addr()
            E.byte(ld)
            E.u(wd_align(wd))
            E.u(RAMOFF + IMM[k])
            rput(rd)
            if op == K_LD and (IMM[k] + 2048) % 8 == 0 and rd != 0:
                SLOTG[si] = GEN
                SLOTR[si] = rd
                SLOTV[si] = REGVER[rd]
        return 0
    rget(RS1[k])
    E.i64c(IMM[k])
    E.o("i64.add")
    E.lset(LOC_EA)
    E.lget(LOC_EA)
    E.i64c(WG_BASE)
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
    global CTXNEST, GEN
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
    if FASTM and RS1[k] == 2:
        # FAST: sp lies in the window: RAM above the code span, below the store limit, never a device
        sp_addr()
        if op == K_FSD:
            E.i32c(0)
            E.mem("i64.load", WL_FREGS + 8 * RS2[k])
        else:
            rget(RS2[k])
        E.byte(st)
        E.u(wd_align(wd))
        E.u(RAMOFF + IMM[k])
        slot_forget(IMM[k], wd)
        if op == K_SD and (IMM[k] + 2048) % 8 == 0:
            si = (IMM[k] + 2048) // 8
            SLOTG[si] = GEN
            SLOTR[si] = RS2[k]
            SLOTV[si] = REGVER[RS2[k]]
        return 0
    if FASTM:
        GEN += 1                      # a store through another base may alias any slot
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
    E.i64c(WG_BASE + PROT_SIZE)
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
    E.i64c(WG_BASE)
    E.o("i64.sub")
    E.o("i32.wrap_i64")
    E.lget(LOC_V)
    E.byte(st)
    E.u(wd_align(wd))
    E.u(RAMOFF)
    if ATTEST:
        E.lget(LOC_EA)
        E.i64c(WG_BASE - wd)
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


def lower_counter(w):
    """rdcycle / rdinstret (csrrs rd, cycle|instret, x0) under ATTEST: rd = the retired count (G_BUDGET -
    LOC_TOT). Returns 1 when the word was one, 0 otherwise"""
    opc = w & 127
    if opc != 115:
        return 0
    f3 = (w >> 12) & 7
    if f3 != 2:
        return 0
    rs1 = (w >> 15) & 31
    if rs1 != 0:
        return 0
    csr = w >> 20
    hit = 0
    if csr == 3072:
        hit = 1
    if csr == 3074:
        hit = 1
    if hit != 1:
        return 0
    rd = (w >> 7) & 31
    E.gget(G_BUDGET)
    E.lget(LOC_TOT)
    E.o("i64.sub")
    rput(rd)
    return 1


def lower_one(k, pc, refund):
    op = OPS[k]
    if op == 0:
        w = WORDS[k]
        if ATTEST != 0:
            if lower_counter(w) == 1:
                return 0
        # yantra_static: Unimplemented and Breakpoint halt as yantra does (kinds 12, 13); the rest is refused (7)
        kind = yantra_static(w)
        if kind == 1:
            fault(12, pc, 0, 0, 0, WORDS[k], refund)
        if kind == 2:
            fault(13, pc, 0, 0, 0, 0, refund)
        if kind == 5:
            fault(14, pc, 0, 0, 0, WORDS[k], refund)
        if kind != 1 and kind != 2 and kind != 5:
            fault(7, pc, 0, 0, 0, WORDS[k], refund)
    if op == K_LUI or op == K_AUIPC or op == K_JAL or op == K_JALR or (op >= K_BEQ and op <= K_BGEU):
        lower_ctl(k, pc, refund)
    if op == K_FLD or op == K_FSD:
        lower_fs_gate(pc, refund)
    if (op >= K_LB and op <= K_LWU) or op == K_FLD:
        lower_load(k, pc, refund)
    if (op >= K_SB and op <= K_SD) or op == K_FSD:
        lower_store(k, pc, refund)
    if op >= K_ADDI and op <= K_REMUW:
        lower_alu(k, pc)
    if op == K_CSRS or op == K_SSCR or op == K_FLD:
        lower_csr(k)


def lower_fs_gate(pc, refund):
    """fld / fsd while sstatus.FS is Off: yantra's illegal instruction, undelivered (stvec is never set here):
    halt kind 15, Undelivered { pc, cause: 2 }. G_SST is the runtime sstatus (a module with an fld / fsd keeps it)"""
    global CTXNEST
    E.gget(G_SST)
    E.i64c(24576)
    E.o("i64.and")
    E.o("i64.eqz")
    E.blk("if")
    CTXNEST = 1
    fault(15, pc, 0, 0, 0, 0, refund)
    CTXNEST = 0
    E.o("end")


def lower_csr(k):
    """sstatus (K_CSRS) and sscratch (K_SSCR) in G_SST / G_SSCR (H8): LOC_T = the source, LOC_V = the old value
    (sstatus: yantra's read, | UXL, | SD when FS or VS reads 3), the new one stored, rd = the old. A module
    without CSRG lowers them to nothing (no word reads sstatus: its FS and VS gate nothing here). An fld makes
    FS Dirty (yantra's mark_fp_dirty), after its load"""
    op = OPS[k]
    rd = RD[k]
    f3 = IMM[k] & 7
    r = IMM[k] >> 3
    if CSRG == 0:
        return 0
    if op == K_FLD:
        E.gget(G_SST)
        E.i64c(24576)
        E.o("i64.or")
        E.gset(G_SST)
        return 0
    g = G_SSCR
    if op == K_CSRS:
        g = G_SST
    if f3 < 4:
        rget(r)
    else:
        E.i64c(r)
    E.lset(LOC_T)
    E.gget(g)
    E.lset(LOC_V)
    if op == K_CSRS:
        E.i64c(0)
        E.lget(LOC_V)
        E.lget(LOC_V)
        E.i64c(1)
        E.o("i64.shr_u")
        E.o("i64.and")
        E.i64c(8704)
        E.o("i64.and")
        E.o("i64.sub")
        E.i64c(63)
        E.o("i64.shr_u")
        E.i64c(63)
        E.o("i64.shl")
        E.lget(LOC_V)
        E.o("i64.or")
        E.i64c(8589934592)
        E.o("i64.or")
        E.lset(LOC_V)
    if f3 & 3 == 1:
        E.lget(LOC_T)
    if f3 & 3 == 2:
        E.lget(LOC_V)
        E.lget(LOC_T)
        E.o("i64.or")
    if f3 & 3 == 3:
        E.lget(LOC_T)
        E.i64c(-1)
        E.o("i64.xor")
        E.lget(LOC_V)
        E.o("i64.and")
    if op == K_CSRS:
        E.i64c(812834)
        E.o("i64.and")
    E.gset(g)
    if rd != 0:
        E.lget(LOC_V)
        rput(rd)
    return 0


def seg_func(si):
    """one segment: load the state, loop { br_table on the block index }, spill and return. In plain mode this
    is the SAFE function (every access guarded, the budget checked per block): at each block start it hands
    over to the FAST function when sp lies in the window and the slice holds the segment's words"""
    global CTXCNT, CTXNEST, FASTM, DISPOFF
    FASTM = 0
    DISPOFF = 2
    a = SEGA[si]
    b = SEGB[si]
    n = b - a
    w = SEGWT[si]
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
    else:
        E.gget(G_BUDGET)
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
        bpc = WG_BASE + 4 * k0
        jj = bi
        while ACC[jj] == 0:
            jj -= 1
        accs = STARTS[jj]
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
            # back to FAST: sp in the window and the slice holds the segment's words (the FAST entry's test)
            win_test()
            E.lget(LOC_TICKS)
            E.i64c(w)
            E.o("i64.ge_s")
            E.o("i32.and")
            E.blk("if")
            CTXNEST = 1
            E.i32c(bi)
            E.lset(LOC_BLK)
            E.i32c(0)
            E.gset(G_MODE)
            E.br(CTXCNT + CTXNEST + 1)
            CTXNEST = 0
            E.o("end")
            # the block's steps (an accounting block's words, at its first block): the slice counter falls by
            # n; short, it is the budget's end (StepLimit at this block, already refunded) or a refill and a
            # deadline poll in the trampoline
            if ACC[bi] > 0:
                E.lget(LOC_TICKS)
                E.i64c(ACC[bi])
                E.o("i64.sub")
                E.ltee(LOC_TICKS)
                E.i64c(0)
                E.o("i64.lt_s")
                E.blk("if")
                CTXNEST = 1
                # not the final slice: a refill and a deadline poll in the trampoline. The final slice: the budget
                # ends inside this accounting block, after r = its remaining words: they run (the ticks stay
                # charged for the whole block; a halt refunds the unstarted tail) and the StepLimit is raised
                # before word r, exactly where yantra stops (LOC_TOT = r, tested before every word)
                E.gget(G_FINAL)
                E.o("i32.eqz")
                E.blk("if")
                CTXNEST = 2
                E.lget(LOC_TICKS)
                E.i64c(ACC[bi])
                E.o("i64.add")
                E.lset(LOC_TICKS)
                E.i32c(bi)
                E.lset(LOC_BLK)
                E.i32c(2)
                E.lset(LOC_RET)
                E.br(CTXCNT + CTXNEST + 1)
                CTXNEST = 1
                E.o("end")
                E.lget(LOC_TICKS)
                E.i64c(ACC[bi])
                E.o("i64.add")
                E.lset(LOC_TOT)
                CTXNEST = 0
                E.o("end")
        for ii in range(ln):
            kk = k0 + ii
            pc = WG_BASE + 4 * kk
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
            else:
                # the final slice's stop: LOC_TOT (-1 until the budget ends inside this accounting block) is the
                # word index, in the accounting block, before which the StepLimit halts
                E.lget(LOC_TOT)
                E.i64c(kk - accs)
                E.o("i64.eq")
                E.blk("if")
                CTXNEST = 1
                fault(9, pc, 0, 0, 0, 0, OEND[bi] - kk)
                CTXNEST = 0
                E.o("end")
            lower_one(kk, pc, OEND[bi] - kk - 1)
    # fell off the segment's last block
    CTXCNT = 0
    CTXNEST = 0
    if b < NBLK:
        E.i32c(b)
        E.lset(LOC_BLK)
        E.br(CTXCNT + CTXNEST + 1)
    else:
        fault(8, WG_BASE + 4 * NWORDS, 0, WG_BASE + 4 * NWORDS, 0, 0, 0)
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
    else:
        E.lget(LOC_TOT)
        E.gset(G_BUDGET)
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


def falls_through(op):
    r = 1
    if op == 0 or op == K_JAL or op == K_JALR:
        r = 0
    return r


def seg_succ(v, i):
    """the FAST order's successors of block v (in the segment, else -1): i = 0 its closing branch or jal's
    (threaded) target, i = 1 the next block when v falls through or calls (a call's return lands there)"""
    k = STARTS[v] + BLEN[v] - 1
    op = OPS[k]
    r = -1
    if i == 0:
        r = block_target(v)
    elif falls_through(op) or ((op == K_JAL or op == K_JALR) and RD[k] != 0):
        r = v + 1
    if r < SEGLO or r >= SEGHI:
        r = -1
    return r


def fast_order(a, b):
    """the FAST functions' block order: a reverse postorder (fall-through successors last, so they follow),
    then every natural loop made contiguous (a loop = a header and the blocks that reach a retreating edge
    to it without passing it; a retreating edge whose walk escapes above its header is no loop's: fast_goto
    takes it through the dispatch). ORD[position] = block, POS[block] = position, positions a..b-1; then
    per POSITION: LEND (a loop's last position, -1 none), LPAR (a header's enclosing loop), INNER"""
    n = b - a
    # the predecessors (CSR, by block - a)
    for v in range(a, b + 1):
        PSTART[v - a] = 0
    for v in range(a, b):
        for i in range(2):
            s = seg_succ(v, i)
            if s >= 0:
                PSTART[s - a + 1] += 1
    for v in range(a, b):
        PSTART[v - a + 1] += PSTART[v - a]
    for v in range(a, b):
        PFILL[v - a] = PSTART[v - a]
    for v in range(a, b):
        for i in range(2):
            s = seg_succ(v, i)
            if s >= 0:
                PLIST[PFILL[s - a]] = v
                PFILL[s - a] += 1
    # reverse postorder: an iterative DFS from every block not yet reached, in layout order; each such tree
    # numbered in its own reverse postorder, the trees one after the other (TREE: the tree's root)
    for v in range(a, b):
        RPON[v] = -1
    post = 0
    for r in range(a, b):
        if RPON[r] == -1:
            post0 = post
            RPON[r] = -2
            DSTK[0] = r
            DIT[0] = 0
            sp = 1
            while sp > 0:
                v = DSTK[sp - 1]
                i = DIT[sp - 1]
                if i < 2:
                    DIT[sp - 1] = i + 1
                    s = seg_succ(v, i)
                    if s >= 0 and RPON[s] == -1:
                        RPON[s] = -2
                        DSTK[sp] = s
                        DIT[sp] = 0
                        sp += 1
                else:
                    sp -= 1
                    BODY[post] = v
                    post += 1
            for q in range(post0, post):
                v = BODY[q]
                RPON[v] = post0 + post - 1 - q
                RPO[RPON[v]] = v
                TREE[v] = r
    # the natural loops, innermost (latest header) first
    for v in range(a, b):
        LOOPH[v] = -1
        LPARH[v] = -1
        ISH[v] = 0
        VIS[v] = -1
        LSIZE[v] = 0
    q = n - 1
    while q >= 0:
        h = RPO[q]
        ws = 0
        for e in range(PSTART[h - a], PSTART[h - a + 1]):
            u = PLIST[e]
            if RPON[u] >= RPON[h] and TREE[u] == TREE[h]:
                WSTK[ws] = u
                ws += 1
        if ws > 0:
            VIS[h] = h
            nb = 0
            bad = 0
            while ws > 0:
                ws -= 1
                x = WSTK[ws]
                if LOOPH[x] >= 0:
                    x = LOOPH[x]
                    while LPARH[x] >= 0:
                        x = LPARH[x]
                if VIS[x] != h and TREE[x] == TREE[h]:
                    VIS[x] = h
                    if RPON[x] < RPON[h]:
                        bad = 1
                    else:
                        BODY[nb] = x
                        nb += 1
                        for e in range(PSTART[x - a], PSTART[x - a + 1]):
                            WSTK[ws] = PLIST[e]
                            ws += 1
            if bad == 0:
                ISH[h] = 1
                LOOPH[h] = h
                for j in range(nb):
                    x = BODY[j]
                    if ISH[x]:
                        LPARH[x] = h
                    else:
                        LOOPH[x] = h
        q -= 1
    for v in range(a, b):
        x = LOOPH[v]
        while x >= 0:
            LSIZE[x] += 1
            x = LPARH[x]
    # the order: each region (the top, a loop) lists its blocks and child loops in reverse postorder, a child
    # loop's own region in place after its header
    cnt = 0
    RSTK[0] = -1
    RCUR[0] = 0
    rs = 1
    while rs > 0:
        rg = RSTK[rs - 1]
        idx = RCUR[rs - 1]
        found = -1
        while idx < n and found < 0:
            v = RPO[idx]
            idx += 1
            if ISH[v] == 0 and LOOPH[v] == rg:
                found = v
            elif ISH[v] == 1 and LPARH[v] == rg:
                found = v
        RCUR[rs - 1] = idx
        if found < 0:
            rs -= 1
        else:
            ORD[a + cnt] = found
            POS[found] = a + cnt
            cnt += 1
            if ISH[found]:
                RSTK[rs] = found
                RCUR[rs] = RPON[found] + 1
                rs += 1
    for p in range(a, b):
        v = ORD[p]
        LEND[p] = -1
        LPAR[p] = -1
        INNER[p] = -1
        if ISH[v]:
            LEND[p] = p + LSIZE[v] - 1
            if LPARH[v] >= 0:
                LPAR[p] = POS[LPARH[v]]
        if LOOPH[v] >= 0:
            INNER[p] = POS[LOOPH[v]]


def fast_paths(a, b):
    """LPATH[position]: the most words a run entering it executes before its next budget check (a loop
    header, the dispatch) or leaving the function: the longest forward path, its own block included, and the
    continue stubs a transfer passes through (they are charged before its br, whichever way it goes).
    Answers the most over all positions (the entry's check, as any block can be entered)"""
    w = 0
    p = b - 1
    while p >= a:
        v = ORD[p]
        k = STARTS[v] + BLEN[v] - 1
        op = OPS[k]
        m = 0
        if falls_through(op) and v + 1 < NBLK:
            if v + 1 < b and POS[v + 1] == p + 1:
                m = path_from(p + 1)
            else:
                m = edge_cost(a, b, p, v + 1)
        if op == K_JAL or (op >= K_BEQ and op <= K_BGEU):
            t = direct_target(WG_BASE + 4 * k, IMM[k])
            if t >= 0:
                c = edge_cost(a, b, p, t)
                if c > m:
                    m = c
        LPATH[p] = ACC[v] + m
        if LPATH[p] > w:
            w = LPATH[p]
        p -= 1
    return w


def edge_cost(a, b, p, t):
    """the words charged on fast_goto's transfer from position p to block t and after it until a check: the
    stubs it threads, then a forward position's run (a backward transfer meets a loop header or the dispatch,
    another segment's block leaves the function)"""
    u = thread(t)
    c = THREADX
    if u >= a and u < b:
        if POS[u] > p:
            c = c + path_from(POS[u])
    return c


def path_from(s):
    """the words a run entering position s executes before a budget check (0: s heads a loop, it checks)"""
    r = LPATH[s]
    if LEND[s] >= 0:
        r = 0
    return r


def fast_budget(h):
    """a loop header's budget check: a slice short of the longest run to the next check goes on in SAFE"""
    global CTXNEST
    E.lget(LOC_TICKS)
    E.i64c(LPATH[h])
    E.o("i64.lt_s")
    E.blk("if")
    CTXNEST = 1
    to_safe(ORD[h])
    CTXNEST = 0
    E.o("end")


def pos_of_blk():
    """the position of the block in LOC_SAV (the POS table below RAMOFF)"""
    E.lget(LOC_SAV)
    E.i32c(2)
    E.o("i32.shl")
    E.mem("i32.load", POSAT)


def open_loop(h):
    """the loop at position h: `loop`, a block per position of its level ending at that position (the deepest
    one first), the dispatch (taken only when G_RT holds an entry block: then reset to -1; a block inside a
    child loop goes through a stub that sets G_RT again and enters the child), the budget check"""
    global CUR
    E.blk("loop")
    CUR += 1
    LOOPAT[h] = CUR
    q = LEND[h]
    while q > h:
        if own(q) == h:
            E.blk("block")
            CUR += 1
            LAB[q] = CUR
        q -= 1
    E.blk("block")
    CUR += 1
    DLAB[h] = CUR
    q = LEND[h]
    while q > h:
        if LEND[q] > q and LPAR[q] == h:
            E.blk("block")
            CUR += 1
            SLAB[q] = CUR
        q -= 1
    E.gget(G_RT)
    E.ltee(LOC_SAV)
    E.i32c(0)
    E.o("i32.lt_s")
    E.br_if(CUR - DLAB[h])
    E.i32c(-1)
    E.gset(G_RT)
    pos_of_blk()
    E.i32c(h)
    E.o("i32.sub")
    E.byte(0x0E)
    E.u(LEND[h] - h + 1)
    E.u(CUR - DLAB[h])
    for j in range(h + 1, LEND[h] + 1):
        x = child_of(h, j)
        if x == j:
            E.u(CUR - LAB[j])
        else:
            E.u(CUR - SLAB[x])
    E.u(CUR - DLAB[h])
    for q in range(h + 1, LEND[h] + 1):
        if LEND[q] > q and LPAR[q] == h:
            E.o("end")
            CUR -= 1
            E.lget(LOC_SAV)
            E.gset(G_RT)
            E.br(CUR - LAB[q])
    E.o("end")
    CUR -= 1
    fast_budget(h)


def pick_locals(a, b):
    """INLOC: every register a local, but in a FAST function of more than BIGSEG blocks only sp and the KREG
    registers of the most weight (an operand use or a write counts 8^loop depth, depth at most 6)"""
    for r in range(32):
        INLOC[r] = 1
        RCNT[r] = 0
    if b - a > BIGSEG:
        for p in range(a, b):
            v = ORD[p]
            w = 1
            x = INNER[p]
            d = 0
            while x >= 0 and d < 6:
                w = w * 8
                d += 1
                x = LPAR[x]
            for k in range(STARTS[v], STARTS[v] + BLEN[v]):
                RCNT[RD[k]] += w
                RCNT[RS1[k]] += w
                RCNT[RS2[k]] += w
        for r in range(32):
            INLOC[r] = 0
        INLOC[2] = 1
        for i in range(KREG):
            best = -1
            for r in range(1, 32):
                if INLOC[r] == 0:
                    if best < 0 or RCNT[r] > RCNT[best]:
                        best = r
            INLOC[best] = 1


def fast_func(si):
    """plain mode: segment si as STRUCTURED wasm, its blocks in the FAST order (fast_order). Every natural loop
    is a `loop` (a back edge is a br to it), every forward edge a br out of a block that ends at its target, a
    fall-through to a block placed elsewhere an explicit transfer; LOC_BLK + a dispatch (the function's
    `loop $top`, each loop's head) only for the entry, jalr, a backward edge that is no loop's and an edge into
    a loop not by its header. The FAST invariant: sp lies in the window (checked at entry and after every
    write to sp), so sp-relative accesses carry no guard. The budget: blocks subtract their words unchecked;
    the entry and every loop header check that the slice holds the longest run to the next check, else the
    SAFE function goes on, per block, so the StepLimit, refill and deadline points are the SAFE ones."""
    global CTXCNT, CTXNEST, FASTM, DISPOFF, CUR, CURBLK, EXITAT, TOPAT, SEGLO, SEGHI, SEGWORDS, GEN
    a = SEGA[si]
    b = SEGB[si]
    SEGLO = a
    SEGHI = b
    FASTM = 1
    DISPOFF = -1
    fast_order(a, b)
    SEGWORDS = fast_paths(a, b)
    SEGWT[si] = SEGWORDS
    pick_locals(a, b)
    pos = begin_body()
    E.u(2)
    E.u(37)
    E.byte(WI64)
    E.u(5)
    E.byte(WI32)
    for r in range(1, 32):
        if INLOC[r]:
            E.gget(r - 1)
            E.lset(r - 1)
        else:
            E.i32c(0)
            E.gget(r - 1)
            E.mem("i64.store", XRAT + 8 * r)
    E.gget(G_TICKS)
    E.lset(LOC_TICKS)
    E.gget(G_BLK)
    E.lset(LOC_BLK)
    E.lget(1)
    E.o("i32.wrap_i64")
    E.i32c(-2147483648)
    E.o("i32.add")
    E.lset(LOC_SPA)
    CUR = 0
    CTXNEST = 0
    E.blk("block")
    CUR += 1
    EXITAT = CUR
    E.blk("block")
    CUR += 1
    E.blk("loop")
    CUR += 1
    TOPAT = CUR
    j = b - 1
    while j > a:
        if own(j) < 0:
            E.blk("block")
            CUR += 1
            LAB[j] = CUR
        j -= 1
    E.blk("block")
    CUR += 1
    LAB[a] = CUR
    j = b - 1
    while j >= a:
        if LEND[j] > j and LPAR[j] < 0:
            E.blk("block")
            CUR += 1
            SLAB[j] = CUR
        j -= 1
    # the entry (and every dispatched transfer in the segment): the window and the budget, then the dispatch
    win_test()
    E.lget(LOC_TICKS)
    E.i64c(SEGWORDS)
    E.o("i64.ge_s")
    E.o("i32.and")
    E.o("i32.eqz")
    E.blk("if")
    CTXNEST = 1
    to_safe(-1)
    CTXNEST = 0
    E.o("end")
    E.lget(LOC_BLK)
    E.ltee(LOC_SAV)
    E.gset(G_BLK)
    pos_of_blk()
    E.i32c(a)
    E.o("i32.sub")
    E.byte(0x0E)
    E.u(b - a)
    for j in range(a, b):
        x = child_of(-1, j)
        if x == j:
            E.u(CUR - LAB[j])
        else:
            E.u(CUR - SLAB[x])
    E.u(CUR - EXITAT)
    for j in range(a, b):
        if LEND[j] > j and LPAR[j] < 0:
            E.o("end")
            CUR -= 1
            E.lget(LOC_SAV)
            E.gset(G_RT)
            E.br(CUR - LAB[j])
    E.o("end")
    CUR -= 1
    n = 0
    for p in range(a, b):
        while n > 0 and LEND[OSTK[n - 1]] < p:
            E.o("end")
            CUR -= 1
            n -= 1
        if p > a:
            E.o("end")
            CUR -= 1
        if LEND[p] >= 0:
            OSTK[n] = p
            n += 1
            open_loop(p)
        v = ORD[p]
        k0 = STARTS[v]
        ln = BLEN[v]
        if ACC[v] > 0:
            E.lget(LOC_TICKS)
            E.i64c(ACC[v])
            E.o("i64.sub")
            E.lset(LOC_TICKS)
        CURBLK = p
        CTXCNT = CUR - EXITAT - 1
        CTXNEST = 0
        GEN += 1
        for ii in range(ln):
            kk = k0 + ii
            pc = WG_BASE + 4 * kk
            lower_one(kk, pc, OEND[v] - kk - 1)
            op = OPS[kk]
            if RD[kk] == 2 and writes_x(op) and op != K_JAL and op != K_JALR and v + 1 < NBLK:
                win_exit(v + 1)
        # the fall-through: free when the next block is placed next, else an explicit transfer (fast_goto
        # threads it past a continue stub)
        op = OPS[k0 + ln - 1]
        if falls_through(op):
            if v + 1 >= NBLK:
                fault(8, WG_BASE + 4 * NWORDS, 0, WG_BASE + 4 * NWORDS, 0, 0, 0)
            elif v + 1 >= b or POS[v + 1] != p + 1:
                fast_goto(v + 1)
    while n > 0:
        E.o("end")
        CUR -= 1
        n -= 1
    E.o("end")
    E.o("end")
    E.i32c(1)
    E.lset(LOC_RET)
    E.o("end")
    for r in range(1, 32):
        if INLOC[r]:
            E.lget(r - 1)
        else:
            E.i32c(0)
            E.mem("i64.load", XRAT + 8 * r)
        E.gset(r - 1)
    E.lget(LOC_TICKS)
    E.gset(G_TICKS)
    E.lget(LOC_RET)
    E.o("return")
    E.o("end")
    end_body(pos)
    FASTM = 0
    DISPOFF = 2


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
    if CSRG != 0:
        E.u(60)
    else:
        E.u(58)
    for i in range(33):
        glob(WI64, 1, 0)
    glob(WI32, 1, BLKOF[(ENTRY - WG_BASE) // 4])
    for i in range(8):
        glob(WI64, 1, 0)
    glob(WI32, 1, 0)
    glob(WI32, 1, 0)
    for i in range(4):
        glob(WI64, 1, 0)
    glob(WI32, 0, RAMOFF)
    glob(WI64, 0, WG_BASE)
    glob(WI64, 0, IMGLEN)
    ramdef = EXTENT + 16777216
    if ramdef < 20971520:
        ramdef = 20971520
    glob(WI64, 0, ramdef)
    glob(WI32, 0, attest)
    glob(WI32, 1, 0)
    glob(WI32, 1, 0)
    glob(WI64, 1, 0)
    glob(WI32, 1, -1)
    glob(WI64, 1, 0)
    if CSRG != 0:
        glob(WI64, 1, 0)
        glob(WI64, 1, 0)
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
        if LEADER[k] == 1:
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
        q = WL_JT + 4 * NWORDS + j - POSAT
        if q >= 0 and q < 4 * NBLK:
            z = (POS[q // 4] >> (8 * (q % 4))) & 255
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
    # the segment functions: attest, NSEG (one per segment); plain, the NSEG FAST ones then the NSEG SAFE ones
    nf = NSEG
    if attest == 0:
        nf = 2 * NSEG
    pos = sec_begin(3)
    E.u(16 + nf)
    for i in range(F_FLUSH, F_LEFT + 1):
        E.u(h_type(i))
    for i in range(nf):
        E.u(T_I32)
    end_body(pos)
    pos = sec_begin(4)
    E.u(1)
    E.byte(0x70)
    E.byte(0)
    E.u(nf)
    end_body(pos)
    pos = sec_begin(5)
    E.u(1)
    E.byte(1)
    E.u((RAMOFF + IMGLEN + 0xFFFF) // 0x10000)
    E.u(65536)
    end_body(pos)
    sec_globals(attest)
    sec_exports()
    # the custom section "entry": e_entry (4 octets, little-endian), for the host's threaded-image check
    pos = sec_begin(0)
    E.str("entry")
    for i in range(4):
        E.byte(ENTRY >> (8 * i))
    end_body(pos)
    pos = sec_begin(9)
    E.u(1)
    E.byte(0)
    E.i32c(0)
    E.o("end")
    E.u(nf)
    for i in range(nf):
        E.u(F_SEG0 + i)
    end_body(pos)
    pos = sec_begin(10)
    E.u(16 + nf)
    for i in range(F_FLUSH, F_LEFT + 1):
        p2 = begin_body()
        h_emit(i)
        end_body(p2)
    if attest == 0:
        for i in range(NSEG):
            fast_func(i)
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
    grow(ACC, NWORDS)
    grow(OEND, NWORDS)
    grow(LEND, NWORDS)
    grow(LPAR, NWORDS)
    grow(INNER, NWORDS)
    grow(LAB, NWORDS)
    grow(DLAB, NWORDS)
    grow(SLAB, NWORDS)
    grow(LOOPAT, NWORDS)
    grow(OSTK, NWORDS)
    grow(LPATH, NWORDS)
    grow(SEGWT, NWORDS)
    grow(ORD, NWORDS)
    grow(POS, NWORDS)
    grow(RPO, NWORDS)
    grow(RPON, NWORDS)
    grow(LOOPH, NWORDS)
    grow(LPARH, NWORDS)
    grow(ISH, NWORDS)
    grow(VIS, NWORDS)
    grow(LSIZE, NWORDS)
    grow(BODY, NWORDS)
    grow(TREE, NWORDS)
    grow(SLOTR, 520)
    grow(SLOTV, 520)
    grow(SLOTG, 520)
    grow(REGVER, 32)
    grow(INLOC, 32)
    grow(RCNT, 32)
    grow(PSTART, NWORDS + 2)
    grow(PFILL, NWORDS)
    grow(PLIST, 2 * NWORDS + 2)
    grow(DSTK, NWORDS)
    grow(DIT, NWORDS)
    grow(WSTK, 3 * NWORDS + 2)
    grow(RSTK, NWORDS + 1)
    grow(RCUR, NWORDS + 1)


def wasm_entry():
    wasm_sizes()
    wasm_plan()
    wasm_module(ATTEST)


def translate_wasm(img, arg0, attest=False):
    """the oracle's entry: parse, decode into the module state, run the restricted code"""
    global E, WORDS, OPS, RD, RS1, RS2, IMM, IMG, NWORDS, CODE_SIZE, PROT_SIZE, ENTRY, IMGLEN, EXTENT, ATTEST
    global LEADER, BLKOF, STARTS, BLEN, SEGA, SEGB, SEGOF, ACC, OEND, LEND, LPAR, INNER, LAB, DLAB, SLAB, LOOPAT, OSTK, LPATH, SEGWT
    global SLOTR, SLOTV, SLOTG, REGVER, INLOC, RCNT
    global ORD, POS, RPO, RPON, LOOPH, LPARH, ISH, VIS, LSIZE, BODY, TREE, PSTART, PFILL, PLIST, DSTK, DIT, WSTK, RSTK, RCUR
    entry, base, span, extent, code_size, prot_size = parse_elf(img)
    require_base(base)
    global WG_BASE
    WG_BASE = base
    NWORDS = code_size // 4
    WORDS = list(struct.unpack_from("<%dI" % NWORDS, span))
    OPS, RD, RS1, RS2, IMM = [], [], [], [], []
    for w in WORDS:
        d = decode(w)
        if d is None or d[0] in ("vec", "amo"):
            d = ("", 0, 0, 0, 0)
        # KNAMES is the wasm op table: the integer subset plus the fld/fsd/sstatus forms. Every other name the
        # decoder yields is classified HERE, by name (WASM_REFUSED: F/D arithmetic, flw/fsw, fmv, the CSR forms, vec):
        # it lowers as an undecodable word does (op 0, refused by name at run time, kind 7). A name in neither list
        # is a decoder/table gap and stops the translation, so no op can fall through unclassified.
        if d[0] == "csr_sscratch":
            OPS.append(K_SSCR)
        elif d[0] in KNAMES:
            OPS.append(KNAMES.index(d[0]))
        else:
            assert d[0] == "" or d[0] in WASM_REFUSED, "wasm32: op %s is neither lowered nor refused by name" % d[0]
            OPS.append(0)
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
    wm = False
    vd = False
    while av and av[0] in ("--target", "--watermark", "--verdict", "--segw"):
        if av[0] == "--segw":
            SEGW = int(av[1])    # words per wasm segment function (default 16384); smaller = V8 tiers each function up cheaply
            assert SEGW >= 256, SEGW
            av = av[2:]
        elif av[0] == "--target":
            target = av[1]
            av = av[2:]
        elif av[0] == "--verdict":
            vd = True            # yantra-run's own accounting: YANTRA_VERDICT, YANTRA_STEPS, `steps:`, StepLimit, spin (L33)
            av = av[1:]
        else:
            wm = True            # keep the store high-water mark; say it at exit when YANTRA_WATERMARK is set
            av = av[1:]
    assert target in ("x86_64-linux", "aarch64-linux", "aarch64-macos", "x86_64-macos", "aarch64-windows",
                      "x86_64-windows", "wasm32", "wasm32-attest"), target
    assert not wm or target in ("x86_64-linux", "aarch64-linux"), "--watermark: x86_64-linux and aarch64-linux only"
    assert not vd or target in ("x86_64-linux", "aarch64-linux"), "--verdict: x86_64-linux and aarch64-linux only"
    img = open(av[0], "rb").read()
    arg0 = av[2] if len(av) > 2 else av[0]
    if target.startswith("wasm32"):
        out, M = translate_wasm(img, arg0, target == "wasm32-attest")
    elif target.startswith("x86_64"):
        out, M = translate(img, arg0, {"x86_64-macos": True, "x86_64-windows": "win"}.get(target, False), wm, vd)
    else:
        out, M = translate_a64(img, arg0, {"aarch64-macos": True, "aarch64-windows": "win"}.get(target, False), wm, vd)
    open(av[1], "wb").write(out)
    import os
    os.chmod(av[1], 0o755)
    if target.startswith("wasm32"):
        sys.stderr.write("anuvada: %d octets, %s\n" % (len(out), M))
    else:
        sys.stderr.write("anuvada: %d octets, mapped %s\n" % (len(out), {("x%d" % r): h for r, h in M.items()}))
