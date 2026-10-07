use crate::t1::ast::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbiLocation {
    /// Integer register (e.g. 0 to 7 corresponding to a0-a7)
    IntReg(u8),
    /// Float argument register — the index WITHIN `fa0-fa7`, not a hardware
    /// number. `FloatReg(0)` is `fa0`, which is `f10`; see
    /// [`FloatRole::hardware`]. It is the same shape as `IntReg`, whose `0` is
    /// `a0` and `x10`.
    FloatReg(u8),
    /// Passed on stack at the given byte offset
    Stack(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentAbi {
    pub locations: Vec<AbiLocation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureAbi {
    pub args: Vec<ArgumentAbi>,
    pub ret: Option<ArgumentAbi>,
}

/// `V-004` — THE FLOAT TYPE NAMES, FROM THE FROZEN GRAMMAR.
///
/// `spec/grammar-t1.ebnf:272` is `float_type = "प" , ( "३२" | "६४" ) ;` and
/// `:1565` records that both are frozen in `type_name` and written ZERO times
/// in the corpus. So these two names are the whole of the float type space,
/// read off the grammar rather than coined here, and NOTHING IN THE RUST T1
/// FRONT END PRODUCES EITHER YET: `ast::Type::Primitive` carries whatever the
/// parser read, and no corpus source writes a `प` type. The arm below is
/// therefore reached today only by a caller that names one itself, which is
/// what the tests do — the same arrangement `regalloc`'s `RegClass::Float`
/// stands in, for the same reason.
pub const FLOAT_TYPES: [&str; 2] = ["प३२", "प६४"];

/// Whether a primitive type name is a float, and so which register file an
/// argument of that type is passed in.
#[must_use]
pub fn is_float_type(name: &str) -> bool {
    FLOAT_TYPES.contains(&name)
}

/// `V-004` — THE THREE ROLES OF THE FLOAT REGISTER FILE.
///
/// The float file is not a flat `f0..f31`. It is split exactly as the integer
/// file is, into scratch, callee-saved and arguments, and the hardware numbers
/// of each role ARE NOT CONTIGUOUS — `ft` is `f0-f7` and then `f28-f31`, `fs`
/// is `f8-f9` and then `f18-f27`. That discontinuity is the whole hazard this
/// type exists to contain: `spec/registers-riscv64.tsv` already shows it on the
/// integer side (`क्षणिक३` is `x28`, `स्थिर२` is `x18`), and a role index used
/// as a hardware number is a WRONG REGISTER rather than a failure.
///
/// The root is `प्लव` and not `भिन्न` (owner ruling Q1, 2026-09-29). The role
/// WORDS are not a fresh decision — they are the integer file's own, which
/// `spec/registers-riscv64.tsv` fixes as `क्षणिक` / `स्थिर` / `अर्थ`; the float
/// names are those under the `प्लव` root, so `प्लवार्थ०` is to `अर्थ०` what
/// `fa0` is to `a0`. ASSUMED, because no ruling names the role words and
/// mirroring the file that already has them is the only choice that does not
/// coin a second vocabulary for one concept.
///
/// `role_name` IS NOT AN ASSEMBLER NAME. The lexicon
/// (`spec/registers-riscv64.tsv`) spells every float register `प्लव<hardware>`
/// and knows no role spelling at all, so emitted text must come from
/// [`FloatRole::register_name`]; `प्लवार्थ०` would not assemble.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatRole {
    /// `ft0-ft11`, scratch — caller-saved, not preserved across a call.
    Temp,
    /// `fs0-fs11`, callee-saved — preserved across a call.
    Saved,
    /// `fa0-fa7`, arguments and the return value.
    Arg,
}

const FLOAT_TEMPS: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 28, 29, 30, 31];
const FLOAT_SAVED: [u8; 12] = [8, 9, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27];
const FLOAT_ARGS: [u8; 8] = [10, 11, 12, 13, 14, 15, 16, 17];

impl FloatRole {
    /// Every role, so a caller can state a property of the whole file without
    /// having to list them and miss one.
    pub const ALL: [FloatRole; 3] = [FloatRole::Temp, FloatRole::Saved, FloatRole::Arg];

    /// The hardware numbers this role covers, in role-index order.
    #[must_use]
    pub fn hardware_numbers(self) -> &'static [u8] {
        match self {
            FloatRole::Temp => &FLOAT_TEMPS,
            FloatRole::Saved => &FLOAT_SAVED,
            FloatRole::Arg => &FLOAT_ARGS,
        }
    }

    /// How many registers the role has: twelve, twelve and eight.
    #[must_use]
    pub fn count(self) -> u8 {
        self.hardware_numbers().len() as u8
    }

    /// The `प्लव`-rooted stem the role's names are built on.
    #[must_use]
    pub fn root(self) -> &'static str {
        match self {
            // प्लव + क्षणिक / स्थिर / अर्थ, the integer file's own role words.
            // `प्लवार्थ` is the sandhi of `प्लव` and `अर्थ`.
            FloatRole::Temp => "प्लवक्षणिक",
            FloatRole::Saved => "प्लवस्थिर",
            FloatRole::Arg => "प्लवार्थ",
        }
    }

    /// The hardware register a role index names, or `None` past the role's end.
    ///
    /// REFUSES rather than wraps or saturates: `Arg` has eight, and a ninth
    /// float argument belongs on the stack, not in `f18`.
    #[must_use]
    pub fn hardware(self, n: u8) -> Option<u8> {
        self.hardware_numbers().get(n as usize).copied()
    }

    /// The role name — `प्लवार्थ०`, for a diagnostic. NOT an assembler name.
    #[must_use]
    pub fn role_name(self, n: u8) -> Option<String> {
        self.hardware(n).map(|_| {
            format!(
                "{}{}",
                self.root(),
                crate::t1::riscv64::devanagari(i64::from(n))
            )
        })
    }

    /// The name the assembler accepts — `प्लव<hardware>`, the only float
    /// spelling in the lexicon.
    #[must_use]
    pub fn register_name(self, n: u8) -> Option<String> {
        self.hardware(n)
            .map(|hw| format!("प्लव{}", crate::t1::riscv64::devanagari(i64::from(hw))))
    }
}

pub struct AbiState {
    pub int_regs_used: u8,
    pub float_regs_used: u8,
    pub stack_offset: usize,
}

/// `W-274`: `new()` takes no arguments, so `Default` is the same
/// constructor under the name the language expects. Written rather than
/// allowed, because `clippy::new_without_default` is asking for an
/// interface and not for silence.
impl Default for AbiState {
    fn default() -> Self {
        Self::new()
    }
}

impl AbiState {
    pub fn new() -> Self {
        Self {
            int_regs_used: 0,
            float_regs_used: 0,
            stack_offset: 0,
        }
    }

    pub fn allocate_int(&mut self) -> Option<u8> {
        if self.int_regs_used < 8 {
            let reg = self.int_regs_used;
            self.int_regs_used += 1;
            Some(reg)
        } else {
            None
        }
    }

    /// The next `fa` index, or `None` once all eight are spoken for.
    ///
    /// `V-004` — IT COUNTS ITS OWN FILE. The integer and float argument
    /// registers are allocated INDEPENDENTLY under this ABI: eight integer
    /// arguments and eight float arguments all travel in registers and none
    /// touches the stack, which is the property that fails if this shares
    /// `int_regs_used`. `float_regs_used` was carried as a field and never
    /// incremented until now.
    pub fn allocate_float(&mut self) -> Option<u8> {
        if self.float_regs_used < FloatRole::Arg.count() {
            let reg = self.float_regs_used;
            self.float_regs_used += 1;
            Some(reg)
        } else {
            None
        }
    }

    pub fn allocate_stack(&mut self, size: usize) -> usize {
        let align = 8;
        let offset = (self.stack_offset + align - 1) & !(align - 1);
        self.stack_offset = offset + size;
        offset
    }
}

pub fn compute_signature(args: &[Type], ret: Option<&Type>) -> SignatureAbi {
    let mut state = AbiState::new();
    let mut arg_abis = Vec::new();

    for ty in args {
        arg_abis.push(compute_arg_abi(ty, &mut state));
    }

    let ret_abi = if let Some(ret_ty) = ret {
        let mut ret_state = AbiState::new();
        Some(compute_arg_abi(ret_ty, &mut ret_state))
    } else {
        None
    };

    SignatureAbi {
        args: arg_abis,
        ret: ret_abi,
    }
}

fn compute_arg_abi(ty: &Type, state: &mut AbiState) -> ArgumentAbi {
    match ty {
        // ADR-0026 puts the array in the SAME arm as the slice deliberately:
        // both are passed as one address-sized word, so an array crossing a
        // call boundary costs no more than the slice it can decay to. Copying
        // `capacity` elements into registers is what a by-value aggregate would
        // do and is not what this ABI does for any other constructor here.
        // `V-004`: a float primitive is passed in `fa0-fa7` and nowhere else
        // in the integer file. The stack is the fallback for BOTH files, and a
        // float that lands there lands at an offset from the SAME cursor — one
        // frame, two register files.
        Type::Primitive(name) if is_float_type(name) => {
            if let Some(reg) = state.allocate_float() {
                ArgumentAbi {
                    locations: vec![AbiLocation::FloatReg(reg)],
                }
            } else {
                let offset = state.allocate_stack(8);
                ArgumentAbi {
                    locations: vec![AbiLocation::Stack(offset)],
                }
            }
        }
        Type::Primitive(_)
        | Type::Pointer(_)
        | Type::Slice(_)
        | Type::Array { .. }
        | Type::Optional(_)
        | Type::ErrorUnion(_) => {
            if let Some(reg) = state.allocate_int() {
                ArgumentAbi {
                    locations: vec![AbiLocation::IntReg(reg)],
                }
            } else {
                let offset = state.allocate_stack(8);
                ArgumentAbi {
                    locations: vec![AbiLocation::Stack(offset)],
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_abi_simple_args() {
        let args = vec![
            Type::Primitive("अ३२".to_string()),
            Type::Primitive("अ३२".to_string()),
            Type::Primitive("बूल".to_string()),
        ];
        let sig = compute_signature(&args, Some(&Type::Primitive("अ३२".to_string())));

        assert_eq!(sig.args.len(), 3);
        assert_eq!(sig.args[0].locations, vec![AbiLocation::IntReg(0)]);
        assert_eq!(sig.args[1].locations, vec![AbiLocation::IntReg(1)]);
        assert_eq!(sig.args[2].locations, vec![AbiLocation::IntReg(2)]);

        assert!(sig.ret.is_some());
        assert_eq!(sig.ret.unwrap().locations, vec![AbiLocation::IntReg(0)]);
    }

    /// `V-004` — THE THREE ROLES PARTITION `f0..f31`, AND THE NUMBERS ARE
    /// NOT CONTIGUOUS.
    ///
    /// Twelve, twelve and eight is thirty-two, so the three roles must cover
    /// every float register EXACTLY ONCE. That is the check that catches a
    /// transcription slip in a table whose rows jump — `ft7` to `ft8` is `f7`
    /// to `f28`, `fs1` to `fs2` is `f9` to `f18` — and a slip there is a wrong
    /// register, not a crash: a callee would save `f17` thinking it callee-saved
    /// and clobber its caller's second float argument.
    ///
    /// Stated as coverage-and-disjointness rather than as a copy of the table,
    /// because asserting the table against itself proves nothing.
    #[test]
    fn v004_the_float_roles_partition_the_file_exactly_once() {
        let mut seen: Vec<u8> = Vec::new();
        for role in FloatRole::ALL {
            for n in 0..role.count() {
                let hw = role.hardware(n).expect("in range");
                assert!(
                    hw < 32,
                    "{:?} index {n} gives f{hw}, which is not a float register",
                    role
                );
                assert!(
                    !seen.contains(&hw),
                    "f{hw} belongs to two roles; {:?} index {n} claims it again",
                    role
                );
                seen.push(hw);
            }
        }
        seen.sort_unstable();
        assert_eq!(
            seen,
            (0u8..32).collect::<Vec<u8>>(),
            "the three roles must cover f0..f31 with nothing left over"
        );
        assert_eq!(FloatRole::Temp.count(), 12, "ft0-ft11");
        assert_eq!(FloatRole::Saved.count(), 12, "fs0-fs11");
        assert_eq!(FloatRole::Arg.count(), 8, "fa0-fa7");

        // The role index is NOT the hardware number, which is the reason this
        // type exists rather than a bare `u8`.
        assert_eq!(FloatRole::Arg.hardware(0), Some(10), "fa0 is f10");
        assert_eq!(FloatRole::Temp.hardware(8), Some(28), "ft8 is f28");
        assert_eq!(FloatRole::Saved.hardware(2), Some(18), "fs2 is f18");
    }

    /// `V-004` — THE CASES THAT MUST BE REFUSED.
    ///
    /// A role index past the role's end has no register, and `None` is the
    /// only answer that is not a lie: saturating would hand out `f31` for
    /// `fa8` and wrapping would hand out `f10`. Both assemble.
    #[test]
    fn v004_a_role_index_past_the_end_has_no_register() {
        assert_eq!(FloatRole::Arg.hardware(8), None, "there is no fa8");
        assert_eq!(FloatRole::Temp.hardware(12), None, "there is no ft12");
        assert_eq!(FloatRole::Saved.hardware(12), None, "there is no fs12");
        assert_eq!(FloatRole::Arg.role_name(8), None);
        assert_eq!(FloatRole::Arg.register_name(8), None);
        assert_eq!(FloatRole::Arg.hardware(255), None);

        // And the ninth float argument goes to the stack rather than to a
        // ninth `fa` that does not exist.
        let args: Vec<Type> = (0..9).map(|_| Type::Primitive("प६४".to_string())).collect();
        let sig = compute_signature(&args, None);
        assert_eq!(sig.args[7].locations, vec![AbiLocation::FloatReg(7)]);
        assert_eq!(
            sig.args[8].locations,
            vec![AbiLocation::Stack(0)],
            "the ninth float argument travels on the stack, not in a ninth fa"
        );
    }

    /// `V-004` — THE EMITTED NAME IS THE LEXICON'S, NOT THE ROLE'S.
    ///
    /// `spec/registers-riscv64.tsv` spells every float register `प्लव<hardware>`
    /// and carries no role spelling, so `प्लवार्थ०` is a diagnostic name only.
    /// Emitting it would produce text the assembler refuses, and this test is
    /// what says the two names are different on purpose.
    #[test]
    fn v004_the_role_name_and_the_assembler_name_are_not_the_same_name() {
        assert_eq!(FloatRole::Arg.role_name(0).unwrap(), "प्लवार्थ०");
        assert_eq!(FloatRole::Arg.register_name(0).unwrap(), "प्लव१०");
        assert_eq!(FloatRole::Temp.role_name(8).unwrap(), "प्लवक्षणिक८");
        assert_eq!(FloatRole::Temp.register_name(8).unwrap(), "प्लव२८");
        assert_eq!(FloatRole::Saved.role_name(2).unwrap(), "प्लवस्थिर२");
        assert_eq!(FloatRole::Saved.register_name(2).unwrap(), "प्लव१८");

        // Every assembler name the roles can produce is a name the register
        // table actually holds — the role tables cannot invent a register.
        for role in FloatRole::ALL {
            for n in 0..role.count() {
                let name = role.register_name(n).expect("in range");
                assert_eq!(
                    crate::encode::register(&name),
                    Some((u32::from(role.hardware(n).unwrap()), true)),
                    "{name} is not the float register {:?} index {n} means",
                    role
                );
                assert_ne!(
                    role.role_name(n).unwrap(),
                    name,
                    "the role name is not the lexicon name"
                );
                assert_eq!(
                    crate::encode::register(&role.role_name(n).unwrap()),
                    None,
                    "a role name must not be in the lexicon; emitted text would \
                     be ambiguous if it were"
                );
            }
        }
    }

    /// `V-004` — THE TWO ARGUMENT FILES ARE COUNTED SEPARATELY.
    ///
    /// Sixteen arguments, eight of each type, interleaved so that neither
    /// counter can be reached by accident: all sixteen are in registers and
    /// the stack is untouched. A shared counter puts the ninth argument —
    /// whichever type it is — on the stack, and interleaving is what makes
    /// that visible rather than leaving it to depend on argument order.
    #[test]
    fn v004_eight_integer_and_eight_float_arguments_all_travel_in_registers() {
        let mut args = Vec::new();
        for _ in 0..8 {
            args.push(Type::Primitive("अ३२".to_string()));
            args.push(Type::Primitive("प३२".to_string()));
        }
        let sig = compute_signature(&args, None);
        assert_eq!(sig.args.len(), 16);
        for i in 0..8u8 {
            assert_eq!(
                sig.args[2 * i as usize].locations,
                vec![AbiLocation::IntReg(i)],
                "integer argument {i}"
            );
            assert_eq!(
                sig.args[2 * i as usize + 1].locations,
                vec![AbiLocation::FloatReg(i)],
                "float argument {i}"
            );
        }
        assert!(
            !sig.args
                .iter()
                .any(|a| matches!(a.locations[0], AbiLocation::Stack(_))),
            "nothing goes to the stack: the two files are counted separately"
        );

        // Both float type names take the float path, and nothing else does —
        // `अ३२` is the near miss that must not.
        for name in FLOAT_TYPES {
            assert!(is_float_type(name));
        }
        for name in ["अ३२", "अ६४", "बूल", "अक्षरम्", "प", "प१२८", "प्लव"]
        {
            assert!(!is_float_type(name), "{name} is not a float type");
        }
    }

    /// `V-004` — A FLOAT RETURN VALUE IS `fa0`, NOT `a0`.
    #[test]
    fn v004_a_float_return_value_is_in_the_float_file() {
        let sig = compute_signature(&[], Some(&Type::Primitive("प६४".to_string())));
        assert_eq!(
            sig.ret.unwrap().locations,
            vec![AbiLocation::FloatReg(0)],
            "fa0"
        );
        let sig = compute_signature(&[], Some(&Type::Primitive("अ६४".to_string())));
        assert_eq!(
            sig.ret.unwrap().locations,
            vec![AbiLocation::IntReg(0)],
            "a0"
        );
    }

    #[test]
    fn test_compute_abi_spill_to_stack() {
        let mut args = vec![];
        for _ in 0..9 {
            args.push(Type::Primitive("अ३२".to_string()));
        }

        let sig = compute_signature(&args, None);
        assert_eq!(sig.args.len(), 9);
        assert_eq!(sig.args[7].locations, vec![AbiLocation::IntReg(7)]);
        assert_eq!(sig.args[8].locations, vec![AbiLocation::Stack(0)]);
    }
}
