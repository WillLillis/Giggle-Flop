use anyhow::Result;
use crate::pipeline::pipeline::InstructionState;

#[derive(Debug, Default)]
pub struct PipeLineWriteBack {}

impl PipeLineWriteBack {
    // clock calls write-back
    pub fn write_back(&self, instr: &mut InstructionState) -> Result<()> {
        // if saved instruction has result -> write to reg, update pending regs
        // if W has branch -> update PC
        // if jump subroutine -> update PC and return reg
        // if noop/stall -> do nothing
        // call memory
        Ok(())
        // save instruction from memory for next cycle
        // return to clock
        // TODO: clock increments cycles counter
        // TODO: begin new cycle
    }

}