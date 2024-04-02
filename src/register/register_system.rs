use core::f32;
use std::fmt::Display;

use bitmaps::Bitmap;
use log::{info, warn};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter, EnumString};

use crate::memory::memory_system::MemBlock;

// NOTE:
// Avoiding over-abstracting here and choosing *not* to make a general register
// struct for both floats and ints. Some of Rust's trait bounds for int to float
// operations makes this messy, and we're fairly limited on the number of inner
// data types we have to worry about. Might reconsider later

pub const GEN_REG_COUNT: usize = 16;
pub const FLOAT_REG_COUNT: usize = 16;
pub const FLAG_COUNT: usize = 32;

#[derive(Debug, Clone, Copy, Display, EnumString, EnumIter, PartialEq)]
pub enum RegisterGroup {
    General = 0,
    FloatingPoint = 1,
    Flag = 2,
}

#[derive(Debug, Clone, Copy)]
pub enum IntRegData {
    Unsigned(u32),
    Signed(i32),
}

impl Display for IntRegData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntRegData::Unsigned(x) => {
                write!(f, "0x{x:08X}")?;
            }
            IntRegData::Signed(x) => {
                write!(f, "0x{x:08X}")?;
            }
        }
        Ok(())
    }
}

pub struct FloatRegData(f32);

impl Display for FloatRegData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08}", self.0)?;
        Ok(())
    }
}

pub struct GeneralRegister {
    pub data: IntRegData,
}

impl GeneralRegister {
    pub fn new() -> Self {
        Self {
            data: IntRegData::Unsigned(0u32),
        }
    }
}

pub struct FloatRegister {
    pub data: FloatRegData,
}

impl FloatRegister {
    pub fn new() -> Self {
        Self {
            data: FloatRegData(0f32),
        }
    }
}

// TODO: Implement display trait

impl GeneralRegister {
    /// Writes underlying contents of `data` into `self`, interpreting the inner
    /// contents as unsigned data
    pub fn write_block_unsigned(&mut self, data: MemBlock) {
        let conv_data = match data {
            MemBlock::Bits8(inner_data) => inner_data as u32,
            MemBlock::Bits16(inner_data) => inner_data as u32,
            MemBlock::Bits32(inner_data) => inner_data as u32,
        };

        self.data = IntRegData::Unsigned(conv_data);
        info!(
            "{:?} written to register as unsigned data: {conv_data}",
            data
        );
    }

    /// Writes underlying contents of `data` into `self`, interpreting the inner
    /// contents as signed data
    pub fn write_block_signed(&mut self, data: MemBlock) {
        let conv_data = match data {
            MemBlock::Bits8(inner_data) => inner_data as i32,
            MemBlock::Bits16(inner_data) => inner_data as i32,
            MemBlock::Bits32(inner_data) => inner_data as i32,
        };

        self.data = IntRegData::Signed(conv_data);
        info!("{:?} written to register as signed data: {conv_data}", data);
    }
}

impl Display for GeneralRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)?;
        Ok(())
    }
}

impl FloatRegister {
    /// Writes underlying contents of `data` into `self`, interpreting the inner
    /// contents as floating point data. Data that is less than 32 bits wide will
    /// be zero extended before writing
    pub fn write_block(&mut self, data: MemBlock) {
        // Any "under-width'd" reads will log an error and 0 extend (garbage no matter what...)
        let conv_data = match data {
            MemBlock::Bits8(inner_data) => {
                warn!("Writing 8 bit block to 32 bit floating point register (Garbage Value)");
                let bytes = inner_data.to_be_bytes();
                f32::from_be_bytes([bytes[0], 0, 0, 0])
            }
            MemBlock::Bits16(inner_data) => {
                warn!("Writing 16 bit block to 32 bit floating point register (Garbage Value)");
                let bytes = inner_data.to_be_bytes();
                f32::from_be_bytes([bytes[0], bytes[1], 0, 0])
            }
            MemBlock::Bits32(inner_data) => {
                let bytes = inner_data.to_be_bytes();
                f32::from_be_bytes(bytes)
            }
        };

        self.data = FloatRegData(conv_data);
        info!("{:?} written to register as float data: {conv_data}", data);
    }
}

impl Display for FloatRegister {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)?;
        Ok(())
    }
}

/// Index of the flag register for each flag
#[derive(Debug, Clone, Copy, EnumString, EnumIter)]
pub enum FlagIndex {
    EQ = 0, // Equal
    LT = 1, // Less than
    GT = 2, // Greater than
    OF = 3, // Overflow
    SG = 4, // Sign (+ = 1, - = 0)
    ZO = 5, // Zero
}

pub struct RegisterSet {
    pub general: [GeneralRegister; GEN_REG_COUNT],
    pub float: [FloatRegister; FLOAT_REG_COUNT],
    pub program_counter: usize,
    pub flag: Bitmap<FLAG_COUNT>,
}

impl RegisterSet {
    pub fn new() -> Self {
        let general = core::array::from_fn(|_| GeneralRegister::new());
        let float = core::array::from_fn(|_| FloatRegister::new());
        let program_counter = 0;
        let flag = Bitmap::new();

        RegisterSet {
            general,
            float,
            program_counter,
            flag,
        }
    }
}

// TODO: Messy/ repeated logic here, clean up later...
impl Display for RegisterSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut accum = String::new();
        let padding = " ".repeat(4);
        for (i, (reg, freg)) in self.general.iter().zip(self.float.iter()).enumerate() {
            accum += &format!("R{i:02}: {}{padding}F{i:02}: {}\n", reg, freg);
        }

        for (i, flag_name) in FlagIndex::iter().enumerate() {
            accum += &format!("{:?}: {}\n", flag_name, self.flag.get(i));
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
                    accum += &format!("{:?}: {}\n", flag_name, self.flag.get(i));
                }
            }
        }

        accum
    }
}
