use iced::widget::button::{Appearance, StyleSheet};
use iced::widget::{button, horizontal_space, row};
use iced::{Background, Color};
use std::default::Default;

use crate::gui::theme::button::Style::*;
use crate::gui::theme::button::{Style, Themed};
use crate::gui::theme::color::ColorExt;
use crate::gui::theme::icon::Icon;
use crate::gui::theme::text::{bold, icon};
use crate::gui::theme::widget::Button;
use crate::gui::theme::Theme;

/// Material Design 3 Extended FAB
/// https://m3.material.io/components/extended-fab/specs
pub struct FAB {
    text: String,
    icon: Icon,
    style: Style,
}

#[allow(dead_code)]
impl FAB {
    pub fn new(text: &str, icon: Icon) -> Self {
        Self {
            text: text.into(),
            icon,
            style: Style::default(),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn build<'a, Message: 'a>(self) -> Button<'a, Message> {
        button(
            row![
                icon(self.icon.clone()).size(24),
                horizontal_space(12),
                bold(self.text.clone()).size(16)
            ]
            .align_items(iced::Alignment::Center),
        )
        .style(Box::new(self) as _)
        .padding([0, 16, 0, 16])
        .height(56)
    }
}

impl Themed for FAB {}

impl StyleSheet for FAB {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> Appearance {
        let palette = style.palette();
        let partial = Appearance {
            border_radius: 16.0,
            ..Appearance::default()
        };
        let from = |background: Color, on_background: Color| Appearance {
            background: background.into(),
            text_color: on_background,
            ..partial
        };

        match self.style {
            Default => from(palette.surface, palette.on_surface),
            Primary => from(palette.primary, palette.on_primary),
            Secondary => from(palette.secondary, palette.on_secondary),
            Danger => from(palette.error, palette.on_error),
            Success => from(palette.success, palette.on_success),
        }
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        let palette = style.palette();
        let base = self.active(style);
        let state = match self.style {
            Default => palette.on_surface,
            Primary => palette.on_primary,
            Secondary => palette.on_secondary,
            Danger => palette.on_error,
            Success => palette.on_success,
        };

        Appearance {
            background: base.background.map(|background| match background {
                Background::Color(color) => Background::Color(color.mix(state.with_alpha(0.12))),
            }),
            ..base
        }
    }
}
