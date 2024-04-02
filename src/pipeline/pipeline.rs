use crate::pipeline::fetch::PipeLineFetch;
use crate::pipeline::decode::PipeLineDecode;
use crate::pipeline::execute::PipeLineExecute;
use crate::pipeline::memory::PipeLineMemory;
use crate::pipeline::write_back::PipeLineWriteBack;
use anyhow::Result;

#[derive(Debug, Default)]
pub struct PipeLine {
    // all the pipeline stages...
    fetch: PipeLineFetch,
    decode: PipeLineDecode,
    execute: PipeLineExecute,
    memory: PipeLineMemory,
    write_back: PipeLineWriteBack,
    // for shared state between stages if necessary...
    state: InstructionState,
}

#[derive(Debug, Clone, Eq, PartialEq, Copy)]
enum OpCode {
    Add,
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct InstructionState {
    opcode: Option<OpCode>,
    value: Option<String>,
    stall: bool,
}

impl InstructionState {
    fn new() -> Self {
        InstructionState {
            opcode: None,
            value: None,
            stall: false,
        }
    }
}

impl PipeLine {
    fn start(&mut self) -> Result<()> {
        return self.write_back.write_back(&mut self.state);
    }
}