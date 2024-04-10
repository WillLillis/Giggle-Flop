#![warn(clippy::all, clippy::pedantic)]

use std::fmt::Display;

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
    pub fn to_be_bytes(&self) -> [u8; 4] {
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
