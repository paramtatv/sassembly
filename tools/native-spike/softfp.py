#!/usr/bin/env python3
# softfp.py - the software F/D engine (RV64 rounding mode RMM, which no host FPU has), written ONCE as a small
# program over a portable instruction set, then compiled by a backend: anuvada.py's x86-64 and A64 backends emit
# machine code from it; the Interp backend below RUNS it, so the algorithm can be checked against an exact
# rational reference (python -c "import softfp; softfp.selftest()") before it is ever assembled.
#
# THE ALGORITHM IS yantra's (crates/yantra/src/fp.rs, section "EXACT IEEE 754 ROUNDING"): unpack, form the exact
# result as +-mant x 2^exp (mant up to 128 bits, with a sticky bit jammed in the lowest bit when the exact value is
# longer), round it ONCE with round_pack under the mode, raise NX / OF / UF with tininess detected AFTER rounding.
# It handles all five modes, but only RMM is routed to it (the others are the host FPU's).
#
# THE INSTRUCTION SET: 64-bit variables in a frame (named by strings; an int is an immediate), wrapping arithmetic
#   set mov add sub mul umulh and_ or_ xor shl shr sar clz            d = a op b   (shifts: a count >= 64 gives 0,
#   (sar: the sign); clz of 0 is 64)
#   br(cond, a, b, label)  cond: eq ne ltu leu gtu geu lts les gts ges        jmp label   call label   ret   label
# No division (the long division is a bit loop), no 128-bit primitive (pairs of variables), no stack frames:
# subroutines pass arguments in named variables.
import itertools

NX, UF, OF, DZ, NV = 1, 2, 4, 8, 16
ZERO, FIN, INF, QNAN, SNAN, INVALID = 0, 1, 2, 3, 4, 5
M64 = (1 << 64) - 1
RNE, RTZ, RDN, RUP, RMM = range(5)


def engine(P):
    """emit the whole engine through the builder P (methods: lab label jmp br call ret set mov add sub mul umulh
    and_ or_ xor shl shr sar clz)"""
    L = P.lab

    def copy_u(X):
        for f in ("cls", "neg", "h", "l", "e"):
            P.mov("%s_%s" % (X, f), "u_" + f)

    def copy(Y, X):
        for f in ("cls", "neg", "h", "l", "e"):
            P.mov("%s_%s" % (Y, f), "%s_%s" % (X, f))

    # ------------------------------------------------------------ the formats
    P.label("sf_fmt_d")
    for k, v in (("f_p", 53), ("f_w", 64), ("f_emin", -1022), ("f_emax", 1023), ("f_fb", 52), ("f_expmax", 0x7FF),
                 ("f_sgnsh", 63), ("f_fracmask", (1 << 52) - 1), ("f_hidden", 1 << 52), ("f_qbit", 1 << 51),
                 ("f_inf", 0x7FF0000000000000), ("f_nan", 0x7FF8000000000000), ("f_bias", 1023)):
        P.set(k, v)
    P.ret()
    P.label("sf_fmt_s")
    for k, v in (("f_p", 24), ("f_w", 32), ("f_emin", -126), ("f_emax", 127), ("f_fb", 23), ("f_expmax", 0xFF),
                 ("f_sgnsh", 31), ("f_fracmask", (1 << 23) - 1), ("f_hidden", 1 << 23), ("f_qbit", 1 << 22),
                 ("f_inf", 0x7F800000), ("f_nan", 0x7FC00000), ("f_bias", 127)):
        P.set(k, v)
    P.ret()

    # ------------------------------------------------------------ unpack(u_x) -> u_cls u_neg u_h u_l u_e
    P.label("sf_unpack")
    P.shr("u_neg", "u_x", "f_sgnsh")
    P.and_("u_neg", "u_neg", 1)
    P.shr("u_e0", "u_x", "f_fb")
    P.and_("u_e0", "u_e0", "f_expmax")
    P.and_("u_fr", "u_x", "f_fracmask")
    P.set("u_h", 0)
    P.set("u_l", 0)
    P.set("u_e", 0)
    nn, nan, q, norm, sub = L(), L(), L(), L(), L()
    P.br("ne", "u_e0", "f_expmax", nn)
    P.br("ne", "u_fr", 0, nan)
    P.set("u_cls", INF)
    P.ret()
    P.label(nan)
    P.and_("u_t", "u_fr", "f_qbit")
    P.br("ne", "u_t", 0, q)
    P.set("u_cls", SNAN)
    P.ret()
    P.label(q)
    P.set("u_cls", QNAN)
    P.ret()
    P.label(nn)
    P.br("ne", "u_e0", 0, norm)
    P.br("ne", "u_fr", 0, sub)
    P.set("u_cls", ZERO)
    P.ret()
    P.label(sub)
    P.set("u_cls", FIN)
    P.mov("u_l", "u_fr")
    P.sub("u_e", "f_emin", "f_fb")
    P.ret()
    P.label(norm)
    P.set("u_cls", FIN)
    P.or_("u_l", "u_fr", "f_hidden")
    P.sub("u_e", "u_e0", "f_bias")
    P.sub("u_e", "u_e", "f_fb")
    P.ret()

    # ------------------------------------------------------------ bitlen(bl_h:bl_l) -> bl_n
    P.label("sf_bitlen")
    hi, z = L(), L()
    P.br("ne", "bl_h", 0, hi)
    P.br("eq", "bl_l", 0, z)
    P.clz("bl_t", "bl_l")
    P.sub("bl_n", 64, "bl_t")
    P.ret()
    P.label(hi)
    P.clz("bl_t", "bl_h")
    P.sub("bl_n", 128, "bl_t")
    P.ret()
    P.label(z)
    P.set("bl_n", 0)
    P.ret()

    # ------------------------------------------------------------ sj: (sj_h:sj_l) >>= sj_n, the bits lost jammed into bit 0
    P.label("sf_shrj")
    done, mid, lo, zz = L(), L(), L(), L()
    P.br("eq", "sj_n", 0, done)
    P.br("ltu", "sj_n", 128, mid)
    P.or_("sj_t", "sj_h", "sj_l")
    P.set("sj_h", 0)
    P.set("sj_l", 0)
    P.br("eq", "sj_t", 0, done)
    P.set("sj_l", 1)
    P.ret()
    P.label(mid)
    P.br("ltu", "sj_n", 64, lo)
    P.sub("sj_m", "sj_n", 64)
    P.shl("sj_k", 1, "sj_m")
    P.sub("sj_k", "sj_k", 1)
    P.and_("sj_s", "sj_h", "sj_k")
    P.or_("sj_s", "sj_s", "sj_l")
    P.shr("sj_l", "sj_h", "sj_m")
    P.set("sj_h", 0)
    P.br("eq", "sj_s", 0, done)
    P.or_("sj_l", "sj_l", 1)
    P.ret()
    P.label(lo)
    P.shl("sj_k", 1, "sj_n")
    P.sub("sj_k", "sj_k", 1)
    P.and_("sj_s", "sj_l", "sj_k")
    P.shr("sj_l", "sj_l", "sj_n")
    P.sub("sj_c", 64, "sj_n")
    P.shl("sj_t", "sj_h", "sj_c")
    P.or_("sj_l", "sj_l", "sj_t")
    P.shr("sj_h", "sj_h", "sj_n")
    P.br("eq", "sj_s", 0, done)
    P.or_("sj_l", "sj_l", 1)
    P.label(done)
    P.ret()

    # ------------------------------------------------------------ sl: (sl_h:sl_l) <<= sl_n (>= 128 gives 0)
    P.label("sf_shl")
    done, big, hi = L(), L(), L()
    P.br("eq", "sl_n", 0, done)
    P.br("geu", "sl_n", 128, big)
    P.br("geu", "sl_n", 64, hi)
    P.shl("sl_h", "sl_h", "sl_n")
    P.sub("sl_c", 64, "sl_n")
    P.shr("sl_t", "sl_l", "sl_c")
    P.or_("sl_h", "sl_h", "sl_t")
    P.shl("sl_l", "sl_l", "sl_n")
    P.ret()
    P.label(hi)
    P.sub("sl_m", "sl_n", 64)
    P.shl("sl_h", "sl_l", "sl_m")
    P.set("sl_l", 0)
    P.ret()
    P.label(big)
    P.set("sl_h", 0)
    P.set("sl_l", 0)
    P.label(done)
    P.ret()

    # ------------------------------------------------------------ round_shift(rs_h:rs_l >> rs_s, rs_neg, rs_rm) -> rs_q rs_inex
    P.label("sf_rshift")
    pos, huge, e128, low, e64, dec = L(), L(), L(), L(), L(), L()
    P.br("gts", "rs_s", 0, pos)
    P.sub("rs_n", 0, "rs_s")                    # shift <= 0: the mantissa moves up; it is exact
    P.shl("rs_q", "rs_l", "rs_n")
    P.set("rs_inex", 0)
    P.ret()
    P.label(pos)
    P.br("gts", "rs_s", 128, huge)
    P.br("eq", "rs_s", 128, e128)
    P.br("ltu", "rs_s", 64, low)
    P.br("eq", "rs_s", 64, e64)
    # 65..127: the quotient is h >> (s - 64); the remainder is (h & mask : l), half = 1 << (s - 1) in the high word
    P.sub("rs_m", "rs_s", 64)
    P.shr("rs_q", "rs_h", "rs_m")
    P.shl("rs_k", 1, "rs_m")
    P.sub("rs_k", "rs_k", 1)
    P.and_("rs_rh", "rs_h", "rs_k")
    P.or_("rs_inex", "rs_rh", "rs_l")
    P.sub("rs_t", "rs_m", 1)
    P.shl("rs_hh", 1, "rs_t")
    gt, lt, c0 = L(), L(), L()
    P.br("gtu", "rs_rh", "rs_hh", gt)
    P.br("ltu", "rs_rh", "rs_hh", lt)
    P.br("ne", "rs_l", 0, gt)
    P.set("rs_c", 0)
    P.jmp(c0)
    P.label(gt)
    P.set("rs_c", 1)
    P.jmp(c0)
    P.label(lt)
    P.set("rs_c", -1)
    P.label(c0)
    P.br("eq", "rs_inex", 0, dec)
    P.set("rs_inex", 1)
    P.jmp(dec)
    P.label(e64)                                # s == 64: q = h, remainder l, half 1 << 63
    P.mov("rs_q", "rs_h")
    P.set("rs_hh", 1 << 63)
    P.mov("rs_rem", "rs_l")
    P.jmp(L() if False else "sf_rs_cmp64")
    P.label(low)                                # 1..63
    P.shr("rs_q", "rs_l", "rs_s")
    P.sub("rs_c64", 64, "rs_s")
    P.shl("rs_t", "rs_h", "rs_c64")
    P.or_("rs_q", "rs_q", "rs_t")
    P.shl("rs_k", 1, "rs_s")
    P.sub("rs_k", "rs_k", 1)
    P.and_("rs_rem", "rs_l", "rs_k")
    P.sub("rs_t", "rs_s", 1)
    P.shl("rs_hh", 1, "rs_t")
    P.label("sf_rs_cmp64")                      # compare rs_rem with rs_hh (both 64-bit), inexact = rem != 0
    gt, lt, c0 = L(), L(), L()
    P.br("gtu", "rs_rem", "rs_hh", gt)
    P.br("ltu", "rs_rem", "rs_hh", lt)
    P.set("rs_c", 0)
    P.jmp(c0)
    P.label(gt)
    P.set("rs_c", 1)
    P.jmp(c0)
    P.label(lt)
    P.set("rs_c", -1)
    P.label(c0)
    P.set("rs_inex", 0)
    P.br("eq", "rs_rem", 0, dec)
    P.set("rs_inex", 1)
    P.jmp(dec)
    P.label(e128)                               # s == 128: q = 0, compare (h:l) with 1 << 127
    P.set("rs_q", 0)
    gt, lt, c0 = L(), L(), L()
    P.br("gtu", "rs_h", 1 << 63, gt)
    P.br("ltu", "rs_h", 1 << 63, lt)
    P.br("ne", "rs_l", 0, gt)
    P.set("rs_c", 0)
    P.jmp(c0)
    P.label(gt)
    P.set("rs_c", 1)
    P.jmp(c0)
    P.label(lt)
    P.set("rs_c", -1)
    P.label(c0)
    P.or_("rs_inex", "rs_h", "rs_l")
    nz = L()
    P.br("eq", "rs_inex", 0, dec)
    P.set("rs_inex", 1)
    P.jmp(dec)
    P.label(huge)
    P.set("rs_q", 0)
    P.set("rs_c", -1)
    P.or_("rs_inex", "rs_h", "rs_l")
    P.br("eq", "rs_inex", 0, dec)
    P.set("rs_inex", 1)
    P.label(dec)                                # up?
    up0, rne, rdn, rup, rmm, add = L(), L(), L(), L(), L(), L()
    P.set("rs_up", 0)
    P.br("eq", "rs_rm", RTZ, add)
    P.br("eq", "rs_rm", RDN, rdn)
    P.br("eq", "rs_rm", RUP, rup)
    P.br("eq", "rs_rm", RMM, rmm)
    P.br("lts", "rs_c", 0, add)                 # RNE: below half: down
    P.br("gts", "rs_c", 0, "sf_rs_up")
    P.and_("rs_t", "rs_q", 1)                   # tie: to even
    P.mov("rs_up", "rs_t")
    P.jmp(add)
    P.label("sf_rs_up")
    P.set("rs_up", 1)
    P.jmp(add)
    P.label(rdn)
    P.and_("rs_up", "rs_neg", "rs_inex")
    P.jmp(add)
    P.label(rup)
    P.xor("rs_t", "rs_neg", 1)
    P.and_("rs_up", "rs_t", "rs_inex")
    P.jmp(add)
    P.label(rmm)
    P.br("lts", "rs_c", 0, add)
    P.set("rs_up", 1)
    P.label(add)
    P.add("rs_q", "rs_q", "rs_up")
    P.ret()

    # ------------------------------------------------------------ round_pack(rp_neg, rp_h:rp_l x 2^rp_e, rp_rm) -> rp_bits
    P.label("sf_rp")
    P.mov("bl_h", "rp_h")
    P.mov("bl_l", "rp_l")
    P.call("sf_bitlen")
    P.mov("rp_n", "bl_n")
    P.add("rp_etop", "rp_e", "rp_n")
    P.sub("rp_etop", "rp_etop", 1)
    P.shl("rp_sign", "rp_neg", "f_sgnsh")
    P.mov("rs_h", "rp_h")
    P.mov("rs_l", "rp_l")
    P.mov("rs_neg", "rp_neg")
    P.mov("rs_rm", "rp_rm")
    P.sub("rs_s", "rp_n", "f_p")
    P.call("sf_rshift")
    P.mov("rp_q", "rs_q")
    P.mov("rp_inx", "rs_inex")
    P.shr("rp_t", "rp_q", "f_p")
    P.mov("rp_erd", "rp_etop")
    nc, noovf, sub, q1, big1, bigend = L(), L(), L(), L(), L(), L()
    P.br("eq", "rp_t", 0, nc)
    P.add("rp_erd", "rp_erd", 1)
    P.label(nc)
    P.br("les", "rp_erd", "f_emax", noovf)
    P.set("rp_big", 1)                          # overflow: infinity or the largest finite, by the mode
    P.br("eq", "rp_rm", RTZ, big1)
    P.br("eq", "rp_rm", RDN, "sf_rp_dn")
    P.br("eq", "rp_rm", RUP, "sf_rp_up")
    P.jmp(bigend)
    P.label(big1)
    P.set("rp_big", 0)
    P.jmp(bigend)
    P.label("sf_rp_dn")
    P.mov("rp_big", "rp_neg")
    P.jmp(bigend)
    P.label("sf_rp_up")
    P.xor("rp_big", "rp_neg", 1)
    P.label(bigend)
    P.or_("rp_bits", "rp_sign", "f_inf")
    P.br("ne", "rp_big", 0, "sf_rp_ovf")
    P.sub("rp_bits", "rp_bits", 1)
    P.label("sf_rp_ovf")
    P.or_("fl", "fl", OF | NX)
    P.ret()
    P.label(noovf)
    P.br("lts", "rp_etop", "f_emin", sub)
    P.mov("rp_e2", "rp_etop")                   # a normal result
    P.br("eq", "rp_t", 0, q1)
    P.shr("rp_q", "rp_q", 1)
    P.mov("rp_e2", "rp_erd")
    P.label(q1)
    P.add("rp_f", "rp_e2", "f_bias")
    P.shl("rp_bits", "rp_f", "f_fb")
    P.and_("rp_t2", "rp_q", "f_fracmask")
    P.or_("rp_bits", "rp_bits", "rp_t2")
    P.or_("rp_bits", "rp_bits", "rp_sign")
    P.br("eq", "rp_inx", 0, "sf_rp_ret")
    P.or_("fl", "fl", NX)
    P.label("sf_rp_ret")
    P.ret()
    P.label(sub)                                # below the normal range: round at the subnormal unit
    P.set("rp_tiny", 0)
    P.br("ges", "rp_erd", "f_emin", "sf_rp_nt")
    P.set("rp_tiny", 1)
    P.label("sf_rp_nt")
    P.sub("rs_s", "f_emin", "f_fb")
    P.sub("rs_s", "rs_s", "rp_e")
    P.mov("rs_h", "rp_h")
    P.mov("rs_l", "rp_l")
    P.mov("rs_neg", "rp_neg")
    P.mov("rs_rm", "rp_rm")
    P.call("sf_rshift")
    P.or_("rp_bits", "rs_q", "rp_sign")
    P.br("eq", "rs_inex", 0, "sf_rp_ret2")
    P.or_("fl", "fl", NX)
    P.br("eq", "rp_tiny", 0, "sf_rp_ret2")
    P.or_("fl", "fl", UF)
    P.label("sf_rp_ret2")
    P.ret()

    # ------------------------------------------------------------ the NaN answer: canonical, NV if x or y signals
    P.label("sf_nanres")                        # in: nr_snan (0/1)
    P.mov("sf_res", "f_nan")
    P.br("eq", "nr_snan", 0, "sf_nr_ret")
    P.or_("fl", "fl", NV)
    P.label("sf_nr_ret")
    P.ret()

    P.label("sf_zero")                          # sf_res = +-0 by z_neg
    P.shl("sf_res", "z_neg", "f_sgnsh")
    P.ret()
    P.label("sf_inf")                           # sf_res = +-inf by z_neg
    P.shl("sf_res", "z_neg", "f_sgnsh")
    P.or_("sf_res", "sf_res", "f_inf")
    P.ret()

    # ------------------------------------------------------------ add_vals(x, y, rm) -> sf_res   (x_*, y_*; rp_rm = rm)
    P.label("sf_addvals")
    nonan, noinf, nozero, xz, yz = L(), L(), L(), L(), L()
    P.br("ltu", "x_cls", QNAN, "sf_av1")
    P.set("nr_snan", 0)
    P.br("ne", "x_cls", SNAN, "sf_av0")
    P.set("nr_snan", 1)
    P.label("sf_av0")
    P.br("ne", "y_cls", SNAN, "sf_av0b")
    P.set("nr_snan", 1)
    P.label("sf_av0b")
    P.jmp("sf_nanres")
    P.label("sf_av1")
    P.br("ltu", "y_cls", QNAN, "sf_av2")
    P.set("nr_snan", 0)
    P.br("ne", "y_cls", SNAN, "sf_av1b")
    P.set("nr_snan", 1)
    P.label("sf_av1b")
    P.jmp("sf_nanres")
    P.label("sf_av2")
    # infinities
    P.br("eq", "x_cls", INF, "sf_av_xi")
    P.br("eq", "y_cls", INF, "sf_av_yi")
    P.jmp("sf_av_fin")
    P.label("sf_av_xi")
    P.br("ne", "y_cls", INF, "sf_av_xi1")
    P.br("ne", "x_neg", "y_neg", "sf_av_inv")
    P.label("sf_av_xi1")
    P.mov("z_neg", "x_neg")
    P.jmp("sf_inf")
    P.label("sf_av_inv")                        # inf - inf
    P.mov("sf_res", "f_nan")
    P.or_("fl", "fl", NV)
    P.ret()
    P.label("sf_av_yi")
    P.mov("z_neg", "y_neg")
    P.jmp("sf_inf")
    P.label("sf_av_fin")
    P.br("ne", "x_cls", ZERO, "sf_av_x1")
    P.br("ne", "y_cls", ZERO, "sf_av_y1")
    # zero + zero
    P.br("ne", "x_neg", "y_neg", "sf_av_zz")
    P.mov("z_neg", "x_neg")
    P.jmp("sf_zero")
    P.label("sf_av_zz")                         # a cancelling pair: +0, or -0 when rounding down
    P.set("z_neg", 0)
    P.br("ne", "rp_rm", RDN, "sf_av_zz1")
    P.set("z_neg", 1)
    P.label("sf_av_zz1")
    P.jmp("sf_zero")
    P.label("sf_av_y1")                         # zero + y: y rounded
    P.mov("rp_neg", "y_neg")
    P.mov("rp_h", "y_h")
    P.mov("rp_l", "y_l")
    P.mov("rp_e", "y_e")
    P.call("sf_rp")
    P.mov("sf_res", "rp_bits")
    P.ret()
    P.label("sf_av_x1")
    P.br("ne", "y_cls", ZERO, "sf_av_both")
    P.mov("rp_neg", "x_neg")                    # x + zero: x rounded
    P.mov("rp_h", "x_h")
    P.mov("rp_l", "x_l")
    P.mov("rp_e", "x_e")
    P.call("sf_rp")
    P.mov("sf_res", "rp_bits")
    P.ret()
    # --- both finite non-zero: add_exact
    P.label("sf_av_both")
    P.mov("bl_h", "x_h")
    P.mov("bl_l", "x_l")
    P.call("sf_bitlen")
    P.add("ae_t1", "x_e", "bl_n")               # top bit of x
    P.mov("bl_h", "y_h")
    P.mov("bl_l", "y_l")
    P.call("sf_bitlen")
    P.add("ae_t2", "y_e", "bl_n")
    P.mov("ae_top", "ae_t1")
    P.br("ges", "ae_t1", "ae_t2", "sf_ae_t")
    P.mov("ae_top", "ae_t2")
    P.label("sf_ae_t")
    P.sub("ae_base", "ae_top", 126)
    for X in "xy":                              # place: left when e >= base, else right with a sticky
        P.mov("sl_h", X + "_h")
        P.mov("sj_h", X + "_h")
        P.mov("sl_l", X + "_l")
        P.mov("sj_l", X + "_l")
        r, e = L(), L()
        P.br("lts", X + "_e", "ae_base", r)
        P.sub("sl_n", X + "_e", "ae_base")
        P.call("sf_shl")
        P.mov(X + "_ph", "sl_h")
        P.mov(X + "_pl", "sl_l")
        P.jmp(e)
        P.label(r)
        P.sub("sj_n", "ae_base", X + "_e")
        P.call("sf_shrj")
        P.mov(X + "_ph", "sj_h")
        P.mov(X + "_pl", "sj_l")
        P.label(e)
    diff = L()
    P.br("ne", "x_neg", "y_neg", diff)
    P.add("rp_l", "x_pl", "y_pl")               # same signs: the sum
    P.set("ae_c", 0)
    P.br("geu", "rp_l", "x_pl", "sf_ae_nc")
    P.set("ae_c", 1)
    P.label("sf_ae_nc")
    P.add("rp_h", "x_ph", "y_ph")
    P.add("rp_h", "rp_h", "ae_c")
    P.mov("rp_neg", "x_neg")
    P.jmp("sf_ae_done")
    P.label(diff)                               # the larger magnitude minus the smaller
    xbig, ybig = L(), L()
    P.br("gtu", "x_ph", "y_ph", xbig)
    P.br("ltu", "x_ph", "y_ph", ybig)
    P.br("gtu", "x_pl", "y_pl", xbig)
    P.br("ltu", "x_pl", "y_pl", ybig)
    P.set("z_neg", 0)                           # exactly zero
    P.br("ne", "rp_rm", RDN, "sf_ae_z1")
    P.set("z_neg", 1)
    P.label("sf_ae_z1")
    P.jmp("sf_zero")
    for lab_, A_, B_, ng in ((xbig, "x", "y", "x_neg"), (ybig, "y", "x", "y_neg")):
        P.label(lab_)
        P.sub("rp_l", A_ + "_pl", B_ + "_pl")
        P.set("ae_c", 0)
        P.br("geu", A_ + "_pl", B_ + "_pl", lab_ + "n")
        P.set("ae_c", 1)
        P.label(lab_ + "n")
        P.sub("rp_h", A_ + "_ph", B_ + "_ph")
        P.sub("rp_h", "rp_h", "ae_c")
        P.mov("rp_neg", ng)
        P.jmp("sf_ae_done")
    P.label("sf_ae_done")
    P.mov("rp_e", "ae_base")
    P.call("sf_rp")
    P.mov("sf_res", "rp_bits")
    P.ret()

    # ------------------------------------------------------------ product(a, b) -> p_cls p_neg p_h p_l p_e   (non-NaN)
    P.label("sf_product")
    P.xor("p_neg", "a_neg", "b_neg")
    P.br("ne", "a_cls", INF, "sf_pr1")
    P.br("ne", "b_cls", ZERO, "sf_pr_inf")
    P.set("p_cls", INVALID)
    P.ret()
    P.label("sf_pr1")
    P.br("ne", "a_cls", ZERO, "sf_pr2")
    P.br("ne", "b_cls", INF, "sf_pr_zero")
    P.set("p_cls", INVALID)
    P.ret()
    P.label("sf_pr2")
    P.br("eq", "b_cls", INF, "sf_pr_inf")
    P.br("eq", "b_cls", ZERO, "sf_pr_zero")
    P.set("p_cls", FIN)
    P.mul("p_l", "a_l", "b_l")
    P.umulh("p_h", "a_l", "b_l")
    P.add("p_e", "a_e", "b_e")
    P.ret()
    P.label("sf_pr_inf")
    P.set("p_cls", INF)
    P.ret()
    P.label("sf_pr_zero")
    P.set("p_cls", ZERO)
    P.ret()

    def unpack_into(X, src):
        P.mov("u_x", src)
        P.call("sf_unpack")
        copy_u(X)

    def set_snan(*Xs):
        P.set("nr_snan", 0)
        for X in Xs:
            l = L()
            P.br("ne", X + "_cls", SNAN, l)
            P.set("nr_snan", 1)
            P.label(l)

    # ------------------------------------------------------------ the entries: inputs sf_a sf_b sf_c, rp_rm; outputs sf_res, fl
    P.label("sf_e_sub")
    unpack_into("x", "sf_a")
    unpack_into("y", "sf_b")
    P.xor("y_neg", "y_neg", 1)
    P.jmp("sf_e_add2")
    P.label("sf_e_add")
    unpack_into("x", "sf_a")
    unpack_into("y", "sf_b")
    P.label("sf_e_add2")
    P.jmp("sf_addvals")

    P.label("sf_e_mul")
    unpack_into("a", "sf_a")
    unpack_into("b", "sf_b")
    P.br("ltu", "a_cls", QNAN, "sf_m1")
    set_snan("a", "b")
    P.jmp("sf_nanres")
    P.label("sf_m1")
    P.br("ltu", "b_cls", QNAN, "sf_m2")
    set_snan("a", "b")
    P.jmp("sf_nanres")
    P.label("sf_m2")
    P.call("sf_product")
    P.br("ne", "p_cls", INVALID, "sf_m3")
    P.mov("sf_res", "f_nan")
    P.or_("fl", "fl", NV)
    P.ret()
    P.label("sf_m3")
    P.mov("z_neg", "p_neg")
    P.br("eq", "p_cls", INF, "sf_inf")
    P.br("eq", "p_cls", ZERO, "sf_zero")
    P.mov("rp_neg", "p_neg")
    P.mov("rp_h", "p_h")
    P.mov("rp_l", "p_l")
    P.mov("rp_e", "p_e")
    P.call("sf_rp")
    P.mov("sf_res", "rp_bits")
    P.ret()

    # fma: sf_a * sf_b +- sf_c; ng_p negates the product, ng_a the addend
    P.label("sf_e_fma")
    unpack_into("a", "sf_a")
    unpack_into("b", "sf_b")
    unpack_into("c", "sf_c")
    # inf_zero: (a inf, b zero) or (a zero, b inf)
    P.set("fm_iz", 0)
    P.br("ne", "a_cls", INF, "sf_f1")
    P.br("ne", "b_cls", ZERO, "sf_f2")
    P.set("fm_iz", 1)
    P.jmp("sf_f2")
    P.label("sf_f1")
    P.br("ne", "a_cls", ZERO, "sf_f2")
    P.br("ne", "b_cls", INF, "sf_f2")
    P.set("fm_iz", 1)
    P.label("sf_f2")
    P.br("ltu", "a_cls", QNAN, "sf_f3")
    P.jmp("sf_f_nan")
    P.label("sf_f3")
    P.br("ltu", "b_cls", QNAN, "sf_f4")
    P.jmp("sf_f_nan")
    P.label("sf_f4")
    P.br("ltu", "c_cls", QNAN, "sf_f5")
    P.label("sf_f_nan")
    set_snan("a", "b", "c")
    P.or_("nr_snan", "nr_snan", "fm_iz")
    P.jmp("sf_nanres")
    P.label("sf_f5")
    P.xor("c_neg", "c_neg", "ng_a")
    P.call("sf_product")
    P.br("ne", "p_cls", INVALID, "sf_f6")
    P.mov("sf_res", "f_nan")
    P.or_("fl", "fl", NV)
    P.ret()
    P.label("sf_f6")
    P.xor("p_neg", "p_neg", "ng_p")
    copy("x", "p")
    copy("y", "c")
    P.jmp("sf_addvals")

    # div: sf_a / sf_b
    P.label("sf_e_div")
    unpack_into("a", "sf_a")
    unpack_into("b", "sf_b")
    P.br("ltu", "a_cls", QNAN, "sf_d1")
    set_snan("a", "b")
    P.jmp("sf_nanres")
    P.label("sf_d1")
    P.br("ltu", "b_cls", QNAN, "sf_d2")
    set_snan("a", "b")
    P.jmp("sf_nanres")
    P.label("sf_d2")
    P.xor("z_neg", "a_neg", "b_neg")
    P.br("ne", "a_cls", INF, "sf_d3")
    P.br("eq", "b_cls", INF, "sf_d_inv")
    P.jmp("sf_inf")                             # inf / x: infinity
    P.label("sf_d3")
    P.br("ne", "a_cls", ZERO, "sf_d4")
    P.br("eq", "b_cls", ZERO, "sf_d_inv")       # 0 / 0
    P.jmp("sf_zero")                            # 0 / x
    P.label("sf_d4")                            # a is finite non-zero
    P.br("ne", "b_cls", INF, "sf_d5")
    P.jmp("sf_zero")
    P.label("sf_d5")
    P.br("ne", "b_cls", ZERO, "sf_d6")
    P.or_("fl", "fl", DZ)
    P.jmp("sf_inf")
    P.label("sf_d_inv")
    P.mov("sf_res", "f_nan")
    P.or_("fl", "fl", NV)
    P.ret()
    P.label("sf_d6")
    # num = a_l << s, s = 127 - bitlen(a_l); q = num / b_l, r = num % b_l; mant = (q << 1) | (r != 0)
    P.set("bl_h", 0)
    P.mov("bl_l", "a_l")
    P.call("sf_bitlen")
    P.sub("dv_s", 127, "bl_n")
    P.set("sl_h", 0)
    P.mov("sl_l", "a_l")
    P.mov("sl_n", "dv_s")
    P.call("sf_shl")
    P.mov("dv_nh", "sl_h")
    P.mov("dv_nl", "sl_l")
    P.set("dv_r", 0)
    for word, q in (("dv_nh", "dv_qh"), ("dv_nl", "dv_ql")):
        P.set(q, 0)
        P.set("dv_i", 64)
        lp, no, nx = L(), L(), L()
        P.label(lp)
        P.shr("dv_bit", word, 63)
        P.shl(word, word, 1)
        P.shl("dv_r", "dv_r", 1)
        P.or_("dv_r", "dv_r", "dv_bit")
        P.shl(q, q, 1)
        P.br("ltu", "dv_r", "b_l", no)
        P.sub("dv_r", "dv_r", "b_l")
        P.or_(q, q, 1)
        P.label(no)
        P.sub("dv_i", "dv_i", 1)
        P.br("ne", "dv_i", 0, lp)
    # mant = (q << 1) | sticky
    P.shr("t_c", "dv_ql", 63)
    P.shl("rp_h", "dv_qh", 1)
    P.or_("rp_h", "rp_h", "t_c")
    P.shl("rp_l", "dv_ql", 1)
    P.br("eq", "dv_r", 0, "sf_d7")
    P.or_("rp_l", "rp_l", 1)
    P.label("sf_d7")
    P.sub("rp_e", "a_e", "dv_s")
    P.sub("rp_e", "rp_e", "b_e")
    P.sub("rp_e", "rp_e", 1)
    P.mov("rp_neg", "z_neg")
    P.call("sf_rp")
    P.mov("sf_res", "rp_bits")
    P.ret()

    # sqrt: sf_a
    P.label("sf_e_sqrt")
    unpack_into("a", "sf_a")
    P.br("ltu", "a_cls", QNAN, "sf_s1")
    set_snan("a")
    P.jmp("sf_nanres")
    P.label("sf_s1")
    P.br("ne", "a_cls", ZERO, "sf_s2")
    P.mov("z_neg", "a_neg")
    P.jmp("sf_zero")
    P.label("sf_s2")
    P.br("eq", "a_neg", 0, "sf_s3")
    P.mov("sf_res", "f_nan")                    # sqrt of a negative (or -inf): invalid
    P.or_("fl", "fl", NV)
    P.ret()
    P.label("sf_s3")
    P.br("ne", "a_cls", INF, "sf_s4")
    P.set("z_neg", 0)
    P.jmp("sf_inf")
    P.label("sf_s4")
    P.mov("sq_m", "a_l")
    P.mov("sq_e", "a_e")
    P.and_("sq_t", "sq_e", 1)
    P.br("eq", "sq_t", 0, "sf_s5")
    P.shl("sq_m", "sq_m", 1)
    P.sub("sq_e", "sq_e", 1)
    P.label("sf_s5")
    P.set("bl_h", 0)
    P.mov("bl_l", "sq_m")
    P.call("sf_bitlen")
    P.sub("sq_s", 126, "bl_n")
    P.and_("sq_t", "sq_s", 1)
    P.sub("sq_s", "sq_s", "sq_t")
    P.set("sl_h", 0)
    P.mov("sl_l", "sq_m")
    P.mov("sl_n", "sq_s")
    P.call("sf_shl")
    P.mov("sq_mh", "sl_h")
    P.mov("sq_ml", "sl_l")
    P.set("sq_r", 0)
    P.set("sq_b", 63)
    lp, nx = L(), L()
    P.label(lp)
    P.shl("sq_c", 1, "sq_b")
    P.or_("sq_c", "sq_c", "sq_r")                # candidate = r | 1 << b
    P.mul("sq_pl", "sq_c", "sq_c")
    P.umulh("sq_ph", "sq_c", "sq_c")
    P.br("gtu", "sq_ph", "sq_mh", nx)
    P.br("ltu", "sq_ph", "sq_mh", "sf_sq_take")
    P.br("gtu", "sq_pl", "sq_ml", nx)
    P.label("sf_sq_take")
    P.mov("sq_r", "sq_c")
    P.label(nx)
    P.sub("sq_b", "sq_b", 1)
    P.br("gts", "sq_b", -1, lp)
    # root = (r << 1) | (r*r != m)
    P.mul("sq_pl", "sq_r", "sq_r")
    P.umulh("sq_ph", "sq_r", "sq_r")
    P.shl("rp_l", "sq_r", 1)
    P.shr("rp_h", "sq_r", 63)
    P.br("ne", "sq_ph", "sq_mh", "sf_sq_st")
    P.br("eq", "sq_pl", "sq_ml", "sf_sq_ex")
    P.label("sf_sq_st")
    P.or_("rp_l", "rp_l", 1)
    P.label("sf_sq_ex")
    P.sub("sq_x", "sq_e", "sq_s")
    P.sar("rp_e", "sq_x", 1)
    P.sub("rp_e", "rp_e", 1)
    P.set("rp_neg", 0)
    P.call("sf_rp")
    P.mov("sf_res", "rp_bits")
    P.ret()

    # fcvt.s.d: sf_a (a double) rounded into a single.  The caller sets the format to S for the result.
    P.label("sf_e_cvt_ds")
    P.call("sf_fmt_d")
    unpack_into("a", "sf_a")
    P.call("sf_fmt_s")
    P.br("ltu", "a_cls", QNAN, "sf_c1")
    set_snan("a")
    P.jmp("sf_nanres")
    P.label("sf_c1")
    P.mov("z_neg", "a_neg")
    P.br("eq", "a_cls", INF, "sf_inf")
    P.br("eq", "a_cls", ZERO, "sf_zero")
    P.mov("rp_neg", "a_neg")
    P.mov("rp_h", "a_h")
    P.mov("rp_l", "a_l")
    P.mov("rp_e", "a_e")
    P.call("sf_rp")
    P.mov("sf_res", "rp_bits")
    P.ret()

    # integer -> float: fi_neg, fi_mag (a 64-bit magnitude)
    P.label("sf_e_fromint")
    P.br("ne", "fi_mag", 0, "sf_i1")
    P.set("sf_res", 0)
    P.ret()
    P.label("sf_i1")
    P.mov("rp_neg", "fi_neg")
    P.set("rp_h", 0)
    P.mov("rp_l", "fi_mag")
    P.set("rp_e", 0)
    P.call("sf_rp")
    P.mov("sf_res", "rp_bits")
    P.ret()


# ---------------------------------------------------------------- the interpreter backend
class Interp:
    """runs the engine in Python: the algorithm's own test bench"""

    def __init__(self):
        self.prog = []
        self.labels = {}
        self.n = itertools.count()
        self.v = {}

    def lab(self):
        return "L%d" % next(self.n)

    def label(self, n):
        assert n not in self.labels, n
        self.labels[n] = len(self.prog)

    def _o(self, x):
        return x & M64 if isinstance(x, int) else self.v.get(x, 0)

    def emit(self, *t):
        self.prog.append(t)

    def jmp(self, n):
        self.emit("jmp", n)

    def br(self, c, a, b, n):
        self.emit("br", c, a, b, n)

    def call(self, n):
        self.emit("call", n)

    def ret(self):
        self.emit("ret")

    def set(self, d, imm):
        self.emit("set", d, imm)

    def mov(self, d, a):
        self.emit("mov", d, a)

    def clz(self, d, a):
        self.emit("clz", d, a)

    def __getattr__(self, name):
        if name.rstrip("_") in ("add", "sub", "mul", "umulh", "and", "or", "xor", "shl", "shr", "sar"):
            op = name.rstrip("_")
            return lambda d, a, b: self.emit(op, d, a, b)
        raise AttributeError(name)

    def run(self, entry, **inputs):
        self.v = {k: val & M64 for k, val in inputs.items()}
        pc, stack = self.labels[entry], []
        o = self._o
        while True:
            t = self.prog[pc]
            pc += 1
            k = t[0]
            if k == "set":
                self.v[t[1]] = t[2] & M64
            elif k == "mov":
                self.v[t[1]] = o(t[2])
            elif k == "jmp":
                pc = self.labels[t[1]]
            elif k == "call":
                stack.append(pc)
                pc = self.labels[t[1]]
            elif k == "ret":
                if not stack:
                    return self.v
                pc = stack.pop()
            elif k == "clz":
                a = o(t[2])
                self.v[t[1]] = 64 - a.bit_length()
            elif k == "br":
                c, a, b = t[1], o(t[2]), o(t[3])
                sa, sb = a - (1 << 64) if a >> 63 else a, b - (1 << 64) if b >> 63 else b
                if {"eq": a == b, "ne": a != b, "ltu": a < b, "leu": a <= b, "gtu": a > b, "geu": a >= b,
                    "lts": sa < sb, "les": sa <= sb, "gts": sa > sb, "ges": sa >= sb}[c]:
                    pc = self.labels[t[4]]
            else:
                a, b = o(t[2]), o(t[3])
                if k == "add":
                    r = a + b
                elif k == "sub":
                    r = a - b
                elif k == "mul":
                    r = a * b
                elif k == "umulh":
                    r = (a * b) >> 64
                elif k == "and":
                    r = a & b
                elif k == "or":
                    r = a | b
                elif k == "xor":
                    r = a ^ b
                elif k == "shl":
                    r = a << b if b < 64 else 0
                elif k == "shr":
                    r = a >> b if b < 64 else 0
                else:                                   # sar
                    sa = a - (1 << 64) if a >> 63 else a
                    r = sa >> min(b, 63)
                self.v[t[1]] = r & M64


# ---------------------------------------------------------------- the self test: an exact rational reference
def selftest(n=20000, seed=1):
    import random
    from fractions import Fraction
    import struct
    it = Interp()
    engine(it)
    FMT = {"d": dict(p=53, w=64, emin=-1022, emax=1023), "s": dict(p=24, w=32, emin=-126, emax=127)}

    def bits_val(f, b):
        p, w = f["p"], f["w"]
        s = b >> (w - 1) & 1
        e = b >> (p - 1) & ((1 << (w - p)) - 1)
        fr = b & ((1 << (p - 1)) - 1)
        emax_f = (1 << (w - p)) - 1
        if e == emax_f:
            return ("inf", s) if fr == 0 else ("nan", fr >> (p - 2) & 1 == 0)
        if e == 0:
            return ("fin", s, Fraction(fr) * Fraction(2) ** (f["emin"] - (p - 1)))
        return ("fin", s, Fraction(fr | 1 << (p - 1)) * Fraction(2) ** (e - f["emax"] - (p - 1)))

    def ref_round(f, x, rm):
        """x: Fraction (exact result, may be 0 -> handled by caller) -> (bits, flags)"""
        p, w, emin, emax = f["p"], f["w"], f["emin"], f["emax"]
        neg = x < 0
        ax = abs(x)
        sign = int(neg) << (w - 1)
        # exponent of the leading bit
        e = ax.numerator.bit_length() - ax.denominator.bit_length()
        if Fraction(2) ** e > ax:
            e -= 1
        # unit of the last place
        ulp_e = max(e, emin) - (p - 1)
        unit = Fraction(2) ** ulp_e
        q, r = divmod(ax, unit)
        q = int(q)
        inexact = r != 0
        half = unit / 2
        up = {RTZ: False, RDN: neg and inexact, RUP: (not neg) and inexact,
              RMM: r >= half, RNE: r > half or (r == half and q & 1)}[rm]
        q2 = q + up
        # tininess after rounding: the result, rounded to p bits with an UNBOUNDED exponent, is below the smallest normal
        tiny = False
        if e < emin:
            # unbounded-exponent rounding of ax to p bits
            ue = e - (p - 1)
            uq, ur = divmod(ax, Fraction(2) ** ue)
            uq = int(uq)
            uhalf = Fraction(2) ** ue / 2
            uup = {RTZ: False, RDN: neg and ur != 0, RUP: (not neg) and ur != 0,
                   RMM: ur >= uhalf, RNE: ur > uhalf or (ur == uhalf and uq & 1)}[rm]
            tiny = (uq + uup) * Fraction(2) ** ue < Fraction(2) ** emin
        val_q = q2 * unit
        flags = NX if inexact else 0
        if tiny and inexact:
            flags |= UF
        if val_q >= Fraction(2) ** (emax + 1):
            big = {RTZ: False, RDN: neg, RUP: not neg}.get(rm, True)
            inf = sign | (((1 << (w - p)) - 1) << (p - 1))
            return (inf if big else inf - 1), OF | NX
        # encode val_q
        if val_q == 0:
            return sign, flags
        if val_q < Fraction(2) ** emin:
            return sign | int(val_q / Fraction(2) ** (emin - (p - 1))), flags
        ee = val_q.numerator.bit_length() - val_q.denominator.bit_length()
        if Fraction(2) ** ee > val_q:
            ee -= 1
        m = int(val_q / Fraction(2) ** (ee - (p - 1)))
        return sign | (ee + emax) << (p - 1) | (m & ((1 << (p - 1)) - 1)), flags

    def run(op, f, rm, a=0, b=0, c=0, ngp=0, nga=0):
        it.v = {}
        fmt = "sf_fmt_d" if f is FMT["d"] else "sf_fmt_s"
        it.run(fmt)                                  # sets f_*; run() resets the dict, so re-run below with inputs
        v = dict(it.v)
        v.update(sf_a=a, sf_b=b, sf_c=c, rp_rm=rm, fl=0, ng_p=ngp, ng_a=nga)
        r = it.run(op, **v)
        return r["sf_res"], r["fl"]

    def ref(op, f, rm, a, b=None, c=None, ngp=0, nga=0):
        A, B, C = bits_val(f, a), (bits_val(f, b) if b is not None else None), (bits_val(f, c) if c is not None else None)
        canon = ((((1 << (f["w"] - f["p"])) - 1) << (f["p"] - 1)) | 1 << (f["p"] - 2))
        sgn = lambda ng: ng << (f["w"] - 1)
        infb = (((1 << (f["w"] - f["p"])) - 1) << (f["p"] - 1))
        vals = [A, B, C]
        if op == "add":
            return ref_addsub(f, rm, A, B, 0)
        if op == "sub":
            return ref_addsub(f, rm, A, B, 1)
        if op == "mul":
            if A[0] == "nan" or B[0] == "nan":
                return canon, NV if (A[0] == "nan" and A[1]) or (B[0] == "nan" and B[1]) else 0
            za = A[0] == "fin" and A[2] == 0
            zb = B[0] == "fin" and B[2] == 0
            if (A[0] == "inf" and zb) or (B[0] == "inf" and za):
                return canon, NV
            sg = A[1] ^ B[1]
            if A[0] == "inf" or B[0] == "inf":
                return sgn(sg) | infb, 0
            if za or zb:
                return sgn(sg), 0
            return ref_round(f, (-1) ** sg * A[2] * B[2], rm)
        if op == "div":
            if A[0] == "nan" or B[0] == "nan":
                return canon, NV if (A[0] == "nan" and A[1]) or (B[0] == "nan" and B[1]) else 0
            za = A[0] == "fin" and A[2] == 0
            zb = B[0] == "fin" and B[2] == 0
            sg = A[1] ^ B[1]
            if (A[0] == "inf" and B[0] == "inf") or (za and zb):
                return canon, NV
            if A[0] == "inf":
                return sgn(sg) | infb, 0
            if za or B[0] == "inf":
                return sgn(sg), 0
            if zb:
                return sgn(sg) | infb, DZ
            return ref_round(f, (-1) ** sg * A[2] / B[2], rm)
        if op == "sqrt":
            if A[0] == "nan":
                return canon, NV if A[1] else 0
            if A[0] == "fin" and A[2] == 0:
                return sgn(A[1]), 0
            if A[1]:
                return canon, NV
            if A[0] == "inf":
                return infb, 0
            # exact root of a rational: integer sqrt at high precision with a sticky
            x = A[2]
            import math
            lo = x.numerator * 2 ** 2600 // x.denominator
            r = math.isqrt(lo)
            exact = r * r == lo and (x.numerator * 2 ** 2600) % x.denominator == 0
            val = Fraction(r, 2 ** 1300)
            if not exact:
                val = val + Fraction(1, 2 ** 1301)      # a sticky strictly inside (r, r + 1)
            return ref_round(f, val, rm)
        raise ValueError(op)

    def ref_addsub(f, rm, A, B, sub):
        canon = ((((1 << (f["w"] - f["p"])) - 1) << (f["p"] - 1)) | 1 << (f["p"] - 2))
        infb = (((1 << (f["w"] - f["p"])) - 1) << (f["p"] - 1))
        if A[0] == "nan" or B[0] == "nan":
            return canon, NV if (A[0] == "nan" and A[1]) or (B[0] == "nan" and B[1]) else 0
        bs = B[1] ^ sub if B[0] != "nan" else 0
        if A[0] == "inf" and B[0] == "inf":
            return ((A[1] << (f["w"] - 1)) | infb, 0) if A[1] == bs else (canon, NV)
        if A[0] == "inf":
            return (A[1] << (f["w"] - 1)) | infb, 0
        if B[0] == "inf":
            return (bs << (f["w"] - 1)) | infb, 0
        x = (-1) ** A[1] * A[2]
        y = (-1) ** bs * B[2]
        if A[2] == 0 and B[2] == 0:
            if A[1] == bs:
                return A[1] << (f["w"] - 1), 0
            return (1 << (f["w"] - 1) if rm == RDN else 0), 0
        s = x + y
        if s == 0:
            return (1 << (f["w"] - 1) if rm == RDN else 0), 0
        return ref_round(f, s, rm)

    def ref_fma(f, rm, a, b, c, ngp, nga):
        A, B, C = bits_val(f, a), bits_val(f, b), bits_val(f, c)
        canon = ((((1 << (f["w"] - f["p"])) - 1) << (f["p"] - 1)) | 1 << (f["p"] - 2))
        isz = lambda v: v[0] == "fin" and v[2] == 0
        iz = (A[0] == "inf" and isz(B)) or (isz(A) and B[0] == "inf")
        if "nan" in (A[0], B[0], C[0]):
            snan = any(v[0] == "nan" and v[1] for v in (A, B, C))
            return canon, NV if (snan or iz) else 0
        if iz:
            return canon, NV
        sg = A[1] ^ B[1] ^ ngp
        if A[0] == "inf" or B[0] == "inf":
            P_ = ("inf", sg)
        else:
            P_ = ("fin", sg, A[2] * B[2])
        return ref_addsub(f, rm, P_, C, nga)

    rnd = random.Random(seed)

    def rnd_bits(f):
        p, w = f["p"], f["w"]
        e = rnd.choice([0, 1, 2, (1 << (w - p)) - 1, (1 << (w - p)) - 2, rnd.randrange(1 << (w - p)),
                        f["emax"] + rnd.randrange(-6, 7), f["emax"] + rnd.randrange(-60, 61) if w == 64 else 127,
                        rnd.randrange(0, 70)])
        e = max(0, min(e, (1 << (w - p)) - 1))
        m = rnd.choice([0, 1, (1 << (p - 1)) - 1, 1 << (p - 2), rnd.getrandbits(p - 1), rnd.getrandbits(p - 1),
                        rnd.getrandbits(p - 1) & ~0xFF])
        return rnd.getrandbits(1) << (w - 1) | e << (p - 1) | m

    bad = 0
    for i in range(n):
        for fk, f in FMT.items():
            rm = rnd.randrange(5)
            a, b = rnd_bits(f), rnd_bits(f)
            for op in ("add", "sub", "mul", "div", "sqrt"):
                got = run("sf_e_" + op, f, rm, a, b)
                want = ref(op, f, rm, a, b)
                if got != want:
                    bad += 1
                    if bad < 12:
                        print("MISMATCH", fk, op, "rm", rm, hex(a), hex(b), "got", tuple(map(hex, got)), "want", tuple(map(hex, want)))
        for fk, f in FMT.items():
            rm = rnd.randrange(5)
            a, b, c = rnd_bits(f), rnd_bits(f), rnd_bits(f)
            ngp, nga = rnd.randrange(2), rnd.randrange(2)
            got = run("sf_e_fma", f, rm, a, b, c, ngp, nga)
            want = ref_fma(f, rm, a, b, c, ngp, nga)
            if got != want:
                bad += 1
                if bad < 12:
                    print("MISMATCH fma", fk, rm, hex(a), hex(b), hex(c), ngp, nga, "got", tuple(map(hex, got)), "want", tuple(map(hex, want)))
            # a (near-)cancelling case: c = -(a*b) rounded, nudged by a few ulps
            Af, Bf = bits_val(f, a), bits_val(f, b)
            if Af[0] == "fin" and Bf[0] == "fin" and Af[2] != 0 and Bf[2] != 0:
                cb, _ = ref_round(f, Af[2] * Bf[2], RNE)
                cb = (cb + rnd.choice([-2, -1, 0, 0, 1, 2])) & ((1 << (f["w"] - 1)) - 1)
                c2 = cb | ((Af[1] ^ Bf[1] ^ 1) << (f["w"] - 1))
                if bits_val(f, c2)[0] == "fin":
                    got = run("sf_e_fma", f, rm, a, b, c2, 0, 0)
                    want = ref_fma(f, rm, a, b, c2, 0, 0)
                    if got != want:
                        bad += 1
                        if bad < 12:
                            print("MISMATCH fma-cancel", fk, rm, hex(a), hex(b), hex(c2), "got", tuple(map(hex, got)), "want", tuple(map(hex, want)))
            # integer -> float
            v = rnd.choice([rnd.getrandbits(64), rnd.getrandbits(rnd.randrange(1, 64)), 1 << rnd.randrange(64), (1 << rnd.randrange(1, 64)) + rnd.randrange(-3, 4)])
            for neg in (0, 1):
                if neg and v >> 63:
                    continue
                mag = v & M64
                if neg:
                    mag = mag if mag else 1
                it.v = {}
                it.run("sf_fmt_d" if fk == "d" else "sf_fmt_s")
                vv = dict(it.v)
                vv.update(fi_neg=neg, fi_mag=mag, rp_rm=rm, fl=0)
                r = it.run("sf_e_fromint", **vv)
                want = ref_round(f, Fraction((-1) ** neg * mag), rm) if mag else (0, 0)
                if (r["sf_res"], r["fl"]) != want:
                    bad += 1
                    if bad < 12:
                        print("MISMATCH fromint", fk, rm, neg, hex(mag), "got", hex(r["sf_res"]), r["fl"], "want", tuple(map(hex, want)))
        # double -> single
        a = rnd_bits(FMT["d"])
        rm = rnd.randrange(5)
        it.v = {}
        it.run("sf_fmt_s")
        vv = dict(it.v)
        vv.update(sf_a=a, rp_rm=rm, fl=0)
        r = it.run("sf_e_cvt_ds", **vv)
        A = bits_val(FMT["d"], a)
        if A[0] == "nan":
            want = (0x7FC00000, NV if A[1] else 0)
        elif A[0] == "inf":
            want = (A[1] << 31 | 0x7F800000, 0)
        elif A[2] == 0:
            want = (A[1] << 31, 0)
        else:
            want = ref_round(FMT["s"], (-1) ** A[1] * A[2], rm)
        if (r["sf_res"], r["fl"]) != want:
            bad += 1
            if bad < 12:
                print("MISMATCH cvt_ds", rm, hex(a), "got", hex(r["sf_res"]), r["fl"], "want", tuple(map(hex, want)))
    print("softfp selftest: %d cases, %d mismatches" % (n * 15, bad))
    return bad


if __name__ == "__main__":
    import sys
    sys.exit(1 if selftest(int(sys.argv[1]) if len(sys.argv) > 1 else 3000) else 0)
