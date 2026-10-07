//! **The V extension, an executed subset** — row `V-007`.
//!
//! # What executes, exactly
//!
//! The owner fixed the shape on 2026-10-03: `VLEN = 128`, and `v0` reserved as the mask
//! register with **masking refused by name**. The executed subset is the coordinator's
//! default from the same day:
//!
//! ```text
//!   OPCFG     vsetvli  vsetivli  vsetvl                any vtype; vill as on QEMU
//!   LOAD-FP   vle64.v  vlse64.v                        unit-stride and strided, e64
//!   STORE-FP  vse64.v  vsse64.v
//!   OPIVV/X   vadd  vsub               .vv .vx
//!   OPMVV/X   vmul                     .vv .vx
//!   OPFVV/F   vfadd vfsub vfmul vfdiv  .vv .vf
//!   OPFVV     vfredosum.vs                             the ordered sum
//! ```
//!
//! The ENGINE computes at `SEW` = 64 and `LMUL` 1, 2, 4 or 8, so `VLMAX` is `2 × LMUL`
//! for everything it executes. **THE `vtype` CSR IS QEMU'S, WHOLE** (`V-009` (i-e)):
//! `vsetvli`/`vsetivli`/`vsetvl` accept every `vtype` QEMU accepts — e32 m1, e8 mf8,
//! e64 mf2 — set `vl` from it, and set `vill` for exactly the ones QEMU rejects
//! ([`Machine::set_vtype`], measured over every `vtypei`). An instruction executed under
//! a legal `vtype` the engine does not compute at halts by name ([`REFUSE_VTYPE`]). Until
//! (i-e) the `vsetvli` itself was refused, which agreed with no hardware: QEMU runs e32 m1
//! and sets `vill` for e64 mf4, and a program probing `vl` for a width saw a halt instead.
//!
//! **THE TAIL IS UNDISTURBED, ALWAYS.** Elements at and past `vl`, up to the end of the
//! register group, and every element of a reduction's destination except element 0, keep
//! what they held. That is the only behaviour `tu` permits and one of the two `ta` permits,
//! so one rule serves both and `vta` is recorded and never consulted. Elements below
//! `vstart` are likewise untouched, and every instruction that completes leaves `vstart`
//! zero.
//!
//! # Everything outside the subset halts BY NAME
//!
//! [`decode`] is a pure function from a word to either an [`Op`] or a refusal string. The
//! machine halts [`crate::Halt::Unimplemented`] on a refusal — the same halt `V-001` used
//! for a rounding mode it could not honour — and `yantra-run` prints [`decode`]'s reason
//! next to it, so a program that reaches past the subset is told which edge it hit:
//! masking, an element width, a segment, an indexed access, an immediate form, or an
//! operation the subset does not list.
//!
//! What is an ILLEGAL INSTRUCTION on hardware stays one here and traps (cause 2) rather
//! than halting: any vector instruction that reads `vtype` while `vtype.vill` is set (it
//! is at reset, so a program that forgets `vsetvli` traps exactly as it would on metal) —
//! inside the subset OR OUTSIDE IT, since (i-e), as QEMU traps a masked or `.vi` form under
//! `vill` before anything else; only the whole-register loads and stores, which do not
//! read `vtype`, are exempt — a register number not aligned to `LMUL`, an ordered sum
//! started with `vstart ≠ 0`, and the reserved space beside `vsetvl`.
//!
//! # Floating point follows `V-001`'s rule
//!
//! The subset's vector float arms — vfadd, vfsub, vfmul, vfdiv (.vv and .vf) and the
//! ordered sum vfredosum.vs, at e64 ONLY; every other vector float instruction (vfsqrt,
//! vfmacc and its family, vfcvt/vfncvt, any e32 form) halts `Unimplemented` by name, as it
//! always has — round through `frm` (they have no `rm` field), and a dynamic mode
//! other than RNE halted until V-009 (i-f) rather than rounding to nearest and agreeing
//! with nobody; since (i-f) every mode rounds, through the scalar arms' core. Every
//! flag — `NV`, `DZ`, and since `V-009` (i-e) `NX`, `OF` and `UF` — accrues exactly as the
//! scalar arms accrue it, through the same rounding core (`crate::fp::add` and its kin).

use crate::{Access, Halt, Machine, Output, fp};

/// `VLEN`, in bits. The owner's figure.
pub const VLEN: u64 = 128;
/// `VLENB` — what CSR `vlenb` (0xc22) reads.
pub const VLENB: u64 = VLEN / 8;
/// Elements of 64 bits per register.
const LANES: usize = (VLEN / 64) as usize;
/// `vtype.vill`, bit 63 on RV64.
pub const VILL: u64 = 1 << 63;

/// The vector register file and its three pieces of control state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorUnit {
    /// `v0`..`v31`, each two 64-bit elements. Lanes rather than octets because the only
    /// element width this machine executes is 64; a wider subset would change this.
    pub v: [[u64; LANES]; 32],
    /// `vl`.
    pub vl: u64,
    /// `vtype`. Reset holds [`VILL`], as the specification recommends: a vector
    /// instruction before the first `vsetvli` is illegal.
    pub vtype: u64,
    /// `vstart` — the element a resumed instruction starts at.
    pub vstart: u64,
    /// `vxrm`, the fixed-point rounding mode: two bits. `V-009` (i-e).
    pub vxrm: u64,
    /// `vxsat`, the fixed-point saturation flag: one bit. `V-009` (i-e).
    pub vxsat: u64,
}

impl Default for VectorUnit {
    fn default() -> Self {
        Self {
            v: [[0; LANES]; 32],
            vl: 0,
            vtype: VILL,
            vstart: 0,
            vxrm: 0,
            vxsat: 0,
        }
    }
}

impl VectorUnit {
    /// Element `i` of the register group based at `reg`.
    #[must_use]
    pub fn get(&self, reg: usize, i: usize) -> u64 {
        self.v[reg + i / LANES][i % LANES]
    }

    fn set(&mut self, reg: usize, i: usize, value: u64) {
        self.v[reg + i / LANES][i % LANES] = value;
    }

    /// `LMUL` as a register count, from the current `vtype`. Asked only once
    /// [`engine_executes`] has said the `vtype` is integer-LMUL e64.
    fn lmul(&self) -> usize {
        1 << (self.vtype & 0x7)
    }
}

/// Integer operations of the subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntOp {
    /// `vadd`
    Add,
    /// `vsub` — `vs2 - vs1` / `vs2 - x[rs1]`.
    Sub,
    /// `vmul` — the low 64 bits.
    Mul,
}

/// Float operations of the subset, all on doubles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FpOp {
    /// `vfadd`
    Add,
    /// `vfsub` — `vs2 - vs1` / `vs2 - f[rs1]`.
    Sub,
    /// `vfmul`
    Mul,
    /// `vfdiv` — `vs2 / vs1` / `vs2 / f[rs1]`.
    Div,
}

/// The second operand of an element-wise operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Src {
    /// `.vv` — a vector register group.
    V(usize),
    /// `.vx` — an integer register.
    X(usize),
    /// `.vf` — a float register.
    F(usize),
}

/// One instruction of the executed subset, decoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// `vsetivli rd, uimm, vtypei` — `V-009` (i-d): the AVL is the five-bit unsigned
    /// immediate in the `rs1` field, and nothing else differs from [`Op::SetVli`].
    SetIvli {
        /// Destination for the new `vl`.
        rd: usize,
        /// The AVL, 0..=31.
        uimm: u64,
        /// The 10-bit immediate `vtype`.
        vtypei: u64,
    },
    /// `vsetvli rd, rs1, vtypei`.
    SetVli {
        /// Destination for the new `vl`.
        rd: usize,
        /// The AVL register.
        rs1: usize,
        /// The 11-bit immediate `vtype`.
        vtypei: u64,
    },
    /// `vsetvl rd, rs1, rs2` — `V-009` (i-e): the whole 64-bit `vtype` from a register.
    SetVl {
        /// Destination for the new `vl`.
        rd: usize,
        /// The AVL register.
        rs1: usize,
        /// The register holding the requested `vtype`.
        rs2: usize,
    },
    /// `vle64.v` (`stride: None`) or `vlse64.v` (`stride: Some(rs2)`).
    Load {
        /// Destination group.
        vd: usize,
        /// Base address register.
        rs1: usize,
        /// The stride register, for the strided form.
        stride: Option<usize>,
    },
    /// `vse64.v` / `vsse64.v`.
    Store {
        /// Source group.
        vs3: usize,
        /// Base address register.
        rs1: usize,
        /// The stride register, for the strided form.
        stride: Option<usize>,
    },
    /// `vadd`/`vsub`/`vmul`, `.vv` or `.vx`.
    Int {
        /// Which.
        op: IntOp,
        /// Destination group.
        vd: usize,
        /// First source group.
        vs2: usize,
        /// Second source.
        src: Src,
    },
    /// `vfadd`/`vfsub`/`vfmul`/`vfdiv`, `.vv` or `.vf`.
    Fp {
        /// Which.
        op: FpOp,
        /// Destination group.
        vd: usize,
        /// First source group.
        vs2: usize,
        /// Second source.
        src: Src,
    },
    /// `vfredosum.vs vd, vs2, vs1` — `vd[0] = vs1[0] + vs2[0] + … + vs2[vl-1]`, in order.
    RedOSum {
        /// Destination: element 0 only.
        vd: usize,
        /// The vector summed.
        vs2: usize,
        /// The scalar the sum starts from, element 0.
        vs1: usize,
    },
}

/// The refusal every masked form gets. One string, so a test can name it.
pub const REFUSE_MASKED: &str = "a masked form (vm = 0): v0 is reserved as the mask register \
     and masking is refused by name in V-007";
/// Element widths other than 64.
pub const REFUSE_WIDTH: &str =
    "an 8-, 16- or 32-bit element: V-007 executes e64 loads, stores and vtypes only";
/// Segment loads and stores.
pub const REFUSE_SEGMENT: &str = "a segment load/store (nf > 0): outside V-007's subset";
/// Indexed loads and stores.
pub const REFUSE_INDEXED: &str =
    "an indexed (gather/scatter) load/store: V-007 executes unit-stride and strided only";
/// Whole-register, mask and fault-only-first forms of the unit-stride space.
pub const REFUSE_UNIT_VARIANT: &str = "a whole-register, mask (vlm/vsm) or fault-only-first \
     unit-stride form: V-007 executes plain vle64/vse64 only";
/// `mew = 1`.
pub const REFUSE_MEW: &str = "mew = 1, a reserved element-width encoding";
/// The reserved space beside `vsetvl`: bits 31:30 = `10` with bits 30:25 not zero. Not a
/// refusal — an ILLEGAL INSTRUCTION, as on QEMU (`probe_ctl.S` section V, `vsetvl-f7`
/// 0x41..0x5f all trap cause 2). `vsetvl` itself executes since `V-009` (i-e).
pub const ILLEGAL_SETVL_SPACE: &str = "bits 31:30 = 10 with bits 30:25 non-zero: reserved, \
     an illegal instruction";
/// An instruction executed under a `vtype` that `vsetvli` ACCEPTED but this machine's
/// element-wise engine does not run: SEW other than 64, or a fractional LMUL. Since
/// `V-009` (i-e) the `vtype` itself is accepted exactly as QEMU accepts it (e32 m1 and
/// e64 mf2 included) and the refusal moved here, to the first instruction that would need
/// the engine to compute at that width.
pub const REFUSE_VTYPE: &str = "an operation under a vtype other than SEW=64 with LMUL 1, \
     2, 4 or 8: the vtype is held, but V-007 executes e64 at integer LMUL only";
/// `.vi` forms.
pub const REFUSE_IMMEDIATE: &str = "a .vi immediate form: V-007 executes .vv and .vx only";
/// An operation the subset does not list.
pub const REFUSE_OP: &str = "a vector operation outside V-007's executed subset (vadd vsub \
     vmul .vv/.vx; vfadd vfsub vfmul vfdiv .vv/.vf; vfredosum.vs)";

/// Is this LOAD-FP/STORE-FP `width` (funct3) a vector one? `0`, `5`, `6`, `7` are e8,
/// e16, e32, e64; `1`–`4` are the scalar `flh`/`flw`/`fld`/`flq`.
#[must_use]
pub fn is_vector_width(funct3: u32) -> bool {
    matches!(funct3, 0 | 5 | 6 | 7)
}

/// Decode a vector word — OP-V (`0x57`), or LOAD-FP/STORE-FP with a vector width — into
/// the subset, or name why it is not in it.
///
/// # Errors
/// The refusal, as one of the `REFUSE_*` strings.
pub fn decode(word: u32) -> Result<Op, &'static str> {
    let opcode = word & 0x7f;
    let rd = ((word >> 7) & 0x1f) as usize;
    let funct3 = (word >> 12) & 0x7;
    let rs1 = ((word >> 15) & 0x1f) as usize;
    let rs2 = ((word >> 20) & 0x1f) as usize;
    let vm = (word >> 25) & 1;
    let funct6 = word >> 26;
    match opcode {
        0x07 | 0x27 => {
            let nf = word >> 29;
            let mew = (word >> 28) & 1;
            let mop = (word >> 26) & 0x3;
            // THE ORDER IS THE ADDRESSING MODE FIRST, then the fields it gives meaning
            // to: a whole-register access spells its register count in `nf` and is
            // always encoded with width 0, so asking about `nf` or the width before
            // asking which access this is names the wrong edge (`vs1r.v` read as an
            // 8-bit element, `vl2re64.v` as a segment).
            if vm == 0 {
                return Err(REFUSE_MASKED);
            }
            let stride = match mop {
                0b00 if rs2 == 0 => None,
                0b00 => return Err(REFUSE_UNIT_VARIANT),
                0b10 => Some(rs2),
                _ => return Err(REFUSE_INDEXED),
            };
            if mew != 0 {
                return Err(REFUSE_MEW);
            }
            if nf != 0 {
                return Err(REFUSE_SEGMENT);
            }
            if funct3 != 7 {
                return Err(REFUSE_WIDTH);
            }
            Ok(if opcode == 0x07 {
                Op::Load {
                    vd: rd,
                    rs1,
                    stride,
                }
            } else {
                Op::Store {
                    vs3: rd,
                    rs1,
                    stride,
                }
            })
        }
        0x57 => {
            if funct3 == 7 {
                // Bit 31 clear: `vsetvli`, an 11-bit vtype. Bits 31:30 = 11: `vsetivli`,
                // a 10-bit vtype and the AVL as an immediate. Bits 31:25 = 1000000:
                // `vsetvl`. EVERY vtype decodes — whether it is legal is `vill`'s to say,
                // at execution ([`Machine::set_vtype`]), as on QEMU.
                let (vtypei, ivli) = match word >> 30 {
                    0 | 1 => (u64::from((word >> 20) & 0x7ff), false),
                    3 => (u64::from((word >> 20) & 0x3ff), true),
                    _ if word >> 25 == 0b100_0000 => return Ok(Op::SetVl { rd, rs1, rs2 }),
                    _ => return Err(ILLEGAL_SETVL_SPACE),
                };
                return Ok(if ivli {
                    Op::SetIvli {
                        rd,
                        uimm: rs1 as u64,
                        vtypei,
                    }
                } else {
                    Op::SetVli { rd, rs1, vtypei }
                });
            }
            if vm == 0 {
                return Err(REFUSE_MASKED);
            }
            let (vd, vs2) = (rd, rs2);
            let int = |op| {
                Ok(Op::Int {
                    op,
                    vd,
                    vs2,
                    src: Src::V(rs1),
                })
            };
            let intx = |op| {
                Ok(Op::Int {
                    op,
                    vd,
                    vs2,
                    src: Src::X(rs1),
                })
            };
            let fv = |op| {
                Ok(Op::Fp {
                    op,
                    vd,
                    vs2,
                    src: Src::V(rs1),
                })
            };
            let ff = |op| {
                Ok(Op::Fp {
                    op,
                    vd,
                    vs2,
                    src: Src::F(rs1),
                })
            };
            match (funct3, funct6) {
                (0, 0b000000) => int(IntOp::Add),
                (0, 0b000010) => int(IntOp::Sub),
                (4, 0b000000) => intx(IntOp::Add),
                (4, 0b000010) => intx(IntOp::Sub),
                (2, 0b100101) => int(IntOp::Mul),
                (6, 0b100101) => intx(IntOp::Mul),
                (1, 0b000000) => fv(FpOp::Add),
                (1, 0b000010) => fv(FpOp::Sub),
                (1, 0b100100) => fv(FpOp::Mul),
                (1, 0b100000) => fv(FpOp::Div),
                (5, 0b000000) => ff(FpOp::Add),
                (5, 0b000010) => ff(FpOp::Sub),
                (5, 0b100100) => ff(FpOp::Mul),
                (5, 0b100000) => ff(FpOp::Div),
                (1, 0b000011) => Ok(Op::RedOSum { vd, vs2, vs1: rs1 }),
                (3, _) => Err(REFUSE_IMMEDIATE),
                _ => Err(REFUSE_OP),
            }
        }
        _ => Err(REFUSE_OP),
    }
}

/// `VLMAX` under `vtype`, or `None` if `vtype` is one QEMU sets `vill` for (see
/// [`Machine::set_vtype`]).
#[must_use]
pub fn vlmax_of(vtype: u64) -> Option<u64> {
    let (vsew, vlmul) = ((vtype >> 3) & 0x7, vtype & 0x7);
    if vtype >> 8 != 0 || vlmul == 4 || vsew > 3 {
        return None;
    }
    let sew = 8 << vsew;
    // LMUL is 2^vlmul for 0..3 and 2^(vlmul - 8) for the fractions 5..7.
    let vlmax = if vlmul < 4 {
        (VLEN << vlmul) / sew
    } else {
        (VLEN >> (8 - vlmul)) / sew
    };
    (vlmax >= 1).then_some(vlmax)
}

/// Does the engine execute at this (legal) `vtype`? SEW 64 at LMUL 1, 2, 4 or 8. An
/// instruction under any other halts by name, [`REFUSE_VTYPE`].
#[must_use]
pub fn engine_executes(vtype: u64) -> bool {
    (vtype >> 3) & 0x7 == 3 && vtype & 0x4 == 0
}

/// A whole-register load or store (`vl<n>re<eew>.v`, `vs<n>r.v`): unit-stride with the
/// `lumop`/`sumop` field 01000. They do not read `vtype`, so `vill` does not make them
/// illegal (QEMU `probe_ctl.S` section L: `vl1re64.v` and `vs1r.v` execute under vill).
fn is_whole_register(word: u32) -> bool {
    matches!(word & 0x7f, 0x07 | 0x27) && (word >> 26) & 0x3 == 0 && (word >> 20) & 0x1f == 0b01000
}

/// Where a `vsetvli`/`vsetivli` takes its application vector length from.
#[derive(Debug, Clone, Copy)]
enum Avl {
    /// A number: `x[rs1]`, or `vsetivli`'s immediate.
    Value(u64),
    /// `vsetvli rd, x0` with `rd != x0`: as many as fit.
    Max,
    /// `vsetvli x0, x0`: keep `vl`.
    Keep,
}

/// What [`Machine::step_vector`] tells `step_inner`: carry on to `pc + 4`, or return this.
pub(crate) type Flow = Result<(), Option<Halt>>;

impl Machine {
    /// Record that the vector state is live: `sstatus.VS = dirty`. The same contract
    /// [`Machine::mark_fp_dirty`] keeps for `FS`.
    fn mark_vs_dirty(&mut self) {
        self.csr.sstatus |= crate::SSTATUS_VS;
    }

    /// Execute one vector word. `Ok(())` means it retired and `pc` advances normally;
    /// `Err(h)` is returned from `step_inner` as is (a halt, a fault, or `None` after an
    /// illegal-instruction trap has already moved `pc`).
    pub(crate) fn step_vector(&mut self, word: u32, pc: u64, out: &mut impl Output) -> Flow {
        let opcode = (word & 0x7f) as u8;
        let vill = self.vec.vtype & VILL != 0;
        let op = match decode(word) {
            Ok(op) => op,
            // ILLEGAL, NOT REFUSED (`V-009` (i-e), QEMU `probe_ctl.S` section L): under
            // `vill` every vector instruction that depends on `vtype` traps cause 2 —
            // masked, `.vi`, outside the subset, it does not matter which — and only the
            // whole-register loads and stores, which do not read `vtype`, go on to be
            // refused by name. The reserved `vsetvl` space is illegal under any `vtype`.
            Err(ILLEGAL_SETVL_SPACE) => return Err(self.illegal(word, pc)),
            Err(_) if vill && !is_whole_register(word) => return Err(self.illegal(word, pc)),
            Err(_) => return Err(Some(Halt::Unimplemented { pc, word, opcode })),
        };
        match op {
            Op::SetVli { rd, rs1, vtypei } => {
                // AVL from a register; `rs1 = x0` asks for VLMAX, or with `rd = x0` too
                // to keep `vl`.
                let avl = if rs1 != 0 {
                    Avl::Value(self.x[rs1])
                } else if rd != 0 {
                    Avl::Max
                } else {
                    Avl::Keep
                };
                self.set_vtype(rd, avl, vtypei);
                return Ok(());
            }
            Op::SetIvli { rd, uimm, vtypei } => {
                // The immediate is the AVL whatever `rd` is — `vsetivli x0, 3, ...` sets
                // vl = 3 (QEMU probe V7) — and 0 is an AVL of zero, not "VLMAX".
                self.set_vtype(rd, Avl::Value(uimm), vtypei);
                return Ok(());
            }
            Op::SetVl { rd, rs1, rs2 } => {
                // `vsetvli`'s AVL rule, with the vtype read whole from `x[rs2]` — bit 63
                // included, which asks for `vill` and gets it (QEMU `probe_keep.S` K10).
                let avl = if rs1 != 0 {
                    Avl::Value(self.x[rs1])
                } else if rd != 0 {
                    Avl::Max
                } else {
                    Avl::Keep
                };
                let vtype = self.x[rs2];
                self.set_vtype(rd, avl, vtype);
                return Ok(());
            }
            _ => {}
        }
        if vill {
            return Err(self.illegal(word, pc));
        }
        // A legal vtype the engine does not compute at — refused by name (yantra-run
        // prints this reason; [`decode`] has none to give for a word that decodes).
        if !engine_executes(self.vec.vtype) {
            return Err(Some(Halt::Unimplemented { pc, word, opcode }));
        }
        let lmul = self.vec.lmul();
        let aligned = |r: usize| r.is_multiple_of(lmul);
        let vl = self.vec.vl as usize;
        let start = self.vec.vstart as usize;
        match op {
            Op::SetVli { .. } | Op::SetIvli { .. } | Op::SetVl { .. } => {
                unreachable!("handled above")
            }
            Op::Load { vd, rs1, stride } => {
                if !aligned(vd) {
                    return Err(self.illegal(word, pc));
                }
                let base = self.x[rs1];
                let step = stride.map_or(8, |r| self.x[r]);
                for i in start..vl {
                    let addr = base.wrapping_add(step.wrapping_mul(i as u64));
                    let got = self
                        .translate_span(addr, 8, Access::Load, pc)
                        .and_then(|pa| self.load(pa, 8, false, pc));
                    match got {
                        Ok(v) => self.vec.set(vd, i, v),
                        Err(h) => {
                            self.vec.vstart = i as u64;
                            return Err(Some(h));
                        }
                    }
                }
                self.mark_vs_dirty();
            }
            Op::Store { vs3, rs1, stride } => {
                if !aligned(vs3) {
                    return Err(self.illegal(word, pc));
                }
                let base = self.x[rs1];
                let step = stride.map_or(8, |r| self.x[r]);
                for i in start..vl {
                    let addr = base.wrapping_add(step.wrapping_mul(i as u64));
                    let value = self.vec.get(vs3, i);
                    let done = self
                        .translate_span(addr, 8, Access::Store, pc)
                        .and_then(|pa| self.store(pa, 8, value, pc, out));
                    match done {
                        Ok(None) => {}
                        Ok(Some(h)) | Err(h) => {
                            self.vec.vstart = i as u64;
                            return Err(Some(h));
                        }
                    }
                }
            }
            Op::Int { op, vd, vs2, src } => {
                if !aligned(vd) || !aligned(vs2) || matches!(src, Src::V(r) if !aligned(r)) {
                    return Err(self.illegal(word, pc));
                }
                for i in start..vl {
                    let a = self.vec.get(vs2, i);
                    let b = match src {
                        Src::V(r) => self.vec.get(r, i),
                        Src::X(r) => self.x[r],
                        Src::F(_) => unreachable!("decode gives integer ops no float source"),
                    };
                    let r = match op {
                        IntOp::Add => a.wrapping_add(b),
                        IntOp::Sub => a.wrapping_sub(b),
                        IntOp::Mul => a.wrapping_mul(b),
                    };
                    self.vec.set(vd, i, r);
                }
                self.mark_vs_dirty();
            }
            Op::Fp { op, vd, vs2, src } => {
                if !aligned(vd) || !aligned(vs2) || matches!(src, Src::V(r) if !aligned(r)) {
                    return Err(self.illegal(word, pc));
                }
                // An invalid `frm` (5..7) is illegal, as for the scalar forms (QEMU probe2
                // P1); any valid one is honoured (V-009 (i-f) lifted V-001's refusal).
                if self.frm() > fp::RMM {
                    return Err(self.illegal(word, pc));
                }
                let rm = self.frm();
                for i in start..vl {
                    let a = self.vec.get(vs2, i);
                    let b = match src {
                        Src::V(r) => self.vec.get(r, i),
                        Src::F(r) => self.f[r],
                        Src::X(_) => unreachable!("decode gives float ops no integer source"),
                    };
                    // The scalar arms' core, element by element (`V-009` (i-e)): every
                    // flag accrues exactly as on QEMU, `NX`/`OF`/`UF` included.
                    let (r, flags) = match op {
                        FpOp::Add => fp::add(fp::F64, a, b, rm),
                        FpOp::Sub => fp::sub(fp::F64, a, b, rm),
                        FpOp::Mul => fp::mul(fp::F64, a, b, rm),
                        FpOp::Div => fp::div(fp::F64, a, b, rm),
                    };
                    self.set_fflags(flags);
                    self.vec.set(vd, i, r);
                }
                self.mark_vs_dirty();
            }
            Op::RedOSum { vd, vs2, vs1 } => {
                // A reduction with vstart != 0 is the specification's own illegal case.
                if !aligned(vs2) || start != 0 || self.frm() > fp::RMM {
                    return Err(self.illegal(word, pc));
                }
                // vl = 0 writes nothing, not even element 0.
                if vl > 0 {
                    let rm = self.frm();
                    let mut acc = self.vec.get(vs1, 0);
                    for i in 0..vl {
                        let (r, flags) = fp::add(fp::F64, acc, self.vec.get(vs2, i), rm);
                        self.set_fflags(flags);
                        acc = r;
                    }
                    self.vec.set(vd, 0, acc);
                    self.mark_vs_dirty();
                }
            }
        }
        self.vec.vstart = 0;
        Ok(())
    }

    /// `vsetvli`, `vsetivli` and `vsetvl`, which differ only in where the AVL and the
    /// `vtype` come from. THE `vill` RULE IS QEMU'S, measured over every 11-bit `vtypei`
    /// (`probe_ctl.S` section V, 3,201 rows, `tests/data/v009e_vtype_qemu.tsv`): a `vtype`
    /// is illegal when any bit above 7 is set, `vlmul` is the reserved 4, SEW exceeds 64,
    /// or `VLMAX` = VLEN / SEW x LMUL is below one (e64 mf4, e32 mf8, ...). An illegal one
    /// sets `vill`, `vl` = 0 and `rd` = 0, and does not trap. Everything else is held —
    /// e32 m1 and e64 mf2 too — and `vl` = min(AVL, VLMAX).
    ///
    /// `vsetvli x0, x0` keeps `vl`, CLAMPED to the new `VLMAX` (`probe_keep.S` K1..K6). The
    /// specification leaves a VLMAX change there reserved; until (i-e) this machine set
    /// `vill` for it, QEMU clamps, and from `vill` (vl 0) it keeps 0.
    fn set_vtype(&mut self, rd: usize, avl: Avl, vtype: u64) {
        let Some(vlmax) = vlmax_of(vtype) else {
            self.vec.vtype = VILL;
            self.vec.vl = 0;
            self.x[rd] = 0;
            self.vec.vstart = 0;
            self.mark_vs_dirty();
            return;
        };
        self.vec.vtype = vtype;
        self.vec.vl = match avl {
            Avl::Value(n) => n.min(vlmax),
            Avl::Max => vlmax,
            Avl::Keep => self.vec.vl.min(vlmax),
        };
        self.x[rd] = self.vec.vl;
        self.vec.vstart = 0;
        self.mark_vs_dirty();
    }
}
