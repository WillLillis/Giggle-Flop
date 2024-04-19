#![warn(clippy::all, clippy::pedantic)]

use std::{
    fmt::Display,
    ops::{BitAnd, BitOr, BitXor},
};

use log::info;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum MemBlock {
    Unsigned8(u8),
    Unsigned16(u16),
    Unsigned32(u32),
    Signed8(i8),
    Signed16(i16),
    Signed32(i32),
    Float32(f32),
}
impl MemBlock {
    pub fn to_be_bytes(self) -> [u8; 4] {
        match self {
            Self::Unsigned8(data) => {
                let bytes = data.to_be_bytes();
                [0, 0, 0, bytes[0]]
            }
            MemBlock::Unsigned16(data) => {
                let bytes = data.to_be_bytes();
                [0, 0, bytes[0], bytes[1]]
            }
            MemBlock::Unsigned32(data) => data.to_be_bytes(),
            MemBlock::Signed8(data) => {
                let bytes = data.to_be_bytes();
                [0, 0, 0, bytes[0]]
            }
            MemBlock::Signed16(data) => {
                let bytes = data.to_be_bytes();
                [0, 0, bytes[0], bytes[1]]
            }
            MemBlock::Signed32(data) => data.to_be_bytes(),
            MemBlock::Float32(data) => data.to_be_bytes(),
        }
    }

    pub fn add_immediate(&mut self, immediate: u32) -> Self {
        match self {
            MemBlock::Unsigned8(data) => {
                let data = u32::from(*data);
                MemBlock::Unsigned32(data.wrapping_add(immediate))
            }
            MemBlock::Unsigned16(data) => {
                let data = u32::from(*data);
                MemBlock::Unsigned32(data.wrapping_add(immediate))
            }
            MemBlock::Unsigned32(data) => MemBlock::Unsigned32(data.wrapping_add(immediate)),
            MemBlock::Signed8(data) => {
                let data = i32::from(*data);
                MemBlock::Signed32(data.wrapping_add(immediate as i32))
            }
            MemBlock::Signed16(data) => {
                let data = i32::from(*data);
                MemBlock::Signed32(data.wrapping_add(immediate as i32))
            }
            MemBlock::Signed32(data) => MemBlock::Signed32(data.wrapping_add(immediate as i32)),
            MemBlock::Float32(data) => MemBlock::Float32(*data + immediate as f32),
        }
    }

    fn get_unsigned(self) -> Option<u32> {
        match self {
            Self::Unsigned8(data) => Some(u32::from(data)),
            Self::Unsigned16(data) => Some(u32::from(data)),
            Self::Unsigned32(data) => Some(data),
            _ => None,
        }
    }

    fn get_signed(self) -> Option<i32> {
        match self {
            Self::Signed8(data) => Some(i32::from(data)),
            Self::Signed16(data) => Some(i32::from(data)),
            Self::Signed32(data) => Some(data),
            _ => None,
        }
    }

    fn get_float(self) -> Option<f32> {
        if let Self::Float32(data) = self {
            Some(data)
        } else {
            None
        }
    }

    fn force_unsigned(self) -> u32 {
        match self {
            MemBlock::Unsigned8(data) => u32::from(data),
            MemBlock::Unsigned16(data) => u32::from(data),
            MemBlock::Unsigned32(data) => data,
            MemBlock::Signed8(data) => data as u32,
            MemBlock::Signed16(data) => data as u32,
            MemBlock::Signed32(data) => data as u32,
            MemBlock::Float32(data) => data as u32,
        }
    }

    fn force_signed(self) -> i32 {
        match self {
            MemBlock::Unsigned8(data) => i32::from(data),
            MemBlock::Unsigned16(data) => i32::from(data),
            MemBlock::Unsigned32(data) => data as i32,
            MemBlock::Signed8(data) => i32::from(data),
            MemBlock::Signed16(data) => i32::from(data),
            MemBlock::Signed32(data) => data,
            MemBlock::Float32(data) => data as i32,
        }
    }

    fn force_float(self) -> f32 {
        match self {
            MemBlock::Unsigned8(data) => f32::from(data),
            MemBlock::Unsigned16(data) => f32::from(data),
            MemBlock::Unsigned32(data) => data as f32,
            MemBlock::Signed8(data) => f32::from(data),
            MemBlock::Signed16(data) => f32::from(data),
            MemBlock::Signed32(data) => data as f32,
            MemBlock::Float32(data) => data,
        }
    }

    // there has to be a better way to do this...look into later
    pub fn add_register(&mut self, conts: MemBlock) -> Self {
        info!("Add register: {self} + {}", conts);
        if let Some(val) = self.get_unsigned() {
            let other = conts.force_unsigned();
            let result = MemBlock::Unsigned32(val.wrapping_add(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_signed() {
            let other = conts.force_signed();
            let result = MemBlock::Signed32(val.wrapping_add(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_float() {
            let other = conts.force_float();
            let result = MemBlock::Float32(val + other);
            info!("Result: {result}");
            result
        } else {
            unreachable!()
        }
    }

    // there has to be a better way to do this...look into later
    pub fn sub_register(&mut self, conts: MemBlock) -> Self {
        info!("Subtract register: {self} - {}", conts);
        if let Some(val) = self.get_unsigned() {
            let other = conts.force_unsigned();
            let result = MemBlock::Unsigned32(val.wrapping_sub(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_signed() {
            let other = conts.force_signed();
            let result = MemBlock::Signed32(val.wrapping_sub(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_float() {
            let other = conts.force_float();
            let result = MemBlock::Float32(val - other);
            info!("Result: {result}");
            result
        } else {
            unreachable!()
        }
    }

    // there has to be a better way to do this...look into later
    pub fn mul_register(&mut self, conts: MemBlock) -> Self {
        info!("Multiply register: {self} * {}", conts);
        if let Some(val) = self.get_unsigned() {
            let other = conts.force_unsigned();
            let result = MemBlock::Unsigned32(val.wrapping_mul(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_signed() {
            let other = conts.force_signed();
            let result = MemBlock::Signed32(val.wrapping_mul(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_float() {
            let other = conts.force_float();
            let result = MemBlock::Float32(val * other);
            info!("Result: {result}");
            result
        } else {
            unreachable!()
        }
    }

    // there has to be a better way to do this...look into later
    pub fn div_register(&mut self, conts: MemBlock) -> Self {
        info!("Divide register: {self} / {}", conts);
        if let Some(val) = self.get_unsigned() {
            let other = conts.force_unsigned();
            let result = MemBlock::Unsigned32(val.wrapping_div(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_signed() {
            let other = conts.force_signed();
            let result = MemBlock::Signed32(val.wrapping_div(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_float() {
            let other = conts.force_float();
            let result = MemBlock::Float32(val / other);
            info!("Result: {result}");
            result
        } else {
            unreachable!()
        }
    }

    // there has to be a better way to do this...look into later
    pub fn mod_register(&mut self, conts: MemBlock) -> Self {
        info!("Modulo register: {self} % {}", conts);
        if let Some(val) = self.get_unsigned() {
            let other = conts.force_unsigned();
            let result = MemBlock::Unsigned32(val % other);
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_signed() {
            let other = conts.force_signed();
            let result = MemBlock::Signed32(val % other);
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_float() {
            let other = conts.force_float();
            let result = MemBlock::Float32(val % other);
            info!("Result: {result}");
            result
        } else {
            unreachable!()
        }
    }

    // there has to be a better way to do this...look into later
    pub fn right_shift_register(&mut self, conts: MemBlock) -> Self {
        info!("Right shift register: {self} >> {}", conts);
        if let Some(val) = self.get_unsigned() {
            let other = conts.force_unsigned();
            let result = MemBlock::Unsigned32(val.wrapping_shr(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_signed() {
            let other = conts.force_unsigned();
            let result = MemBlock::Signed32(val.wrapping_shr(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_float() {
            let other = conts.force_unsigned();
            let val = val as u32;
            let result = MemBlock::Unsigned32(val.wrapping_shr(other));
            info!("Result: {result}");
            result
        } else {
            unreachable!()
        }
    }

    // there has to be a better way to do this...look into later
    pub fn xor_register(&mut self, conts: MemBlock) -> Self {
        info!("XOR register: {self} ^ {}", conts);
        if let Some(val) = self.get_unsigned() {
            let other = conts.force_unsigned();
            let result = MemBlock::Unsigned32(val.bitxor(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_signed() {
            let other = conts.force_signed();
            let result = MemBlock::Signed32(val.bitxor(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_float() {
            let other = conts.force_unsigned();
            let val = val as u32;
            let result = MemBlock::Unsigned32(val.bitxor(other));
            info!("Result: {result}");
            result
        } else {
            unreachable!()
        }
    }

    // there has to be a better way to do this...look into later
    pub fn and_register(&mut self, conts: MemBlock) -> Self {
        info!("AND register: {self} & {}", conts);
        if let Some(val) = self.get_unsigned() {
            let other = conts.force_unsigned();
            let result = MemBlock::Unsigned32(val.bitand(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_signed() {
            let other = conts.force_signed();
            let result = MemBlock::Signed32(val.bitand(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_float() {
            let other = conts.force_unsigned();
            let val = val as u32;
            let result = MemBlock::Unsigned32(val.bitand(other));
            info!("Result: {result}");
            result
        } else {
            unreachable!()
        }
    }

    // there has to be a better way to do this...look into later
    pub fn or_register(&mut self, conts: MemBlock) -> Self {
        info!("OR register: {self} | {}", conts);
        if let Some(val) = self.get_unsigned() {
            let other = conts.force_unsigned();
            let result = MemBlock::Unsigned32(val.bitor(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_signed() {
            let other = conts.force_signed();
            let result = MemBlock::Signed32(val.bitor(other));
            info!("Result: {result}");
            result
        } else if let Some(val) = self.get_float() {
            let other = conts.force_unsigned();
            let val = val as u32;
            let result = MemBlock::Unsigned32(val.bitor(other));
            info!("Result: {result}");
            result
        } else {
            unreachable!()
        }
    }
}

impl Default for MemBlock {
    fn default() -> Self {
        Self::Unsigned8(0u8)
    }
}

impl Display for MemBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Unsigned8(data) => {
                write!(f, "0x{data:08X}")?;
            }
            Self::Unsigned16(data) => {
                let bytes = data.to_be_bytes();
                write!(f, "0x{:04X}{:04X}", bytes[0], bytes[1])?;
            }
            Self::Unsigned32(data) => {
                let bytes = data.to_be_bytes();
                write!(
                    f,
                    "0x{:02X}{:02X}{:02X}{:02X}",
                    bytes[0], bytes[1], bytes[2], bytes[3]
                )?;
            }
            Self::Signed8(data) => {
                write!(f, "0x{data:08X}")?;
            }
            Self::Signed16(data) => {
                let bytes = data.to_be_bytes();
                write!(f, "0x{:04X}{:04X}", bytes[0], bytes[1])?;
            }
            Self::Signed32(data) => {
                let bytes = data.to_be_bytes();
                write!(
                    f,
                    "0x{:02X}{:02X}{:02X}{:02X}",
                    bytes[0], bytes[1], bytes[2], bytes[3]
                )?;
            }
            Self::Float32(data) => {
                let bytes = data.to_be_bytes();
                write!(
                    f,
                    "0x{:02X}{:02X}{:02X}{:02X}",
                    bytes[0], bytes[1], bytes[2], bytes[3]
                )?;
            }
        }

        Ok(())
    }
}
