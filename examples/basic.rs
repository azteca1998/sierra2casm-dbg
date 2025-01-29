use bincode::de::read::SliceReader;
use cairo_lang_casm::{
    instructions::InstructionBody,
    operand::{CellRef, DerefOrImmediate, Register, ResOperand},
};
use cairo_vm::vm::trace::trace_entry::RelocatedTraceEntry;
use sierra2casm_dbg::{decode_instruction, Memory, Trace};
use starknet_types_core::felt::Felt;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    ops::Index,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
struct StepId(usize);
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
struct ValueId(usize);

#[derive(Debug)]
struct GraphMappings {
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum NodeId {
    Step(StepId),
    Value(ValueId),
}

trait QueueContainer<T> {
    fn new(init: T) -> Self;
    fn current_step(&self) -> usize;

    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>;
    fn pop(&mut self) -> Option<T>;
}

struct BfsQueue<T>(VecDeque<T>, usize);
impl<T> QueueContainer<T> for BfsQueue<T> {
    fn new(init: T) -> Self {
        Self(VecDeque::from([init]), 0)
    }

    fn current_step(&self) -> usize {
        self.1
    }

    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.0.extend(iter);
    }

    fn pop(&mut self) -> Option<T> {
        self.0.pop_front().inspect(|_| self.1 += 1)
    }
}

struct DfsQueue<T>(Vec<T>, usize);
impl<T> QueueContainer<T> for DfsQueue<T> {
    fn new(init: T) -> Self {
        Self(Vec::from([init]), 0)
    }

    fn current_step(&self) -> usize {
        self.1
    }

    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.0.extend(iter);
    }

    fn pop(&mut self) -> Option<T> {
        self.0.pop().inspect(|_| self.1 += 1)
    }
}

fn run_search_algorithm<Q>(
    memory: &Memory,
    mappings: &GraphMappings,
    source: ValueId,
    target: ValueId,
) -> (Q, Option<Vec<NodeId>>)
where
    Q: QueueContainer<Vec<NodeId>>,
{
    let mut visited = HashSet::from([NodeId::Value(source)]);
    let mut queue = Q::new(vec![NodeId::Value(source)]);
    while let Some(path) = queue.pop() {
        if *path.last().unwrap() == NodeId::Value(target) {
            return (queue, Some(path));
        }

        match *path.last().unwrap() {
            NodeId::Step(id) => {
                queue.extend(
                    mappings[id]
                        .iter()
                        .copied()
                        .filter(|x| visited.insert(NodeId::Value(*x)))
                        .map(|x| {
                            let mut new_path = path.clone();
                            new_path.push(NodeId::Value(x));
                            new_path
                        }),
                );
            }
            NodeId::Value(id) => {
                queue.extend(
                    mappings[id]
                        .iter()
                        .copied()
                        .filter(|x| visited.insert(NodeId::Step(*x)))
                        .map(|x| {
                            let mut new_path = path.clone();
                            new_path.push(NodeId::Step(x));
                            new_path
                        }),
                );
            }
        }
    }

    (queue, None)
}

fn main() {
    const MEMORY_PATH: &str = "memory-2.bin";
    const TRACE_PATH: &str = "trace-2.bin";
    let initial_value: Felt = 9997055710u64.into();
    let final_value: Felt = 9919498708u64.into();

    // const MEMORY_PATH: &str = "run/out/sample.run.memory";
    // const TRACE_PATH: &str = "run/out/sample.run.trace";
    // let initial_value: Felt = Felt::from_str("1234567890123456789012345678901234567890").unwrap();
    // let final_value: Felt = Felt::from_str(
    //     "2847638865979330485095276698780488300980749455614881795634162556004069877332",
    // )
    // .unwrap();

    //
    // Load data from disk.
    //
    println!("Loading memory and trace.");
    let memory = Memory::decode(SliceReader::new(&fs::read(MEMORY_PATH).unwrap()));
    let trace = Trace::decode(SliceReader::new(&fs::read(TRACE_PATH).unwrap()));

    //
    // Generate graph mappings.
    //
    println!("Generating graph mappings.");
    let mappings = GraphMappings::new(&memory, &trace);

    //
    // Find initial and final values.
    //
    println!("Finding initial and final values within the data.");
    let source_value = mappings
        .value2step
        .keys()
        .copied()
        .filter(|x| memory[x.0].unwrap() == initial_value)
        .min()
        .unwrap();
    let target_value = mappings
        .value2step
        .keys()
        .copied()
        .filter(|x| memory[x.0].unwrap() == final_value)
        .max()
        .unwrap();

    println!();

    //
    // Find a path between the source and target nodes.
    //
    println!("Search algorithm started.");
    let (queue, path) =
        run_search_algorithm::<DfsQueue<_>>(&memory, &mappings, source_value, target_value);
    println!(
        "Search algorithm finished in {} steps.",
        queue.current_step()
    );
    println!();

    match path {
        None => println!("No connecting path found."),
        Some(path) => {
            println!("Connecting path:");
            for id in path {
                match id {
                    NodeId::Step(offset) => {
                        println!("{}", decode_instruction(&memory, trace[offset.0].pc));
                        println!("    {:?}", trace[offset.0]);
                    }
                    NodeId::Value(offset) => {
                        println!("  [{}] = {}", offset.0, memory[offset.0].unwrap());
                        println!();
                    }
                }
            }
        }
    }
}
