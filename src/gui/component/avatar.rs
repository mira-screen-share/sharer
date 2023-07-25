use iced::alignment::{Horizontal, Vertical};
use iced::widget::container;

use crate::gui::app::Message;
use crate::gui::theme;
use crate::gui::theme::text::bold;
use crate::gui::theme::widget::Element;
use crate::gui::theme::{text, PaletteColor};

pub fn avatar(fill: PaletteColor, content: Element<Message>) -> Element<Message> {
    container(content)
        .width(iced::Length::Fixed(40.))
        .height(iced::Length::Fixed(40.))
        .style(theme::container::Style::FilledEllipse(fill))
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .into()
}

pub fn text_avatar<'a>(fill: PaletteColor, text: impl ToString) -> Element<'a, Message> {
    avatar(
        fill,
        bold(text)
            .style(text::Style::Colored(fill.on()))
            .size(20.)
            .into(),
    )
}
