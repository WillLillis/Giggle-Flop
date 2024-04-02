use crate::pipeline::decode::PipeLineDecode;
use crate::pipeline::execute::PipeLineExecute;
use crate::pipeline::fetch::PipeLineFetch;
use crate::pipeline::memory::PipeLineMemory;
use crate::pipeline::write_back::PipeLineWriteBack;
use crate::pipeline::instruction::RawInstruction;
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

// TODO: Make a bad ass flow chart, see what information is flowing where, and then code it up

impl PipeLine {
    pub fn start(&mut self) -> Result<()> {
        self.write_back()
    }

    fn fetch(&mut self) -> Option<RawInstruction> {
        todo!()
    }

    fn decode(&mut self) -> Result<()> {
    
        if let Some(instr) = self.fetch() {
            // do the stuff???
        }

        todo!()
    }

    fn execute(&mut self) -> Result<()> {
        todo!()
    }

    fn memory(&mut self) -> Result<()> {
        todo!()
    }

    fn write_back(&mut self) -> Result<()> {
        todo!()
    }
}

