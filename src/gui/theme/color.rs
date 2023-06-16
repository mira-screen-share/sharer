use iced::Color;

pub trait ColorExt {
    fn with_alpha(self, a: f32) -> Self;
    fn mix(self, other: Color) -> Self;
    fn from_hex(hex: &str) -> Self;
}

impl ColorExt for Color {
    fn with_alpha(self, a: f32) -> Self {
        Color { a, ..self }
    }

    fn mix(self, other: Color) -> Self {
        mix(self, other)
    }

    fn from_hex(hex: &str) -> Color {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap() as f32 / 255.;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap() as f32 / 255.;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap() as f32 / 255.;
        Color::from_rgb(r, g, b)
    }
}

fn mix(bg: Color, fg: Color) -> Color {
    let a = 1. - (1. - fg.a) * (1. - bg.a);
    Color {
        r: fg.r * fg.a / a + bg.r * bg.a * (1. - fg.a) / a,
        g: fg.g * fg.a / a + bg.g * bg.a * (1. - fg.a) / a,
        b: fg.b * fg.a / a + bg.b * bg.a * (1. - fg.a) / a,
        a,
    }
}
