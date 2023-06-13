use iced::{Background, Color, Element, Padding, Theme};
use iced::Length::Fixed;
use iced::theme::Button;
use iced::widget::{button, horizontal_space, image, row, text};
use iced::widget::button::Appearance;

use crate::column_iced;
use crate::gui::message::Message;

#[allow(dead_code)]
pub fn material_button<'a>(
    button_text: &str,
    on_press: Message,
    style: Button,
) -> Element<'a, Message> {
    button(text(button_text).size(16))
        .style(Button::Custom(Box::new(MaterialButton { style })))
        .padding([10, 24, 10, 24])
        .on_press(on_press)
        .into()
}

pub fn material_fab<'a, P: Into<Padding>>(
    button_text: &str,
    on_press: Message,
    style: Button,
    icon: Option<&str>,
    padding: P,
) -> Element<'a, Message> {
    button(
        row![
            if let Some(icon) = icon {
                Element::from(
                    row![
                        image(format!("resources/{}", icon))
                            .width(iced::Length::Fixed(18.))
                            .height(iced::Length::Fixed(18.)),
                        horizontal_space(15),
                    ]
                )
            } else { Element::from(row![]) },
            text(button_text).size(16)
        ]
    ).style(Button::Custom(Box::new(MaterialFAB { style })))
        .padding(padding)
        .on_press(on_press)
        .into()
}

pub fn material_icon_button<'a>(
    icon: &str,
    on_press: Message,
) -> Element<'a, Message> {
    button(
        column_iced![
            image(format!("resources/{}", icon))
                .width(iced::Length::Fixed(18.))
                .height(iced::Length::Fixed(18.))
        ].width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .align_items(iced::Alignment::Center)
            .padding(7)
    ).width(Fixed(32.))
        .height(Fixed(32.))
        .style(Button::Custom(Box::new(MaterialIconButton {})))
        .padding(0)
        .on_press(on_press)
        .into()
}

pub fn active(base: Appearance, style: &Button, theme: &Theme) -> Appearance {
    let from_pair = |pair: iced::theme::palette::Pair| Appearance {
        background: Some(pair.color.into()),
        text_color: pair.text,
        ..base
    };

    let palette = theme.extended_palette();

    match style {
        Button::Primary => from_pair(palette.primary.strong),
        Button::Secondary => from_pair(palette.secondary.base),
        Button::Positive => from_pair(palette.success.base),
        Button::Destructive => from_pair(palette.danger.base),
        Button::Text | Button::Custom(_) => Appearance {
            text_color: palette.background.base.text,
            ..base
        },
    }
}

pub fn hovered(base: Appearance) -> Appearance {
    Appearance {
        background: base.background.map(|background| match background {
            Background::Color(color) => Background::Color(Color {
                r: color.r - 0.1,
                g: color.g - 0.1,
                b: color.b - 0.1,
                a: color.a,
            }),
        }),
        ..base
    }
}

struct MaterialButton {
    style: Button,
}

impl button::StyleSheet for MaterialButton {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> Appearance {
        active(Appearance {
            border_radius: 32.0,
            ..Appearance::default()
        }, &self.style, style)
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        hovered(self.active(style))
    }
}

struct MaterialFAB {
    style: Button,
}

impl button::StyleSheet for MaterialFAB {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> Appearance {
        active(Appearance {
            border_radius: 16.0,
            ..Appearance::default()
        }, &self.style, style)
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        hovered(self.active(style))
    }
}

struct MaterialIconButton {}

impl button::StyleSheet for MaterialIconButton {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> Appearance {
        active(Appearance {
            border_width: 0.,
            border_radius: 16.,
            ..Appearance::default()
        }, &Button::Text, style)
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        let base = self.active(style);
        Appearance {
            background: Some(Background::Color(Color::from_rgba(1., 1., 1., 0.1))),
            ..base
        }
    }

    fn pressed(&self, style: &Self::Style) -> Appearance {
        let base = self.active(style);
        Appearance {
            background: Some(Background::Color(Color::from_rgba(1., 1., 1., 0.3))),
            ..base
        }
    }
}
