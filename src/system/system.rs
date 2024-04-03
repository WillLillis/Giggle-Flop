use crate::common::PipelineStage;
use crate::execution::execution_state::ExecutionState;
use crate::memory::memory_system::{LoadRequest, MemRequest, MemResponse, MemWidth, Memory, StoreRequest};
use crate::pipeline::execute::PipelineExecute;
use crate::pipeline::instruction::{
    Instruction, InstructionResult, InstructionState, RawInstruction,
};
use crate::pipeline::memory::PipelineMemory;
use crate::pipeline::pipeline::{PipeLine, PipelineState};
use crate::pipeline::write_back::PipelineWriteBack;
use crate::register::register_system::RegisterSet;

use crate::memory::memory_system::MemBlock;
use crate::pipeline::decode::PipelineDecode;
use crate::pipeline::fetch::PipelineFetch;

use anyhow::Result;

pub struct System {
    pub clock: usize,
    pub pipeline: PipeLine,
    pub memory_system: Memory,
    pub registers: RegisterSet,
    pub execution_state: ExecutionState,
    // Pipeline v
    // all the pipeline stages...
    fetch: PipelineFetch,
    decode: PipelineDecode,
    execute: PipelineExecute,
    memory: PipelineMemory,
    write_back: PipelineWriteBack,
    // for shared state between stages if necessary...
    pipeline_state: PipelineState,
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
        memory_system.force_store(128, MemBlock::Bits32(1));
        let load_instr = 0b00000000000001000000000010010100;
        let add_instr = 0b00000000000000011001000010001101;
        let tmp_add_instr = Instruction::from(add_instr);
        let tmp_load_instr = Instruction::from(load_instr);
        println!("HEY RIGHT HERE {:?}", tmp_add_instr);
        println!("HEY RIGHT HERE {:?}", tmp_load_instr);
        memory_system.force_store(0, MemBlock::Bits32(add_instr));

        Self {
            clock: 0,
            pipeline: PipeLine::default(),
            // memory_system: Memory::new(4, &[32, 64], &[1, 5]),
            memory_system,
            registers: RegisterSet::new(),
            execution_state: ExecutionState::default(),
            fetch: PipelineFetch::default(),
            decode: PipelineDecode::default(),
            execute: PipelineExecute::default(),
            memory: PipelineMemory::default(),
            write_back: PipelineWriteBack::default(),
            pipeline_state: PipelineState::default(),
        }
    }

    fn pipeline_run(&mut self) {
        self.pipeline_write_back()
    }

    fn pipeline_fetch(&mut self, decode_blocked: bool, execute_blocked: bool) -> InstructionState {
        println!("start fetch");
        // if no current instruction -> send load to cache with PC as address
        if self.fetch.instruction.instr == None {
            let pc = self.registers.program_counter;
            let request = MemRequest::Load(LoadRequest {
                issuer: PipelineStage::Fetch,
                address: self.registers.program_counter,
                width: MemWidth::Bits32,
            });
            let resp = self.memory_system.request(&request).unwrap();
            match resp {
                MemResponse::Miss | MemResponse::Wait => {
                    // return noop/stall
                    self.fetch.instruction.stall = true;
                    println!("fetch stall");
                    return self.fetch.instruction.clone();
                }
                MemResponse::Load(load_resp) => {
                    // if cache returns value -> set current instruction
                    // TODO: Check if unsigned, signed, or float result, set fetch's
                    // value accordingly
                    // let mem_type = self.execute.instruction.get_mem_type();
                    let val = load_resp.data.get_contents(pc).unwrap().get_data();
                    self.fetch.instruction.val =
                        Some(InstructionResult::UnsignedIntegerResult {
                            dest: pc,
                            val,
                        })
                }
                MemResponse::Store => unreachable!(),
            }
        }
        // if no current instruction or decode blocked -> return noop/stall
        if self.fetch.instruction.instr == None  || decode_blocked {
            self.fetch.instruction.stall = true;
        }
        // if current instruction & decode not blocked -> return instruction, increment PC
        if self.fetch.instruction.instr != None && !decode_blocked {
            self.registers.program_counter += 32;
        }
        println!("end fetch");
        return self.fetch.instruction.clone()
    }

    fn pipeline_decode(&mut self, execute_blocked: bool, memory_blocked: bool) -> InstructionState {
        println!("start decode");
        // TODO: split instruction into fields
        // if source regs not pending -> get values, create instruction object
        //      call fetch with blocked status from execute
        // if register values pending -> call fetch with blocked
        let has_operands = true;
        // save instruction from fetch as next instruction
        let mut state = self.pipeline_fetch(false, execute_blocked);
        self.decode.instruction = state;
        // if instruction has operands and execute not blocked -> put dest register in pending, return instruction object to E
        // if instruction missing operands or execute blocked-> return noop/stall
        println!("end decode");
        if has_operands && !execute_blocked {
            return self.decode.instruction.clone();
        }
        if !has_operands || execute_blocked {
            self.decode.instruction.stall = true;   
        }
        return self.decode.instruction.clone();
    }

    fn pipeline_execute(&mut self, memory_blocked: bool) -> InstructionState {
        println!("start execute");
        if self.execute.instruction.stall {
            // if noop -> do nothing
        } else {
            // if ALU op -> do op
            // if jump -> get address
            // if jump subroutine -> get PC, get address
            // if branch -> check condition, set flag, calculate target address
            // if memory -> do address calculation
            if let Some(instr) = self.execute.instruction.instr {
                if instr.is_alu_instr() {
                    // TODO: fix hardcoding
                    if let Instruction::Type5 {
                        opcode: 1,
                        reg_1: 1,
                        reg_2: 2,
                        reg_3: 3
                    } = instr {
                    }
                }
            }
        }
        // call decode with blocked status from memory
        let decode_instr = self.pipeline_decode(false, true);

        // if memory not blocked -> return instruction object to memory with result
        // if memory blocked -> return noop/stall
        if memory_blocked {
            self.execute.instruction.stall = true;
        } else {
            self.execute.instruction.val = Some(InstructionResult::UnsignedIntegerResult {
                dest: 3 as usize,
                val: 1,
            });
        }        
        // save instruction from decode as next instruction
        self.execute.instruction = decode_instr;
        println!("end execute");
        return self.execute.instruction.clone();

    }

    // BUG: Where do we return the instruction to writeback???
    fn pipeline_memory(&mut self) -> InstructionState {
        println!("start memory");
        let mut exec_instr: Option<InstructionState> = None;
        // if noop/nonmem instruction -> do nothing
        if let Some(instr) = self.memory.instruction.instr {
            if instr.is_mem_instr() {
                // if load -> call cache
                if instr.is_load_instr() {
                    let width = instr.get_mem_width().unwrap();
                    // TODO: also support Type 2 loads
                    if let Instruction::Type4 {
                        opcode,
                        reg_1,
                        immediate,
                        ..
                    } = instr
                    {
                        // destructure to grab immediate field
                        let request = MemRequest::Load(LoadRequest {
                            issuer: PipelineStage::Memory,
                            address: immediate as usize,
                            width,
                        });
                        let resp = self.memory_system.request(&request).unwrap();
                        // in cache -> if hit and no delay -> cache returns value
                        //          -> if hit and delay/miss -> cache return wait
                        //          -> if miss -> cache calls memory
                        // in memory -> return value or wait
                        //          -> if value -> update cache
                        //          -> if whole process behind that in slides
                        //          -> if store -> send data, address to cache, update accordingly
                        // if value returned -> call execute with not blocked
                        // else call execute with blocked
                        match resp {
                            MemResponse::Miss | MemResponse::Wait => {
                                exec_instr = Some(self.pipeline_execute(true));
                            }
                            MemResponse::Load(load_resp) => {
                                // if value returned -> call execute with not blocked
                                // TODO: Check if unsigned, signed, or float result, set execute's
                                // value accordingly
                                // let mem_type = self.execute.instruction.get_mem_type();
                                let val = load_resp
                                    .data
                                    .get_contents(immediate as usize)
                                    .unwrap()
                                    .get_data();
                                self.execute.instruction.val =
                                    Some(InstructionResult::UnsignedIntegerResult {
                                        dest: reg_1 as usize,
                                        val,
                                    });
                                    exec_instr = Some(self.pipeline_execute(false));
                            }
                            MemResponse::Store => unreachable!(),
                        }
                    }
                }
                // if store -> send data, address to cache
                if instr.is_store_instr() {
                    // TODO: width is prob still relevant idk where tho
                    let width = instr.get_mem_width().unwrap();
                    if let Instruction::Type4 {
                        opcode,
                        reg_1,
                        immediate,
                        ..
                    } = instr
                    {
                        let data = if let InstructionResult::AddressResult { ref addr } = self.memory.instruction.val.as_ref().unwrap() {
                            MemBlock::Bits32(addr.clone())
                        } else {
                            panic!("Bad result");
                        };
                        // destructure to grab immediate field
                        let request = MemRequest::Store(StoreRequest {
                            issuer: PipelineStage::Memory,
                            address: immediate as usize,
                            data,
                        });
                        let resp = self.memory_system.request(&request).unwrap();
                        match resp {
                            MemResponse::Wait => {
                                self.execute.instruction.stall = true;
                                exec_instr = Some(self.pipeline_execute(true));
                            }
                            MemResponse::Store => {
                                // if value returned -> call execute with not blocked
                                exec_instr = Some(self.pipeline_execute(false));
                            }
                            MemResponse::Miss | MemResponse::Load(_) => unreachable!(),
                        }
                    }
                }
            } else {
                // do nothing
            }
        }
        exec_instr = Some(self.pipeline_execute(false));
        if let Some(instruction) = exec_instr {
            if let Some(exec_instr) = instruction.instr {
                // if instruction isnt load/store -> return to write_back forwarding instruction
                if !exec_instr.is_mem_instr() {
                    return instruction;
                } else {
                    // if instruction is load/store ->
                    //      if cache returns wait -> return to write_back with noop/stall
                    //      if cache returns value -> put value in instruction result and return to write_back
                    if exec_instr.is_load_instr() {
                        let width = exec_instr.get_mem_width().unwrap();
                        // TODO: also support Type 2 loads
                        if let Instruction::Type4 {
                            opcode,
                            reg_1,
                            immediate,
                            ..
                        } = exec_instr
                        {
                            // destructure to grab immediate field
                            let request = MemRequest::Load(LoadRequest {
                                issuer: PipelineStage::Memory,
                                address: immediate as usize,
                                width,
                            });
                            let resp = self.memory_system.request(&request).unwrap();
                            match resp {
                                MemResponse::Miss | MemResponse::Wait => {
                                    self.memory.instruction.stall = true;
                                    return self.memory.instruction.clone();
                                }
                                MemResponse::Load(load_resp) => {
                                    // TODO: Check if unsigned, signed, or float result, set execute's
                                    // value accordingly
                                    // let mem_type = self.execute.instruction.get_mem_type();
                                    let val = load_resp
                                        .data
                                        .get_contents(immediate as usize)
                                        .unwrap()
                                        .get_data();
                                    self.memory.instruction.val =
                                        Some(InstructionResult::UnsignedIntegerResult {
                                            dest: reg_1 as usize,
                                            val,
                                        });
                                    return self.memory.instruction.clone();
                                }
                                MemResponse::Store => unreachable!(),
                            }
                        }
                    }
                    if exec_instr.is_store_instr() {
                        // TODO: width is prob still relevant idk where tho
                        let width = exec_instr.get_mem_width().unwrap();
                        if let Instruction::Type4 {
                            immediate,
                            ..
                        } = exec_instr
                        {
                            let data = if let InstructionResult::AddressResult { addr } = instruction.val.unwrap() {
                                MemBlock::Bits32(addr)
                            } else {
                                panic!("Bad result");
                            };
                            // destructure to grab immediate field
                            let request = MemRequest::Store(StoreRequest {
                                issuer: PipelineStage::Memory,
                                address: immediate as usize,
                                data,
                            });
                            let resp = self.memory_system.request(&request).unwrap();
                            match resp {
                                MemResponse::Wait => {
                                    self.memory.instruction.stall = true;
                                    return self.memory.instruction.clone();
                                }
                                MemResponse::Store => {
                                    return self.memory.instruction.clone();
                                }
                                MemResponse::Miss | MemResponse::Load(_) => unreachable!(),
                            }
                        } 
                    }
                }
            }
        }
        println!("end memory");
        return self.memory.instruction.clone();
    }

    fn pipeline_write_back(&mut self) {
        // if saved instruction has result -> write to reg, update pending regs
        println!("start write_back");
        if !self.write_back.instruction.stall {
            match self.write_back.instruction.val {
                Some(InstructionResult::UnsignedIntegerResult { dest, val }) => {
                    self.registers.general[dest].write_block_unsigned(MemBlock::Bits32(val));
                }
                Some(InstructionResult::IntegerResult { dest, val }) => {
                    let bytes = val.to_be_bytes();
                    let conv = u32::from_be_bytes(bytes);
                    self.registers.general[dest].write_block_signed(MemBlock::Bits32(conv));
                }
                Some(InstructionResult::FloatResult { dest, val }) => {
                    let bytes = val.to_be_bytes();
                    let conv = u32::from_be_bytes(bytes);
                    self.registers.float[dest].write_block(MemBlock::Bits32(conv));
                }
                Some(InstructionResult::AddressResult { addr }) => {
                    // if W has branch -> update PC
                    // if jump subroutine -> update PC and return reg
                    // TODO: Lol
                }
                None => {
                    // if noop/stall -> do nothing
                }
            }
        }
        self.write_back.instruction = self.pipeline_memory();
        println!("end write_back");
        // return to clock
    }

    pub fn step(&mut self) {
        self.pipeline_run();
        self.clock += 1;
    }
}
