use iced::Font;

#[allow(dead_code)]
pub fn get(file: String) -> String {
    format!("resources/{}", file)
}

pub const ICON_FONT: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../../resources/material_icons.ttf"),
};
