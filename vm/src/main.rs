#![warn(clippy::all, clippy::pedantic)]

// TODO: Do these need to be here?
mod instruction;
mod memory;
mod register;
mod system;
mod ui;

use anyhow::{anyhow, Result};
// TODO: Probably delete dialoguer dependency...
//use dialoguer::{theme::ColorfulTheme, Input, Select};

fn main() -> Result<()> {
    flexi_logger::Logger::try_with_str("info")?.start()?;
    ui::ui::enter().map_err(|e| anyhow!(e))
}