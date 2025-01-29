use bincode::de::read::SliceReader;
use sierra2casm_dbg::Memory;
use starknet_types_core::felt::Felt;
use std::{fs, str::FromStr};

fn main() {
    let memory = Memory::decode(SliceReader::new(&fs::read("memory-2.bin").unwrap()));
    // let trace = Trace::decode(SliceReader::new(&fs::read(args.trace_path).unwrap()));

    let value_idx =
        memory.iter().copied().enumerate().find_map(|(idx, val)| {
            (val == Some(Felt::from_str("9962924310").unwrap())).then_some(idx)
        });

    dbg!(memory[25386].unwrap().to_string());
    dbg!(value_idx);
}
