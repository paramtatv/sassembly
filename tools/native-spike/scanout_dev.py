"""YANTRA_SCANOUT natively (L33): yantra's virtio-mmio slot-0 GPU (virtio_mmio.rs, virtqueue,
virtio_gpu.rs, gpu) as ONE device program in a tiny memory-to-memory IR, lowered to x86-64 and
aarch64 by two small backends (emit_x86, emit_a64). Oracle-only (ANUVADA_SC=1), like --net.

State is one block (cells, then arrays). Anything yantra HALTS on, and any resource failure, is
`refuse`: stderr `native: not in milestone 1 (virtio-gpu: WHY; run it under yantra-run)`, exit 2;
t1-run discards the native run and reruns the whole program under yantra-run. Everything yantra
ANSWERS is implemented exactly (see agent-handovers/L33-SCANOUT.md)."""

MASK = (1 << 64) - 1
CELLS = 8192                      # the scalar cells (1024 of 8 octets); arrays follow
SEGS_N = 256
RES_N, RES_STRIDE = 256, 128
ARR = {}                          # name -> (offset, size), filled by Prog


class V:
    """a cell, or an expression temp, with operators that emit IR"""
    __slots__ = ("p", "off")

    def __init__(s, p, off):
        s.p, s.off = p, off

    def _b(s, op, o, rev=False):
        a, b = (o, s) if rev else (s, o)
        return s.p.bin(op, a, b)

    def __add__(s, o): return s._b("add", o)
    def __radd__(s, o): return s._b("add", o, True)
    def __sub__(s, o): return s._b("sub", o)
    def __rsub__(s, o): return s._b("sub", o, True)
    def __and__(s, o): return s._b("and", o)
    def __rand__(s, o): return s._b("and", o, True)
    def __or__(s, o): return s._b("or", o)
    def __ror__(s, o): return s._b("or", o, True)
    def __xor__(s, o): return s._b("xor", o)
    def __lshift__(s, o): return s._b("shl", o)
    def __rshift__(s, o): return s._b("shr", o)
    def __mul__(s, o): return s._b("mul", o)
    def __rmul__(s, o): return s._b("mul", o, True)
    def __floordiv__(s, o): return s._b("divu", o)
    def __mod__(s, o): return s._b("remu", o)


def opd(x):
    if isinstance(x, V):
        return ("c", x.off)
    assert isinstance(x, int)
    return ("i", x & MASK)


INV = {"eq": "ne", "ne": "eq", "ltu": "geu", "geu": "ltu", "gtu": "leu", "leu": "gtu"}


class Prog:
    def __init__(s):
        s.ops = []
        s.nc = 0
        s.names = {}
        s.arrs = {}
        s.anext = CELLS
        s.ntmp = 0
        s.nl = 0
        s.stack = []
        s.loops = []
        s.reasons = []
        s.envs = []
        s.tmps = [s.cell("t%d" % i) for i in range(48)]

    # -- storage
    def cell(s, name):
        assert name not in s.names, name
        off = 8 * s.nc
        s.nc += 1
        assert off < CELLS
        s.names[name] = off
        return V(s, off)

    def arr(s, name, size):
        off = s.anext
        s.anext += (size + 7) & ~7
        s.arrs[name] = (off, size)
        return off

    def lea(s, name):
        t = s.tmp()
        s.ops.append(("lea", opd(t), s.arrs[name][0]))
        return t

    def size(s):
        return (s.anext + 15) & ~15

    # -- temps
    def tmp(s):
        t = s.tmps[s.ntmp]
        s.ntmp += 1
        return t

    def end(s):
        s.ntmp = 0

    def label(s, L):
        s.ops.append(("label", L))

    def newl(s, hint="L"):
        s.nl += 1
        return "%s%d" % (hint, s.nl)

    # -- expressions
    def bin(s, op, a, b):
        t = s.tmp()
        s.ops.append(("bin", op, opd(t), opd(a), opd(b)))
        return t

    def ld(s, w, a, off=0):
        t = s.tmp()
        s.ops.append(("ld", w, opd(t), opd(a), off))
        return t

    def sys(s, name, *args):
        t = s.tmp()
        s.ops.append(("sys", opd(t), name, [opd(a) for a in args]))
        return t

    def getenv(s, name):
        t = s.tmp()
        if name not in s.envs:
            s.envs.append(name)
        s.ops.append(("getenv", opd(t), name))
        return t

    # -- statements
    def set(s, d, e):
        s.ops.append(("mov", opd(d), opd(e)))
        s.end()

    def st(s, w, a, off, b):
        s.ops.append(("st", w, opd(a), off, opd(b)))
        s.end()

    def copy(s, dst, src, n):
        s.ops.append(("copy", opd(dst), opd(src), opd(n)))
        s.end()

    def jmp(s, L):
        s.ops.append(("jmp", L))

    def br(s, a, cc, b, L):
        s.ops.append(("br", cc, opd(a), opd(b), L))
        s.end()

    def refuse(s, why):
        if why not in s.reasons:
            s.reasons.append(why)
        s.ops.append(("refuse", why))
        s.end()

    def ret(s):
        s.ops.append(("ret",))
        s.end()

    def If(s, a, cc, b):
        c = _If(s, INV[cc], a, b)
        return c

    def While(s, fn):
        return _While(s, fn)

    def brk(s):
        s.ops.append(("jmp", s.loops[-1][1]))

    def cont(s):
        s.ops.append(("jmp", s.loops[-1][0]))


class _If:
    def __init__(s, p, cc, a, b):
        s.p, s.cc, s.a, s.b = p, cc, a, b
        s.L = p.newl("F")
        s.has_else = False
        s.Lend = None

    def __enter__(s):
        s.p.br(s.a, s.cc, s.b, s.L)
        s.p.stack.append(s)

    def __exit__(s, *x):
        s.p.stack.pop()
        s.p.label(s.Lend if s.has_else else s.L)


class _While:
    def __init__(s, p, fn):
        s.p, s.fn = p, fn
        s.top, s.out = p.newl("W"), p.newl("X")

    def __enter__(s):
        s.p.label(s.top)
        a, cc, b = s.fn()
        s.p.br(a, INV[cc], b, s.out)
        s.p.loops.append((s.top, s.out))

    def __exit__(s, *x):
        s.p.loops.pop()
        s.p.jmp(s.top)
        s.p.label(s.out)


# ------------------------------------------------------------------------- the device
def build(base):
    p = Prog()
    C = p.cell
    names = ("ADDR VAL WOK ISST TOP CE RV INIT DUMP OFF VV STATUS PAGE QSEL ISR NRES HELD SCO SCX SCY SCW SCH "
             "QP SIZE DESC AVAIL USED LAST SERVED HEAD CNT TOTAL SEENW CI DA DL DF DN DW RL HV RQ RQSZ POS J SP "
             "REPLEN RC FNC FL BODY BL RP K E ID RX RY RW RH N1 N2 N3 N4 RPY QRP PB PO PT PC PX PY PROW PFD "
             "PTOT PL OFFS ROWB STRIDE CV BKN QN QA QV SZ SGP").split()
    g = {n: C(n) for n in names}
    ADDR, VAL, WOK, ISST, TOP, CE, RV = (g[n] for n in "ADDR VAL WOK ISST TOP CE RV".split())
    INIT, DUMP, OFF, VV = g["INIT"], g["DUMP"], g["OFF"], g["VV"]
    STATUS, PAGE, QSEL, ISR = g["STATUS"], g["PAGE"], g["QSEL"], g["ISR"]
    NRES, HELD, SCO, SCX, SCY, SCW, SCH = (g[n] for n in "NRES HELD SCO SCX SCY SCW SCH".split())
    QP, SIZE, DESC, AVAIL, USED, LAST, SERVED = (g[n] for n in "QP SIZE DESC AVAIL USED LAST SERVED".split())
    HEAD, CNT, TOTAL, SEENW, CI = (g[n] for n in "HEAD CNT TOTAL SEENW CI".split())
    RL, HV, RQ, RQSZ, POS, J, SP = (g[n] for n in "RL HV RQ RQSZ POS J SP".split())
    REPLEN, RC, FNC, FL, BODY, BL = (g[n] for n in "REPLEN RC FNC FL BODY BL".split())
    RP, K, E, ID = g["RP"], g["K"], g["E"], g["ID"]
    RX, RY, RW, RH, N1, N2, N3, N4 = (g[n] for n in "RX RY RW RH N1 N2 N3 N4".split())
    RPY, QRP = g["RPY"], g["QRP"]
    PB, PO, PT, PC, PX, PY, PROW, PFD, PTOT, PL = (g[n] for n in "PB PO PT PC PX PY PROW PFD PTOT PL".split())
    OFFS, ROWB, STRIDE, CV = g["OFFS"], g["ROWB"], g["STRIDE"], g["CV"]
    BKN, SGP = g["BKN"], g["SGP"]
    DA, DL, DF, DN, DW = (g[n] for n in "DA DL DF DN DW".split())
    p.arr("QR", 64)
    p.arr("SEGS", 16 * SEGS_N)
    p.arr("RES", RES_N * RES_STRIDE)
    p.arr("REPLY", 512)
    ERR = 0xFFFFFFFFFFFFF000

    def qptr(q):
        p.set(QP, p.lea("QR") + q * 32)

    def pow2_or_refuse(v, why):
        with p.If(v, "eq", 0):
            p.refuse(why)
        with p.If(v & (v - 1), "ne", 0):
            p.refuse(why)

    def free_all():
        p.set(K, 0)
        with p.While(lambda: (K, "ltu", NRES)):
            p.set(E, p.lea("RES") + K * RES_STRIDE)
            p.sys("munmap", p.ld(8, E, 32), p.ld(8, E, 40))
            p.end()
            with p.If(p.ld(8, E, 56), "ne", 0):
                p.sys("munmap", p.ld(8, E, 48), p.ld(8, E, 56) * 16)
                p.end()
            p.set(K, K + 1)

    def reset():
        free_all()
        for c in (NRES, HELD, SCO, SCX, SCY, SCW, SCH, STATUS, PAGE, QSEL, ISR):
            p.set(c, 0)
        p.set(QRP, p.lea("QR"))
        for o in range(0, 64, 8):
            p.st(8, QRP, o, 0)

    def find(idv):
        p.set(RP, 0)
        p.set(K, 0)
        with p.While(lambda: (K, "ltu", NRES)):
            p.set(E, p.lea("RES") + K * RES_STRIDE)
            with p.If(p.ld(8, E, 0), "eq", idv):
                p.set(RP, E)
                p.brk()
            p.set(K, K + 1)

    # ================================================================= main
    with p.If(INIT, "eq", 0):
        with p.If(p.getenv("YANTRA_VIRTIO_DEFER="), "ne", 0):
            p.refuse("YANTRA_VIRTIO_DEFER is set")
        p.set(DUMP, p.getenv("YANTRA_SCANOUT="))
        p.set(INIT, 1)
    p.set(OFF, ADDR - 0x10001000)
    with p.If(WOK, "eq", 0):
        p.refuse("slot 0 takes 32-bit accesses only")
    with p.If(OFF & 3, "ne", 0):
        p.refuse("unaligned register")
    p.set(VV, VAL & 0xFFFFFFFF)
    L_store = p.newl("S")
    p.br(ISST, "ne", 0, L_store)

    # ---------------- loads
    def rd(off, val):
        with p.If(OFF, "eq", off):
            p.set(RV, val)
            p.ret()
    rd(0x000, 0x74726976)
    rd(0x004, 1)
    rd(0x008, 16)
    rd(0x00C, 0x534E4153)
    rd(0x010, 0)
    with p.If(OFF, "eq", 0x070):
        p.set(RV, STATUS)
        p.ret()
    with p.If(OFF, "eq", 0x060):
        p.set(RV, ISR)
        p.ret()
    with p.If(OFF, "eq", 0x034):
        p.set(RV, 0)
        with p.If(QSEL, "geu", 2):
            p.ret()
        qptr(QSEL)
        with p.If(p.ld(8, QP, 0), "ne", 0):
            p.refuse("QueueNumMax read while the queue is in use")
        p.set(RV, 256)
        p.ret()
    with p.If(OFF, "eq", 0x040):
        p.set(RV, 0)
        with p.If(QSEL, "geu", 2):
            p.ret()
        qptr(QSEL)
        p.set(RV, p.ld(8, QP, 0))
        p.ret()
    p.refuse("a register this phase does not answer")

    # ---------------- stores
    p.label(L_store)
    with p.If(OFF, "eq", 0x070):
        with p.If(VV, "eq", 0):
            reset()
            p.ret()
        p.set(STATUS, VV)
        p.ret()
    with p.If(OFF, "eq", 0x014):
        p.ret()
    with p.If(OFF, "eq", 0x024):
        p.ret()
    with p.If(OFF, "eq", 0x020):
        with p.If(VV, "ne", 0):
            p.refuse("GuestFeatures names a feature bit")
        p.ret()
    with p.If(OFF, "eq", 0x028):
        pow2_or_refuse(VV, "GuestPageSize is not a power of 2")
        qptr(0)
        with p.If(p.ld(8, QP, 0), "ne", 0):
            p.refuse("GuestPageSize written while a queue is in use")
        qptr(1)
        with p.If(p.ld(8, QP, 0), "ne", 0):
            p.refuse("GuestPageSize written while a queue is in use")
        with p.If(VV, "gtu", 0x100000000):
            p.refuse("GuestPageSize above 2^32")
        p.set(PAGE, VV)
        p.ret()
    with p.If(OFF, "eq", 0x030):
        p.set(QSEL, VV)
        p.ret()
    with p.If(OFF, "eq", 0x038):
        with p.If(QSEL, "geu", 2):
            p.refuse("QueueNum for a queue the device does not have")
        qptr(QSEL)
        with p.If(p.ld(8, QP, 0), "ne", 0):
            p.refuse("QueueNum written while the queue is in use")
        pow2_or_refuse(VV, "QueueNum is not a power of 2")
        with p.If(VV, "gtu", 256):
            p.refuse("QueueNum above QueueNumMax")
        p.st(8, QP, 8, VV)
        p.ret()
    with p.If(OFF, "eq", 0x03C):
        with p.If(QSEL, "geu", 2):
            p.refuse("QueueAlign for a queue the device does not have")
        qptr(QSEL)
        with p.If(p.ld(8, QP, 0), "ne", 0):
            p.refuse("QueueAlign written while the queue is in use")
        pow2_or_refuse(VV, "QueueAlign is not a power of 2")
        with p.If(VV, "gtu", 0x100000000):
            p.refuse("QueueAlign above 2^32")
        p.st(8, QP, 16, VV)
        p.ret()
    with p.If(OFF, "eq", 0x040):
        with p.If(QSEL, "geu", 2):
            p.refuse("QueuePFN for a queue the device does not have")
        qptr(QSEL)
        with p.If(VV, "eq", 0):
            p.st(8, QP, 0, 0)
            p.st(8, QP, 24, 0)
            p.ret()
        with p.If(PAGE, "eq", 0):
            p.refuse("QueuePFN before GuestPageSize")
        with p.If(p.ld(8, QP, 8), "eq", 0):
            p.refuse("QueuePFN before QueueNum")
        with p.If(p.ld(8, QP, 16), "eq", 0):
            p.refuse("QueuePFN before QueueAlign")
        p.set(DESC, VV * PAGE)
        with p.If(DESC, "ltu", base):
            p.refuse("QueuePFN places the queue outside RAM")
        with p.If(DESC, "gtu", TOP + base):
            p.refuse("QueuePFN places the queue outside RAM")
        p.set(SIZE, p.ld(8, QP, 8))
        p.set(AVAIL, DESC + SIZE * 16)
        p.set(USED, AVAIL + (SIZE + 3) * 2)
        p.set(N1, p.ld(8, QP, 16))
        p.set(USED, (USED + N1 - 1) & ((N1 - 1) ^ MASK))
        with p.If(USED + SIZE * 8 + 6, "gtu", TOP + base):
            p.refuse("QueuePFN places the queue outside RAM")
        p.st(8, QP, 0, VV)
        p.st(8, QP, 24, 0)
        p.ret()
    with p.If(OFF, "eq", 0x064):
        p.set(ISR, ISR & (VV ^ 0xFFFFFFFF))
        p.ret()
    with p.If(OFF, "ne", 0x050):
        p.refuse("a register this phase does not answer")

    # ---------------- QueueNotify: serve queue VV
    with p.If(VV, "geu", 2):
        p.refuse("QueueNotify names a queue the device does not have")
    qptr(VV)
    with p.If(p.ld(8, QP, 0), "eq", 0):
        p.refuse("QueueNotify for a queue that is not set up")
    p.set(SIZE, p.ld(8, QP, 8))
    p.set(DESC, p.ld(8, QP, 0) * PAGE)
    p.set(AVAIL, DESC + SIZE * 16)
    p.set(USED, AVAIL + (SIZE + 3) * 2)
    p.set(N1, p.ld(8, QP, 16))
    p.set(USED, (USED + N1 - 1) & ((N1 - 1) ^ MASK))
    p.set(LAST, p.ld(8, QP, 24))
    p.set(SERVED, 0)
    p.set(SGP, p.lea("SEGS"))
    p.set(RPY, p.lea("REPLY"))
    with p.While(lambda: (0, "eq", 0)):
        with p.If(p.ld(2, AVAIL, 2), "eq", LAST):
            p.brk()
        with p.If(USED, "ltu", CE):
            p.refuse("the used ring lies in the code span")
        p.set(HEAD, p.ld(2, AVAIL + ((LAST & (SIZE - 1)) * 2), 4))
        with p.If(HEAD, "geu", SIZE):
            p.refuse("the available ring offered a head outside the table")
        # ---- the chain
        p.set(CNT, 0)
        p.set(SEENW, 0)
        p.set(TOTAL, 0)
        p.set(CI, HEAD)
        L_chain = p.newl("CH")
        p.label(L_chain)
        with p.If(CNT, "geu", SIZE):
            p.refuse("the chain loops")
        p.set(DA, DESC + CI * 16)
        p.set(DN, p.ld(8, DA, 0))                  # addr
        p.set(DL, p.ld(4, DA, 8))                  # len
        p.set(DF, p.ld(2, DA, 12))
        p.set(DW, p.ld(2, DA, 14))                 # next
        with p.If(DF & 4, "ne", 0):
            p.refuse("an INDIRECT descriptor")
        p.set(N2, (DF >> 1) & 1)                   # write
        with p.If(SEENW, "ne", 0):
            with p.If(N2, "eq", 0):
                p.refuse("a readable descriptor follows a writable one")
        p.set(SEENW, SEENW | N2)
        p.set(TOTAL, TOTAL + DL)
        with p.If(TOTAL, "gtu", 0xFFFFFFFF):
            p.refuse("the chain's buffers exceed 2^32")
        with p.If(DN, "ltu", base):
            p.refuse("a buffer is outside RAM")
        p.set(N3, DN - base)
        p.set(N4, N3 + DL)
        with p.If(N4, "ltu", N3):
            p.refuse("a buffer is outside RAM")
        with p.If(N4, "gtu", TOP):
            p.refuse("a buffer is outside RAM")
        p.set(N1, SGP + CNT * 16)
        p.st(8, N1, 0, DN)
        p.st(4, N1, 8, DL)
        p.st(4, N1, 12, N2)
        p.set(CNT, CNT + 1)
        with p.If(DF & 1, "ne", 0):
            with p.If(DW, "geu", SIZE):
                p.refuse("a descriptor's next is outside the table")
            p.set(CI, DW)
            p.jmp(L_chain)
        # ---- the request: the readable segments, concatenated
        p.set(RL, 0)
        p.set(HV, 0)
        p.set(J, 0)
        with p.While(lambda: (J, "ltu", CNT)):
            p.set(N1, SGP + J * 16)
            p.set(N2, p.ld(4, N1, 12))
            with p.If(N2, "eq", 0):
                p.set(RL, RL + p.ld(4, N1, 8))
            with p.If(N2, "ne", 0):
                p.set(HV, HV + p.ld(4, N1, 8))
            p.set(J, J + 1)
        p.set(RQSZ, RL)
        with p.If(RL, "ltu", 4096):
            p.set(RQSZ, 4096)
        p.set(RQ, p.sys("mmap", 0, RQSZ, 3, 0x22, MASK, 0))
        with p.If(RQ, "geu", ERR):
            p.refuse("no memory for the request")
        p.set(POS, 0)
        p.set(J, 0)
        with p.While(lambda: (J, "ltu", CNT)):
            p.set(N1, SGP + J * 16)
            with p.If(p.ld(4, N1, 12), "eq", 0):
                p.copy(RQ + POS, p.ld(8, N1, 0), p.ld(4, N1, 8))
                p.set(POS, POS + p.ld(4, N1, 8))
            p.set(J, J + 1)
        gpu(p, g, base)
        p.sys("munmap", RQ, RQSZ)
        p.end()
        with p.If(REPLEN, "gtu", HV):
            p.refuse("the answer is longer than the chain's writable space")
        # ---- scatter the reply into the writable segments
        p.set(POS, 0)
        p.set(J, 0)
        with p.While(lambda: (J, "ltu", CNT)):
            p.set(N1, SGP + J * 16)
            with p.If(p.ld(4, N1, 12), "ne", 0):
                p.set(N2, REPLEN - POS)
                p.set(N3, p.ld(4, N1, 8))
                with p.If(N3, "gtu", N2):
                    p.set(N3, N2)
                with p.If(N3, "ne", 0):
                    with p.If(p.ld(8, N1, 0), "ltu", CE):
                        p.refuse("the answer lands in the code span")
                    p.copy(p.ld(8, N1, 0), RPY + POS, N3)
                    p.set(POS, POS + N3)
            p.set(J, J + 1)
        p.set(N1, p.ld(2, USED, 2))
        p.set(N2, USED + ((N1 & (SIZE - 1)) * 8) + 4)
        p.st(4, N2, 0, HEAD)
        p.st(4, N2, 4, REPLEN)
        p.st(2, USED, 2, (N1 + 1) & 0xFFFF)
        p.set(LAST, (LAST + 1) & 0xFFFF)
        p.set(SERVED, 1)
    p.st(8, QP, 24, LAST)
    with p.If(SERVED, "ne", 0):
        p.set(ISR, ISR | 1)
    p.ret()
    return p


def gpu(p, g, base):
    """Gpu::serve: the request is RQ[0..RL), the reply goes to REPLY[0..REPLEN)."""
    C = g
    RQ, RL, REPLEN, RC, FNC, FL, BODY, BL = (C[n] for n in "RQ RL REPLEN RC FNC FL BODY BL".split())
    RPY, RP, K, E, NRES, HELD, SCO, SCX, SCY, SCW, SCH = (C[n] for n in "RPY RP K E NRES HELD SCO SCX SCY SCW SCH".split())
    RX, RY, RW, RH, N1, N2, N3, N4, ID, DUMP, TOP = (C[n] for n in "RX RY RW RH N1 N2 N3 N4 ID DUMP TOP".split())
    OFFS, ROWB, STRIDE, CV, BKN = C["OFFS"], C["ROWB"], C["STRIDE"], C["CV"], C["BKN"]
    PB, PO, PT, PC, PX, PY, PROW, PFD, PTOT, PL = (C[n] for n in "PB PO PT PC PX PY PROW PFD PTOT PL".split())
    ERR = 0xFFFFFFFFFFFFF000
    L_fin, L_unspec = p.newl("FIN"), p.newl("UNS")

    def err(code):
        p.set(RC, code)
        p.jmp(L_fin)

    def find(idv):
        p.set(RP, 0)
        p.set(K, 0)
        with p.While(lambda: (K, "ltu", NRES)):
            p.set(E, p.lea("RES") + K * RES_STRIDE)
            with p.If(p.ld(8, E, 0), "eq", idv):
                p.set(RP, E)
                p.brk()
            p.set(K, K + 1)

    def rect(o):                       # RX RY RW RH from BODY+o
        p.set(RX, p.ld(4, BODY, o))
        p.set(RY, p.ld(4, BODY, o + 4))
        p.set(RW, p.ld(4, BODY, o + 8))
        p.set(RH, p.ld(4, BODY, o + 12))

    def covered_or(code):              # RP's resource covers rect, else err(code)
        with p.If(RX + RW, "gtu", p.ld(8, RP, 16)):
            err(code)
        with p.If(RY + RH, "gtu", p.ld(8, RP, 24)):
            err(code)

    p.set(REPLEN, 24)
    p.set(FNC, 0)
    p.set(FL, 0)
    p.set(RC, 0x1100)
    p.set(RPY, p.lea("REPLY"))
    for o in range(0, 512, 8):
        p.st(8, RPY, o, 0)
    p.set(BODY, RQ + 24)
    p.set(BL, RL - 24)
    with p.If(RL, "ltu", 24):
        p.jmp(L_unspec)
    p.set(FL, p.ld(4, RQ, 4))
    with p.If(FL & 0xFFFFFFFC, "ne", 0):
        p.jmp(L_unspec)
    p.set(N1, p.ld(4, RQ, 0))                          # kind
    # ---- GET_DISPLAY_INFO
    with p.If(N1, "eq", 0x100):
        p.set(FNC, 1)
        p.set(REPLEN, 408)
        p.st(4, RPY, 24 + 8, 640)
        p.st(4, RPY, 24 + 12, 480)
        p.st(4, RPY, 24 + 16, 1)
        err(0x1101)
    # ---- RESOURCE_CREATE_2D
    with p.If(N1, "eq", 0x101):
        with p.If(BL, "ltu", 16):
            p.jmp(L_unspec)
        p.set(ID, p.ld(4, BODY, 0))
        with p.If(ID, "eq", 0):
            p.jmp(L_unspec)
        p.set(N2, p.ld(4, BODY, 4))                    # format
        L_fok = p.newl("FOK")
        for f in (1, 2, 3, 4, 67, 68, 121, 134):
            p.br(N2, "eq", f, L_fok)
        p.jmp(L_unspec)
        p.label(L_fok)
        p.set(RW, p.ld(4, BODY, 8))
        p.set(RH, p.ld(4, BODY, 12))
        with p.If(RW, "eq", 0):
            p.jmp(L_unspec)
        with p.If(RH, "eq", 0):
            p.jmp(L_unspec)
        p.set(FNC, 1)
        find(ID)
        with p.If(RP, "ne", 0):
            err(0x1203)
        with p.If(N2, "gtu", 2):
            err(0x1205)
        with p.If(RW * RH, "gtu", 0x1000000):
            err(0x1201)
        p.set(N3, RW * RH * 4)
        with p.If(HELD + N3, "gtu", 0x10000000):
            err(0x1201)
        with p.If(NRES, "geu", RES_N):
            p.refuse("more than 256 resources")
        p.set(N4, p.sys("mmap", 0, N3, 3, 0x22, MASK, 0))
        with p.If(N4, "geu", ERR):
            p.refuse("no memory for a resource")
        p.set(E, p.lea("RES") + NRES * RES_STRIDE)
        p.st(8, E, 0, ID)
        p.st(8, E, 8, N2)
        p.st(8, E, 16, RW)
        p.st(8, E, 24, RH)
        p.st(8, E, 32, N4)
        p.st(8, E, 40, N3)
        p.st(8, E, 48, 0)
        p.st(8, E, 56, 0)
        p.st(8, E, 64, 0)
        p.set(NRES, NRES + 1)
        p.set(HELD, HELD + N3)
        err(0x1100)
    # ---- RESOURCE_ATTACH_BACKING
    with p.If(N1, "eq", 0x106):
        with p.If(BL, "ltu", 8):
            p.jmp(L_unspec)
        p.set(ID, p.ld(4, BODY, 0))
        with p.If(ID, "eq", 0):
            p.jmp(L_unspec)
        p.set(BKN, p.ld(4, BODY, 4))
        with p.If(BKN, "eq", 0):
            p.jmp(L_unspec)
        with p.If(BL, "ltu", BKN * 16 + 8):
            p.jmp(L_unspec)
        p.set(FNC, 1)
        find(ID)
        with p.If(RP, "eq", 0):
            err(0x1203)
        p.set(N2, 0)                                   # total
        p.set(K, 0)
        with p.While(lambda: (K, "ltu", BKN)):
            p.set(E, BODY + 8 + K * 16)
            p.set(N3, p.ld(8, E, 0))
            p.set(N4, p.ld(4, E, 8))
            with p.If(N3, "ltu", base):
                err(0x1205)
            with p.If(N3 + N4, "ltu", N3):
                err(0x1205)
            with p.If(N3 + N4, "gtu", TOP + base):
                err(0x1205)
            p.set(N2, N2 + N4)
            p.set(K, K + 1)
        with p.If(N2, "gtu", TOP):
            err(0x1205)
        with p.If(p.ld(8, RP, 56), "ne", 0):
            p.sys("munmap", p.ld(8, RP, 48), p.ld(8, RP, 56) * 16)
            p.end()
        p.set(N3, p.sys("mmap", 0, BKN * 16, 3, 0x22, MASK, 0))
        with p.If(N3, "geu", ERR):
            p.refuse("no memory for a backing list")
        p.copy(N3, BODY + 8, BKN * 16)
        p.st(8, RP, 48, N3)
        p.st(8, RP, 56, BKN)
        p.st(8, RP, 64, N2)
        err(0x1100)
    # ---- SET_SCANOUT
    with p.If(N1, "eq", 0x103):
        with p.If(BL, "ltu", 24):
            p.jmp(L_unspec)
        p.set(FNC, 1)
        rect(0)
        with p.If(p.ld(4, BODY, 16), "ne", 0):
            err(0x1202)
        p.set(ID, p.ld(4, BODY, 20))
        with p.If(ID, "eq", 0):
            p.set(SCO, 0)
            err(0x1100)
        find(ID)
        with p.If(RP, "eq", 0):
            err(0x1203)
        covered_or(0x1205)
        p.set(SCO, ID)
        p.set(SCX, RX)
        p.set(SCY, RY)
        p.set(SCW, RW)
        p.set(SCH, RH)
        err(0x1100)
    # ---- TRANSFER_TO_HOST_2D
    with p.If(N1, "eq", 0x105):
        with p.If(BL, "ltu", 32):
            p.jmp(L_unspec)
        p.set(ID, p.ld(4, BODY, 24))
        with p.If(ID, "eq", 0):
            p.jmp(L_unspec)
        p.set(FNC, 1)
        rect(0)
        p.set(OFFS, p.ld(8, BODY, 16))
        find(ID)
        with p.If(RP, "eq", 0):
            err(0x1203)
        with p.If(p.ld(8, RP, 56), "eq", 0):
            err(0x1200)
        covered_or(0x1205)
        p.set(STRIDE, p.ld(8, RP, 16) * 4)
        p.set(ROWB, RW * 4)
        with p.If(RH, "ne", 0):
            p.set(N2, OFFS + (RH - 1) * STRIDE)
            with p.If(N2, "ltu", OFFS):
                err(0x1205)
            p.set(N3, N2 + ROWB)
            with p.If(N3, "ltu", N2):
                err(0x1205)
            with p.If(N3, "gtu", p.ld(8, RP, 64)):
                err(0x1205)
        p.set(CV, 0)                                   # k
        with p.While(lambda: (CV, "ltu", RH)):
            # destination: pixels + (y+k)*stride + x*4
            p.set(PX, p.ld(8, RP, 32) + (RY + CV) * STRIDE + RX * 4)
            p.set(PY, OFFS + CV * STRIDE)              # the offset into the backing
            p.set(PO, 0)                               # filled
            p.set(K, 0)
            with p.While(lambda: (PO, "ltu", ROWB)):
                p.set(E, p.ld(8, RP, 48) + K * 16)
                p.set(N2, p.ld(4, E, 8))
                with p.If(PY, "geu", N2):
                    p.set(PY, PY - N2)
                    p.set(K, K + 1)
                    p.cont()
                p.set(N3, N2 - PY)
                p.set(N4, ROWB - PO)
                with p.If(N3, "gtu", N4):
                    p.set(N3, N4)
                p.copy(PX + PO, p.ld(8, E, 0) + PY, N3)
                p.set(PO, PO + N3)
                p.set(PY, 0)
                p.set(K, K + 1)
            p.set(CV, CV + 1)
        err(0x1100)
    # ---- RESOURCE_FLUSH
    with p.If(N1, "eq", 0x104):
        with p.If(BL, "ltu", 24):
            p.jmp(L_unspec)
        p.set(ID, p.ld(4, BODY, 16))
        with p.If(ID, "eq", 0):
            p.jmp(L_unspec)
        p.set(FNC, 1)
        rect(0)
        find(ID)
        with p.If(RP, "eq", 0):
            err(0x1203)
        covered_or(0x1205)
        with p.If(SCO, "ne", ID):
            err(0x1100)
        with p.If(DUMP, "eq", 0):
            err(0x1100)
        writeppm(p, g)
        err(0x1100)
    p.jmp(L_unspec)                                    # unknown / unsupported kinds
    # ---- parse failures: ERR_UNSPEC, no fence
    p.label(L_unspec)
    p.set(FNC, 0)
    p.set(RC, 0x1200)
    p.label(L_fin)
    p.st(4, RPY, 0, RC)
    with p.If(FNC, "ne", 0):
        with p.If(FL & 1, "ne", 0):
            p.st(4, RPY, 4, 1)
            p.st(8, RPY, 8, p.ld(8, RQ, 8))


def writeppm(p, g):
    C = g
    RP, SCX, SCY, SCW, SCH, DUMP = (C[n] for n in "RP SCX SCY SCW SCH DUMP".split())
    PB, PO, PT, PC, PX, PY, PROW, PFD, PTOT, PL = (C[n] for n in "PB PO PT PC PX PY PROW PFD PTOT PL".split())
    ERR = 0xFFFFFFFFFFFFF000

    def itoa(n):                        # PO = write pointer; digits of cell n; PO advanced
        p.set(PT, n)
        p.set(PC, 1)
        with p.While(lambda: (PT, "geu", 10)):
            p.set(PT, PT // 10)
            p.set(PC, PC + 1)
        p.set(PT, n)
        p.set(PL, PC)
        with p.While(lambda: (PL, "ne", 0)):
            p.set(PL, PL - 1)
            p.st(1, PO + PL, 0, 48 + PT % 10)
            p.set(PT, PT // 10)
        p.set(PO, PO + PC)

    p.set(PTOT, 32 + SCW * SCH * 3)
    p.set(PB, p.sys("mmap", 0, PTOT, 3, 0x22, MASK, 0))
    with p.If(PB, "geu", ERR):
        p.refuse("no memory for the scanout file")
    p.set(PO, PB)
    p.st(1, PO, 0, 0x50)
    p.st(1, PO, 1, 0x36)
    p.st(1, PO, 2, 10)
    p.set(PO, PO + 3)
    itoa(SCW)
    p.st(1, PO, 0, 0x20)
    p.set(PO, PO + 1)
    itoa(SCH)
    p.st(1, PO, 0, 10)
    p.st(1, PO, 1, 0x32)
    p.st(1, PO, 2, 0x35)
    p.st(1, PO, 3, 0x35)
    p.st(1, PO, 4, 10)
    p.set(PO, PO + 5)
    p.set(PY, SCY)
    p.set(PT, SCY + SCH)
    with p.While(lambda: (PY, "ltu", PT)):
        p.set(PROW, p.ld(8, RP, 32) + PY * (p.ld(8, RP, 16) * 4) + SCX * 4)
        p.set(PX, 0)
        with p.While(lambda: (PX, "ltu", SCW)):
            p.set(PC, PROW + PX * 4)
            p.st(1, PO, 0, p.ld(1, PC, 2))
            p.st(1, PO, 1, p.ld(1, PC, 1))
            p.st(1, PO, 2, p.ld(1, PC, 0))
            p.set(PO, PO + 3)
            p.set(PX, PX + 1)
        p.set(PY, PY + 1)
    p.set(PFD, p.sys("open", DUMP, 0x241, 0x1B6))
    with p.If(PFD, "geu", ERR):
        p.refuse("the scanout file could not be opened")
    p.set(PL, PO - PB)                  # bytes left
    p.set(PC, PB)
    with p.While(lambda: (PL, "ne", 0)):
        p.set(PX, p.sys("write", PFD, PC, PL))
        with p.If(PX, "eq", 0):
            p.refuse("the scanout file could not be written")
        with p.If(PX, "geu", ERR):
            p.refuse("the scanout file could not be written")
        p.set(PC, PC + PX)
        p.set(PL, PL - PX)
    p.sys("close", PFD)
    p.end()
    p.sys("munmap", PB, PTOT)
    p.end()


# ------------------------------------------------------------------------- x86-64 backend
SYS_X86 = {"mmap": 9, "munmap": 11, "open": 2, "write": 1, "close": 3}
JCC = {"eq": "e", "ne": "ne", "ltu": "b", "geu": "ae", "gtu": "a", "leu": "be"}


def emit_x86(A, P, base, RAX, RCX, RDX, RBX, RSI, RDI, R8, R9, R10, R11, sc_size):
    """the entries rt_sc_st / rt_sc_ld, the program, the refusal stubs and the strings. The block is
    the data symbol `gp`; rbx = its address while the program runs; rax rcx rdx rsi rdi r8..r11 are
    scratch (rbx rsi rdi r8..r11 saved on the host stack)."""
    saved = (RBX, RSI, RDI, R8, R9, R10, R11)

    def LC(r, o):
        if o[0] == "c":
            A.mov_r_rm(r, ("m", RBX, o[1]))
        else:
            A.mov_r_imm64(r, o[1])

    def SC(o, r):
        A.mov_rm_r(("m", RBX, o[1]), r)

    for kind in ("st", "ld"):
        A.label("rt_sc_" + kind)
        if kind == "st":
            A.push(RCX)
            A.mov_r_rm(RDX, ("abs", "fault_off", 0), 0)
            A.mov_r_imm64(RCX, base)
            A.alu_r_rm("add", RDX, ("r", RCX))
            A.mov_r_rm(RDX, ("m", RDX, 0), 0)
            A.shift_imm("shr", ("r", RDX), 12, 0)
            A.alu_rm_imm("and", ("r", RDX), 3, 0)
            A.pop(RCX)
            A.alu_rm_imm("cmp", ("r", RDX), 2, 0)
        else:
            A.alu_rm_imm("cmp", ("r", RDX), 4, 0)
        A.mov_rm_imm32(("r", RDX), 0, 0)
        A.jcc("ne", "rt_sc_w" + kind)
        A.mov_rm_imm32(("r", RDX), 1, 0)
        A.label("rt_sc_w" + kind)
        for r in saved:
            A.push(r)
        A.lea(RBX, ("abs", "gp", 0))
        A.mov_rm_r(("m", RBX, P.names["ADDR"]), RAX)
        A.mov_rm_r(("m", RBX, P.names["VAL"]), RCX)
        A.mov_rm_r(("m", RBX, P.names["WOK"]), RDX)
        A.mov_rm_imm32(("m", RBX, P.names["ISST"]), 1 if kind == "st" else 0)
        A.mov_r_rm(RAX, ("abs", "top", 0))
        A.mov_rm_r(("m", RBX, P.names["TOP"]), RAX)
        A.mov_r_rm(RAX, ("abs", "code_end", 0))
        A.mov_rm_r(("m", RBX, P.names["CE"]), RAX)
        A.jmp("rt_sc_prog")
    A.label("rt_sc_prog")
    why = {}
    for op in P.ops:
        k = op[0]
        if k == "label":
            A.label("sc_" + op[1])
        elif k == "jmp":
            A.jmp("sc_" + op[1])
        elif k == "mov":
            LC(RAX, op[2])
            SC(op[1], RAX)
        elif k == "bin":
            _, o, d, a, b = op
            LC(RAX, a)
            LC(RCX, b)
            if o in ("add", "sub", "and", "or", "xor"):
                A.alu_r_rm(o, RAX, ("r", RCX))
            elif o in ("shl", "shr"):
                A.shift_cl(o, ("r", RAX))
            elif o == "mul":
                A.imul_r_rm(RAX, ("r", RCX))
            else:
                A.mov_rm_imm32(("r", RDX), 0)
                A.f7(6, ("r", RCX))
                if o == "remu":
                    A.mov_r_rm(RAX, ("r", RDX))
            SC(d, RAX)
        elif k == "ld":
            _, w, d, a, off = op
            LC(RAX, a)
            m = ("m", RAX, off)
            if w in (1, 2):
                A.movzx(RAX, m, w)
            elif w == 4:
                A.mov_r_rm(RAX, m, 0)
            else:
                A.mov_r_rm(RAX, m)
            SC(d, RAX)
        elif k == "st":
            _, w, a, off, b = op
            LC(RAX, a)
            LC(RCX, b)
            A.mov_rm_r(("m", RAX, off), RCX, width=w)
        elif k == "lea":
            A.lea(RAX, ("m", RBX, op[2]))
            SC(op[1], RAX)
        elif k == "br":
            _, cc, a, b, L = op
            LC(RAX, a)
            LC(RCX, b)
            A.alu_r_rm("cmp", RAX, ("r", RCX))
            A.jcc(JCC[cc], "sc_" + L)
        elif k == "sys":
            _, d, name, args = op
            for r, a in zip((RDI, RSI, RDX, R10, R8, R9), args):
                LC(r, a)
            A.mov_rm_imm32(("r", RAX), SYS_X86[name])
            A.syscall()
            SC(d, RAX)
        elif k == "copy":
            _, dst, src, n = op
            LC(RDI, dst)
            LC(RSI, src)
            LC(RCX, n)
            A.raw(0xF3, 0xA4)
        elif k == "getenv":
            A.lea(RDI, ("abs", "sc_env_" + str(P.envs.index(op[2])), 0))
            A.call("rt_getenv")
            SC(op[1], RAX)
        elif k == "refuse":
            A.jmp("sc_why%d" % P.reasons.index(op[1]))
        elif k == "ret":
            A.jmp("sc_epi")
        else:
            raise ValueError(op)
    A.label("sc_epi")
    A.lea(RAX, ("m", RBX, P.names["RV"]))
    for r in reversed(saved):
        A.pop(r)
    A.ret()
    for i, text in enumerate(P.reasons):
        msg = ("native: not in milestone 1 (virtio-gpu: " + text + "; run it under yantra-run)\n").encode()
        A.label("sc_why%d" % i)
        A.lea(RSI, ("abs", "sc_why_s%d" % i, 0))
        A.mov_rm_imm32(("r", RDX), len(msg))
        A.mov_rm_imm32(("r", RDI), 2)
        A.mov_rm_imm32(("r", RAX), 1)
        A.syscall()
        A.mov_rm_imm32(("r", RDI), 2)
        A.jmp("rt_exit")
    for i, text in enumerate(P.reasons):
        msg = ("native: not in milestone 1 (virtio-gpu: " + text + "; run it under yantra-run)\n").encode()
        A.label("sc_why_s%d" % i)
        A.raw(msg)
    for i, name in enumerate(P.envs):
        A.label("sc_env_%d" % i)
        A.raw(name.encode() + b"\0")


# ------------------------------------------------------------------------- aarch64 backend
SYS_A64 = {"mmap": 222, "munmap": 215, "open": 56, "write": 64, "close": 57}


def emit_a64(A, P, base, FR, CC):
    """the entries rt_sc_st / rt_sc_ld (bl'd-from-cold-stub contract: T0 = addr, T1 = value, x30 = the
    stub's return), the program, the refusal stubs and the strings. The block lives at sp + FR["scs"]
    (host x18 = its address while the program runs). x0..x13 and x30 are saved in cells and restored;
    x0..x8 are the program's registers (x9..x13 are rt_getenv's); x14 and x19.. are never touched."""
    SPR, ZR, T0, T1, T2, BP = 31, 31, 16, 17, 15, 18
    sv = [P.cell("SV%d" % i).off for i in range(15)]          # x0..x13, x30
    saved = list(range(14)) + [30]

    def LC(r, o):
        if o[0] == "c":
            A.mem("ldr", r, BP, o[1])
        else:
            A.movi(r, o[1])

    def SC(o, r):
        A.mem("str", r, BP, o[1])

    def blk(r, off):                                          # r = BP + off
        A.movi(r, off)
        A.rrr("add", r, BP, r)

    nc = [0]
    for kind in ("st", "ld"):
        A.label("rt_sc_" + kind)
        A.from_sp(BP)
        A.movi(T2, FR["scs"])
        A.rrr("add", BP, BP, T2)
        for r, off in zip(saved, sv):
            A.mem("str", r, BP, off)
        A.mem("str", T0, BP, P.names["ADDR"])
        A.mem("str", T1 if kind == "st" else ZR, BP, P.names["VAL"])
        A.mem("ldr", 0, SPR, FR["fault_off"])
        A.movl(1, "image")
        A.rrr("add", 0, 0, 1)
        A.mem("ldrw", 0, 0, 0)
        A.ubfm(0, 0, 12, 13)
        A.cmp_i(0, 2)
        A.cset(1, CC["eq"])
        A.mem("str", 1, BP, P.names["WOK"])
        A.movz(1, 1 if kind == "st" else 0)
        A.mem("str", 1, BP, P.names["ISST"])
        A.mem("ldr", 0, SPR, FR["top"])
        A.mem("str", 0, BP, P.names["TOP"])
        A.mem("ldr", 0, SPR, FR["code_end"])
        A.mem("str", 0, BP, P.names["CE"])
        A.b_("rt_sc_prog")
    A.label("rt_sc_prog")
    RRR = {"add": "add", "sub": "sub", "and": "and", "or": "orr", "xor": "eor"}
    LDK = {1: "ldrb", 2: "ldrh", 4: "ldrw", 8: "ldr"}
    STK = {1: "strb", 2: "strh", 4: "strw", 8: "str"}
    for op in P.ops:
        k = op[0]
        if k == "label":
            A.label("sc_" + op[1])
        elif k == "jmp":
            A.b_("sc_" + op[1])
        elif k == "mov":
            LC(0, op[2])
            SC(op[1], 0)
        elif k == "bin":
            _, o, d, a, b = op
            LC(0, a)
            LC(1, b)
            if o in RRR:
                A.rrr(RRR[o], 0, 0, 1)
            elif o == "shl":
                A.dp2("lslv", 0, 0, 1)
            elif o == "shr":
                A.dp2("lsrv", 0, 0, 1)
            elif o == "mul":
                A.madd(0, 0, 1, ZR)
            elif o == "divu":
                A.dp2("udiv", 0, 0, 1)
            else:
                A.dp2("udiv", 2, 0, 1)
                A.msub(0, 2, 1, 0)
            SC(d, 0)
        elif k == "ld":
            _, w, d, a, off = op
            LC(0, a)
            A.mem(LDK[w], 0, 0, off)
            SC(d, 0)
        elif k == "st":
            _, w, a, off, b = op
            LC(0, a)
            LC(1, b)
            A.mem(STK[w], 1, 0, off)
        elif k == "lea":
            blk(0, op[2])
            SC(op[1], 0)
        elif k == "br":
            _, cc, a, b, L = op
            LC(0, a)
            LC(1, b)
            A.cmp(0, 1)
            A.bcond(CC[cc], "sc_" + L)
        elif k == "sys":
            _, d, name, args = op
            if name == "open":
                A.movi(0, -100)
                for r, a in zip((1, 2, 3), args):
                    LC(r, a)
            else:
                for r, a in zip(range(6), args):
                    LC(r, a)
            A.movz(8, SYS_A64[name])
            A.svc()
            SC(d, 0)
        elif k == "copy":
            _, dst, src, n = op
            LC(0, dst)
            LC(1, src)
            LC(2, n)
            nc[0] += 1
            lp, en = "sc_cp%d" % nc[0], "sc_cpe%d" % nc[0]
            A.label(lp)
            A.cbz(2, en)
            A.mem("ldrb", 3, 1, 0)
            A.mem("strb", 3, 0, 0)
            A.add_i(0, 0, 1)
            A.add_i(1, 1, 1)
            A.add_i(2, 2, -1)
            A.b_(lp)
            A.label(en)
        elif k == "getenv":
            A.movl(0, "sc_env_" + str(P.envs.index(op[2])))
            A.bl("rt_getenv")
            SC(op[1], 0)
        elif k == "refuse":
            A.b_("sc_why%d" % P.reasons.index(op[1]))
        elif k == "ret":
            A.b_("sc_epi")
        else:
            raise ValueError(op)
    A.label("sc_epi")
    blk(T0, P.names["RV"])
    for r, off in zip(saved, sv):
        A.mem("ldr", r, BP, off)
    A.ret()
    for i, text in enumerate(P.reasons):
        msg = ("native: not in milestone 1 (virtio-gpu: " + text + "; run it under yantra-run)\n").encode()
        A.label("sc_why%d" % i)
        A.movz(0, 2)
        A.movl(1, "sc_why_s%d" % i)
        A.movi(2, len(msg))
        A.movz(8, 64)
        A.svc()
        A.movz(0, 2)
        A.b_("rt_exit")
    for i, text in enumerate(P.reasons):
        msg = ("native: not in milestone 1 (virtio-gpu: " + text + "; run it under yantra-run)\n").encode()
        A.label("sc_why_s%d" % i)
        A.raw(msg)
    for i, name in enumerate(P.envs):
        A.label("sc_env_%d" % i)
        A.raw(name.encode() + b"\0")
    A.raw(b"\0" * (-len(A.b) % 4))
