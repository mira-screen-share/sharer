use iced::widget::button;
use iced::widget::button::Appearance;

use crate::gui::theme::Theme;

mod filled;

#[allow(dead_code)]
#[derive(Default)]
pub enum Style {
    #[default]
    Primary,
    Secondary,
    Danger,
}

/// Material Design 3 Filled Button
/// https://m3.material.io/components/buttons/specs#0b1b7bd2-3de8-431a-afa1-d692e2e18b0d
pub type Filled<'a> = filled::FilledButton<'a>;

pub enum Variant {
    Filled(filled::Style),
}

impl Default for Variant {
    fn default() -> Self {
        Self::Filled(Default::default())
    }
}

impl button::StyleSheet for Theme {
    type Style = Variant;

    fn active(&self, style: &Self::Style) -> Appearance {
        match style {
            Variant::Filled(style) => style.active(self),
        }
    }

    fn hovered(&self, style: &Self::Style) -> Appearance {
        match style {
            Variant::Filled(style) => style.hovered(self),
        }
    }

    fn pressed(&self, style: &Self::Style) -> Appearance {
        match style {
            Variant::Filled(style) => style.pressed(self),
        }
    }

    fn disabled(&self, style: &Self::Style) -> Appearance {
        match style {
            Variant::Filled(style) => style.disabled(self),
        }
    }
}
