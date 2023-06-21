use std::path::PathBuf;

use iced::widget::svg;
use iced::widget::svg::{Appearance, Handle, StyleSheet};

use crate::gui::theme::Theme;
use crate::gui::{resource, theme};

#[allow(dead_code)]
#[derive(Default)]
pub struct Svg {
    svg: String,
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

#[allow(dead_code)]
impl Svg {
    pub fn new(svg: String) -> Self {
        Self {
            svg,
            color: Default::default(),
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn build(&self) -> theme::widget::Svg {
        svg(Handle::from_path(PathBuf::from(resource::get(
            self.svg.clone(),
        ))))
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
            }
            .into(),
        }
    }
}
