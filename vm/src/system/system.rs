use std::collections::HashSet;
use std::path::PathBuf;

use log::{error, info};

use crate::instruction::instruction::{decode_raw_instr, Instruction, RawInstruction};
use crate::memory::memory_system::{
    LoadRequest, LoadResponse, MemRequest, MemResponse, MemType, Memory, StoreRequest,
    MEM_BLOCK_WIDTH,
};
use crate::register::register_system::{
    get_comparison_flags, FlagIndex, Register, RegisterGroup, RegisterSet, FLAG_COUNT, RET_REG,
};

use crate::memory::memory_system::MemBlock;

pub type Cycle = usize;

/// Messages to ne passed back from the pipeline_run() and run_no_pipeline()
/// functions to indicate if the system should halt execution, or if some other
/// important state changes occurred
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Default)]
pub enum SystemMessage {
    Halt,
    #[default]
    InstructionCompleted,
    InstructionPending,
    // fill in others as needed
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Default, Hash)]
pub enum PipelineStage {
    Fetch,
    Decode,
    Execute,
    Memory,
    WriteBack,
    #[default]
    System, // for testing calls from outside the pipeline
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PipelineInstructionResult {
    Register {
        reg_group: RegisterGroup,
        dest_reg: usize,
        data: MemBlock,
    },
    Branch {
        new_pc: u32,
    },
    JumpSubRoutine {
        new_pc: u32,
        ret_reg_val: u32, // return register should be by convention, this is just the address
                          // value to store in it
    },
    Flag {
        flags: [Option<bool>; FLAG_COUNT],
    },
    Empty, // indicate an operation was completed, but there's no data to show for it (e.g.
           // a store to memory)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FetchState {
    pub src_addr: Option<usize>,
    pub raw_instr: Option<u32>,
}

pub struct System {
    pub clock: usize,
    pub memory_system: Memory,
    pub registers: RegisterSet,
    should_use_pipeline: bool,
    // Pipeline v
    pub fetch: FetchState,
    pub decode: PipelineStageStatus,
    pub execute: PipelineStageStatus,
    pub memory: PipelineStageStatus,
    pub writeback: PipelineStageStatus,
    pub pending_reg: HashSet<(RegisterGroup, usize)>,
}

impl System {
    // For debugging purposes, will need to make this
    // configurable later...
    pub fn default() -> Self {
        Self {
            clock: 0,
            pending_reg: HashSet::new(),
            memory_system: Memory::new(4, &[32, 256], &[1, 2]),
            should_use_pipeline: true,
            registers: RegisterSet::new(),
            fetch: FetchState::default(),
            decode: PipelineStageStatus::Noop,
            execute: PipelineStageStatus::Noop,
            memory: PipelineStageStatus::Noop,
            writeback: PipelineStageStatus::Noop,
        }
    }

    pub fn reset(&mut self) {
        let n_levels = self.memory_system.num_levels();

        let mut capacities = Vec::new();
        let mut latencies = Vec::new();
        for level in 0..n_levels {
            capacities.push(self.memory_system.num_lines(level).unwrap());
            latencies.push(self.memory_system.get_latency(level).unwrap());
        }

        self.clock = 0;
        self.pending_reg.clear();
        self.memory_system = Memory::new(4, &capacities, &latencies);
        self.registers = RegisterSet::new();
        self.fetch = FetchState::default();
        self.decode = PipelineStageStatus::Noop;
        self.execute = PipelineStageStatus::Noop;
        self.memory = PipelineStageStatus::Noop;
        self.writeback = PipelineStageStatus::Noop;
    }

    // TODO: Improve this by utilizing the drop file event
    pub fn load_program(&mut self, path: PathBuf) {
        info!("Loading program file {:?}", path);
        let program = std::fs::read(&path).unwrap();
        info!("Loaded: {:?}", program);

        // check the length
        let program_len = program.len() * 8;
        let mem_len = self.memory_system.main_capacity().unwrap();
        if program_len > mem_len {
            error!(
                "Program {} is too large to fit in main memory: {program_len} > {mem_len}",
                path.display()
            );
            panic!("Program too large");
        }

        // TODO: Perform some sanitation here...
        for (i, instr) in program.windows(4).step_by(4).enumerate() {
            if instr.len() != 4 {
                error!("Program length isn't an integer multiple of 32 bits");
                panic!("Invalid program length");
            }
            let bytes = [instr[0], instr[1], instr[2], instr[3]];
            let data = MemBlock::Unsigned32(u32::from_be_bytes(bytes));
            self.memory_system.force_store(i * MEM_BLOCK_WIDTH, data);
        }
        info!("Done");
    }

    fn run_no_pipeline(&mut self) -> SystemMessage {
        info!("NoPipeline: Starting a non-pipelined cycle");
        // NOTE: just going to make this an absolutely disgusting monolith of a function
        // for now, will clean up "later"

        // fetch instruction from memory
        let req = MemRequest::Load(LoadRequest {
            issuer: PipelineStage::Fetch,
            address: self.registers.program_counter as usize,
            width: MemType::Unsigned32,
        });
        // need to store this across calls, otherwise we can block
        let raw_instr = if let Some(raw) = self.fetch.raw_instr {
            raw
        } else {
            let raw_resp = self.memory_system.request(&req);
            if let Ok(MemResponse::Load(LoadResponse { data })) = raw_resp {
                let block_data = data
                    .get_contents(self.registers.program_counter as usize)
                    .unwrap();
                if let Some(raw) = block_data.get_unsigned() {
                    self.fetch.raw_instr = Some(raw);
                    raw
                } else {
                    info!(
                        "NoPipeline: Got unnexpected data for instruction fetch, passing on a noop"
                    );
                    todo!()
                }
            } else {
                info!(
                    "NoPipeline: Recieved resp from memory system {:?}",
                    raw_resp
                );
                return SystemMessage::InstructionPending;
            }
        };
        info!("NoPipeline: Fetched {raw_instr}");

        let decoded_instr = if let Some(instr) = decode_raw_instr(raw_instr) {
            instr
        } else {
            error!("NoPipeline: Failed to decode raw instruction {raw_instr}, passing on a NOOP");
            todo!()
        };
        info!("NoPipeline: Decoded instruction to {:?}", decoded_instr);
        // TODO: Just do the rest of the work here? Will be a little repeptive but
        // that's fine for now...
        match decoded_instr {
            Instruction::Type0 { opcode } => {
                info!("NoPipeline: Got Type 0 instruction, opcode: {opcode}");
                match opcode {
                    // destination registers go into pending
                    // RET
                    0 => {
                        let jump_addr = self.registers.general[RET_REG].data.force_unsigned();
                        info!("NoPipeline: Jump address of {jump_addr} taken from Register R{RET_REG}");
                        self.registers.program_counter = jump_addr;
                        self.fetch = FetchState::default();
                        return SystemMessage::InstructionCompleted;
                    }
                    // HALT
                    1 => {
                        info!("NoPipeline: Halting");
                        self.fetch = FetchState::default();
                        return SystemMessage::Halt;
                    }
                    _ => {
                        error!("NoPipeline: Unrecognized opcode, passing on as NOOP");
                    }
                }
            }
            Instruction::Type1 { opcode, immediate } => {
                info!("NoPipeline: Got Type 0 instruction, opcode: {opcode}");
                match opcode {
                    // CALL
                    0 => {
                        info!("NoPipeline: CALL instruction");
                        self.pending_reg.insert((RegisterGroup::General, RET_REG));
                        // TODO: Need to change over to one past current instruction
                        self.registers.general[RET_REG] = Register {
                            data: MemBlock::Unsigned32(self.registers.program_counter),
                        };
                        self.registers.program_counter = immediate;
                        self.fetch = FetchState::default();
                        return SystemMessage::InstructionCompleted;
                    }
                    // JE
                    1 => {
                        info!("NoPipeline: JE instruction");
                        if self.registers.status.get(FlagIndex::EQ as usize) {
                            info!("NoPipeline: Jumping");
                            self.registers.program_counter = immediate;
                            self.fetch = FetchState::default();
                            return SystemMessage::InstructionCompleted;
                        } else {
                            info!("NoPipeline: Not jumping");
                        }
                    }
                    // JNE
                    2 => {
                        info!("NoPipeline: JNE instruction");
                        if !self.registers.status.get(FlagIndex::EQ as usize) {
                            info!("NoPipeline: Jumping");
                            self.registers.program_counter = immediate;
                            self.fetch = FetchState::default();
                            return SystemMessage::InstructionCompleted;
                        } else {
                            info!("NoPipeline: Not jumping");
                        }
                    }
                    // JGT
                    3 => {
                        info!("NoPipeline: JGT instruction");
                        if self.registers.status.get(FlagIndex::GT as usize) {
                            info!("NoPipeline: Jumping");
                            self.registers.program_counter = immediate;
                            self.fetch = FetchState::default();
                            return SystemMessage::InstructionCompleted;
                        } else {
                            info!("NoPipeline: Not jumping");
                        }
                    }
                    // JLT
                    4 => {
                        info!("NoPipeline: JLT instruction");
                        if self.registers.status.get(FlagIndex::LT as usize) {
                            info!("NoPipeline: Jumping");
                            self.registers.program_counter = immediate;
                            self.fetch = FetchState::default();
                            return SystemMessage::InstructionCompleted;
                        } else {
                            info!("NoPipeline: Not jumping");
                        }
                    }
                    // JGTE
                    5 => {
                        info!("NoPipeline: JGTE instruction");
                        if self.registers.status.get(FlagIndex::EQ as usize)
                            || self.registers.status.get(FlagIndex::GT as usize)
                        {
                            info!("NoPipeline: Jumping");
                            self.registers.program_counter = immediate;
                            self.fetch = FetchState::default();
                            return SystemMessage::InstructionCompleted;
                        } else {
                            info!("NoPipeline: Not jumping");
                        }
                    }
                    // JLTE
                    6 => {
                        info!("NoPipeline: JLTE instruction");
                        if self.registers.status.get(FlagIndex::EQ as usize)
                            || self.registers.status.get(FlagIndex::LT as usize)
                        {
                            info!("NoPipeline: Jumping");
                            self.registers.program_counter = immediate;
                            self.fetch = FetchState::default();
                            return SystemMessage::InstructionCompleted;
                        } else {
                            info!("NoPipeline: Not jumping");
                        }
                    }
                    // TODO: Jump relative is half worthless because we don't allow negative
                    // immediate arguments...fix this
                    // IJE
                    7 => {
                        info!("NoPipeline: IJE instruction");
                        if self.registers.status.get(FlagIndex::EQ as usize) {
                            info!("NoPipeline: Jumping");
                            self.registers.program_counter += immediate;
                            self.fetch = FetchState::default();
                            return SystemMessage::InstructionCompleted;
                        } else {
                            info!("NoPipeline: Not jumping");
                        }
                    }
                    // IJNE
                    8 => {
                        info!("NoPipeline: IJNE instruction");
                        if !self.registers.status.get(FlagIndex::EQ as usize) {
                            info!("NoPipeline: Jumping");
                            self.registers.program_counter += immediate;
                            self.fetch = FetchState::default();
                            return SystemMessage::InstructionCompleted;
                        } else {
                            info!("NoPipeline: Not jumping");
                        }
                    }
                    // IJGT
                    9 => {
                        info!("NoPipeline: IJGT instruction");
                        if self.registers.status.get(FlagIndex::GT as usize) {
                            info!("NoPipeline: Jumping");
                            self.registers.program_counter += immediate;
                            self.fetch = FetchState::default();
                            return SystemMessage::InstructionCompleted;
                        } else {
                            info!("NoPipeline: Not jumping");
                        }
                    }
                    // IJLT
                    10 => {
                        info!("NoPipeline: IJLT instruction");
                        if self.registers.status.get(FlagIndex::LT as usize) {
                            info!("NoPipeline: Jumping");
                            self.registers.program_counter += immediate;
                            self.fetch = FetchState::default();
                            return SystemMessage::InstructionCompleted;
                        } else {
                            info!("NoPipeline: Not jumping");
                        }
                    }
                    // IJGTE
                    11 => {
                        info!("NoPipeline: IJGTE instruction");
                        if self.registers.status.get(FlagIndex::EQ as usize)
                            || self.registers.status.get(FlagIndex::GT as usize)
                        {
                            info!("NoPipeline: Jumping");
                            self.registers.program_counter += immediate;
                            self.fetch = FetchState::default();
                            return SystemMessage::InstructionCompleted;
                        } else {
                            info!("NoPipeline: Not jumping");
                        }
                    }
                    // IJLTE
                    12 => {
                        info!("NoPipeline: IJLTE instruction");
                        if self.registers.status.get(FlagIndex::EQ as usize)
                            || self.registers.status.get(FlagIndex::LT as usize)
                        {
                            info!("NoPipeline: Jumping");
                            self.registers.program_counter += immediate;
                            self.fetch = FetchState::default();
                            return SystemMessage::InstructionCompleted;
                        } else {
                            info!("NoPipeline: Not jumping");
                        }
                    }
                    _ => {
                        error!("NoPipeline: Unrecognized opcode, passing on as NOOP");
                    }
                }
            }
            Instruction::Type2 {
                opcode,
                reg_1,
                reg_2,
            } => match opcode {
                0..=2 => {
                    // TODO: Account for bit widths
                    info!("NoPipeline: Comparing general registers {reg_1} and {reg_2}");
                    let flags = get_comparison_flags(
                        self.registers.general[reg_1],
                        self.registers.general[reg_2],
                    );
                    info!("NoPipeline: Comparison result: {:?}", flags);
                    for (idx, flag) in flags.iter().enumerate() {
                        if let Some(new_val) = flag {
                            self.registers.status.set(idx, *new_val);
                        }
                    }
                }
                //LDIN8
                3 => {
                    let address = self.registers.general[reg_2]
                        .data
                        .force_unsigned()
                        .try_into()
                        .unwrap();
                    let req = MemRequest::Load(LoadRequest {
                        issuer: PipelineStage::Execute,
                        address,
                        width: MemType::Unsigned8,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::Load(LoadResponse { data })) = resp {
                        let block_data = data.get_contents(address).unwrap();
                        if let MemBlock::Unsigned8(val) = block_data {
                            info!("NoPipeline: Got {val} back from memory request");
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Unsigned8(val),
                            };
                        } else {
                            info!("NoPipeline: Forcing to memory contents to u8");
                            let val = u8::try_from(block_data.force_unsigned()).unwrap();
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Unsigned8(val),
                            };
                        }
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                //LDIN16
                4 => {
                    let address = self.registers.general[reg_2]
                        .data
                        .force_unsigned()
                        .try_into()
                        .unwrap();
                    let req = MemRequest::Load(LoadRequest {
                        issuer: PipelineStage::Execute,
                        address,
                        width: MemType::Unsigned16,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::Load(LoadResponse { data })) = resp {
                        let block_data = data.get_contents(address).unwrap();
                        if let MemBlock::Unsigned16(val) = block_data {
                            info!("NoPipeline: Got {val} back from memory request");
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Unsigned16(val),
                            };
                        } else {
                            info!("NoPipeline: Forcing to memory contents to u16");
                            let val = u16::try_from(block_data.force_unsigned()).unwrap();
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Unsigned16(val),
                            };
                        }
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                //LDIN32
                5 => {
                    let address = self.registers.general[reg_2]
                        .data
                        .force_unsigned()
                        .try_into()
                        .unwrap();
                    let req = MemRequest::Load(LoadRequest {
                        issuer: PipelineStage::Execute,
                        address,
                        width: MemType::Unsigned32,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::Load(LoadResponse { data })) = resp {
                        let block_data = data.get_contents(address).unwrap();
                        if let MemBlock::Unsigned32(val) = block_data {
                            info!("NoPipeline: Got {val} back from memory request");
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Unsigned32(val),
                            };
                        } else {
                            info!("NoPipeline: Forcing to memory contents to u32");
                            let val = u32::try_from(block_data.force_unsigned()).unwrap();
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Unsigned32(val),
                            };
                        }
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                // STIN8
                6 => {
                    let address = self.registers.general[reg_2]
                        .data
                        .force_unsigned()
                        .try_into()
                        .unwrap();
                    let data = MemBlock::Unsigned32(
                        self.registers.general[reg_1]
                            .data
                            .force_unsigned()
                            .try_into()
                            .unwrap(),
                    );
                    let req = MemRequest::Store(StoreRequest {
                        issuer: PipelineStage::Execute,
                        address,
                        data,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::StoreComplete) = resp {
                        info!("NoPipeline: Received store complete response");
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                // STIN16
                7 => {
                    let address = self.registers.general[reg_2]
                        .data
                        .force_unsigned()
                        .try_into()
                        .unwrap();
                    let data = MemBlock::Unsigned32(
                        self.registers.general[reg_1]
                            .data
                            .force_unsigned()
                            .try_into()
                            .unwrap(),
                    );
                    let req = MemRequest::Store(StoreRequest {
                        issuer: PipelineStage::Execute,
                        address,
                        data,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::StoreComplete) = resp {
                        info!("NoPipeline: Received store complete response");
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                // STIN32
                8 => {
                    let address = self.registers.general[reg_2]
                        .data
                        .force_unsigned()
                        .try_into()
                        .unwrap();
                    let data = MemBlock::Unsigned32(
                        self.registers.general[reg_1]
                            .data
                            .force_unsigned()
                            .try_into()
                            .unwrap(),
                    );
                    let req = MemRequest::Store(StoreRequest {
                        issuer: PipelineStage::Execute,
                        address,
                        data,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::StoreComplete) = resp {
                        info!("NoPipeline: Received store complete response");
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                _ => {
                    error!("NoPipeline: Unrecognized opcode, passing on as NOOP");
                }
            },
            Instruction::Type3 {
                opcode,
                freg_1,
                freg_2,
            } => {
                // CMPF
                if opcode == 0 {
                    info!("NoPipeline: Comparing floating point registers {freg_1} and {freg_2}");
                    let flags = get_comparison_flags(
                        self.registers.float[freg_1],
                        self.registers.float[freg_2],
                    );
                    info!("NoPipeline: Comparison result: {:?}", flags);
                    for (idx, flag) in flags.iter().enumerate() {
                        if let Some(new_val) = flag {
                            self.registers.status.set(idx, *new_val);
                        }
                    }
                } else {
                    error!("NoPipeline: Unrecognized opcode, passing on as NOOP");
                }
            }
            Instruction::Type4 {
                opcode,
                reg_1,
                immediate,
            } => match opcode {
                // LD8
                0 => {
                    let address = self.registers.general[reg_1]
                        .data
                        .force_unsigned()
                        .try_into()
                        .unwrap();
                    let req = MemRequest::Load(LoadRequest {
                        issuer: PipelineStage::Execute,
                        address,
                        width: MemType::Unsigned8,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::Load(LoadResponse { data })) = resp {
                        let block_data = data.get_contents(address).unwrap();
                        if let MemBlock::Unsigned8(val) = block_data {
                            info!("NoPipeline: Got {val} back from memory request");
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Unsigned8(val),
                            };
                        } else {
                            info!("NoPipeline: Forcing to memory contents to u8");
                            let val = u8::try_from(block_data.force_unsigned()).unwrap();
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Unsigned8(val),
                            };
                        }
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                // LD16
                1 => {
                    let address = self.registers.general[reg_1]
                        .data
                        .force_unsigned()
                        .try_into()
                        .unwrap();
                    let req = MemRequest::Load(LoadRequest {
                        issuer: PipelineStage::Execute,
                        address,
                        width: MemType::Unsigned16,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::Load(LoadResponse { data })) = resp {
                        let block_data = data.get_contents(address).unwrap();
                        if let MemBlock::Unsigned16(val) = block_data {
                            info!("NoPipeline: Got {val} back from memory request");
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Unsigned16(val),
                            };
                        } else {
                            info!("NoPipeline: Forcing to memory contents to u16");
                            let val = u16::try_from(block_data.force_unsigned()).unwrap();
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Unsigned16(val),
                            };
                        }
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                // LD32
                2 => {
                    let address = self.registers.general[reg_1]
                        .data
                        .force_unsigned()
                        .try_into()
                        .unwrap();
                    let req = MemRequest::Load(LoadRequest {
                        issuer: PipelineStage::Execute,
                        address,
                        width: MemType::Unsigned32,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::Load(LoadResponse { data })) = resp {
                        let block_data = data.get_contents(address).unwrap();
                        if let MemBlock::Unsigned32(val) = block_data {
                            info!("NoPipeline: Got {val} back from memory request");
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Unsigned32(val),
                            };
                        } else {
                            info!("NoPipeline: Forcing to memory contents to u32");
                            let val = u32::try_from(block_data.force_unsigned()).unwrap();
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Unsigned32(val),
                            };
                        }
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                // LDI8
                3 => {
                    let address = self.registers.general[reg_1]
                        .data
                        .force_signed()
                        .try_into()
                        .unwrap();
                    let req = MemRequest::Load(LoadRequest {
                        issuer: PipelineStage::Execute,
                        address,
                        width: MemType::Signed8,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::Load(LoadResponse { data })) = resp {
                        let block_data = data.get_contents(address).unwrap();
                        if let MemBlock::Signed8(val) = block_data {
                            info!("NoPipeline: Got {val} back from memory request");
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Signed8(val),
                            };
                        } else {
                            info!("NoPipeline: Forcing to memory contents to i8");
                            let val = i8::try_from(block_data.force_signed()).unwrap();
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Signed8(val),
                            };
                        }
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                // LDI16
                4 => {
                    let address = self.registers.general[reg_1]
                        .data
                        .force_signed()
                        .try_into()
                        .unwrap();
                    let req = MemRequest::Load(LoadRequest {
                        issuer: PipelineStage::Execute,
                        address,
                        width: MemType::Signed16,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::Load(LoadResponse { data })) = resp {
                        let block_data = data.get_contents(address).unwrap();
                        if let MemBlock::Signed16(val) = block_data {
                            info!("NoPipeline: Got {val} back from memory request");
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Signed16(val),
                            };
                        } else {
                            info!("NoPipeline: Forcing to memory contents to i16");
                            let val = i16::try_from(block_data.force_signed()).unwrap();
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Signed16(val),
                            };
                        }
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                // LDI32
                5 => {
                    let address = self.registers.general[reg_1]
                        .data
                        .force_signed()
                        .try_into()
                        .unwrap();
                    let req = MemRequest::Load(LoadRequest {
                        issuer: PipelineStage::Execute,
                        address,
                        width: MemType::Signed32,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::Load(LoadResponse { data })) = resp {
                        let block_data = data.get_contents(address).unwrap();
                        if let MemBlock::Signed32(val) = block_data {
                            info!("NoPipeline: Got {val} back from memory request");
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Signed32(val),
                            };
                        } else {
                            info!("NoPipeline: Forcing to memory contents to u32");
                            let val = i32::try_from(block_data.force_unsigned()).unwrap();
                            self.registers.general[reg_1] = Register {
                                data: MemBlock::Signed32(val),
                            };
                        }
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                // ST8
                6 => {
                    info!("NoPipeline: ST8");
                    let req = MemRequest::Store(StoreRequest {
                        issuer: PipelineStage::Execute,
                        address: usize::try_from(immediate).unwrap(),
                        data: self.registers.general[reg_1].data,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::StoreComplete) = resp {
                        info!("NoPipeline: Store complete");
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                // ST16
                7 => {
                    info!("NoPipeline: ST16");
                    let req = MemRequest::Store(StoreRequest {
                        issuer: PipelineStage::Execute,
                        address: usize::try_from(immediate).unwrap(),
                        data: self.registers.general[reg_1].data,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::StoreComplete) = resp {
                        info!("NoPipeline: Store complete");
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                // ST32
                8 => {
                    info!("NoPipeline: ST32");
                    let req = MemRequest::Store(StoreRequest {
                        issuer: PipelineStage::Execute,
                        address: usize::try_from(immediate).unwrap(),
                        data: self.registers.general[reg_1].data,
                    });
                    let resp = self.memory_system.request(&req);
                    if let Ok(MemResponse::StoreComplete) = resp {
                        info!("NoPipeline: Store complete");
                    } else {
                        info!("NoPipeline: Recieved resp from memory system {:?}", resp);
                        return SystemMessage::InstructionPending;
                    }
                }
                // ADDIM
                9 => {
                    // TODO: Add overflow checks later...
                    info!(
                        "NoPipeline: Adding immediate {} to register {}",
                        immediate, reg_1
                    );
                    let data = self.registers.general[reg_1].data.add_immediate(immediate);
                    self.registers.general[reg_1] = Register { data };
                }
                _ => {
                    error!("NoPipeline: Unrecognized opcode, passing on as NOOP");
                }
            },
            Instruction::Type5 {
                opcode,
                reg_1,
                reg_2,
                reg_3,
            } => {
                // TODO: Created signed and unsigned variants...
                match opcode {
                    // ADDI
                    0 => {
                        // TODO: Add overflow checks later...
                        let data = self.registers.general[reg_2]
                            .data
                            .add_register(self.registers.general[reg_3].data);
                        info!(
                            "NoPipeline: Adding register {} to register {}",
                            reg_2, reg_3
                        );
                        self.registers.general[reg_1] = Register { data };
                    }
                    // SUBI
                    1 => {
                        let data = self.registers.general[reg_2]
                            .data
                            .sub_register(self.registers.general[reg_3].data);
                        info!(
                            "NoPipeline: Subtracting register {} from register {}",
                            reg_3, reg_2
                        );
                        self.registers.general[reg_1] = Register { data };
                    }
                    // MULI
                    2 => {
                        let data = self.registers.general[reg_2]
                            .data
                            .mul_register(self.registers.general[reg_3].data);
                        info!(
                            "NoPipeline: Multiplying register {} with register {}",
                            reg_2, reg_3
                        );
                        self.registers.general[reg_1] = Register { data };
                    }
                    // DIVI
                    3 => {
                        let data = self.registers.general[reg_2]
                            .data
                            .div_register(self.registers.general[reg_3].data);
                        info!(
                            "NoPipeline: Dividing register {} by register {}",
                            reg_2, reg_3
                        );
                        self.registers.general[reg_1] = Register { data };
                    }
                    // MODI
                    4 => {
                        let data = self.registers.general[reg_2]
                            .data
                            .mod_register(self.registers.general[reg_3].data);
                        info!(
                            "NoPipeline: Modulo register {} by register {}",
                            reg_2, reg_3
                        );
                        self.registers.general[reg_1] = Register { data };
                    }
                    // RBSI
                    5 => {
                        let data = self.registers.general[reg_2]
                            .data
                            .right_shift_register(self.registers.general[reg_3].data);
                        info!(
                            "NoPipeline: Right bit shift register {} by register {}",
                            reg_2, reg_3
                        );
                        self.registers.general[reg_1] = Register { data };
                    }
                    // XORI
                    6 => {
                        let data = self.registers.general[reg_2]
                            .data
                            .xor_register(self.registers.general[reg_3].data);
                        info!("NoPipeline: XOR register {} with register {}", reg_2, reg_3);
                        self.registers.general[reg_1] = Register { data };
                    }
                    // ANDI
                    7 => {
                        let data = self.registers.general[reg_2]
                            .data
                            .and_register(self.registers.general[reg_3].data);
                        info!("NoPipeline: AND register {} with register {}", reg_2, reg_3);
                        self.registers.general[reg_1] = Register { data };
                    }
                    // ORI
                    8 => {
                        let data = self.registers.general[reg_2]
                            .data
                            .or_register(self.registers.general[reg_3].data);
                        info!("NoPipeline: OR register {} with register {}", reg_2, reg_3);
                        self.registers.general[reg_1] = Register { data };
                    }
                    // ADDU
                    9 => {
                        let data = self.registers.general[reg_2]
                            .data
                            .add_register(self.registers.general[reg_3].data);
                        info!("NoPipeline: Add register {} with register {}", reg_2, reg_3);
                        self.registers.general[reg_1] = Register { data };
                    }
                    // SUBU
                    10 => {
                        let data = self.registers.general[reg_2]
                            .data
                            .sub_register(self.registers.general[reg_3].data);
                        info!(
                            "NoPipeline: Subtract register {} from register {}",
                            reg_3, reg_2
                        );
                        self.registers.general[reg_1] = Register { data };
                    }
                    // MULU
                    11 => {
                        let data = self.registers.general[reg_2]
                            .data
                            .mul_register(self.registers.general[reg_3].data);
                        info!(
                            "NoPipeline: Multiply register {} with register {}",
                            reg_2, reg_3
                        );
                        self.registers.general[reg_1] = Register { data };
                    }
                    // DIVU
                    12 => {
                        let data = self.registers.general[reg_2]
                            .data
                            .div_register(self.registers.general[reg_3].data);
                        info!(
                            "NoPipeline: Divide register {} by register {}",
                            reg_2, reg_3
                        );
                        self.registers.general[reg_1] = Register { data };
                    }
                    // MODU
                    13 => {
                        let data = self.registers.general[reg_2]
                            .data
                            .mod_register(self.registers.general[reg_3].data);
                        info!("NoPipeline: Mod register {} by register {}", reg_2, reg_3);
                        self.registers.general[reg_1] = Register { data };
                    }
                    _ => {
                        error!("NoPipeline: Unrecognized opcode, passing on as NOOP");
                    }
                }
            }
            Instruction::Type6 {
                opcode,
                freg_1,
                freg_2,
                freg_3,
            } => {
                match opcode {
                    // ADDF
                    0 => {
                        // TODO: Add overflow checks later...
                        let data = self.registers.float[freg_2]
                            .data
                            .add_register(self.registers.float[freg_3].data);
                        info!(
                            "Pipeline::Execute: Add register {} with register {}",
                            freg_2, freg_3
                        );
                        self.registers.float[freg_1] = Register { data };
                    }
                    // SUBF
                    1 => {
                        let data = self.registers.float[freg_2]
                            .data
                            .sub_register(self.registers.float[freg_3].data);
                        info!(
                            "Pipeline::Execute: Subtracting register {} from register {}",
                            freg_3, freg_2
                        );
                        self.registers.float[freg_1] = Register { data };
                    }
                    // MULF
                    2 => {
                        let data = self.registers.float[freg_2]
                            .data
                            .mul_register(self.registers.float[freg_3].data);
                        info!(
                            "Pipeline::Execute: Multiplying register {} with register {}",
                            freg_2, freg_3
                        );
                        self.registers.float[freg_1] = Register { data };
                    }
                    // DIVF
                    3 => {
                        let data = self.registers.float[freg_2]
                            .data
                            .div_register(self.registers.float[freg_3].data);
                        info!(
                            "Pipeline::Execute: Dividing register {} by register {}",
                            freg_2, freg_3
                        );
                        self.registers.float[freg_1] = Register { data };
                    }
                    _ => {
                        error!("NoPipeline: Unrecognized opcode, passing on as NOOP");
                    }
                }
            }
        }

        // By default, the program counter will increment here to advance to the
        // next instruction. Instructions in an unfinished state at the end of the
        // current clock cycle (e.g. waiting on a request to return from the memory
        // subsystem) should return out of the function *early* to avoid skipping
        // to the next instruction
        self.registers.step_pc();
        self.fetch = FetchState::default();
        SystemMessage::InstructionCompleted
    }

    fn pipeline_run(&mut self) -> SystemMessage {
        info!("Entering the pipeline");
        self.pipeline_writeback()
    }

    // BUG: Memory requests further down in the pipeline conflict with fetch, causes
    // deadlock (make finished requests a hashset instead of a single optional?)
    #[allow(clippy::too_many_lines)] // TODO: Fix this later..
    fn pipeline_fetch(&mut self, decode_blocked: bool) -> PipelineStageStatus {
        info!(
            "Pipeline::Fetch: In fetch stage, current PC: {}, current instruction: {:?}",
            self.registers.program_counter, self.fetch
        );
        match (self.fetch.raw_instr, decode_blocked) {
            (None, _) => {
                // If no current instruction, send load to cache with PC as address
                let fetch_addr = self.registers.program_counter as usize;
                let req = MemRequest::Load(LoadRequest {
                    issuer: PipelineStage::Fetch,
                    address: fetch_addr,
                    width: MemType::Unsigned32,
                });
                info!(
                    "Pipeline::Fetch: No current instruction, issuing fetch to memory subsystem: {:?}",
                    req
                );
                // TODO: Lots of cleanup here with the memory system
                let resp = self.memory_system.request(&req);
                info!("Pipeline::Fetch: Memory subsystem response: {:?}", resp);
                match resp {
                    Ok(MemResponse::Load(LoadResponse { data })) => {
                        info!("Pipeline::Fetch: Got valid load response",);
                        self.registers.step_pc();
                        if let Some(conts) = data.get_contents(req.get_address()) {
                            let raw = match conts {
                                MemBlock::Unsigned8(data) => {
                                    error!(
                                            "Pipeline::Fetch: Received u8 for instruction fetch, translating to u32"
                                        );
                                    u32::from(data)
                                }
                                MemBlock::Unsigned16(data) => {
                                    error!("Pipeline::Fetch: Received u16 for instruction fetch, translating to u32");
                                    u32::from(data)
                                }
                                MemBlock::Unsigned32(data) => data,
                                MemBlock::Signed8(_) => {
                                    error!(
                                            "Pipeline::Fetch: Received i8 for instruction fetch, passing 0"
                                        );
                                    0
                                }
                                MemBlock::Signed16(_) => {
                                    error!("Pipeline::Fetch: Received i16 for instruction fetch, passing to 0");
                                    0
                                }
                                MemBlock::Signed32(_) => {
                                    error!("Pipeline::Fetch: Received i32 for instruction fetch, passing to 0");
                                    0
                                }
                                MemBlock::Float32(_) => {
                                    error!("Pipeline::Fetch: Received f32 for instruction fetch, passing to 0");
                                    0
                                }
                            };
                            error!("GOT HERE: {decode_blocked}");
                            if decode_blocked {
                                info!("Pipeline::Fetch: Fetched instruction, decode is blocked, saving for next cycle");
                                self.fetch.raw_instr = Some(raw);
                                self.fetch.src_addr = Some(fetch_addr);
                                PipelineStageStatus::Noop
                            } else {
                                let fetched =
                                    PipelineStageStatus::Instruction(PipelineInstruction {
                                        src_addr: Some(fetch_addr),
                                        raw_instr: Some(raw),
                                        decode_instr: None,
                                        instr_result: PipelineInstructionResult::Empty,
                                    });
                                info!("Pipeline::Fetch: Passing on raw instruction: {:?}", fetched);
                                fetched
                            }
                        } else {
                            error!("Pipeline::Fetch: Received empty memory response, treating as a NOOP");
                            PipelineStageStatus::Noop
                        }
                    }
                    Ok(MemResponse::Miss) => {
                        info!("Pipeline::Fetch: Request missed");
                        PipelineStageStatus::Stall
                    }
                    Ok(MemResponse::Wait) => {
                        info!("Pipeline::Fetch: Request got wait");
                        PipelineStageStatus::Stall
                    }
                    Ok(MemResponse::StoreComplete) => {
                        error!("Pipeline::Fetch: Got StoreComplete response for fetch request");
                        PipelineStageStatus::Stall
                    }
                    Err(e) => {
                        error!("Pipeline::Fetch: Got error {e} from memory subsystem, translating into NOOP");
                        PipelineStageStatus::Noop
                    }
                }
            }
            (Some(instr), false) => {
                info!(
                    "Pipeline::Fetch: Have instruction {:?}, decode is unblocked, returning instruction result",
                    instr
                );
                let raw_instr = self.fetch.raw_instr;
                let src_addr = self.fetch.src_addr;
                self.fetch.raw_instr = None;
                self.fetch.src_addr = None;
                PipelineStageStatus::Instruction(PipelineInstruction {
                    raw_instr,
                    src_addr,
                    decode_instr: None,
                    instr_result: PipelineInstructionResult::Empty,
                })
            }
            (Some(instr), true) => {
                info!(
                    "Pipeline::Fetch: Have instruction {:?}, decode is blocked, returning NOOP",
                    instr
                );
                PipelineStageStatus::Noop
            }
        }
    }

    fn pipeline_decode(&mut self, exec_blocked: bool) -> PipelineStageStatus {
        info!(
            "Pipeline::Decode: In decode stage, current instruction: {:?}, exec blocked: {}",
            self.decode, exec_blocked
        );
        let mut pending_regs = false;
        match self.decode {
            // make sure we're not just repeating a decode here if exec is blocked
            PipelineStageStatus::Instruction(ref mut instruction)
                if instruction.decode_instr.is_none() =>
            {
                if let Some(raw) = instruction.raw_instr {
                    // split instruction into fields
                    if let Some(instr) = decode_raw_instr(raw) {
                        let src_regs = instr.get_src_regs();
                        pending_regs = src_regs.iter().any(|src| self.pending_reg.contains(src));
                        info!("Pipeline::Decode: Pending source registers: {pending_regs}");
                        if !pending_regs {
                            instruction.decode_instr = Some(instr);
                        }
                    } else {
                        error!("Pipeline::Decode: Failed to decode raw instruction {raw}, passing on a NOOP");
                        self.decode = PipelineStageStatus::Noop;
                    }
                } else {
                    error!(
                        "Pipeline::Decode: Received empty raw instruction field, passing on a NOOP"
                    );
                    self.decode = PipelineStageStatus::Noop;
                }
            }
            PipelineStageStatus::Instruction(ref instr) => {
                info!(
                    "Pipeline::Decode: Current instruction already decoded: {:?}",
                    instr
                );
            }
            PipelineStageStatus::Stall => {
                // if Noop/Stall, do nothing
                info!("Pipeline::Decode: Stall is current state");
            }
            PipelineStageStatus::Noop => {
                // if Noop/Stall, do nothing
                info!("Pipeline::Decode: Noop is current state");
            }
        }
        // NOTE: if we can just grab the registers' contents by value here, maybe
        // we can simplify logic down the line...
        match (pending_regs, exec_blocked) {
            // instruction missing operands OR execute is blocked
            (_, true) => {
                info!("Pipeline::Decode: Calling fetch with blocked status");
                self.pipeline_fetch(true); // shouldn't get anything back because we're blocked...
                info!("Pipeline::Decode: Passing on a Noop status");
                PipelineStageStatus::Noop
            }
            (true, _) => {
                info!("Pipeline::Decode: Calling fetch with blocked status");
                self.pipeline_fetch(true); // shouldn't get anything back because we're blocked...
                info!("Pipeline::Decode: Passing on a Noop status");
                PipelineStageStatus::Noop
            }
            (false, false) => {
                let completed_instr = if PipelineStageStatus::Stall == self.decode {
                    info!("Pipeline::Decode: Translating Stall to Noop");
                    PipelineStageStatus::Noop
                } else {
                    self.decode
                };
                // BUG: dependencies not properly observed, not sure why
                if let PipelineStageStatus::Instruction(instr) = completed_instr {
                    if let Some(reg) = instr.get_dest_reg() {
                        info!(
                            "Pipeline::Decode: Inserting {:?} into pending registers",
                            reg
                        );
                        self.pending_reg.insert(reg);
                        error!("Adding {:?} to pending registers", reg);
                    }
                }
                info!("Pipeline::Decode: Calling fetch with unblocked status");
                self.decode = self.pipeline_fetch(false);
                info!(
                    "Pipeline::Decode: Instruction saved for next decode: {:?}",
                    self.decode
                );
                info!(
                    "Pipeline::Decode: Returning decoded instruction {:?} to execute",
                    completed_instr
                );
                completed_instr
            }
        }
    }

    // TODO: Fill in memory results here...
    #[allow(clippy::too_many_lines)] // TODO: Fix this later...
                                     // NOTE: Make sure to set flag status in result for all ALU ops...
    fn pipeline_execute(&mut self, mem_blocked: bool) -> PipelineStageStatus {
        info!(
            "Pipeline::Execute: In execute stage, current instruction: {:?}, memory blocked: {}",
            self.execute, mem_blocked
        );
        // get current pc of execute instruction?
        let src_addr = if let PipelineStageStatus::Instruction(PipelineInstruction {
            src_addr: Some(src_addr),
            ..
        }) = self.execute
        {
            u32::try_from(src_addr).unwrap()
        } else {
            error!("Pipeline::Execute: Unable to find address for instruction ");
            0
        };
        // execute appears to pass along a more "filled in" instruction object, look into this...
        match self.execute {
            PipelineStageStatus::Instruction(ref mut instr) => {
                info!("Pipeline::Execute: Have current instruction: {:?}", instr);
                if let Some(ref mut instruction) = instr.decode_instr {
                    match instruction {
                        Instruction::Type0 { opcode } => {
                            info!("Pipeline::Execute: Type 0 instruction");
                            match opcode {
                                // RET
                                0 => {
                                    info!("RET instruction, setting branch result");
                                    let addr =
                                        self.registers.general[RET_REG].data.force_unsigned();
                                    instr.instr_result =
                                        PipelineInstructionResult::Branch { new_pc: addr }
                                }
                                _ => {
                                    info!("Other instruction, setting empty result");
                                    instr.instr_result = PipelineInstructionResult::Empty;
                                }
                            }
                        }
                        Instruction::Type1 { opcode, immediate } => {
                            info!("Pipeline::Execute: Type 1 instruction");
                            match opcode {
                                // CALL
                                0 => {
                                    info!("CALL instruction, setting JSR result");
                                    instr.instr_result =
                                        PipelineInstructionResult::JumpSubRoutine {
                                            new_pc: *immediate,
                                            ret_reg_val: src_addr
                                                + u32::try_from(MEM_BLOCK_WIDTH).unwrap(),
                                        };
                                }
                                // Branch immediate
                                // JE
                                1 => {
                                    instr.instr_result =
                                        if self.registers.status.get(FlagIndex::EQ as usize) {
                                            info!("JE Instruction...EQ flag is set");
                                            PipelineInstructionResult::Branch { new_pc: *immediate }
                                        } else {
                                            info!("JE Instruction...EQ flag is not set");
                                            PipelineInstructionResult::Empty
                                        }
                                }
                                // JNE
                                2 => {
                                    instr.instr_result =
                                        if !self.registers.status.get(FlagIndex::EQ as usize) {
                                            info!("JNE Instruction...EQ flag is not set");
                                            PipelineInstructionResult::Branch { new_pc: *immediate }
                                        } else {
                                            info!("JNE Instruction...EQ flag is set");
                                            PipelineInstructionResult::Empty
                                        }
                                }
                                // JGT
                                3 => {
                                    instr.instr_result =
                                        if self.registers.status.get(FlagIndex::GT as usize) {
                                            info!("JNE Instruction...GT flag is set");
                                            PipelineInstructionResult::Branch { new_pc: *immediate }
                                        } else {
                                            info!("JNE Instruction...GT flag is not set");
                                            PipelineInstructionResult::Empty
                                        }
                                }
                                // JLT
                                4 => {
                                    instr.instr_result =
                                        if self.registers.status.get(FlagIndex::LT as usize) {
                                            info!("JLE Instruction...LT flag is set");
                                            PipelineInstructionResult::Branch { new_pc: *immediate }
                                        } else {
                                            info!("JLE Instruction...LT flag is not set");
                                            PipelineInstructionResult::Empty
                                        }
                                }
                                // JGTE
                                5 => {
                                    instr.instr_result =
                                        if self.registers.status.get(FlagIndex::EQ as usize)
                                            || self.registers.status.get(FlagIndex::GT as usize)
                                        {
                                            info!("JGTE Instruction...EQ or GT flag is set");
                                            PipelineInstructionResult::Branch { new_pc: *immediate }
                                        } else {
                                            info!("JGTE Instruction...EQ and GT flag are not set");
                                            PipelineInstructionResult::Empty
                                        }
                                }
                                // JLTE
                                6 => {
                                    instr.instr_result =
                                        if self.registers.status.get(FlagIndex::EQ as usize)
                                            || self.registers.status.get(FlagIndex::LT as usize)
                                        {
                                            info!("JLTE Instruction...EQ or LT flag is set");
                                            PipelineInstructionResult::Branch { new_pc: *immediate }
                                        } else {
                                            info!("JLTE Instruction...EQ and LT flag are not set");
                                            PipelineInstructionResult::Empty
                                        }
                                }
                                // Branch indirect
                                // IJE
                                7 => {
                                    instr.instr_result =
                                        if self.registers.status.get(FlagIndex::EQ as usize) {
                                            info!("IJE Instruction...EQ flag is set");
                                            PipelineInstructionResult::Branch {
                                                new_pc: src_addr + *immediate,
                                            }
                                        } else {
                                            info!("IJE Instruction...EQ flag is not set");
                                            PipelineInstructionResult::Empty
                                        }
                                }
                                // IJNE
                                8 => {
                                    instr.instr_result =
                                        if !self.registers.status.get(FlagIndex::EQ as usize) {
                                            info!("IJNE Instruction...EQ flag is not set");
                                            PipelineInstructionResult::Branch {
                                                new_pc: src_addr + *immediate,
                                            }
                                        } else {
                                            info!("IJNE Instruction...EQ flag is set");
                                            PipelineInstructionResult::Empty
                                        }
                                }
                                // IJGT
                                9 => {
                                    instr.instr_result =
                                        if self.registers.status.get(FlagIndex::GT as usize) {
                                            info!("IJNE Instruction...GT flag is set");
                                            PipelineInstructionResult::Branch {
                                                new_pc: src_addr + *immediate,
                                            }
                                        } else {
                                            info!("IJNE Instruction...GT flag is not set");
                                            PipelineInstructionResult::Empty
                                        }
                                }
                                // IJLT
                                10 => {
                                    instr.instr_result =
                                        if self.registers.status.get(FlagIndex::LT as usize) {
                                            info!("IJLE Instruction...LT flag is set");
                                            PipelineInstructionResult::Branch {
                                                new_pc: src_addr + *immediate,
                                            }
                                        } else {
                                            info!("IJLE Instruction...LT flag is not set");
                                            PipelineInstructionResult::Empty
                                        }
                                }
                                // IJGTE
                                11 => {
                                    instr.instr_result =
                                        if self.registers.status.get(FlagIndex::EQ as usize)
                                            || self.registers.status.get(FlagIndex::GT as usize)
                                        {
                                            info!("IJGTE Instruction...EQ or GT flag is set");
                                            PipelineInstructionResult::Branch {
                                                new_pc: src_addr + *immediate,
                                            }
                                        } else {
                                            info!("IJGTE Instruction...EQ and GT flag are not set");
                                            PipelineInstructionResult::Empty
                                        }
                                }
                                // IJLTE
                                12 => {
                                    instr.instr_result =
                                        if self.registers.status.get(FlagIndex::EQ as usize)
                                            || self.registers.status.get(FlagIndex::LT as usize)
                                        {
                                            info!("IJLTE Instruction...EQ or LT flag is set");
                                            PipelineInstructionResult::Branch {
                                                new_pc: src_addr + *immediate,
                                            }
                                        } else {
                                            info!("IJLTE Instruction...EQ and LT flag are not set");
                                            PipelineInstructionResult::Empty
                                        }
                                }
                                _ => {
                                    info!("Pipeline::Execute: Other instruction, empty result");
                                    instr.instr_result = PipelineInstructionResult::Empty;
                                }
                            }
                        }
                        Instruction::Type2 {
                            opcode,
                            reg_1,
                            reg_2,
                        } => match opcode {
                            0..=2 => {
                                info!("Pipeline::Execute: Comparing general registers {reg_1} and {reg_2}");
                                let flags = get_comparison_flags(
                                    self.registers.general[*reg_1],
                                    self.registers.general[*reg_2],
                                );
                                instr.instr_result = PipelineInstructionResult::Flag { flags };
                            }
                            _ => {
                                instr.instr_result = PipelineInstructionResult::Empty;
                            }
                        },
                        Instruction::Type3 {
                            opcode: _,
                            freg_1,
                            freg_2,
                        } => {
                            info!("Pipeline::Execute: Comparing floating point registers {freg_1} and {freg_2}");
                            let flags = get_comparison_flags(
                                self.registers.float[*freg_1],
                                self.registers.float[*freg_2],
                            );
                            instr.instr_result = PipelineInstructionResult::Flag { flags };
                        }
                        Instruction::Type4 {
                            opcode,
                            reg_1,
                            immediate,
                        } => match opcode {
                            9 => {
                                // TODO: Add overflow checks later...
                                info!(
                                    "Pipeline::Execute: Adding immediate {} to register {}",
                                    *immediate, *reg_1
                                );
                                let data = self.registers.general[*reg_1]
                                    .data
                                    .add_immediate(*immediate);
                                instr.instr_result = PipelineInstructionResult::Register {
                                    reg_group: RegisterGroup::General,
                                    dest_reg: *reg_1,
                                    data,
                                };
                                info!("Pipeline::Execute: instruction: {:?}", self.execute)
                            }
                            _ => {
                                instr.instr_result = PipelineInstructionResult::Empty;
                            }
                        },
                        Instruction::Type5 {
                            opcode,
                            reg_1,
                            reg_2,
                            reg_3,
                        } => {
                            // TODO: Created signed and unsigned variants...
                            match opcode {
                                // ADDI
                                0 => {
                                    // TODO: Add overflow checks later...
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .add_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: Adding register {} to register {}",
                                        *reg_2, *reg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                // SUBI
                                1 => {
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .sub_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: Subtracting register {} from register {}",
                                        *reg_3, *reg_2
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                // MULI
                                2 => {
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .mul_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: Multiplying register {} with register {}",
                                        *reg_2, *reg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                // DIVI
                                3 => {
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .div_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: Dividing register {} by register {}",
                                        *reg_2, *reg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                // MODI
                                4 => {
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .mod_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: Modulo register {} by register {}",
                                        *reg_2, *reg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                // RBSI
                                5 => {
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .right_shift_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: Right bit shift register {} by register {}",
                                        *reg_2, *reg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                // XORI
                                6 => {
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .xor_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: XOR register {} with register {}",
                                        *reg_2, *reg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                // ANDI
                                7 => {
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .and_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: AND register {} with register {}",
                                        *reg_2, *reg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                // ORI
                                8 => {
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .or_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: OR register {} with register {}",
                                        *reg_2, *reg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                // ADDU
                                9 => {
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .add_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: Add register {} with register {}",
                                        *reg_2, *reg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                // SUBU
                                10 => {
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .sub_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: Subtract register {} from register {}",
                                        *reg_3, *reg_2
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                // MULU
                                11 => {
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .mul_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: Multiply register {} with register {}",
                                        *reg_2, *reg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                // DIVU
                                12 => {
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .div_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: Divide register {} by register {}",
                                        *reg_2, *reg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                // MODU
                                13 => {
                                    let data = self.registers.general[*reg_2]
                                        .data
                                        .mod_register(self.registers.general[*reg_3].data);
                                    info!(
                                        "Pipeline::Execute: Mod register {} by register {}",
                                        *reg_2, *reg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                _ => {
                                    instr.instr_result = PipelineInstructionResult::Empty;
                                    info!("Pipeline::Execute: Nothing to do here",);
                                }
                            }
                        }
                        Instruction::Type6 {
                            opcode,
                            freg_1,
                            freg_2,
                            freg_3,
                        } => {
                            match opcode {
                                // ADDF
                                0 => {
                                    // TODO: Add overflow checks later...
                                    let data = self.registers.float[*freg_2]
                                        .data
                                        .add_register(self.registers.float[*freg_3].data);
                                    info!(
                                        "Pipeline::Execute: Add register {} with register {}",
                                        *freg_2, *freg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::FloatingPoint,
                                        dest_reg: *freg_1,
                                        data,
                                    }
                                }
                                // SUBF
                                1 => {
                                    let data = self.registers.float[*freg_2]
                                        .data
                                        .sub_register(self.registers.float[*freg_3].data);
                                    info!(
                                        "Pipeline::Execute: Subtracting register {} from register {}",
                                        *freg_3, *freg_2
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::FloatingPoint,
                                        dest_reg: *freg_1,
                                        data,
                                    }
                                }
                                // MULF
                                2 => {
                                    let data = self.registers.float[*freg_2]
                                        .data
                                        .mul_register(self.registers.float[*freg_3].data);
                                    info!(
                                        "Pipeline::Execute: Multiplying register {} with register {}",
                                        *freg_2, *freg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::FloatingPoint,
                                        dest_reg: *freg_1,
                                        data,
                                    }
                                }
                                // DIVF
                                3 => {
                                    let data = self.registers.float[*freg_2]
                                        .data
                                        .div_register(self.registers.float[*freg_3].data);
                                    info!(
                                        "Pipeline::Execute: Dividing register {} by register {}",
                                        *freg_2, *freg_3
                                    );
                                    instr.instr_result = PipelineInstructionResult::Register {
                                        reg_group: RegisterGroup::FloatingPoint,
                                        dest_reg: *freg_1,
                                        data,
                                    }
                                }
                                _ => {
                                    instr.instr_result = PipelineInstructionResult::Empty;
                                    info!("Pipeline::Execute: Nothing to do here",);
                                }
                            }
                        }
                    }
                } else {
                    error!("Pipeline::Execute: Received non-decoded instruction in execute stage");
                    panic!("Non-decoded instruction encountered in execute stage");
                }
            }
            PipelineStageStatus::Stall => {
                // if Noop/Stall, do nothing
                info!("Pipeline::Execute: Stall is current state");
            }
            PipelineStageStatus::Noop => {
                // if Noop/Stall, do nothing
                info!("Pipeline::Execute: Noop is current state");
            }
        }

        if mem_blocked {
            info!("Calling decode with blocked status");
            self.pipeline_decode(mem_blocked);
            info!("Pipeline::Execute: Returning Noop");
            PipelineStageStatus::Noop
        } else {
            let completed_instr = self.execute; // TODO: Fill in result for this...
            info!("Pipeline::Execute: Calling decode with memory blocked = {mem_blocked}, saving result to execute's state");
            self.execute = self.pipeline_decode(mem_blocked);
            if completed_instr == PipelineStageStatus::Stall {
                info!("Pipeline::Execute: Returning Noop");
                PipelineStageStatus::Noop
            } else {
                info!(
                    "Pipeline::Execute: Returning instruction {:?}",
                    completed_instr
                );
                completed_instr // check if translation from stall is needed here
            }
        }
    }

    #[allow(clippy::too_many_lines)] // TODO: Fix this later...
    #[must_use]
    fn pipeline_memory(&mut self) -> PipelineStageStatus {
        info!(
            "Pipeline::Memory: Pipeline: In memory stage, current instruction: {:?}",
            self.memory
        );
        match self.memory {
            PipelineStageStatus::Instruction(instr) => {
                info!("Pipeline::Memory: Have current instruction: {:?}", instr);
                if let Some(instruction) = instr.decode_instr {
                    if let Some(req) = instruction
                        .get_mem_req(Some(PipelineStage::Memory), &self.registers.general)
                    {
                        // If load, call memory system
                        //  - if hit and delay or miss, get wait back
                        //      - assuming we have to pass the Wait/Stall along...
                        // If value returned, call E non-blocked
                        // If wait returned, call E with blocked
                        info!(
                                "Pipeline::Memory: Associated memory request: {:?}, issuing to memory system",
                                req
                            );
                        let resp = self.memory_system.request(&req);
                        info!(
                            "Pipeline::Memory: Got {:?} response from memory system",
                            resp
                        );
                        match resp {
                            Ok(MemResponse::Miss | MemResponse::Wait) => {
                                // if not blocked, return instruction with result
                                // if blocked, return Noop/ Stall
                                info!("Pipeline::Memory: Calling execute with memory blocked");
                                self.pipeline_execute(true);
                                info!("Pipeline::Memory: Returning stall status to writeback");
                                PipelineStageStatus::Stall
                            }
                            // should this happen here?
                            Ok(MemResponse::StoreComplete) => {
                                info!(
                                    "Pipeline::Memory: Store request returned StoreComplete status"
                                );
                                let mut completed_instr = instr;
                                completed_instr.instr_result = PipelineInstructionResult::Empty;
                                info!("Pipeline::Memory: Calling execute stage");
                                self.memory = self.pipeline_execute(false);
                                info!(
                                    "Pipeline::Memory: Got new status from execute stage: {:?}",
                                    self.memory
                                );

                                // return instruction with result
                                info!(
                                        "Pipeline::Memory: Passing completed instruction back to writeback: {:?}",
                                        completed_instr
                                    );
                                PipelineStageStatus::Instruction(completed_instr)
                            }
                            Ok(MemResponse::Load(load_resp)) => {
                                info!(
                                    "Pipeline::Memory: Load request returned data: {:?}",
                                    load_resp
                                );
                                let (reg_group, dest_reg) = if let Some(reg_info) =
                                    instr.get_dest_reg()
                                {
                                    info!("Pipeline::Memory: Target register group {}, register {} extracted from instruction {:?}", reg_info.0, reg_info.1, instr);
                                    reg_info
                                } else {
                                    error!("Pipeline::Memory: Failed to extract register group and number information from instruction {:?} (Assumed to be Load)", instr);
                                    panic!("Pipeline::Memory: Failed to extract destination register info from instruction");
                                };
                                let address = req.get_address();
                                let data = load_resp.data.get_contents(address).expect(
                                    "Pipeline::Memory: Failed to extract data from memory response",
                                );

                                let mut completed_instr = instr;
                                completed_instr.instr_result =
                                    PipelineInstructionResult::Register {
                                        reg_group,
                                        dest_reg,
                                        data,
                                    };
                                info!("Pipeline::Memory: Calling execute stage unblocked");
                                self.memory = self.pipeline_execute(false);
                                info!(
                                    "Pipeline::Memory: Got new status from execute stage: {:?}",
                                    self.memory
                                );

                                // return instruction with result
                                info!(
                                        "Pipeline::Memory: Passing completed instruction back to writeback: {:?}",
                                        completed_instr
                                    );
                                PipelineStageStatus::Instruction(completed_instr)
                            }
                            Err(e) => {
                                error!("Pipeline::Memory: Request returned error: {e}");
                                panic!("Pipeline::Memory: Error returned from memory system: {e}");
                            }
                        }
                    } else {
                        // Assuming otherwise we just pass the instruction along...
                        info!(
                            "Pipeline::Memory: No memory action to take for instruction {:?}",
                            self.memory
                        );
                        let completed_instr = instr;
                        self.memory = self.pipeline_execute(false);
                        PipelineStageStatus::Instruction(completed_instr)
                    }
                } else {
                    error!("Pipeline::Memory: Recieved non-decoded instruction in pipeline memory stage");
                    panic!("Pipeline::Memory: Recieved non-decoded instruction in pipeline memory stage");
                }
            }
            PipelineStageStatus::Stall => {
                // if Noop/Stall, do nothing
                info!("Pipeline::Memory: Stall is current state");
                // TODO: temporary stopgap to allow pipeline to not get stuck, this should be true
                self.pipeline_execute(true);
                PipelineStageStatus::Stall
            }
            PipelineStageStatus::Noop => {
                // if Noop/Stall, do nothing
                info!("Pipeline::Memory: Noop is current state");
                self.memory = self.pipeline_execute(false);
                PipelineStageStatus::Noop
            }
        }
    }

    fn pipeline_writeback(&mut self) -> SystemMessage {
        info!(
            "Pipeline::Writeback: Pipeline: In writeback stage, current instruction: {:?}",
            self.writeback
        );
        match self.writeback {
            PipelineStageStatus::Instruction(instr) => {
                info!("Pipeline::Writeback: Have current instruction: {:?}", instr);
                match instr.instr_result {
                    PipelineInstructionResult::Register {
                        reg_group,
                        dest_reg,
                        data,
                    } => {
                        // if W instruction has result
                        //  - write result to registers
                        //  - update pending registers
                        info!(
                            "Pipeline::Writeback: Instruction has register result. Group: {}, Number: {}, Data: {}",
                            reg_group, dest_reg, data
                        );
                        info!("Pipeline::Writeback: Writing result to register");
                        self.registers.write_normal(data, reg_group, dest_reg);
                        info!("Pipeline::Writeback: Updating pending registers");
                        if self.pending_reg.remove(&(reg_group, dest_reg)) {
                            error!(
                                "Pipeline::Writeback: Register group {}, number {} cleared from pending",
                                reg_group, dest_reg
                            );
                        }
                    }
                    PipelineInstructionResult::Branch { new_pc } => {
                        // if W has branch
                        //  - update PC
                        info!(
                            "Pipeline::Writeback: Instruction has branch result. New PC: {}",
                            new_pc
                        );
                        // need to write return address to R15 here in case of call, remove from
                        // pending registers?
                        self.registers.program_counter = new_pc;
                        info!("Pipeline::Writeback: Branch instruction, squashing the rest of the pipeline");
                        // breaking stuff here, causes fetch to skip over an instruction????
                        self.squash();
                    }
                    // only used for CALL instruction?
                    PipelineInstructionResult::JumpSubRoutine {
                        new_pc,
                        ret_reg_val,
                    } => {
                        // if JumpServiceRoutine
                        //  - update PC and return reg
                        info!(
                            "Pipeline::Writeback: Instruction has JSR result. New PC: {}, Return Register Value: {}",
                            new_pc, ret_reg_val
                        );
                        // BUG: double check this logic...
                        self.registers.program_counter = new_pc;
                        let addr_data = MemBlock::Unsigned32(ret_reg_val);
                        self.registers
                            .write_normal(addr_data, RegisterGroup::General, RET_REG);
                        info!("Pipeline::Writeback: Jump Subroutine instruction, squashing the rest of the pipeline");
                        self.squash();
                        self.pending_reg.remove(&(RegisterGroup::General, RET_REG));
                    }
                    PipelineInstructionResult::Flag { flags } => {
                        error!(
                            "Pipeline::Writeback: Instruction {} has flag result: {:?}",
                            instr.decode_instr.unwrap(),
                            flags
                        );
                        // TODO: Handle this...
                        for (idx, val) in flags.iter().enumerate() {
                            if let Some(flag) = val {
                                self.registers.write_status(idx, *flag)
                            }
                        }
                        self.pending_reg.remove(&(RegisterGroup::Flag, 0));
                    }
                    PipelineInstructionResult::Empty => {
                        info!("Pipeline::Writeback: Instruction has empty result, doing nothing");
                    }
                }
            }
            PipelineStageStatus::Stall => {
                // if Noop/Stall, do nothing
                info!("Pipeline::Writeback: Stall is current state");
            }
            PipelineStageStatus::Noop => {
                // if Noop/Stall, do nothing
                info!("Pipeline::Writeback: Noop is current state");
            }
        }

        // call M
        //  - Save instr returned from M for next cycle
        let finished_instr = self.writeback;
        info!("Pipeline::Writeback: Calling memory stage");
        self.writeback = self.pipeline_memory();
        info!(
            "Pipeline::Writeback: Saving message returned from memory stage: {:?}",
            self.writeback
        );
        // if we have a Halt instr, we need to pass that message back to the system
        if let PipelineStageStatus::Instruction(PipelineInstruction {
            decode_instr: Some(Instruction::Type0 { opcode: 1 }),
            ..
        }) = finished_instr
        {
            info!("Passing Halt message");
            SystemMessage::Halt
        } else {
            SystemMessage::InstructionCompleted
        }
    }

    pub fn step(&mut self) -> SystemMessage {
        info!("Starting a system step");
        let msg = if self.should_use_pipeline() {
            self.pipeline_run()
        } else {
            self.run_no_pipeline()
        };
        info!("Updating the memory system's clock");
        self.memory_system.update_clock();
        info!("Incrementing the clock");
        self.clock += 1;
        msg
    }

    fn squash(&mut self) {
        self.memory = PipelineStageStatus::Noop;
        self.execute = PipelineStageStatus::Noop;
        self.decode = PipelineStageStatus::Noop;
        self.fetch = FetchState::default();
        self.memory_system.clear_reqs();
        self.pending_reg.clear();
    }

    // TODO: do this???
    pub fn skip_instruction(&mut self) {
        info!("Starting an instruction step");
        todo!()
    }

    fn should_use_pipeline(&self) -> bool {
        self.should_use_pipeline
    }

    pub fn toggle_pipeline(&mut self) {
        self.should_use_pipeline = !self.should_use_pipeline;
    }

    pub fn get_display_instr_addr(&self) -> Option<usize> {
        if let PipelineStageStatus::Instruction(PipelineInstruction {
            src_addr: Some(src_addr),
            ..
        }) = self.writeback
        {
            Some(src_addr)
        } else if let PipelineStageStatus::Instruction(PipelineInstruction {
            src_addr: Some(src_addr),
            ..
        }) = self.execute
        {
            Some(src_addr)
        } else if let PipelineStageStatus::Instruction(PipelineInstruction {
            src_addr: Some(src_addr),
            ..
        }) = self.memory
        {
            Some(src_addr)
        } else if let PipelineStageStatus::Instruction(PipelineInstruction {
            src_addr: Some(src_addr),
            ..
        }) = self.decode
        {
            Some(src_addr)
        } else if let FetchState {
            src_addr: Some(src_addr),
            ..
        } = self.fetch
        {
            Some(src_addr)
        } else {
            None
        }
    }
}

/// A common object to be passed between pipeline stages
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PipelineStageStatus {
    Instruction(PipelineInstruction),
    Stall,
    Noop,
}

/// Stores instruction results to pass between pipeline stages
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct PipelineInstruction {
    src_addr: Option<usize>,           // address the instruction was fetched from
    raw_instr: Option<RawInstruction>, // the instruction as stored in memory
    decode_instr: Option<Instruction>, // the decoded instruction
    instr_result: PipelineInstructionResult, // the result of executing this instruction
}
impl PipelineInstruction {
    /// Returns the target register group and number, if applicable
    /// TODO: Clean up flag registers for comparisons...
    pub fn get_dest_reg(&self) -> Option<(RegisterGroup, usize)> {
        match self.decode_instr {
            Some(Instruction::Type1 { opcode, .. }) => {
                if opcode == 0 {
                    Some((RegisterGroup::General, RET_REG))
                } else {
                    None
                }
            }
            Some(Instruction::Type2 { opcode: 0..=2, .. } | Instruction::Type3 { .. }) => {
                Some((RegisterGroup::Flag, 0))
            }
            Some(
                Instruction::Type2 {
                    opcode: 3..=5,
                    reg_1,
                    ..
                }
                | Instruction::Type5 { reg_1, .. },
            ) => Some((RegisterGroup::General, reg_1)),
            Some(Instruction::Type0 { .. } | Instruction::Type2 { .. }) | None => None,
            Some(Instruction::Type4 { opcode, reg_1, .. }) => match opcode {
                0 | 1 | 2 | 3 | 4 | 5 | 9 => Some((RegisterGroup::General, reg_1)),
                _ => None,
            },
            Some(Instruction::Type6 { freg_1, .. }) => Some((RegisterGroup::FloatingPoint, freg_1)),
        }
    }
}
