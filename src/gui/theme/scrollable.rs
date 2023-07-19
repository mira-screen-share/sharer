use crate::gui::theme::color::ColorExt;
use crate::gui::theme::Theme;
use iced::widget::scrollable::{Scrollbar, Scroller, StyleSheet};

impl StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: &Self::Style) -> Scrollbar {
        let palette = self.palette();
        Scrollbar {
            background: palette.surface.into(),
            border_radius: f32::MAX,
            border_width: 0.,
            border_color: palette.outline,
            scroller: Scroller {
                color: palette.on_surface.with_alpha(0.1),
                border_radius: f32::MAX,
                border_width: 0.,
                border_color: palette.outline,
            },
        }
    }

    fn hovered(&self, _style: &Self::Style, _is_mouse_over_scrollbar: bool) -> Scrollbar {
        let palette = self.palette();
        Scrollbar {
            background: palette.surface.into(),
            border_radius: f32::MAX,
            border_width: 0.,
            border_color: palette.outline,
            scroller: Scroller {
                color: palette.on_surface,
                border_radius: f32::MAX,
                border_width: 0.,
                border_color: palette.outline,
            },
        }
    }
}
