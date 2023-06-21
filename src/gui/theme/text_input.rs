use iced::widget::text_input::{Appearance, StyleSheet};
use iced::Color;

use crate::gui::theme::color::ColorExt;
use crate::gui::theme::Theme;

#[allow(dead_code)]
#[derive(Default)]
pub enum Style {
    #[default]
    Selectable,
}

impl StyleSheet for Theme {
    type Style = Style;

    fn active(&self, style: &Self::Style) -> Appearance {
        match style {
            Style::Selectable => Appearance {
                background: Color::TRANSPARENT.into(),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::BLACK,
                icon_color: Color::BLACK,
            },
        }
    }

    fn focused(&self, style: &Self::Style) -> Appearance {
        match style {
            Style::Selectable => self.active(style),
        }
    }

    fn placeholder_color(&self, style: &Self::Style) -> Color {
        match style {
            Style::Selectable => Color::TRANSPARENT,
        }
    }

    fn value_color(&self, style: &Self::Style) -> Color {
        match style {
            Style::Selectable => self.palette().on_background,
        }
    }

    fn disabled_color(&self, style: &Self::Style) -> Color {
        match style {
            Style::Selectable => Color::TRANSPARENT,
        }
    }

    fn selection_color(&self, style: &Self::Style) -> Color {
        match style {
            Style::Selectable => self.palette().primary.with_alpha(0.2),
        }
    }

    fn disabled(&self, style: &Self::Style) -> Appearance {
        match style {
            Style::Selectable => self.active(style),
        }
    }
}
