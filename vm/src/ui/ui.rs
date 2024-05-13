use iced::event::Event;
use iced::widget::scrollable::Properties;
use iced::widget::{button, checkbox, pane_grid, Button, Column, PaneGrid, Text};
use iced::widget::{column, container, pick_list, row, scrollable, text, Scrollable};
use iced::window;
use iced::{event, Alignment, Color, Command, Element, Length, Subscription, Theme};
use log::info;
use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;

use once_cell::sync::Lazy;
use strum::IntoEnumIterator;

use crate::instruction::instruction::{decode_raw_instr, Instruction};
use crate::memory::memory_system::{MemBlock, MEM_BLOCK_WIDTH};
use crate::register::register_system::RegisterGroup;
use crate::system::system::{System, SystemMessage};

static SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);

pub fn enter() -> iced::Result {
    iced::program("Giggle-Flop", GiggleFlopUI::update, GiggleFlopUI::view)
        //.load(|| window::change_mode(window::Id::MAIN, Mode::Fullscreen))
        .subscription(GiggleFlopUI::subscription)
        .theme(GiggleFlopUI::theme)
        .run()
}

struct GiggleFlopUI {
    system: System,
    memory_levels: Vec<usize>,
    current_memory_level: usize,
    run: bool, // run without stopping after every clock cycle
    register_groups: Vec<RegisterGroup>,
    current_register_group: RegisterGroup,
    current_scroll_offset: scrollable::RelativeOffset,
    panes: pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    use_pipeline: bool,
    breakpoints: HashSet<u32>,
}

#[derive(Debug, Clone)]
enum Message {
    Scrolled(scrollable::Viewport),
    SelectMemoryLevel(usize),
    SelectRegisterGroup(RegisterGroup),
    AdvanceClock,
    RunProgram,
    LoadProgram,
    LineClicked(u32),
    EventOccurred(Event),
    // UsePipeline(bool),
    // maybe delete
    Clicked(pane_grid::Pane),
    Resized(pane_grid::ResizeEvent),
}

#[derive(Clone, Copy)]
struct Pane {
    // Add data in here as necessary...
}

impl Pane {
    fn new() -> Self {
        Self {}
    }
}

impl GiggleFlopUI {
    fn new() -> Self {
        let system = System::default();
        let memory_levels = (0..system.memory_system.num_levels()).collect();
        let register_groups = {
            let mut groups = Vec::new();
            for group in RegisterGroup::iter() {
                groups.push(group);
            }
            groups
        };
        let (panes, _) = pane_grid::State::new(Pane::new());

        // Create these by reading from memory?
        GiggleFlopUI {
            memory_levels,
            current_memory_level: system.memory_system.num_levels() - 1,
            run: false,
            register_groups,
            current_register_group: RegisterGroup::General,
            current_scroll_offset: scrollable::RelativeOffset::START,
            panes,
            focus: None,
            system,
            use_pipeline: true,
            breakpoints: HashSet::new(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen().map(Message::EventOccurred)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Scrolled(viewport) => {
                self.current_scroll_offset = viewport.relative_offset();
            }
            Message::SelectMemoryLevel(level) => {
                if level < self.system.memory_system.num_levels() {
                    self.current_memory_level = level;
                }
            }
            Message::SelectRegisterGroup(group) => {
                self.current_register_group = group;
            }
            Message::AdvanceClock => {
                let mut cont = true;
                while cont {
                    if let SystemMessage::Halt = self.system.step() {
                        info!("Got halt message");
                        self.run = false;
                    }
                    let effective_pc = if let Some(addr) = self.system.get_display_instr_addr() {
                        u32::try_from(addr).unwrap()
                    } else {
                        self.system.registers.program_counter
                    };

                    if self.breakpoints.contains(&effective_pc) {
                        info!(
                            "Hit breakpoint at address 0x{:08X}",
                            self.system.registers.program_counter
                        );
                        self.run = false;
                    }

                    cont = self.run;
                }
            }
            Message::RunProgram => {
                self.run = !self.run;
                let mut cont = true;
                while cont {
                    if let SystemMessage::Halt = self.system.step() {
                        info!("Got halt message");
                        self.run = false;
                    }
                    let effective_pc = if let Some(addr) = self.system.get_display_instr_addr() {
                        u32::try_from(addr).unwrap()
                    } else {
                        self.system.registers.program_counter
                    };

                    if self.breakpoints.contains(&effective_pc) {
                        info!(
                            "Hit breakpoint at address 0x{:08X}",
                            self.system.registers.program_counter
                        );
                        self.run = false;
                    }

                    cont = self.run;
                }
            }
            Message::EventOccurred(event) => {
                if let Event::Window(_id, window::Event::FileDropped(path)) = event {
                    self.system.load_program(path);
                }
                // NOTE: Check for other file events, maybe some different actions for them?
                // e.g. hover, hover left, etc.
            }
            Message::LoadProgram => {
                self.system.reset();
                self.system
                    .load_program(PathBuf::from_str("test_bin").unwrap());
                /*
                 * Matrix Multiply Benchmark Init
                 */
                let mut addr = 2432;
                let data = MemBlock::Unsigned32(15);
                self.system.memory_system.force_store(addr, data);
                addr += MEM_BLOCK_WIDTH;
                let data = MemBlock::Unsigned32(15);
                self.system.memory_system.force_store(addr, data);
                addr += MEM_BLOCK_WIDTH;
                let data = MemBlock::Unsigned32(15);
                self.system.memory_system.force_store(addr, data);
                addr += MEM_BLOCK_WIDTH;
                for _ in 0..15 {
                    for _ in 0..15 {
                        let data = MemBlock::Unsigned32(2);
                        self.system.memory_system.force_store(addr, data);
                        addr += MEM_BLOCK_WIDTH;
                    }
                }

                for _ in 0..15 {
                    for _ in 0..15 {
                        let data = MemBlock::Unsigned32(2);
                        self.system.memory_system.force_store(addr, data);
                        addr += MEM_BLOCK_WIDTH;
                    }
                }
                /*
                 * Sorting Benchmark Init
                 */
                // let len = 100;
                // let mut addr = 1152;
                // for val in (0..len).rev() {
                //     // store the value
                //     let data = MemBlock::Unsigned32(val);
                //     self.system.memory_system.force_store(addr, data);
                //     // store the next pointer
                //     addr += MEM_BLOCK_WIDTH;
                //     let next = if val > 0 {
                //         MemBlock::Unsigned32(addr as u32 + MEM_BLOCK_WIDTH as u32 * 4)
                //     } else {
                //         MemBlock::Unsigned32(0)
                //     };
                //     self.system.memory_system.force_store(addr, next);
                //     addr += MEM_BLOCK_WIDTH * 4;
                // }
                /*
                 * Linked List Sum Benchmark Init
                 */
                // let len = 100;
                // let mut addr = 1152;
                // for val in (0..len).rev() {
                //     // store the value
                //     let data = MemBlock::Unsigned32(val);
                //     self.system.memory_system.force_store(addr, data);
                //     // store the next pointer
                //     addr += MEM_BLOCK_WIDTH;
                //     let next = if val > 0 {
                //         MemBlock::Unsigned32(addr as u32 + MEM_BLOCK_WIDTH as u32 * 4)
                //     } else {
                //         MemBlock::Unsigned32(0)
                //     };
                //     self.system.memory_system.force_store(addr, next);
                //     addr += MEM_BLOCK_WIDTH * 4;
                // }
            }
            Message::LineClicked(addr) => {
                if !self.breakpoints.remove(&addr) {
                    self.breakpoints.insert(addr);
                }
            }
            Message::Clicked(pane) => {
                self.focus = Some(pane);
            }
            Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);
            }
        }
        Command::none()
    }

    fn get_config_element(&self) -> Element<Message> {
        let config_content: Element<Message> = Element::from({
            let step_button = || {
                button("Step clock")
                    .padding(10)
                    .on_press(Message::AdvanceClock)
            };
            let run_button = || {
                button(if self.run { "Pause" } else { "Run" })
                    .padding(10)
                    .on_press(Message::RunProgram)
            };
            let load_button = || {
                button("Load test program")
                    .padding(10)
                    .on_press(Message::LoadProgram)
            };
            let clock_text = format!("Clock: {}", self.system.clock);
            Scrollable::with_direction(
                row![text(clock_text), step_button(), run_button(), load_button(),]
                    .align_items(Alignment::Center)
                    .padding([0, 0, 0, 0])
                    .spacing(20),
                {
                    let properties = Properties::new()
                        .width(10)
                        .margin(0)
                        .scroller_width(10)
                        .alignment(scrollable::Alignment::Start);

                    scrollable::Direction::Both {
                        horizontal: properties,
                        vertical: properties,
                    }
                },
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .id(SCROLLABLE_ID.clone())
            .on_scroll(Message::Scrolled)
        });

        let content: Element<Message> = column![config_content]
            .align_items(Alignment::Center)
            .spacing(10)
            .into();

        container(content).padding(20).center_x().center_y().into()
    }

    fn get_pipeline_element(&self) -> Element<Message> {
        let pipeline_content: Element<Message> = Element::from({
            let fetch_state = format!("{:?}", self.system.fetch.raw_instr);
            let decode_state = format!("{:?}", self.system.decode);
            let execute_state = format!("{:?}", self.system.execute);
            let memory_state = format!("{:?}", self.system.memory);
            let writeback_state = format!("{:?}", self.system.writeback);
            Scrollable::with_direction(
                row![
                    column![text("Fetch: "), text(fetch_state)].align_items(Alignment::Center),
                    column![text("Decode: "), text(decode_state)].align_items(Alignment::Center),
                    column![text("Execute: "), text(execute_state)].align_items(Alignment::Center),
                    column![text("Memory: "), text(memory_state)].align_items(Alignment::Center),
                    column![text("Writeback: "), text(writeback_state)]
                        .align_items(Alignment::Center),
                ]
                .align_items(Alignment::Start)
                .spacing(200),
                {
                    let properties = Properties::new()
                        .width(10)
                        .margin(0)
                        .scroller_width(10)
                        .alignment(scrollable::Alignment::Start);

                    scrollable::Direction::Both {
                        horizontal: properties,
                        vertical: properties,
                    }
                },
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .id(SCROLLABLE_ID.clone())
            .on_scroll(Message::Scrolled)
        });

        let content: Element<Message> = column![pipeline_content]
            .align_items(Alignment::Center)
            .spacing(10)
            .into();

        container(content).padding(20).center_x().center_y().into()
    }

    fn get_register_element(&self) -> Element<Message> {
        let scrollable_content: Element<Message> = Element::from({
            Scrollable::with_direction(
                row![
                    column![text(
                        &self
                            .system
                            .registers
                            .group_to_string(self.current_register_group)
                    )]
                    .align_items(Alignment::Center)
                    .padding([0, 0, 0, 0])
                    .spacing(40),
                    text(" ".repeat(8)) // padding so scrollbar doesn't cover text
                ],
                {
                    let properties = Properties::new()
                        .width(10)
                        .margin(0)
                        .scroller_width(10)
                        .alignment(scrollable::Alignment::Start);

                    scrollable::Direction::Both {
                        horizontal: properties,
                        vertical: properties,
                    }
                },
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .id(SCROLLABLE_ID.clone())
            .on_scroll(Message::Scrolled)
        });

        let reg_select = pick_list(
            self.register_groups.as_ref(),
            Some(self.current_register_group),
            Message::SelectRegisterGroup,
        );

        let content: Element<Message> = column![reg_select, scrollable_content]
            .align_items(Alignment::Start)
            .spacing(10)
            .into();

        container(content).padding(20).center_x().center_y().into()
    }

    fn get_memory_element(&self) -> Element<Message> {
        let scrollable_content: Element<Message> = Element::from({
            Scrollable::with_direction(
                row![
                    column![
                        text(
                            self.system
                                .memory_system
                                .get_level(self.current_memory_level)
                                .unwrap()
                        ),
                        text("")
                    ] // padding
                    .align_items(Alignment::Center)
                    .padding([0, 0, 0, 0])
                    .spacing(40),
                    text(" ".repeat(8)) // padding so scrollbar doesn't cover text
                ],
                {
                    let properties = Properties::new()
                        .width(10)
                        .margin(0)
                        .scroller_width(10)
                        .alignment(scrollable::Alignment::Start);

                    scrollable::Direction::Both {
                        horizontal: properties,
                        vertical: properties,
                    }
                },
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .id(SCROLLABLE_ID.clone())
            .on_scroll(Message::Scrolled)
        });

        let mem_level_select = pick_list(
            self.memory_levels.as_ref(),
            Some(self.current_memory_level),
            Message::SelectMemoryLevel,
        )
        .placeholder("Memory level");

        let content: Element<Message> = column![mem_level_select, scrollable_content]
            .align_items(Alignment::Start)
            .spacing(10)
            .into();

        container(content).padding(20).center_x().center_y().into()
    }

    fn get_instruction_element(&self) -> Element<Message> {
        let curr_pc = self.system.registers.program_counter as usize;
        let lookahead = MEM_BLOCK_WIDTH * 10;
        let raw_instrs: Vec<(usize, Option<Instruction>)> = (curr_pc.saturating_sub(lookahead)
            ..curr_pc.saturating_add(lookahead))
            .step_by(MEM_BLOCK_WIDTH)
            .into_iter()
            .map(|addr| (addr, self.system.memory_system.force_instr_load(addr)))
            .map(|(addr, raw_instr)| (addr, decode_raw_instr(raw_instr)))
            .collect();

        let mut column = Column::new();
        for (addr, decoded_instr) in raw_instrs {
            let mut text = if let Some(instr) = decoded_instr {
                let formatted = format!("0x{addr:08X}: {}", instr);
                Text::new(formatted)
            } else {
                Text::new(format!("0x{addr:08X}: INVALID INSTRUCTION"))
            };

            // iterate through source addresses of instructions in pipeline (backwards)
            if let Some(display_addr) = self.system.get_display_instr_addr() {
                if addr == usize::try_from(display_addr).unwrap() {
                    text = text.color(Color::from_rgb(0.0, 1.0, 0.0));
                }
            } else {
                // if none of the stages have a valid source address in their state,
                // then we must be blocked on an instruction fetch. We can just grab
                // the program counter in this case
                if addr == usize::try_from(self.system.registers.program_counter).unwrap() {
                    text = text.color(Color::from_rgb(0.0, 1.0, 0.0));
                }
            }

            let button = Button::new(text)
                .on_press(Message::LineClicked(addr as u32))
                .style(if self.breakpoints.contains(&(addr as u32)) {
                    style::breakpoint_button
                } else {
                    style::regular_button
                })
                .padding(0);
            column = column.push(button);
        }

        let scrollable_content: Element<Message> = Element::from({
            Scrollable::with_direction(
                row![column.align_items(Alignment::Start).padding([0, 0, 0, 0])], // padding so scrollbar doesn't cover text
                {
                    let properties = Properties::new()
                        .width(10)
                        .margin(0)
                        .scroller_width(10)
                        .alignment(scrollable::Alignment::Start);

                    scrollable::Direction::Both {
                        horizontal: properties,
                        vertical: properties,
                    }
                },
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .id(SCROLLABLE_ID.clone())
            .on_scroll(Message::Scrolled)
        });
        let content: Element<Message> = column![scrollable_content]
            .align_items(Alignment::Start)
            .spacing(10)
            .into();

        container(content).padding(20).center_x().center_y().into()
    }

    fn view(&self) -> Element<Message> {
        let memory_block = PaneGrid::new(&self.panes, |_id, _pane, _is_maximized| {
            let title = row!["Memory Subsystem"].spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .padding(10)
                .style(style::title_bar);

            pane_grid::Content::new(self.get_memory_element())
                .title_bar(title_bar)
                .style(style::pane)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .on_click(Message::Clicked)
        .on_resize(10, Message::Resized);

        let memory_pane: Element<Message> = container(memory_block)
            .width(Length::FillPortion(4))
            .height(Length::Fill)
            .padding(10)
            .into();

        let register_block = PaneGrid::new(&self.panes, |_id, _pane, _is_maximized| {
            let title = row!["Register System"].spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .padding(10)
                .style(style::title_bar);

            pane_grid::Content::new(self.get_register_element())
                .title_bar(title_bar)
                .style(style::pane)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .on_click(Message::Clicked)
        .on_resize(10, Message::Resized);

        let register_pane: Element<Message> = container(register_block)
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .padding(10)
            .into();

        let instruction_block = PaneGrid::new(&self.panes, |_id, _pane, _is_maximized| {
            let title = row!["Instructions"].spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .padding(10)
                .style(style::title_bar);

            pane_grid::Content::new(self.get_instruction_element())
                .title_bar(title_bar)
                .style(style::pane)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .on_click(Message::Clicked)
        .on_resize(10, Message::Resized);

        let instruction_pane: Element<Message> = container(instruction_block)
            .width(Length::FillPortion(3))
            .height(Length::FillPortion(4))
            .padding(10)
            .into();

        let config_block = PaneGrid::new(&self.panes, |_id, _pane, _is_maximized| {
            let title = row!["Config"].spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .padding(10)
                .style(style::title_bar);

            pane_grid::Content::new(self.get_config_element())
                .title_bar(title_bar)
                .style(style::pane)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .on_click(Message::Clicked)
        .on_resize(10, Message::Resized);

        let config_pane: Element<Message> = container(config_block)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .into();

        let pipeline_block = PaneGrid::new(&self.panes, |_id, _pane, _is_maximized| {
            let title = row!["Pipeline Stages"].spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .padding(10)
                .style(style::title_bar);

            pane_grid::Content::new(self.get_pipeline_element())
                .title_bar(title_bar)
                .style(style::pane)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .on_click(Message::Clicked)
        .on_resize(10, Message::Resized);

        let pipeline_pane: Element<Message> = container(pipeline_block)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .into();

        column![
            config_pane,
            row![instruction_pane, register_pane, memory_pane],
            pipeline_pane
        ]
        .height(Length::Fill)
        .into()
    }

    #[allow(clippy::unused_self)]
    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

impl Default for GiggleFlopUI {
    fn default() -> Self {
        Self::new()
    }
}

mod style {
    use iced::{
        border::Radius,
        widget::{
            button::{self, Status},
            container,
        },
    };
    use iced::{Border, Color, Shadow, Theme};

    pub fn title_bar(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            text_color: Some(palette.primary.strong.text),
            background: Some(palette.primary.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn pane(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            background: Some(palette.background.weak.color.into()),
            border: Border {
                width: 2.0,
                color: palette.primary.strong.color,
                ..Border::default()
            },
            ..Default::default()
        }
    }

    pub fn regular_button(_theme: &Theme, _status: Status) -> button::Style {
        button::Style {
            background: None,
            text_color: Color::WHITE,
            border: Border {
                color: Color::WHITE,
                width: 0.0,
                radius: Radius::from(0.0),
            },
            shadow: Shadow::default(),
        }
    }

    pub fn breakpoint_button(_theme: &Theme, _status: Status) -> button::Style {
        button::Style {
            background: Some(iced::Background::Color(Color::from_rgb(1.0, 0.0, 0.0))),
            text_color: Color::WHITE,
            border: Border {
                color: Color::WHITE,
                width: 0.0,
                radius: Radius::from(0.0),
            },
            shadow: Shadow::default(),
        }
    }
}
