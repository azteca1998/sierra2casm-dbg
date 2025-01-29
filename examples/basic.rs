use bincode::de::read::SliceReader;
use cairo_vm::vm::trace::trace_entry::RelocatedTraceEntry;
use sierra2casm_dbg::{decode_instruction, Memory, Trace};
use std::fs;

fn main() {
    let mem = Memory::decode(SliceReader::new(
        &fs::read("run/out/sample.run.memory").unwrap(),
    ));

    for (i, x) in mem[0..102].iter().enumerate() {
        println!("{i} => {x:?}");
    }
    println!();

    let trace = Trace::decode(SliceReader::new(
        &fs::read("run/out/sample.run.trace").unwrap(),
    ));

    for &RelocatedTraceEntry { pc, ap, fp } in trace.iter() {
        let instr = decode_instruction(&mem, pc);
        let instr_str = instr.to_string();

        if instr_str == "jmp rel 0" {
            break;
        }

        println!("[pc={pc}, ap={ap}, fp={fp}] {instr_str}");
    }
    println!();
}
