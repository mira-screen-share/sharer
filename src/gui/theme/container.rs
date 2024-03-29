use iced::widget::container::{Appearance, StyleSheet};

use crate::gui::theme::{PaletteColor, Theme};

#[allow(dead_code)]
#[derive(Default)]
pub enum Style {
    /// Material Design 3 Outlined Card
    /// https://m3.material.io/components/cards/specs#9ad208b3-3d37-475c-a0eb-68cf845718f8
    #[default]
    Default,
    OutlinedCard,
    FilledEllipse(PaletteColor),
}

impl StyleSheet for Theme {
    type Style = Style;

    fn appearance(&self, style: &Self::Style) -> Appearance {
        let palette = self.palette();

        match style {
            Style::Default => Default::default(),
            Style::OutlinedCard => Appearance {
                background: palette.surface.into(),
                border_radius: 12.,
                border_width: 1.,
                border_color: palette.outline,
                ..Appearance::default()
            },
            Style::FilledEllipse(fill) => Appearance {
                background: palette.get_palette_color(fill).into(),
                border_radius: f32::MAX,
                border_width: 0.,
                ..Appearance::default()
            },
        }
    }
}
