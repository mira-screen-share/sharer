use iced::{Background, Color};
use iced::widget::{button, image};
use iced::widget::button::{Appearance, StyleSheet};

use crate::gui::theme::button::{Buildable, Style, Style::*, Themed};
use crate::gui::theme::color::ColorExt;
use crate::gui::theme::Theme;
use crate::gui::theme::widget::Button;

/// Material Design 3 Icon Button
/// https://m3.material.io/components/icon-button/specs
pub struct IconButton<'a> {
    icon: &'a str,
    filled: bool,
    style: Style,
}

impl<'a> IconButton<'a> {
    pub fn new(icon: &'a str) -> Self {
        Self {
            icon,
            filled: false,
            style: Style::default(),
        }
    }

    pub fn filled(mut self, filled: bool) -> Self {
        self.filled = filled;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a> Buildable<'a> for IconButton<'static> {
    fn build<Message: 'a>(self) -> Button<'a, Message> {
        button(
            image(format!("resources/{}", self.icon))
                .width(iced::Length::Fixed(18.))
                .height(iced::Length::Fixed(18.)),
        ).style(Box::new(self) as _)
            .padding(11)
            .width(40)
            .height(40)
    }
}

impl Themed for IconButton<'_> {}

impl StyleSheet for IconButton<'_> {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> Appearance {
        let palette = style.palette();
        let partial = Appearance {
            border_radius: 20.0,
            ..Appearance::default()
        };

        if self.filled {
            let from = |background: Color, on_background: Color| Appearance {
                background: background.into(),
                text_color: on_background.into(),
                ..partial
            };

            match &self.style {
                Primary => from(palette.primary, palette.on_primary),
                Secondary => from(palette.secondary, palette.on_secondary),
                Danger => from(palette.danger, palette.on_danger),
            }
        } else {
            Appearance {
                text_color: palette.on_surface_variant.into(),
                ..partial
            }
        }
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        let palette = style.palette();
        let base = self.active(style);

        if self.filled {
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
        } else {
            Appearance {
                background: palette.on_surface_variant.with_alpha(0.12).into(),
                ..base
            }
        }
    }
}
