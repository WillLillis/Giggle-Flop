use core::f32;
use std::fmt::Display;

use bitmaps::Bitmap;
use log::{error, info, warn};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};

use crate::memory::memory_system::{MemBlock, MEM_BLOCK_WIDTH};

pub const GEN_REG_COUNT: usize = 16;
pub const FLOAT_REG_COUNT: usize = 16;
pub const FLAG_COUNT: usize = 32;
pub const RET_REG: usize = GEN_REG_COUNT - 1;

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

#[derive(Debug, Clone, Copy)]
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
        let program_counter = 0; // TODO: Different default value?
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
            self.program_counter + MEM_BLOCK_WIDTH
        );
        self.program_counter += MEM_BLOCK_WIDTH;
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
                match data {
                    MemBlock::Float32(inner) => {
                        let bytes = inner.to_be_bytes();
                        let conv = u32::from_be_bytes(bytes);
                        warn!("Attempted to write float data {inner} to general register {num}, converted to u32 {conv}");
                        self.general[num] = Register::new(MemBlock::Unsigned32(conv));
                    }
                    _ => {
                        info!("Wrote {data} to general register {num}");
                        self.general[num] = Register::new(data);
                    }
                }
            }
            RegisterGroup::FloatingPoint => {
                if num >= FLOAT_REG_COUNT {
                    error!("Attempted to write to general register {num}, max index is {FLOAT_REG_COUNT}, treating write as NOOP");
                    return;
                }
                match data {
                    MemBlock::Float32(inner) => {
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

    pub fn write_status(&mut self, idx: FlagIndex, data: bool) {
        info!("Setting status flag {idx} to {data}");
        self.status.set(idx as usize, data);
    }
}

impl Display for RegisterSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut accum = String::new();
        let padding = " ".repeat(4);
        for (i, (reg, freg)) in self.general.iter().zip(self.float.iter()).enumerate() {
            accum += &format!("R{i:02}: {}{padding}F{i:02}: {}\n", reg, freg);
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
                    accum += &format!("R{i:02}: {}\n", reg);
                }
            }
            RegisterGroup::FloatingPoint => {
                for (i, reg) in self.float.iter().enumerate() {
                    accum += &format!("F{i:02}: {}\n", reg);
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
