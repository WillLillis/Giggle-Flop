use std::intrinsics::unreachable;

use crate::common::PipelineStage;
use crate::execution::execution_state::ExecutionState;
use crate::memory::memory_system::{LoadRequest, MemRequest, MemResponse, Memory};
use crate::memory::MemWidth;
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
        Self {
            clock: 0,
            pipeline: PipeLine::default(),
            memory_system: Memory::new(4, &[32, 64], &[1, 5]),
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

    fn pipeline_start(&mut self) {
        self.pipeline_write_back()
    }

    fn pipeline_fetch(&mut self, decode_blocked: bool, execute_blocked: bool) -> InstructionState {
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
                    return self.fetch.instruction;
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
            self.registers.program_counter += 1;
        }
        return self.fetch.instruction
    }

    fn pipeline_decode(&mut self, execute_blocked: bool, memory_blocked: bool) -> InstructionState {
        match self.pipeline_fetch() {
            Some(instr) => {
                Some(Instruction::from(instr))
            },
            None => None
        }
    }

    fn pipeline_execute(&mut self, memory_blocked: bool) -> InstructionState {
        // if noop -> do nothing
        if let Some(instr) = self.execute.instruction.instr {}
        // if ALU op -> do op
        // if jump -> get address
        // if jump subroutine -> get PC, get address
        // if branch -> check condition, set flag, calculate target address
        // if memory -> do address calculation
        // call decode with blocked status from memory
        // if instr.stall {
        // call decode?
        // }
        // if instr.instruction == None {
        //     panic!("this shouldnt happen probably")
        // }
        // let instruction = instr.instruction.unwrap();
        // check ops here idk how
        // Ok(())
        // if memory not blocked -> return instruction object to memory with result
        // if memory blocked -> return noop/stall
        // save instruction from decode as next instruction
        todo!()
    }

    // BUG: Where do we return the instruction to writeback???
    fn pipeline_memory(&mut self) -> InstructionState {
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
                                self.pipeline_execute(true);
                            }
                            MemResponse::Load(load_resp) => {
                                // if value returned -> call execute with not blocked
                                // TODO: Check if unsigned, signed, or float result, set execute's
                                // value accordingly
                                // let mem_type = self.execute.instruction.get_mem_type();
                                let val = load_resp.data.get_contents(immediate as usize).unwrap().get_data();
                                self.execute.instruction.val =
                                    Some(InstructionResult::UnsignedIntegerResult {
                                        dest: reg_1 as usize,
                                        val,
                                    });
                                self.pipeline_execute(false);
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
                        // destructure to grab immediate field
                        let request = MemRequest::Store(StoreRequest {
                            issuer: PipelineStage::Memory,
                            address: immediate as usize,
                            data: self.memory.instruction.val,
                        });
                        let resp = self.memory_system.request(&request).unwrap();
                        match resp {
                            MemResponse::Wait => {
                                self.execute.instruction.stall = true;
                                self.pipeline_execute(true)
                            }
                            MemResponse::Store => {
                                // if value returned -> call execute with not blocked
                                // TODO: Check if unsigned, signed, or float result, set execute's
                                // value accordingly
                                // let mem_type = self.execute.instruction.get_mem_type();
                                let val = load_resp.data.get_contents(immediate as usize).unwrap().get_data();
                                self.execute.instruction.val =
                                    Some(InstructionResult::UnsignedIntegerResult {
                                        dest: reg_1 as usize,
                                        val,
                                    });
                                self.pipeline_execute(false);
                            }
                            MemResponse::Miss | MemResponse::Load => unreachable!(),
                        }
                }
            } else {
                // do nothing
            }
            // if instruction isnt load/store -> return to write_back forwarding instruction
            // if instruction is load/store ->
            //          if cache returns wait -> return to write_back with noop/stall
            //          if cache returns value -> put value in instruction result and return to write_back
        }

    }

    fn pipeline_write_back(&mut self) {
        // if saved instruction has result -> write to reg, update pending regs
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
        // call memory
        if !self.write_back.instruction.stall {
            // save instruction from memory for next cycle
            self.write_back.instruction = self.pipeline_memory();
        }
        // return to clock
    }
}
