#![warn(clippy::all, clippy::pedantic)]

use std::collections::{HashMap, VecDeque};
use std::fmt::Display;

use crate::memory::memory_block::MemBlock;
use crate::memory::memory_line::MemLine;
use crate::memory::memory_system::{
    LoadRequest, LoadResponse, MemRequest, MemResponse, MEM_BLOCK_WIDTH,
};
use crate::system::system::Cycle;

use anyhow::{anyhow, Result};
use log::{error, info};

#[derive(Debug, Clone, Default)]
pub struct MemoryLevel {
    contents: Vec<MemLine>,
    pub reqs: VecDeque<MemRequest>,
    pub curr_reqs: HashMap<MemRequest, usize>,
    latency: Cycle,
    is_main: bool,
    line_len: usize,
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
            self.latency, self.reqs, self.curr_reqs, conts
        )?;

        Ok(())
    }
}

impl MemoryLevel {
    /// Creates a new `MemoryLevel` instances with `n_lines` lines, each
    /// consisting of `line_len` `MEM_BLOCK_WIDTH` bit blocks
    pub fn new(n_lines: usize, line_len: usize, latency: Cycle, is_main: bool) -> Self {
        assert!(n_lines != 0, "Constructing empty memory level");

        Self {
            contents: vec![MemLine::new(None, line_len); n_lines],
            latency,
            reqs: VecDeque::new(),
            curr_reqs: HashMap::new(),
            is_main,
            line_len,
        }
    }

    // for testing/ debugging
    pub fn force_store(&mut self, address: usize, data: MemBlock) {
        let idx = self.address_index(address);
        if let Err(e) = self.contents[idx].write(address, data) {
            error!(
                "force_store: write to {address} with {:?} failed -- error {e}",
                data
            );
        }
    }

    // for testing/ debugging
    pub fn force_load(&self, address: usize) -> Option<MemBlock> {
        let idx = self.address_index(address);
        let conts = &self.contents[idx];
        conts.get_contents(address)
    }

    /// Issues a new load request, or checks the status of an existing (matching)
    /// load request
    pub fn load(&mut self, req: &LoadRequest) -> MemResponse {
        let address = req.address % (self.contents.len() * self.line_len * MEM_BLOCK_WIDTH);
        let line_idx = self.address_index(address);

        if !self.is_main && !self.contents[line_idx].contains_address(address) {
            return MemResponse::Miss;
        }
        let mem_req = MemRequest::from(req.clone());
        match self.curr_reqs.get(&mem_req) {
            Some(0) => {
                info!("Load request completed, request: {:?}", mem_req);
                let data = self.contents[line_idx].clone();

                self.curr_reqs.remove(&mem_req);
                if !self.curr_reqs.iter().any(|(_req, delay)| *delay > 0) {
                    if let Some(next_req) = self.reqs.pop_front() {
                        info!(
                            "Moving next pending request to the head, request: {:?}",
                            next_req
                        );
                        self.curr_reqs.insert(next_req, self.latency);
                    }
                }
                return MemResponse::Load(LoadResponse { data });
            }
            Some(delay) => {
                info!("Request pending: {delay} cycles left");
            }
            None => {
                if !self.curr_reqs.iter().any(|(_req, delay)| *delay > 0) {
                    if let Some(next_req) = self.reqs.pop_front() {
                        self.curr_reqs.insert(next_req, self.latency);
                        self.reqs.push_back(mem_req);
                    } else {
                        self.curr_reqs.insert(mem_req, self.latency);
                    }
                } else {
                    self.reqs.push_back(mem_req);
                }
            }
        }

        MemResponse::Wait
    }

    /// Returns the index of the internal Vec of `MemLine`s that would contain
    /// the supplied `address`
    pub fn address_index(&self, address: usize) -> usize {
        (address / (self.line_len * MEM_BLOCK_WIDTH)) % self.num_lines()
    }

    /// Removes any cache entries containing the given `address`
    pub fn invalidate_address(&mut self, address: usize) {
        // don't invalidate entries in the main memory
        if self.is_main {
            return;
        }

        let line = self.address_index(address);
        self.contents[line] = MemLine::new(None, self.line_len);
    }

    /// Writes a single word to the appropriate address within the line
    pub fn write_block(&mut self, address: usize, data: MemBlock) -> Result<()> {
        let line_idx = self.address_index(address);
        self.contents[line_idx].write(address, data)
    }

    /// Writes an entire line to the appropriate address within the line
    /// `address` must match the starting address of the line
    pub fn write_line(&mut self, address: usize, data: &MemLine) -> Result<()> {
        let line_idx = self.address_index(address);
        // check start address is aligned, if provided
        if let Some(start_addr) = data.start_address() {
            if start_addr % (self.line_len * MEM_BLOCK_WIDTH) != 0 {
                return Err(anyhow!("Invalid start address for line"));
            }
        }
        self.contents[line_idx] = data.clone();

        Ok(())
    }

    /// Decrements the latency count for the pending request
    pub fn update_clock(&mut self) {
        for (req, latency) in self.curr_reqs.iter_mut() {
            *latency = latency.saturating_sub(1);
        }
        // if let Some((ref mut latency, _req)) = &mut self.curr_req {
        //     *latency = latency.saturating_sub(1);
        // }
    }

    /// Returns the latency in clock cycles
    pub fn latency(&self) -> usize {
        self.latency
    }

    /// Returns the number of lines in the memory level
    pub fn num_lines(&self) -> usize {
        self.contents.len()
    }
}
