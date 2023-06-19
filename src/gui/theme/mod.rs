use iced::{application, Color};

use crate::gui::theme::color::ColorExt;

pub mod button;
pub mod color;
pub mod container;
pub mod svg;
pub mod tab;
pub mod icon;
pub mod text;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Default, Copy)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

/// Material Design 3 Color System
/// https://m3.material.io/styles/color/the-color-system/tokens#7fd4440e-986d-443f-8b3a-4933bff16646
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub primary: Color,
    pub primary_container: Color,
    pub on_primary: Color,
    pub on_primary_container: Color,
    pub inverse_primary: Color,
    pub secondary: Color,
    pub secondary_container: Color,
    pub on_secondary: Color,
    pub on_secondary_container: Color,
    pub tertiary: Color,
    pub tertiary_container: Color,
    pub on_tertiary: Color,
    pub on_tertiary_container: Color,
    pub surface: Color,
    pub surface_dim: Color,
    pub surface_bright: Color,
    pub surface_container_lowest: Color,
    pub surface_container_low: Color,
    pub surface_container: Color,
    pub surface_container_high: Color,
    pub surface_container_highest: Color,
    pub surface_variant: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub inverse_surface: Color,
    pub inverse_on_surface: Color,
    pub background: Color,
    pub on_background: Color,
    pub error: Color,
    pub error_container: Color,
    pub on_error: Color,
    pub on_error_container: Color,
    pub outline: Color,
    pub outline_variant: Color,
    pub shadow: Color,
    pub surface_tint: Color,
    pub scrim: Color,
}

impl Palette {
    pub fn light() -> Self {
        Self {
            primary: Color::from_hex("6750A4"),
            primary_container: Color::from_hex("EADDFF"),
            on_primary: Color::from_hex("FFFFFF"),
            on_primary_container: Color::from_hex("21005E"),
            inverse_primary: Color::from_hex("D0BCFF"),
            secondary: Color::from_hex("625B71"),
            secondary_container: Color::from_hex("E8DEF8"),
            on_secondary: Color::from_hex("FFFFFF"),
            on_secondary_container: Color::from_hex("1E192B"),
            tertiary: Color::from_hex("7D5260"),
            tertiary_container: Color::from_hex("FFD8E4"),
            on_tertiary: Color::from_hex("FFFFFF"),
            on_tertiary_container: Color::from_hex("370B1E"),
            surface: Color::from_hex("FEF7FF"),
            surface_dim: Color::from_hex("DED8E1"),
            surface_bright: Color::from_hex("FEF7FF"),
            surface_container_lowest: Color::from_hex("FFFFFF"),
            surface_container_low: Color::from_hex("F7F2FA"),
            surface_container: Color::from_hex("F3EDF7"),
            surface_container_high: Color::from_hex("ECE6F0"),
            surface_container_highest: Color::from_hex("E6E0E9"),
            surface_variant: Color::from_hex("E7E0EC"),
            on_surface: Color::from_hex("1C1B1F"),
            on_surface_variant: Color::from_hex("49454E"),
            inverse_surface: Color::from_hex("313033"),
            inverse_on_surface: Color::from_hex("F4EFF4"),
            background: Color::from_hex("FEF7FF"),
            on_background: Color::from_hex("1C1B1F"),
            error: Color::from_hex("B3261E"),
            error_container: Color::from_hex("F9DEDC"),
            on_error: Color::from_hex("FFFFFF"),
            on_error_container: Color::from_hex("410E0B"),
            outline: Color::from_hex("79747E"),
            outline_variant: Color::from_hex("C4C7C5"),
            shadow: Color::from_hex("000000"),
            surface_tint: Color::from_hex("6750A4"),
            scrim: Color::from_hex("000000"),
        }
    }

    pub fn dark() -> Self {
        Self {
            primary: Color::from_hex("D0BCFF"),
            primary_container: Color::from_hex("4F378B"),
            on_primary: Color::from_hex("371E73"),
            on_primary_container: Color::from_hex("EADDFF"),
            inverse_primary: Color::from_hex("6750A4"),
            secondary: Color::from_hex("CCC2DC"),
            secondary_container: Color::from_hex("4A4458"),
            on_secondary: Color::from_hex("332D41"),
            on_secondary_container: Color::from_hex("E8DEF8"),
            tertiary: Color::from_hex("EFB8C8"),
            tertiary_container: Color::from_hex("633B48"),
            on_tertiary: Color::from_hex("492532"),
            on_tertiary_container: Color::from_hex("FFD8E4"),
            surface: Color::from_hex("141218"),
            surface_dim: Color::from_hex("141218"),
            surface_bright: Color::from_hex("3B383E"),
            surface_container_lowest: Color::from_hex("0F0D13"),
            surface_container_low: Color::from_hex("1D1B20"),
            surface_container: Color::from_hex("211F26"),
            surface_container_high: Color::from_hex("2B2930"),
            surface_container_highest: Color::from_hex("36343B"),
            surface_variant: Color::from_hex("49454F"),
            on_surface: Color::from_hex("E6E1E5"),
            on_surface_variant: Color::from_hex("CAC4D0"),
            inverse_surface: Color::from_hex("E6E1E5"),
            inverse_on_surface: Color::from_hex("313033"),
            background: Color::from_hex("141218"),
            on_background: Color::from_hex("E6E1E5"),
            error: Color::from_hex("F2B8B5"),
            error_container: Color::from_hex("8C1D18"),
            on_error: Color::from_hex("601410"),
            on_error_container: Color::from_hex("F9DEDC"),
            outline: Color::from_hex("938F99"),
            outline_variant: Color::from_hex("444746"),
            shadow: Color::from_hex("000000"),
            surface_tint: Color::from_hex("D0BCFF"),
            scrim: Color::from_hex("000000"),
        }
    }
}

impl Theme {
    pub fn palette(&self) -> Palette {
        match self {
            Self::Light => Palette::light(),
            Self::Dark => Palette::dark(),
        }
    }
}

impl application::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _: &Self::Style) -> application::Appearance {
        let palette = self.palette();

        application::Appearance {
            background_color: palette.background,
            text_color: palette.on_background,
        }
    }
}
