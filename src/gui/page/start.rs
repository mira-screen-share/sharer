use std::sync::Arc;

use iced::Alignment::Center;
use iced::Length::Fill;
use tokio::sync::Mutex;

use crate::capture::capturer::Capturer;
use crate::column_iced;
use crate::gui::{app};
use crate::gui::page::Page;
use crate::gui::theme::button;
use crate::gui::theme::button::FAB;
use crate::gui::theme::widget::Element;

pub struct StartPage {
    pub capturer: Arc<Mutex<Capturer>>,
}

impl StartPage {
    pub fn new(capturer: Arc<Mutex<Capturer>>) -> Self {
        Self {
            capturer,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    Start,
}

impl From<Message> for app::Message {
    fn from(msg: Message) -> Self {
        app::Message::Start(msg)
    }
}

impl Page for StartPage {
    type Message = Message;
    type Props = ();

    fn update(&mut self, message: Message) -> iced::Command<app::Message> {
        match message {
            Message::Start => {
                if let Ok(mut capturer) = self.capturer.try_lock() {
                    capturer.run();
                }
            }
        }
        iced::Command::none()
    }

    fn view<'a>(&self, _params: Self::Props) -> Element<'a, app::Message> {
        column_iced![
            FAB::new("Start Sharing", "play.svg")
                .style(button::Style::Primary)
                .build()
                .on_press(Message::Start.into()),
        ].align_items(Center)
            .width(Fill)
            .into()
    }
}
