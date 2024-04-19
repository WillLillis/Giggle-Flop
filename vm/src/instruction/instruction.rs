use log::{error, info};

use crate::{
    memory::memory_system::{LoadRequest, MemRequest, MemType},
    register::register_system::{RegisterGroup, RET_REG},
    system::system::PipelineStage,
};

const MASK_1: u32 = 0b1;
const MASK_2: u32 = 0b11;
const MASK_3: u32 = 0b111;
const MASK_4: u32 = 0b1111;
const MASK_21: u32 = 0b1_1111_1111_1111_1111_1111;

pub type RawInstruction = u32;

#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub enum Instruction {
    Type0 {
        opcode: u32,
    }, // No arguments
    Type1 {
        opcode: u32,
        immediate: u32,
    }, // One immediate argument
    Type2 {
        opcode: u32,
        reg_1: usize,
        reg_2: usize,
    }, // Two general purpose register arguments
    Type3 {
        opcode: u32,
        freg_1: usize,
        freg_2: usize,
    }, // Two floating point register arguments
    Type4 {
        opcode: u32,
        reg_1: usize,
        immediate: u32,
    }, // One general purpose register argument, one immediate
    Type5 {
        opcode: u32,
        reg_1: usize,
        reg_2: usize,
        reg_3: usize,
    }, // Three general purpose register arguments
    Type6 {
        opcode: u32,
        freg_1: usize,
        freg_2: usize,
        freg_3: usize,
    }, // Three floating point register arguments
}

impl Instruction {
    /// Returns the associated `MemoryRequest` for an instruction if appropriate
    pub fn get_mem_req(&self, issuer: Option<PipelineStage>) -> Option<MemRequest> {
        info!("Generating memory request for instruction {:?}", self);
        match self {
            Instruction::Type2 {
                opcode,
                reg_1: _,
                reg_2,
            } => {
                let mem_type = match opcode {
                    3 => MemType::Unsigned8,
                    4 => MemType::Unsigned16,
                    5 => MemType::Unsigned32,
                    _ => {
                        return None;
                    }
                };
                Some(MemRequest::Load(LoadRequest {
                    issuer: issuer.unwrap_or_default(),
                    address: *reg_2,
                    width: mem_type,
                }))
            }
            Instruction::Type4 {
                opcode,
                reg_1: _,
                immediate,
            } => {
                let mem_type = match opcode {
                    0 => MemType::Unsigned8,
                    1 => MemType::Unsigned16,
                    2 => MemType::Unsigned32,
                    3 => MemType::Signed8,
                    4 => MemType::Signed16,
                    5 => MemType::Signed32,
                    _ => {
                        return None;
                    }
                };
                Some(MemRequest::Load(LoadRequest {
                    issuer: issuer.unwrap_or_default(),
                    address: *immediate as usize,
                    width: mem_type,
                }))
            }
            _ => None,
        }
    }

    /// Returns the source registers associated with the given instruction
    pub fn get_src_regs(&self) -> Vec<(RegisterGroup, usize)> {
        match self {
            Instruction::Type0 { opcode } => match opcode {
                0 => {
                    vec![(RegisterGroup::General, RET_REG)]
                }
                _ => Vec::new(),
            },
            Instruction::Type1 { .. } => Vec::new(),
            Instruction::Type2 {
                opcode,
                reg_1,
                reg_2,
            } => match opcode {
                0..=2 => {
                    vec![
                        (RegisterGroup::General, *reg_1),
                        (RegisterGroup::General, *reg_2),
                    ]
                }
                3..=5 => {
                    vec![(RegisterGroup::General, *reg_1)]
                }
                _ => Vec::new(),
            },
            Instruction::Type3 {
                opcode: _,
                freg_1,
                freg_2,
            } => {
                vec![
                    (RegisterGroup::General, *freg_1),
                    (RegisterGroup::General, *freg_2),
                ]
            }
            Instruction::Type4 {
                opcode,
                reg_1,
                immediate: _,
            } => match opcode {
                6..=8 => {
                    vec![(RegisterGroup::General, *reg_1)]
                }
                _ => Vec::new(),
            },
            Instruction::Type5 {
                opcode: _,
                reg_1: _,
                reg_2,
                reg_3,
            } => {
                vec![
                    (RegisterGroup::General, *reg_2),
                    (RegisterGroup::General, *reg_3),
                ]
            }
            Instruction::Type6 {
                opcode: _,
                freg_1: _,
                freg_2,
                freg_3,
            } => {
                vec![
                    (RegisterGroup::General, *freg_2),
                    (RegisterGroup::General, *freg_3),
                ]
            }
        }
    }
}

/// Transform a raw u32 into an Instruction Object
pub fn decode_raw_instr(raw: u32) -> Option<Instruction> {
    let mut value = raw;
    //let instruction =
    // type field is always 3 bits
    // get first three bits
    let instr_type = value & MASK_3;
    value >>= 3;
    // switch type off of that
    match instr_type {
        0 => {
            // opcode takes one bit
            let opcode = value & MASK_1;
            // value >>= 1;

            // 28 remaining bits of padding to ignore

            Some(Instruction::Type0 { opcode })
        }
        1 => {
            // opcode takes four bits
            let opcode = value & MASK_4;
            value >>= 4;

            // immediate argument takes 21 bits
            let immediate = value & MASK_21;
            // value >>= 21;
            // 4 remaining bits of padding to ignore

            Some(Instruction::Type1 { opcode, immediate })
        }
        2 => {
            // opcode takes three bits
            let opcode = value & MASK_3;
            value >>= 3;

            // general register 1 argument takes 4 bits
            let reg_1 = value & MASK_4;
            value >>= 4;

            // general register 2 argument takes 4 bits
            let reg_2 = value & MASK_4;
            // value >>= 4;
            // 18 remaining bits of padding to ignore

            Some(Instruction::Type2 {
                opcode,
                reg_1: reg_1.try_into().unwrap(),
                reg_2: reg_2.try_into().unwrap(),
            })
        }
        3 => {
            // opcode takes one bit
            let opcode = value & MASK_1;
            value >>= 1;

            // floating point register 1 argument takes 4 bits
            let freg_1 = value & MASK_4;
            value >>= 4;

            // floating point register 2 argument takes 4 bits
            let freg_2 = value & MASK_4;
            // value >>= 4;
            // 20 remaining bits of padding to ignore

            Some(Instruction::Type3 {
                opcode,
                freg_1: freg_1.try_into().unwrap(),
                freg_2: freg_2.try_into().unwrap(),
            })
        }
        4 => {
            // opcode takes four bits
            let opcode = value & MASK_4;
            value >>= 4;

            // general register argument takes 4 bits
            let reg_1 = value & MASK_4;
            value >>= 4;

            // immediate argument takes 21 bits
            let immediate = value & MASK_21;
            // value >>= 21;
            // 0 remaining bits of padding

            Some(Instruction::Type4 {
                opcode,
                reg_1: reg_1.try_into().unwrap(),
                immediate,
            })
        }
        5 => {
            // opcode takes four bits
            let opcode = value & MASK_4;
            value >>= 4;

            // general register 1 argument takes 4 bits
            let reg_1 = value & MASK_4;
            value >>= 4;

            // general register 2 argument takes 4 bits
            let reg_2 = value & MASK_4;
            value >>= 4;

            // general register 2 argument takes 4 bits
            let reg_3 = value & MASK_4;
            // value >>= 4;
            // 13 remaining bits of padding to ignore

            Some(Instruction::Type5 {
                opcode,
                reg_1: reg_1.try_into().unwrap(),
                reg_2: reg_2.try_into().unwrap(),
                reg_3: reg_3.try_into().unwrap(),
            })
        }
        6 => {
            // opcode takes two bits
            let opcode = value & MASK_2;
            value >>= 4;

            // general register 1 argument takes 4 bits
            let freg_1 = value & MASK_4;
            value >>= 4;

            // general register 2 argument takes 4 bits
            let freg_2 = value & MASK_4;
            value >>= 4;

            // general register 2 argument takes 4 bits
            let freg_3 = value & MASK_4;
            // value >>= 4;
            // 15 remaining bits of padding to ignore

            Some(Instruction::Type6 {
                opcode,
                freg_1: freg_1.try_into().unwrap(),
                freg_2: freg_2.try_into().unwrap(),
                freg_3: freg_3.try_into().unwrap(),
            })
        }
        x => {
            error!("Invalid instruction type field: {x}");
            None
        }
    }
}