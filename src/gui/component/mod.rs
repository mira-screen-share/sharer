use std::fmt::Debug;

use iced::Command;

use crate::gui::app;
use crate::gui::theme::widget::Element;

pub mod sharing;
pub mod start;

pub trait Component<'a> {
    type Message: Into<app::Message> + Clone + Debug;
    type UpdateProps;
    type ViewProps;

    fn update(&mut self, message: Self::Message, props: Self::UpdateProps)
        -> Command<app::Message>;
    fn view(&self, props: Self::ViewProps) -> Element<'_, app::Message>;
}
