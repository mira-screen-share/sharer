use iced::widget::{container, row, text, text_input, vertical_space};

use crate::column_iced;
use crate::gui;
use crate::gui::message::Message;
use crate::gui::message::Message::Ignore;
use crate::gui::theme::button::{Buildable, IconButton};
use crate::gui::theme::widget::Element;

pub fn invite_info_card<'a>(
    head: &str,
    body: &str,
    on_copy: Message,
    width: f32,
) -> Element<'a, Message> {
    container(
        row![
            column_iced![
                text(head).size(14).width(iced::Length::Shrink),
                vertical_space(6),
                text_input("", body)
                    .style(gui::theme::text_input::Style::Selectable)
                    .size(18)
                    .on_input(move |_| { Ignore })
                    .width(iced::Length::Fill)
                    .padding(0)
            ]
            .width(iced::Length::Fixed(width - 80.)),
            IconButton::new("copy.svg").build().on_press(on_copy)
        ]
        .align_items(iced::Alignment::Center)
        .spacing(8)
        .padding(16),
    )
    .style(gui::theme::container::Style::OutlinedCard)
    .width(width)
    .into()
}
