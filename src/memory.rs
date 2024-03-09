use std::fmt::Display;

use log::{error, info, warn};

use crate::common::Cycle;

// TODO: Fetch queues and delays...
// Need to specify the issuer of a memory operation?
// What is our cache population policy w.r.t multiple levels?
// Test stuff....
// Write driver program for Wednesday demo...

const MEM_BLOCK_WIDTH: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MemWidth {
    Bits8,
    Bits16,
    Bits32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MemBlock {
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
                write!(f, "0x{:X}", data)?;
            }
            Self::Bits16(data) => {
                let bytes = data.to_be_bytes();
                write!(f, "0x{:X}{:X}", bytes[0], bytes[1])?;
            }
            Self::Bits32(data) => {
                let bytes = data.to_be_bytes();
                write!(
                    f,
                    "0x{:X}{:X}{:X}{:X}",
                    bytes[0], bytes[1], bytes[2], bytes[3]
                )?;
            }
        }

        Ok(())
    }
}

// TODO: Adds tags and stuff
#[derive(Debug, Clone)]
struct MemLine {
    // just store address, mess with tags later if necessary...
    start_addr: Option<usize>,
    data: Vec<MemBlock>, // data
}

impl MemLine {
    fn new(line_len: usize) -> Self {
        Self {
            start_addr: None,
            data: vec![MemBlock::default(); line_len],
        }
    }

    // checks if a `width` bits started at `address` are contained
    // with the given line
    fn get(&self, address: usize, width: MemWidth) -> Option<MemBlock> {
        if let Some(start_addr) = self.start_addr {
            let offset = (address - start_addr) / MEM_BLOCK_WIDTH;
            if offset < self.data.len() {
                return Some(self.data[offset]);
            }
        }

        None
    }
}

impl Display for MemLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let blocks = self
            .data
            .iter()
            .fold(String::new(), |accum, block| accum + &format!(" {block}"));

        if let Some(addr) = self.start_addr {
            write!(f, "<0x{:X}>:{}", addr, blocks)?;
        } else {
            write!(f, "<No Entry>:{}", blocks)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
struct MemoryLevel {
    contents: Vec<MemLine>,
    latency: Cycle,
}

impl MemoryLevel {
    /// Creates a new `MemoryLevel` instances with `n_lines` lines, each
    /// consisting of `line_len` 32 bit blocks
    fn new(n_lines: usize, line_len: usize, latency: Cycle) -> Self {
        Self {
            contents: vec![MemLine::new(line_len); n_lines],
            latency,
        }
    }

    fn load(&self, address: usize, width: MemWidth) -> Option<MemBlock> {
        let entry = &self.contents[address];
        let data = entry.get(address, width);

        if data.is_some() {
            return data;
        }
        None
    }
}

#[derive(Debug, Clone)]
pub struct Memory {
    levels: Vec<MemoryLevel>,
    line_len: usize, // number of MEM_BLOCK_WIDTH-bit words in a cache line
}

impl Memory {
    pub fn new(line_len: usize, capacities: &[usize], latencies: &[Cycle]) -> Self {
        if capacities.is_empty() {
            panic!("Attempted to construct empty memory");
        }
        if capacities.len() != latencies.len() {
            panic!(
                "{} capacities specified, {} latencies specified",
                capacities.len(),
                latencies.len()
            );
        }

        let mut mem = Memory {
            levels: Vec::new(),
            line_len,
        };

        let mut last_size = 0;
        let mut last_latency = 0;
        for (level, (&size, &latency)) in capacities.iter().zip(latencies.iter()).enumerate() {
            info!("Creating memory level {level}");
            if size < last_size {
                warn!("Decreasing memory size with increasing level: Level {}: {last_size}, Level {level}: {size}", level - 1);
            }
            if latency < last_latency {
                warn!("Decreasing memory latency with increasing level: Level {}: {last_latency}, Level {level}: {latency}", level - 1);
            }

            mem.levels.push(MemoryLevel::new(size, line_len, latency));
            last_size = size;
            last_latency = latency;
        }

        info!(
            "Populating line address fields of main memory (Level {})",
            mem.levels.len() - 1
        );
        let mut start_addr = 0usize;
        for line in mem.levels.last_mut().unwrap().contents.iter_mut() {
            line.start_addr = Some(start_addr);
            start_addr += MEM_BLOCK_WIDTH * line_len;
        }

        mem
    }

    /// Returns the number of 32-bit words in the provided memory level
    /// Returns 0 for an invalid index
    fn get_capacity(&self, level: usize) -> usize {
        if level >= self.levels.len() {
            warn!("Checked capacity of invalid memory level: {level}");
            0
        } else {
            self.levels.len() * self.line_len * MEM_BLOCK_WIDTH
        }
    }

    // Convenience method
    /// Returns the capacity of the system's main memory in bits
    fn main_capacity(&self) -> usize {
        self.get_capacity(self.levels.len() - 1)
    }

    fn load(&self, address: usize, width: MemWidth) -> Option<MemBlock> {
        if address % MEM_BLOCK_WIDTH != 0 {
            error!("Unaligned access: {address}");
            return None;
        }

        for (level, mem) in self.levels.iter().enumerate() {
            // address wraps around, no out of bounds runtime errors today
            let curr_addr = address % self.get_capacity(level);
            let data = mem.load(curr_addr, width);
            if data.is_some() {
                info!("Hit: Level {level}, Address: {address}, Data: {:?}", data);
                return data;
            } else {
                info!("Miss: Level {level}, Address: {address}");
            }
        }

        None
    }
}
