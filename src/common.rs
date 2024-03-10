pub type Cycle = usize;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum PipelineStage {
    Fetch,
    Decode,
    Execute,
    Memory,
    WriteBack,
    System, // for testing calls from outside the pipeline
}
