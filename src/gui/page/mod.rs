use std::fmt::Debug;

use iced::Command;

use crate::gui::app;
use crate::gui::theme::widget::Element;

pub mod start;
pub mod sharing;

pub trait Page {
    type Message: Into<app::Message> + Clone + Debug;
    type Props;

    fn update(&mut self, message: Self::Message) -> Command<app::Message>;
    fn view<'a>(&self, params: Self::Props) -> Element<'a, app::Message>;
}
