use crate::execution::execution_state::ExecutionState;
use crate::memory::memory_system::Memory;
use crate::pipeline::pipeline::PipeLine;
use crate::register::register_system::RegisterSet;

pub struct System {
    pub clock: usize,
    pub pipeline: PipeLine,
    pub memory_system: Memory,
    pub registers: RegisterSet,
    pub execution_state: ExecutionState,
}


impl System {
    // For debugging purposes, will need to make this 
    // configurable later...
    pub fn default() -> Self {
        Self {
            clock: 0,
            pipeline: PipeLine::default(),
            memory_system: Memory::new(4, &[32, 64], &[1, 5]),
            registers: RegisterSet::new(),
            execution_state: ExecutionState::default(),
        }
    }
}
