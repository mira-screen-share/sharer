use crate::gui::theme::Theme;
use iced::widget::pick_list::{Appearance, StyleSheet};

impl StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: &<Self as StyleSheet>::Style) -> Appearance {
        let palette = self.palette();
        Appearance {
            text_color: palette.on_surface,
            placeholder_color: palette.on_surface,
            handle_color: palette.on_surface,
            background: palette.surface.into(),
            border_width: 1.,
            border_radius: 4.,
            border_color: palette.outline,
        }
    }

    fn hovered(&self, style: &<Self as StyleSheet>::Style) -> Appearance {
        self.active(style)
    }
}
