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
//! So: **arithmetic** (`fadd`/`fsub`/`fmul`/`fdiv`/`fsqrt`/`fmadd`…) honours `RNE` and
//! `DYN`-resolving-to-`RNE`, and **halts `Unimplemented` on any other mode**. A halt names
//! the gap; a wrong low bit hides it.
//!
//! **Float-to-integer conversion honours all five modes**, because rounding there is
//! exact integer arithmetic rather than a hardware mode — and because `rtz` is not
//! exotic: `fcvt.w.d rd, rs1, rtz` is the ordinary way a compiler truncates a float, so
//! refusing it would refuse the common case.
//!
//! # `fflags` — WHAT IS SET AND WHAT IS NOT, stated rather than implied
//!
//! `NV` (invalid) and `DZ` (divide-by-zero) are set, because both are decidable from the
//! operands without emulating the rounding hardware. **`NX`, `OF` and `UF` are NOT tracked
//! for arithmetic**, and this is a known gap rather than an oversight: deciding "inexact"
//! requires the rounding decision this module delegates to Rust. `NX` *is* set for
//! float-to-integer conversions, where it is exactly "the value had a fractional part".
//! A program that reads `fflags` expecting `NX` after `fdiv` will read 0. Do not close
//! this by guessing; it needs the software rounding the margin above declined.
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
    let mut flags = 0;
    if r != x {
        flags |= NX;
    }
    if r < lo {
        let v = if signed {
            match bits {
                32 => i64::from(i32::MIN) as u64,
                _ => i64::MIN as u64,
            }
        } else {
            0
        };
        return (v, flags | NV);
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
        return (v, flags | NV);
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

/// Is this rounding mode one the arithmetic path can honour?
///
/// See the module margin: `RNE` and `DYN` are accepted, everything else is refused so a
/// wrong low bit cannot pass for a right one.
#[must_use]
pub fn arith_rm_supported(rm: u32, frm: u32) -> bool {
    let effective = if rm == DYN { frm } else { rm };
    effective == RNE
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
