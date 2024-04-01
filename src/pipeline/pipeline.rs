use crate::pipeline::fetch::PipeLineFetch;
use crate::pipeline::decode::PipeLineDecode;
use crate::pipeline::execute::PipeLineExecute;
use crate::pipeline::memory::PipeLineMemory;
use crate::pipeline::write_back::PipeLineWriteBack;


#[derive(Debug, Default)]
pub struct PipeLine {
    // all the pipeline stages...
    fetch: PipeLineFetch,
    decode: PipeLineDecode,
    execute: PipeLineExecute,
    memory: PipeLineMemory,
    write_back: PipeLineWriteBack,
    // for shared state between stages if necessary...
    //state: PipeLineState,
}
