use std::collections::HashSet;

use log::{error, info};

use crate::common::PipelineStage;
use crate::execution::execution_state::ExecutionState;
use crate::memory::memory_system::{
    LoadRequest, MemRequest, MemResponse, MemType, Memory, StoreRequest,
};
use crate::pipeline::execute::PipelineExecute;
use crate::pipeline::instruction::{
    decode_raw_instr, Instruction, InstructionResult, InstructionState, RawInstruction,
};
use crate::pipeline::memory::PipelineMemory;
use crate::pipeline::pipeline::{PipeLine, PipelineState};
use crate::pipeline::write_back::PipelineWriteBack;
use crate::register::register_system::{
    get_comparison_flags, FlagIndex, RegisterGroup, RegisterSet, FLAG_COUNT, RET_REG,
};

use crate::memory::memory_system::MemBlock;
use crate::pipeline::decode::PipelineDecode;
use crate::pipeline::fetch::PipelineFetch;

pub struct System {
    pub clock: usize,
    pub pipeline: PipeLine,
    pub memory_system: Memory,
    pub registers: RegisterSet,
    pub execution_state: ExecutionState,
    // Pipeline v
    // all the pipeline stages...
    // fetch: PipelineFetch,
    // decode: PipelineDecode,
    // execute: PipelineExecute,
    // memory: PipelineMemory,
    // write_back: PipelineWriteBack,
    // // for shared state between stages if necessary...
    // pipeline_state: PipelineState,
    // Take 2 mfs
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
        let tmp_add_instr = Instruction::from(add_instr);
        let tmp_load_instr = Instruction::from(load_instr);
        println!("HEY RIGHT HERE {:?}", tmp_add_instr);
        println!("HEY RIGHT HERE {:?}", tmp_load_instr);
        memory_system.force_store(0, MemBlock::Unsigned32(add_instr));

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
    //
    // fn pipeline_fetch(&mut self, decode_blocked: bool, execute_blocked: bool) -> InstructionState {
    //     println!("start fetch");
    //     // if no current instruction -> send load to cache with PC as address
    //     if self.fetch.instruction.instr == None {
    //         let pc = self.registers.program_counter;
    //         let request = MemRequest::Load(LoadRequest {
    //             issuer: PipelineStage::Fetch,
    //             address: self.registers.program_counter,
    //             width: MemType::Unsigned32,
    //         });
    //         let resp = self.memory_system.request(&request).unwrap();
    //         match resp {
    //             MemResponse::Miss | MemResponse::Wait => {
    //                 // return noop/stall
    //                 self.fetch.instruction.stall = true;
    //                 println!("fetch stall");
    //                 return self.fetch.instruction.clone();
    //             }
    //             MemResponse::Load(load_resp) => {
    //                 // if cache returns value -> set current instruction
    //                 // TODO: Check if unsigned, signed, or float result, set fetch's
    //                 // value accordingly
    //                 // let mem_type = self.execute.instruction.get_mem_type();
    //                 let val = load_resp.data.get_contents(pc).unwrap().get_data();
    //                 self.fetch.instruction.instr = Some(Instruction::Type4 {
    //                     opcode: 2,
    //                     reg_1: pc as u32,
    //                     immediate: pc as u32,
    //                 });
    //                 self.fetch.instruction.val =
    //                     Some(InstructionResult::UnsignedIntegerResult { dest: pc, val });
    //             }
    //             MemResponse::StoreComplete => unreachable!(),
    //         }
    //     }
    //     let mut fetch_instr: Option<InstructionState> = None;
    //     // if no current instruction or decode blocked -> return noop/stall
    //     if self.fetch.instruction.val == None || decode_blocked {
    //         self.fetch.instruction.stall = true;
    //         fetch_instr = Some(self.fetch.instruction.clone());
    //     }
    //     // if current instruction & decode not blocked -> return instruction, increment PC
    //     if self.fetch.instruction.val != None && !decode_blocked {
    //         self.fetch.instruction.stall = false;
    //         self.registers.program_counter += 32;
    //         fetch_instr = Some(self.fetch.instruction.clone());
    //         // TODO: fix this - clear current instruction?
    //         self.fetch.instruction = InstructionState::default();
    //     }
    //     println!("end fetch");
    //     return fetch_instr.unwrap();
    // }
    //
    // fn pipeline_decode(&mut self, execute_blocked: bool, memory_blocked: bool) -> InstructionState {
    //     println!("start decode");
    //     // TODO: split instruction into fields
    //     // if source regs not pending -> get values, create instruction object
    //     //      call fetch with blocked status from execute
    //     // if register values pending -> call fetch with blocked
    //     let has_operands = true;
    //     // save instruction from fetch as next instruction
    //     let mut state = self.pipeline_fetch(false, execute_blocked);
    //     self.decode.instruction = state;
    //     // if instruction has operands and execute not blocked -> put dest register in pending, return instruction object to E
    //     // if instruction missing operands or execute blocked-> return noop/stall
    //     println!("end decode");
    //     if has_operands && !execute_blocked {
    //         self.decode.instruction.stall = false;
    //         return self.decode.instruction.clone();
    //     }
    //     if !has_operands || execute_blocked {
    //         self.decode.instruction.stall = true;
    //     } else {
    //         // TODO: not sure what this case is..
    //         println!("shouldnt be here..")
    //     }
    //     return self.decode.instruction.clone();
    // }
    //
    // fn pipeline_execute(&mut self, memory_blocked: bool) -> InstructionState {
    //     println!("start execute");
    //     if self.execute.instruction.stall {
    //         // if noop -> do nothing
    //     } else {
    //         // if jump subroutine -> get PC, get address
    //         // if branch -> check condition, set flag, calculate target address
    //         if let Some(instr) = self.execute.instruction.instr {
    //             // if jump -> get address
    //             if instr.is_jump_instr() {}
    //             // if ALU op -> do op
    //             if instr.is_alu_instr() {
    //                 match instr {
    //                     // panic for this case, shouldn't happen
    //                     Instruction::Type0 { .. } | Instruction::Type1 { .. } => {}
    //                     Instruction::Type2 {
    //                         opcode,
    //                         reg_1,
    //                         reg_2,
    //                     } => {
    //                         // CMP8/CMP16/CMP32 Rx, Ry
    //                         if opcode == 0 {
    //                         } else if opcode == 1 {
    //                         } else if opcode == 2 {
    //                         } else {
    //                             // shouldn't reach here so panic?
    //                             // TODO: figure out what to actually do here
    //                             panic!()
    //                         }
    //                     }
    //                     Instruction::Type3 { freg_1, freg_2, .. } => {
    //                         // CMPF <Fx>, <Fy>
    //                         // Sets appropriate status bits in the S register by comparing the appropriate number of bits between Fx and Fy
    //                         if self.registers.float[freg_1 as usize].data
    //                             > self.registers.float[freg_2 as usize].data
    //                         {
    //                         } else if self.registers.float[freg_1 as usize].data
    //                             < self.registers.float[freg_2 as usize].data
    //                         {
    //                         } else {
    //                         }
    //                     }
    //                     Instruction::Type4 {
    //                         opcode,
    //                         reg_1,
    //                         immediate,
    //                     } => {
    //                         if opcode == 9 {
    //                             // ADDIM <Rx>, <Immediate Address>
    //                             // Adds the immediate value to the contents of Rx, placing the result into Rx. The appropriate status bits in the S register are set.
    //                             // TODO: fix this
    //                             let x = self.registers.general[reg_1 as usize];
    //                             let sum = immediate + x;
    //                             self.execute.instruction.val =
    //                                 Some(InstructionResult::IntegerResult {
    //                                     dest: reg_1,
    //                                     val: sum,
    //                                 })
    //                         } else {
    //                             // shouldn't reach here so panic?
    //                             // TODO: figure out what to actually do here
    //                             panic!()
    //                         }
    //                     }
    //                     Instruction::Type5 {
    //                         opcode,
    //                         reg_1,
    //                         reg_2,
    //                         reg_3,
    //                     } => {
    //                         // different thing for each opcode, might be slow
    //                         match opcode {
    //                             0 => {
    //                                 // ADDI <Rx>, <Ry>, <Rz>
    //                             }
    //                             1 => {
    //                                 // SUBI <Rx>, <Ry>, <Rz>
    //                             }
    //                             2 => {
    //                                 // MULI <Rx>, <Ry>, <Rz>
    //                             }
    //                             3 => {
    //                                 // DIVI <Rx>, <Ry>, <Rz>
    //                             }
    //                             4 => {
    //                                 // MODI <Rx>, <Ry>, <Rz>
    //                             }
    //                             5 => {
    //                                 // RBSI <Rx>, <Ry>, <Rz>
    //                             }
    //                             6 => {
    //                                 // XORI <Rx>, <Ry>, <Rz>
    //                             }
    //                             7 => {
    //                                 // ANDI <Rx>, <Ry>, <Rz>
    //                             }
    //                             8 => {
    //                                 // ORI <Rx>, <Ry>, <Rz>
    //                             }
    //                             9 => {
    //                                 // ADDU <Rx>, <Ry>, <Rz>
    //                             }
    //                             10 => {
    //                                 // SUBU <Rx>, <Ry>, <Rz>
    //                             }
    //                             11 => {
    //                                 // MULU <Rx>, <Ry>, <Rz>=
    //                             }
    //                             12 => {
    //                                 // DIVU <Rx>, <Ry>, <Rz>
    //                             }
    //                             13 => {
    //                                 // MODU <Rx>, <Ry>, <Rz>
    //                             }
    //                             // shouldn't reach here so panic?
    //                             // TODO: figure out what to actually do here
    //                             _ => panic!(),
    //                         }
    //                     }
    //                     Instruction::Type6 {
    //                         opcode,
    //                         freg_1,
    //                         freg_2,
    //                         freg_3,
    //                     } => {
    //                         // same as type 5, might be slow
    //                         match opcode {
    //                             0 => {
    //                                 // ADDF <Fx>, <Fy>, <Fz>
    //                             }
    //                             1 => {
    //                                 // SUBF <Fx>, <Fy>, <Fz>
    //                             }
    //                             2 => {
    //                                 // MULF <Fx>, <Fy>, <Fz>
    //                             }
    //                             3 => {
    //                                 // DIVF <Fx>, <Fy>, <Fz>
    //                             }
    //                             // shouldn't reach here so panic?
    //                             // TODO: figure out what to actually do here
    //                             _ => panic!(),
    //                         }
    //                     }
    //                     _ => {}
    //                 }
    //                 // TODO: fix hardcoding
    //                 if let Instruction::Type5 {
    //                     opcode: 1,
    //                     reg_1: 1,
    //                     reg_2: 2,
    //                     reg_3: 3,
    //                 } = instr
    //                 {}
    //             }
    //             // if memory -> do address calculation
    //             if instr.is_mem_instr() {}
    //         }
    //     }
    //     // call decode with blocked status from memory
    //     let decode_instr = self.pipeline_decode(false, memory_blocked);
    //
    //     // if memory not blocked -> return instruction object to memory with result
    //     // if memory blocked -> return noop/stall
    //     let mut exec_instr: Option<InstructionState> = None;
    //     if memory_blocked {
    //         self.execute.instruction.stall = true;
    //         exec_instr = Some(self.execute.instruction.clone());
    //     } else {
    //         println!("memory not blocked");
    //         // TODO: placement setting, probably needs to change
    //         self.execute.instruction.instr = decode_instr.instr;
    //         self.execute.instruction.val = decode_instr.val;
    //         self.execute.instruction.stall = false;
    //         exec_instr = Some(self.execute.instruction.clone());
    //     }
    //     // save instruction from decode as next instruction
    //     self.execute.instruction = decode_instr;
    //     println!("end execute");
    //     return exec_instr.unwrap();
    // }
    //
    // // BUG: Where do we return the instruction to writeback???
    // fn pipeline_memory(&mut self) -> InstructionState {
    //     println!("start memory");
    //     let mut exec_instr: Option<InstructionState> = None;
    //     // if noop/nonmem instruction -> do nothing
    //     if let Some(instr) = self.memory.instruction.instr {
    //         if instr.is_mem_instr() {
    //             // if load -> call cache
    //             if instr.is_load_instr() {
    //                 let width = instr.get_mem_width().unwrap();
    //                 // TODO: also support Type 2 loads
    //                 if let Instruction::Type4 {
    //                     opcode,
    //                     reg_1,
    //                     immediate,
    //                     ..
    //                 } = instr
    //                 {
    //                     // destructure to grab immediate field
    //                     let request = MemRequest::Load(LoadRequest {
    //                         issuer: PipelineStage::Memory,
    //                         address: immediate as usize,
    //                         width,
    //                     });
    //                     let resp = self.memory_system.request(&request).unwrap();
    //                     // in cache -> if hit and no delay -> cache returns value
    //                     //          -> if hit and delay/miss -> cache return wait
    //                     //          -> if miss -> cache calls memory
    //                     // in memory -> return value or wait
    //                     //          -> if value -> update cache
    //                     //          -> if whole process behind that in slides
    //                     //          -> if store -> send data, address to cache, update accordingly
    //                     // if value returned -> call execute with not blocked
    //                     // else call execute with blocked
    //                     match resp {
    //                         MemResponse::Miss | MemResponse::Wait => {
    //                             exec_instr = Some(self.pipeline_execute(true));
    //                         }
    //                         MemResponse::Load(load_resp) => {
    //                             // if value returned -> call execute with not blocked
    //                             // TODO: Check if unsigned, signed, or float result, set execute's
    //                             // value accordingly
    //                             // let mem_type = self.execute.instruction.get_mem_type();
    //                             // let val = load_resp
    //                             //     .data
    //                             //     .get_contents(immediate as usize)
    //                             //     .unwrap()
    //                             //     .get_data();
    //                             // self.execute.instruction.instr = Some(instr);
    //                             // self.execute.instruction.val =
    //                             //     Some(InstructionResult::UnsignedIntegerResult {
    //                             //         dest: reg_1 as usize,
    //                             //         val,
    //                             //     });
    //                             exec_instr = Some(self.pipeline_execute(false));
    //                         }
    //                         MemResponse::StoreComplete => unreachable!(),
    //                     }
    //                 }
    //             }
    //             // if store -> send data, address to cache
    //             if instr.is_store_instr() {
    //                 // TODO: width is prob still relevant idk where tho
    //                 let width = instr.get_mem_width().unwrap();
    //                 if let Instruction::Type4 {
    //                     opcode,
    //                     reg_1,
    //                     immediate,
    //                     ..
    //                 } = instr
    //                 {
    //                     let data = if let InstructionResult::AddressResult { ref addr } =
    //                         self.memory.instruction.val.as_ref().unwrap()
    //                     {
    //                         MemBlock::Unsigned32(addr.clone())
    //                     } else {
    //                         panic!("Bad result");
    //                     };
    //                     // destructure to grab immediate field
    //                     let request = MemRequest::Store(StoreRequest {
    //                         issuer: PipelineStage::Memory,
    //                         address: immediate as usize,
    //                         data,
    //                     });
    //                     let resp = self.memory_system.request(&request).unwrap();
    //                     match resp {
    //                         MemResponse::Wait => {
    //                             self.execute.instruction.stall = true;
    //                             exec_instr = Some(self.pipeline_execute(true));
    //                         }
    //                         MemResponse::StoreComplete => {
    //                             // if value returned -> call execute with not blocked
    //                             exec_instr = Some(self.pipeline_execute(false));
    //                         }
    //                         MemResponse::Miss | MemResponse::Load(_) => unreachable!(),
    //                     }
    //                 }
    //             }
    //         } else {
    //             // do nothing
    //         }
    //     }
    //     // TODO: this probably shouldnt need to exist, figure out where to put it
    //     if exec_instr == None {
    //         exec_instr = Some(self.pipeline_execute(false));
    //     }
    //
    //     if let Some(instruction) = exec_instr {
    //         if let Some(instr) = instruction.instr {
    //             // if instruction isnt load/store -> return to write_back forwarding instruction
    //             if !instr.is_mem_instr() {
    //                 println!("shouldnt be here...");
    //                 return instruction;
    //             } else {
    //                 // if instruction is load/store ->
    //                 //      if cache returns wait -> return to write_back with noop/stall
    //                 //      if cache returns value -> put value in instruction result and return to write_back
    //                 println!("is mem instruction...");
    //                 if instr.is_load_instr() | instr.is_store_instr() {
    //                     self.memory.instruction = instruction;
    //                     let res = self.memory.instruction.clone();
    //                     self.memory.instruction = InstructionState::default();
    //                     return res;
    //                 }
    //             }
    //         }
    //     }
    //     println!("end memory");
    //     return self.memory.instruction.clone();
    // }
    //
    // fn pipeline_write_back(&mut self) {
    //     // if saved instruction has result -> write to reg, update pending regs
    //     println!("start write_back");
    //     if !self.write_back.instruction.stall {
    //         match self.write_back.instruction.val {
    //             Some(InstructionResult::UnsignedIntegerResult { dest, val }) => {
    //                 self.registers.general[dest].write_block_unsigned(MemBlock::Unsigned32(val));
    //             }
    //             Some(InstructionResult::IntegerResult { dest, val }) => {
    //                 let bytes = val.to_be_bytes();
    //                 let conv = u32::from_be_bytes(bytes);
    //                 self.registers.general[dest].write_block_signed(MemBlock::Unsigned32(conv));
    //             }
    //             Some(InstructionResult::FloatResult { dest, val }) => {
    //                 let bytes = val.to_be_bytes();
    //                 let conv = u32::from_be_bytes(bytes);
    //                 self.registers.float[dest].write_block(MemBlock::Unsigned32(conv));
    //             }
    //             Some(InstructionResult::AddressResult { addr }) => {
    //                 // if W has branch -> update PC
    //                 // if jump subroutine -> update PC and return reg
    //                 // TODO: Lol
    //             }
    //             None => {
    //                 // if noop/stall -> do nothing
    //             }
    //         }
    //     }
    //     self.write_back.instruction = self.pipeline_memory();
    //     println!("end write_back");
    //     // return to clock
    // }

    // NOTE: Keep in mind, execute needs to pass along blocked status to D from M
    fn pipeline_decode(&mut self, mem_blocked: bool) -> PipelineStageStatus {
        // returns an instruction with decode field filled?
        match self.decode {
            PipelineStageStatus::Instruction(ref mut instruction) => {
                if let Some(raw) = instruction.raw_instr {
                    // split instruction into fields
                    match decode_raw_instr(raw) {
                        Some(instr) => {
                            instruction.decode_instr = Some(instr);
                            let src_regs = instr.get_src_regs();
                            let pending = src_regs.iter().any(|src| self.pending_reg.contains(src));
                            // TODO:
                            // Add logging here...
                            // If source regs not pending, get values and create instruction object
                            // If source regs pending, call fetch with blocked
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
                info!("Stall is current state");
            }
            PipelineStageStatus::Noop => {
                // if Noop/Stall, do nothing
                info!("Noop is current state");
            }
        }

        // NOTE: if we can just grab the registers' contents by value here, maybe
        // we can simplify logic down the line...

        todo!()
    }

    // NOTE: Make sure to set flag status in result for all ALU ops...
    fn pipeline_execute(&mut self, mem_blocked: bool) -> PipelineStageStatus {
        info!("Pipeline: In execute stage");
        // execute appears to pass along a more "filled in" instruction object, look into this...
        match self.execute {
            PipelineStageStatus::Instruction(mut instr) => {
                info!("Have current instruction: {:?}", instr);
                match instr.decode_instr {
                    Some(ref mut instruction) => match instruction {
                        Instruction::Type0 { opcode } => {
                            info!("No work to be done, empty result");
                            instr.instr_result = PipelineInstructionResult::EmptyResult;
                        }
                        Instruction::Type1 { opcode, immediate } => {
                            info!("No work to be done, empty result");
                        }
                        Instruction::Type2 {
                            opcode,
                            reg_1,
                            reg_2,
                        } => match opcode {
                            0 | 1 | 2 => {
                                info!("Comparing general registers {reg_1} and {reg_2}");
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
                            opcode,
                            freg_1,
                            freg_2,
                        } => {
                            info!("Comparing floating point registers {freg_1} and {freg_2}");
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
                                let data = self.registers.general[*reg_1]
                                    .data
                                    .add_immediate(*immediate);
                                instr.instr_result = PipelineInstructionResult::RegisterResult {
                                    reg_group: RegisterGroup::General,
                                    dest_reg: *reg_1,
                                    data,
                                }
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
                                        reg_group: RegisterGroup::General,
                                        dest_reg: *reg_1,
                                        data,
                                    }
                                }
                                _ => {
                                    instr.instr_result = PipelineInstructionResult::EmptyResult;
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
                                    instr.instr_result = PipelineInstructionResult::RegisterResult {
                                        reg_group: RegisterGroup::FloatingPoint,
                                        dest_reg: *freg_1,
                                        data,
                                    }
                                }
                                _ => {
                                    instr.instr_result = PipelineInstructionResult::EmptyResult;
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
                info!("Stall is current state");
            }
            PipelineStageStatus::Noop => {
                // if Noop/Stall, do nothing
                info!("Noop is current state");
            }
        }

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
        info!("Pipeline: In memory stage");
        match self.memory {
            PipelineStageStatus::Instruction(instr) => {
                info!("Have current instruction: {:?}", instr);
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
                            info!("Got {:?} response from memory system", resp);
                            match resp {
                                Ok(MemResponse::Miss) | Ok(MemResponse::Wait) => {
                                    // if not blocked, return instruction with result
                                    // if blocked, return Noop/ Stall
                                    info!("Calling execute with memory blocked");
                                    // BUG: Make sure this doesn't return anything?
                                    self.pipeline_execute(true);
                                    info!("Returning stall status to writeback");
                                    return PipelineStageStatus::Stall;
                                }
                                Ok(MemResponse::StoreComplete) => {
                                    info!("Store request returned StoreComplete status");
                                    let mut completed_instr = instr;
                                    completed_instr.instr_result =
                                        PipelineInstructionResult::EmptyResult;
                                    info!("Calling execute stage");
                                    self.memory = self.pipeline_execute(false);
                                    info!("Got new status from execute stage: {:?}", self.memory);

                                    // return instruction with result
                                    info!(
                                        "Passing completed instruction back to writeback: {:?}",
                                        completed_instr
                                    );
                                    return PipelineStageStatus::Instruction(completed_instr);
                                }
                                Ok(MemResponse::Load(load_resp)) => {
                                    info!("Load request returned data: {:?}", load_resp);
                                    let (reg_group, dest_reg) = match instr.get_target_reg() {
                                        Some(reg_info) => {
                                            info!("Target register group {}, register {} extracted from instruction {:?}", reg_info.0, reg_info.1, instr);
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
                                    info!("Calling execute stage unblocked");
                                    self.memory = self.pipeline_execute(false);
                                    info!("Got new status from execute stage: {:?}", self.memory);

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
                            info!("No memory action to take for instruction {:?}", self.memory);
                            let mut completed_instr = instr;
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
                info!("Stall is current state");
                self.memory = self.pipeline_execute(true);
                return PipelineStageStatus::Stall;
            }
            PipelineStageStatus::Noop => {
                // if Noop/Stall, do nothing
                info!("Noop is current state");
                self.memory = self.pipeline_execute(true);
                return PipelineStageStatus::Stall;
            }
        }
    }

    fn pipeline_writeback(&mut self) {
        info!("Pipeline: In writeback stage");
        match self.writeback {
            PipelineStageStatus::Instruction(instr) => {
                info!("Have current instruction: {:?}", instr);
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
                        info!("Writing result to register");
                        self.registers.write_normal(data, reg_group, dest_reg);
                        info!("Updating pending registers");
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
                        info!("Instruction has branch result. New PC: {}", new_pc);
                        // TODO: Need to set this back one word to account for later increment?
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
                }
            }
            PipelineStageStatus::Stall => {
                // if Noop/Stall, do nothing
                info!("Stall is current state");
            }
            PipelineStageStatus::Noop => {
                // if Noop/Stall, do nothing
                info!("Noop is current state");
            }
        }

        // call M
        //  - Save instr returned from M for next cycle
        info!("Calling memory stage");
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
    pub fn get_target_reg(&self) -> Option<(RegisterGroup, usize)> {
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
