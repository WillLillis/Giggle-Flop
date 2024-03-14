#![warn(clippy::all, clippy::pedantic)]

use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemBlock {
    Bits8(u8),
    Bits16(u16),
    Bits32(u32),
}

impl Default for MemBlock {
    fn default() -> Self {
        Self::Bits8(0u8)
    }
}

impl Display for MemBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Bits8(data) => {
                write!(f, "0x{data:08X}")?;
            }
            Self::Bits16(data) => {
                let bytes = data.to_be_bytes();
                write!(f, "0x{:04X}{:04X}", bytes[0], bytes[1])?;
            }
            Self::Bits32(data) => {
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
