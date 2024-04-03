use crate::pipeline::pipeline::PipelineState;
use anyhow::Result;

#[derive(Debug, Default)]
pub struct PipelineDecode {}

impl PipelineDecode {
    pub fn decode(instr: Option<u32>) -> Result<()> {
        // split instruction into fields
        // if source regs not pending -> get values, create instruction object
        //      call fetch with blocked status from execute
        // if register values pending -> call fetch with blocked
        Ok(())
        // if instruction has operands and execute not blocked -> put dest register in pending, return instruction object to E
        // if instruction missing operands or execute blocked-> return noop/stall
        // save instruction from fetch as next instruction
    }
}
