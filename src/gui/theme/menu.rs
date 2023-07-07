use crate::gui::theme::Theme;
use iced::overlay::menu::{Appearance, StyleSheet};

impl StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> Appearance {
        Appearance {
            text_color: self.palette().on_surface,
            background: self.palette().surface.into(),
            border_width: 1.,
            border_radius: 4.,
            border_color: self.palette().outline,
            selected_text_color: self.palette().on_primary,
            selected_background: self.palette().primary.into(),
        }
    }
}
