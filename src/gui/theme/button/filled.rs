use iced::{Background, Color};
use iced::widget::{button, horizontal_space, image, row, text};
use iced::widget::button::{Appearance, StyleSheet};

use crate::gui::theme::button::{Buildable, Style, Themed};
use crate::gui::theme::button::Style::{Danger, Primary, Secondary};
use crate::gui::theme::color::ColorExt;
use crate::gui::theme::Theme;
use crate::gui::theme::widget::Button;

/// Material Design 3 Filled Button
/// https://m3.material.io/components/buttons/specs#0b1b7bd2-3de8-431a-afa1-d692e2e18b0d
pub struct FilledButton<'a> {
    text: &'a str,
    icon: Option<&'a str>,
    style: Style,
}

impl<'a> FilledButton<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            icon: None,
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }
}

impl<'a> Buildable<'a> for FilledButton<'static> {
    fn build<Message: 'a>(self) -> Button<'a, Message> {
        if let Some(icon) = self.icon {
            button(
                row![
                    image(format!("resources/{}", icon))
                        .width(iced::Length::Fixed(18.))
                        .height(iced::Length::Fixed(18.)),
                    horizontal_space(8),
                    text(self.text).size(16)
                ].align_items(iced::Alignment::Center)
            ).padding([0, 24, 0, 19])
        } else {
            button(
                row![
                    text(self.text).size(16)
                ].align_items(iced::Alignment::Center)
            ).padding([0, 24, 0, 24])
        }.style(Box::new(self) as _)
            .height(40)
    }
}

impl Themed for FilledButton<'_> {}

impl StyleSheet for FilledButton<'_> {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> Appearance {
        let palette = style.palette();
        let partial = Appearance {
            border_radius: 32.0,
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
