use std::default;

pub type Cycle = usize;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Default)]
pub enum PipelineStage {
    Fetch,
    Decode,
    Execute,
    Memory,
    WriteBack,
    #[default]
    System, // for testing calls from outside the pipeline
}
