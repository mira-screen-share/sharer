use iced::widget::text::{Appearance, StyleSheet};

use crate::gui::resource;
use crate::gui::theme::icon::Icon;
use crate::gui::theme::widget::Text;
use crate::gui::theme::Theme;

pub trait Themed: StyleSheet<Style = Theme> {}

#[derive(Clone, Copy, Debug, Default)]
pub enum Style {
    #[default]
    Text,
    Label,
}

impl StyleSheet for Theme {
    type Style = Style;

    fn appearance(&self, style: Self::Style) -> Appearance {
        Appearance {
            color: match style {
                Style::Text => None,
                Style::Label => Some(self.palette().on_surface_variant),
            },
        }
    }
}

pub fn text<'a>(text: impl ToString) -> Text<'a> {
    iced::widget::text(text).font(resource::font::BARLOW)
}

pub fn bold<'a>(text: impl ToString) -> Text<'a> {
    iced::widget::text(text).font(resource::font::BARLOW_BOLD)
}

pub fn icon<'a>(icon: Icon) -> Text<'a> {
    text(icon).font(resource::font::ICON)
}
