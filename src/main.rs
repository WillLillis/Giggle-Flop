mod common;
mod memory;

use anyhow::Result;

use dialoguer::{theme::ColorfulTheme, Input, Select};

use crate::common::{Cycle, PipelineStage};
use crate::memory::{LoadRequest, MemRequest, MemWidth, StoreRequest, MEM_BLOCK_WIDTH};

fn main() -> Result<()> {
    flexi_logger::Logger::try_with_str("info")?.start()?;

    // NOTE: This code won't make it in the final version, this
    // is just for the purposes of the 3-13-24 Demo...

    let giggle = cfonts::render(cfonts::Options {
        text: String::from("Giggle"),
        font: cfonts::Fonts::FontBlock,
        colors: vec![cfonts::Colors::Yellow, cfonts::Colors::Blue],
        ..cfonts::Options::default()
    });
    let flop = cfonts::render(cfonts::Options {
        text: String::from("  Flop"),
        font: cfonts::Fonts::FontBlock,
        colors: vec![cfonts::Colors::Yellow, cfonts::Colors::Blue],
        ..cfonts::Options::default()
    });

    print!("{}", giggle.text);
    print!("{}", flop.text);

    let mut mem = memory::Memory::new(4, &[32, 64, 128], &[1, 5, 6]);
    let actions = &["Advance Clock", "Load", "Store", "Display", "Quit"];
    let data_widths = &["8  bits", "16 bits", "32 bits"];
    let mut curr_cycle: Cycle = 0;

    loop {
        println!("Clock Cycle: {curr_cycle}");
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select action")
            .default(0)
            .items(actions)
            .interact()
            .unwrap();

        match selection {
            // Advance clock
            0 => {
                curr_cycle += 1;
                mem.update_clock();
            }
            // Load
            1 => {
                let address = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Address")
                    .validate_with({
                        move |input: &String| -> Result<(), String> {
                            let main_cap = 2097152; // 2^21
                            let parsed_num = match input.parse::<usize>() {
                                Ok(num) => num,
                                Err(e) => {
                                    return Err(format!("Must be a valid number -- Error {e}"));
                                }
                            };
                            if parsed_num > main_cap {
                                return Err(String::from(
                                    "Value must lie in the range (0, 2097152]",
                                ));
                            }
                            if parsed_num % MEM_BLOCK_WIDTH != 0 {
                                return Err(format!(
                                    "Value must be a multiple of {MEM_BLOCK_WIDTH}"
                                ));
                            }
                            Ok(())
                        }
                    })
                    .interact_text()
                    .unwrap();
                let address = address.parse::<usize>().unwrap();

                let width = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Data width")
                    .default(data_widths.len() - 1)
                    .items(data_widths)
                    .interact()
                    .unwrap();

                let width = match width {
                    0 => MemWidth::Bits8,
                    1 => MemWidth::Bits16,
                    2 => MemWidth::Bits32,
                    _ => {
                        unreachable!()
                    }
                };

                let request = MemRequest::Load(LoadRequest {
                    issuer: PipelineStage::System,
                    address,
                    width,
                });
                let val = mem.request(&request)?;
                println!("Load Response: {:?}", val);
                curr_cycle += 1;
                mem.update_clock();
            }
            // Store
            2 => {
                let address = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Address")
                    .validate_with({
                        move |input: &String| -> Result<(), String> {
                            let main_cap = 2097152; // 2^21
                            let parsed_num = match input.parse::<usize>() {
                                Ok(num) => num,
                                Err(e) => {
                                    return Err(format!("Must be a valid number -- Error {e}"));
                                }
                            };
                            if parsed_num > main_cap {
                                return Err(String::from(
                                    "Value must lie in the range (0, 2097152]",
                                ));
                            }
                            if parsed_num % MEM_BLOCK_WIDTH != 0 {
                                return Err(format!(
                                    "Value must be a multiple of {MEM_BLOCK_WIDTH}"
                                ));
                            }
                            Ok(())
                        }
                    })
                    .interact_text()
                    .unwrap();
                let address = address.parse::<usize>().unwrap();

                let width = Select::with_theme(&ColorfulTheme::default())
                    .with_prompt("Data width")
                    .default(data_widths.len() - 1)
                    .items(data_widths)
                    .interact()
                    .unwrap();

                let width = match width {
                    0 => MemWidth::Bits8,
                    1 => MemWidth::Bits16,
                    2 => MemWidth::Bits32,
                    _ => {
                        unreachable!()
                    }
                };
                let max_val: usize = match width {
                    MemWidth::Bits8 => u8::MAX as usize,
                    MemWidth::Bits16 => u16::MAX as usize,
                    MemWidth::Bits32 => u32::MAX as usize,
                };

                let data = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Data (unsigned integer)")
                    .validate_with({
                        move |input: &String| -> Result<(), String> {
                            let parsed_num = match input.parse::<usize>() {
                                Ok(num) => num,
                                Err(e) => {
                                    return Err(format!("Must be a valid number -- Error {e}"));
                                }
                            };
                            if parsed_num > max_val {
                                return Err(format!(
                                    "Exceeded maximum value ({max_val}) allowed by data width"
                                ));
                            }
                            Ok(())
                        }
                    })
                    .interact_text()
                    .unwrap();
                let data = match width {
                    MemWidth::Bits8 => memory::MemBlock::Bits8(data.parse().unwrap()),
                    MemWidth::Bits16 => memory::MemBlock::Bits16(data.parse().unwrap()),
                    MemWidth::Bits32 => memory::MemBlock::Bits32(data.parse().unwrap()),
                };

                let request = MemRequest::Store(StoreRequest {
                    issuer: PipelineStage::System,
                    address,
                    data,
                });
                let val = mem.request(&request)?;
                println!("Store Response: {:?}", val);
                curr_cycle += 1;
                mem.update_clock();
            }
            // Display
            3 => {
                let n_levels = mem.num_levels();
                let level = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Level Number")
                    .validate_with({
                        move |input: &String| -> Result<(), String> {
                            let parsed_num = match input.parse::<usize>() {
                                Ok(num) => num,
                                Err(e) => {
                                    return Err(format!("Must be a valid number -- Error {e}"));
                                }
                            };
                            if parsed_num > n_levels {
                                return Err(format!(
                                    "Level select must lie in the range [0,{})",
                                    n_levels
                                ));
                            }
                            Ok(())
                        }
                    })
                    .interact_text()
                    .unwrap();
                let level = level.parse::<usize>().unwrap();
                mem.print_level(level).unwrap();
            }
            // Quit
            4 => {
                break;
            }
            _ => {
                unreachable!()
            }
        }
    }

    Ok(())
}
