#![warn(clippy::all, clippy::pedantic)]

use std::fmt::Display;

use crate::memory::memory_block::MemBlock;
use crate::memory::memory_system::MEM_BLOCK_WIDTH;

use anyhow::{anyhow, Result};
use log::error;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct MemLine {
    start_addr: Option<usize>,
    data: Vec<MemBlock>,
}

impl MemLine {
    /// Constructs a new `MemLine` instance. Optionally specify the address
    /// of the first block in the line with `start_addr`, and specify the number
    /// of MEM_BLOCK_WIDTH-bit words in the line
    pub fn new(start_addr: Option<usize>, line_len: usize) -> Self {
        Self {
            start_addr,
            data: vec![MemBlock::default(); line_len],
        }
    }

    /// Returns the address of the first block in the line
    pub fn start_address(&self) -> Option<usize> {
        self.start_addr
    }

    /// Returns the contents stored at `address`
    pub fn get_contents(&self, address: usize) -> Option<MemBlock> {
        if self.contains_address(address) {
            let idx = (address - self.start_addr.unwrap()) / MEM_BLOCK_WIDTH;
            Some(self.data[idx])
        } else {
            None
        }
    }

    /// Indicates whether the given `adress` is contained within the memory
    /// line
    pub fn contains_address(&self, address: usize) -> bool {
        let Some(start_addr) = self.start_addr else {
            return false;
        };
        let line_len = self.data.len();
        let range = start_addr..start_addr + (MEM_BLOCK_WIDTH * line_len);

        range.contains(&address)
    }

    /// Writes a `MemBlock` data block at `address`
    pub fn write(&mut self, address: usize, data: MemBlock) -> Result<()> {
        if !self.contains_address(address) {
            return Err(anyhow!("Address not contained within line"));
        }
        let line_len = self.data.len();
        let line_idx = (address % (line_len * MEM_BLOCK_WIDTH)) / MEM_BLOCK_WIDTH;
        error!("Force store: {:?}", data);
        self.data[line_idx] = data;

        Ok(())
    }
}

impl Display for MemLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let blocks = self
            .data
            .iter()
            .fold(String::new(), |accum, block| accum + &format!(" {block}"));

        if let Some(addr) = self.start_addr {
            write!(f, "<0x{addr:08X}>:{blocks}")?;
        } else {
            write!(f, "<<No Entry>>:{blocks}")?; // Extra '<' and '>' to align with addresses
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::memory::memory_block::MemBlock;
    use crate::memory::memory_line::MemLine;
    use crate::memory::memory_system::{ADDRESS_SPACE_SIZE, MEM_BLOCK_WIDTH};

    use rand::random;

    fn get_test_memline(base_addr: usize, line_len: usize) -> MemLine {
        MemLine::new(Some(base_addr), line_len)
    }

    #[test]
    fn contains_addresses() {
        for _ in 0..10 {
            // use %'s to prevent overflow...
            let line_len = (random::<usize>() % 64) + 1;
            let base_addr = (random::<usize>() % 128) * line_len;
            let line = get_test_memline(base_addr, line_len);
            for offset in 0..line_len * MEM_BLOCK_WIDTH {
                let addr = base_addr + offset;
                assert!(line.contains_address(addr));
            }
        }
    }

    #[test]
    fn does_not_contain_addresses() {
        for _ in 0..10 {
            // use %'s to prevent overflow...
            let line_len = (random::<usize>() % 64) + 1;
            let base_addr = (random::<usize>() % 128) * line_len;
            let line = get_test_memline(base_addr, line_len);
            for offset in line_len * MEM_BLOCK_WIDTH..ADDRESS_SPACE_SIZE {
                let addr = base_addr + offset;
                assert!(!line.contains_address(addr));
            }
        }
    }

    #[test]
    fn writes_correct_index() {
        for _ in 0..10 {
            // use %'s to prevent overflow...
            let line_len = (random::<usize>() % 64) + 1;
            let base_addr = (random::<usize>() % 128) * line_len;
            let blocks = vec![MemBlock::Unsigned32(777); line_len];
            let mut line = get_test_memline(base_addr, line_len);

            for i in 0..line_len {
                line.write(base_addr + (MEM_BLOCK_WIDTH * i), blocks[i])
                    .unwrap();
            }

            for i in 0..blocks.len() {
                assert!(line.data[i] == blocks[i]);
            }
        }
    }
}
