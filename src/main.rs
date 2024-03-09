mod common;
mod memory;

fn main() -> anyhow::Result<()> {
    // Driver program here...
    flexi_logger::Logger::try_with_str("info")?.start()?;


    let giggle =  cfonts::render(cfonts::Options {
		text: String::from("Giggle"),
		font: cfonts::Fonts::FontBlock,
        colors: vec![cfonts::Colors::Yellow, cfonts::Colors::Blue],
		..cfonts::Options::default()
	});
    let flop =  cfonts::render(cfonts::Options {
		text: String::from("Flop"),
		font: cfonts::Fonts::FontBlock,
        colors: vec![cfonts::Colors::Yellow, cfonts::Colors::Blue],
		..cfonts::Options::default()
	});

    print!("{}", giggle.text);
    print!("{}", flop.text);

    let mut mem = memory::Memory::new(1, &[32], &[1]);
    // just sit in a while loop with dialoguer prompts?
    loop {
        // prompt for line size, level size(s), level latencies here
        // Also option to quit
        loop {
            // Prompt for storing, loading, or printing 
            // Also option to quit with this configuration
        }
    }
    

    todo!()
}
