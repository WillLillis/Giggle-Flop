// TODO: Start work on cutting out unnecessary parts from this demo...
use std::borrow::Cow;

use iced::widget::scrollable::Properties;
use iced::widget::{
    button, column, container, horizontal_space, progress_bar, radio, row, scrollable, slider,
    text, vertical_space, Scrollable,
};
use iced::{Alignment, Border, Color, Command, Element, Length, Theme};

use once_cell::sync::Lazy;

use crate::memory;
use crate::system::system::System;

static SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);

pub fn enter_ui() -> iced::Result {
    iced::program("Giggle-Flop", GiggleFlopUI::update, GiggleFlopUI::view)
        .theme(GiggleFlopUI::theme)
        .run()
}

struct GiggleFlopUI {
    current_scroll_offset: scrollable::RelativeOffset,
    alignment: scrollable::Alignment,
    system: System,
}

#[derive(Debug, Clone, Eq, PartialEq, Copy)]
enum Direction {
    Vertical,
    Horizontal,
    Multi,
}

#[derive(Debug, Clone)]
enum Message {
    SwitchDirection(Direction),
    AlignmentChanged(scrollable::Alignment),
    ScrollToBeginning,
    ScrollToEnd,
    Scrolled(scrollable::Viewport),
}

impl GiggleFlopUI {
    fn new() -> Self {
        GiggleFlopUI {
            current_scroll_offset: scrollable::RelativeOffset::START,
            alignment: scrollable::Alignment::Start,
            system: System::default(),
        }
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SwitchDirection(direction) => {
                self.current_scroll_offset = scrollable::RelativeOffset::START;

                scrollable::snap_to(SCROLLABLE_ID.clone(), self.current_scroll_offset)
            }
            Message::AlignmentChanged(alignment) => {
                self.current_scroll_offset = scrollable::RelativeOffset::START;
                self.alignment = alignment;

                scrollable::snap_to(SCROLLABLE_ID.clone(), self.current_scroll_offset)
            }
            Message::ScrollToBeginning => {
                self.current_scroll_offset = scrollable::RelativeOffset::START;

                scrollable::snap_to(SCROLLABLE_ID.clone(), self.current_scroll_offset)
            }
            Message::ScrollToEnd => {
                self.current_scroll_offset = scrollable::RelativeOffset::END;

                scrollable::snap_to(SCROLLABLE_ID.clone(), self.current_scroll_offset)
            }
            Message::Scrolled(viewport) => {
                self.current_scroll_offset = viewport.relative_offset();

                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        let scroll_alignment_controls = column![
            text("Scrollable alignment:"),
            radio(
                "Start",
                scrollable::Alignment::Start,
                Some(self.alignment),
                Message::AlignmentChanged,
            ),
            radio(
                "End",
                scrollable::Alignment::End,
                Some(self.alignment),
                Message::AlignmentChanged,
            )
        ]
        .spacing(10);

        let scroll_controls = row![scroll_alignment_controls, "This is a test",].spacing(20);

        let scroll_to_end_button = || {
            button("Scroll to end")
                .padding(10)
                .on_press(Message::ScrollToEnd)
        };

        let scroll_to_beginning_button = || {
            button("Scroll to beginning")
                .padding(10)
                .on_press(Message::ScrollToBeginning)
        };

        let scrollable_content: Element<Message> = Element::from({
            Scrollable::with_direction(
                column![
                    scroll_to_end_button(),
                    text(
                        // BUG: This is a test...
                        self.system.memory_system.get_level(0).unwrap()
                    ),
                    scroll_to_beginning_button(),
                ]
                .align_items(Alignment::Center)
                .padding([40, 0, 40, 0])
                .spacing(40),
                scrollable::Direction::Vertical(
                    Properties::new()
                        .width(10)
                        .margin(10)
                        .scroller_width(10)
                        .alignment(self.alignment),
                ),
            )
            .width(Length::Fill) // TODO: Don't fill entire window...
            .height(Length::Fill)
            .id(SCROLLABLE_ID.clone())
            .on_scroll(Message::Scrolled)
        });

        let progress_bar: Element<Message> =
            progress_bar(0.0..=1.0, self.current_scroll_offset.y).into();

        let content: Element<Message> = column![scroll_controls, scrollable_content, progress_bar]
            .align_items(Alignment::Center)
            .spacing(10)
            .into();

        container(content).padding(20).center_x().center_y().into()
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

fn progress_bar_custom_style(theme: &Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: theme.extended_palette().background.strong.color.into(),
        bar: Color::from_rgb8(250, 85, 134).into(),
        border: Border::default(),
    }
}
