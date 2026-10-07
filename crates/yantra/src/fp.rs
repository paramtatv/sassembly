//! The **F and D extensions** — row `V-001`, the first unit of the scientific-compute
//! plan.
//!
//! # Why this exists, and why it is the FIRST unit
//!
//! `Sassembly Scientific & Vector ISA Extension.md` specifies the R/I/S/R4 bit layouts,
//! the `fmt` field, the five rounding modes and an `f0`–`f31` register file in exhaustive
//! detail — and never states that the target machine implements none of it. Before this
//! module, [`crate::Machine`] was **RV64IMA**: `fadd.d` halted
//! [`crate::Halt::Unimplemented`], and `lib.rs`'s own `SSTATUS_MASK` margin used that fact
//! as the safety argument for dropping the `FS`/`XS`/`VS` bits.
//!
//! So a floating-point *encoder* in `vishlesana` could not have been validated by
//! anything: an oracle cannot disagree with an instruction the machine refuses to run.
//! The compiler work is downstream of this file.
//!
//! # What is implemented, exactly
//!
//! Both widths, because the formats are parallel and doing one would have meant reading
//! the same encoding table twice:
//!
//! ```text
//!   OP-FP    0x53   fadd fsub fmul fdiv fsqrt · fsgnj{,n,x} · fmin fmax
//!                   fle flt feq · fcvt (both directions, both widths, and s<->d)
//!                   fmv.x.{w,d} fmv.{w,d}.x · fclass
//!   LOAD-FP  0x07   flw fld
//!   STORE-FP 0x27   fsw fsd
//!   MADD     0x43   fmadd        MSUB  0x47  fmsub
//!   NMSUB    0x4b   fnmsub       NMADD 0x4f  fnmadd
//! ```
//!
//! # THE ROUNDING MODE IS HONOURED OR REFUSED, NEVER IGNORED
//!
//! Rust's `f64` arithmetic is round-to-nearest-even and offers no way to ask for another
//! mode. Three responses were available: implement software rounding, ignore `rm` and
//! round to nearest anyway, or refuse. **Ignoring it is the one that had to be excluded**
//! — an encoder emitting `fadd.d rd, rs1, rs2, rtz` would get nearest-even results, agree
//! with this machine, and disagree with real hardware and with `spike`. That is a twin
//! agreeing with a stub, which this tree has already been burned by.
//!
//! So until `V-009` (i-f), **arithmetic** (`fadd`/`fsub`/`fmul`/`fdiv`/`fsqrt`/`fmadd`…)
//! honoured `RNE` and `DYN`-resolving-to-`RNE` and **halted `Unimplemented` on any other
//! mode**: a halt names the gap; a wrong low bit hides it.
//!
//! **THE GAP IS CLOSED: EVERY MODE IS HONOURED** (`V-009` (i-f), owner ruling (b): "Allow
//! rounding modes beyond round-to-nearest-even (RNE)"). The arithmetic rounds in software
//! since (i-e) — the first of the three responses above — so `RTZ`, `RDN`, `RUP` and `RMM`
//! round exactly, static or `DYN` through `frm`, in the scalar arms and in the vector
//! subset's float arms (vfadd/vfsub/vfmul/vfdiv .vv/.vf, vfredosum.vs, e64 only), and agree
//! with QEMU on every case of the sweep in every mode and both forms. Refusing is still the
//! answer for what no hart may execute: a reserved mode (5, 6, or `DYN` with `frm` 5..7)
//! is an illegal instruction.
//!
//! **Float-to-integer conversion honours all five modes**, because rounding there is
//! exact integer arithmetic rather than a hardware mode — and because `rtz` is not
//! exotic: `fcvt.w.d rd, rs1, rtz` is the ordinary way a compiler truncates a float, so
//! refusing it would refuse the common case.
//!
//! # `fflags` — EVERY FLAG, EXACTLY AS QEMU ACCRUES IT (since `V-009` part (i-e))
//!
//! `NV` (invalid) and `DZ` (divide-by-zero) were set from the start, because both are
//! decidable from the operands. **`NX`, `OF` and `UF` were not tracked for arithmetic
//! until (i-e)**: deciding them needs the rounding this module used to delegate to Rust,
//! which rounds correctly and reports nothing. The arithmetic now rounds in software —
//! [`add`], [`sub`], [`mul`], [`div`], [`sqrt`], [`fma`], [`convert`], [`from_int`], see
//! the section "EXACT IEEE 754 ROUNDING" below — and every flag agrees with QEMU 10.1.0 on
//! the whole (i-e) sweep (`tests/v009e_hardware_agreement.rs`, 37 operations x 2,104
//! inputs, 0 disagreements), with the subset's vector float arms (vfadd/vfsub/vfmul/vfdiv
//! .vv/.vf and vfredosum.vs, e64 only) — through the MACHINE in all five modes, DYN and
//! static, since (i-f), and through the core alone besides. The other vector float instructions (vfsqrt, vfmacc and its family,
//! vfcvt/vfncvt, any e32 form) are not in the subset and still halt `Unimplemented`.
//! Underflow is detected AFTER rounding, as RISC-V requires. `NX` for float-to-integer
//! conversions was always exact ("the value had a fractional part", [`cvt_to_int`]).
//!
//! **The flags are READABLE since `V-009` part (i-d)**: `fflags` (0x001), `frm` (0x002)
//! and `fcsr` (0x003) are CSRs on this machine, gated on `sstatus.FS` as on QEMU. The
//! (i-d) measurements that pinned the old gap — `1 + 2^-60` read 0 here and `NX` there,
//! `DBL_MAX * 2` 0 against `OF|NX`, `2^-1000 * 2^-100` 0 against `UF|NX` — are assertions
//! of QEMU's values now. Three more disagreements the sweep found went with them: an
//! `fma` addend that signals raised no `NV` (the flags were read off the product's
//! operands alone), `inf / 0` raised `DZ` (only a FINITE dividend does), and an
//! int-to-single conversion judged its exactness through `f64` (`fcvt.s.w` of 2^24 + 1
//! read exact, and a wide integer was rounded twice).
//!
//! **An op that only accrues flags leaves `sstatus.FS` as it was** — `feq`/`flt` on a NaN,
//! `fcvt.w.d` of 1.5 — because QEMU, this project's hardware oracle, leaves it Initial
//! (probe C6..C8); the ISA's wording (FS Dirty on any change to the float state, `fcsr`
//! included) is broader, and yantra follows QEMU.
//!
//! **A RESERVED ROUNDING MODE IS AN ILLEGAL INSTRUCTION** (also (i-d)): a static `rm` of 5
//! or 6, or `DYN` while `frm` holds 5, 6 or 7, traps cause 2 on every instruction with an
//! `rm` field, as on QEMU. `frm` became writable in the same change, which is what made
//! those states reachable.
//!
//! # NaN-boxing
//!
//! A single-precision value in a 64-bit `f` register is stored with all upper 32 bits set
//! ([`box_s`]), and a register whose upper half is not all ones does not hold a valid
//! single ([`unbox_s`] answers the canonical NaN). This is the RISC-V convention and it
//! is load-bearing: `fmv.w.x` boxes, `flw` boxes, and `fadd.s` on a register written by
//! `fld` must see a NaN rather than reinterpreting the low half of a double.

/// The canonical quiet NaN, double precision.
pub const CANONICAL_NAN_D: u64 = 0x7ff8_0000_0000_0000;
/// The canonical quiet NaN, single precision.
pub const CANONICAL_NAN_S: u32 = 0x7fc0_0000;

/// `fflags.NX` — inexact.
pub const NX: u64 = 1;
/// `fflags.UF` — underflow.
pub const UF: u64 = 2;
/// `fflags.OF` — overflow.
pub const OF: u64 = 4;
/// `fflags.DZ` — divide by zero.
pub const DZ: u64 = 8;
/// `fflags.NV` — invalid operation.
pub const NV: u64 = 16;

/// Round to nearest, ties to even.
pub const RNE: u32 = 0;
/// Round towards zero.
pub const RTZ: u32 = 1;
/// Round down, towards -infinity.
pub const RDN: u32 = 2;
/// Round up, towards +infinity.
pub const RUP: u32 = 3;
/// Round to nearest, ties to max magnitude.
pub const RMM: u32 = 4;
/// Take the mode from `fcsr.frm`.
pub const DYN: u32 = 7;

/// Wrap a single into a 64-bit `f` register, upper half all ones.
#[must_use]
pub fn box_s(v: f32) -> u64 {
    0xffff_ffff_0000_0000 | u64::from(v.to_bits())
}

/// Read a single out of a 64-bit `f` register, honouring the boxing convention.
///
/// A register whose upper 32 bits are not all ones does not hold a valid single — it
/// holds a double, or a partially written value — and the answer is the canonical NaN
/// rather than the low half reinterpreted.
#[must_use]
pub fn unbox_s(bits: u64) -> f32 {
    if bits >> 32 == 0xffff_ffff {
        f32::from_bits((bits & 0xffff_ffff) as u32)
    } else {
        f32::from_bits(CANONICAL_NAN_S)
    }
}

/// Is this a signalling NaN? `fmin`/`fmax`/comparisons set `NV` for one where a quiet
/// NaN alone does not, so the distinction has to be available.
#[must_use]
pub fn is_snan_d(x: f64) -> bool {
    let b = x.to_bits();
    b & 0x7ff0_0000_0000_0000 == 0x7ff0_0000_0000_0000
        && b & 0x000f_ffff_ffff_ffff != 0
        && b & 0x0008_0000_0000_0000 == 0
}

/// Single-precision [`is_snan_d`].
#[must_use]
pub fn is_snan_s(x: f32) -> bool {
    let b = x.to_bits();
    b & 0x7f80_0000 == 0x7f80_0000 && b & 0x007f_ffff != 0 && b & 0x0040_0000 == 0
}

/// `fclass.d` — the ten-bit class mask, RISC-V bit order.
#[must_use]
pub fn fclass_d(x: f64) -> u64 {
    let b = x.to_bits();
    let neg = b >> 63 == 1;
    let exp = (b >> 52) & 0x7ff;
    let man = b & 0x000f_ffff_ffff_ffff;
    match (exp, man) {
        (0x7ff, 0) => {
            if neg {
                1 << 0
            } else {
                1 << 7
            }
        } // -inf / +inf
        (0x7ff, _) => {
            if man & 0x0008_0000_0000_0000 == 0 {
                1 << 8
            } else {
                1 << 9
            }
        }
        (0, 0) => {
            if neg {
                1 << 3
            } else {
                1 << 4
            }
        } // -0 / +0
        (0, _) => {
            if neg {
                1 << 2
            } else {
                1 << 5
            }
        } // subnormal
        _ => {
            if neg {
                1 << 1
            } else {
                1 << 6
            }
        } // normal
    }
}

/// Single-precision [`fclass_d`].
#[must_use]
pub fn fclass_s(x: f32) -> u64 {
    let b = x.to_bits();
    let neg = b >> 31 == 1;
    let exp = (b >> 23) & 0xff;
    let man = b & 0x007f_ffff;
    match (exp, man) {
        (0xff, 0) => {
            if neg {
                1 << 0
            } else {
                1 << 7
            }
        }
        (0xff, _) => {
            if man & 0x0040_0000 == 0 {
                1 << 8
            } else {
                1 << 9
            }
        }
        (0, 0) => {
            if neg {
                1 << 3
            } else {
                1 << 4
            }
        }
        (0, _) => {
            if neg {
                1 << 2
            } else {
                1 << 5
            }
        }
        _ => {
            if neg {
                1 << 1
            } else {
                1 << 6
            }
        }
    }
}

/// `fmin.d`/`fmax.d`, whose NaN rule is **not** Rust's.
///
/// RISC-V: if exactly one operand is NaN the answer is the other operand; if both are
/// NaN the answer is the canonical NaN; a signalling NaN sets `NV` either way. `-0.0`
/// compares less than `+0.0`, which `f64::min` does not guarantee.
#[must_use]
pub fn minmax_d(a: f64, b: f64, max: bool) -> (f64, u64) {
    let mut flags = 0;
    if is_snan_d(a) || is_snan_d(b) {
        flags |= NV;
    }
    let v = match (a.is_nan(), b.is_nan()) {
        (true, true) => f64::from_bits(CANONICAL_NAN_D),
        (true, false) => b,
        (false, true) => a,
        (false, false) => {
            // -0.0 < +0.0, which a bare comparison does not see.
            if a == 0.0 && b == 0.0 {
                let a_neg = a.is_sign_negative();
                if max == a_neg { b } else { a }
            } else if (a < b) == max {
                b
            } else {
                a
            }
        }
    };
    (v, flags)
}

/// Single-precision [`minmax_d`].
#[must_use]
pub fn minmax_s(a: f32, b: f32, max: bool) -> (f32, u64) {
    let mut flags = 0;
    if is_snan_s(a) || is_snan_s(b) {
        flags |= NV;
    }
    let v = match (a.is_nan(), b.is_nan()) {
        (true, true) => f32::from_bits(CANONICAL_NAN_S),
        (true, false) => b,
        (false, true) => a,
        (false, false) => {
            if a == 0.0 && b == 0.0 {
                let a_neg = a.is_sign_negative();
                if max == a_neg { b } else { a }
            } else if (a < b) == max {
                b
            } else {
                a
            }
        }
    };
    (v, flags)
}

/// Round `x` to an integral `f64` under RISC-V rounding mode `rm`.
///
/// Used by every float-to-integer conversion. `RMM` is "ties away from zero", which is
/// Rust's `round`; `RNE` is "ties to even", which is `round_ties_even`.
#[must_use]
pub fn round_to_integral(x: f64, rm: u32) -> f64 {
    match rm {
        RTZ => x.trunc(),
        RDN => x.floor(),
        RUP => x.ceil(),
        RMM => x.round(),
        _ => x.round_ties_even(),
    }
}

/// `fcvt.{w,wu,l,lu}.{s,d}` — float to integer, with the saturation RISC-V mandates.
///
/// **Out-of-range and NaN do not trap and do not wrap.** The spec pins the result: NaN
/// gives the maximum *unsigned*-or-*positive* value, `-inf` and anything below range give
/// the minimum, `+inf` and anything above give the maximum. All three set `NV`. Wrapping
/// here — the natural thing an `as` cast does in some languages — would make a converted
/// NaN indistinguishable from a small negative integer.
///
/// `signed` picks `w`/`l` over `wu`/`lu`; `bits` is 32 or 64.
#[must_use]
pub fn cvt_to_int(x: f64, rm: u32, signed: bool, bits: u32) -> (u64, u64) {
    let (lo, hi) = if signed {
        match bits {
            32 => (f64::from(i32::MIN), f64::from(i32::MAX)),
            _ => (-(2f64.powi(63)), 2f64.powi(63) - 1024.0),
        }
    } else {
        match bits {
            32 => (0.0, f64::from(u32::MAX)),
            _ => (0.0, 2f64.powi(64) - 2048.0),
        }
    };

    if x.is_nan() {
        // NaN converts to the MAXIMUM, signed or not — not to zero.
        let v = if signed {
            match bits {
                32 => i64::from(i32::MAX) as u64,
                _ => i64::MAX as u64,
            }
        } else {
            match bits {
                32 => u64::from(u32::MAX),
                _ => u64::MAX,
            }
        };
        return (v, NV);
    }

    let r = round_to_integral(x, rm);
    // A SATURATED RESULT RAISES `NV` ALONE: `NX` belongs to a result that was rounded and
    // delivered, and a saturated one was neither. QEMU agrees on all 660 conversions of
    // the (i-d) review's table (`tests/data/v009d_cvt_qemu.tsv`); before, the 86 that
    // round AND saturate — `fcvt.w.d` of 2147483647.5, `fcvt.wu.d rtz` of -1.5 — read
    // `NV|NX`.
    let flags = if r == x { 0 } else { NX };
    if r < lo {
        let v = if signed {
            match bits {
                32 => i64::from(i32::MIN) as u64,
                _ => i64::MIN as u64,
            }
        } else {
            0
        };
        return (v, NV);
    }
    if r > hi {
        let v = if signed {
            match bits {
                32 => i64::from(i32::MAX) as u64,
                _ => i64::MAX as u64,
            }
        } else {
            match bits {
                32 => u64::from(u32::MAX),
                _ => u64::MAX,
            }
        };
        return (v, NV);
    }
    let v = if signed {
        match bits {
            32 => i64::from(r as i32) as u64,
            _ => r as i64 as u64,
        }
    } else {
        match bits {
            32 => u64::from(r as u32),
            _ => r as u64,
        }
    };
    (v, flags)
}

/// `feq`/`flt`/`fle` for doubles. `op` is 2 (`feq`), 1 (`flt`), 0 (`fle`).
///
/// `feq` is the *quiet* comparison: a quiet NaN answers false without `NV`. `flt` and
/// `fle` are the signalling ones and set `NV` on any NaN.
#[must_use]
pub fn compare_d(a: f64, b: f64, op: u32) -> (bool, u64) {
    let any_nan = a.is_nan() || b.is_nan();
    match op {
        2 => {
            let nv = if is_snan_d(a) || is_snan_d(b) { NV } else { 0 };
            (a == b, nv)
        }
        1 => (
            if any_nan { false } else { a < b },
            if any_nan { NV } else { 0 },
        ),
        _ => (
            if any_nan { false } else { a <= b },
            if any_nan { NV } else { 0 },
        ),
    }
}

/// Single-precision [`compare_d`].
#[must_use]
pub fn compare_s(a: f32, b: f32, op: u32) -> (bool, u64) {
    let any_nan = a.is_nan() || b.is_nan();
    match op {
        2 => {
            let nv = if is_snan_s(a) || is_snan_s(b) { NV } else { 0 };
            (a == b, nv)
        }
        1 => (
            if any_nan { false } else { a < b },
            if any_nan { NV } else { 0 },
        ),
        _ => (
            if any_nan { false } else { a <= b },
            if any_nan { NV } else { 0 },
        ),
    }
}

/// Quieten a NaN result to the canonical one, which is what the hardware produces.
#[must_use]
pub fn canonicalise_d(x: f64) -> f64 {
    if x.is_nan() {
        f64::from_bits(CANONICAL_NAN_D)
    } else {
        x
    }
}

/// Single-precision [`canonicalise_d`].
#[must_use]
pub fn canonicalise_s(x: f32) -> f32 {
    if x.is_nan() {
        f32::from_bits(CANONICAL_NAN_S)
    } else {
        x
    }
}

/// `NV` for an arithmetic result: an invalid operation produced a NaN from non-NaN
/// operands (`0 * inf`, `inf - inf`, `0/0`, `sqrt(-x)`).
#[must_use]
pub fn arith_nv(a: f64, b: f64, out: f64) -> u64 {
    // Two ways to be invalid and one flag: a NaN conjured from non-NaN operands
    // (`0 * inf`, `inf - inf`, `0/0`, `sqrt(-x)`), or a signalling NaN on either
    // side. Written as one condition because two arms returning the same value is
    // a distinction the reader has to check and discard.
    let conjured = out.is_nan() && !a.is_nan() && !b.is_nan();
    if conjured || is_snan_d(a) || is_snan_d(b) {
        NV
    } else {
        0
    }
}

/// Single-precision [`arith_nv`].
#[must_use]
pub fn arith_nv_s(a: f32, b: f32, out: f32) -> u64 {
    let conjured = out.is_nan() && !a.is_nan() && !b.is_nan();
    if conjured || is_snan_s(a) || is_snan_s(b) {
        NV
    } else {
        0
    }
}

// ── EXACT IEEE 754 ROUNDING, `V-009` part (i-e) ────────────────────────────────────────
//
// The arithmetic above delegated rounding to Rust, which rounds correctly to nearest-even
// and reports nothing: whether the result was rounded (`NX`), overflowed (`OF`) or
// underflowed (`UF`) is lost. What follows decides all three EXACTLY, the way QEMU's
// softfloat does for RISC-V, and returns the result beside them:
//
//   1. each operation forms its exact result as `±mant × 2^exp` in a `u128` — exactly
//      for `mul` (a 106-bit product) and for `add`/`fma` whenever the operands overlap;
//      when they do not, the far smaller one is JAMMED into one sticky bit, which is
//      exact for rounding because that bit then lies at least 70 places below the
//      result's rounding position ([`add_exact`] carries the argument); `div` and `sqrt`
//      take an integer quotient / root of at least 73 bits and jam a non-zero remainder;
//   2. [`round_pack`] rounds that value once, under any of the five modes — the mode the
//      instruction names, static or through `frm` (the machine refused all but RNE until
//      `V-009` (i-f)).
//
// UNDERFLOW IS DETECTED AFTER ROUNDING, as the RISC-V ISA requires and QEMU implements
// (`tininess_before_rounding` false for the target): a result is tiny when rounding it to
// the format's precision WITH AN UNBOUNDED EXPONENT gives a magnitude below the smallest
// normal, and `UF` is raised only for a tiny result that is also inexact. The witness is
// `(1 + 2^-52) × maxsub = 2^-1022 (1 - 2^-104)`: it rounds to `2^-1022` at 53 bits, so
// RNE flags `NX` alone, where before-rounding detection would add `UF`; RTZ rounds down
// and flags `NX|UF` (probe `probe_fp.S`, edge `d:(1+ulp) * maxsub`).
//
// Every flag and every result bit is checked against QEMU 10.1.0 by
// `tests/v009e_hardware_agreement.rs`: 2,104 inputs x 37 operations x 5 rounding modes.

/// A binary interchange format: precision `p` (with the hidden bit), the normal exponent
/// range, and the encoding width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format {
    /// Significand bits, the hidden one included: 53 or 24.
    pub p: u32,
    /// The exponent of the smallest normal.
    pub emin: i32,
    /// The exponent of the largest finite.
    pub emax: i32,
    /// Encoding width in bits.
    pub width: u32,
}

/// binary64.
pub const F64: Format = Format {
    p: 53,
    emin: -1022,
    emax: 1023,
    width: 64,
};
/// binary32.
pub const F32: Format = Format {
    p: 24,
    emin: -126,
    emax: 127,
    width: 32,
};

impl Format {
    fn sign_bit(self) -> u64 {
        1 << (self.width - 1)
    }
    fn frac_mask(self) -> u64 {
        (1 << (self.p - 1)) - 1
    }
    fn exp_field_max(self) -> u64 {
        (1 << (self.width - self.p)) - 1
    }
    fn bias(self) -> i32 {
        self.emax
    }
    fn inf(self, neg: bool) -> u64 {
        (u64::from(neg) << (self.width - 1)) | (self.exp_field_max() << (self.p - 1))
    }
    fn max_finite(self, neg: bool) -> u64 {
        self.inf(neg) - 1
    }
    fn zero(self, neg: bool) -> u64 {
        u64::from(neg) << (self.width - 1)
    }
    /// RISC-V's canonical quiet NaN for this format.
    fn nan(self) -> u64 {
        (self.exp_field_max() << (self.p - 1)) | (1 << (self.p - 2))
    }
}

/// An operand, classified.
#[derive(Debug, Clone, Copy)]
enum Val {
    Nan {
        signalling: bool,
    },
    Inf(bool),
    Zero(bool),
    /// `±mant × 2^exp`, `mant` non-zero.
    Fin {
        neg: bool,
        mant: u128,
        exp: i32,
    },
}

fn unpack(f: Format, bits: u64) -> Val {
    let neg = bits & f.sign_bit() != 0;
    let e = (bits >> (f.p - 1)) & f.exp_field_max();
    let frac = bits & f.frac_mask();
    if e == f.exp_field_max() {
        return if frac == 0 {
            Val::Inf(neg)
        } else {
            Val::Nan {
                signalling: frac & (1 << (f.p - 2)) == 0,
            }
        };
    }
    if e == 0 {
        if frac == 0 {
            return Val::Zero(neg);
        }
        return Val::Fin {
            neg,
            mant: u128::from(frac),
            exp: f.emin - (f.p as i32 - 1),
        };
    }
    Val::Fin {
        neg,
        mant: u128::from(frac | (1 << (f.p - 1))),
        exp: e as i32 - f.bias() - (f.p as i32 - 1),
    }
}

fn bitlen(m: u128) -> i32 {
    128 - m.leading_zeros() as i32
}

/// `m >> s`, with every bit shifted out ORed into bit 0 (`s` may be any size).
fn shift_right_jam(m: u128, s: i32) -> u128 {
    if s <= 0 {
        m
    } else if s >= 128 {
        u128::from(m != 0)
    } else {
        (m >> s) | u128::from(m & ((1u128 << s) - 1) != 0)
    }
}

/// Drop the low `shift` bits of `mant` and round what is left under `rm`. Returns the
/// rounded integer (it may have carried into a new top bit) and whether anything
/// non-zero was dropped.
fn round_shift(mant: u128, shift: i32, neg: bool, rm: u32) -> (u128, bool) {
    if shift <= 0 {
        return (mant << -shift, false);
    }
    // Compare the dropped part with one half of the kept part's unit.
    let (q, cmp_half, inexact) = if shift > 128 {
        (0, core::cmp::Ordering::Less, mant != 0)
    } else if shift == 128 {
        (0, mant.cmp(&(1u128 << 127)), mant != 0)
    } else {
        let rem = mant & ((1u128 << shift) - 1);
        (mant >> shift, rem.cmp(&(1u128 << (shift - 1))), rem != 0)
    };
    use core::cmp::Ordering::{Equal, Greater};
    let up = match rm {
        RTZ => false,
        RDN => neg && inexact,
        RUP => !neg && inexact,
        RMM => cmp_half != core::cmp::Ordering::Less,
        _ => cmp_half == Greater || (cmp_half == Equal && q & 1 == 1),
    };
    (q + u128::from(up), inexact)
}

/// Round `±mant × 2^exp` (`mant` non-zero) into `f` under `rm`: the encoding and the
/// `NX`/`OF`/`UF` it raises. See the section margin for the rules, after-rounding
/// tininess above all.
fn round_pack(f: Format, neg: bool, mant: u128, exp: i32, rm: u32) -> (u64, u64) {
    debug_assert!(mant != 0);
    let p = f.p as i32;
    let sign = u64::from(neg) << (f.width - 1);
    let len = bitlen(mant);
    let e_top = exp + len - 1;
    // Rounded to p bits with an unbounded exponent.
    let (q, inexact_u) = round_shift(mant, len - p, neg, rm);
    let e_rounded = if q >> p != 0 { e_top + 1 } else { e_top };
    if e_rounded > f.emax {
        let big = match rm {
            RTZ => false,
            RDN => neg,
            RUP => !neg,
            _ => true,
        };
        let v = if big { f.inf(neg) } else { f.max_finite(neg) };
        return (v, OF | NX);
    }
    if e_top >= f.emin {
        let (q, e) = if q >> p != 0 {
            (q >> 1, e_rounded)
        } else {
            (q, e_top)
        };
        let field = (e + f.bias()) as u64;
        let bits = sign | (field << (f.p - 1)) | (q as u64 & f.frac_mask());
        return (bits, if inexact_u { NX } else { 0 });
    }
    // Below the normal range: round at the subnormal unit, 2^(emin - p + 1). A carry to
    // 2^(p-1) is the smallest normal, and its encoding is the same integer.
    let tiny = e_rounded < f.emin;
    let (q, inexact) = round_shift(mant, f.emin - (p - 1) - exp, neg, rm);
    let flags = match (inexact, tiny) {
        (false, _) => 0,
        (true, false) => NX,
        (true, true) => NX | UF,
    };
    (sign | q as u64, flags)
}

/// `±m1 × 2^e1 + ±m2 × 2^e2`, each `m` non-zero and at most 106 bits: the exact sum as
/// `(neg, mant, exp)`, or `None` for an exact zero.
///
/// Both are placed in a 126-bit window topped by the larger of the two top bits. The
/// operand that sets the window fits whole (it is at most 106 bits). The other one fits
/// whole too UNLESS its top bit is at least two places below the first one's — and then
/// the sum is at least `2^(T-2)`, so its rounding position (`p + 2 <= 55` bits down) is
/// far above bit 0, where the jammed sticky bit lands. One sticky bit that low decides
/// inexactness and the round-up exactly as the dropped bits would.
fn add_exact(
    n1: bool,
    m1: u128,
    e1: i32,
    n2: bool,
    m2: u128,
    e2: i32,
) -> Option<(bool, u128, i32)> {
    let top = (e1 + bitlen(m1)).max(e2 + bitlen(m2));
    let base = top - 126;
    let place = |m: u128, e: i32| {
        if e >= base {
            m << (e - base)
        } else {
            shift_right_jam(m, base - e)
        }
    };
    let (x1, x2) = (place(m1, e1), place(m2, e2));
    if n1 == n2 {
        Some((n1, x1 + x2, base))
    } else if x1 > x2 {
        Some((n1, x1 - x2, base))
    } else if x2 > x1 {
        Some((n2, x2 - x1, base))
    } else {
        None
    }
}

/// The NaN answer: the canonical NaN, with `NV` if any operand signals.
fn nan_result(f: Format, ops: &[Val]) -> (u64, u64) {
    let nv = ops
        .iter()
        .any(|v| matches!(v, Val::Nan { signalling: true }));
    (f.nan(), if nv { NV } else { 0 })
}

fn is_nan(v: Val) -> bool {
    matches!(v, Val::Nan { .. })
}

/// An exact zero from cancellation: `+0`, or `-0` when rounding down.
fn cancel_zero(f: Format, rm: u32) -> u64 {
    f.zero(rm == RDN)
}

fn add_vals(f: Format, a: Val, b: Val, rm: u32) -> (u64, u64) {
    match (a, b) {
        (Val::Nan { .. }, _) | (_, Val::Nan { .. }) => nan_result(f, &[a, b]),
        (Val::Inf(x), Val::Inf(y)) => {
            if x == y {
                (f.inf(x), 0)
            } else {
                (f.nan(), NV)
            }
        }
        (Val::Inf(x), _) | (_, Val::Inf(x)) => (f.inf(x), 0),
        (Val::Zero(x), Val::Zero(y)) => (
            if x == y {
                f.zero(x)
            } else {
                cancel_zero(f, rm)
            },
            0,
        ),
        (Val::Zero(_), Val::Fin { neg, mant, exp })
        | (Val::Fin { neg, mant, exp }, Val::Zero(_)) => round_pack(f, neg, mant, exp, rm),
        (
            Val::Fin {
                neg: n1,
                mant: m1,
                exp: e1,
            },
            Val::Fin {
                neg: n2,
                mant: m2,
                exp: e2,
            },
        ) => match add_exact(n1, m1, e1, n2, m2, e2) {
            Some((n, m, e)) => round_pack(f, n, m, e, rm),
            None => (cancel_zero(f, rm), 0),
        },
    }
}

fn negate(v: Val) -> Val {
    match v {
        Val::Inf(n) => Val::Inf(!n),
        Val::Zero(n) => Val::Zero(!n),
        Val::Fin { neg, mant, exp } => Val::Fin {
            neg: !neg,
            mant,
            exp,
        },
        nan => nan,
    }
}

/// `a + b` in format `f`, rounding mode `rm` (0..=4): the result bits and the flags.
#[must_use]
pub fn add(f: Format, a: u64, b: u64, rm: u32) -> (u64, u64) {
    add_vals(f, unpack(f, a), unpack(f, b), rm)
}

/// `a - b`.
#[must_use]
pub fn sub(f: Format, a: u64, b: u64, rm: u32) -> (u64, u64) {
    add_vals(f, unpack(f, a), negate(unpack(f, b)), rm)
}

/// The exact product of two operands that are not NaN.
enum Product {
    /// `inf × 0`.
    Invalid,
    Inf(bool),
    Zero(bool),
    /// `±mant × 2^exp`.
    Fin(bool, u128, i32),
}

fn product(a: Val, b: Val) -> Product {
    match (a, b) {
        (Val::Inf(_), Val::Zero(_)) | (Val::Zero(_), Val::Inf(_)) => Product::Invalid,
        (Val::Inf(x), Val::Inf(y)) => Product::Inf(x != y),
        (Val::Inf(x), Val::Fin { neg, .. }) | (Val::Fin { neg, .. }, Val::Inf(x)) => {
            Product::Inf(x != neg)
        }
        (Val::Zero(x), Val::Zero(y)) => Product::Zero(x != y),
        (Val::Zero(x), Val::Fin { neg, .. }) | (Val::Fin { neg, .. }, Val::Zero(x)) => {
            Product::Zero(x != neg)
        }
        (
            Val::Fin {
                neg: n1,
                mant: m1,
                exp: e1,
            },
            Val::Fin {
                neg: n2,
                mant: m2,
                exp: e2,
            },
        ) => Product::Fin(n1 != n2, m1 * m2, e1 + e2),
        _ => unreachable!("NaN operands are answered before the product"),
    }
}

/// `a × b`.
#[must_use]
pub fn mul(f: Format, a: u64, b: u64, rm: u32) -> (u64, u64) {
    let (a, b) = (unpack(f, a), unpack(f, b));
    if is_nan(a) || is_nan(b) {
        return nan_result(f, &[a, b]);
    }
    match product(a, b) {
        Product::Invalid => (f.nan(), NV),
        Product::Inf(neg) => (f.inf(neg), 0),
        Product::Zero(neg) => (f.zero(neg), 0),
        Product::Fin(neg, mant, exp) => round_pack(f, neg, mant, exp, rm),
    }
}

/// `±(a × b) ± c`, fused: one rounding. `neg_prod` negates the product and `neg_add` the
/// addend, which is how the four fused instructions differ.
///
/// `inf × 0` is invalid EVEN WHEN `c` IS A QUIET NaN — the ISA says so and QEMU agrees
/// (probe edge `inf * 0 + qnan`: `NV`).
#[must_use]
pub fn fma(
    f: Format,
    a: u64,
    b: u64,
    c: u64,
    neg_prod: bool,
    neg_add: bool,
    rm: u32,
) -> (u64, u64) {
    let (a, b, c) = (unpack(f, a), unpack(f, b), unpack(f, c));
    let inf_zero = matches!(
        (a, b),
        (Val::Inf(_), Val::Zero(_)) | (Val::Zero(_), Val::Inf(_))
    );
    if is_nan(a) || is_nan(b) || is_nan(c) {
        let (v, fl) = nan_result(f, &[a, b, c]);
        return (v, if inf_zero { fl | NV } else { fl });
    }
    let c = if neg_add { negate(c) } else { c };
    let p = match product(a, b) {
        Product::Invalid => return (f.nan(), NV),
        Product::Inf(neg) => Val::Inf(neg != neg_prod),
        Product::Zero(neg) => Val::Zero(neg != neg_prod),
        Product::Fin(neg, mant, exp) => Val::Fin {
            neg: neg != neg_prod,
            mant,
            exp,
        },
    };
    add_vals(f, p, c, rm)
}

/// `a / b`. `DZ` only for a finite non-zero dividend: `inf / 0` is an exact infinity
/// with no flag (probe edge `inf / 0`).
#[must_use]
pub fn div(f: Format, a: u64, b: u64, rm: u32) -> (u64, u64) {
    let (a, b) = (unpack(f, a), unpack(f, b));
    match (a, b) {
        (Val::Nan { .. }, _) | (_, Val::Nan { .. }) => nan_result(f, &[a, b]),
        (Val::Inf(_), Val::Inf(_)) | (Val::Zero(_), Val::Zero(_)) => (f.nan(), NV),
        (Val::Inf(x), Val::Zero(y) | Val::Fin { neg: y, .. }) => (f.inf(x != y), 0),
        (Val::Zero(x), Val::Inf(y) | Val::Fin { neg: y, .. }) => (f.zero(x != y), 0),
        (Val::Fin { neg: x, .. }, Val::Inf(y)) => (f.zero(x != y), 0),
        (Val::Fin { neg: x, .. }, Val::Zero(y)) => (f.inf(x != y), DZ),
        (
            Val::Fin {
                neg: n1,
                mant: m1,
                exp: e1,
            },
            Val::Fin {
                neg: n2,
                mant: m2,
                exp: e2,
            },
        ) => {
            // A dividend of 127 bits over a divisor of at most 53 leaves a quotient of at
            // least 74; a non-zero remainder becomes the jammed bit below it.
            let s = 127 - bitlen(m1);
            let num = m1 << s;
            let (q, r) = (num / m2, num % m2);
            let mant = (q << 1) | u128::from(r != 0);
            round_pack(f, n1 != n2, mant, e1 - s - e2 - 1, rm)
        }
    }
}

/// `floor(sqrt(m))`, exactly.
fn isqrt(m: u128) -> u128 {
    if m == 0 {
        return 0;
    }
    // A float estimate, Newton's steps (the first lands at or above the root, and from
    // there they descend), then an exact correction of at most a step or two.
    let r0 = ((m as f64).sqrt() as u128).max(1);
    let mut r = (r0 + m / r0) / 2 + 1;
    loop {
        let next = (r + m / r) / 2;
        if next >= r {
            break;
        }
        r = next;
    }
    while r * r > m {
        r -= 1;
    }
    while (r + 1) * (r + 1) <= m {
        r += 1;
    }
    r
}

/// `sqrt(a)`. Never overflows or underflows; `NX` is a non-zero integer remainder.
#[must_use]
pub fn sqrt(f: Format, a: u64, rm: u32) -> (u64, u64) {
    match unpack(f, a) {
        v @ Val::Nan { .. } => nan_result(f, &[v]),
        Val::Zero(neg) => (f.zero(neg), 0),
        Val::Inf(false) => (f.inf(false), 0),
        Val::Inf(true) | Val::Fin { neg: true, .. } => (f.nan(), NV),
        Val::Fin { mant, exp, .. } => {
            let (mant, exp) = if exp & 1 != 0 {
                (mant << 1, exp - 1)
            } else {
                (mant, exp)
            };
            // An even shift up to 125 or 126 bits leaves a root of at least 63.
            let mut s = 126 - bitlen(mant);
            if s & 1 != 0 {
                s -= 1;
            }
            let m = mant << s;
            let r = isqrt(m);
            let root = (r << 1) | u128::from(r * r != m);
            round_pack(f, false, root, (exp - s) / 2 - 1, rm)
        }
    }
}

/// Convert between formats (`fcvt.s.d`, `fcvt.d.s`).
#[must_use]
pub fn convert(from: Format, to: Format, a: u64, rm: u32) -> (u64, u64) {
    match unpack(from, a) {
        v @ Val::Nan { .. } => nan_result(to, &[v]),
        Val::Inf(neg) => (to.inf(neg), 0),
        Val::Zero(neg) => (to.zero(neg), 0),
        Val::Fin { neg, mant, exp } => round_pack(to, neg, mant, exp, rm),
    }
}

/// An integer to format `f` (`fcvt.{s,d}.{w,wu,l,lu}`); zero is `+0`.
#[must_use]
pub fn from_int(f: Format, v: i128, rm: u32) -> (u64, u64) {
    if v == 0 {
        return (0, 0);
    }
    round_pack(f, v < 0, v.unsigned_abs(), 0, rm)
}
