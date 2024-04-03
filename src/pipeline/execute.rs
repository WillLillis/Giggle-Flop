use anyhow::Result;
use crate::pipeline::pipeline::PipelineState;

use super::{decode, instruction::{Instruction, InstructionState}};

#[derive(Debug, Default)]
pub struct PipelineExecute {
    pub instruction: InstructionState,
}

impl PipelineExecute {
    fn execute(instr: &mut PipelineState) -> Result<()> {
        // if noop -> do nothing
        // if ALU op -> do op
        // if jump -> get address
        // if jump subroutine -> get PC, get address
        // if branch -> check condition, set flag, calculate target address
        // if memory -> do address calculation 
        // call decode with blocked status from memory
        if instr.stall {
            // call decode?
        }
        if instr.instruction == None {
            panic!("this shouldnt happen probably")
        }
        let instruction = instr.instruction.unwrap();
        // check ops here idk how
        Ok(())
        // if memory not blocked -> return instruction object to memory with result
        // if memory blocked -> return noop/stall
        // save instruction from decode as next instruction
    }
}
