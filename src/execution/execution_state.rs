#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Default)]
pub enum ExecutionMode {
    #[default]
    Pause,
    Play,
}

/// Holds information regarding how we're executing the given system that
/// isn't a necessary part of the system (e.g. single step vs. normal execution,
/// other debug information, etc.)
#[derive(Debug, Clone, Copy, Default)]
pub struct ExecutionState {
    pub mode: ExecutionMode,
}
impl ExecutionState {
    pub fn new(mode: ExecutionMode) -> Self {
        Self { mode }
    }
}
