use iced::{Color, Element, Theme};
use iced::widget::{container, row, text, text_input, vertical_space};
use iced::widget::container::Appearance;

use crate::column_iced;
use crate::gui::message::Message;
use crate::gui::message::Message::Ignore;

pub fn material_card<'a>(
    head: &str,
    body: &str,
    width: iced::Length,
    trailing: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let content = row![
        column_iced![
            text(head).size(14).width(iced::Length::Shrink),
            vertical_space(6),
            selectable_text(body),
        ].width(
            match width {
                iced::Length::Fixed(x) => iced::Length::Fixed(x - 72.),
                iced::Length::Shrink => iced::Length::Shrink,
                iced::Length::Fill => iced::Length::Fill,
                iced::Length::FillPortion(_) => iced::Length::Shrink,
            }
        )
    ];
    container(
        if let Some(trailing) = trailing {
            content.push(trailing)
        } else {
            content
        }.align_items(iced::Alignment::Center)
            .spacing(8)
            .padding(16)
    ).width(width)
        .style(iced::theme::Container::Custom(Box::new(CardContainer {})))
        .into()
}

fn selectable_text<'a>(
    body: &str,
) -> Element<'a, Message> {
    text_input("", body)
        .size(18)
        .on_input(move |_| { Ignore })
        .style(iced::theme::TextInput::Custom(Box::new(SelectableText {})))
        .width(iced::Length::Fill)
        .padding(0)
        .into()
}

struct CardContainer {}

impl container::StyleSheet for CardContainer {
    type Style = Theme;

    fn appearance(&self, _: &Self::Style) -> Appearance {
        Appearance {
            background: Some(Color::from_rgb8(29, 27, 32).into()),
            border_color: Color::from_rgb8(73, 69, 79),
            border_radius: 12.,
            border_width: 1.,
            ..Appearance::default()
        }
    }
}

struct SelectableText {}

impl text_input::StyleSheet for SelectableText {
    type Style = Theme;

    fn active(&self, _: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: Color::from_rgba(0.0, 0.0, 0.0, 0.0).into(),
            border_radius: 0.0,
            border_width: 0.0,
            border_color: Color::BLACK,
            icon_color: Color::BLACK,
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }

    fn placeholder_color(&self, _: &Self::Style) -> Color {
        Color::from_rgba(0.0, 0.0, 0.0, 0.0).into()
    }

    fn value_color(&self, style: &Self::Style) -> Color {
        style.extended_palette().background.base.text
    }

    fn disabled_color(&self, _: &Self::Style) -> Color {
        Color::from_rgba(0.0, 0.0, 0.0, 0.0).into()
    }

    fn selection_color(&self, style: &Self::Style) -> Color {
        style.extended_palette().primary.weak.color
    }

    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }
}
