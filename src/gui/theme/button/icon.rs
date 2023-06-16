use iced::{Background, Color};
use iced::widget::{button};
use iced::widget::button::{Appearance, StyleSheet};

use crate::gui::theme::button::{Buildable, Style, Style::*, Themed};
use crate::gui::theme::color::ColorExt;
use crate::gui::theme::svg::Svg;
use crate::gui::theme::Theme;
use crate::gui::theme::widget::{Button};

/// Material Design 3 Icon Button
/// https://m3.material.io/components/icon-button/specs
pub struct IconButton {
    icon: String,
    filled: bool,
    style: Style,
}

#[allow(dead_code)]
impl IconButton {
    pub fn new(icon: &str) -> Self {
        Self {
            icon: icon.into(),
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

impl<'a> Buildable<'a> for IconButton {
    fn build<Message: 'a>(self) -> Button<'a, Message> {
        button(
            Svg::new(format!("resources/{}", self.icon))
                .build()
                .width(iced::Length::Fixed(18.))
                .height(iced::Length::Fixed(18.)),
        ).style(Box::new(self) as _)
            .padding(11)
            .width(40)
            .height(40)
    }
}

impl Themed for IconButton {}

impl StyleSheet for IconButton {
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
                Danger => from(palette.error, palette.on_error),
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
                Danger => palette.on_error,
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
