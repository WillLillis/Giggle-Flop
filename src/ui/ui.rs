use iced::widget::scrollable::Properties;
use iced::widget::{button, checkbox, pane_grid, Button, Column, PaneGrid, Text};
use iced::widget::{column, container, pick_list, row, scrollable, text, Scrollable};
use iced::{Alignment, Color, Command, Element, Length, Theme};
use log::info;
use std::fs::File;
use std::io::{BufRead, BufReader};

use once_cell::sync::Lazy;
use strum::IntoEnumIterator;

use crate::register::register_system::RegisterGroup;
use crate::system::system::System;

static SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);

pub fn enter() -> iced::Result {
    iced::program("Giggle-Flop", GiggleFlopUI::update, GiggleFlopUI::view)
        .theme(GiggleFlopUI::theme)
        .run()
}

struct GiggleFlopUI {
    system: System,
    memory_levels: Vec<usize>,
    current_memory_level: usize,
    register_groups: Vec<RegisterGroup>,
    current_register_group: RegisterGroup,
    current_scroll_offset: scrollable::RelativeOffset,
    panes: pane_grid::State<Pane>,
    focus: Option<pane_grid::Pane>,
    use_pipeline: bool,
    instr_lines: Vec<Line>,
    program_counter: u32,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Scrolled(scrollable::Viewport),
    SelectMemoryLevel(usize),
    SelectRegisterGroup(RegisterGroup),
    AdvanceClock,
    AdvanceInstruction,
    SetBreakpoint,
    UsePipeline(bool),
    LoadProgram,
    LineClicked(usize),
    // maybe delete
    Clicked(pane_grid::Pane),
    Resized(pane_grid::ResizeEvent),
}

#[derive(Clone)]
struct Line {
    number: usize,
    instr: String,
    is_red: bool,
    is_green: bool,
}

#[derive(Clone, Copy)]
struct Pane {
    // TODO: Add data in here as necessary...
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
        let program_counter = system.registers.program_counter;
        let (panes, _) = pane_grid::State::new(Pane::new());

        let instructions = Self::get_instructions_from_file().unwrap();
        let mut instr_obj = Vec::new();
        for (line, instr) in instructions.into_iter().enumerate() {
            instr_obj.push(Line {
                number: line + 1,
                instr,
                is_red: false,
                is_green: false,
            })
        }
        GiggleFlopUI {
            memory_levels,
            current_memory_level: system.memory_system.num_levels() - 1,
            register_groups,
            current_register_group: RegisterGroup::General,
            current_scroll_offset: scrollable::RelativeOffset::START,
            panes,
            focus: None,
            system,
            use_pipeline: true,
            instr_lines: instr_obj,
            program_counter,
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Scrolled(viewport) => {
                self.current_scroll_offset = viewport.relative_offset();

                Command::none()
            }
            Message::SelectMemoryLevel(level) => {
                if level < self.system.memory_system.num_levels() {
                    self.current_memory_level = level;
                }
                Command::none()
            }
            Message::SelectRegisterGroup(group) => {
                self.current_register_group = group;
                Command::none()
            }
            Message::AdvanceClock => {
                self.program_counter = self.system.registers.program_counter;
                println!("program counter: {}", self.program_counter);
                for line in &mut self.instr_lines {
                    line.is_green = line.number == (self.program_counter / 32 + 1) as usize;
                }
                self.system.step();
                Command::none()
            }
            Message::AdvanceInstruction => {
                // TODO: this
                // self.system.
                Command::none()
            }
            Message::SetBreakpoint => {
                // TODO: this
                Command::none()
            }
            Message::UsePipeline(default) => {
                // TODO: this
                self.use_pipeline = default;
                Command::none()
            }
            Message::LoadProgram => {
                // TODO: Fill in later...
                self.system.load_program();
                Command::none()
            }
            Message::LineClicked(line_num) => {
                if let Some(instr) = self.instr_lines.get_mut(line_num - 1) {
                    instr.is_red = !instr.is_red;
                }
                Command::none()
            }
            Message::Clicked(pane) => {
                self.focus = Some(pane);
                Command::none()
            }
            Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);
                Command::none()
            }
        }
    }

    fn get_instructions_from_file() -> Result<Vec<String>, std::io::Error> {
        let program_file = "test.gf";
        info!("Loading instruction file {program_file}");
        let f = File::open(program_file).expect("Unable to open instruction file");
        let f = BufReader::new(f);
        let mut lines = Vec::new();

        for line in f.lines() {
            lines.push(line?);
        }
        Ok(lines)
    }

    fn get_config_element(&self) -> Element<Message> {
        let config_content: Element<Message> = Element::from({
            let step_button = || {
                button("Step clock")
                    .padding(10)
                    .on_press(Message::AdvanceClock)
            };
            let load_button = || {
                button("Load test program")
                    .padding(10)
                    .on_press(Message::LoadProgram)
            };
            let break_button = || {
                button("Set breakpoint")
                    .padding(10)
                    .on_press(Message::SetBreakpoint)
            };
            let skip_instruction_button = || {
                button("Skip instruction")
                    .padding(10)
                    .on_press(Message::AdvanceInstruction)
            };
            let pipeline_checkbox =
                || checkbox("Use Pipeline", self.use_pipeline).on_toggle(Message::UsePipeline);
            let clock_text = format!("Clock: {}", self.system.clock);
            Scrollable::with_direction(
                //column![
                //column![text("TODO: Code goes here..."), step_button()]
                row![
                    text(clock_text),
                    step_button(),
                    load_button(),
                    break_button(),
                    skip_instruction_button(),
                    pipeline_checkbox(),
                ]
                .align_items(Alignment::Center)
                .padding([0, 0, 0, 0])
                .spacing(20),
                //text(" ".repeat(8))
                //], // padding so scrollbar doesn't cover text
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
            //let content: Element<Message> = config_content
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
                    text(" ".repeat(8))
                ], // padding so scrollbar doesn't cover text
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

        // TODO: Use pretty printing crate?
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
                    text(" ".repeat(8))
                ], // padding so scrollbar doesn't cover text
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

        // TODO: Use pretty printing crate?
        let content: Element<Message> = column![mem_level_select, scrollable_content]
            .align_items(Alignment::Start)
            .spacing(10)
            .into();

        container(content).padding(20).center_x().center_y().into()
    }

    fn get_instruction_element(&self) -> Element<Message> {
        let mut column = Column::new();
        for instr in &self.instr_lines {
            let text = Text::new(format!("{}: {}", instr.number, instr.instr));
            let text = if instr.is_green {
                text.color(Color::from_rgb(0.0, 1.0, 0.0))
            } else {
                text
            };
            let text = if instr.is_red {
                text.color(Color::from_rgb(1.0, 0.0, 0.0))
            } else {
                text
            };
            let button = Button::new(text)
                .on_press(Message::LineClicked(instr.number))
                .style(style::btn)
                // TODO: add style here to remove background?
                .padding(0);
            column = column.push(button);
        }
        let scrollable_content: Element<Message> = Element::from({
            Scrollable::with_direction(
                row![column // padding
                    .align_items(Alignment::Start)
                    .padding([0, 0, 0, 0])], // padding so scrollbar doesn't cover text
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

        column![
            config_pane,
            row![instruction_pane, register_pane, memory_pane]
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

    pub fn btn(_theme: &Theme, _status: Status) -> button::Style {
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
}
