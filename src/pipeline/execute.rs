use anyhow::Result;
use crate::pipeline::pipeline::InstructionState;

#[derive(Debug, Default)]
pub struct PipeLineExecute {}

impl PipeLineExecute {
    fn execute(instr: &mut InstructionState) -> Result<()> {
        // if noop -> do nothing
        // if ALU op -> do op
        // if jump -> get address
        // if jump subrouting -> get PC, get address
        // if branch -> check condition, set flag, calculate target address
        // if memory -> do address calculation 
        // call decode with blocked status from memory
        Ok(())
        // if memory not blocked -> return instruction object to memory with result
        // if memory blocked -> return noop/stall
        // save instruction from decode as next instruction
    }
}