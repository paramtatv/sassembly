use crate::t1::ast::Type;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbiLocation {
    /// Integer register (e.g. 0 to 7 corresponding to a0-a7)
    IntReg(u8),
    /// Floating point register (fa0-fa7)
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
