use iced::{Background, Color};
use iced::widget::{button, horizontal_space, image, row, text};
use iced::widget::button::{Appearance, StyleSheet};

use crate::gui::theme::button::{Buildable, Style, Themed};
use crate::gui::theme::button::Style::{Danger, Primary, Secondary};
use crate::gui::theme::color::ColorExt;
use crate::gui::theme::Theme;
use crate::gui::widget::Button;

/// Material Design 3 Extended FAB
/// https://m3.material.io/components/extended-fab/specs
pub struct FAB<'a> {
    text: &'a str,
    icon: &'a str,
    style: Style,
}

impl<'a> FAB<'a> {
    pub fn new(text: &'a str, icon: &'a str) -> Self {
        Self {
            text,
            icon,
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a> Buildable<'a> for FAB<'static> {
    fn build<Message: 'a>(self) -> Button<'a, Message> {
        button(
            row![
                image(format!("resources/{}", self.icon))
                    .width(iced::Length::Fixed(21.))
                    .height(iced::Length::Fixed(21.)),
                horizontal_space(12),
                text(self.text).size(16)
            ].align_items(iced::Alignment::Center)
        ).style(Box::new(self) as _)
            .padding([0, 16, 0, 16])
            .height(56)
    }
}

impl Themed for FAB<'_> {}

impl StyleSheet for FAB<'_> {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> Appearance {
        let palette = style.palette();
        let partial = Appearance {
            border_radius: 16.0,
            ..Appearance::default()
        };
        let from = |background: Color, on_background: Color| Appearance {
            background: background.into(),
            text_color: on_background.into(),
            ..partial
        };

        match self.style {
            Primary => from(palette.primary, palette.on_primary),
            Secondary => from(palette.secondary, palette.on_secondary),
            Danger => from(palette.danger, palette.on_danger),
        }
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        let palette = style.palette();
        let base = self.active(style);
        let state = match self.style {
            Primary => palette.on_primary,
            Secondary => palette.on_secondary,
            Danger => palette.on_danger,
        };

        Appearance {
            background: base.background.map(|background| match background {
                Background::Color(color) =>
                    Background::Color(color.mix(state.with_alpha(0.12))),
            }),
            ..base
        }
    }
}
