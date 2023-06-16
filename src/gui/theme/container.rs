use iced::widget::container::{Appearance, StyleSheet};

use crate::gui::theme::Theme;

#[allow(dead_code)]
#[derive(Default)]
pub enum Style {
    #[default]
    Card,
}

impl StyleSheet for Theme {
    type Style = Style;

    fn appearance(&self, style: &Self::Style) -> Appearance {
        let palette = self.palette();

        match style {
            Style::Card => Appearance {
                background: palette.surface.into(),
                border_radius: 12.,
                border_width: 1.,
                border_color: palette.outline,
                ..Appearance::default()
            },
        }
    }
}
