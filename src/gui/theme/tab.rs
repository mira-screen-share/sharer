use iced::{Background, Color, Length};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::container;
use iced_aw::{TabLabel, tabs};
use iced_aw::style::tab_bar;
use crate::gui::theme::color::ColorExt;

use crate::gui::theme::Theme;
use crate::gui::theme::widget::Element;

pub trait Tab {
    type Message;
    type Props;

    fn title(&self) -> String;

    fn tab_label(&self) -> TabLabel;

    fn view(&self, props: Self::Props) -> Element<'_, Self::Message> {
        container(self.content(props))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .padding(16)
            .into()
    }

    fn content(&self, props: Self::Props) -> Element<'_, Self::Message>;
}

impl tabs::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _style: Self::Style, is_active: bool) -> tab_bar::Appearance {
        let palette = self.palette();
        let tint = if is_active {
            palette.primary
        } else {
            palette.on_surface_variant
        };
        tab_bar::Appearance {
            background: Color::TRANSPARENT.into(),
            border_color: None,
            border_width: 0.0,
            tab_label_background: palette.surface.into(),
            tab_label_border_color: Default::default(),
            tab_label_border_width: 0.0,
            icon_color: tint,
            text_color: tint,
        }
    }

    fn hovered(&self, _style: Self::Style, is_active: bool) -> tab_bar::Appearance {
        let palette = self.palette();
        let base = self.active(_style, is_active);
        let tint = if is_active {
            palette.primary
        } else {
            palette.on_surface_variant
        };
        tab_bar::Appearance {
            tab_label_background: match base.tab_label_background {
                Background::Color(color) => color.mix(tint.with_alpha(0.12)).into(),
            },
            ..base
        }
    }
}

