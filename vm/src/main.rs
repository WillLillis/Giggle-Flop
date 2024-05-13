#![warn(clippy::all, clippy::pedantic)]

mod instruction;
mod memory;
mod register;
mod system;
mod ui;

use anyhow::{anyhow, Result};

fn main() -> Result<()> {
    // NOTE: Uncomment the line below to enable logging
    // flexi_logger::Logger::try_with_str("info")?.start()?;
    ui::ui::enter().map_err(|e| anyhow!(e))
}
