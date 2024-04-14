use std::collections::HashSet;

use log::{error, info};

use crate::common::PipelineStage;
use crate::memory::memory_system::{
    LoadRequest, LoadResponse, MemRequest, MemResponse, MemType, Memory, MEM_BLOCK_WIDTH,
};
use crate::pipeline::instruction::{decode_raw_instr, Instruction, RawInstruction};
use crate::register::register_system::{
    get_comparison_flags, RegisterGroup, RegisterSet, FLAG_COUNT, RET_REG,
};

use crate::memory::memory_system::MemBlock;

pub struct System {
    pub clock: usize,
    pub memory_system: Memory,
    pub registers: RegisterSet,
    // Pipeline v
    pub fetch: Option<u32>,
    pub decode: PipelineStageStatus,
    pub execute: PipelineStageStatus,
    pub memory: PipelineStageStatus,
    pub writeback: PipelineStageStatus,
    pub pending_reg: HashSet<(RegisterGroup, usize)>,
}

// TODO: Figure out what these todo comments mean (from writeback)
// TODO: clock increments cycles counter
// TODO: begin new cycle
impl System {
    // For debugging purposes, will need to make this
    // configurable later...
    pub fn default() -> Self {
        let mut memory_system = Memory::new(4, &[32, 64], &[1, 2]);
        // Load up a sample program
        // we will simply add two numbers inside two registers
        memory_system.force_store(128, MemBlock::Unsigned32(1));
        let load_instr = 0b00000000000001000000000010010100;
        let add_instr = 0b00000000000000011001000010001101;
        let tmp_add_instr = decode_raw_instr(add_instr);
        let tmp_load_instr = decode_raw_instr(load_instr);
        println!("HEY RIGHT HERE {:?}", tmp_add_instr);
        println!("HEY RIGHT HERE {:?}", tmp_load_instr);
        memory_system.force_store(0, MemBlock::Unsigned32(add_instr));

        Self {
            clock: 0,
            pending_reg: HashSet::new(),
            // memory_system: Memory::new(4, &[32, 64], &[1, 5]),
            memory_system,
            registers: RegisterSet::new(),
            fetch: None,
            decode: PipelineStageStatus::Noop,
            execute: PipelineStageStatus::Noop,
            memory: PipelineStageStatus::Noop,
            writeback: PipelineStageStatus::Noop,
        }
    }

    fn pipeline_run(&mut self) {
        self.pipeline_writeback()
    }

    fn pipeline_fetch(&mut self, decode_blocked: bool) -> PipelineStageStatus {
        info!(
            "Pipeline: In fetch stage, current PC: {}, current instruction: {:?}",
            self.registers.program_counter, self.fetch
        );
        match (self.fetch, decode_blocked) {
            (None, _) => {
                // If no current instruction, send load to cache with PC as address
                let req = MemRequest::Load(LoadRequest {
                    issuer: PipelineStage::Fetch,
                    address: self.registers.program_counter as usize,
                    width: MemType::Unsigned32,
                });
                info!(
                    "No current instruction, issuing fetch to memory subsystem: {:?}",
                    req
                );
                // TODO: Lots of cleanup here with the memory system
                let resp = self.memory_system.request(&req);
                info!("Pipeline::Fetch: Memory subsystem response: {:?}", resp);
                match resp {
                    Ok(MemResponse::Load(LoadResponse { data })) => {
                        info!(
                            "Got valid load response, incrementing the PC from {} to {}",
                            self.registers.program_counter,
                            self.registers.program_counter + MEM_BLOCK_WIDTH as u32
                        );
                        self.registers.program_counter += MEM_BLOCK_WIDTH as u32;
                        // match data.get_contents(self.registers.program_counter as usize) {
                        match data.get_contents(req.get_address()) {
                            Some(conts) => {
                                let raw = match conts {
                                    MemBlock::Unsigned8(data) => {
                                        error!(
                                            "Received u8 for instruction fetch, translating to u32"
                                        );
                                        data as u32
                                    }
                                    MemBlock::Unsigned16(data) => {
                                        error!("Received u16 for instruction fetch, translating to u32");
                                        data as u32
                                    }
                                    MemBlock::Unsigned32(data) => data,
                                    MemBlock::Signed8(data) => {
                                        error!(
                                            "Received i8 for instruction fetch, translating to u32"
                                        );
                                        data as u32
                                    }
                                    MemBlock::Signed16(data) => {
                                        error!("Received i16 for instruction fetch, translating to u32");
                                        data as u32
                                    }
                                    MemBlock::Signed32(data) => {
                                        error!("Received i32 for instruction fetch, translating to u32");
                                        data as u32
                                    }
                                    MemBlock::Float32(data) => {
                                        error!("Received f32 for instruction fetch, translating to u32");
                                        data as u32
                                    }
                                };

                                let decoded =
                                    PipelineStageStatus::Instruction(PipelineInstruction {
                                        raw_instr: Some(raw),
                                        decode_instr: None,
                                        instr_result: PipelineInstructionResult::EmptyResult,
                                    });
                                info!(
                                    "Pipeline::Fetch: Passing on raw instruction: {:?}",
                                    decoded
                                );
                                return decoded;
                            }
                            None => {
                                error!("Received empty memory response, treating as a NOOP");
                                return PipelineStageStatus::Noop;
                            }
                        }
                    }
                    Ok(MemResponse::Miss) => {
                        info!("Pipeline::Fetch: Request missed");
                        return PipelineStageStatus::Stall;
                    }
                    Ok(MemResponse::Wait) => {
                        info!("Pipeline::Fetch: Request got wait");
                        return PipelineStageStatus::Stall;
                    }
                    Ok(MemResponse::StoreComplete) => {
                        error!("Got StoreComplete response for fetch request");
                        return PipelineStageStatus::Stall;
                    }
                    Err(e) => {
                        error!("Got error {e} from memory subsystem, translating into NOOP");
                        return PipelineStageStatus::Noop;
                    }
                }
            }
            (Some(instr), false) => {
                info!(
                    "Have instruction {:?}, decode is unblocked, returning instruction result",
                    instr
                );
                return PipelineStageStatus::Instruction(PipelineInstruction {
                    raw_instr: self.fetch,
                    decode_instr: None,
                    instr_result: PipelineInstructionResult::EmptyResult,
                });
            }
            (Some(instr), true) => {
                info!(
                    "Have instruction {:?}, decode is blocked, returning NOOP",
                    instr
                );
                return PipelineStageStatus::Noop;
            }
        }
    }

    // NOTE: Keep in mind, execute needs to pass along blocked status to D from M
    fn pipeline_decode(&mut self, mem_blocked: bool) -> PipelineStageStatus {
        info!(
            "Pipeline: In decode stage, current instruction: {:?}, memory blocked: {}",
            self.decode, mem_blocked
        );
        let mut pending_regs = false;
        match self.decode {
            PipelineStageStatus::Instruction(ref mut instruction) => {
                if let Some(raw) = instruction.raw_instr {
                    // split instruction into fields
                    match decode_raw_instr(raw) {
                        Some(instr) => {
                            instruction.decode_instr = Some(instr);
                            let src_regs = instr.get_src_regs();
                            pending_regs =
                                src_regs.iter().any(|src| self.pending_reg.contains(src));
                            info!("Pipeline::Decode: Pending registers: {pending_regs}");
                            // TODO:
                            // Add logging here...
                        }
                        None => {
                            error!("Failed to decode raw instruction {raw}, passing on a NOOP");
                            self.decode = PipelineStageStatus::Noop;
                        }
                    };
                } else {
                    error!("Received empty raw instruction field, passing on a NOOP");
                    self.decode = PipelineStageStatus::Noop;
                }
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
        match (pending_regs, mem_blocked) {
            // instruction missing operands OR execute is blocked
            (true, _) | (_, true) => {
                info!("Pipeline::Decode: Calling fetch with blocked status");
                // BUG: Later stages incorrectly get stuck in blocked state, we ignore every
                // instruction coming out of fetch...
                self.pipeline_fetch(true); // shouldn't get anything back because we're blocked...
                info!("Pipeline::Decode: Passing on a Stall status");
                return PipelineStageStatus::Stall;
            }
            // instruction has operands, memory (execute) not blocked
            (false, false) => {
                let completed_instr = self.decode;
                info!("Pipeline::Decode: Calling fetch with unblocked status");
                self.decode = self.pipeline_fetch(mem_blocked);
                info!(
                    "Pipeline::Decode: Instruction saved for next decode: {:?}",
                    self.decode
                );
                // pattern matching?
                if let PipelineStageStatus::Instruction(instr) = completed_instr {
                    if let Some(reg) = instr.get_dest_reg() {
                        info!(
                            "Pipeline::Decode: Inserting {:?} into pending registers",
                            reg
                        );
                        self.pending_reg.insert(reg);
                    }
                }
                // BUG: Issue is here???
                info!(
                    "Returning decoded instruction {:?} to execute",
                    completed_instr
                );
                return completed_instr;
            }
        }
    }

    // NOTE: Make sure to set flag status in result for all ALU ops...
    fn pipeline_execute(&mut self, mem_blocked: bool) -> PipelineStageStatus {
        info!(
            "Pipeline: In execute stage, current instruction: {:?}, memory blocked: {}",
            self.execute, mem_blocked
        );
        // execute appears to pass along a more "filled in" instruction object, look into this...
        match self.execute {
            PipelineStageStatus::Instruction(mut instr) => {
                info!("Pipeline::Execute: Have current instruction: {:?}", instr);
                match instr.decode_instr {
                    Some(ref mut instruction) => match instruction {
                        Instruction::Type0 { opcode } => {
                            info!("Pipeline::Execute: No work to be done, empty result");
                            instr.instr_result = PipelineInstructionResult::EmptyResult;
                        }
                        Instruction::Type1 { opcode, immediate } => {
                            info!("Pipeline::Execute: No work to be done, empty result");
                        }
                        Instruction::Type2 {
                            opcode,
                            reg_1,
                            reg_2,
                        } => match opcode {
                            0 | 1 | 2 => {
                                info!("Pipeline::Execute: Comparing general registers {reg_1} and {reg_2}");
                                let flags = get_comparison_flags(
                                    self.registers.general[*reg_1],
                                    self.registers.general[*reg_2],
                                );
                                instr.instr_result =
                                    PipelineInstructionResult::FlagResult { flags };
                            }
                            _ => {
                                instr.instr_result = PipelineInstructionResult::EmptyResult;
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
                            instr.instr_result = PipelineInstructionResult::FlagResult { flags };
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
                                instr.instr_result = PipelineInstructionResult::RegisterResult {
                                    reg_group: RegisterGroup::General,
                                    dest_reg: *reg_1,
                                    data,
                                }
                            }
                            _ => {
                                instr.instr_result = PipelineInstructionResult::EmptyResult;
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                _ => {
                                    instr.instr_result = PipelineInstructionResult::EmptyResult;
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
                                        reg_group: RegisterGroup::FloatingPoint,
                                        dest_reg: *freg_1,
                                        data,
                                    }
                                }
                                _ => {
                                    instr.instr_result = PipelineInstructionResult::EmptyResult;
                                    info!("Pipeline::Execute: Nothing to do here",);
                                }
                            }
                        }
                    },
                    None => {
                        error!("Received non-decoded instruction in execute stage");
                        panic!("Non-decoded instruction encountered in execute stage");
                    }
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

        // BUG: Look here for blocked issue?
        // Don't need to check if we're blocked here by pending registers?
        // if memory blocked, return Noop/Stall
        if mem_blocked {
            self.pipeline_decode(mem_blocked);
            PipelineStageStatus::Stall
        } else {
            // if memory not blocked, return instruction object with result to memory
            let completed_instr = self.execute; // TODO: Fill in result for this...
            self.execute = self.pipeline_decode(mem_blocked);
            completed_instr
        }
    }

    #[must_use]
    fn pipeline_memory(&mut self) -> PipelineStageStatus {
        info!(
            "Pipeline::Memory: Pipeline: In memory stage, current instruction: {:?}",
            self.memory
        );
        match self.memory {
            PipelineStageStatus::Instruction(instr) => {
                info!("Pipeline::Memory: Have current instruction: {:?}", instr);
                match instr.decode_instr {
                    Some(instruction) => {
                        if let Some(req) = instruction.get_mem_req(Some(PipelineStage::Memory)) {
                            // If load, call memory system
                            //  - if hit and delay or miss, get wait back
                            //      - assuming we have to pass the Wait/Stall along...
                            // If value returned, call E non-blocked
                            // If wait returned, call E with blocked
                            info!(
                                "Associated memory request: {:?}, issuing to memory system",
                                req
                            );
                            let resp = self.memory_system.request(&req);
                            info!(
                                "Pipeline::Memory: Got {:?} response from memory system",
                                resp
                            );
                            match resp {
                                Ok(MemResponse::Miss) | Ok(MemResponse::Wait) => {
                                    // if not blocked, return instruction with result
                                    // if blocked, return Noop/ Stall
                                    info!("Pipeline::Memory: Calling execute with memory blocked");
                                    // BUG: Make sure this doesn't return anything?
                                    self.pipeline_execute(true);
                                    info!("Pipeline::Memory: Returning stall status to writeback");
                                    return PipelineStageStatus::Stall;
                                }
                                Ok(MemResponse::StoreComplete) => {
                                    info!("Pipeline::Memory: Store request returned StoreComplete status");
                                    let mut completed_instr = instr;
                                    completed_instr.instr_result =
                                        PipelineInstructionResult::EmptyResult;
                                    info!("Pipeline::Memory: Calling execute stage");
                                    self.memory = self.pipeline_execute(false);
                                    info!(
                                        "Pipeline::Memory: Got new status from execute stage: {:?}",
                                        self.memory
                                    );

                                    // return instruction with result
                                    info!(
                                        "Passing completed instruction back to writeback: {:?}",
                                        completed_instr
                                    );
                                    return PipelineStageStatus::Instruction(completed_instr);
                                }
                                Ok(MemResponse::Load(load_resp)) => {
                                    info!(
                                        "Pipeline::Memory: Load request returned data: {:?}",
                                        load_resp
                                    );
                                    let (reg_group, dest_reg) = match instr.get_dest_reg() {
                                        Some(reg_info) => {
                                            info!("Pipeline::Memory: Target register group {}, register {} extracted from instruction {:?}", reg_info.0, reg_info.1, instr);
                                            reg_info
                                        }
                                        None => {
                                            error!("Failed to extract register group and number information from instruction {:?} (Assumed to be Load)", instr);
                                            panic!("Failed to extract destination register info from instruction");
                                        }
                                    };
                                    let address = req.get_address();
                                    let data = load_resp
                                        .data
                                        .get_contents(address)
                                        .expect("Failed to extract data from memory response");

                                    let mut completed_instr = instr;
                                    completed_instr.instr_result =
                                        PipelineInstructionResult::RegisterResult {
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
                                        "Passing completed instruction back to writeback: {:?}",
                                        completed_instr
                                    );
                                    return PipelineStageStatus::Instruction(completed_instr);
                                }
                                Err(e) => {
                                    error!("Request returned error: {e}");
                                    panic!("Error returned from memory system: {e}");
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
                            return PipelineStageStatus::Instruction(completed_instr);
                        }
                    }
                    None => {
                        error!("Recieved non-decoded instruction in pipeline memory stage");
                        panic!("Recieved non-decoded instruction in pipeline memory stage");
                    }
                }
            }
            PipelineStageStatus::Stall => {
                // if Noop/Stall, do nothing
                info!("Pipeline::Memory: Stall is current state");
                self.memory = self.pipeline_execute(true);
                return PipelineStageStatus::Stall;
            }
            PipelineStageStatus::Noop => {
                // if Noop/Stall, do nothing
                info!("Pipeline::Memory: Noop is current state");
                self.memory = self.pipeline_execute(false);
                return PipelineStageStatus::Noop;
            }
        }
    }

    fn pipeline_writeback(&mut self) {
        info!(
            "Pipeline::Writeback: Pipeline: In writeback stage, current instruction: {:?}",
            self.writeback
        );
        match self.writeback {
            PipelineStageStatus::Instruction(instr) => {
                info!("Pipeline::Writeback: Have current instruction: {:?}", instr);
                match instr.instr_result {
                    PipelineInstructionResult::RegisterResult {
                        reg_group,
                        dest_reg,
                        data,
                    } => {
                        // if W instruction has result
                        //  - write result to registers
                        //  - update pending registers
                        info!(
                            "Instruction has register result. Group: {}, Number: {}, Data: {}",
                            reg_group, dest_reg, data
                        );
                        info!("Pipeline::Writeback: Writing result to register");
                        self.registers.write_normal(data, reg_group, dest_reg);
                        info!("Pipeline::Writeback: Updating pending registers");
                        if self.pending_reg.remove(&(reg_group, dest_reg)) {
                            info!(
                                "Register group {}, number {} cleared from pending",
                                reg_group, dest_reg
                            );
                        }
                    }
                    PipelineInstructionResult::BranchResult { new_pc } => {
                        // if W has branch
                        //  - update PC
                        info!(
                            "Pipeline::Writeback: Instruction has branch result. New PC: {}",
                            new_pc
                        );
                        self.registers.program_counter = new_pc;
                    }
                    PipelineInstructionResult::JSRResult {
                        new_pc,
                        ret_reg_val,
                    } => {
                        // if JumpServiceRoutine
                        //  - update PC and return reg
                        info!(
                            "Instruction has JSR result. New PC: {}, Return Register Value: {}",
                            new_pc, ret_reg_val
                        );
                        self.registers.program_counter = new_pc;
                        let addr_data = MemBlock::Unsigned32(ret_reg_val);
                        self.registers
                            .write_normal(addr_data, RegisterGroup::General, RET_REG);
                    }
                    PipelineInstructionResult::FlagResult { flags } => {
                        info!(
                            "Pipeline::Writeback: Instruction has flag result: {:?}",
                            flags
                        );
                        // TODO: Handle this...
                    }
                    PipelineInstructionResult::EmptyResult => {
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
        info!("Pipeline::Writeback: Calling memory stage");
        self.writeback = self.pipeline_memory();
        info!(
            "Saving message returned from memory stage: {:?}",
            self.writeback
        );
    }

    pub fn step(&mut self) {
        self.pipeline_run();
        self.memory_system.update_clock();
        self.clock += 1;
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
    raw_instr: Option<RawInstruction>, // the instruction as stored in memory
    decode_instr: Option<Instruction>, // the decoded instruction
    instr_result: PipelineInstructionResult, // the result of executing this instruction
}
impl PipelineInstruction {
    /// Returns the target register group and number, if applicable
    pub fn get_dest_reg(&self) -> Option<(RegisterGroup, usize)> {
        match self.decode_instr {
            Some(Instruction::Type0 { .. })
            | Some(Instruction::Type1 { .. })
            | Some(Instruction::Type3 { .. }) => None,
            Some(Instruction::Type2 { opcode, reg_1, .. }) => match opcode {
                3 | 4 | 5 => Some((RegisterGroup::General, reg_1)),
                _ => None,
            },
            Some(Instruction::Type4 {
                opcode,
                reg_1,
                immediate,
            }) => match opcode {
                0 | 1 | 2 | 3 | 4 | 5 | 9 => Some((RegisterGroup::General, reg_1)),
                _ => None,
            },
            Some(Instruction::Type5 { reg_1, .. }) => Some((RegisterGroup::General, reg_1)),
            Some(Instruction::Type6 { freg_1, .. }) => Some((RegisterGroup::FloatingPoint, freg_1)),
            None => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PipelineInstructionResult {
    RegisterResult {
        reg_group: RegisterGroup,
        dest_reg: usize,
        data: MemBlock,
    },
    BranchResult {
        new_pc: u32,
    },
    JSRResult {
        new_pc: u32,
        ret_reg_val: u32, // return register should be by convention, this is just the address
                          // value to store in it
    },
    FlagResult {
        flags: [Option<bool>; FLAG_COUNT],
    },
    EmptyResult, // indicate an operation was completed, but there's no data to show for it (e.g.
                 // a store to memory)
}
