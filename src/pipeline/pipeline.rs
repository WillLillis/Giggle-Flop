use std::collections::HashSet;

use crate::pipeline::decode::PipelineDecode;
use crate::pipeline::execute::PipelineExecute;
use crate::pipeline::fetch::PipelineFetch;
use crate::pipeline::memory::PipelineMemory;
use crate::pipeline::write_back::PipelineWriteBack;
use crate::pipeline::instruction::RawInstruction;
use crate::register::register_system::RegisterGroup;
use anyhow::Result;

#[derive(Debug, Default)]
pub struct PipeLine {
    // all the pipeline stages...
    fetch: PipelineFetch,
    decode: PipelineDecode,
    execute: PipelineExecute,
    memory: PipelineMemory,
    write_back: PipelineWriteBack,
    // for shared state between stages if necessary...
    state: PipelineState,
}

#[derive(Default, Debug, Clone)]
pub struct PipelineState {
    // pub instruction: Option<Instruction>,
    // pub value: Option<String>,
    // pub stall: bool,
    pending_regs: HashSet<(RegisterGroup, usize)>,
}

impl PipelineState {
    fn new() -> Self {
        PipelineState {
            // instruction: None,
            // value: None,
            // stall: false,
            pending_regs: HashSet::new(),
        }
    }
}

// TODO: Make a bad ass flow chart, see what information is flowing where, and then code it up

impl PipeLine {
}

