// hey a new file
const MASK_1: u32 = 0b1;
const MASK_2: u32 = 0b11;
const MASK_3: u32 = 0b111;
const MASK_4: u32 = 0b11111;
const MASK_21: u32 = 0b111111111111111111111;

pub type RawInstruction = u32;

#[derive(Debug, Copy, Clone)]
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
        reg_1: u32,
        reg_2: u32,
    }, // Two general purpose register arguments
    Type3 {
        opcode: u32,
        freg_1: u32,
        freg_2: u32,
    }, // Two floating point register arguments
    Type4 {
        opcode: u32,
        reg_1: u32,
        immediate: u32,
    }, // One general purpose register argument, one immediate
    Type5 {
        opcode: u32,
        reg_1: u32,
        reg_2: u32,
        reg_3: u32,
    }, // Three general purpose register arguments
    Type6 {
        opcode: u32,
        freg_1: u32,
        freg_2: u32,
        freg_3: u32,
    }, // Three floating point register arguments
}

impl From<u32> for Instruction {
    fn from(value: u32) -> Self {
        let mut value = value;
        //let instruction =
        // type field is always 3 bits
        // get first three bits
        let instr_type = value & MASK_3;
        value <<= 3;
        // switch type off of that
        match instr_type {
            0 => {
                // opcode takes one bit
                let opcode = value & MASK_1;
                // value <<= 1;

                // 28 remaining bits of padding to ignore

                Instruction::Type0 { opcode }
            }
            1 => {
                // opcode takes four bits
                let opcode = value & MASK_4;
                value <<= 4;

                // immediate argument takes 21 bits
                let immediate = value & MASK_21;
                // value <<= 21;
                // 4 remaining bits of padding to ignore

                Instruction::Type1 { opcode, immediate }
            }
            2 => {
                // opcode takes three bits
                let opcode = value & MASK_3;
                value <<= 3;

                // general register 1 argument takes 4 bits
                let reg_1 = value & MASK_4;
                value <<= 4;

                // general register 2 argument takes 4 bits
                let reg_2 = value & MASK_4;
                // value <<= 4;
                // 18 remaining bits of padding to ignore
                
                Instruction::Type2 { opcode, reg_1, reg_2 }
            }
            3 => {
                // opcode takes one bit
                let opcode = value & MASK_1;
                value <<= 1;

                // floating point register 1 argument takes 4 bits
                let freg_1 = value & MASK_4;
                value <<= 4;

                // floating point register 2 argument takes 4 bits
                let freg_2 = value & MASK_4;
                // value <<= 4;
                // 20 remaining bits of padding to ignore

                Instruction::Type3 { opcode, freg_1, freg_2 }
            }
            4 => {
                // opcode takes four bits
                let opcode = value & MASK_4;
                value <<= 4;

                // general register argument takes 4 bits
                let reg_1 = value & MASK_4;
                value <<= 4;

                // immediate argument takes 21 bits
                let immediate = value & MASK_21;
                // value <<= 21;
                // 0 remaining bits of padding
                
                Instruction::Type4 { opcode, reg_1, immediate }
            }
            5 => {
                // opcode takes four bits
                let opcode = value & MASK_4;
                value <<= 4;

                // general register 1 argument takes 4 bits
                let reg_1 = value & MASK_4;
                value <<= 4;

                // general register 2 argument takes 4 bits
                let reg_2 = value & MASK_4;
                value <<= 4;
                
                // general register 2 argument takes 4 bits
                let reg_3 = value & MASK_4;
                // value <<= 4;
                // 13 remaining bits of padding to ignore

                Instruction::Type5 { opcode, reg_1, reg_2, reg_3 }
            }
            6 => {
                // opcode takes two bits
                let opcode = value & MASK_2;
                value <<= 4;

                // general register 1 argument takes 4 bits
                let freg_1 = value & MASK_4;
                value <<= 4;

                // general register 2 argument takes 4 bits
                let freg_2 = value & MASK_4;
                value <<= 4;
                
                // general register 2 argument takes 4 bits
                let freg_3 = value & MASK_4;
                // value <<= 4;
                // 15 remaining bits of padding to ignore

                Instruction::Type6 { opcode, freg_1, freg_2, freg_3 }
            }
            x => {
                panic!("Invalid instruction type field: {x}")
            }
        }
    }
}
