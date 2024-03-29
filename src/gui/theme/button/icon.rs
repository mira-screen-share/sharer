use std::default::Default;

use iced::widget::button;
use iced::widget::button::{Appearance, StyleSheet};
use iced::{Background, Color};

use crate::gui::theme::button::{Style, Style::*, Themed};
use crate::gui::theme::color::ColorExt;
use crate::gui::theme::icon::Icon;
use crate::gui::theme::text::icon;
use crate::gui::theme::widget::Button;
use crate::gui::theme::Theme;

/// Material Design 3 Icon Button
/// https://m3.material.io/components/icon-button/specs
pub struct IconButton {
    icon: Icon,
    filled: bool,
    style: Style,
}

#[allow(dead_code)]
impl IconButton {
    pub fn new(icon: Icon) -> Self {
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

    pub fn build<'a, Message: 'a>(self) -> Button<'a, Message> {
        button(icon(self.icon.clone()).size(18))
            .style(Box::new(self) as _)
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
                text_color: on_background,
                ..partial
            };

            match &self.style {
                Default => from(palette.surface, palette.on_surface_variant),
                Primary => from(palette.primary, palette.on_primary),
                Secondary => from(palette.secondary, palette.on_secondary),
                Danger => from(palette.error, palette.on_error),
                Success => from(palette.success, palette.on_success),
            }
        } else {
            Appearance {
                text_color: match &self.style {
                    Default => palette.on_surface_variant,
                    Primary => palette.primary,
                    Secondary => palette.secondary,
                    Danger => palette.error,
                    Success => palette.success,
                },
                ..partial
            }
        }
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        let palette = style.palette();
        let base = self.active(style);

        if self.filled {
            let state = match self.style {
                Default => palette.on_surface,
                Primary => palette.on_primary,
                Secondary => palette.on_secondary,
                Danger => palette.on_error,
                Success => palette.on_success,
            };

            Appearance {
                background: base.background.map(|background| match background {
                    Background::Color(color) => {
                        Background::Color(color.mix(state.with_alpha(0.12)))
                    }
                }),
                ..base
            }
        } else {
            Appearance {
                background: match &self.style {
                    Default => palette.on_surface_variant,
                    Primary => palette.primary,
                    Secondary => palette.secondary,
                    Danger => palette.error,
                    Success => palette.success,
                }
                .with_alpha(0.12)
                .into(),
                ..base
            }
        }
    }
}
