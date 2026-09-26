use crate::t1::ir::*;
use crate::t1::regalloc::{AllocationMap, Location};

pub struct X86_64Emitter {
    pub output: String,
}

/// `W-274`: `new()` takes no arguments, so `Default` is the same
/// constructor under the name the language expects. Written rather than
/// allowed, because `clippy::new_without_default` is asking for an
/// interface and not for silence.
impl Default for X86_64Emitter {
    fn default() -> Self {
        Self::new()
    }
}

impl X86_64Emitter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    pub fn emit_function(&mut self, func: &Function, alloc: &AllocationMap) {
        // Just mock the name for now, as emit.rs does
        self.output.push_str(
            "  .globl main
",
        );
        self.output.push_str(
            "main:
",
        );

        // Prologue
        self.output.push_str(
            "  pushq %rbp
",
        );
        self.output.push_str(
            "  movq %rsp, %rbp
",
        );

        let mut sorted_blocks: Vec<_> = func.blocks.keys().copied().collect();
        sorted_blocks.sort_by_key(|b| b.0);

        for block_id in sorted_blocks {
            let block = &func.blocks[&block_id];

            for (vid, inst) in &block.insts {
                let dest_loc = if let Some(loc) = alloc.locations.get(vid) {
                    loc
                } else {
                    continue;
                };

                match inst {
                    Instruction::ConstInt(c) => {
                        let rname = self.format_location(dest_loc);
                        self.output.push_str(&format!(
                            "  movq ${}, {}
",
                            c, rname
                        ));
                    }
                    Instruction::Add(v1, v2) => {
                        let r1 = &alloc.locations[v1];
                        let r2 = &alloc.locations[v2];
                        let rn1 = self.format_location(r1);
                        let rn2 = self.format_location(r2);
                        let rn_out = self.format_location(dest_loc);

                        if rn_out != rn1 {
                            self.output.push_str(&format!(
                                "  movq {}, {}
",
                                rn1, rn_out
                            ));
                        }
                        self.output.push_str(&format!(
                            "  addq {}, {}
",
                            rn2, rn_out
                        ));
                    }
                    _ => {}
                }
            }
        }

        // Epilogue
        self.output.push_str(
            "  popq %rbp
",
        );
        self.output.push_str(
            "  ret
",
        );
    }

    fn format_location(&self, loc: &Location) -> String {
        match loc {
            Location::Register(id) => format!("%{}", self.reg_name((*id).into())),
            Location::Spill(slot) => format!("-{}(%rbp)", (slot + 1) * 8),
        }
    }

    // A simple mapping of registers to AMD64 general purpose registers
    fn reg_name(&self, reg_num: usize) -> &'static str {
        match reg_num {
            0 => "rax",
            1 => "rbx",
            2 => "rcx",
            3 => "rdx",
            4 => "rsi",
            5 => "rdi",
            6 => "r8",
            7 => "r9",
            8 => "r10",
            9 => "r11",
            10 => "r12",
            11 => "r13",
            12 => "r14",
            13 => "r15",
            _ => panic!("out of registers"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::t1::ast::SymbolId;
    use std::collections::HashMap;

    #[test]
    fn test_x86_64_emission() {
        let v1 = ValueId(1);
        let v2 = ValueId(2);
        let v3 = ValueId(3);

        let block = Block {
            id: BlockId(0),
            insts: vec![
                (v1, Instruction::ConstInt(10)),
                (v2, Instruction::ConstInt(20)),
                (v3, Instruction::Add(v1, v2)),
            ],
            terminator: Some(Terminator::Return(Some(v3))),
        };

        let mut blocks = HashMap::new();
        blocks.insert(BlockId(0), block);

        let func = Function {
            name: SymbolId(0),
            blocks,
            entry_block: BlockId(0),
        };

        let mut locations = HashMap::new();
        locations.insert(v1, Location::Register(0));
        locations.insert(v2, Location::Register(1));
        locations.insert(v3, Location::Register(0));

        let alloc = AllocationMap {
            locations,
            num_spills: 0,
        };

        let mut emitter = X86_64Emitter::new();
        emitter.emit_function(&func, &alloc);

        assert!(emitter.output.contains("movq $10, %rax"));
        assert!(emitter.output.contains("movq $20, %rbx"));
        assert!(emitter.output.contains("addq %rbx, %rax"));
    }
}
