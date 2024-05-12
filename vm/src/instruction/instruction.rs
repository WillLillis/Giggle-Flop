use std::{fmt::Display, ops::RangeBounds};

use log::{error, info};

use crate::{
    memory::memory_system::{LoadRequest, MemBlock, MemRequest, MemType, StoreRequest},
    register::register_system::{
        Register, RegisterGroup, RegisterSet, ALL_INSTR_TYPES, RET_REG, TYPE_0_INSTRS,
        TYPE_1_INSTRS, TYPE_2_INSTRS, TYPE_3_INSTRS, TYPE_4_INSTRS, TYPE_5_INSTRS, TYPE_6_INSTRS,
    },
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
    pub fn get_mem_req(
        &self,
        issuer: Option<PipelineStage>,
        gen_regs: &[Register],
    ) -> Option<MemRequest> {
        info!("Generating memory request for instruction {:?}", self);
        match self {
            Instruction::Type2 {
                opcode,
                reg_1,
                reg_2,
            } => {
                let mem_type = match opcode {
                    3 | 6 => MemType::Unsigned8,
                    4 | 7 => MemType::Unsigned16,
                    5 | 8 => MemType::Unsigned32,
                    _ => {
                        return None;
                    }
                };

                let address =
                    usize::try_from(gen_regs[*reg_2].data.force_unsigned()).unwrap_or_default();

                if (3..=5).contains(opcode) {
                    Some(MemRequest::Load(LoadRequest {
                        issuer: issuer.unwrap_or_default(),
                        address,
                        width: mem_type,
                    }))
                } else if (6..=8).contains(opcode) {
                    let data = MemBlock::Unsigned32(
                        u32::try_from(gen_regs[*reg_1].data.force_unsigned()).unwrap_or_default(),
                    );
                    Some(MemRequest::Store(StoreRequest {
                        issuer: issuer.unwrap_or_default(),
                        address,
                        data,
                    }))
                } else {
                    None
                }
            }
            Instruction::Type4 {
                opcode,
                reg_1,
                immediate,
            } => {
                let mem_type = match opcode {
                    0 | 6 => MemType::Unsigned8,
                    1 | 7 => MemType::Unsigned16,
                    2 | 8 => MemType::Unsigned32,
                    3 => MemType::Signed8,
                    4 => MemType::Signed16,
                    5 => MemType::Signed32,
                    _ => {
                        return None;
                    }
                };
                if *opcode <= 5 {
                    Some(MemRequest::Load(LoadRequest {
                        issuer: issuer.unwrap_or_default(),
                        address: *immediate as usize,
                        width: mem_type,
                    }))
                } else {
                    Some(MemRequest::Store(StoreRequest {
                        issuer: issuer.unwrap_or_default(),
                        address: *immediate as usize,
                        data: gen_regs[*reg_1].data,
                    }))
                }
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
            Instruction::Type1 { .. } => {
                vec![(RegisterGroup::Flag, 0)]
            }
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
                6..=9 => {
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

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Instruction::Type0 { opcode } => {
                write!(
                    f,
                    "{}",
                    TYPE_0_INSTRS
                        .get(*opcode as usize)
                        .unwrap_or(&"INVALID INSTRUCTION")
                )?;
            }
            Instruction::Type1 { opcode, immediate } => {
                write!(
                    f,
                    "{} 0x{immediate:08X}",
                    TYPE_1_INSTRS
                        .get(*opcode as usize)
                        .unwrap_or(&"INVALID INSTRUCTION"),
                )?;
            }
            Instruction::Type2 {
                opcode,
                reg_1,
                reg_2,
            } => {
                write!(
                    f,
                    "{} R{}, R{}",
                    TYPE_2_INSTRS
                        .get(*opcode as usize)
                        .unwrap_or(&"INVALID INSTRUCTION"),
                    reg_1,
                    reg_2,
                )?;
            }
            Instruction::Type3 {
                opcode,
                freg_1,
                freg_2,
            } => {
                write!(
                    f,
                    "{} F{}, F{}",
                    TYPE_3_INSTRS
                        .get(*opcode as usize)
                        .unwrap_or(&"INVALID INSTRUCTION"),
                    freg_1,
                    freg_2,
                )?;
            }
            Instruction::Type4 {
                opcode,
                reg_1,
                immediate,
            } => {
                write!(
                    f,
                    "{} R{}, 0x{:08X}",
                    TYPE_4_INSTRS
                        .get(*opcode as usize)
                        .unwrap_or(&"INVALID INSTRUCTION"),
                    reg_1,
                    immediate,
                )?;
            }
            Instruction::Type5 {
                opcode,
                reg_1,
                reg_2,
                reg_3,
            } => {
                write!(
                    f,
                    "{} R{}, R{}, R{}",
                    TYPE_5_INSTRS
                        .get(*opcode as usize)
                        .unwrap_or(&"INVALID INSTRUCTION"),
                    reg_1,
                    reg_2,
                    reg_3
                )?;
            }
            Instruction::Type6 {
                opcode,
                freg_1,
                freg_2,
                freg_3,
            } => {
                write!(
                    f,
                    "{} F{}, F{}, F{}",
                    TYPE_6_INSTRS
                        .get(*opcode as usize)
                        .unwrap_or(&"INVALID INSTRUCTION"),
                    freg_1,
                    freg_2,
                    freg_3
                )?;
            }
        }
        Ok(())
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
            // opcode takes four bits
            let opcode = value & MASK_4;
            value >>= 4;

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
