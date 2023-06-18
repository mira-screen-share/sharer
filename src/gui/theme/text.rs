use iced::widget::text::{Appearance, StyleSheet};

use crate::gui::resource;
use crate::gui::theme::Theme;
use crate::gui::theme::icon::Icon;
use crate::gui::theme::widget::Text;

pub trait Themed: StyleSheet<Style=Theme> {}

impl StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: Self::Style) -> Appearance {
        Default::default()
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
