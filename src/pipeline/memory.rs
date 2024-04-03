use anyhow::Result;
use crate::pipeline::pipeline::PipelineState;

use super::instruction::InstructionState;

#[derive(Debug, Default)]
pub struct PipelineMemory {
    pub instruction: InstructionState
}

impl PipelineMemory {
    fn memory(instr: &mut PipelineState) -> Result<()> {
        // if noop/nonmem instruction -> do nothing
        // if load -> call cache
        // in cache -> if hit and no delay -> cache returns value
        //          -> if hit and delay/miss -> cache return wait
        //          -> if miss -> cache calls memory
        // in memory -> return value or wait
        //          -> if value -> update cache
        //          -> if whole process behind that in slides
        //          -> if store -> send data, address to cache, update accordingly
        // if value returned -> call execute with not blocked
        // else call execute with blocked
        Ok(())
        // if instruction isnt load/store -> return to write_back forwarding instruction
        // if instruction is load/store -> 
        //          if cache returns wait -> return to write_back with noop/stall
        //          if cache returns value -> put value in instruction result and return to write_back
    }
}
