use iced::Color;
use iced::widget::text_input::{Appearance, StyleSheet};

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
                background: Color::from_rgba(0.0, 0.0, 0.0, 0.0).into(),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Color::BLACK,
                icon_color: Color::BLACK,
            },
        }
    }

    fn focused(&self, style: &Self::Style) -> Appearance {
        match style {
            Style::Selectable => self.active(style)
        }
    }

    fn placeholder_color(&self, style: &Self::Style) -> Color {
        match style {
            Style::Selectable => Color::from_rgba(0.0, 0.0, 0.0, 0.0).into(),
        }
    }

    fn value_color(&self, style: &Self::Style) -> Color {
        match style {
            Style::Selectable => self.palette().text,
        }
    }

    fn disabled_color(&self, style: &Self::Style) -> Color {
        match style {
            Style::Selectable => Color::from_rgba(0.0, 0.0, 0.0, 0.0).into(),
        }
    }

    fn selection_color(&self, style: &Self::Style) -> Color {
        match style {
            Style::Selectable => self.palette().primary,
        }
    }

    fn disabled(&self, style: &Self::Style) -> Appearance {
        match style {
            Style::Selectable => self.active(style),
        }
    }
}
