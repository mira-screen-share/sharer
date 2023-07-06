use crate::gui::theme::Theme;
use iced::widget::scrollable::{Scrollbar, Scroller, StyleSheet};

impl StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: &Self::Style) -> Scrollbar {
        let palette = self.palette();
        Scrollbar {
            background: palette.surface.into(),
            border_radius: 4.,
            border_width: 1.,
            border_color: palette.outline,
            scroller: Scroller {
                color: palette.on_surface,
                border_radius: 4.,
                border_width: 1.,
                border_color: palette.outline,
            },
        }
    }

    fn hovered(&self, style: &Self::Style, _is_mouse_over_scrollbar: bool) -> Scrollbar {
        self.active(style)
    }
}
