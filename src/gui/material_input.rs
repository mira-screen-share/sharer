use iced::Element;
use iced::widget::{row, text, text_input};

use crate::gui::message::Message;

#[allow(dead_code)]
pub fn material_input<'a>(
    name: &str,
    value: &str,
    message: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        text(format!("{}: ", name)),
        text_input(name,value)
        .on_input(move |value| { message(value) })
        .width(iced::Length::Fixed(100.)),
    ].width(iced::Length::Shrink)
        .align_items(iced::Alignment::Center)
        .padding(10)
        .spacing(10)
        .into()
}
