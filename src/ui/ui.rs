use cfonts::render::RenderedString;
use iced::widget::scrollable::Properties;
use iced::widget::{
    button, horizontal_rule, horizontal_space, pane_grid, vertical_space, PaneGrid,
};
use iced::widget::{column, container, pick_list, row, scrollable, text, Scrollable};
use iced::{Alignment, Color, Command, Element, Length, Theme};

use once_cell::sync::Lazy;
use strum::IntoEnumIterator;

use crate::register::register_system::RegisterGroup;
use crate::system::system::System;

static SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);

pub fn enter_ui() -> iced::Result {
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
}

#[derive(Debug, Clone)]
enum Message {
    Scrolled(scrollable::Viewport),
    SelectMemoryLevel(usize),
    SelectRegisterGroup(RegisterGroup),
    AdvanceClock,
    // maybe delete
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
}

#[derive(Clone, Copy)]
struct Pane {
    id: usize,
    pub is_pinned: bool,
}

impl Pane {
    fn new(id: usize) -> Self {
        Self {
            id,
            is_pinned: false,
        }
    }
}

impl GiggleFlopUI {
    fn new() -> Self {
        let system = System::default();
        let memory_levels = (0..system.memory_system.num_levels()).into_iter().collect();
        let register_groups = {
            let mut groups = Vec::new();
            for group in RegisterGroup::iter() {
                groups.push(group);
            }
            groups
        };
        let (panes, _) = pane_grid::State::new(Pane::new(0));
        GiggleFlopUI {
            memory_levels,
            current_memory_level: system.memory_system.num_levels() - 1,
            register_groups,
            current_register_group: RegisterGroup::General,
            current_scroll_offset: scrollable::RelativeOffset::START,
            panes,
            focus: None,
            system,
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
                self.system.step();
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
            Message::Dragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                self.panes.drop(pane, target);
                Command::none()
            }
            Message::Dragged(_) => Command::none(),
        }
    }

    fn get_code_element(&self) -> Element<Message> {
        let scrollable_content: Element<Message> = Element::from({
            let step_button = || {
                button("Click to advance")
                    .padding(10)
                    .on_press(Message::AdvanceClock)
            };
            let code_text = format!("TODO: Code goes here...Clock: {}", self.system.clock);
            Scrollable::with_direction(
                row![
                    //column![text("TODO: Code goes here..."), step_button()]
                    column![text(code_text), step_button()]
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

        let content: Element<Message> = column![scrollable_content]
            .align_items(Alignment::Start)
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
        .placeholder("Select memory level...");

        // TODO: Use pretty printing crate?
        let content: Element<Message> = column![mem_level_select, scrollable_content]
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
                .style(style::title_bar_style);

            pane_grid::Content::new(self.get_memory_element())
                .title_bar(title_bar)
                .style(style::pane_style)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .on_click(Message::Clicked)
        .on_drag(Message::Dragged)
        .on_resize(10, Message::Resized);

        let memory_pane: Element<Message> = container(memory_block)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .into();

        let register_block = PaneGrid::new(&self.panes, |_id, _pane, _is_maximized| {
            let title = row!["Register System"].spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .padding(10)
                .style(style::title_bar_style);

            pane_grid::Content::new(self.get_register_element())
                .title_bar(title_bar)
                .style(style::pane_style)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .on_click(Message::Clicked)
        .on_drag(Message::Dragged)
        .on_resize(10, Message::Resized);

        let register_pane: Element<Message> = container(register_block)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .into();

        let code_block = PaneGrid::new(&self.panes, |_id, _pane, _is_maximized| {
            let title = row!["Source Code"].spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .padding(10)
                .style(style::title_bar_style);

            pane_grid::Content::new(self.get_code_element())
                .title_bar(title_bar)
                .style(style::pane_style)
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .on_click(Message::Clicked)
        .on_drag(Message::Dragged)
        .on_resize(10, Message::Resized);

        let code_pane: Element<Message> = container(code_block)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(10)
            .into();



        row![column![register_pane, memory_pane], column![code_pane]].into()
    }

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
    use iced::widget::container;
    use iced::{Border, Theme};

    pub fn title_bar_style(theme: &Theme) -> container::Style {
        let palette = theme.extended_palette();

        container::Style {
            text_color: Some(palette.primary.strong.text),
            background: Some(palette.primary.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn pane_style(theme: &Theme) -> container::Style {
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
}
