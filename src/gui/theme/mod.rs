use iced::{application, Color};
use iced::widget::text;

pub mod button;
pub mod color;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub background: Color,
    pub text: Color,
    pub primary: Color,
    pub secondary: Color,
    pub on_primary: Color,
    pub on_secondary: Color,
    pub on_danger: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub success: Color,
    pub danger: Color,
}

impl Palette {
    pub const LIGHT: Self = Self {
        background: Color::WHITE,
        text: Color::BLACK,
        on_primary: Color::WHITE,
        on_secondary: Color::BLACK,
        on_danger: Color::WHITE,
        on_surface: Color::BLACK,
        on_surface_variant: Color::WHITE,
        primary: Color::from_rgb(79. / 255., 55. / 255., 139. / 255.),
        secondary: Color::from_rgb(
            0x5E as f32 / 255.0,
            0x7C as f32 / 255.0,
            0xE2 as f32 / 255.0,
        ),
        success: Color::from_rgb(
            0x12 as f32 / 255.0,
            0x66 as f32 / 255.0,
            0x4F as f32 / 255.0,
        ),
        danger: Color::from_rgb(
            0xC3 as f32 / 255.0,
            0x42 as f32 / 255.0,
            0x3F as f32 / 255.0,
        ),
    };

    pub const DARK: Self = Self {
        background: Color::from_rgb(
            0x20 as f32 / 255.0,
            0x22 as f32 / 255.0,
            0x25 as f32 / 255.0,
        ),
        text: Color::from_rgb(0.90, 0.90, 0.90),
        on_primary: Color::from_rgb(0.90, 0.90, 0.90),
        on_secondary: Color::from_rgb(0.90, 0.90, 0.90),
        on_danger: Color::WHITE,
        on_surface: Color::WHITE,
        on_surface_variant: Color::from_rgb(202. / 255., 196. / 255., 208. / 255.),
        primary: Color::from_rgb(79. / 255., 55. / 255., 139. / 255.),
        secondary: Color::from_rgb(
            0x5E as f32 / 255.0,
            0x7C as f32 / 255.0,
            0xE2 as f32 / 255.0,
        ),
        success: Color::from_rgb(
            0x12 as f32 / 255.0,
            0x66 as f32 / 255.0,
            0x4F as f32 / 255.0,
        ),
        danger: Color::from_rgb(
            0xC3 as f32 / 255.0,
            0x42 as f32 / 255.0,
            0x3F as f32 / 255.0,
        ),
    };
}

impl Theme {
    pub fn palette(&self) -> Palette {
        match self {
            Self::Light => Palette::LIGHT,
            Self::Dark => Palette::DARK,
        }
    }
}

impl application::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _: &Self::Style) -> application::Appearance {
        let palette = self.palette();

        application::Appearance {
            background_color: palette.background,
            text_color: palette.text,
        }
    }
}

impl text::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _style: Self::Style) -> text::Appearance {
        text::Appearance {
            color: self.palette().text.into(),
        }
    }
}
