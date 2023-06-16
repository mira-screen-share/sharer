use std::path::PathBuf;

use iced::widget::svg;
use iced::widget::svg::{Appearance, Handle, StyleSheet};

use crate::gui::theme;
use crate::gui::theme::Theme;

#[allow(dead_code)]
#[derive(Default)]
pub struct Svg {
    resource: String,
    color: Color,
}

#[allow(dead_code)]
#[derive(Default, Clone)]
pub enum Color {
    #[default]
    OnSurface,
    OnPrimary,
    OnSecondary,
    OnError,
}

impl Svg {
    pub fn new(resource: String) -> Self {
        Self {
            resource: resource.to_owned(),
            color: Default::default(),
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color.into();
        self
    }

    pub fn build(&self) -> theme::widget::Svg {
        svg(Handle::from_path(PathBuf::from(&self.resource)))
            .style(self.color.clone())
    }
}

impl StyleSheet for Theme {
    type Style = Color;

    fn appearance(&self, style: &Self::Style) -> Appearance {
        let palette = self.palette();

        Appearance {
            color: match style {
                Color::OnSurface => palette.on_surface,
                Color::OnPrimary => palette.on_primary,
                Color::OnSecondary => palette.on_secondary,
                Color::OnError => palette.on_error,
            }.into(),
        }
    }
}
