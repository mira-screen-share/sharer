#[allow(dead_code)]
pub fn get(file: String) -> String {
    format!("resources/{}", file)
}

pub mod font {
    pub const ICON: iced::Font = iced::Font::External {
        name: "Icons",
        bytes: include_bytes!("../../resources/Material-Icons.ttf"),
    };

    pub const BARLOW: iced::Font = iced::Font::External {
        name: "Barlow",
        bytes: include_bytes!("../../resources/Barlow-Regular.ttf"),
    };

    pub const BARLOW_BOLD: iced::Font = iced::Font::External {
        name: "Barlow-Bold",
        bytes: include_bytes!("../../resources/Barlow-Bold.ttf"),
    };
}

pub const APP_ICON: iced::window::Icon = iced::window::Icon::from_file_data(
    include_bytes!("../../resources/icons/mira.ico")
);
