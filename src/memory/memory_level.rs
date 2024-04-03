#![warn(clippy::all, clippy::pedantic)]

use std::collections::VecDeque;
use std::fmt::Display;

use crate::common::Cycle;
use crate::memory::memory_block::MemBlock;
use crate::memory::memory_line::MemLine;
use crate::memory::memory_system::{
    LoadRequest, LoadResponse, MemRequest, MemResponse, MEM_BLOCK_WIDTH,
};

use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Default)]
pub struct MemoryLevel {
    contents: Vec<MemLine>,
    pub reqs: VecDeque<MemRequest>,
    pub curr_req: Option<(usize, MemRequest)>,
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
            self.latency, self.reqs, self.curr_req, conts
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
            curr_req: None,
            is_main,
            line_len,
        }
    }


    // for testing/ debugging, get rid of later (TODO:)
    pub fn force_store(&mut self, address: usize, data: MemBlock) {
        let idx = self.address_index(address);
        self.contents[idx].write(address, data);
    }

    /// Issues a new load request, or checks the status of an existing (matching)
    /// load request
    pub fn load(&mut self, req: &LoadRequest) -> MemResponse {
        let address = req.address % (self.contents.len() * self.line_len * MEM_BLOCK_WIDTH);
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
        if let Some((ref mut latency, _req)) = &mut self.curr_req {
            *latency = latency.saturating_sub(1);
        }
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
