use iced::{Background, Color};
use iced::widget::{button, horizontal_space, row, text};
use iced::widget::button::{Appearance, StyleSheet};

use crate::gui::resource;
use crate::gui::theme::button::{Style, Themed};
use crate::gui::theme::button::Style::{Danger, Primary, Secondary};
use crate::gui::theme::color::ColorExt;
use crate::gui::theme::icon::Icon;
use crate::gui::theme::Theme;
use crate::gui::theme::widget::Button;

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
                text(self.icon.clone()).font(resource::ICON_FONT).size(24),
                horizontal_space(12),
                text(self.text.clone()).size(16)
            ].align_items(iced::Alignment::Center)
        ).style(Box::new(self) as _)
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
            text_color: on_background.into(),
            ..partial
        };

        match self.style {
            Primary => from(palette.primary, palette.on_primary),
            Secondary => from(palette.secondary, palette.on_secondary),
            Danger => from(palette.error, palette.on_error),
        }
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        let palette = style.palette();
        let base = self.active(style);
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
    }
}
