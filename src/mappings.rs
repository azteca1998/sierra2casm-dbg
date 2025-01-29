use crate::{decode_instruction, Memory, StepId, Trace, ValueId};
use cairo_lang_casm::{
    instructions::InstructionBody,
    operand::{CellRef, DerefOrImmediate, Register, ResOperand},
};
use cairo_vm::vm::trace::trace_entry::RelocatedTraceEntry;
use std::{
    collections::{HashMap, HashSet},
    ops::Index,
};

#[derive(Debug)]
pub struct GraphMappings {
    step2value: HashMap<StepId, HashSet<ValueId>>,
    value2step: HashMap<ValueId, HashSet<StepId>>,
}

impl GraphMappings {
    pub fn new(memory: &Memory, trace: &Trace) -> Self {
        let mut step2value = HashMap::<StepId, HashSet<ValueId>>::new();
        let mut value2step = HashMap::<ValueId, HashSet<StepId>>::new();

        for (step, trace) in trace.iter().enumerate() {
            Self::iter_memory_references(memory, trace, |value| {
                step2value
                    .entry(StepId(step))
                    .or_default()
                    .insert(ValueId(value));
                value2step
                    .entry(ValueId(value))
                    .or_default()
                    .insert(StepId(step));
            });
        }

        Self {
            step2value,
            value2step,
        }
    }

    pub fn step2value(&self) -> &HashMap<StepId, HashSet<ValueId>> {
        &self.step2value
    }

    pub fn value2step(&self) -> &HashMap<ValueId, HashSet<StepId>> {
        &self.value2step
    }

    fn iter_memory_references(
        memory: &Memory,
        trace: &RelocatedTraceEntry,
        mut callback: impl FnMut(usize),
    ) {
        let instr = decode_instruction(memory, trace.pc);

        let mut process_cell_ref = |x: CellRef| {
            let offset = match x.register {
                Register::AP => trace.ap.wrapping_add_signed(x.offset as isize),
                Register::FP => trace.fp.wrapping_add_signed(x.offset as isize),
            };
            callback(offset);
            offset
        };

        match instr.body {
            InstructionBody::AddAp(add_ap_instruction) => match add_ap_instruction.operand {
                ResOperand::Deref(cell_ref) => todo!(),
                ResOperand::DoubleDeref(cell_ref, _) => todo!(),
                ResOperand::Immediate(_) => {}
                ResOperand::BinOp(bin_op_operand) => todo!(),
            },
            InstructionBody::AssertEq(assert_eq_instruction) => {
                process_cell_ref(assert_eq_instruction.a);
                match assert_eq_instruction.b {
                    ResOperand::Deref(cell_ref) => {
                        process_cell_ref(cell_ref);
                    }
                    ResOperand::DoubleDeref(cell_ref, _) => {
                        let offset = process_cell_ref(cell_ref);
                        callback(memory[offset].unwrap().try_into().unwrap());
                    }
                    ResOperand::Immediate(_) => {}
                    ResOperand::BinOp(bin_op_operand) => {
                        process_cell_ref(bin_op_operand.a);
                        match bin_op_operand.b {
                            DerefOrImmediate::Deref(cell_ref) => {
                                process_cell_ref(cell_ref);
                            }
                            DerefOrImmediate::Immediate(_) => {}
                        }
                    }
                }
            }
            InstructionBody::Call(call_instruction) => match call_instruction.target {
                DerefOrImmediate::Deref(cell_ref) => todo!(),
                DerefOrImmediate::Immediate(_) => {}
            },
            InstructionBody::Jnz(jnz_instruction) => {
                process_cell_ref(jnz_instruction.condition);
                match jnz_instruction.jump_offset {
                    DerefOrImmediate::Deref(cell_ref) => todo!(),
                    DerefOrImmediate::Immediate(_) => {}
                }
            }
            InstructionBody::Jump(jump_instruction) => match jump_instruction.target {
                DerefOrImmediate::Deref(cell_ref) => {
                    process_cell_ref(cell_ref);
                }
                DerefOrImmediate::Immediate(_) => {}
            },
            InstructionBody::Ret(_) => {}
        }
    }
}

impl Index<StepId> for GraphMappings {
    type Output = HashSet<ValueId>;

    fn index(&self, index: StepId) -> &Self::Output {
        self.step2value.get(&index).unwrap()
    }
}

impl Index<ValueId> for GraphMappings {
    type Output = HashSet<StepId>;

    fn index(&self, index: ValueId) -> &Self::Output {
        self.value2step.get(&index).unwrap()
    }
}
