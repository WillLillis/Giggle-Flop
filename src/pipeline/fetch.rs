use anyhow::Result;
use crate::pipeline::pipeline::PipelineState;

use super::instruction::InstructionState;

#[derive(Debug, Default)]
pub struct PipelineFetch {
    pub instruction: InstructionState,
}

impl PipelineFetch {
    fn fetch(instr: &mut PipelineState) -> Result<()> {
        Ok(())
    }
}
