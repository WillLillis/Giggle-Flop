#![warn(clippy::all, clippy::pedantic)]

use std::{collections::VecDeque, fmt::Display};

use crate::common::{Cycle, PipelineStage};

use anyhow::{anyhow, Result};
use log::{error, info, warn};

pub const MEM_BLOCK_WIDTH: usize = 32;
#[allow(dead_code)]
pub const N_ADDRESS_BITS: usize = 21;
#[allow(dead_code, clippy::cast_possible_truncation)]
pub const ADDRESS_SPACE_SIZE: usize = 2usize.pow(N_ADDRESS_BITS as u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemWidth {
    Bits8,
    Bits16,
    Bits32,
}

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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MemLine {
    // just store address of the first entry in the line, mess with tags later if necessary...
    start_addr: Option<usize>,
    data: Vec<MemBlock>,
}

impl MemLine {
    fn new(start_addr: Option<usize>, line_len: usize) -> Self {
        Self {
            start_addr,
            data: vec![MemBlock::default(); line_len],
        }
    }

    pub fn contains_address(&self, address: usize) -> bool {
        let Some(start_addr) = self.start_addr else {
            return false;
        };
        let line_len = self.data.len();
        let range = start_addr..start_addr + (MEM_BLOCK_WIDTH * line_len);

        range.contains(&address)
    }

    pub fn write(&mut self, address: usize, data: MemBlock) -> Result<()> {
        if !self.contains_address(address) {
            return Err(anyhow!("Address not contained within line"));
        }
        let line_len = self.data.len();
        let line_idx = (address % (line_len * MEM_BLOCK_WIDTH)) / MEM_BLOCK_WIDTH;
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
    use crate::memory::MemBlock;

    use super::{MemLine, ADDRESS_SPACE_SIZE, MEM_BLOCK_WIDTH};

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
            let blocks = vec![MemBlock::Bits32(777); line_len];
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

#[derive(Debug, Clone, Default)]
struct MemoryLevel {
    contents: Vec<MemLine>,
    latency: Cycle,
    reqs: VecDeque<MemRequest>,
    curr_req: Option<(usize, MemRequest)>,
    is_main: bool,
}

impl Display for MemoryLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let conts = self
            .contents
            .iter()
            .fold(String::new(), |accum, line| accum + &format!("{line}\n"));
        write!(
            f,
            "Latency: {}\nRequest Queue: {:?}\nCurrent Request: {:?}\n\nContents:\n{}",
            self.latency, self.reqs, self.curr_req, conts
        )?;

        Ok(())
    }
}

impl MemoryLevel {
    /// Creates a new `MemoryLevel` instances with `n_lines` lines, each
    /// consisting of `line_len` `MEM_BLOCK_WIDTH` bit blocks
    fn new(n_lines: usize, line_len: usize, latency: Cycle) -> Self {
        assert!(n_lines != 0, "Constructing empty memory level");

        Self {
            contents: vec![MemLine::new(None, line_len); n_lines],
            latency,
            reqs: VecDeque::new(),
            curr_req: None,
            is_main: false,
        }
    }

    /// Issues a new load request, or checks the status of an existing (matching)
    /// load request
    fn load(&mut self, req: &LoadRequest) -> MemResponse {
        let line_len = self.contents.first().unwrap().data.len();
        let address = req.address % (self.contents.len() * line_len * MEM_BLOCK_WIDTH);
        let line_idx = self.address_index(address);

        if !self.is_main && !self.contents[line_idx].contains_address(address) {
            return MemResponse::Miss;
        }
        match self.curr_req {
            Some((0, MemRequest::Load(ref completed_req))) if completed_req == req => {
                let data = self.contents[line_idx].clone();

                self.curr_req = None;
                if let Some(next_req) = self.reqs.pop_front() {
                    self.curr_req = Some((self.latency, next_req));
                }
                return MemResponse::Load(LoadResponse { data });
            }
            Some((_delay, MemRequest::Load(ref pending_req))) => {
                if pending_req != req {
                    self.reqs.push_back(MemRequest::Load(req.clone()));
                }
            }
            Some((_, _)) => {
                self.reqs.push_back(MemRequest::Load(req.clone()));
            }
            None => {
                self.curr_req = Some((self.latency, MemRequest::Load(req.clone())));
            }
        }

        MemResponse::Wait
    }

    /// Returns the index of the internal Vec of `MemLine`s that would contain
    /// the supplied `address`
    fn address_index(&self, address: usize) -> usize {
        let line_len = self.contents.first().unwrap().data.len();
        address / (line_len * MEM_BLOCK_WIDTH)
    }

    /// Removes any cache entries containing the given `address`
    pub fn invalidate_address(&mut self, address: usize) {
        // don't invalidate entries in the main memory
        if self.is_main {
            return;
        }

        let line_len = self.contents.first().unwrap().data.len();
        let line = address / (line_len * MEM_BLOCK_WIDTH);
        // TODO: Add check here so we can avoid some redundant allocations
        self.contents[line] = MemLine::new(None, line_len);
    }

    /// Writes a single word to the appropriate address
    pub fn write(&mut self, address: usize, data: MemBlock) -> Result<()> {
        let line_idx = self.address_index(address);
        self.contents[line_idx].write(address, data)
    }

    pub fn update_clock(&mut self) {
        if let Some((ref mut latency, _req)) = &mut self.curr_req {
            *latency = latency.saturating_sub(1);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LoadRequest {
    pub issuer: PipelineStage,
    pub address: usize,
    pub width: MemWidth,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StoreRequest {
    pub issuer: PipelineStage,
    pub address: usize,
    pub data: MemBlock,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemRequest {
    Load(LoadRequest),
    Store(StoreRequest),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LoadResponse {
    data: MemLine,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StoreResponse {}

#[derive(Debug, Clone)]
pub enum MemResponse {
    Miss,
    Wait,
    Load(LoadResponse),
    Store,
}

#[derive(Debug, Clone)]
pub struct Memory {
    levels: Vec<MemoryLevel>,
    line_len: usize, // number of MEM_BLOCK_WIDTH-bit words in a cache line
}

impl Memory {
    pub fn new(line_len: usize, capacities: &[usize], latencies: &[Cycle]) -> Self {
        assert!(
            !capacities.is_empty(),
            "Attempted to construct empty memory"
        );
        assert!(
            capacities.len() == latencies.len(),
            "{} capacities specified, {} latencies specified",
            capacities.len(),
            latencies.len()
        );

        let mut mem = Memory {
            levels: Vec::new(),
            line_len,
        };

        let mut last_size = 0;
        let mut last_latency = 0;
        for (level, (&size, &latency)) in capacities.iter().zip(latencies.iter()).enumerate() {
            info!(
                "Creating memory level {level} with {size} lines and a latency of {latency} cycles"
            );
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

        mem.levels.last_mut().unwrap().is_main = true;
        let mut start_addr = 0usize;
        for line in &mut mem.levels.last_mut().unwrap().contents {
            line.start_addr = Some(start_addr);
            start_addr += MEM_BLOCK_WIDTH * line_len;
        }

        mem
    }

    #[allow(dead_code)]
    // Remove if necessary
    /// Returns the number of bits in the provided memory level
    pub fn get_capacity(&self, level: usize) -> Result<usize> {
        if level >= self.levels.len() {
            Err(anyhow!("Checked capacity of invalid memory level: {level}"))
        } else {
            Ok(self.levels.len() * self.line_len * MEM_BLOCK_WIDTH)
        }
    }

    /// Returns the latency of the provided memory level in clock cycles
    pub fn get_latency(&self, level: usize) -> Result<usize> {
        if level >= self.levels.len() {
            Err(anyhow!("Checked latency of invalid memory level: {level}"))
        } else {
            Ok(self.levels[level].latency)
        }
    }

    // Convenience function
    // Returns the latency of the system's main memory in terms of clock cycles
    pub fn main_latency(&self) -> Result<usize> {
        self.get_latency(self.levels.len() - 1)
    }

    #[allow(dead_code)]
    // Convenience method
    /// Returns the capacity of the system's main memory in bits
    pub fn main_capacity(&self) -> Result<usize> {
        self.get_capacity(self.levels.len() - 1)
    }

    fn load(&mut self, request: &LoadRequest) -> Result<MemResponse> {
        if request.address % MEM_BLOCK_WIDTH != 0 {
            return Err(anyhow!("Unaligned load access: {}", request.address));
        }

        for level in 0..self.levels.len() {
            let resp = self.levels[level].load(request);
            match resp {
                MemResponse::Miss => {
                    info!("Cache miss at level {level}");
                    continue;
                }
                MemResponse::Wait => {
                    info!("Wait response at level {level}");
                    return Ok(resp);
                }
                MemResponse::Load(ref data) => {
                    info!("Data returned: {:?}", data);
                    self.populate_cache(level.saturating_sub(1), &data.data);
                    return Ok(resp);
                }
                MemResponse::Store => {
                    panic!("Received Store response in load()");
                }
            }
        }

        // accesses to main memory will *always* hit
        unreachable!()
    }

    // Our memory subsystem ONLY allows stores to the main memory, no need to
    // handle on a per-level basis...
    /// Store a value to the system's main memory
    fn store(&mut self, req: &StoreRequest) -> Result<MemResponse> {
        if req.address % MEM_BLOCK_WIDTH != 0 {
            return Err(anyhow!("Unaligned store access: {:?}", req));
        }

        // only use request queue for main memory
        let latency = self.main_latency().unwrap();
        let main_mem = self.levels.last_mut().unwrap();
        match main_mem.curr_req {
            Some((0, MemRequest::Store(ref completed_req))) if completed_req == req => {
                // actually write the data...
                main_mem
                    .write(completed_req.address, completed_req.data)
                    .expect("Write failed -- Error {e}");

                // book-keeping on request queue
                main_mem.curr_req = None;
                if let Some(next_req) = main_mem.reqs.pop_front() {
                    main_mem.curr_req = Some((main_mem.latency, next_req));
                }
                return Ok(MemResponse::Store);
            }
            Some((_delay, MemRequest::Store(ref pending_req))) => {
                if pending_req != req {
                    main_mem.reqs.push_back(MemRequest::Store(req.clone()));
                }
            }
            Some((_, _)) => {
                main_mem.reqs.push_back(MemRequest::Store(req.clone()));
            }
            None => main_mem.curr_req = Some((latency, MemRequest::Store(req.clone()))),
        }

        Ok(MemResponse::Wait)
    }

    pub fn update_clock(&mut self) {
        // go through all request queues
        for level in &mut self.levels {
            level.update_clock();
        }
    }

    fn invalidate_address(&mut self, address: usize) {
        info!("Invalidating cache entries for address 0x{:08X}", address);
        // invalidate cache entries, but don't touch main memory
        for level in 0..self.num_levels() - 1 {
            info!("Invalidating cache level {level}");
            self.levels[level].invalidate_address(address);
        }
    }

    fn populate_cache(&mut self, start_level: usize, data: &MemLine) {
        let address = data.start_addr.expect("Empty address field");
        for level in 0..=start_level {
            info!("Populating cache level {level} with {:?}", data);
            let line = address / (self.line_len * MEM_BLOCK_WIDTH);
            self.levels[level].contents[line] = data.clone();
        }
    }

    /// Returns the number of memory levels, including main memory
    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }

    pub fn print_level(&self, level: usize) -> Result<()> {
        if level >= self.num_levels() {
            return Err(anyhow!("Invalid level number"));
        }

        println!("Memory Level {level}:\n{}", self.levels[level]);
        Ok(())
    }

    pub fn request(&mut self, request: &MemRequest) -> Result<MemResponse> {
        match request {
            MemRequest::Load(req) => {
                info!("Issuing load request to memory system: {:?}", req);
                let resp = self.load(req);
                match resp {
                    Ok(MemResponse::Load(ref data)) => {
                        info!("Load operation completed -- Data: {:?}", data);
                        resp
                    }
                    Ok(MemResponse::Wait) => {
                        info!("Wait response for request {:?}", req);
                        resp
                    }
                    Ok(MemResponse::Miss) => {
                        info!(
                            "Miss response for request {:?}, re-issuing to lower level",
                            req
                        );
                        self.load(req)
                    }
                    Ok(MemResponse::Store) => {
                        unreachable!()
                    }
                    Err(e) => {
                        error!("Error occured during load operation -- Error {e}");
                        panic!("Bad load");
                    }
                }
            }
            MemRequest::Store(req) => {
                info!("Issuing store request to memory system: {:?}", req);
                let resp = self.store(req);
                match resp {
                    Ok(MemResponse::Store) => {
                        info!("Successsful store: {:?}", resp);
                        self.invalidate_address(req.address);
                        Ok(MemResponse::Store)
                    }
                    Ok(_) => resp,
                    Err(e) => {
                        error!("Error occurred during store operation -- Error {e}");
                        panic!("Bad store");
                    }
                }
            }
        }
    }
}
