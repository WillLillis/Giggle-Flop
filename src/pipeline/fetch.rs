use anyhow::Result;
use crate::pipeline::pipeline::InstructionState;

#[derive(Debug, Default)]
pub struct PipeLineFetch {}

impl PipeLineFetch {
    fn fetch(instr: &mut InstructionState) -> Result<()> {
        // if no current instruction -> send load to cache with PC as address
        //      if cache returns value -> set current instruction
        // if current instruction & decode not blocked -> return instruction, increment PC
        // if no current instruction or decode blocked -> return noop/stall
        Ok(())
        // cache needs to record what process is asking for it, prevent memory/fetch conflict
        //      make other process wait, doesnt affect delay
        // memory returns line of words, cache returns requested word
    }
}