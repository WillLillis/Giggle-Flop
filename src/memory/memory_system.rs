#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)] // pub mod memory_system, Memory

use crate::common::{Cycle, PipelineStage};
pub use crate::memory::memory_block::MemBlock;
use crate::memory::memory_level::MemoryLevel;
use crate::memory::memory_line::MemLine;

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
    pub data: MemLine,
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

#[allow(clippy::module_name_repetitions)]
impl Memory {
    /// Construct a new `Memory` object, with cache lines of `line_len`
    /// MEM_BLOCK_WIDTH-bit words, and capacities (in number of lines) and latencies
    /// (in terms of clock cycles) specified
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

        let n_levels = capacities.len();
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

            mem.levels.push(MemoryLevel::new(
                size,
                line_len,
                latency,
                level == n_levels - 1,
            ));
            last_size = size;
            last_latency = latency;
        }

        info!(
            "Populating line address fields of main memory (Level {})",
            mem.levels.len() - 1
        );

        let main_mem = mem.levels.last_mut().unwrap();
        let mut start_addr = 0usize;
        for _ in 0..*capacities.last().unwrap() {
            main_mem
                .write_line(start_addr, &MemLine::new(Some(start_addr), line_len))
                .unwrap();
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
            Ok(self.levels[level].latency())
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

    /// Returns the number of lines for a given memory level
    pub fn num_lines(&self, level: usize) -> Result<usize> {
        if level >= self.levels.len() {
            Err(anyhow!(
                "Checked line count of invalid memory level: {level}"
            ))
        } else {
            Ok(self.levels[level].num_lines())
        }
    }

    /// Process a load request
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
                    self.populate_cache(level.saturating_sub(1), &data.data)?;
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

    // Because we're using a write-through no-allocate scheme, we ONLY allow stores
    // to the main memory
    /// Store a value in the system's main memory
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
                    .write_block(completed_req.address, completed_req.data)
                    .expect("Write failed -- Error {e}");

                // book-keeping on request queue
                main_mem.curr_req = None;
                if let Some(next_req) = main_mem.reqs.pop_front() {
                    main_mem.curr_req = Some((main_mem.latency(), next_req));
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

    /// Decrements the latency counters for all current requests, effectively
    /// moving the system forward in time
    pub fn update_clock(&mut self) {
        // update timer for all request queues
        for level in &mut self.levels {
            level.update_clock();
        }
    }

    /// Invalidates all cache lines (in all cache levels) containing the
    /// given `address`
    fn invalidate_address(&mut self, address: usize) {
        info!("Invalidating cache entries for address 0x{:08X}", address);
        // invalidate cache entries, but don't touch main memory
        for level in 0..self.num_levels() - 1 {
            info!("Invalidating cache level {level}");
            self.levels[level].invalidate_address(address);
        }
    }

    /// Writes the line `data` to cache level 0 through cache level `start_level`
    fn populate_cache(&mut self, start_level: usize, data: &MemLine) -> Result<()> {
        let address = data.start_address().expect("Empty address field");
        for level in 0..=start_level {
            info!("Populating cache level {level} with {:?}", data);
            let address = address % self.num_lines(level).unwrap();
            self.levels[level].write_line(address, data)?;
        }

        Ok(())
    }

    /// Returns the number of memory levels, including main memory
    pub fn num_levels(&self) -> usize {
        self.levels.len()
    }

    /// Prints the latency, current request, request queue, and contents of the
    /// given memory `level`
    pub fn print_level(&self, level: usize) -> Result<()> {
        if level >= self.num_levels() {
            return Err(anyhow!("Invalid level number"));
        }

        println!("Memory Level {level}:\n{}", self.levels[level]);
        Ok(())
    }

    /// Issue a `MemRequest` to the memory system
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
