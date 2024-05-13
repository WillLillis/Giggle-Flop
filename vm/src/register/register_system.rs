use core::f32;
use std::fmt::Display;

use bitmaps::Bitmap;
use log::{error, info, warn};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};

use crate::memory::memory_system::{MemBlock, MEM_BLOCK_WIDTH};

pub const GEN_REG_COUNT: usize = 16;
pub const FLOAT_REG_COUNT: usize = 16;
pub const FLAG_COUNT: usize = 6;
pub const RET_REG: usize = GEN_REG_COUNT - 1;

pub const TYPE_0_INSTRS: &[&str] = &["RET", "HALT"];
pub const TYPE_1_INSTRS: &[&str] = &[
    "CALL", "JE", "JNE", "JGT", "JLT", "JGTE", "JLTE", "IJE", "IJNE", "IJGT", "IJLT", "IJGTE",
    "IJLTE",
];
pub const TYPE_2_INSTRS: &[&str] = &[
    "CMP8", "CMP16", "CMP32", "LDIN8", "LDIN16", "LDIN32", "STIN8", "STIN16", "STIN32",
];
pub const TYPE_3_INSTRS: &[&str] = &["CMPF"];
pub const TYPE_4_INSTRS: &[&str] = &[
    "LD8", "LD16", "LD32", "LDI8", "LDI16", "LDI32", "ST8", "ST16", "ST32", "ADDIM",
];
pub const TYPE_5_INSTRS: &[&str] = &[
    "ADDI", "SUBI", "MULI", "DIVI", "MODI", "RBSI", "XORI", "ANDI", "ORI", "ADDU", "SUBU", "MULU",
    "DIVU", "MODU",
];
pub const TYPE_6_INSTRS: &[&str] = &["ADDF", "SUBF", "MULF", "DIVF"];

pub const ALL_INSTR_TYPES: &[&[&str]] = &[
    TYPE_0_INSTRS,
    TYPE_1_INSTRS,
    TYPE_2_INSTRS,
    TYPE_3_INSTRS,
    TYPE_4_INSTRS,
    TYPE_5_INSTRS,
    TYPE_6_INSTRS,
];

#[derive(Debug, Clone, Copy, Display, EnumString, EnumIter, PartialEq, Eq, Hash)]
pub enum RegisterGroup {
    General = 0,
    FloatingPoint = 1,
    Flag = 2,
}

/// Index of the flag register for each flag
#[derive(Debug, Clone, Copy, EnumString, EnumIter, Display)]
pub enum FlagIndex {
    EQ = 0, // Equal
    LT = 1, // Less than
    GT = 2, // Greater than
    OF = 3, // Overflow
    SG = 4, // Sign (+ = 1, - = 0)
    ZO = 5, // Zero
}

/// Returns the set of flag values resulting from a comparison of the two values
pub fn get_comparison_flags(reg_1: Register, reg_2: Register) -> [Option<bool>; FLAG_COUNT] {
    error!("Comparing {:?} and {:?}", reg_1, reg_2);
    let mut flags = [None; FLAG_COUNT];
    flags[FlagIndex::EQ as usize] = Some(reg_1 == reg_2);
    flags[FlagIndex::LT as usize] = Some(reg_1 < reg_2);
    flags[FlagIndex::GT as usize] = Some(reg_1 > reg_2);
    // No overflow with comparisons...
    // No sign with comparisons...
    // No zero with comparisons...
    error!("Comparison result: {:?}", flags);

    flags
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Register {
    pub data: MemBlock,
}

impl Register {
    pub fn default() -> Self {
        Self {
            data: MemBlock::Unsigned32(0),
        }
    }

    pub fn new(data: MemBlock) -> Self {
        Self { data }
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)?;
        Ok(())
    }
}

pub struct RegisterSet {
    pub general: [Register; GEN_REG_COUNT],
    pub float: [Register; FLOAT_REG_COUNT],
    pub program_counter: u32,
    pub status: Bitmap<FLAG_COUNT>,
}

impl RegisterSet {
    pub fn new() -> Self {
        let general = core::array::from_fn(|_| Register::default());
        let float = core::array::from_fn(|_| Register::default());
        let program_counter = 0;
        let flags = Bitmap::new();

        RegisterSet {
            general,
            float,
            program_counter,
            status: flags,
        }
    }

    /// Increments the program counter by one word
    pub fn step_pc(&mut self) {
        info!(
            "Incrementing program counter, old: {}, new: {}",
            self.program_counter,
            self.program_counter + u32::try_from(MEM_BLOCK_WIDTH).unwrap()
        );
        self.program_counter += u32::try_from(MEM_BLOCK_WIDTH).unwrap();
    }

    /// Writes a value to a "normal" (non-PC) register
    /// Mismatching datatypes will be converted with a logged warning
    pub fn write_normal(&mut self, data: MemBlock, group: RegisterGroup, num: usize) {
        match group {
            RegisterGroup::General => {
                if num >= GEN_REG_COUNT {
                    error!("Attempted to write to general register {num}, max index is {GEN_REG_COUNT}, treating write as NOOP");
                    return;
                }
                if let MemBlock::Float32(inner) = data {
                    let bytes = inner.to_be_bytes();
                    let conv = u32::from_be_bytes(bytes);
                    warn!("Attempted to write float data {inner} to general register {num}, converted to u32 {conv}");
                    self.general[num] = Register::new(MemBlock::Unsigned32(conv));
                } else {
                    info!("Wrote {data} to general register {num}");
                    self.general[num] = Register::new(data);
                }
            }
            RegisterGroup::FloatingPoint => {
                if num >= FLOAT_REG_COUNT {
                    error!("Attempted to write to general register {num}, max index is {FLOAT_REG_COUNT}, treating write as NOOP");
                    return;
                }
                match data {
                    MemBlock::Float32(_) => {
                        info!("Wrote {data} to floating point register {num}");
                        self.float[num] = Register::new(data);
                    }
                    other => {
                        let bytes = other.to_be_bytes();
                        let conv = f32::from_be_bytes(bytes);
                        warn!("Attempted to write float data {other} to general register {num}, converted to f32 {conv}");
                        self.float[num] = Register::new(MemBlock::Float32(conv));
                    }
                }
            }
            RegisterGroup::Flag => {
                error!(
                    "Attempted to a normal write to the status register, treating write as NOOP"
                );
            }
        }
    }

    pub fn write_status(&mut self, idx: usize, data: bool) {
        info!("Setting status flag {idx} to {data}");
        self.status.set(idx, data);
    }
}

impl Display for RegisterSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut accum = String::new();
        let padding = " ".repeat(4);
        for (i, (reg, freg)) in self.general.iter().zip(self.float.iter()).enumerate() {
            accum += &format!("R{i:02}: {reg}{padding}F{i:02}: {freg}\n");
        }

        for (i, flag_name) in FlagIndex::iter().enumerate() {
            accum += &format!("{}: {}\n", flag_name, self.status.get(i));
        }

        write!(f, "{accum}")?;
        Ok(())
    }
}

impl RegisterSet {
    pub fn group_to_string(&self, group: RegisterGroup) -> String {
        let mut accum = String::new();
        match group {
            RegisterGroup::General => {
                for (i, reg) in self.general.iter().enumerate() {
                    accum += &format!("R{i:02}: {reg}\n");
                }
            }
            RegisterGroup::FloatingPoint => {
                for (i, reg) in self.float.iter().enumerate() {
                    accum += &format!("F{i:02}: {reg}\n");
                }
            }
            RegisterGroup::Flag => {
                for (i, flag_name) in FlagIndex::iter().enumerate() {
                    accum += &format!("{:?}: {}\n", flag_name, self.status.get(i));
                }
            }
        }

        accum
    }
}
